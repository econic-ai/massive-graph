//! Flat Storage - Multi-user storage management

use std::sync::Arc;
use crate::storage::{ZeroCopyDocumentStorage};
use crate::structures::optimised_index::{OptimisedIndex, Snapshot, MphIndexer, DeltaOp};
use crate::structures::segmented_stream::SegmentedStream;
use crate::types::{UserId, DocId};
use crate::storage::user_space::UserSpace;

/// Flat Storage - Maps users to their isolated storage instances
/// 
/// This struct manages multiple UserDocumentSpace instances, providing a flat
/// namespace where each user has their own isolated storage.
/// 
/// Generic parameter S allows compile-time selection of storage implementation
/// for zero-cost abstraction.
pub struct Store {
    /// Lock-free map of user ID to UserDocumentSpace instances
    user_spaces: OptimisedIndex<UserId, Arc<UserSpace>>, 
    // user_spaces: DashMap<UserId, Arc<UserSpace<S>>>,

}

impl Store {
    /// Create a new flat storage with a factory function for storage instances
    pub fn new() -> Self
    {
        // Minimal empty snapshot and delta stream to satisfy skeleton wiring
        struct DummyMph;
        impl MphIndexer<UserId> for DummyMph { fn eval(&self, _key: &UserId) -> usize { 0 } }
        let snapshot = Snapshot {
            version: 0,
            reserved_keys: Arc::from([]),
            reserved_vals: Arc::from([] as [Arc<Arc<UserSpace>>; 0]),
            mph_vals: Arc::from([] as [Arc<Arc<UserSpace>>; 0]),
            mph_indexer: crate::structures::optimised_index::ArcIndexer(Arc::new(DummyMph)),
        };
        let delta_stream = Arc::new(SegmentedStream::<DeltaOp<UserId, Arc<UserSpace>>>::new());
        Self { user_spaces: OptimisedIndex::new(snapshot, delta_stream) }
    }
    
    /// Get or create an isolate for a specific user
    fn get_or_create_user_space(&self, user_id: UserId) -> Arc<UserSpace> {
        self.user_spaces
            .get(&user_id)
            .unwrap_or_else(|| {
                Arc::new(Arc::new(UserSpace::new(user_id)))
            })
            .as_ref()
            .clone()
    }

    /// Get or create an isolate for a specific user
    fn get_user_space(&self, user_id: UserId) -> Arc<UserSpace> {
        self.user_spaces
            .get(&user_id).unwrap().as_ref().clone()
    }
    
    /// Get the number of active users
    pub fn user_count(&self) -> usize {
        self.user_spaces.len()
    }
    
    /// Get total document count across all users
    pub fn total_document_count(&self) -> usize {
        self.user_spaces
            .iter()
            .map(|entry| entry.1.document_count())
            .sum()
    }
    
    /// Get document count for a specific user
    pub fn user_document_count(&self, user_id: UserId) -> usize {
        self.get_user_space(user_id).document_count()
    }

    /// Register a new user
    pub fn get_or_crete_user_space(&self, user_id: UserId) -> Arc<UserSpace> {
        self.get_or_create_user_space(user_id)
    }
    
    /// Create a document for a specific user
    pub fn create_document(&self, user_id: UserId, doc_id: DocId, doc_data: Vec<u8>) -> Result<(), String> {
        self.get_user_space(user_id).create_document(doc_id, doc_data)
    }
    
    /// Get a document for a specific user
    pub fn get_document(&self, user_id: UserId, doc_id: DocId) -> Option<Arc<ZeroCopyDocumentStorage>> {
        self.get_user_space(user_id).get_document(doc_id)
    }
    
    /// Remove a document for a specific user
    pub fn remove_document(&self, user_id: UserId, doc_id: DocId) -> Result<(), String> {
        self.get_user_space(user_id).remove_document(doc_id);
        Ok(())
    }
    
    /// Check if a document exists for a specific user
    pub fn document_exists(&self, user_id: UserId, doc_id: DocId) -> bool {
        self.get_user_space(user_id).document_exists(doc_id)
    }
    
    /// Apply a delta to a document for a specific user
    pub fn apply_delta(&self, user_id: UserId, doc_id: DocId, delta: Vec<u8>) -> Result<(), String> {
        self.get_user_space(user_id).apply_delta(doc_id, delta)
    }

}

// Type aliases for common configurations
// / Type alias for Store with SimpleDocumentStorage
// pub type SimpleStore = Store<crate::storage::document_simple::SimpleDocumentStorage>;
// /// Type alias for Store with ZeroCopyStorage  
// pub type ZeroCopyStore = Store<crate::storage::document_storage::ZeroCopyDocumentStorage>;
