//! Storage layer for Massive Graph Database
//! 
//! This module provides the storage abstraction layer that enables different
//! storage backends while maintaining consistent APIs for document operations.

use crate::core::types::{ID16, document::{DocumentHeader, Document, DocumentType, Value, AdaptiveMap}};

/// Trait for document storage implementations
pub trait DocumentStorage {
    /// Get the total number of documents in storage
    fn document_count(&self) -> usize;
    
    /// Get a document by ID
    fn get_document(&self, id: &ID16) -> Option<&AdaptiveMap<String, Value>>;
    
    /// Get a mutable reference to a document by ID
    fn get_document_mut(&mut self, id: &ID16) -> Option<&mut AdaptiveMap<String, Value>>;
    
    /// Create a new document
    fn create_document(&mut self, id: ID16, properties: AdaptiveMap<String, Value>) -> Result<(), String>;
    
    /// Remove a document
    fn remove_document(&mut self, id: &ID16) -> Result<(), String>;
    
    /// Check if a document exists
    fn document_exists(&self, id: &ID16) -> bool;
    
    /// Add a child relationship
    fn add_child_relationship(&mut self, parent_id: ID16, child_id: ID16) -> Result<(), String>;
    
    /// Remove a child relationship
    fn remove_child_relationship(&mut self, parent_id: ID16, child_id: ID16) -> Result<(), String>;
}

/// Statistics about storage performance and health.
/// 
/// Different implementations may provide different metrics,
/// but these represent common performance indicators.
#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    /// Total number of documents
    pub total_documents: usize,
    /// Total number of root documents
    pub root_documents: usize,
    /// Implementation-specific metrics (optional)
    pub implementation_stats: std::collections::HashMap<String, f64>,
}

/// Mem Store
pub mod mem_store;

/// Engine
pub mod engine;

/// Factory
pub mod factory;

/// Heap
pub mod heap;
pub use heap::{RootDocumentHeap, DeltaHeap};

/// Re-export main storage types
pub use mem_store::MemStore;
pub use factory::{create_storage, StorageFactoryError};