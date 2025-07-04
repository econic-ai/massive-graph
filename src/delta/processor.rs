//! Delta processor with work-stealing and sequential per-document processing

use crate::core::types::{ID8, ID16};
use crate::delta::types::{DeltaHeader, DeltaStatus};
use crate::storage::heap::DeltaHeap;
use crate::storage::DocumentStorage;
use crate::constants::{DEFAULT_WORKER_THREADS, MAX_DELTA_OPERATIONS_PER_BATCH, WORKER_PARK_TIMEOUT_MS};
use dashmap::DashMap;
use crossbeam::deque::{Injector, Worker, Stealer};
use crossbeam::queue::SegQueue;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Delta processor with strict per-document ordering and work-stealing load balancing.
/// 
/// This processor provides:
/// - Sequential processing of deltas per document (maintains ordering)
/// - Work-stealing at document level for load balancing
/// - Concurrent processing across different documents
/// - Thread-safe delta submission and status tracking
pub struct DeltaProcessor<S: DocumentStorage + Send + Sync + 'static> {
    /// Chunked delta storage with nested organization
    delta_heap: Arc<DeltaHeap>,
    
    /// Per-document sequential queues (NO work-stealing here)
    document_queues: Arc<DashMap<ID16, SegQueue<ID8>>>,
    
    /// Global work queue - documents that have pending deltas
    /// Work-stealing happens at the DOCUMENT level, not delta level
    work_queue: Arc<Injector<ID16>>,
    
    /// Worker deques for local work distribution
    worker_deques: Vec<Worker<ID16>>,
    worker_stealers: Vec<Stealer<ID16>>,
    
    /// Worker threads
    workers: Vec<std::thread::JoinHandle<()>>,
    
    /// Shutdown signal
    shutdown: Arc<AtomicBool>,
    
    /// Document storage for applying deltas
    document_storage: Arc<S>,
}

impl<S: DocumentStorage + Send + Sync + 'static> DeltaProcessor<S> {
    /// Create a new delta processor with default number of worker threads
    pub fn new(delta_heap: Arc<DeltaHeap>, document_storage: Arc<S>) -> Self {
        Self::with_workers(delta_heap, document_storage, DEFAULT_WORKER_THREADS)
    }
    
    /// Create a new delta processor with specified number of worker threads
    /// 
    /// # Arguments
    /// 
    /// * `delta_heap` - Shared delta storage
    /// * `document_storage` - Document storage for applying deltas
    /// * `num_workers` - Number of worker threads to spawn
    pub fn with_workers(delta_heap: Arc<DeltaHeap>, document_storage: Arc<S>, num_workers: usize) -> Self {
        let work_queue = Arc::new(Injector::new());
        let document_queues = Arc::new(DashMap::new());
        let shutdown = Arc::new(AtomicBool::new(false));
        
        // Create worker deques for work-stealing
        let mut worker_deques = Vec::with_capacity(num_workers);
        let mut worker_stealers = Vec::with_capacity(num_workers);
        for _ in 0..num_workers {
            let worker = Worker::new_fifo();
            worker_stealers.push(worker.stealer());
            worker_deques.push(worker);
        }
        
        // Start worker threads
        let mut workers = Vec::with_capacity(num_workers);
        for i in 0..num_workers {
            let delta_heap_clone = Arc::clone(&delta_heap);
            let document_queues_clone = Arc::clone(&document_queues);
            let work_queue_clone = Arc::clone(&work_queue);
            let worker_stealers_clone = worker_stealers.clone();
            let shutdown_clone = Arc::clone(&shutdown);
            
            let worker = std::thread::Builder::new()
                .name(format!("delta-worker-{}", i))
                .spawn(move || {
                    Self::worker_loop(
                        i,
                        delta_heap_clone,
                        document_queues_clone,
                        work_queue_clone,
                        worker_stealers_clone,
                        shutdown_clone,
                    );
                })
                .expect("Failed to spawn delta worker thread");
                
            workers.push(worker);
        }
        
        Self {
            delta_heap,
            document_queues,
            work_queue,
            worker_deques,
            worker_stealers,
            workers,
            shutdown,
            document_storage,
        }
    }
    
    /// Submit a delta for processing
    /// 
    /// # Arguments
    /// 
    /// * `target_document_id` - Document this delta will modify
    /// * `executor_id` - User who submitted this delta
    /// * `operations` - Binary operation data
    /// 
    /// # Returns
    /// 
    /// Delta ID for tracking processing status
    pub fn submit_delta(
        &self,
        target_document_id: ID16,
        executor_id: ID16,
        operations: &[u8],
    ) -> Result<ID8, String> {
        let delta_id = ID8::random();
        
        // Create delta header
        let header = DeltaHeader::new(
            delta_id,
            Self::current_timestamp(),
            executor_id,
            operations.len() as u32,
            Self::count_operations(operations),
            DeltaStatus::Pending,
        );
        
        // Store in heap
        self.delta_heap.store_delta(delta_id, target_document_id, header, operations)?;
        
        // Add to document's sequential queue
        let document_queue = self.document_queues
            .entry(target_document_id)
            .or_insert_with(SegQueue::new);
        
        let was_empty = document_queue.is_empty();
        document_queue.push(delta_id);
        
        // If this was the first delta for this document, add to work queue
        if was_empty {
            self.work_queue.push(target_document_id);
        }
        
        Ok(delta_id)
    }
    
    /// Get the processing status of a delta
    pub fn get_delta_status(&self, target_document_id: &ID16, delta_id: &ID8) -> Option<DeltaStatus> {
        if let Some((header, _)) = self.delta_heap.get_delta(target_document_id, delta_id) {
            Some(header.status)
        } else {
            None
        }
    }
    
    /// Get statistics about the processor
    pub fn stats(&self) -> ProcessorStats {
        let heap_stats = self.delta_heap.stats();
        let pending_documents = self.document_queues
            .iter()
            .filter(|entry| !entry.value().is_empty())
            .count();
        
        ProcessorStats {
            heap_stats,
            pending_documents,
            total_document_queues: self.document_queues.len(),
            workers_active: self.workers.len(),
        }
    }
    
    /// Worker thread main loop
    fn worker_loop(
        worker_id: usize,
        delta_heap: Arc<DeltaHeap>,
        document_queues: Arc<DashMap<ID16, SegQueue<ID8>>>,
        work_queue: Arc<Injector<ID16>>,
        worker_stealers: Vec<Stealer<ID16>>,
        shutdown: Arc<AtomicBool>,
    ) {
        while !shutdown.load(Ordering::Acquire) {
            // Get next document to process (work-stealing at document level)
            if let Some(document_id) = Self::get_next_document(worker_id, &work_queue, &worker_stealers) {
                // Process ALL deltas for this document sequentially (NO stealing here)
                Self::process_document_deltas(
                    document_id,
                    &delta_heap,
                    &document_queues,
                    &work_queue,
                );
            } else {
                // No work available, park briefly
                std::thread::park_timeout(Duration::from_millis(WORKER_PARK_TIMEOUT_MS));
            }
        }
    }
    
    /// Get next document to work on (with work-stealing)
    fn get_next_document(
        worker_id: usize,
        work_queue: &Arc<Injector<ID16>>,
        worker_stealers: &[Stealer<ID16>],
    ) -> Option<ID16> {
        // Try global work queue first
        match work_queue.steal() {
            crossbeam::deque::Steal::Success(document_id) => return Some(document_id),
            _ => {}
        }
        
        // Try stealing from other workers (at document level)
        for (i, stealer) in worker_stealers.iter().enumerate() {
            if i == worker_id { continue; } // Don't steal from self
            
            match stealer.steal() {
                crossbeam::deque::Steal::Success(document_id) => return Some(document_id),
                _ => continue,
            }
        }
        
        None
    }
    
    /// Process all pending deltas for a document sequentially
    fn process_document_deltas(
        document_id: ID16,
        delta_heap: &Arc<DeltaHeap>,
        document_queues: &Arc<DashMap<ID16, SegQueue<ID8>>>,
        work_queue: &Arc<Injector<ID16>>,
    ) {
        if let Some(document_queue) = document_queues.get(&document_id) {
            // Process deltas sequentially for this document
            while let Some(delta_id) = document_queue.pop() {
                Self::process_single_delta(
                    document_id,
                    delta_id,
                    delta_heap,
                );
            }
            
            // Check if more deltas arrived while we were processing
            if !document_queue.is_empty() {
                work_queue.push(document_id);
            }
        }
    }
    
    /// Process a single delta operation
    fn process_single_delta(
        document_id: ID16,
        delta_id: ID8,
        delta_heap: &Arc<DeltaHeap>,
    ) {
        // Get delta from heap
        let (header, operations) = match delta_heap.get_delta(&document_id, &delta_id) {
            Some((h, ops)) => (h.clone(), ops.to_vec()),
            None => return, // Delta not found
        };
        
        // Update status to validating
        let _ = delta_heap.update_delta_status(&document_id, &delta_id, DeltaStatus::Validating);
        
        // Validate delta
        if !Self::validate_delta(&header, &operations, &document_id) {
            let _ = delta_heap.update_delta_status(&document_id, &delta_id, DeltaStatus::Rejected);
            return;
        }
        
        // Update status to applying
        let _ = delta_heap.update_delta_status(&document_id, &delta_id, DeltaStatus::Applying);
        
        // Apply delta to document
        match Self::apply_delta_operations(&document_id, &operations) {
            Ok(_) => {
                let _ = delta_heap.update_delta_status(&document_id, &delta_id, DeltaStatus::Applied);
                // TODO: Propagate to network
            }
            Err(_) => {
                let _ = delta_heap.update_delta_status(&document_id, &delta_id, DeltaStatus::Failed);
            }
        }
    }
    
    /// Validate a delta before applying it
    fn validate_delta(_header: &DeltaHeader, _operations: &[u8], _document_id: &ID16) -> bool {
        // TODO: Implement proper validation
        // - Checksum verification
        // - User ID validation  
        // - Document ID validation
        // - Cryptographic proof validation
        true
    }
    
    /// Apply delta operations to a document
    fn apply_delta_operations(
        _document_id: &ID16,
        _operations: &[u8],
    ) -> Result<(), String> {
        // TODO: Parse operations and apply to document
        // This would parse the binary operations and call storage methods
        Ok(())
    }
    
    /// Get current timestamp in nanoseconds
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
    
    /// Count the number of operations in the binary data
    fn count_operations(_operations: &[u8]) -> u16 {
        // TODO: Parse operations and count them
        // This would scan the binary format and count operation headers
        1
    }
}

/// Statistics about delta processor performance
#[derive(Debug)]
pub struct ProcessorStats {
    /// Delta heap statistics
    pub heap_stats: crate::storage::heap::HeapStats,
    /// Number of documents with pending deltas
    pub pending_documents: usize,
    /// Total number of document queues
    pub total_document_queues: usize,
    /// Number of active worker threads
    pub workers_active: usize,
}

impl<S: DocumentStorage + Send + Sync + 'static> Drop for DeltaProcessor<S> {
    fn drop(&mut self) {
        // Signal shutdown
        self.shutdown.store(true, Ordering::Release);
        
        // Wait for workers to finish
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
} 