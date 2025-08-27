/// Storage
use std::sync::atomic::{AtomicU64, AtomicUsize};
use std::sync::Arc;
use arc_swap::ArcSwap;
use crate::constants::CHUNK_SIZE;
use crate::types::DocId;
use dashmap::DashMap;
use crossbeam::queue::SegQueue;

pub struct Chunk {
    data: Box<[u8; CHUNK_SIZE]>,    // 16MB contiguous
    used: AtomicUsize,                   // Bytes allocated
    chunk_idx: u32,                      // Index in chunks vector
    created_at: u64,
}

pub struct ChunkRef {
    chunk_idx: u32,
    offset: u32,
    length: u32,
}

#[repr(u8)]
pub enum ChunkState {
   Active = 0,      // Currently receiving writes
   Sealed = 1,      // No more writes, can persist
   Persisted = 2,   // Written to disk
   Archived = 3,    // Moved to cold storage
}

#[repr(usize)]
pub enum ChunkSize {
   Tiny = 65_536,        // 64KB - Very slow documents
   Small = 1_048_576,    // 1MB - Normal documents  
   Medium = 4_194_304,   // 4MB - Active documents
   Large = 16_777_216,   // 16MB - Streams/hot documents
}

pub struct ChunkStorage {
    // Thread-safe list of all chunks per document
    document_chunks: DashMap<DocId, ArcSwap<Vec<Arc<Chunk>>>>,
    
    // Current active chunk for fast allocation
    active_chunks: DashMap<DocId, Arc<Chunk>>,
    
    // Pre-allocated pools
    chunk_pools: [SegQueue<Arc<Chunk>>; 4],
    
    sequence: AtomicU64,
}

impl ChunkStorage {
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
    fn get_or_create_chunk(&self, size: ChunkSize) -> Arc<Chunk> {
        let pool_idx = size as usize;
        
        // Try to get from pool first (atomic, fast)
        if let Some(chunk) = self.chunk_pools[pool_idx].pop() {
            chunk
        } else {
            // Create new chunk with requested size
            Arc::new(Chunk {
                data: Box::new([0; CHUNK_SIZE]),
                used: AtomicUsize::new(0),
                chunk_idx: 0,
                created_at: 0,
            })
        }
    }    
}

