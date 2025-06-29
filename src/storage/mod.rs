//! Storage layer for Massive Graph Database
//! 
//! This module provides the storage abstraction layer that enables different
//! storage backends while maintaining consistent APIs for document operations.

use crate::core::types::{ID16, document::{DocumentHeader, Document, DocumentType, Value, AdaptiveMap}};

/// Zero-copy document storage trait for high-performance document access.
/// 
/// This trait defines the core operations needed for document storage without
/// prescribing implementation details. Storage backends can use any approach
/// (caching, logging, direct access, etc.) as long as they provide these operations.
pub trait ZeroCopyDocumentStorage {
    // ===== CORE DOCUMENT OPERATIONS =====
    
    /// Get a zero-copy view of a document.
    /// 
    /// Returns a lightweight view that references existing memory without copying.
    /// The view remains valid as long as the storage exists.
    fn get_document(&self, id: &ID16) -> Option<&Document<'_>>;
    
    /// Get document header for metadata-only access.
    /// 
    /// Optimized for cases where only metadata is needed,
    /// avoiding the overhead of constructing full document views.
    fn get_document_header(&self, id: &ID16) -> Option<&DocumentHeader>;
    
    /// Create a document from properties with proper typing and hierarchy.
    /// 
    /// This creates a new document with the specified type and parent relationship.
    /// The properties are serialized into the storage backend's internal format.
    fn create_document(
        &self, 
        id: ID16, 
        doc_type: DocumentType,
        parent_id: ID16,
        properties: &AdaptiveMap<String, Value>
    ) -> Result<(), String>;
    
    /// Update a single property atomically.
    /// 
    /// This enables concurrent property updates without blocking other operations.
    /// Implementation may use locks, lock-free algorithms, or other synchronization.
    fn update_property(
        &self, 
        id: &ID16, 
        property: &str, 
        value: &Value
    ) -> Result<(), String>;
    
    /// Remove a document and all its relationships.
    fn remove_document(&self, id: &ID16) -> Result<(), String>;
    
    /// Check if a document exists.
    fn document_exists(&self, id: &ID16) -> bool;
    
    // ===== RELATIONSHIP OPERATIONS =====
    
    /// Add a parent-child relationship between documents.
    fn add_child_relationship(&self, parent_id: ID16, child_id: ID16) -> Result<(), String>;
    
    /// Remove a parent-child relationship between documents.
    fn remove_child_relationship(&self, parent_id: ID16, child_id: ID16) -> Result<(), String>;
    
    // ===== HIERARCHY OPERATIONS =====
    
    /// Create a root document (convenience method).
    /// 
    /// This is equivalent to create_document with parent_id = ID16::default()
    /// but may have optimized implementation for root document management.
    fn create_root_document(
        &self,
        id: ID16,
        name: String,
        description: String
    ) -> Result<(), String>;
    
    /// Get all root document IDs.
    fn get_root_documents(&self) -> Vec<ID16>;
    
    /// Find the root document for a given document ID.
    /// 
    /// Traverses the hierarchy upward to find the root container.
    fn find_root_for_document(&self, document_id: &ID16) -> Option<ID16>;
    
    // ===== STORAGE INFORMATION =====
    
    /// Get the total number of documents in storage.
    fn document_count(&self) -> usize;
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

/// Re-export main storage types
pub use mem_store::MemStore;
pub use factory::{create_storage, StorageFactoryError};