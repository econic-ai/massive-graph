//! Storage layer for Massive Graph Database
//! 
//! This module provides the storage abstraction layer that enables different
//! storage backends while maintaining consistent APIs for document operations.


// use crate::types::{DocId};

/// Trait for document storage implementations
pub trait DocumentStorage: Send + Sync {
    /// Get a document by ID
    /// Returns document data as bytes that can be interpreted by the implementation
    fn get_document(&self) -> Option<Vec<u8>>;
    
    /// Apply a delta to a document
    fn apply_delta(&self, delta: Vec<u8>) -> Result<(), String>;
    
    /// Create a new document
    fn create_document(&self) -> Result<(), String>;
    
    /// Remove a document
    fn delete_document(&self) -> Result<(), String>;
    
    
}


/// Zero-Copy Store (high-performance implementation)
pub mod document_storage;

/// Simple Store (JSON-based implementation)
pub mod document_simple;

/// User Isolate (wrapper for user isolation)
pub mod user_space;

/// Flat Storage (multi-user storage management)
pub mod store;

/// Re-export main storage types
pub use document_storage::{ZeroCopyDocumentStorage};
pub use document_simple::SimpleDocumentStorage;
pub use user_space::UserSpace;
pub use store::{Store};

/// Helper trait that combines all requirements for storage implementations
/// This cleans up generic bounds throughout the codebase
pub trait StorageImpl: DocumentStorage + Send + Sync + 'static {}

/// Blanket implementation for any type that meets the requirements
impl<T> StorageImpl for T where T: DocumentStorage + Send + Sync + 'static {}