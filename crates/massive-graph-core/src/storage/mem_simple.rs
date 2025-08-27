//! Simple in-memory storage implementation using DashMap
//! 
//! This implementation stores documents and deltas as JSON objects in memory
//! using DashMap for concurrent access. It provides a simplified alternative
//! to the more complex MemStore implementation.

use dashmap::DashMap;
use serde_json::Value;
use crate::types::DocId;
use crate::DocumentStorage;

// use tracing::info; // TODO: Will be used when logging is implemented

/// Simple storage implementation using DashMap and JSON
pub struct SimpleStorage {
    /// Map of document_id to JSON documents
    documents: DashMap<DocId, Value>,
    
    /// Map of delta_id to JSON deltas (delta_id is doc_id + counter)
    deltas: DashMap<String, Value>,
    
    /// Counter for generating delta IDs
    delta_counter: std::sync::atomic::AtomicU64,
}

impl SimpleStorage {
    /// Create a new SimpleStorage instance
    pub fn new() -> Self {
        Self {
            documents: DashMap::new(),
            deltas: DashMap::new(),
            delta_counter: std::sync::atomic::AtomicU64::new(0),
        }
    }
    
    /// Generate a new delta ID
    fn next_delta_id(&self) -> String {
        let id = self.delta_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        format!("delta_{}", id)
    }
}

impl DocumentStorage for SimpleStorage {
    fn get_document(&self, doc_id: DocId) -> Option<Vec<u8>> {
        // Get JSON document and convert to bytes
        self.documents.get(&doc_id).map(|doc| {
            serde_json::to_vec(&*doc).unwrap_or_else(|_| Vec::new())
        })
    }
    
    fn apply_delta(&self, doc_id: DocId, delta: Vec<u8>) -> Result<(), String> {
        // Parse delta as JSON
        let delta_json: Value = serde_json::from_slice(&delta)
            .map_err(|e| format!("Failed to parse delta as JSON: {}", e))?;
        
        // Store delta with generated ID
        let delta_id = format!("{}_delta_{}", doc_id, self.next_delta_id());
        
        // Create delta object with metadata
        let delta_object = serde_json::json!({
            "id": delta_id.clone(),
            "doc_id": doc_id.to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "data": delta_json
        });
        
        self.deltas.insert(delta_id, delta_object);
        
        // In a real implementation, we would also apply the delta to the document
        // For now, just store it
        Ok(())
    }
    
    fn create_document(&self, doc_id: DocId, doc_data: Vec<u8>) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        tracing::info!("💾 SimpleStorage::create_document - doc: {}, data_size: {}", doc_id, doc_data.len());
        
        // Check if document already exists
        #[cfg(not(target_arch = "wasm32"))]
        tracing::debug!("🔍 Checking if document already exists");
        if self.documents.contains_key(&doc_id) {
            #[cfg(not(target_arch = "wasm32"))]
            tracing::error!("❌ Document already exists: {}", doc_id);
            return Err(format!("Document {} already exists", doc_id));
        }
        #[cfg(not(target_arch = "wasm32"))]
        tracing::debug!("✅ Document doesn't exist, proceeding");
        
        // Parse the incoming document data as JSON
        #[cfg(not(target_arch = "wasm32"))]
        tracing::debug!("📄 Parsing document data as JSON");
        let mut doc_json: Value = serde_json::from_slice(&doc_data)
            .map_err(|e| {
                #[cfg(not(target_arch = "wasm32"))]
                tracing::error!("❌ Failed to parse JSON: {}", e);
                format!("Failed to parse document data as JSON: {}", e)
            })?;
        #[cfg(not(target_arch = "wasm32"))]
        tracing::debug!("✅ JSON parsed successfully");
        
        // Add metadata if not present
        #[cfg(not(target_arch = "wasm32"))]
        tracing::debug!("🏷️ Adding metadata to document");
        if let Some(obj) = doc_json.as_object_mut() {
            obj.entry("id").or_insert(serde_json::json!(doc_id.to_string()));
            obj.entry("created_at").or_insert(serde_json::json!(chrono::Utc::now().to_rfc3339()));
            obj.entry("updated_at").or_insert(serde_json::json!(chrono::Utc::now().to_rfc3339()));
            obj.entry("version").or_insert(serde_json::json!(1));
            obj.entry("children").or_insert(serde_json::json!([]));
        }
        #[cfg(not(target_arch = "wasm32"))]
        tracing::debug!("✅ Metadata added");
        
        #[cfg(not(target_arch = "wasm32"))]
        tracing::debug!("💾 Inserting document into storage");
        self.documents.insert(doc_id, doc_json);
        #[cfg(not(target_arch = "wasm32"))]
        tracing::info!("✅ SimpleStorage::create_document successful - doc count now: {}", self.documents.len());
        Ok(())
    }
    
    fn remove_document(&self, doc_id: DocId) -> Result<(), String> {
        if self.documents.remove(&doc_id).is_some() {
            Ok(())
        } else {
            Err(format!("Document {} not found", doc_id))
        }
    }
    
    fn document_exists(&self, doc_id: DocId) -> bool {
        self.documents.contains_key(&doc_id)
    }
    
    fn add_child_relationship(&self, parent_id: DocId, child_id: DocId) -> Result<(), String> {
        // Get parent document
        if let Some(mut parent_doc) = self.documents.get_mut(&parent_id) {
            // Add child to children array
            if let Some(children) = parent_doc.get_mut("children").and_then(|v| v.as_array_mut()) {
                children.push(serde_json::json!(child_id.to_string()));
                Ok(())
            } else {
                Err("Parent document has invalid structure".to_string())
            }
        } else {
            Err(format!("Parent document {} not found", parent_id))
        }
    }
    
    fn remove_child_relationship(&self, parent_id: DocId, child_id: DocId) -> Result<(), String> {
        // Get parent document
        if let Some(mut parent_doc) = self.documents.get_mut(&parent_id) {
            // Remove child from children array
            if let Some(children) = parent_doc.get_mut("children").and_then(|v| v.as_array_mut()) {
                let child_id_str = child_id.to_string();
                children.retain(|v| v.as_str() != Some(&child_id_str));
                Ok(())
            } else {
                Err("Parent document has invalid structure".to_string())
            }
        } else {
            Err(format!("Parent document {} not found", parent_id))
        }
    }
    
    fn document_count(&self) -> usize {
        self.documents.len()
    }
}

impl Default for SimpleStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for SimpleStorage {
    fn clone(&self) -> Self {
        Self {
            documents: self.documents.clone(),
            deltas: self.deltas.clone(),
            delta_counter: std::sync::atomic::AtomicU64::new(
                self.delta_counter.load(std::sync::atomic::Ordering::Relaxed)
            ),
        }
    }
} 