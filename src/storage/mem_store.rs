/// In-memory document store with sequential heap layout and HashMap indexing.
/// 
/// This implementation uses a growing heap approach:
/// - Documents stored sequentially in memory segments
/// - HashMap provides O(1) lookup into heap positions
/// - Cache rebuilding means reconstructing HashMap from heap data
/// - Each root document tree can be rebuilt independently
/// 
/// Architecture:
/// - document_heap: Sequential storage of all document data
/// - header_index: HashMap pointing to header positions in heap
/// - data_index: HashMap pointing to data positions in heap
/// - root_documents: Set of root document IDs for tree operations
/// 
/// Performance characteristics:
/// - Reads: O(1) HashMap lookup into sequential memory
/// - Writes: O(1) append to heap + HashMap update
/// - Cache rebuild: O(n) scan of heap to reconstruct HashMap
/// - Memory: Optimal cache locality due to sequential layout

use crate::core::types::{ID16, Handle};
use crate::core::types::document::{Value, AdaptiveMap, DocumentHeader, Document, DocumentType, AppendOnlyStream, BloomFilter};
use crate::core::types::delta::{Delta, Operation};
use crate::storage::ZeroCopyDocumentStorage;
use crate::security::{UserID, PermissionSet};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU16, AtomicU32, AtomicU64, Ordering};
use dashmap::DashMap;

/// Trait for different heap storage strategies
pub trait HeapStorage {
    type Address: Copy + Clone + std::fmt::Debug;
    
    /// Append document with header and data to heap
    fn append_document(&mut self, header: &DocumentHeader, data: &[u8]) -> Result<Self::Address, String>;
    
    /// Get document header at address
    fn get_header(&self, addr: Self::Address) -> Option<&DocumentHeader>;
    
    /// Get document data at address  
    fn get_data(&self, addr: Self::Address) -> Option<&[u8]>;
    
    /// Rebuild index by scanning heap for all document headers
    fn rebuild_index(&self) -> HashMap<ID16, Self::Address>;
}

/// Heap address for chunked storage
#[derive(Copy, Clone, Debug)]
pub struct HeapAddress {
    pub chunk_id: u32,
    pub offset: u32,
    pub length: u32,
}

/// Chunked heap implementation for incremental growth
#[derive(Debug)]
pub struct ChunkedHeap {
    chunks: Vec<Vec<u8>>,
    chunk_size: usize,
    current_chunk: usize,
    current_offset: usize,
}

impl ChunkedHeap {
    pub fn new(chunk_size: usize) -> Self {
        let mut chunks = Vec::new();
        chunks.push(Vec::with_capacity(chunk_size));
        
        Self {
            chunks,
            chunk_size,
            current_chunk: 0,
            current_offset: 0,
        }
    }
    
    fn ensure_space(&mut self, needed: usize) -> Result<(), String> {
        if self.current_offset + needed > self.chunk_size {
            self.chunks.push(Vec::with_capacity(self.chunk_size));
            self.current_chunk += 1;
            self.current_offset = 0;
        }
        Ok(())
    }
}

impl HeapStorage for ChunkedHeap {
    type Address = HeapAddress;
    
    fn append_document(&mut self, header: &DocumentHeader, data: &[u8]) -> Result<Self::Address, String> {
        let total_size = 128 + data.len(); // 128-byte header + data
        if total_size > self.chunk_size {
            return Err("Document too large for single chunk".to_string());
        }
        
        self.ensure_space(total_size)?;
        
        let addr = HeapAddress {
            chunk_id: self.current_chunk as u32,
            offset: self.current_offset as u32,
            length: total_size as u32,
        };
        
        let chunk = &mut self.chunks[self.current_chunk];
        if chunk.len() < self.current_offset + total_size {
            chunk.resize(self.current_offset + total_size, 0);
        }
        
        // Write 128-byte header
        let header_bytes = unsafe {
            std::slice::from_raw_parts(header as *const DocumentHeader as *const u8, 128)
        };
        chunk[self.current_offset..self.current_offset + 128].copy_from_slice(header_bytes);
        
        // Write document data
        chunk[self.current_offset + 128..self.current_offset + total_size]
            .copy_from_slice(data);
        
        self.current_offset += total_size;
        Ok(addr)
    }
    
    fn get_header(&self, addr: Self::Address) -> Option<&DocumentHeader> {
        let chunk = self.chunks.get(addr.chunk_id as usize)?;
        let start = addr.offset as usize;
        
        if start + 128 <= chunk.len() {
            // Safe cast: DocumentHeader is repr(C, align(128))
            unsafe {
                Some(&*(chunk.as_ptr().add(start) as *const DocumentHeader))
            }
        } else {
            None
        }
    }
    
    fn get_data(&self, addr: Self::Address) -> Option<&[u8]> {
        let chunk = self.chunks.get(addr.chunk_id as usize)?;
        let header_start = addr.offset as usize;
        let data_start = header_start + 128;
        let total_end = header_start + addr.length as usize;
        
        if total_end <= chunk.len() && data_start < total_end {
            Some(&chunk[data_start..total_end])
        } else {
            None
        }
    }
    
    fn rebuild_index(&self) -> HashMap<ID16, Self::Address> {
        let mut index = HashMap::new();
        
        for (chunk_id, chunk) in self.chunks.iter().enumerate() {
            let mut offset = 0;
            
            while offset + 128 <= chunk.len() {
                // Read header
                let header = unsafe {
                    &*(chunk.as_ptr().add(offset) as *const DocumentHeader)
                };
                
                // Get data size from header
                let data_size = header.data_size.load(Ordering::Acquire) as usize;
                let total_size = 128 + data_size;
                
                // Verify we have complete document
                if offset + total_size <= chunk.len() {
                    let addr = HeapAddress {
                        chunk_id: chunk_id as u32,
                        offset: offset as u32,
                        length: total_size as u32,
                    };
                    index.insert(header.id, addr);
                    offset += total_size;
                } else {
                    break; // Incomplete document, end of valid data
                }
            }
        }
        
        index
    }
}

/// Simple heap for testing/development
#[derive(Debug)]
pub struct SimpleHeap {
    data: Vec<u8>,
}

impl SimpleHeap {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }
}

impl HeapStorage for SimpleHeap {
    type Address = (usize, usize); // (offset, total_length)
    
    fn append_document(&mut self, header: &DocumentHeader, data: &[u8]) -> Result<Self::Address, String> {
        let offset = self.data.len();
        let total_size = 128 + data.len();
        
        // Append 128-byte header
        let header_bytes = unsafe {
            std::slice::from_raw_parts(header as *const DocumentHeader as *const u8, 128)
        };
        self.data.extend_from_slice(header_bytes);
        
        // Append document data
        self.data.extend_from_slice(data);
        
        Ok((offset, total_size))
    }
    
    fn get_header(&self, addr: Self::Address) -> Option<&DocumentHeader> {
        let (offset, _length) = addr;
        if offset + 128 <= self.data.len() {
            unsafe {
                Some(&*(self.data.as_ptr().add(offset) as *const DocumentHeader))
            }
        } else {
            None
        }
    }
    
    fn get_data(&self, addr: Self::Address) -> Option<&[u8]> {
        let (offset, total_length) = addr;
        let data_start = offset + 128;
        let data_end = offset + total_length;
        
        if data_end <= self.data.len() && data_start < data_end {
            Some(&self.data[data_start..data_end])
        } else {
            None
        }
    }
    
    fn rebuild_index(&self) -> HashMap<ID16, Self::Address> {
        let mut index = HashMap::new();
        let mut offset = 0;
        
        while offset + 128 <= self.data.len() {
            // Read header
            let header = unsafe {
                &*(self.data.as_ptr().add(offset) as *const DocumentHeader)
            };
            
            // Get data size from header
            let data_size = header.data_size.load(Ordering::Acquire) as usize;
            let total_size = 128 + data_size;
            
            // Verify we have complete document
            if offset + total_size <= self.data.len() {
                let addr = (offset, total_size);
                index.insert(header.id, addr);
                offset += total_size;
            } else {
                break; // Incomplete document, end of valid data
            }
        }
        
        index
    }
}

/// Per-user document heap with configurable storage strategy
#[derive(Debug)]
pub struct RootDocumentHeap<H: HeapStorage> {
    header_heap: H,
    data_heap: H,
    delta_heap: H,
    
    header_index: HashMap<ID16, H::Address>,
    data_index: HashMap<ID16, H::Address>,
    
    owner_id: UserID,
    cache_dirty: AtomicBool,
}

impl<H: HeapStorage> RootDocumentHeap<H> {
    pub fn new(owner_id: UserID, header_heap: H, data_heap: H, delta_heap: H) -> Self {
        Self {
            header_heap,
            data_heap,
            delta_heap,
            header_index: HashMap::new(),
            data_index: HashMap::new(),
            owner_id,
            cache_dirty: AtomicBool::new(false),
        }
    }
    
    /// Create a new document in the heap
    pub fn create_document(&mut self, header: DocumentHeader, data: &[u8]) -> Result<(), String> {
        let addr = self.data_heap.append_document(&header, data)?;
        self.data_index.insert(header.id, addr);
        Ok(())
    }
    
    /// Get document data by ID
    pub fn get_data_from_heap(&self, id: &ID16) -> Option<&[u8]> {
        let addr = self.data_index.get(id)?;
        self.data_heap.get_data(*addr)
    }
    
    /// Get document header by ID
    pub fn get_header_from_heap(&self, id: &ID16) -> Option<&DocumentHeader> {
        let addr = self.data_index.get(id)?;
        self.data_heap.get_header(*addr)
    }
    
    /// Rebuild indexes from heap data (for corruption recovery)
    pub fn rebuild_indexes(&mut self) {
        self.data_index = self.data_heap.rebuild_index();
        self.clear_cache_dirty();
    }
    
    pub fn mark_cache_dirty(&self) {
        self.cache_dirty.store(true, Ordering::Release);
    }
    
    pub fn is_cache_dirty(&self) -> bool {
        self.cache_dirty.load(Ordering::Acquire)
    }
    
    pub fn clear_cache_dirty(&self) {
        self.cache_dirty.store(false, Ordering::Release);
    }
}

// Type aliases for different heap strategies
pub type ChunkedDocumentHeap = RootDocumentHeap<ChunkedHeap>;
pub type SimpleDocumentHeap = RootDocumentHeap<SimpleHeap>;

/// In-memory document store with per-user heap isolation.
/// 
/// Each user gets their own RootDocumentHeap for security isolation and performance.
/// Documents are stored sequentially in heaps with HashMap indexes for O(1) lookup.
/// Cache corruption triggers per-user heap rebuilding rather than global rebuilds.
#[derive(Debug)]
pub struct MemStore {
    /// Per-user document heaps for security isolation
    user_heaps: DashMap<UserID, ChunkedDocumentHeap>,
    
    /// Global root document registry (documents that are entry points)
    root_documents: DashMap<ID16, UserID>, // document_id -> owner_user_id
    
    /// Handle-based pools for variable-sized data (stable references)
    string_pool: DashMap<Handle, Box<String>>,
    binary_stream_pool: DashMap<Handle, Box<AppendOnlyStream<Vec<u8>>>>,
    text_stream_pool: DashMap<Handle, Box<AppendOnlyStream<String>>>,
    doc_stream_pool: DashMap<Handle, Box<AppendOnlyStream<ID16>>>,
    
    /// Handle generation (atomic counter)
    next_handle: AtomicU64,
}

impl MemStore {
    /// Create new MemStore with handle-based pools
    pub fn new() -> Self {
        Self {
            user_heaps: DashMap::new(),
            root_documents: DashMap::new(),
            string_pool: DashMap::new(),
            binary_stream_pool: DashMap::new(),
            text_stream_pool: DashMap::new(),
            doc_stream_pool: DashMap::new(),
            next_handle: AtomicU64::new(1), // Start at 1, 0 reserved for null handle
        }
    }
    
    /// Get or create heap for a user
    pub fn get_or_create_user_heap(&self, user_id: UserID) -> dashmap::mapref::one::RefMut<'_, UserID, ChunkedDocumentHeap> {
        self.user_heaps.entry(user_id).or_insert_with(|| {
            let chunk_size = 64 * 1024 * 1024; // 64MB chunks
            ChunkedDocumentHeap::new(
                user_id,
                ChunkedHeap::new(chunk_size),
                ChunkedHeap::new(chunk_size),
                ChunkedHeap::new(chunk_size),
            )
        })
    }
    
    /// Get heap for a user (read-only)
    pub fn get_user_heap(&self, user_id: &UserID) -> Option<dashmap::mapref::one::Ref<'_, UserID, ChunkedDocumentHeap>> {
        self.user_heaps.get(user_id)
    }
    
    /// Check if user has any documents
    pub fn user_has_documents(&self, user_id: &UserID) -> bool {
        if let Some(heap) = self.user_heaps.get(user_id) {
            !heap.data_index.is_empty()
        } else {
            false
        }
    }
    
    /// Get total document count across all users
    pub fn total_document_count(&self) -> usize {
        self.user_heaps.iter().map(|entry| entry.value().data_index.len()).sum()
    }
    
    /// Get user count
    pub fn user_count(&self) -> usize {
        self.user_heaps.len()
    }
    
    /// Register a root document
    pub fn register_root_document(&self, document_id: ID16, owner_id: UserID) -> Result<(), String> {
        if self.root_documents.contains_key(&document_id) {
            return Err("Root document already exists".to_string());
        }
        
        self.root_documents.insert(document_id, owner_id);
        Ok(())
    }
    
    /// Get root document owner
    pub fn get_root_document_owner(&self, document_id: &ID16) -> Option<UserID> {
        self.root_documents.get(document_id).map(|entry| *entry.value())
    }
    
    /// Check if document is a root document
    pub fn is_root_document(&self, document_id: &ID16) -> bool {
        self.root_documents.contains_key(document_id)
    }
    
    /// Get all root documents for a user
    pub fn get_user_root_documents(&self, user_id: &UserID) -> Vec<ID16> {
        self.root_documents
            .iter()
            .filter(|entry| entry.value() == user_id)
            .map(|entry| *entry.key())
            .collect()
    }
    
    /// Generate a new unique handle
    pub fn new_handle(&self) -> Handle {
        Handle::new(self.next_handle.fetch_add(1, Ordering::AcqRel))
    }
    
    /// Store string and return handle
    pub fn store_string(&self, content: String) -> Handle {
        let handle = self.new_handle();
        self.string_pool.insert(handle, Box::new(content));
        handle
    }
    
    /// Get string by handle
    pub fn get_string(&self, handle: Handle) -> Option<String> {
        self.string_pool.get(&handle).map(|boxed| (**boxed).clone())
    }
    
    /// Store binary stream and return handle
    pub fn store_binary_stream(&self, stream: AppendOnlyStream<Vec<u8>>) -> Handle {
        let handle = self.new_handle();
        self.binary_stream_pool.insert(handle, Box::new(stream));
        handle
    }
    
    /// Get binary stream by handle (read-only reference)
    pub fn get_binary_stream(&self, handle: Handle) -> Option<dashmap::mapref::one::Ref<'_, Handle, Box<AppendOnlyStream<Vec<u8>>>>> {
        self.binary_stream_pool.get(&handle)
    }
    
    /// Get mutable binary stream by handle for appending
    pub fn get_binary_stream_mut(&self, handle: Handle) -> Option<dashmap::mapref::one::RefMut<'_, Handle, Box<AppendOnlyStream<Vec<u8>>>>> {
        self.binary_stream_pool.get_mut(&handle)
    }
    
    /// Store text stream and return handle
    pub fn store_text_stream(&self, stream: AppendOnlyStream<String>) -> Handle {
        let handle = self.new_handle();
        self.text_stream_pool.insert(handle, Box::new(stream));
        handle
    }
    
    /// Get text stream by handle (read-only reference)
    pub fn get_text_stream(&self, handle: Handle) -> Option<dashmap::mapref::one::Ref<'_, Handle, Box<AppendOnlyStream<String>>>> {
        self.text_stream_pool.get(&handle)
    }
    
    /// Get mutable text stream by handle for appending
    pub fn get_text_stream_mut(&self, handle: Handle) -> Option<dashmap::mapref::one::RefMut<'_, Handle, Box<AppendOnlyStream<String>>>> {
        self.text_stream_pool.get_mut(&handle)
    }
    
    /// Store document stream and return handle
    pub fn store_doc_stream(&self, stream: AppendOnlyStream<ID16>) -> Handle {
        let handle = self.new_handle();
        self.doc_stream_pool.insert(handle, Box::new(stream));
        handle
    }
    
    /// Get document stream by handle (read-only reference)
    pub fn get_doc_stream(&self, handle: Handle) -> Option<dashmap::mapref::one::Ref<'_, Handle, Box<AppendOnlyStream<ID16>>>> {
        self.doc_stream_pool.get(&handle)
    }
    
    /// Get mutable document stream by handle for appending
    pub fn get_doc_stream_mut(&self, handle: Handle) -> Option<dashmap::mapref::one::RefMut<'_, Handle, Box<AppendOnlyStream<ID16>>>> {
        self.doc_stream_pool.get_mut(&handle)
    }
    
    /// Remove string from pool
    pub fn remove_string(&self, handle: Handle) -> Option<Box<String>> {
        self.string_pool.remove(&handle).map(|(_, boxed)| boxed)
    }
    
    /// Remove stream from pool
    pub fn remove_binary_stream(&self, handle: Handle) -> Option<Box<AppendOnlyStream<Vec<u8>>>> {
        self.binary_stream_pool.remove(&handle).map(|(_, boxed)| boxed)
    }
    
    /// Remove text stream from pool
    pub fn remove_text_stream(&self, handle: Handle) -> Option<Box<AppendOnlyStream<String>>> {
        self.text_stream_pool.remove(&handle).map(|(_, boxed)| boxed)
    }
    
    /// Remove document stream from pool
    pub fn remove_doc_stream(&self, handle: Handle) -> Option<Box<AppendOnlyStream<ID16>>> {
        self.doc_stream_pool.remove(&handle).map(|(_, boxed)| boxed)
    }
    
    /// Get current timestamp
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}

impl ZeroCopyDocumentStorage for MemStore {
    fn get_document(&self, id: &ID16) -> Option<&Document<'_>> {
        // TODO: Implement heap-based document lookup
        // Need to determine which user owns this document first
        todo!("Implement heap-based document lookup")
    }
    
    fn get_document_header(&self, id: &ID16) -> Option<&DocumentHeader> {
        // TODO: Find user heap and get header
        todo!("Implement header lookup across user heaps")
    }
    
    fn create_document(
        &self,
        id: ID16,
        doc_type: DocumentType,
        parent_id: ID16,
        properties: &AdaptiveMap<String, Value>,
    ) -> Result<(), String> {
        // TODO: Determine user context and create in appropriate heap
        todo!("Implement document creation in user heap")
    }
    
    fn update_property(&self, id: &ID16, property: &str, value: &Value) -> Result<(), String> {
        // TODO: Find document and update property
        todo!("Implement property updates")
    }
    
    fn remove_document(&self, id: &ID16) -> Result<(), String> {
        // TODO: Remove from appropriate user heap
        todo!("Implement document removal")
    }
    
    fn document_exists(&self, id: &ID16) -> bool {
        // TODO: Check across all user heaps
        todo!("Implement document existence check")
    }
    
    fn document_count(&self) -> usize {
        self.total_document_count()
    }
    
    fn add_child_relationship(&self, parent_id: ID16, child_id: ID16) -> Result<(), String> {
        // TODO: Update parent document's children
        todo!("Implement child relationship addition")
    }
    
    fn remove_child_relationship(&self, parent_id: ID16, child_id: ID16) -> Result<(), String> {
        // TODO: Remove from parent document's children
        todo!("Implement child relationship removal")
    }
    
    fn create_root_document(
        &self,
        id: ID16,
        name: String,
        description: String,
    ) -> Result<(), String> {
        // TODO: Determine user context - this needs user_id parameter
        // For now, create a placeholder implementation
        todo!("Implement root document creation - needs user context")
    }
    
    fn get_root_documents(&self) -> Vec<ID16> {
        self.root_documents.iter().map(|entry| *entry.key()).collect()
    }
    
    fn find_root_for_document(&self, _document_id: &ID16) -> Option<ID16> {
        // TODO: Traverse up parent chain to find root
        todo!("Implement root document discovery")
    }
}

/// Get current timestamp in nanoseconds since epoch
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
} 