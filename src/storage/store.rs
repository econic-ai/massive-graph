//! Flat Storage - Multi-user storage management

use dashmap::DashMap;
use std::sync::Arc;
use crate::types::{UserId, DocId};
use crate::storage::DocumentStorage;
use crate::storage::user_space::UserDocumentSpace;

/// Flat Storage - Maps users to their isolated storage instances
/// 
/// This struct manages multiple UserDocumentSpace instances, providing a flat
/// namespace where each user has their own isolated storage.
/// 
/// Generic parameter S allows compile-time selection of storage implementation
/// for zero-cost abstraction.
pub struct Store<S: DocumentStorage + Clone + Send + Sync + 'static> {
    /// Lock-free map of user ID to UserDocumentSpace instances
    user_spaces: DashMap<UserId, Arc<UserDocumentSpace<S>>>,
    /// Function to create new storage instances
    storage_factory: Arc<dyn Fn() -> S + Send + Sync>,
}

impl<S: DocumentStorage + Clone + Send + Sync + 'static> Store<S> {
    /// Create a new flat storage with a factory function for storage instances
    pub fn new<F>(storage_factory: F) -> Self
    where
        F: Fn() -> S + Send + Sync + 'static,
    {
        Self {
            user_spaces: DashMap::new(),
            storage_factory: Arc::new(storage_factory),
        }
    }
    
    /// Get or create an isolate for a specific user
    fn get_or_create_isolate(&self, user_id: UserId) -> Arc<UserDocumentSpace<S>> {
        self.user_spaces
            .entry(user_id)
            .or_insert_with(|| {
                let storage = (self.storage_factory)();
                Arc::new(UserDocumentSpace::new(user_id, storage))
            })
            .clone()
    }
    
    /// Get the number of active users
    pub fn user_count(&self) -> usize {
        self.user_spaces.len()
    }
    
    /// Get total document count across all users
    pub fn total_document_count(&self) -> usize {
        self.user_spaces
            .iter()
            .map(|entry| entry.value().document_count())
            .sum()
    }
    
    /// Get document count for a specific user
    pub fn user_document_count(&self, user_id: UserId) -> usize {
        self.get_or_create_isolate(user_id).document_count()
    }
    
    /// Create a document for a specific user
    pub fn create_document(&self, user_id: UserId, doc_id: DocId, doc_data: Vec<u8>) -> Result<(), String> {
        self.get_or_create_isolate(user_id).create_document(doc_id, doc_data)
    }
    
    /// Get a document for a specific user
    pub fn get_document(&self, user_id: UserId, doc_id: DocId) -> Option<Vec<u8>> {
        self.get_or_create_isolate(user_id).get_document(doc_id)
    }
    
    /// Remove a document for a specific user
    pub fn remove_document(&self, user_id: UserId, doc_id: DocId) -> Result<(), String> {
        self.get_or_create_isolate(user_id).remove_document(doc_id)
    }
    
    /// Check if a document exists for a specific user
    pub fn document_exists(&self, user_id: UserId, doc_id: DocId) -> bool {
        self.get_or_create_isolate(user_id).document_exists(doc_id)
    }
    
    /// Apply a delta to a document for a specific user
    pub fn apply_delta(&self, user_id: UserId, doc_id: DocId, delta: Vec<u8>) -> Result<(), String> {
        self.get_or_create_isolate(user_id).apply_delta(doc_id, delta)
    }
    
    /// Add a child relationship between documents for a specific user
    pub fn add_child_relationship(&self, user_id: UserId, parent_id: DocId, child_id: DocId) -> Result<(), String> {
        self.get_or_create_isolate(user_id).add_child_relationship(parent_id, child_id)
    }
    
    /// Remove a child relationship between documents for a specific user
    pub fn remove_child_relationship(&self, user_id: UserId, parent_id: DocId, child_id: DocId) -> Result<(), String> {
        self.get_or_create_isolate(user_id).remove_child_relationship(parent_id, child_id)
    }
}

// Type aliases for common configurations
/// Type alias for Store with SimpleStorage
pub type SimpleStore = Store<crate::storage::mem_simple::SimpleStorage>;
/// Type alias for Store with ZeroCopyStorage  
pub type ZeroCopyStore = Store<crate::storage::mem_advanced::ZeroCopyStorage>;
