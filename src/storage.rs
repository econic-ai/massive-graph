//! Storage layer for Massive Graph Database
//! 
//! This module provides the storage abstraction layer that enables different
//! storage backends while maintaining consistent APIs for document operations.
//! 
//! The architecture follows the principle of user isolation, where each user
//! has their own document space to ensure security and eliminate concurrency
//! bottlenecks between different users.
//!
//! 🚨 ARCHITECTURE COMPLIANCE: All changes to this module must follow principles
//! defined in ARCHITECTURE_PRINCIPLES.md. Key requirements:
//! - User isolation first (no cross-user locks)
//! - Vec<u8> interface for storage flexibility
//! - Lock-free patterns preferred over Mutex

use crate::types::{DocId};

/// Trait for document storage implementations
/// 
/// This trait provides the interface for document storage operations.
/// Storage implementations are user-agnostic - user isolation is handled
/// at a higher level by UserDocumentSpace and Store.
/// 
/// All document data is handled as bytes to allow different implementations
/// to use different serialization formats (JSON, binary protocols, etc.)
pub trait DocumentStorage: Send + Sync {
    /// Get a document by ID
    /// Returns document data as bytes that can be interpreted by the implementation
    fn get_document(&self, doc_id: DocId) -> Option<Vec<u8>>;
    
    /// Apply a delta to a document
    /// 
    /// # Arguments
    /// 
    /// * `doc_id` - The document identifier
    /// * `delta` - The delta operations to apply as bytes
    /// 
    /// # Returns
    /// 
    /// * `Ok(())` - If the delta was applied successfully
    /// * `Err(String)` - If the delta application failed
    fn apply_delta(&self, doc_id: DocId, delta: Vec<u8>) -> Result<(), String>;
    
    /// Create a new document
    /// 
    /// # Arguments
    /// 

    /// * `doc_id` - The document identifier
    /// * `doc_data` - The document data as bytes (implementation-specific format)
    /// 
    /// # Returns
    /// 
    /// * `Ok(())` - If the document was created successfully
    /// * `Err(String)` - If document creation failed
    fn create_document(&self, doc_id: DocId, doc_data: Vec<u8>) -> Result<(), String>;
    
    /// Remove a document
    /// 
    /// # Arguments
    /// 
    /// * `doc_id` - The document identifier
    /// 
    /// # Returns
    /// 
    /// * `Ok(())` - If the document was removed successfully
    /// * `Err(String)` - If document removal failed
    fn remove_document(&self, doc_id: DocId) -> Result<(), String>;
    
    /// Check if a document exists
    /// 
    /// # Arguments
    /// 

    /// * `doc_id` - The document identifier
    /// 
    /// # Returns
    /// 
    /// * `true` - If the document exists
    /// * `false` - If the document doesn't exist
    fn document_exists(&self, doc_id: DocId) -> bool;
    
    /// Add a child relationship between documents
    /// 
    /// # Arguments
    /// 

    /// * `parent_id` - The parent document identifier
    /// * `child_id` - The child document identifier
    /// 
    /// # Returns
    /// 
    /// * `Ok(())` - If the relationship was added successfully
    /// * `Err(String)` - If adding the relationship failed
    fn add_child_relationship(&self, parent_id: DocId, child_id: DocId) -> Result<(), String>;
    
    /// Remove a child relationship between documents
    /// 
    /// # Arguments
    /// 

    /// * `parent_id` - The parent document identifier
    /// * `child_id` - The child document identifier
    /// 
    /// # Returns
    /// 
    /// * `Ok(())` - If the relationship was removed successfully
    /// * `Err(String)` - If removing the relationship failed
    fn remove_child_relationship(&self, parent_id: DocId, child_id: DocId) -> Result<(), String>;
    
    /// Get the total number of documents
    /// 
    /// # Returns
    /// 
    /// The total number of documents in storage
    fn document_count(&self) -> usize;
    
}


/// Zero-Copy Store (high-performance implementation)
pub mod mem_advanced;

/// Simple Store (JSON-based implementation)
pub mod mem_simple;

/// User Isolate (wrapper for user isolation)
pub mod user_space;

/// Flat Storage (multi-user storage management)
pub mod store;

/// Re-export main storage types
pub use mem_advanced::{ZeroCopyStorage, UserDocuments};
pub use mem_simple::SimpleStorage;
pub use user_space::{UserDocumentSpace, SimpleUserDocumentSpace};
pub use store::{Store, SimpleStore, ZeroCopyStore};

/// Helper trait that combines all requirements for storage implementations
/// This cleans up generic bounds throughout the codebase
pub trait StorageImpl: DocumentStorage + Clone + Send + Sync + 'static {}

/// Blanket implementation for any type that meets the requirements
impl<T> StorageImpl for T where T: DocumentStorage + Clone + Send + Sync + 'static {}