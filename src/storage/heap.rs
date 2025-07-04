//! Chunked delta heap storage with immutable delta documents

use crate::core::types::{ID8, ID16, ID32};
use crate::delta::types::{DeltaHeader, DeltaStatus, ChunkAddress};
use crate::constants::CHUNK_SIZE;
use dashmap::DashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Chunked delta heap with nested organization by document.
/// 
/// This structure provides:
/// - Immutable storage for delta documents
/// - Zero-copy access to delta data
/// - Thread-safe concurrent operations
/// - Organized by document for cache locality
pub struct DeltaHeap {
    /// Fixed-size chunks that never move once allocated
    chunks: Vec<Box<[u8; CHUNK_SIZE]>>,
    
    /// Current chunk being written to
    current_chunk: AtomicUsize,
    
    /// Current offset within the current chunk
    current_offset: AtomicUsize,
    
    /// Nested organization: Document -> Delta -> ChunkAddress
    /// This improves cache locality when processing document-specific deltas
    document_deltas: DashMap<ID16, DashMap<ID8, ChunkAddress>>,
    
    /// Mutex only for chunk allocation (rare operation)
    chunk_allocation_lock: std::sync::Mutex<()>,
}

impl DeltaHeap {
    /// Create a new delta heap with initial chunk
    pub fn new() -> Self {
        // Create initial chunk on the heap, not the stack
        let chunk = vec![0u8; CHUNK_SIZE].into_boxed_slice();
        let chunk: Box<[u8; CHUNK_SIZE]> = chunk.try_into().unwrap();
        
        let mut chunks = Vec::new();
        chunks.push(chunk);
        
        Self {
            chunks,
            current_chunk: AtomicUsize::new(0),
            current_offset: AtomicUsize::new(0),
            document_deltas: DashMap::new(),
            chunk_allocation_lock: std::sync::Mutex::new(()),
        }
    }
    
    /// Store a delta document in the heap
    /// 
    /// # Arguments
    /// 
    /// * `delta_id` - Unique identifier for this delta
    /// * `target_document_id` - Document this delta targets
    /// * `header` - Delta header with metadata
    /// * `operations` - Binary operation data
    /// 
    /// # Returns
    /// 
    /// The chunk address where the delta was stored
    pub fn store_delta(
        &self, 
        delta_id: ID8, 
        target_document_id: ID16,
        header: DeltaHeader, 
        operations: &[u8]
    ) -> Result<ChunkAddress, String> {
        let total_size = 32 + operations.len(); // header + operations
        
        // Atomic allocation within current chunk
        let allocated_offset = self.current_offset.fetch_add(total_size, Ordering::AcqRel);
        let chunk_id = self.current_chunk.load(Ordering::Acquire);
        
        // Check if we need a new chunk
        if allocated_offset + total_size > CHUNK_SIZE {
            self.allocate_new_chunk()?;
            // Retry allocation in new chunk
            return self.store_delta(delta_id, target_document_id, header, operations);
        }
        
        // Write to chunk (lock-free once we have the space)
        let chunk = &self.chunks[chunk_id];
        unsafe {
            let ptr = chunk.as_ptr().add(allocated_offset) as *mut u8;
            // Write header
            std::ptr::copy_nonoverlapping(
                &header as *const DeltaHeader as *const u8, 
                ptr, 
                32
            );
            // Write operations
            std::ptr::copy_nonoverlapping(
                operations.as_ptr(), 
                ptr.add(32), 
                operations.len()
            );
        }
        
        let address = ChunkAddress {
            chunk_id,
            offset: allocated_offset,
            length: total_size,
        };
        
        // Update nested index structure
        let document_map = self.document_deltas
            .entry(target_document_id)
            .or_insert_with(DashMap::new);
        document_map.insert(delta_id, address);
        
        Ok(address)
    }
    
    /// Allocate a new chunk when current chunk is full
    fn allocate_new_chunk(&self) -> Result<(), String> {
        let _lock = self.chunk_allocation_lock.lock()
            .map_err(|_| "Failed to acquire chunk allocation lock")?;
        
        // Double-check after acquiring lock
        if self.current_offset.load(Ordering::Acquire) < CHUNK_SIZE {
            return Ok(()); // Another thread already allocated
        }
        
        // Allocate new chunk on heap, not stack
        let chunk = vec![0u8; CHUNK_SIZE].into_boxed_slice();
        let new_chunk: Box<[u8; CHUNK_SIZE]> = chunk.try_into().unwrap();
        let new_chunk_id = self.chunks.len();
        
        // This is unsafe but we're the only thread that can modify chunks
        // due to the mutex above
        unsafe {
            let chunks_ptr = &self.chunks as *const Vec<_> as *mut Vec<_>;
            (*chunks_ptr).push(new_chunk);
        }
        
        // Update atomics
        self.current_chunk.store(new_chunk_id, Ordering::Release);
        self.current_offset.store(0, Ordering::Release);
        
        Ok(())
    }
    
    /// Get delta header and operations data
    /// 
    /// # Arguments
    /// 
    /// * `target_document_id` - Document the delta targets
    /// * `delta_id` - Delta identifier
    /// 
    /// # Returns
    /// 
    /// Tuple of (header reference, operations slice) if found
    pub fn get_delta(&self, target_document_id: &ID16, delta_id: &ID8) -> Option<(&DeltaHeader, &[u8])> {
        if let Some(document_map) = self.document_deltas.get(target_document_id) {
            if let Some(address) = document_map.get(delta_id) {
                if let Some(chunk) = self.chunks.get(address.chunk_id) {
                    let header_ptr = &chunk[address.offset] as *const u8 as *const DeltaHeader;
                    let header = unsafe { &*header_ptr };
                    let operations = &chunk[address.offset + 32..address.offset + address.length];
                    return Some((header, operations));
                }
            }
        }
        None
    }
    
    /// Update the processing status of a delta
    /// 
    /// # Arguments
    /// 
    /// * `target_document_id` - Document the delta targets
    /// * `delta_id` - Delta identifier
    /// * `status` - New processing status
    pub fn update_delta_status(
        &self, 
        target_document_id: &ID16, 
        delta_id: &ID8, 
        status: DeltaStatus
    ) -> Result<(), String> {
        if let Some(document_map) = self.document_deltas.get(target_document_id) {
            if let Some(address) = document_map.get(delta_id) {
                if let Some(chunk) = self.chunks.get(address.chunk_id) {
                    let header_ptr = &chunk[address.offset] as *const u8 as *mut DeltaHeader;
                    unsafe {
                        (*header_ptr).status = status;
                    }
                    return Ok(());
                }
            }
        }
        Err("Delta not found".to_string())
    }
    
    /// Get all delta IDs for a specific document
    /// 
    /// Useful for processing or debugging
    pub fn get_document_deltas(&self, document_id: &ID16) -> Vec<ID8> {
        if let Some(document_map) = self.document_deltas.get(document_id) {
            document_map.iter().map(|entry| *entry.key()).collect()
        } else {
            Vec::new()
        }
    }
    
    /// Get the current storage statistics
    pub fn stats(&self) -> HeapStats {
        let total_chunks = self.chunks.len();
        let current_chunk = self.current_chunk.load(Ordering::Acquire);
        let current_offset = self.current_offset.load(Ordering::Acquire);
        let total_documents = self.document_deltas.len();
        let total_deltas = self.document_deltas
            .iter()
            .map(|entry| entry.value().len())
            .sum();
        
        HeapStats {
            total_chunks,
            current_chunk,
            current_offset,
            total_documents,
            total_deltas,
            bytes_used: (current_chunk * CHUNK_SIZE) + current_offset,
            bytes_allocated: total_chunks * CHUNK_SIZE,
        }
    }
}

/// Statistics about delta heap usage
#[derive(Debug)]
pub struct HeapStats {
    /// Total number of allocated chunks
    pub total_chunks: usize,
    /// Currently active chunk
    pub current_chunk: usize,
    /// Current offset within active chunk
    pub current_offset: usize,
    /// Number of documents with deltas
    pub total_documents: usize,
    /// Total number of deltas stored
    pub total_deltas: usize,
    /// Bytes actually used for data
    pub bytes_used: usize,
    /// Total bytes allocated (may be larger than used)
    pub bytes_allocated: usize,
}

impl Default for DeltaHeap {
    fn default() -> Self {
        Self::new()
    }
}

// Safety: DeltaHeap is designed for concurrent access
unsafe impl Send for DeltaHeap {}
unsafe impl Sync for DeltaHeap {}

/// Per-user document heap for isolated storage
/// 
/// This structure provides:
/// - Isolated memory space per user for security
/// - Sequential heap storage for cache locality  
/// - Zero-copy access to document data
/// - Thread-safe concurrent operations
pub struct RootDocumentHeap {
    /// User who owns this heap
    owner_id: ID32,
    
    /// Fixed-size chunks for document headers
    header_chunks: Vec<Box<[u8; CHUNK_SIZE]>>,
    
    /// Fixed-size chunks for document data
    data_chunks: Vec<Box<[u8; CHUNK_SIZE]>>,
    
    /// Current header chunk being written to
    current_header_chunk: AtomicUsize,
    
    /// Current header offset within the current chunk
    current_header_offset: AtomicUsize,
    
    /// Current data chunk being written to
    current_data_chunk: AtomicUsize,
    
    /// Current data offset within the current chunk
    current_data_offset: AtomicUsize,
    
    /// Document header index: Document ID -> (chunk_id, offset, length)
    header_index: DashMap<ID16, (usize, usize, usize)>,
    
    /// Document data index: Document ID -> (chunk_id, offset, length)
    data_index: DashMap<ID16, (usize, usize, usize)>,
    
    /// Mutex for chunk allocation (rare operation)
    chunk_allocation_lock: std::sync::Mutex<()>,
}

impl RootDocumentHeap {
    /// Create a new document heap for a specific user
    pub fn new(owner_id: ID32) -> Self {
        // Create initial chunks on the heap, not the stack
        let header_chunk = vec![0u8; CHUNK_SIZE].into_boxed_slice();
        let data_chunk = vec![0u8; CHUNK_SIZE].into_boxed_slice();
        
        // Convert to the right type
        let header_chunk: Box<[u8; CHUNK_SIZE]> = header_chunk.try_into().unwrap();
        let data_chunk: Box<[u8; CHUNK_SIZE]> = data_chunk.try_into().unwrap();
        
        let mut header_chunks = Vec::new();
        header_chunks.push(header_chunk);
        
        let mut data_chunks = Vec::new();
        data_chunks.push(data_chunk);
        
        Self {
            owner_id,
            header_chunks,
            data_chunks,
            current_header_chunk: AtomicUsize::new(0),
            current_header_offset: AtomicUsize::new(0),
            current_data_chunk: AtomicUsize::new(0),
            current_data_offset: AtomicUsize::new(0),
            header_index: DashMap::new(),
            data_index: DashMap::new(),
            chunk_allocation_lock: std::sync::Mutex::new(()),
        }
    }
    
    /// Get the owner ID for this heap
    pub fn get_owner(&self) -> ID32 {
        self.owner_id
    }
    
    /// Get the total number of documents in this heap
    pub fn document_count(&self) -> usize {
        self.header_index.len()
    }
}

impl Default for RootDocumentHeap {
    fn default() -> Self {
        // Use a default user ID for default constructor
        Self::new(ID32::random())
    }
} 