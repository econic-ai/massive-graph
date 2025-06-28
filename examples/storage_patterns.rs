/// Example comparing dependency injection vs static storage patterns.
/// 
/// This demonstrates the trade-offs between passing storage as parameters
/// versus using a global static storage instance.

use massive_graph::storage::{MemStore, DocumentStorage};
use massive_graph::core::DeltaProcessor;
use massive_graph::core::types::{ID16, document::{Value, AdaptiveMap}};
use std::sync::{Mutex, OnceLock};
use std::collections::HashMap;

// Static storage approach - global singleton
static GLOBAL_STORAGE: OnceLock<Mutex<MemStore>> = OnceLock::new();

/// Initialize the global storage (must be called once at startup)
pub fn initialize_global_storage() {
    GLOBAL_STORAGE.set(Mutex::new(MemStore::new())).expect("Storage already initialized");
}

/// Static storage version of delta processor
pub struct StaticDeltaProcessor;

impl StaticDeltaProcessor {
    /// Apply delta using global storage
    pub fn apply_delta_static(doc_id: ID16, properties: AdaptiveMap<String, Value>) -> Result<(), String> {
        let storage = GLOBAL_STORAGE.get()
            .ok_or("Storage not initialized")?;
        
        let mut storage = storage.lock()
            .map_err(|_| "Failed to acquire storage lock")?;
        
        // Simulate delta operation
        storage.create_document(doc_id, properties)
    }
    
    /// Get document using global storage
    pub fn get_document_static(doc_id: &ID16) -> Result<Option<String>, String> {
        let storage = GLOBAL_STORAGE.get()
            .ok_or("Storage not initialized")?;
        
        let storage = storage.lock()
            .map_err(|_| "Failed to acquire storage lock")?;
        
        if let Some(doc) = storage.get_document(doc_id) {
            if let Some(Value::String(title)) = doc.get("title") {
                return Ok(Some(title.clone()));
            }
        }
        Ok(None)
    }
}

/// Dependency injection version (current approach)
pub struct InjectedDeltaProcessor;

impl InjectedDeltaProcessor {
    /// Apply delta with injected storage
    pub fn apply_delta_injected<S: DocumentStorage>(
        storage: &mut S,
        doc_id: ID16, 
        properties: AdaptiveMap<String, Value>
    ) -> Result<(), String> {
        // Simulate delta operation
        storage.create_document(doc_id, properties)
    }
    
    /// Get document with injected storage
    pub fn get_document_injected<S: DocumentStorage>(
        storage: &S,
        doc_id: &ID16
    ) -> Option<String> {
        if let Some(doc) = storage.get_document(doc_id) {
            if let Some(Value::String(title)) = doc.get("title") {
                return Some(title.clone());
            }
        }
        None
    }
}

fn main() {
    println!("=== Storage Pattern Comparison ===\n");
    
    // Demonstrate static storage pattern
    demonstrate_static_storage();
    
    // Demonstrate dependency injection pattern
    demonstrate_dependency_injection();
    
    // Show testing implications
    demonstrate_testing_differences();
}

fn demonstrate_static_storage() {
    println!("1. Static Storage Pattern:");
    
    // Must initialize global storage first
    initialize_global_storage();
    
    let doc_id = ID16::random();
    let mut properties = AdaptiveMap::new();
    properties.insert("title".to_string(), Value::String("Static Document".to_string()));
    
    // Clean API - no storage parameter needed
    StaticDeltaProcessor::apply_delta_static(doc_id, properties).unwrap();
    println!("   ✓ Created document using static storage");
    
    // Convenient access from anywhere
    let title = StaticDeltaProcessor::get_document_static(&doc_id).unwrap();
    println!("   ✓ Retrieved title: {:?}", title);
    
    // But what about concurrent access?
    println!("   ⚠ All operations must coordinate through global mutex");
    println!("   ⚠ Hidden dependency - functions look pure but aren't");
    println!();
}

fn demonstrate_dependency_injection() {
    println!("2. Dependency Injection Pattern:");
    
    let mut store = MemStore::new();
    let doc_id = ID16::random();
    let mut properties = AdaptiveMap::new();
    properties.insert("title".to_string(), Value::String("Injected Document".to_string()));
    
    // Explicit dependencies - clear what each function needs
    InjectedDeltaProcessor::apply_delta_injected(&mut store, doc_id, properties).unwrap();
    println!("   ✓ Created document with explicit storage dependency");
    
    let title = InjectedDeltaProcessor::get_document_injected(&store, &doc_id);
    println!("   ✓ Retrieved title: {:?}", title);
    
    // Can easily use multiple storage instances
    let mut store2 = MemStore::new();
    let mut redis_store = MockRedisStore::new();
    
    InjectedDeltaProcessor::apply_delta_injected(&mut store2, ID16::random(), AdaptiveMap::new()).unwrap();
    InjectedDeltaProcessor::apply_delta_injected(&mut redis_store, ID16::random(), AdaptiveMap::new()).unwrap();
    
    println!("   ✓ Can work with multiple storage instances simultaneously");
    println!("   ✓ No global state - each function call is explicit about dependencies");
    println!();
}

fn demonstrate_testing_differences() {
    println!("3. Testing Implications:");
    
    // Static storage testing challenges
    println!("   Static Storage Testing:");
    println!("   ⚠ Global state can leak between tests");
    println!("   ⚠ Must reset/reinitialize storage for each test");
    println!("   ⚠ Parallel tests can interfere with each other");
    
    // Dependency injection testing benefits
    println!("   Dependency Injection Testing:");
    println!("   ✓ Each test gets fresh storage instance");
    println!("   ✓ Tests are isolated and can run in parallel");
    println!("   ✓ Easy to inject mocks for different test scenarios");
    
    // Demonstrate isolated testing
    let mut test_store1 = MemStore::new();
    let mut test_store2 = MemStore::new();
    
    // These operations are completely isolated
    InjectedDeltaProcessor::apply_delta_injected(&mut test_store1, ID16::random(), AdaptiveMap::new()).unwrap();
    InjectedDeltaProcessor::apply_delta_injected(&mut test_store2, ID16::random(), AdaptiveMap::new()).unwrap();
    
    println!("   ✓ Isolated test operations completed");
    println!();
}

/// Mock storage implementation for demonstration
struct MockRedisStore {
    data: HashMap<ID16, AdaptiveMap<String, Value>>,
}

impl MockRedisStore {
    fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
}

impl DocumentStorage for MockRedisStore {
    fn get_document(&self, id: &ID16) -> Option<&AdaptiveMap<String, Value>> {
        self.data.get(id)
    }
    
    fn get_document_mut(&mut self, id: &ID16) -> Option<&mut AdaptiveMap<String, Value>> {
        self.data.get_mut(id)
    }
    
    fn create_document(&mut self, id: ID16, properties: AdaptiveMap<String, Value>) -> Result<(), String> {
        self.data.insert(id, properties);
        println!("   📡 MockRedisStore: Document created (simulated Redis operation)");
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
        // Simplified implementation
        Ok(())
    }
    
    fn remove_child_relationship(&mut self, parent_id: ID16, child_id: ID16) -> Result<(), String> {
        // Simplified implementation
        Ok(())
    }
}

/// Performance comparison between approaches
#[allow(dead_code)]
mod performance_analysis {
    use super::*;
    
    /// Static storage has overhead from mutex locking
    pub fn static_storage_overhead() {
        // Every operation must:
        // 1. Get global storage reference
        // 2. Acquire mutex lock
        // 3. Perform operation
        // 4. Release lock
        
        // This creates contention under high concurrency
    }
    
    /// Dependency injection has no locking overhead
    pub fn injected_storage_performance() {
        // Direct access to storage instance
        // No global coordination required
        // Better performance under high concurrency
    }
} 