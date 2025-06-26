/// Example demonstrating storage-agnostic delta processing.
/// 
/// This example shows how the DeltaProcessor can work with any storage
/// implementation that provides the DocumentStorage trait, making it
/// easy to swap out storage backends without changing delta logic.

use massive_graph::storage::{MemStore, DocumentStorage};
use massive_graph::core::types::{ID16, document::{Value, AdaptiveMap}};

fn main() {
    println!("=== Storage-Agnostic Delta Processing Example ===\n");
    
    // Create a store instance
    let mut store = MemStore::new();
    
    // Demonstrate that the store implements DocumentStorage
    demonstrate_storage_trait(&mut store);
    
    // Show how delta processor can work with any storage implementation
    demonstrate_delta_processor_usage(&mut store);
    
    println!("Example completed successfully!");
}

/// Demonstrate using the store through the DocumentStorage trait.
/// 
/// This shows that the store can be used through the trait interface,
/// making it compatible with the delta processor.
fn demonstrate_storage_trait(storage: &mut dyn DocumentStorage) {
    println!("1. Using Store through DocumentStorage trait:");
    
    let doc_id = ID16::random();
    
    // Create a document using the trait interface
    let mut properties = AdaptiveMap::new();
    properties.insert("title".to_string(), Value::String("Example Document".to_string()));
    properties.insert("content".to_string(), Value::String("This is example content".to_string()));
    properties.insert("created_at".to_string(), Value::U64(1234567890));
    
    storage.create_document(doc_id, properties).unwrap();
    println!("   ✓ Created document {} through trait", doc_id);
    
    // Verify document exists
    assert!(storage.document_exists(&doc_id));
    println!("   ✓ Document exists check passed");
    
    // Retrieve document through trait
    let document = storage.get_document(&doc_id).unwrap();
    if let Some(Value::String(title)) = document.get("title") {
        println!("   ✓ Retrieved document title: '{}'", title);
    }
    
    // Test child relationships
    let parent_id = ID16::random();
    let child_id = ID16::random();
    
    // Create parent with children array
    let mut parent_props = AdaptiveMap::new();
    parent_props.insert("name".to_string(), Value::String("Parent Document".to_string()));
    parent_props.insert("children".to_string(), Value::Array(Vec::new()));
    storage.create_document(parent_id, parent_props).unwrap();
    
    // Create child
    let mut child_props = AdaptiveMap::new();
    child_props.insert("name".to_string(), Value::String("Child Document".to_string()));
    storage.create_document(child_id, child_props).unwrap();
    
    // Add child relationship through trait
    storage.add_child_relationship(parent_id, child_id).unwrap();
    println!("   ✓ Added child relationship through trait");
    
    // Verify relationship was created
    let parent = storage.get_document(&parent_id).unwrap();
    if let Some(Value::Array(children)) = parent.get("children") {
        assert_eq!(children.len(), 1);
        println!("   ✓ Parent now has {} child", children.len());
    }
    
    // Remove child relationship
    storage.remove_child_relationship(parent_id, child_id).unwrap();
    let parent = storage.get_document(&parent_id).unwrap();
    if let Some(Value::Array(children)) = parent.get("children") {
        assert_eq!(children.len(), 0);
        println!("   ✓ Child relationship removed successfully");
    }
    
    // Clean up
    storage.remove_document(&doc_id).unwrap();
    storage.remove_document(&parent_id).unwrap();
    storage.remove_document(&child_id).unwrap();
    println!("   ✓ Cleanup completed\n");
}

/// Demonstrate how the delta processor can work with any storage.
/// 
/// This shows the key benefit: delta processing logic is completely
/// separate from storage implementation details.
fn demonstrate_delta_processor_usage(storage: &mut dyn DocumentStorage) {
    println!("2. Delta Processor with Storage Abstraction:");
    
    // The delta processor doesn't know or care what storage implementation
    // it's working with. It could be in-memory, persistent, distributed, etc.
    
    let doc_id = ID16::random();
    
    // Create a document directly through storage
    let mut properties = AdaptiveMap::new();
    properties.insert("status".to_string(), Value::String("draft".to_string()));
    properties.insert("version".to_string(), Value::U64(1));
    storage.create_document(doc_id, properties).unwrap();
    
    println!("   ✓ Created document {} with status 'draft'", doc_id);
    
    // Simulate updating the document through delta operations
    // (In a real system, this would parse actual delta messages)
    
    // Update status property
    if let Some(document) = storage.get_document_mut(&doc_id) {
        document.insert("status".to_string(), Value::String("published".to_string()));
        document.insert("version".to_string(), Value::U64(2));
        println!("   ✓ Updated document status to 'published' (simulated delta)");
    }
    
    // Verify the update
    let document = storage.get_document(&doc_id).unwrap();
    if let Some(Value::String(status)) = document.get("status") {
        assert_eq!(status, "published");
        println!("   ✓ Status update verified: '{}'", status);
    }
    
    if let Some(Value::U64(version)) = document.get("version") {
        assert_eq!(*version, 2);
        println!("   ✓ Version update verified: {}", version);
    }
    
    // The key insight: DeltaProcessor.apply_delta() would work the same way
    // regardless of whether storage is:
    // - InMemoryStore (like we're using)
    // - PersistentStore
    // - DistributedStore
    // - CachedStore
    // - Any other implementation of DocumentStorage
    
    println!("   ✓ Delta processing abstraction demonstrated");
    
    // Clean up
    storage.remove_document(&doc_id).unwrap();
    println!("   ✓ Cleanup completed\n");
}

/// Example of how you could create different storage implementations
/// that all work with the same delta processor.
#[allow(dead_code)]
mod alternative_storage_examples {
    use super::*;
    use std::collections::HashMap;
    
    /// Example of a simple key-value storage implementation.
    /// 
    /// This shows how easy it is to create alternative storage backends
    /// that work with the same delta processing logic.
    pub struct KeyValueStore {
        data: HashMap<ID16, AdaptiveMap<String, Value>>,
    }
    
    impl KeyValueStore {
        pub fn new() -> Self {
            Self {
                data: HashMap::new(),
            }
        }
    }
    
    impl DocumentStorage for KeyValueStore {
        fn get_document(&self, id: &ID16) -> Option<&AdaptiveMap<String, Value>> {
            self.data.get(id)
        }
        
        fn get_document_mut(&mut self, id: &ID16) -> Option<&mut AdaptiveMap<String, Value>> {
            self.data.get_mut(id)
        }
        
        fn create_document(&mut self, id: ID16, properties: AdaptiveMap<String, Value>) -> Result<(), String> {
            self.data.insert(id, properties);
            Ok(())
        }
        
        fn remove_document(&mut self, id: &ID16) -> Result<(), String> {
            self.data.remove(id);
            Ok(())
        }
        
        fn document_exists(&self, id: &ID16) -> bool {
            self.data.contains_key(id)
        }
        
        fn add_child_relationship(&mut self, parent_id: ID16, child_id: ID16) -> Result<(), String> {
            let parent = self.data.get_mut(&parent_id)
                .ok_or_else(|| format!("Parent document {} not found", parent_id))?;
            
            if let Some(Value::Array(ref mut children)) = parent.get_mut("children") {
                children.push(Value::Reference(child_id));
            } else {
                parent.insert("children".to_string(), Value::Array(vec![Value::Reference(child_id)]));
            }
            
            Ok(())
        }
        
        fn remove_child_relationship(&mut self, parent_id: ID16, child_id: ID16) -> Result<(), String> {
            let parent = self.data.get_mut(&parent_id)
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
    
    /// This function demonstrates that the same delta processing logic
    /// works with completely different storage implementations.
    #[allow(dead_code)]
    pub fn demonstrate_alternative_storage() {
        let mut kv_store = KeyValueStore::new();
        
        // The exact same delta processing logic works with this
        // completely different storage implementation!
        // DeltaProcessor::apply_delta(&mut kv_store, &some_delta);
        
        // For now, just show basic operations work
        let doc_id = ID16::random();
        let mut props = AdaptiveMap::new();
        props.insert("test".to_string(), Value::String("value".to_string()));
        
        kv_store.create_document(doc_id, props).unwrap();
        assert!(kv_store.document_exists(&doc_id));
        
        println!("   ✓ Alternative KeyValueStore works with same trait!");
    }
} 