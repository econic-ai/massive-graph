//! Flat Storage - Multi-user storage management

use crate::storage::{ZeroCopyDocumentStorage};
use crate::structures::mph_delta_index::{OptimisedIndexGen, mph_indexer::MphIndexer};
use crate::types::{UserId, DocId};
use crate::storage::user_space::UserSpace;
use std::sync::Arc;

/// Dummy MPH indexer for placeholder wiring (always returns slot 0).
#[derive(Clone)]
struct DummyMph;
impl MphIndexer<UserId> for DummyMph { 
    fn eval(&self, _key: &UserId) -> usize { 0 }
    fn build(_keys: &[UserId]) -> Self { DummyMph }
}

/// Flat Storage - Maps users to their isolated storage instances
/// 
/// This struct manages multiple UserDocumentSpace instances, providing a flat
/// namespace where each user has their own isolated storage.
/// 
/// Generic parameter S allows compile-time selection of storage implementation
/// for zero-cost abstraction.
pub struct Store {
    /// Lock-free map of user ID to UserDocumentSpace instances
    user_spaces: OptimisedIndexGen<UserId, Arc<UserSpace>, DummyMph>, 
    // user_spaces: DashMap<UserId, Arc<UserSpace<S>>>,

}

impl Store {
    /// Create a new flat storage with a factory function for storage instances
    pub fn new() -> Self
    {
        Self { user_spaces: OptimisedIndexGen::new_with_indexer_and_capacity(DummyMph, 4096, 8192) }
    }
    
    /// Get or create an isolate for a specific user
    fn get_or_create_user_space(&self, user_id: UserId) -> Arc<UserSpace> {
        self.user_spaces
            .get_owned(&user_id)
            .unwrap_or_else(|| Arc::new(UserSpace::new(user_id)))
    }

    /// Get or create an isolate for a specific user
    fn get_user_space(&self, user_id: UserId) -> Arc<UserSpace> {
        self.user_spaces
            .get_owned(&user_id).map(|v| v.clone()).unwrap()
    }
    
    /// Get the number of active users
    pub fn user_count(&self) -> usize {
        self.user_spaces.len()
    }
    
    /// Get total document count across all users
    pub fn total_document_count(&self) -> usize {
        0
        // self.user_spaces
        //     .mph_index.slots.len()
        //     .iter()
        //     .map(|entry| entry.1.document_count())
        //     .sum()
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
