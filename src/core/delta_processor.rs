/// Delta operation processor that is agnostic to the underlying storage layer.
/// 
/// This module provides the core logic for applying delta operations to documents
/// without being tied to any specific storage implementation. It works through
/// a trait-based interface that any storage layer can implement.

use crate::core::types::document::{Value, AdaptiveMap, DocumentType};
use crate::core::types::delta::{Delta, Operation, OpType};
use crate::core::types::ID16;

/// Trait that any storage layer must implement to support delta operations.
/// 
/// This abstraction allows the delta processor to work with different storage
/// backends (in-memory, persistent, distributed, etc.) without knowing the
/// implementation details.
pub trait DocumentStorage {
    /// Get a document by ID.
    fn get_document(&self, id: &ID16) -> Option<&AdaptiveMap<String, Value>>;
    
    /// Get a mutable reference to a document by ID.
    fn get_document_mut(&mut self, id: &ID16) -> Option<&mut AdaptiveMap<String, Value>>;
    
    /// Create a new document with the given ID and properties.
    fn create_document(&mut self, id: ID16, properties: AdaptiveMap<String, Value>) -> Result<(), String>;
    
    /// Remove a document by ID.
    fn remove_document(&mut self, id: &ID16) -> Result<(), String>;
    
    /// Check if a document exists.
    fn document_exists(&self, id: &ID16) -> bool;
    
    /// Add a document as a child of another document.
    fn add_child_relationship(&mut self, parent_id: ID16, child_id: ID16) -> Result<(), String>;
    
    /// Remove a child relationship between documents.
    fn remove_child_relationship(&mut self, parent_id: ID16, child_id: ID16) -> Result<(), String>;
}

/// Storage-agnostic delta processor.
/// 
/// This processor can apply delta operations to any storage layer that
/// implements the DocumentStorage trait. It contains all the business logic
/// for handling different operation types.
pub struct DeltaProcessor;

impl DeltaProcessor {
    /// Process a delta operation on the given storage.
    /// 
    /// This is the main entry point for applying delta operations received
    /// from the network or generated locally. Operations are applied atomically
    /// and update the storage state accordingly.
    /// 
    /// # Arguments
    /// 
    /// * `storage` - The storage layer to apply operations to
    /// * `delta` - The delta containing operations to apply
    /// 
    /// # Returns
    /// 
    /// Success or error describing what went wrong
    pub fn apply_delta<S: DocumentStorage>(storage: &mut S, delta: &Delta) -> Result<(), String> {
        // Parse operations from the delta data
        let operations = Self::parse_operations(&delta.data)?;
        
        // Apply each operation in sequence
        for operation in operations {
            Self::apply_operation(storage, operation)?;
        }
        
        Ok(())
    }
    
    /// Apply a single operation to the storage.
    /// 
    /// # Arguments
    /// 
    /// * `storage` - The storage layer to apply the operation to
    /// * `operation` - The operation to apply
    /// 
    /// # Returns
    /// 
    /// Success or error if operation failed
    fn apply_operation<S: DocumentStorage>(storage: &mut S, operation: Operation<'_>) -> Result<(), String> {
        match operation.op_type {
            OpType::DocumentCreate => Self::handle_document_create(storage, operation),
            OpType::DocumentDelete => Self::handle_document_delete(storage, operation),
            OpType::DocumentMove => Self::handle_document_move(storage, operation),
            OpType::PropertySet => Self::handle_property_set(storage, operation),
            OpType::PropertyDelete => Self::handle_property_delete(storage, operation),
            OpType::ChildAdd => Self::handle_child_add(storage, operation),
            OpType::ChildRemove => Self::handle_child_remove(storage, operation),
            _ => Err(format!("Unsupported operation type: {:?}", operation.op_type)),
        }
    }
    
    /// Handle DocumentCreate operation.
    /// 
    /// Creates a new document in the storage. The payload should contain:
    /// - Parent document ID (16 bytes) - which document to add this as child of
    /// - Document type (1 byte) - what type of document to create
    /// - Document properties (remaining bytes) - serialized properties
    /// 
    /// # Arguments
    /// 
    /// * `storage` - The storage layer
    /// * `operation` - The DocumentCreate operation
    /// 
    /// # Returns
    /// 
    /// Success or error if creation failed
    fn handle_document_create<S: DocumentStorage>(storage: &mut S, operation: Operation<'_>) -> Result<(), String> {
        // Parse the payload
        if operation.payload.len() < 17 {
            return Err("DocumentCreate payload too short".to_string());
        }
        
        // Extract parent document ID (first 16 bytes)
        let mut parent_id_bytes = [0u8; 16];
        parent_id_bytes.copy_from_slice(&operation.payload[0..16]);
        let parent_id = ID16::new(parent_id_bytes);
        
        // Extract document type (byte 16)
        let doc_type_byte = operation.payload[16];
        let doc_type = Self::byte_to_document_type(doc_type_byte)?;
        
        // Extract properties (remaining bytes)
        let properties_data = &operation.payload[17..];
        
        // Deserialize properties from the payload
        let mut properties = Self::deserialize_properties(properties_data)?;
        
        // Ensure document has required metadata
        properties.insert("doc_type".to_string(), Value::U8(doc_type as u8));
        properties.insert("created_at".to_string(), Value::U64(Self::current_timestamp()));
        properties.insert("modified_at".to_string(), Value::U64(Self::current_timestamp()));
        
        // Initialize children array if not present
        if !properties.get("children").is_some() {
            properties.insert("children".to_string(), Value::Array(Vec::new()));
        }
        
        // Verify parent document exists (unless this is a root document)
        if doc_type != DocumentType::Root && !storage.document_exists(&parent_id) {
            return Err(format!("Parent document {} does not exist", parent_id));
        }
        
        // Create the document
        storage.create_document(operation.target_id, properties)?;
        
        // Add as child to parent (unless this is a root document)
        if doc_type != DocumentType::Root {
            storage.add_child_relationship(parent_id, operation.target_id)?;
        }
        
        Ok(())
    }
    
    /// Handle DocumentDelete operation.
    /// 
    /// # Arguments
    /// 
    /// * `storage` - The storage layer
    /// * `operation` - The DocumentDelete operation
    /// 
    /// # Returns
    /// 
    /// Success or error if deletion failed
    fn handle_document_delete<S: DocumentStorage>(storage: &mut S, operation: Operation<'_>) -> Result<(), String> {
        storage.remove_document(&operation.target_id)
    }
    
    /// Handle DocumentMove operation.
    /// 
    /// Moves a document from one parent to another.
    /// Payload should contain the new parent document ID (16 bytes).
    /// 
    /// # Arguments
    /// 
    /// * `storage` - The storage layer
    /// * `operation` - The DocumentMove operation
    /// 
    /// # Returns
    /// 
    /// Success or error if move failed
    fn handle_document_move<S: DocumentStorage>(storage: &mut S, operation: Operation<'_>) -> Result<(), String> {
        if operation.payload.len() != 16 {
            return Err("DocumentMove payload must be exactly 16 bytes (parent ID)".to_string());
        }
        
        // Extract new parent document ID
        let mut new_parent_id_bytes = [0u8; 16];
        new_parent_id_bytes.copy_from_slice(operation.payload);
        let new_parent_id = ID16::new(new_parent_id_bytes);
        
        // Verify new parent document exists
        if !storage.document_exists(&new_parent_id) {
            return Err(format!("Target parent document {} does not exist", new_parent_id));
        }
        
        // Find current parent by looking through all documents
        // TODO: This could be optimized with a parent index in the storage layer
        let current_parent_id = Self::find_parent_document(storage, &operation.target_id)?;
        
        // Remove from old parent
        storage.remove_child_relationship(current_parent_id, operation.target_id)?;
        
        // Add to new parent
        storage.add_child_relationship(new_parent_id, operation.target_id)?;
        
        Ok(())
    }
    
    /// Handle PropertySet operation.
    /// 
    /// Sets a property value on a document. Payload format:
    /// - Property name length (2 bytes)
    /// - Property name (variable bytes)
    /// - Property value (remaining bytes, serialized)
    /// 
    /// # Arguments
    /// 
    /// * `storage` - The storage layer
    /// * `operation` - The PropertySet operation
    /// 
    /// # Returns
    /// 
    /// Success or error if property set failed
    fn handle_property_set<S: DocumentStorage>(storage: &mut S, operation: Operation<'_>) -> Result<(), String> {
        if operation.payload.len() < 2 {
            return Err("PropertySet payload too short".to_string());
        }
        
        // Extract property name length
        let name_len = u16::from_le_bytes([operation.payload[0], operation.payload[1]]) as usize;
        
        if operation.payload.len() < 2 + name_len {
            return Err("PropertySet payload incomplete".to_string());
        }
        
        // Extract property name
        let property_name = String::from_utf8(operation.payload[2..2 + name_len].to_vec())
            .map_err(|_| "Invalid UTF-8 in property name".to_string())?;
        
        // Extract and deserialize property value
        let value_data = &operation.payload[2 + name_len..];
        let property_value = Self::deserialize_value(value_data)?;
        
        // Get the document and update the property
        let document = storage.get_document_mut(&operation.target_id)
            .ok_or_else(|| format!("Document {} not found", operation.target_id))?;
        
        document.insert(property_name, property_value);
        
        // Update modification time
        document.insert("modified_at".to_string(), Value::U64(Self::current_timestamp()));
        
        Ok(())
    }
    
    /// Handle PropertyDelete operation.
    /// 
    /// Deletes a property from a document. Payload contains the property name.
    /// 
    /// # Arguments
    /// 
    /// * `storage` - The storage layer
    /// * `operation` - The PropertyDelete operation
    /// 
    /// # Returns
    /// 
    /// Success or error if property deletion failed
    fn handle_property_delete<S: DocumentStorage>(storage: &mut S, operation: Operation<'_>) -> Result<(), String> {
        // Property name is the entire payload
        let property_name = String::from_utf8(operation.payload.to_vec())
            .map_err(|_| "Invalid UTF-8 in property name".to_string())?;
        
        // Get the document and remove the property
        let document = storage.get_document_mut(&operation.target_id)
            .ok_or_else(|| format!("Document {} not found", operation.target_id))?;
        
        document.remove(&property_name);
        
        // Update modification time
        document.insert("modified_at".to_string(), Value::U64(Self::current_timestamp()));
        
        Ok(())
    }
    
    /// Handle ChildAdd operation.
    /// 
    /// Adds a child relationship to a document. Payload contains the child document ID (16 bytes).
    /// 
    /// # Arguments
    /// 
    /// * `storage` - The storage layer
    /// * `operation` - The ChildAdd operation
    /// 
    /// # Returns
    /// 
    /// Success or error if child addition failed
    fn handle_child_add<S: DocumentStorage>(storage: &mut S, operation: Operation<'_>) -> Result<(), String> {
        if operation.payload.len() != 16 {
            return Err("ChildAdd payload must be exactly 16 bytes (child ID)".to_string());
        }
        
        // Extract child document ID
        let mut child_id_bytes = [0u8; 16];
        child_id_bytes.copy_from_slice(operation.payload);
        let child_id = ID16::new(child_id_bytes);
        
        // Add the child relationship
        storage.add_child_relationship(operation.target_id, child_id)
    }
    
    /// Handle ChildRemove operation.
    /// 
    /// Removes a child relationship from a document. Payload contains the child document ID (16 bytes).
    /// 
    /// # Arguments
    /// 
    /// * `storage` - The storage layer
    /// * `operation` - The ChildRemove operation
    /// 
    /// # Returns
    /// 
    /// Success or error if child removal failed
    fn handle_child_remove<S: DocumentStorage>(storage: &mut S, operation: Operation<'_>) -> Result<(), String> {
        if operation.payload.len() != 16 {
            return Err("ChildRemove payload must be exactly 16 bytes (child ID)".to_string());
        }
        
        // Extract child document ID
        let mut child_id_bytes = [0u8; 16];
        child_id_bytes.copy_from_slice(operation.payload);
        let child_id = ID16::new(child_id_bytes);
        
        // Remove the child relationship
        storage.remove_child_relationship(operation.target_id, child_id)
    }
    
    /// Find the parent document of a given document.
    /// 
    /// This is a helper function that searches through the storage to find
    /// which document contains the target as a child.
    /// 
    /// # Arguments
    /// 
    /// * `storage` - The storage layer to search
    /// * `target_id` - The document to find the parent of
    /// 
    /// # Returns
    /// 
    /// Parent document ID or error if not found
    fn find_parent_document<S: DocumentStorage>(storage: &S, target_id: &ID16) -> Result<ID16, String> {
        // TODO: This is inefficient and should be replaced with a parent index
        // For now, this is a placeholder that would need storage-specific implementation
        Err("Parent lookup not implemented - storage layer should maintain parent index".to_string())
    }
    
    /// Convert a byte value to DocumentType enum.
    /// 
    /// # Arguments
    /// 
    /// * `byte` - The byte value to convert
    /// 
    /// # Returns
    /// 
    /// DocumentType or error if invalid
    fn byte_to_document_type(byte: u8) -> Result<DocumentType, String> {
        match byte {
            0 => Ok(DocumentType::Root),
            1 => Ok(DocumentType::Generic),
            2 => Ok(DocumentType::Text),
            3 => Ok(DocumentType::Binary),
            4 => Ok(DocumentType::Json),
            11 => Ok(DocumentType::Graph),
            12 => Ok(DocumentType::Node),
            13 => Ok(DocumentType::Edge),
            _ => Err(format!("Unknown document type byte: {}", byte)),
        }
    }
    
    /// Parse operations from binary delta data.
    /// 
    /// This is a placeholder implementation that would normally parse
    /// the binary format defined by the delta protocol.
    /// 
    /// # Arguments
    /// 
    /// * `data` - Binary data containing serialized operations
    /// 
    /// # Returns
    /// 
    /// Vector of parsed operations or error
    fn parse_operations(data: &[u8]) -> Result<Vec<Operation<'_>>, String> {
        // TODO: Implement proper binary parsing of delta operations
        // For now, this is a placeholder that would be replaced with
        // actual binary protocol parsing
        let _ = data; // Suppress unused parameter warning
        Err("Operation parsing not yet implemented".to_string())
    }
    
    /// Deserialize properties from binary data.
    /// 
    /// This is a placeholder for the actual serialization format.
    /// 
    /// # Arguments
    /// 
    /// * `data` - Binary data containing serialized properties
    /// 
    /// # Returns
    /// 
    /// Deserialized properties map or error
    fn deserialize_properties(data: &[u8]) -> Result<AdaptiveMap<String, Value>, String> {
        // TODO: Implement proper binary deserialization
        // For now, return an empty properties map
        let _ = data; // Suppress unused parameter warning
        Ok(AdaptiveMap::new())
    }
    
    /// Deserialize a single value from binary data.
    /// 
    /// This is a placeholder for the actual serialization format.
    /// 
    /// # Arguments
    /// 
    /// * `data` - Binary data containing serialized value
    /// 
    /// # Returns
    /// 
    /// Deserialized value or error
    fn deserialize_value(data: &[u8]) -> Result<Value, String> {
        // TODO: Implement proper binary deserialization
        // For now, return a null value
        let _ = data; // Suppress unused parameter warning
        Ok(Value::Null)
    }
    
    /// Get current timestamp in nanoseconds since epoch.
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    /// Mock storage implementation for testing
    struct MockStorage {
        documents: HashMap<ID16, AdaptiveMap<String, Value>>,
    }
    
    impl MockStorage {
        fn new() -> Self {
            Self {
                documents: HashMap::new(),
            }
        }
    }
    
    impl DocumentStorage for MockStorage {
        fn get_document(&self, id: &ID16) -> Option<&AdaptiveMap<String, Value>> {
            self.documents.get(id)
        }
        
        fn get_document_mut(&mut self, id: &ID16) -> Option<&mut AdaptiveMap<String, Value>> {
            self.documents.get_mut(id)
        }
        
        fn create_document(&mut self, id: ID16, properties: AdaptiveMap<String, Value>) -> Result<(), String> {
            self.documents.insert(id, properties);
            Ok(())
        }
        
        fn remove_document(&mut self, id: &ID16) -> Result<(), String> {
            self.documents.remove(id);
            Ok(())
        }
        
        fn document_exists(&self, id: &ID16) -> bool {
            self.documents.contains_key(id)
        }
        
        fn add_child_relationship(&mut self, parent_id: ID16, child_id: ID16) -> Result<(), String> {
            let parent = self.documents.get_mut(&parent_id)
                .ok_or_else(|| format!("Parent document {} not found", parent_id))?;
            
            if let Some(Value::Array(ref mut children)) = parent.get_mut("children") {
                children.push(Value::Reference(child_id));
            } else {
                parent.insert("children".to_string(), Value::Array(vec![Value::Reference(child_id)]));
            }
            
            Ok(())
        }
        
        fn remove_child_relationship(&mut self, parent_id: ID16, child_id: ID16) -> Result<(), String> {
            let parent = self.documents.get_mut(&parent_id)
                .ok_or_else(|| format!("Parent document {} not found", parent_id))?;
            
            if let Some(Value::Array(ref mut children)) = parent.get_mut("children") {
                children.retain(|child| {
                    if let Value::Reference(child_ref_id) = child {
                        *child_ref_id != child_id
                    } else {
                        true
                    }
                });
            }
            
            Ok(())
        }
    }
    
    #[test]
    fn test_mock_storage_creation() {
        let storage = MockStorage::new();
        assert_eq!(storage.documents.len(), 0);
    }
    
    #[test]
    fn test_document_creation() {
        let mut storage = MockStorage::new();
        let doc_id = ID16::random();
        let properties = AdaptiveMap::new();
        
        storage.create_document(doc_id, properties).unwrap();
        assert!(storage.document_exists(&doc_id));
    }
    
    #[test]
    fn test_child_relationships() {
        let mut storage = MockStorage::new();
        let parent_id = ID16::random();
        let child_id = ID16::random();
        
        // Create parent document
        let mut parent_props = AdaptiveMap::new();
        parent_props.insert("children".to_string(), Value::Array(Vec::new()));
        storage.create_document(parent_id, parent_props).unwrap();
        
        // Add child relationship
        storage.add_child_relationship(parent_id, child_id).unwrap();
        
        // Verify child was added
        let parent = storage.get_document(&parent_id).unwrap();
        if let Some(Value::Array(children)) = parent.get("children") {
            assert_eq!(children.len(), 1);
            if let Value::Reference(ref_id) = &children[0] {
                assert_eq!(*ref_id, child_id);
            } else {
                panic!("Expected Reference value");
            }
        } else {
            panic!("Expected children array");
        }
    }
} 