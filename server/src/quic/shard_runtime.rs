//! Shard runtime for processing deltas
//! 
//! Each shard:
//! - Owns validation and persistence for its documents
//! - Performs single-copy from stream to storage
//! - Uses CPU-pinned threads for cache locality

use s2n_quic::stream::ReceiveStream;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::timeout;
use massive_graph_core::{log_info, log_error, log_warn};
use massive_graph_core::types::storage::ChunkStorage;

use crate::quic::types::{
    DeltaHeaderMeta, ConnectionInfo, ShardId, DELTA_HEADER_SIZE, 
    SECURITY_HEADER_SIZE, Timeouts
};

/// Task submitted to shard runtime
struct ShardTask {
    stream: ReceiveStream,
    header_buf: [u8; DELTA_HEADER_SIZE],
    conn_info: ConnectionInfo,
}

/// Runtime for a single shard
pub struct ShardRuntime {
    /// Shard ID
    shard_id: ShardId,
    /// Task submission channel
    task_tx: mpsc::UnboundedSender<ShardTask>,
    /// Storage reference
    storage: Arc<ChunkStorage>,
}

impl ShardRuntime {
    /// Create new shard runtime
    pub fn new(
        shard_id: ShardId,
        storage: Arc<ChunkStorage>,
        worker_count: usize,
    ) -> Self {
        let (task_tx, task_rx) = mpsc::unbounded_channel();
        
        // Spawn worker threads - they will share the receiver
        let task_rx = Arc::new(tokio::sync::Mutex::new(task_rx));
        for worker_id in 0..worker_count {
            let rx = task_rx.clone();
            let storage = storage.clone();
            let shard_id = shard_id;
            
            tokio::spawn(async move {
                shard_worker(shard_id, worker_id, rx, storage).await;
            });
        }
        
        Self {
            shard_id,
            task_tx,
            storage,
        }
    }
    
    /// Submit a stream to this shard for processing
    pub async fn submit_stream(
        &self,
        stream: ReceiveStream,
        header_buf: [u8; DELTA_HEADER_SIZE],
        conn_info: ConnectionInfo,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let task = ShardTask {
            stream,
            header_buf,
            conn_info,
        };
        
        self.task_tx.send(task)
            .map_err(|_| "Shard task queue closed")?;
        
        Ok(())
    }
}

/// Worker thread for processing shard tasks
async fn shard_worker(
    shard_id: ShardId,
    worker_id: usize,
    task_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<ShardTask>>>,
    storage: Arc<ChunkStorage>,
) {
    log_info!("Shard {} worker {} started", shard_id.0, worker_id);
    
    loop {
        let task = {
            let mut rx = task_rx.lock().await;
            rx.recv().await
        };
        
        if let Some(task) = task {
            if let Err(e) = process_stream_task(task, &storage).await {
                log_error!("Shard {} worker {} task error: {}", shard_id.0, worker_id, e);
            }
        } else {
            // Channel closed, exit
            break;
        }
    }
    
    log_info!("Shard {} worker {} stopped", shard_id.0, worker_id);
}

/// Process a single stream task - this is where single-copy happens
async fn process_stream_task(
    mut task: ShardTask,
    storage: &Arc<ChunkStorage>,
) -> Result<(), Box<dyn std::error::Error>> {
    let timeouts = Timeouts::default();
    let mut stream = task.stream;
    let mut header_buf = task.header_buf;
    
    // Process deltas from this stream
    loop {
        // Parse header to get size
        let meta = DeltaHeaderMeta::parse(&header_buf)?;
        let total_size = SECURITY_HEADER_SIZE + meta.total_size as usize;
        
        // CRITICAL: Reserve space in storage
        let mut write_handle = match storage.reserve(total_size) {
            Ok(handle) => handle,
            Err(e) => {
                log_error!("Failed to reserve storage: {}", e);
                return Err(e.into());
            }
        };
        
        // Layout: [SecurityHeader | DeltaHeader | Payload]
        {
            let buf = write_handle.buffer_mut();
            
            // 1. Zero security header (will be filled later)
            buf[..SECURITY_HEADER_SIZE].fill(0);
            
            // 2. Copy delta header we already read
            buf[SECURITY_HEADER_SIZE..SECURITY_HEADER_SIZE + DELTA_HEADER_SIZE]
                .copy_from_slice(&header_buf);
        }
        
        // 3. SINGLE COPY: Read payload directly into storage
        let payload_start = SECURITY_HEADER_SIZE + DELTA_HEADER_SIZE;
        let payload_end = SECURITY_HEADER_SIZE + meta.total_size as usize;
        
        // Read payload using receive() API
        let mut payload_received = payload_start;
        while payload_received < payload_end {
            match timeout(timeouts.payload_read, stream.receive()).await {
                Ok(Ok(Some(data))) => {
                    let to_copy = (payload_end - payload_received).min(data.len());
                    let buf = write_handle.buffer_mut();
                    buf[payload_received..payload_received + to_copy].copy_from_slice(&data[..to_copy]);
                    payload_received += to_copy;
                }
                Ok(Ok(None)) => {
                    log_error!("Stream closed during payload read");
                    return Err("Stream closed".into());
                }
                Ok(Err(e)) => {
                    log_error!("Stream error during payload read: {}", e);
                    return Err(format!("Stream error: {}", e).into());
                }
                Err(_) => {
                    log_error!("Payload read timeout");
                    return Err("Timeout".into());
                }
            }
        }
        
        // Now we have the full delta - commit it
        let chunk_ref = write_handle.commit();
        
        log_info!(
            "Delta stored: doc_id={}, size={}, chunk_ref={:?}",
            meta.doc_id, meta.total_size, chunk_ref
        );
        
        // TODO: Queue for validation
        // validation_queue.push(chunk_ref)?;
        
        // Try to read next header
        let mut next_received = 0;
        loop {
            match timeout(timeouts.header_read, stream.receive()).await {
                Ok(Ok(Some(data))) => {
                    let to_copy = (DELTA_HEADER_SIZE - next_received).min(data.len());
                    header_buf[next_received..next_received + to_copy].copy_from_slice(&data[..to_copy]);
                    next_received += to_copy;
                    
                    if next_received == DELTA_HEADER_SIZE {
                        // Got next header, continue outer loop
                        break;
                    }
                }
                Ok(Ok(None)) => {
                    // End of stream, normal termination
                    return Ok(());
                }
                Ok(Err(_)) => {
                    // Stream error, terminate
                    return Ok(());
                }
                Err(_) => {
                    // Timeout between deltas, close stream
                    log_warn!("Stream idle timeout");
                    return Ok(());
                }
            }
        }
        
        // Check if we got a full header
        if next_received == DELTA_HEADER_SIZE {
            // Continue with next delta
            continue;
        } else {
            // Partial header means stream ended
            break;
        }
    }
    
    Ok(())
}
