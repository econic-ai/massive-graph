# Massive Graph: Property Index Implementation

## Overview
The Property Index is a core component of Massive Graph's document storage system. It enables efficient pattern-based property access while maintaining a finite registry size, solving the problem of unbounded property proliferation in large-scale data structures (arrays, maps, tensors).

---

## PropertyId: Compact Wire Format Encoding

### Purpose
Provides variable-length encoding for property IDs to minimize wire format overhead while supporting up to 32K unique patterns per document.

### Design Constraints
- Must minimize bytes on wire (1 byte for common patterns, 2 bytes for rare ones)
- First 128 patterns use single byte (most common case)
- Supports up to 32,768 unique patterns per document
- Encoding/decoding must be deterministic and fast

```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicU16, AtomicUsize, AtomicPtr, Ordering};
use dashmap::DashMap;
use bytes::{Bytes, BytesMut, BufMut};

/// Variable-length property ID encoding
/// - 0-127: Single byte (0xxxxxxx)
/// - 128-32K: Two bytes (1xxxxxxx xxxxxxxx)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PropertyId(u16);

impl PropertyId {
    pub fn encode(&self, buf: &mut BytesMut) {
        if self.0 < 128 {
            buf.put_u8(self.0 as u8);
        } else {
            buf.put_u8((self.0 >> 8) as u8 | 0x80);
            buf.put_u8(self.0 as u8);
        }
    }
}
```

---

## PropertyPattern: Template-Based Property Definitions

### Purpose
Defines reusable patterns that can represent infinite concrete properties with a finite registry entry. One pattern like `"users[*].name"` can represent millions of actual user names.

### Design Constraints
- Must support arbitrarily nested structures (arrays within maps within arrays)
- Pattern registration happens rarely (at schema definition time)
- Must be able to represent any JSON-like path structure
- Template string for human readability and debugging
- Fixed param_types array for type safety and validation

```rust
/// Pattern templates for property paths
#[derive(Debug, Clone)]
pub struct PropertyPattern {
    /// Template string like "users[*].classDict.{}.assessments[*].score"
    template: String,
    
    /// Types needed to fill in the template's dynamic parts
    param_types: Vec<ParamType>,
}

#[derive(Debug, Clone)]
pub enum ParamType {
    Index,      // Array index [*] - u32
    Key,        // Map key {} - String/Bytes
    Coords(u8), // Tensor coordinates [*,*,*] - Vec<u32> with dimension count
}

impl PropertyPattern {
    /// Create a new pattern with explicit template and types
    pub fn new(template: String, param_types: Vec<ParamType>) -> Self {
        PropertyPattern { template, param_types }
    }
    
    /// Convenience builder for common patterns
    pub fn array(base: &str, element: &str) -> Self {
        PropertyPattern {
            template: format!("{}[*].{}", base, element),
            param_types: vec![ParamType::Index],
        }
    }
    
    pub fn map(base: &str, value: &str) -> Self {
        PropertyPattern {
            template: format!("{}.{{}}.{}", base, value),
            param_types: vec![ParamType::Key],
        }
    }
    
    pub fn nested(template: String, param_types: Vec<ParamType>) -> Self {
        PropertyPattern { template, param_types }
    }
}
```

---

## PropertyIndex: Lock-Free Pattern Registry

### Purpose
Maintains the mapping from PropertyId to PropertyPattern with O(1) lock-free lookups and rare resize operations.

### Design Constraints
- Reads must NEVER block (high-performance requirement)
- PropertyId must equal vector index for O(1) lookup
- Patterns are write-once (never modified after registration)
- Registry size is small (typically < 1000 patterns per document)
- Resize is extremely rare (only when exceeding initial capacity)
- Must support concurrent pattern registration

### Implementation Details
- Uses `ArcSwap<Vec<AtomicPtr>>` for lock-free reads with safe resize
- Pre-allocates capacity to minimize resizes
- Resize uses mutex but only blocks writers, never readers
- AtomicPtr allows write-once semantics without locking

```rust
use arc_swap::ArcSwap;
use std::sync::Mutex;

/// Per-document property registry (append-only, lock-free reads)
pub struct PropertyIndex {
    /// Pattern registry - swappable for resize operations
    patterns: ArcSwap<Vec<AtomicPtr<PropertyPattern>>>,
    
    /// Reverse lookup: pattern string -> ID
    pattern_lookup: DashMap<String, PropertyId>,
    
    /// Next available PropertyId
    next_id: AtomicU16,
    
    /// Current capacity
    capacity: AtomicUsize,
    
    /// Lock ONLY for resize operations
    resize_lock: Mutex<()>,
}

impl PropertyIndex {
    pub fn new(initial_capacity: usize) -> Self {
        let mut patterns = Vec::with_capacity(initial_capacity);
        for _ in 0..initial_capacity {
            patterns.push(AtomicPtr::new(std::ptr::null_mut()));
        }
        
        PropertyIndex {
            patterns: ArcSwap::from_pointee(patterns),
            pattern_lookup: DashMap::new(),
            next_id: AtomicU16::new(0),
            capacity: AtomicUsize::new(initial_capacity),
            resize_lock: Mutex::new(()),
        }
    }
    
    pub fn register_pattern(&self, pattern: PropertyPattern) -> PropertyId {
        let pattern_str = format!("{:?}", pattern);
        
        // Check if already exists
        if let Some(id) = self.pattern_lookup.get(&pattern_str) {
            return *id;
        }
        
        // Allocate new ID
        let id_val = self.next_id.fetch_add(1, Ordering::SeqCst);
        let id = PropertyId(id_val);
        
        // Check if resize needed (rare)
        if id_val as usize >= self.capacity.load(Ordering::Acquire) {
            // Lock only for resize
            let _guard = self.resize_lock.lock().unwrap();
            
            // Double-check after acquiring lock
            if id_val as usize >= self.capacity.load(Ordering::Acquire) {
                let old_capacity = self.capacity.load(Ordering::Acquire);
                let new_capacity = (old_capacity * 2).max(id_val as usize + 1);
                
                // Load current vector
                let current = self.patterns.load();
                
                // Create new vector with increased capacity
                let mut new_vec = Vec::with_capacity(new_capacity);
                
                // Copy existing AtomicPtr values
                for i in 0..old_capacity {
                    let ptr = current[i].load(Ordering::Acquire);
                    new_vec.push(AtomicPtr::new(ptr));
                }
                
                // Fill rest with null pointers
                for _ in old_capacity..new_capacity {
                    new_vec.push(AtomicPtr::new(std::ptr::null_mut()));
                }
                
                // Atomically swap in the new vector
                self.patterns.store(Arc::new(new_vec));
                self.capacity.store(new_capacity, Ordering::Release);
            }
            // Lock released here
        }
        
        // Store pattern - write once, never modified
        // Re-load patterns in case it was resized
        let patterns = self.patterns.load();
        let pattern_box = Box::into_raw(Box::new(pattern.clone()));
        patterns[id_val as usize].store(pattern_box, Ordering::Release);
        
        // Update lookup
        self.pattern_lookup.insert(pattern_str, id);
        
        id
    }
    
    /// Get pattern by ID - O(1) lock-free lookup
    pub fn get_pattern(&self, id: PropertyId) -> Option<PropertyPattern> {
        let patterns = self.patterns.load();
        
        if id.0 as usize >= patterns.len() {
            return None;
        }
        
        let ptr = patterns[id.0 as usize].load(Ordering::Acquire);
        if ptr.is_null() {
            None
        } else {
            unsafe { Some((*ptr).clone()) }
        }
    }
}
```

---

## PropertyPath: Concrete Property Instances

### Purpose
Combines a PropertyPattern with specific runtime values to create a concrete property path that can be encoded as a HashMap key.

### Design Constraints
- Must support type-safe value specification
- Values must match the pattern's param_types in order and type
- Encoding must be deterministic and unique
- Encoded format must be compact for memory efficiency

```rust
/// Concrete property path: pattern + runtime values
pub struct PropertyPath {
    pattern_id: PropertyId,
    values: Vec<PathValue>,  // Must match pattern's dynamic segments
}

#[derive(Debug, Clone)]
pub enum PathValue {
    Index(u32),              // For Array segments
    Key(Bytes),              // For Map segments  
    Coords(Vec<u32>),        // For Tensor segments
}

impl PropertyPath {
    /// Encode path to bytes for use as HashMap key
    pub fn encode(&self, index: &PropertyIndex) -> Bytes {
        let mut buf = BytesMut::new();
        
        // Write pattern ID
        self.pattern_id.encode(&mut buf);
        
        // Write each dynamic value in order
        for value in &self.values {
            match value {
                PathValue::Index(i) => {
                    buf.put_u8(0x01);  // Type tag for array index
                    buf.put_u32(*i);
                },
                PathValue::Key(key) => {
                    buf.put_u8(0x02);  // Type tag for map key
                    buf.put_u16(key.len() as u16);
                    buf.put_slice(key);
                },
                PathValue::Coords(coords) => {
                    buf.put_u8(0x03);  // Type tag for tensor
                    buf.put_u8(coords.len() as u8);
                    for coord in coords {
                        buf.put_u32(*coord);
                    }
                },
            }
        }
        
        buf.freeze()
    }
}
```

---

## Document: The Storage Layer

### Purpose
Combines the PropertyIndex with actual data storage using encoded paths as keys. Every document has a Map at its root, providing uniform access patterns across all document types.

### Design Constraints
- Must support concurrent reads and writes without blocking readers
- Values stored in wire format (Arc<Vec<u8>>) for zero-copy transmission
- Encoded property paths serve as unique keys
- No structural reallocation needed for collection updates
- All documents use Map at root for consistency
- Iterations must be fast for wire transmission

### Document Structure
Every document in Massive Graph has a Map at its root, regardless of document type. This provides uniform access patterns and consistent wire format across all document types. A binary file, JSON document, or graph all use the same property-indexed Map structure at their root.

```rust
use arc_swap::ArcSwap;
use std::sync::atomic::{AtomicPtr, AtomicU64, Ordering};

/// Document storage with Map at root
pub struct Document {
    /// Document type (hint for expected schema)
    doc_type: DocumentType,
    
    /// Document metadata and indices
    meta: DocumentMeta,
    
    /// Document state (integrated with activity tracking)
    state: DocumentState,
    
    /// Document identifier
    doc_id: DocumentId,
    
    /// Creation timestamp
    created_at: AtomicU64,
    
    /// Last modification timestamp
    last_modified: AtomicU64,
    
    /// Property registry for this document
    index: Arc<PropertyIndex>,
    
    /// Actual data storage - lock-free map structure
    /// Map structure is immutable (cloned on new paths)
    /// Values are atomic pointers (swapped on updates)
    data: ArcSwap<HashMap<Bytes, AtomicPtr<Arc<Vec<u8>>>>>,
}

impl Document {
    pub fn new(doc_type: DocumentType, doc_id: DocumentId) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        Document {
            doc_type,
            meta: DocumentMeta::new(),
            state: DocumentState::new(),
            doc_id,
            created_at: AtomicU64::new(now),
            last_modified: AtomicU64::new(now),
            index: Arc::new(PropertyIndex::new(1000)),
            data: ArcSwap::from_pointee(HashMap::new()),
        }
    }
    
    pub fn set(&self, path: PropertyPath, value: Vec<u8>) {
        let encoded = path.encode(&self.index);
        let value_arc = Arc::new(value);
        
        let data = self.data.load();
        
        if let Some(existing_slot) = data.get(&encoded) {
            // Property exists - just update the value atomically
            // NO MAP CLONE NEEDED!
            let value_ptr = Arc::into_raw(value_arc);
            let old = existing_slot.swap(value_ptr, Ordering::Release);
            // Clean up old value
            if !old.is_null() {
                unsafe { Arc::from_raw(old); }
            }
        } else {
            // New property - need to clone map structure (rare)
            self.data.rcu(|old| {
                let mut new_map = (**old).clone();
                let value_ptr = AtomicPtr::new(Arc::into_raw(value_arc));
                new_map.insert(encoded, value_ptr);
                Arc::new(new_map)
            });
        }
        
        // Update last_modified
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.last_modified.store(now, Ordering::Relaxed);
    }
    
    pub fn get(&self, path: &PropertyPath) -> Option<Arc<Vec<u8>>> {
        let encoded = path.encode(&self.index);
        let data = self.data.load();
        
        data.get(&encoded).and_then(|ptr| {
            let value_ptr = ptr.load(Ordering::Acquire);
            if value_ptr.is_null() {
                None
            } else {
                unsafe { Some(Arc::clone(&*value_ptr)) }
            }
        })
    }
    
    /// Zero-copy iteration for wire transmission
    pub fn iter_for_wire(&self) -> impl Iterator<Item = (&Bytes, Arc<Vec<u8>>)> + '_ {
        let data = self.data.load();
        data.iter().filter_map(|(key, ptr)| {
            let value_ptr = ptr.load(Ordering::Acquire);
            if value_ptr.is_null() {
                None
            } else {
                unsafe { Some((key, Arc::clone(&*value_ptr))) }
            }
        })
    }
}
}
```

---

## Usage Examples

### Basic Usage
```rust
// Create document - always has Map at root
let doc = Document::new(DocumentType::Json);

// Register a complex nested pattern
let pattern = PropertyPattern::new(
    "users[*].classDict.{}.assessments[*].score".to_string(),
    vec![
        ParamType::Index,  // for users[*]
        ParamType::Key,    // for classDict.{}
        ParamType::Index,  // for assessments[*]
    ]
);
let pattern_id = doc.index.register_pattern(pattern);

// Set a specific value: users[42].classDict["Math101"].assessments[2].score = 95
let path = PropertyPath {
    pattern_id,
    values: vec![
        PathValue::Index(42),              // users[42]
        PathValue::Key("Math101".into()),  // classDict["Math101"]
        PathValue::Index(2),               // assessments[2]
    ],
};
// Store as Value::Integer
doc.set(path, serialize_value(Value::Integer(95)));

// Binary document example - still uses Map at root
let binary_doc = Document::new(DocumentType::Binary);

// Register patterns
let filename_pattern = PropertyPattern::new("filename".to_string(), vec![]);
let content_pattern = PropertyPattern::new("content".to_string(), vec![]);
let size_pattern = PropertyPattern::new("size".to_string(), vec![]);
let created_pattern = PropertyPattern::new("created_at".to_string(), vec![]);

// Set fields with different Value types
binary_doc.set(
    PropertyPath { pattern_id: filename_id, values: vec![] },
    serialize_value(Value::String("video.mp4".into()))
);
binary_doc.set(
    PropertyPath { pattern_id: content_id, values: vec![] },
    serialize_value(Value::Binary(video_data))
);
binary_doc.set(
    PropertyPath { pattern_id: size_id, values: vec![] },
    serialize_value(Value::Integer(1048576))
);
binary_doc.set(
    PropertyPath { pattern_id: created_id, values: vec![] },
    serialize_value(Value::Timestamp(1703001600))
);

// Serialization would encode Value type + data
fn serialize_value(value: Value) -> Vec<u8> {
    // Encodes [ValueType discriminant][data bytes]
    match value {
        Value::String(s) => { /* ValueType::String + length + bytes */ },
        Value::Integer(i) => { /* ValueType::Integer + i64 bytes */ },
        Value::Binary(b) => { /* ValueType::Binary + length + bytes */ },
        Value::Timestamp(t) => { /* ValueType::Timestamp + u64 bytes */ },
        // ... etc
    }
}
```

---

## Wire Format Example

### Encoding Breakdown
Shows how a complex property path is encoded for network transmission or storage.

```rust
/// Delta operation on wire
struct DeltaOp {
    encoded_path: Bytes,  // Pattern ID + index bytes
    value: Option<Arc<Vec<u8>>>,
}

// Example: users[42].classDict["Math101"].assessments[2].score = 95
// Pattern ID: 0 (assuming first pattern registered)
// Encoded as:
// [0x00]                    - Pattern ID 0 (1 byte since < 128)
// [0x01][0x00,0x00,0x00,0x2A] - Type tag 0x01 + index 42 (5 bytes)
// [0x02][0x00,0x07][M,a,t,h,1,0,1] - Type tag 0x02 + length 7 + "Math101" (10 bytes)
// [0x01][0x00,0x00,0x00,0x02] - Type tag 0x01 + index 2 (5 bytes)
// Total: 21 bytes for the path (vs ~45 bytes for the string path)
```

---

## Key Architecture Benefits

1. **Finite Registry Size**: Only patterns are registered, not individual elements
2. **O(1) Pattern Lookup**: Direct vector index access via PropertyId
3. **Lock-Free Reads**: No locks on read path, ever
4. **Compact Wire Format**: 1-2 byte pattern ID + minimal dynamic data
5. **Zero-Copy Values**: Arc<Vec<u8>> throughout the system
6. **No Structural Reallocation**: Adding/removing collection elements doesn't require copying
7. **Type Safety**: Pattern param_types ensure correct value types
8. **Scalability**: Can represent millions of properties with ~100 patterns