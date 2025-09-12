use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use arc_swap::ArcSwap;
use dashmap::DashMap;

use crate::types::{ FieldDescriptor };



/// Immutable schema snapshot - never changes after creation
/// This is the core versioned data that can be safely shared with zero-copy
pub struct ImmutableSchema {
    /// Version number (monotonically increasing for actual schema changes)
    pub version: u32,
    
    /// Immutable field descriptors - never modified after creation
    /// Using Arc<[T]> instead of Vec for true immutability and zero-copy sharing
    pub fields: Arc<[Option<FieldDescriptor>]>,
}

impl ImmutableSchema {
    /// Create a new immutable schema from a vector of fields
    pub fn new(version: u32, fields: Vec<Option<FieldDescriptor>>) -> Self {
        Self {
            version,
            fields: fields.into_boxed_slice().into(),
        }
    }
    
    /// Get field by index - zero-copy access
    pub fn get_field(&self, index: u32) -> Option<&FieldDescriptor> {
        self.fields
            .get(index as usize)
            .and_then(|opt| opt.as_ref())
    }
    
    /// Get the number of fields in this schema
    pub fn len(&self) -> usize {
        self.fields.len()
    }
}

/// Cached schema version with pending changes and lookup acceleration
/// This wraps an immutable schema with mutable additions for performance
pub struct CachedSchemaVersion {
    /// The immutable base schema - shared, never changes
    pub base: Arc<ImmutableSchema>,
    
    /// Pending fields awaiting consolidation
    /// All new fields are added here to avoid mutations
    pub pending: Arc<DashMap<u32, FieldDescriptor>>,
    
    /// Combined lookup: path -> index for fast encoding
    /// Includes both base and pending fields
    pub path_lookup: Arc<DashMap<String, u32>>,
    
    /// Next available field index
    pub next_index: AtomicU32,
}

impl CachedSchemaVersion {
    /// Create a new cached version from an immutable base
    pub fn new(base: Arc<ImmutableSchema>) -> Self {
        let next_index = base.len() as u32;
        
        Self {
            base,
            pending: Arc::new(DashMap::new()),
            path_lookup: Arc::new(DashMap::new()),
            next_index: AtomicU32::new(next_index),
        }
    }
    
    /// Create from existing with new base schema
    pub fn with_new_base(base: Arc<ImmutableSchema>, existing: &CachedSchemaVersion) -> Self {
        Self {
            base,
            pending: Arc::new(DashMap::new()),
            path_lookup: existing.path_lookup.clone(), // Share the lookup
            next_index: AtomicU32::new(existing.next_index.load(Ordering::Acquire)),
        }
    }
    
    /// Get field descriptor by index - checks both base and pending
    pub fn get_field(&self, index: u32) -> Option<FieldDescriptor> {
        // Check immutable base first (fast path)
        if let Some(field) = self.base.get_field(index) {
            return Some(field.clone());
        }
        
        // Check pending (slower path)
        self.pending.get(&index).map(|v| v.value().clone())
    }
    
    /// Lookup field index by path
    pub fn get_field_index(&self, path: &str) -> Option<u32> {
        self.path_lookup.get(path).map(|v| *v.value())
    }
    
    /// Get the underlying schema version
    pub fn version(&self) -> u32 {
        self.base.version
    }
}

/// Thread-safe schema registry with versioning support
/// Manages field mappings with a hybrid immutable/pending approach
pub struct SchemaRegistry {
    /// Current cached version using ArcSwap for clean atomic swapping
    current: ArcSwap<CachedSchemaVersion>,
    
    /// Historical immutable schemas for backward compatibility
    /// Maps version number to immutable schema for O(1) historical lookup
    history: Arc<DashMap<u32, Arc<ImmutableSchema>>>,
    
    /// Next version number to assign
    next_version: AtomicU32,
    
    /// Flag to prevent concurrent consolidations
    consolidating: Arc<AtomicBool>,  // Wrap in Arc so it can be shared
    
    /// Threshold for triggering consolidation
    consolidation_threshold: usize,
}

impl SchemaRegistry {
    /// Create a new schema registry
    pub fn new() -> Self {
        let initial_schema = Arc::new(ImmutableSchema::new(0, vec![]));
        let initial_cached = CachedSchemaVersion::new(initial_schema.clone());
        
        let history = DashMap::new();
        history.insert(0, initial_schema);
        
        Self {
            current: ArcSwap::from(Arc::new(initial_cached)),
            history: Arc::new(history),
            next_version: AtomicU32::new(1),
            consolidating: Arc::new(AtomicBool::new(false)),
            consolidation_threshold: 100, // Consolidate after 100 pending fields
        }
    }
    
    /// Get current cached schema version for reading
    pub fn current_version(&self) -> Arc<CachedSchemaVersion> {
        self.current.load().clone()
    }
    
    /// Add a new field to the schema - always non-blocking
    pub fn add_field(self: &Arc<Self>, descriptor: FieldDescriptor) -> u32 {
        let current = self.current.load();
        
        // Atomically claim next index
        let index = current.next_index.fetch_add(1, Ordering::AcqRel);
        
        // Always add to pending map (no unsafe code!)
        current.pending.insert(index, descriptor.clone());
        current.path_lookup.insert(descriptor.path.clone(), index);
        
        // Trigger consolidation if pending gets large
        if current.pending.len() > self.consolidation_threshold {
            self.try_consolidate();
        }
        
        index
    }
    
    /// Get field by index from current version
    pub fn get_field(&self, index: u32) -> Option<FieldDescriptor> {
        self.current.load().get_field(index)
    }
    
    /// Get field by index from specific historical version
    pub fn get_field_at_version(&self, index: u32, version: u32) -> Option<FieldDescriptor> {
        // Check current first
        let current = self.current.load();
        if version == current.version() {
            return current.get_field(index);
        }
        
        // Check history for immutable schema
        self.history
            .get(&version)
            .and_then(|schema| schema.get_field(index).cloned())
    }
    
    /// Lookup field index by path in current version
    pub fn get_field_index(&self, path: &str) -> Option<u32> {
        self.current.load().get_field_index(path)
    }
    
    /// Lookup field index by path in specific version
    /// Note: This requires the path_lookup to be rebuilt for historical versions
    pub fn get_field_index_at_version(&self, path: &str, version: u32) -> Option<u32> {
        let current = self.current.load();
        if version == current.version() {
            return current.get_field_index(path);
        }
        
        // For historical versions, need to search the immutable schema
        self.history.get(&version).and_then(|schema| {
            // Linear search through fields (could be optimized with cached lookups)
            for (idx, field_opt) in schema.fields.iter().enumerate() {
                if let Some(field) = field_opt {
                    if field.path == path {
                        return Some(idx as u32);
                    }
                }
            }
            None
        })
    }
    
    /// Try to consolidate pending fields into immutable schema
    fn try_consolidate(self: &Arc<Self>) {
        // Only one consolidation at a time
        if self.consolidating.compare_exchange(
            false,
            true,
            Ordering::Acquire,
            Ordering::Relaxed
        ).is_err() {
            return;
        }
        
        let current = self.current.load();
        let current_arc = Arc::clone(&*current);  
        let registry = Arc::clone(self);  // Clone the Arc<SchemaRegistry>
        // Consolidate in background thread
        std::thread::spawn(move || {
            registry.consolidate_background(current_arc);
            registry.consolidating.store(false, Ordering::Release);
        });
    }
    
    /// Background consolidation - creates new immutable schema with merged fields
    fn consolidate_background(&self, current: Arc<CachedSchemaVersion>) {
        let pending_count = current.pending.len();
        if pending_count == 0 {
            return; // Nothing to consolidate
        }
        
        let base_len = current.base.len();
        let total_fields = base_len + pending_count;
        let new_capacity = total_fields.next_power_of_two();
        
        // Build new fields vector
        let mut new_fields = Vec::with_capacity(new_capacity);
        
        // Copy from immutable base (zero-copy potential here with SIMD)
        new_fields.extend_from_slice(&current.base.fields);
        
        // Sort and add pending fields
        let mut pending_sorted: Vec<(u32, FieldDescriptor)> = 
            current.pending.iter()
                .map(|e| (*e.key(), e.value().clone()))
                .collect();
        pending_sorted.sort_by_key(|&(idx, _)| idx);
        
        // Fill gaps and add pending fields to vector
        for (idx, descriptor) in pending_sorted {
            while new_fields.len() < idx as usize {
                new_fields.push(None); // Fill gaps
            }
            if idx as usize == new_fields.len() {
                new_fields.push(Some(descriptor));
            }
        }
        
        // Create new immutable schema (same version - just storage optimization)
        let new_immutable = Arc::new(ImmutableSchema::new(
            current.base.version,
            new_fields
        ));
        
        // Create new cached version with empty pending
        let new_cached = CachedSchemaVersion::with_new_base(
            new_immutable,
            &current
        );
        
        // Atomic swap to new consolidated version
        self.current.store(Arc::new(new_cached));
    }
    
    /// Create a new schema version with field reordering
    /// This is the only operation that actually changes the version number
    pub fn create_optimized_version(&self) -> u32 {
        let current = self.current.load();
        
        // TODO: Implement field reordering based on access patterns
        // This would:
        // 1. Analyze field access patterns
        // 2. Reorder fields so hot fields get lower indices (fewer bytes on wire)
        // 3. Create truly new version with new field ordering
        // 4. Store old version in history for backward compatibility
        
        // let new_version_num = self.next_version.fetch_add(1, Ordering::AcqRel);
        
        // Store current immutable schema in history
        self.history.insert(current.base.version, current.base.clone());
        
        // For now, just return current version
        current.base.version
    }
    
    /// Get historical immutable schema by version number
    pub fn get_historical_schema(&self, version: u32) -> Option<Arc<ImmutableSchema>> {
        // Check current first
        let current = self.current.load();
        if version == current.base.version {
            return Some(current.base.clone());
        }
        
        // Check history
        self.history.get(&version).map(|v| v.clone())
    }
}

// impl Clone for SchemaRegistry {
//     fn clone(&self) -> Self {
//         Self {
//             current: ArcSwap::from(self.current.load().clone()),
//             history: self.history.clone(),
//             next_version: AtomicU32::new(self.next_version.load(Ordering::Acquire)),
//             consolidating: AtomicBool::new(false), // Reset consolidating flag for clone
//             consolidation_threshold: self.consolidation_threshold,
//         }
//     }
// }

impl SchemaRegistry {
    // TODO: Update these methods for the new parameter system
    /*
    /// Encode a field reference with optional parameters to wire format using TLV
    pub fn encode(&self, field_index: u32, params: &Params) -> Vec<u8> {
        let mut output = Vec::with_capacity(16);
        
        // Schema version (2 bytes, big-endian)
        let version = self.current_version();
        output.push((version.version() >> 8) as u8);
        output.push((version.version() & 0xFF) as u8);
        
        // Field index (1-3 bytes varint)
        self.encode_varint(field_index, &mut output);
        
        // Parameter count (1 byte) - 0 for most fields
        output.push(params.params.len() as u8);
        
        // Encode each parameter as TLV
        for param in &params.params {
            // Type byte
            output.push(param.type_byte());
            
            // Length and value based on type
            match param {
                Param::Parent(idx) => {
                    output.push(1);  // Length = 1 varint
                    self.encode_varint(*idx, &mut output);
                }
                Param::ArrayIndex(idx) => {
                    output.push(1);  // Length = 1 varint
                    self.encode_varint(*idx, &mut output);
                }
                Param::ArrayRange(start, end) => {
                    output.push(2);  // Length = 2 varints
                    self.encode_varint(*start, &mut output);
                    self.encode_varint(*end, &mut output);
                }
                Param::ArrayIndices(indices) => {
                    self.encode_varint(indices.len() as u32, &mut output);  // Length
                    self.encode_varint(indices.len() as u32, &mut output);  // Count
                    for idx in indices {
                        self.encode_varint(*idx, &mut output);
                    }
                }
                Param::ArrayRanges(ranges) => {
                    self.encode_varint((ranges.len() * 2) as u32, &mut output);  // Length
                    self.encode_varint(ranges.len() as u32, &mut output);  // Count
                    for (start, end) in ranges {
                        self.encode_varint(*start, &mut output);
                        self.encode_varint(*end, &mut output);
                    }
                }
                Param::MapKey(key) => {
                    self.encode_varint(key.len() as u32, &mut output);  // Length
                    output.extend_from_slice(key.as_bytes());
                }
            }
        }
        
        output
    }
    */
    
    // TODO: Update for new parameter system
    /*
    /// Decode wire format to field descriptor and parameters using TLV
    pub fn decode(&self, bytes: &[u8]) -> Option<(FieldDescriptor, Params)> {
        if bytes.len() < 4 {
            return None;  // Minimum: 2 version + 1 index + 1 param count
        }
        
        let mut offset = 0;
        
        // Schema version
        let wire_version = ((bytes[0] as u32) << 8) | (bytes[1] as u32);
        offset += 2;
        
        // Get appropriate schema version (current or historical)
        let schema = if wire_version == self.current_version().version() {
            self.current_version()
        } else {
            self.get_historical_version(wire_version)?
        };
        
        // Field index
        let (field_index, consumed) = self.decode_varint(&bytes[offset..])?;
        offset += consumed;
        
        // Get field descriptor
        let field = schema.get_field(field_index)?.clone();
        
        // Parameter count
        let param_count = bytes[offset];
        offset += 1;
        
        // Decode TLV parameters
        let mut params = Params { params: Vec::new() };
        
        for _ in 0..param_count {
            if offset >= bytes.len() {
                return None;  // Incomplete parameter
            }
            
            // Type byte
            let param_type = bytes[offset];
            offset += 1;
            
            // Length
            let (length, consumed) = self.decode_varint(&bytes[offset..])?;
            offset += consumed;
            
            // Value based on type
            let param = match param_type {
                0x01 => {  // Parent
                    let (idx, consumed) = self.decode_varint(&bytes[offset..])?;
                    offset += consumed;
                    Param::Parent(idx)
                }
                0x02 => {  // ArrayIndex
                    let (idx, consumed) = self.decode_varint(&bytes[offset..])?;
                    offset += consumed;
                    Param::ArrayIndex(idx)
                }
                0x03 => {  // ArrayRange
                    let (start, consumed) = self.decode_varint(&bytes[offset..])?;
                    offset += consumed;
                    let (end, consumed) = self.decode_varint(&bytes[offset..])?;
                    offset += consumed;
                    Param::ArrayRange(start, end)
                }
                0x04 => {  // ArrayIndices
                    let (count, consumed) = self.decode_varint(&bytes[offset..])?;
                    offset += consumed;
                    let mut indices = Vec::with_capacity(count as usize);
                    for _ in 0..count {
                        let (idx, consumed) = self.decode_varint(&bytes[offset..])?;
                        offset += consumed;
                        indices.push(idx);
                    }
                    Param::ArrayIndices(indices)
                }
                0x05 => {  // ArrayRanges
                    let (count, consumed) = self.decode_varint(&bytes[offset..])?;
                    offset += consumed;
                    let mut ranges = Vec::with_capacity(count as usize);
                    for _ in 0..count {
                        let (start, consumed) = self.decode_varint(&bytes[offset..])?;
                        offset += consumed;
                        let (end, consumed) = self.decode_varint(&bytes[offset..])?;
                        offset += consumed;
                        ranges.push((start, end));
                    }
                    Param::ArrayRanges(ranges)
                }
                0x06 => {  // MapKey
                    let key = String::from_utf8(
                        bytes[offset..offset + length as usize].to_vec()
                    ).ok()?;
                    offset += length as usize;
                    Param::MapKey(key)
                }
                _ => return None,  // Unknown parameter type
            };
            
            params.params.push(param);
        }
        
        Some((field, params))
    }
    */
    
    /// Variable-length integer encoding (1-3 bytes)
    /// Encode a 32-bit value as variable-length integer
    fn encode_varint(&self, value: u32, output: &mut Vec<u8>) {
        if value < 128 {
            // 1 byte: 0xxxxxxx
            output.push(value as u8);
        } else if value < 16384 {
            // 2 bytes: 10xxxxxx xxxxxxxx
            output.push(0x80 | ((value >> 8) as u8));
            output.push((value & 0xFF) as u8);
        } else {
            // 3 bytes: 11xxxxxx xxxxxxxx xxxxxxxx
            output.push(0xC0 | ((value >> 16) as u8));
            output.push(((value >> 8) & 0xFF) as u8);
            output.push((value & 0xFF) as u8);
        }
    }
    
    /// Variable-length integer decoding
    /// Decode a variable-length integer from bytes
    fn decode_varint(&self, bytes: &[u8]) -> Option<(u32, usize)> {
        if bytes.is_empty() {
            return None;
        }
        
        let first = bytes[0];
        if first < 128 {
            // 1 byte
            Some((first as u32, 1))
        } else if first < 192 && bytes.len() >= 2 {
            // 2 bytes
            let value = ((first & 0x3F) as u32) << 8 | bytes[1] as u32;
            Some((value, 2))
        } else if bytes.len() >= 3 {
            // 3 bytes
            let value = ((first & 0x3F) as u32) << 16 
                      | (bytes[1] as u32) << 8 
                      | bytes[2] as u32;
            Some((value, 3))
        } else {
            None
        }
    }
    
    /// Get a historical schema version by number
    fn get_historical_version(&self, version: u32) -> Option<Arc<CachedSchemaVersion>> {
        // Check current first
        let current = self.current.load();
        if version == current.version() {
            return Some(current.clone());
        }
        
        // Check history for immutable schema and wrap it in a cached version
        self.history.get(&version).map(|immutable_schema| {
            Arc::new(CachedSchemaVersion::new(immutable_schema.clone()))
        })
    }
}

// fn example_usage() {
//     // Create registry
//     let registry = SchemaRegistry::new();
    
//     // Add simple field
//     let user_id = registry.add_field(
//         FieldDescriptor::new("user.id".to_string(), ValueType::String)
//     );
    
//     // Add array template field (detected from [] in path)
//     let items_field = registry.add_field(
//         FieldDescriptor::new("items[].name".to_string(), ValueType::String)
//     );
    
//     // Example 1: Simple field with no parameters (most common case)
//     let encoded = registry.encode(user_id, &Params { params: vec![] });
//     // Wire: [version:2][index:1][count:0] = 4 bytes total
    
//     // Example 2: Multiple array indices
//     let multi_indices = registry.encode(items_field, &Params {
//         params: vec![
//             Param::ArrayIndices(vec![5, 12, 23, 45])
//         ]
//     });
//     // Wire: [version:2][index:1-3][count:1][type:0x04][length][count:4][5][12][23][45]
    
//     // Example 3: Multiple array ranges (complex selection)
//     let multi_ranges = registry.encode(items_field, &Params {
//         params: vec![
//             Param::ArrayRanges(vec![(0, 5), (10, 15), (20, 25)])
//         ]
//     });
//     // Wire: [version:2][index:1-3][count:1][type:0x05][length][count:3][0][5][10][15][20][25]
    
//     // Example 4: Nested with parent and array operations
//     let complex = registry.encode(items_field, &Params {
//         params: vec![
//             Param::Parent(89),  // Applied first
//             Param::ArrayIndex(0),  // Then array index
//             Param::MapKey("primary".to_string()),  // Then map key
//         ]
//     });
//     // Parameters are applied in order for proper nesting
    
//     // Decode
//     if let Some((field, params)) = registry.decode(&encoded) {
//         println!("Decoded field: {} (type: {:?})", field.path, field.value_type);
//         println!("Parameters: {} params", params.params.len());
//     }
// }