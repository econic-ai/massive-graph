//! User Isolate - Single user storage wrapper
use std::sync::atomic::AtomicU64;
use dashmap::DashMap;
use crate::types::{UserId, DocId};
use crate::storage::DocumentStorage;
use crate::types::storage::ChunkStorage;
use crate::types::storage::ChunkRef;
use crate::types::document::DocumentType;


struct UserStorageSpace {
    user_id: UserId,
    
    // Storage by data type
    documents: ChunkStorage,      // Headers only (no version reference)
    deltas: ChunkStorage,         // All deltas
    versions: ChunkStorage,       // Document versions (separate)
    
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

/// User Isolate - Represents a single user's isolated storage
/// 
/// This struct combines a user ID with a storage implementation,
/// providing a single-user view of the storage system.
/// 
/// Generic parameter S allows compile-time selection of storage implementation
/// for zero-cost abstraction.
pub struct UserDocumentSpace<S: DocumentStorage> {
    /// The user ID this isolate represents
    user_id: UserId,
    /// The storage implementation for this user
    storage: S,
}

impl<S: DocumentStorage> UserDocumentSpace<S> {
    /// Create a new user isolate for a specific user
    pub fn new(user_id: UserId, storage: S) -> Self {
        Self { user_id, storage }
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
        tracing::info!("🔒 UserDocumentSpace::create_document - user: {}, doc: {}, data_size: {}", self.user_id, doc_id, doc_data.len());
        let result = self.storage.create_document(doc_id, doc_data);
        match &result {
            Ok(()) => tracing::info!("✅ UserDocumentSpace storage successful"),
            Err(e) => tracing::error!("❌ UserDocumentSpace storage failed: {}", e),
        }
        result
    }
    
    /// Get a document for this user
    pub fn get_document(&self, doc_id: DocId) -> Option<Vec<u8>> {
        self.storage.get_document(doc_id)
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
    
    /// Add a child relationship between documents for this user
    pub fn add_child_relationship(&self, parent_id: DocId, child_id: DocId) -> Result<(), String> {
        self.storage.add_child_relationship(parent_id, child_id)
    }
    
    /// Remove a child relationship between documents for this user
    pub fn remove_child_relationship(&self, parent_id: DocId, child_id: DocId) -> Result<(), String> {
        self.storage.remove_child_relationship(parent_id, child_id)
    }
}

// Temporarily keep these type aliases for backward compatibility
// TODO: Remove these once Store is implemented
/// Type alias for UserDocumentSpace with SimpleStorage
pub type SimpleUserDocumentSpace = UserDocumentSpace<crate::storage::SimpleStorage>;
