use tracing::info;

use crate::core::types::{ID16, ID32, DocumentType};
use crate::core::types::document::{Value, AdaptiveMap, DocumentHeader, Document};
use crate::storage::heap::{RootDocumentHeap, DeltaHeap};
use crate::storage::DocumentStorage;

/// UserID is an alias for ID32
pub type UserID = ID32;

/// In-memory document store for a single user, with strict isolation.
pub struct MemStore {
    owner_id: UserID,
    document_heap: RootDocumentHeap,
    delta_heap: DeltaHeap,
}

impl MemStore {
    /// Create a new MemStore for a single user
    pub fn new(owner_id: UserID) -> Self {
        info!("Creating MemStore for user: {}", owner_id);
        Self {
            owner_id,
            document_heap: RootDocumentHeap::new(owner_id),
            delta_heap: DeltaHeap::new(),
        }
    }
    
    /// Get the owner (user id) of this store
    pub fn get_owner(&self) -> UserID {
        self.owner_id
    }

    /// Get the total number of documents in this store
    pub fn document_count(&self) -> usize {
        self.document_heap.document_count()
    }

    // Add document/data methods as needed, all scoped to this user
    // ...

    // Add delta methods as needed, all scoped to this user
    // ...
}

impl DocumentStorage for MemStore {
    /// Get the total number of documents in storage
    fn document_count(&self) -> usize {
        self.document_heap.document_count()
    }
    
    /// Get a document by ID
    fn get_document(&self, _id: &ID16) -> Option<&AdaptiveMap<String, Value>> {
        // TODO: Implement document retrieval from heap
        None
    }
    
    /// Get a mutable reference to a document by ID
    fn get_document_mut(&mut self, _id: &ID16) -> Option<&mut AdaptiveMap<String, Value>> {
        // TODO: Implement mutable document retrieval from heap
        None
    }
    
    /// Create a new document
    fn create_document(&mut self, _id: ID16, _properties: AdaptiveMap<String, Value>) -> Result<(), String> {
        // TODO: Implement document creation in heap
        Err("Document creation not yet implemented".to_string())
    }
    
    /// Remove a document
    fn remove_document(&mut self, _id: &ID16) -> Result<(), String> {
        // TODO: Implement document removal from heap
        Err("Document removal not yet implemented".to_string())
    }
    
    /// Check if a document exists
    fn document_exists(&self, _id: &ID16) -> bool {
        // TODO: Implement document existence check
        false
    }
    
    /// Add a child relationship
    fn add_child_relationship(&mut self, _parent_id: ID16, _child_id: ID16) -> Result<(), String> {
        // TODO: Implement child relationship management
        Err("Child relationships not yet implemented".to_string())
    }
    
    /// Remove a child relationship
    fn remove_child_relationship(&mut self, _parent_id: ID16, _child_id: ID16) -> Result<(), String> {
        // TODO: Implement child relationship removal
        Err("Child relationship removal not yet implemented".to_string())
    }
} 