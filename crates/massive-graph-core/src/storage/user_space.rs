//! UserSpace - Single user storage, indexes, and streams
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use dashmap::DashMap;
use crate::types::document::DocumentView;
use crate::types::{UserId, DocId};
use crate::types::user::{UserDocNode, UserDocStream, UserDocumentRef, UserView};
use crate::DocumentStorage;
use crate::types::storage::ChunkStorage;
use crate::types::storage::ChunkRef;
// use crate::{log_info, log_error};
// use crate::core::logging::logging::{log_info, log_error};
use crate::{log_info, log_error};
// Final index API (to be implemented under core/structures/optimised_index)
// #[allow(unused_imports)]
// use crate::structures::optimised_index::{OptimisedIndex, OptimisedIndexStats};


/// use info; // TODO: Will be used when logging is implemented


#[allow(dead_code)] // POC: Struct will be used in future implementation
struct UserStorageSpace {
    user_id: UserId,
    
    // Storage by data type
    documents: ChunkStorage,      // Headers only (no version reference)
    deltas: ChunkStorage,         // All deltas
    snapshots: ChunkStorage,     // Document versions (separate)
    versions: ChunkStorage,     // Document versions (separate)
    
    // Indexes
    // indexes: [DashMap<DocId, ChunkRef>; DocumentType::MAX_TYPES],
    indexes: [DashMap<DocId, ChunkRef>; 12],
    
    // Version management
    // version_index: VersionIndex,
    
    // Metadata
    created_at: u64,
    total_bytes_used: AtomicU64,
    quota_bytes: u64,
}

/// UserSpace - per-user storage, document index, and document streams
/// Holds the user's document lookup index and document append-only stream with cursor.
pub struct UserSpace<S: DocumentStorage> {
    /// The user ID this isolate represents
    user_id: UserId,
    /// The storage implementation for this user
    storage: S,
    /// Append-only stream of this user's documents (payload uses 'static empty bytes for now).
    user_docs_stream: UserDocStream<'static>,
    /// Runtime view with cursor for batched traversal.
    user_view: UserView<'static>,
    // Per-user document lookup index (final API shape; wired once implemented)
    doc_index: OptimisedIndex<u128, Arc<DocumentView<'static>>>,
}

impl<S: DocumentStorage> UserSpace<S> {
    /// Create a new user space for a specific user
    pub fn new(user_id: UserId, storage: S) -> Self {
        // Create a sentinel head node for the user's document stream.
        let sentinel: *mut UserDocNode<'static> = UserDocNode::boxed(UserDocumentRef { bytes: &[], doc_id: DocId::default() });
        let user_docs_stream = UserDocStream::new(sentinel);
        let user_view = UserView::new(user_id, sentinel);
        Self { user_id, storage, user_docs_stream, user_view }
    }
    
    /// Get the user ID this isolate represents
    pub fn user_id(&self) -> UserId {
        self.user_id
    }
    
    /// Get document count for this user
    pub fn document_count(&self) -> usize {
        self.storage.document_count()
    }
    
    /// Create a document for this user
    pub fn create_document(&self, doc_id: DocId, doc_data: Vec<u8>) -> Result<(), String> {
        log_info!("🔒 UserDocumentSpace::create_document - user: {}, doc: {}, data_size: {}", self.user_id, doc_id, doc_data.len());
        let result = self.storage.create_document(doc_id, doc_data);
        match &result {
            Ok(()) => log_info!("✅ UserDocumentSpace storage successful"),
            Err(e) => log_error!("❌ UserDocumentSpace storage failed: {}", e),
        }
        if result.is_ok() {
            // Append the new document to the user's append-only stream (payload has static empty bytes for now).
            let node = UserDocNode::boxed(UserDocumentRef { bytes: &[], doc_id });
            self.user_docs_stream.append(node);
        }
        result
    }
    
    /// Get a document for this user
    pub fn get_document(&self, doc_id: DocId) -> Option<Vec<u8>> {
        // self.storage.get_document(doc_id)
        self.doc_index.get(&doc_id)
    }
    
    /// Remove a document for this user
    pub fn remove_document(&self, doc_id: DocId) -> Result<(), String> {
        self.storage.remove_document(doc_id)
    }
    
    /// Check if a document exists for this user
    pub fn document_exists(&self, doc_id: DocId) -> bool {
        self.storage.document_exists(doc_id)
    }
    
    /// Apply a delta to a document for this user
    pub fn apply_delta(&self, doc_id: DocId, delta: Vec<u8>) -> Result<(), String> {
        self.storage.apply_delta(doc_id, delta)
    }

    /// Build next batch of user documents into an existing vector to reuse capacity; advances the internal cursor.
    pub fn build_next_user_docs_into(&self, max_scan: usize, out: &mut Vec<*mut UserDocNode<'static>>) {
        self.user_view.build_next_user_docs_into(&self.user_docs_stream, max_scan, out);
    }
    

}

