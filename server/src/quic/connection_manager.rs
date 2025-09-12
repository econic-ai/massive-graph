//! Connection manager for QUIC ingress
//! 
//! Handles:
//! - Connection acceptance
//! - Stream demuxing to lanes
//! - Header parsing and shard routing

use s2n_quic::stream::ReceiveStream;
use s2n_quic::connection::Connection;
use s2n_quic::Server;
use std::sync::Arc;
use tokio::time::timeout;
use massive_graph_core::{log_info, log_error, log_warn};
use crate::constants::{DELTA_HEADER_SIZE, LANES_PER_CONNECTION};

use crate::quic::types::{
    DeltaHeaderMeta, ShardId, ConnectionInfo, Timeouts, doc_hash_u64
};
use crate::quic::shard_runtime::ShardRuntime;

/// Manages incoming QUIC connections and routes to shards
pub struct ConnectionManager {
    /// QUIC server instance
    server: Server,
    /// Shard runtimes for processing
    shards: Vec<Arc<ShardRuntime>>,
    /// Configuration
    shard_count: u16,
    /// Timeouts
    timeouts: Timeouts,
}

impl ConnectionManager {
    /// Create new connection manager
    pub fn new(
        server: Server,
        shards: Vec<Arc<ShardRuntime>>,
        shard_count: u16,
    ) -> Self {
        Self {
            server,
            shards,
            shard_count,
            timeouts: Timeouts::default(),
        }
    }
    
    /// Run the connection accept loop
    pub async fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        log_info!("QUIC ConnectionManager starting");
        
        while let Some(connection) = self.server.accept().await {
            let conn_info = ConnectionInfo {
                user_id: massive_graph_core::types::UserId::random(), // TODO: Extract from auth
                connection_id: connection.id().to_string(),
                established_at: std::time::Instant::now(),
            };
            
            log_info!("New QUIC connection: {}", conn_info.connection_id);
            
            // Spawn task to handle this connection
            let shards = self.shards.clone();
            let shard_count = self.shard_count;
            let timeouts = self.timeouts.clone();
            
            tokio::spawn(async move {
                if let Err(e) = handle_connection(connection, conn_info, shards, shard_count, timeouts).await {
                    log_error!("Connection handler error: {}", e);
                }
            });
        }
        
        Ok(())
    }
}

/// Handle a single connection's streams
async fn handle_connection(
    mut connection: Connection,
    conn_info: ConnectionInfo,
    shards: Vec<Arc<ShardRuntime>>,
    shard_count: u16,
    timeouts: Timeouts,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn_id = conn_info.connection_id.clone();
    
    // Accept unidirectional streams (lanes)
    let mut lane_count = 0;
    while lane_count < LANES_PER_CONNECTION {
        match connection.accept_receive_stream().await {
            Ok(Some(stream)) => {
                log_info!("Accepted lane {} on connection {}", lane_count, conn_id);
                
                // Spawn task to handle this lane
                let shards = shards.clone();
                let conn_info = conn_info.clone();
                let timeouts = timeouts.clone();
                
                tokio::spawn(async move {
                    if let Err(e) = handle_lane(stream, conn_info, shards, shard_count, timeouts).await {
                        log_error!("Lane handler error: {}", e);
                    }
                });
                
                lane_count += 1;
            }
            Ok(None) => {
                log_info!("Connection {} closed", conn_id);
                break;
            }
            Err(e) => {
                log_error!("Error accepting stream on {}: {}", conn_id, e);
                break;
            }
        }
    }
    
    Ok(())
}

/// Handle a single lane (unidirectional stream)
async fn handle_lane(
    mut stream: ReceiveStream,
    conn_info: ConnectionInfo,
    shards: Vec<Arc<ShardRuntime>>,
    shard_count: u16,
    timeouts: Timeouts,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read deltas from this lane
    loop {
        // Read delta header first
        let mut header_buf = [0u8; DELTA_HEADER_SIZE];
        
        // s2n-quic uses receive() API
        let mut received = 0;
        while received < DELTA_HEADER_SIZE {
            match timeout(timeouts.header_read, stream.receive()).await {
                Ok(Ok(Some(data))) => {
                    // s2n-quic returns Result<Option<Bytes>, Error>
                    let to_copy = (DELTA_HEADER_SIZE - received).min(data.len());
                    header_buf[received..received + to_copy].copy_from_slice(&data[..to_copy]);
                    received += to_copy;
                }
                Ok(Ok(None)) => {
                    return Err("Stream closed".into());
                }
                Ok(Err(e)) => {
                    return Err(format!("Stream error: {}", e).into());
                }
                Err(_) => {
                    return Err("Header read timeout".into());
                }
            }
        }
        
        // Parse header to get doc_id for routing
        let meta = DeltaHeaderMeta::parse(&header_buf)?;
        
        // Compute shard (and keep doc hash to avoid recompute downstream)
        let doc_hash = doc_hash_u64(&meta.doc_id);
        let shard_id = ShardId((doc_hash % shard_count as u64) as u16);
        
        // Hand off to shard runtime
        // This is where we transfer ownership of the stream
        let shard = &shards[shard_id.0 as usize];
        shard.submit_stream(stream, header_buf, doc_hash, conn_info.clone()).await?;
        
        // Stream is now owned by shard, break from loop
        break;
    }
    
    Ok(())
}

/// Helper to create s2n-quic server
pub async fn create_quic_server(
    config: &massive_graph_core::core::config::QuicConfig,
) -> Result<Server, Box<dyn std::error::Error>> {
    use s2n_quic::provider::tls;
    use std::fs;
    
    // Load certificate and key
    let tls_config = if let (Some(cert_path), Some(key_path)) = (&config.cert_path, &config.key_path) {
        // log_info!("Loading TLS certificate from: {}", cert_path);
        // log_info!("Loading TLS key from: {}", key_path);
        
        // Verify files exist and are readable
        if !std::path::Path::new(cert_path).exists() {
            return Err(format!("Certificate file not found: {}", cert_path).into());
        }
        if !std::path::Path::new(key_path).exists() {
            return Err(format!("Key file not found: {}", key_path).into());
        }
        
        // Read the certificate content to verify it's PEM format
        let cert_content = fs::read_to_string(cert_path)?;
        if !cert_content.contains("-----BEGIN CERTIFICATE-----") {
            return Err("Certificate file does not appear to be in PEM format".into());
        }
        
        // Read the key content to verify it's PEM format
        let key_content = fs::read_to_string(key_path)?;
        if !key_content.contains("-----BEGIN") {
            return Err("Key file does not appear to be in PEM format".into());
        }
        
        log_info!("Certificate and key files validated successfully");
        
        // Try loading from memory instead of file paths
        tls::default::Server::builder()
            .with_certificate(&cert_content, &key_content)?
            .build()?
    } else {
        log_warn!("No TLS cert/key provided in config");
        return Err("TLS certificate and key required for QUIC server. Please set cert_path and key_path in config".into());
    };
    
    let server = Server::builder()
        .with_tls(tls_config)?
        .with_io(config.bind_address)?
        .start()?;
    
    Ok(server)
}
