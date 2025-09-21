/// Storage
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use arc_swap::ArcSwap;
use crossbeam::queue::SegQueue;
use core::marker::PhantomData;
use crate::structures::optimised_index::OptimisedIndex;
use crate::types::document::{DocumentHeader};
use crate::types::ChunkId;
use crate::DocId;

/// Typed reference to a chunk location
#[derive(Debug, Clone)]
pub struct ChunkRef<T> {
    /// The chunk identifier
    pub chunk: Arc<Chunk>,
    /// Byte offset within the chunk
    pub offset: u32,
    /// Length in bytes starting at offset
    pub length: u32,
    /// Compile-time marker for the reference kind
    _marker: PhantomData<fn() -> T>,
}

impl<T> ChunkRef<T> {
    /// Create a new typed chunk reference
    pub fn new(chunk: Arc<Chunk>, offset: u32, length: u32) -> Self {
        Self { chunk, offset, length, _marker: PhantomData }
    }
}

impl<T> ChunkRef<T> 
where 
    T: ChunkType,
{
    /// Read the wire format data from this chunk reference
    pub fn read(&self) -> T::WireType<'_> {
        let bytes = self.chunk.as_slice(self.offset, self.length);
        T::WireType::from_bytes(bytes)
    }
}

/// State of a chunk in the storage system
#[repr(u8)]
#[derive(Debug)]
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

/// Metadata for a chunk
#[derive(Debug)]
pub struct ChunkMetadata {
    /// Document ID
    pub document_id: Option<DocId>,
    /// Chunk Size
    pub chunk_size: ChunkSize,
    /// total re
    pub final_record_count: u64,
    /// state
    pub state: ChunkState,
    /// Created at timestamp
    pub created_at: u64,    
}

/// Memory chunk instance storing contiguous bytes
#[derive(Debug)]
pub struct Chunk {
    /// Backing storage for this chunk
    pub data: Box<[u8]>,          // size determined by ChunkStorage::chunk_size
    /// Bytes allocated within this chunk
    pub used: AtomicUsize,        // bytes allocated within this chunk
    /// Unique identifier for this chunk
    pub id: ChunkId,              // unique id
    /// Next chunk in the chain
    pub next: Option<Arc<Chunk>>,
    /// Chunk metadata
    pub metadata: ChunkMetadata,
}

impl Chunk {
    /// Get a slice of data from this chunk
    pub fn as_slice(&self, offset: u32, length: u32) -> &[u8] {
        &self.data[offset as usize..(offset + length) as usize]
    }
}

/// Size categories for chunks
#[repr(usize)]
#[derive(Clone, Copy, Debug)]
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

/// Per-kind chunk storage (typed by T)
pub struct ChunkStorage<T> {
    /// Index mapping chunk id to chunk instance
    pub chunks: Vec<Arc<Chunk>>,
    /// Currently active chunk for allocation
    pub active_chunk: ArcSwap<Arc<Chunk>>,
    /// Next chunk to use
    pub next_chunk: ArcSwap<Option<Arc<Chunk>>>,
    /// Pool of ready-to-use chunks
    pub chunk_pool: SegQueue<Arc<Chunk>>,
    /// Monotonic chunk id sequence
    pub sequence: AtomicU64,
    /// Compile-time marker for storage kind
    _marker: PhantomData<fn() -> T>,
}

// impl<T> ChunkStorage<T> {
//     /// Create a new typed chunk storage with a fixed chunk size
//     pub fn new() -> Self {
//         let chunk_size = ChunkSize::Tiny;
//         Self::new_with_size(chunk_size)
//     }

//     /// Create a new typed chunk storage with a fixed chunk size
//     pub fn new_with_size(chunk_size: ChunkSize) -> Self {
//         // Allocate first active chunk
//         let id: ChunkId = 0;
//         let size = chunk_size as usize;
//         let chunk = Arc::new(Chunk { data: vec![0u8; size].into_boxed_slice(), used: AtomicUsize::new(0), id });
//         let active = Arc::clone(&chunk);
//         let pool: SegQueue<Arc<Chunk>> = SegQueue::new();
        
//         // Insert first chunk into index
//         let storage = Self {
//             chunks: OptimisedIndex::new(),
//             active_chunk: ArcSwap::from_pointee(active),
//             chunk_pool: pool,
//             sequence: AtomicU64::new(1), // Next ID will be 1
//             chunk_size,
//             _marker: PhantomData,
//         };
//         storage.chunks.upsert(id, chunk.clone());
//         storage
//     }

//     /// Read immutable bytes at a typed reference (zero-copy). Not yet implemented.
//     pub fn read_typed<'a>(&'a self, _r: ChunkRef<T>) -> Result<&'a [u8], String> {
//         Err("ChunkStorage::read_typed not yet implemented".to_string())
//     }

//     /// Reserve space for a write and return a RAII handle
//     pub fn reserve(&self, size: usize) -> Result<WriteHandle<T>, String> {
//         if size == 0 {
//             return Err("Cannot reserve zero bytes".to_string());
//         }
        
//         // Ensure we have capacity in the active chunk
//         self.ensure_active_capacity(size)?;
        
//         // Get the active chunk and reserve space atomically
//         let active = self.active_chunk.load();
//         let offset = active.used.fetch_add(size, Ordering::SeqCst);
//         let chunk_size = self.chunk_size as usize;
        
//         if offset + size > chunk_size {
//             // This shouldn't happen if ensure_active_capacity worked correctly
//             return Err("Chunk overflow during reservation".to_string());
//         }
        
//         // Create chunk reference for the reserved space
//         let chunk_ref = ChunkRef::new(Arc::clone(&active), offset as u32, size as u32);
        
//         // Create mutable slice to the reserved region
//         // SAFETY: We atomically reserved this region, giving us exclusive access.
//         // The 'static lifetime is a controlled lie - the Arc<Chunk> in WriteHandle
//         // ensures the memory remains valid for the handle's lifetime.
//         let buffer: &'static mut [u8] = unsafe {
//             let ptr = active.data.as_ptr().add(offset) as *mut u8;
//             let slice = std::slice::from_raw_parts_mut(ptr, size);
//             std::mem::transmute(slice)
//         };
        
//         Ok(WriteHandle {
//             chunk_ref: Some(chunk_ref),
//             buffer,
//             _chunk: Arc::clone(&active),
//             committed: false,
//         })
//     }

//     /// Ensure the active chunk has capacity for the requested size
//     fn ensure_active_capacity(&self, size: usize) -> Result<(), String> {
//         let chunk_size = self.chunk_size as usize;
//         if size > chunk_size {
//             return Err(format!("Requested size {} exceeds chunk size {}", size, chunk_size));
//         }
        
//         loop {
//             let active = self.active_chunk.load();
//             let current_used = active.used.load(Ordering::SeqCst);
            
//             // Check if we have enough space
//             if current_used + size <= chunk_size {
//                 return Ok(());
//             }
            
//             // Need to rotate to a new chunk
//             self.rotate_active()?;
//         }
//     }

//     /// Rotate the active chunk when the current one is full
//     fn rotate_active(&self) -> Result<(), String> {
//         // Try to get a chunk from the pool first
//         let new_chunk = if let Some(pooled) = self.chunk_pool.pop() {
//             // Reset the pooled chunk
//             pooled.used.store(0, Ordering::SeqCst);
//             pooled
//         } else {
//             // Allocate a new chunk
//             let id = self.sequence.fetch_add(1, Ordering::SeqCst);
//             let size = self.chunk_size as usize;
//             Arc::new(Chunk {
//                 data: vec![0u8; size].into_boxed_slice(),
//                 used: AtomicUsize::new(0),
//                 id,
//             })
//         };
        
//         // Insert the new chunk into the index
//         self.chunks.upsert(new_chunk.id, new_chunk.clone());
        
//         // Atomically swap the active chunk
//         self.active_chunk.store(Arc::new(new_chunk));
        
//         Ok(())
//     }

//     /// Refill the chunk pool to maintain a target number of ready chunks
//     pub fn refill_pool(&self, target: usize) {
//         let current_size = self.chunk_pool.len();
//         for _ in current_size..target {
//             let id = self.sequence.fetch_add(1, Ordering::SeqCst);
//             let size = self.chunk_size as usize;
//             let chunk = Arc::new(Chunk {
//                 data: vec![0u8; size].into_boxed_slice(),
//                 used: AtomicUsize::new(0),
//                 id,
//             });
//             self.chunk_pool.push(chunk);
//         }
//     }

// }



/// RAII write handle for a typed storage reservation
/// 
/// This handle provides direct mutable access to reserved chunk memory,
/// enabling zero-copy writes from network streams or other sources.
pub struct WriteHandle<T> {
    /// Reference to where data will be written upon commit
    pub chunk_ref: Option<ChunkRef<T>>,
    /// Direct mutable slice into the chunk's reserved region
    /// The 'static lifetime is a controlled lie - _chunk keeps memory alive
    pub buffer: &'static mut [u8],
    /// Keep chunk alive while we hold the mutable reference
    _chunk: Arc<Chunk>,
    /// Whether this handle has been committed
    committed: bool,
}

// Note: Send/Sync are automatic for &mut [u8] - no unsafe impl needed

impl<T> WriteHandle<T> {
    /// Commit the reservation and finalize the write, returning the typed reference
    pub fn commit(mut self) -> ChunkRef<T> {
        // No copy needed - data was written directly to chunk
        self.committed = true;
        self.chunk_ref.take().expect("valid chunk_ref")
    }

    /// Get mutable access to the buffer (zero-copy into chunk storage)
    pub fn buffer_mut(&mut self) -> &mut [u8] {
        // Direct access - no dereferencing needed!
        // The &'static lifetime is safe because _chunk keeps the memory alive
        self.buffer
    }
}

impl<T> Drop for WriteHandle<T> {
    fn drop(&mut self) {
        if !self.committed {
            // In real implementation, would roll back the reservation
            // For now, the space is just "leaked" until the chunk rotates
        }
    }
}

/// Trait for types that can be created from raw bytes with zero-copy
pub trait WireFormat<'a>: Sized {
    /// Create instance from raw bytes slice
    fn from_bytes(bytes: &'a [u8]) -> Self;
    /// Get the size of the wire format
    fn to_bytes(&self) -> &[u8];
    // // Get the length of the wire format
    // fn length(&self) -> usize;
}


/// Trait to associate chunk types with their wire format types
pub trait ChunkType {
    /// The wire format type this chunk stores
    type WireType<'a>: WireFormat<'a>;
}

/// Marker for document header chunks (type tag only)
#[derive(Debug)]
pub struct DocumentHeaderChunk;

impl ChunkType for DocumentHeaderChunk {
    type WireType<'a> = DocumentHeader<'a>;
}

/// Typed reference to a document header chunk
pub type DocumentHeaderChunkRef = ChunkRef<DocumentHeaderChunk>;
/// Typed storage for document headers
pub type DocumentHeaderStorage = ChunkStorage<DocumentHeaderChunk>;

// /// Marker for document version chunks (type tag only)
// #[derive(Debug)]
// pub struct DocumentVersionChunk;

// impl ChunkType for DocumentVersionChunk {
//     type WireType<'a> = DocumentVersion<'a>;
// }

// /// Typed reference to a document version chunk
// pub type DocumentVersionChunkRef = ChunkRef<DocumentVersionChunk>;
// /// Typed storage for document versions
// pub type DocumentVersionStorage = ChunkStorage<DocumentVersionChunk>;
