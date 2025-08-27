/// Storage
use std::sync::atomic::{AtomicU64, AtomicUsize};
use std::sync::Arc;
use arc_swap::ArcSwap;
use crate::constants::CHUNK_SIZE;
use super::DocId;
use dashmap::DashMap;
use crossbeam::queue::SegQueue;

/// Memory chunk for storage
#[allow(dead_code)] // POC: Fields will be used in future implementation
pub struct Chunk {
    data: Box<[u8; CHUNK_SIZE]>,    // 16MB contiguous
    used: AtomicUsize,                   // Bytes allocated
    chunk_idx: u32,                      // Index in chunks vector
    created_at: u64,
}

/// Reference to a chunk location
#[allow(dead_code)] // POC: Fields will be used in future implementation
pub struct ChunkRef {
    chunk_idx: u32,
    offset: u32,
    length: u32,
}

/// State of a chunk in the storage system
#[repr(u8)]
pub enum ChunkState {
   /// Currently receiving writes
   Active = 0,
   /// No more writes, can persist
   Sealed = 1,
   /// Written to disk
   Persisted = 2,
   /// Moved to cold storage
   Archived = 3,
}

/// Size categories for chunks
#[repr(usize)]
pub enum ChunkSize {
   /// 64KB - Very slow documents
   Tiny = 65_536,
   /// 1MB - Normal documents
   Small = 1_048_576,
   /// 4MB - Active documents
   Medium = 4_194_304,
   /// 16MB - Streams/hot documents
   Large = 16_777_216,
}

/// Storage system for managing chunks
#[allow(dead_code)] // POC: Fields will be used in future implementation
pub struct ChunkStorage {
    // Thread-safe list of all chunks per document
    document_chunks: DashMap<DocId, ArcSwap<Vec<Arc<Chunk>>>>,
    
    // Current active chunk for fast allocation
    active_chunks: DashMap<DocId, Arc<Chunk>>,
    
    // Pre-allocated chunk pools for different sizes (native only)
    chunk_pools: [SegQueue<Arc<Chunk>>; 4],
    
    sequence: AtomicU64,
}

impl ChunkStorage {
    #[allow(dead_code)] // POC: Method will be used in future implementation
    fn seal_and_add_new_chunk(&self, doc_id: DocId) {
        // Get new chunk from pool
        let new_chunk = self.get_or_create_chunk(ChunkSize::Medium);
        
        // Add to document's chunk list (thread-safe)
        let entry = self.document_chunks.entry(doc_id)
            .or_insert_with(|| ArcSwap::new(Arc::new(Vec::new())));
        
        let mut chunks = entry.load().as_ref().clone();
        chunks.push(new_chunk.clone());
        entry.store(Arc::new(chunks));
        
        // Update active chunk
        self.active_chunks.insert(doc_id, new_chunk);
    }
    
    #[allow(dead_code)] // POC: Method will be used in future implementation
    fn get_or_create_chunk(&self, _size: ChunkSize) -> Arc<Chunk> {
        let pool_idx = _size as usize;
        // Try to get from pool first (atomic, fast)
        if let Some(chunk) = self.chunk_pools[pool_idx].pop() {
            return chunk;
        }

        // Fallback: Create new chunk with requested size
        Arc::new(Chunk {
            data: Box::new([0; CHUNK_SIZE]),
            used: AtomicUsize::new(0),
            chunk_idx: 0,
            created_at: 0,
        })
    }    
}

