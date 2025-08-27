
# Document Meta and Property Index Architecture

### Core Design Philosophy
The Document Meta system is handle special data mappings. It facilitates delta wire format optimisation through a bytecode to property index. 

### Document Meta Structure
```rust
struct DocumentMeta {
    // Append-only property registry for wire stability
    property_index: Vec<PropertyEntry>,
    
    // Optimised structures (can be rebuilt/modified)
    lookup_indices: DashMap<IndexId, IndexDescriptor>,
    
    // References to global structures (cross-document indices)
    global_refs: Vec<GlobalIndexRef>,
    
    // Future extensibility
    extensions: DashMap<String, MetaExtension>,
}
```

Each document maintains its own meta structure, eliminating global namespace exhaustion whilst enabling document-specific optimisations. The property index remains append-only to ensure delta stability, whilst lookup indices can be rebuilt or reorganised without affecting wire format compatibility.

### Property Index with Pattern System

The property index uses patterns to represent unbounded structures efficiently:

```rust
enum PropertyEntry {
    Static(&'static str),                    // "title" - simple field
    ArrayPattern(String, Box<PropertyEntry>), // "users[*].name" - all array elements
    MapPattern(String, Box<PropertyEntry>),   // "events.{}.timestamp" - dynamic keys
    RangePattern(String, usize, usize),       // "tensor[0..1000000]" - fixed range
}
```

This pattern system solves the unbounded key problem:
- **Arrays**: `users[*].name` represents unlimited array elements with one entry
- **Maps**: `preferences.{userId}.theme` handles dynamic keys
- **Time-series**: `readings.{timestamp}.value` for temporal data
- **Sparse matrices**: `matrix.{row,col}` for coordinate-based access

### PropertyId Encoding

PropertyIds use variable-length encoding for efficient wire format:
- **0-127**: Single byte (0xxxxxxx) - most common properties
- **128-32,767**: Two bytes (1xxxxxxx xxxxxxxx) - extended properties
- **32,768+**: Three bytes (11xxxxxx xxxxxxxx xxxxxxxx) - rare properties

This encoding keeps common operations compact whilst supporting unlimited properties.

### Delta Wire Format with Patterns

Deltas targeting pattern-based properties include parameters:

```rust
struct DeltaPayload {
    pattern_id: PropertyId,       // Which pattern (1-3 bytes)
    param_length: u16,            // Length of parameters (2 bytes)
    parameters: Vec<u8>,          // Encoded parameters (indices, keys, etc.)
    value_length: u32,            // Length of value (4 bytes)
    value: Vec<u8>,              // Actual value
}
```

Examples:
- **Update `users[5].name`**: pattern_id=1, parameters=[5], value="Alice"
- **Update `events["uuid-123"].timestamp`**: pattern_id=2, parameters="uuid-123", value=timestamp
- **Update `matrix[100,200]`**: pattern_id=3, parameters=[100,200], value=3.14

The parameters are encoded in minimal bytes - array indices as varints, string keys with length prefixes, ensuring wire efficiency.

### Lookup Indices and Query Acceleration

Separate from the wire-stable property index, lookup indices provide query optimisation:

```rust
struct IndexDescriptor {
    pattern: PropertyEntry,                    // What we're indexing
    index_type: IndexType,                    // BTree, Hash, Bitmap
    data: Arc<DashMap<Vec<u8>, Vec<PropertyId>>>, // Index data
}

enum IndexType {
    BTree,      // Range queries
    Hash,       // Exact match
    Bitmap,     // Set membership
    Statistical, // Future: statistical inference structures
}
```

These indices can be:
- Created and dropped without affecting deltas
- Rebuilt for optimisation
- Shared across documents via global references
- Extended for statistical inference (future capability)

### Global References

Documents can reference cross-document structures:

```rust
struct GlobalIndexRef {
    index_id: GlobalIndexId,
    index_type: GlobalIndexType,
    scope: IndexScope,  // Which documents participate
}
```

This enables:
- Indices spanning multiple documents
- Shared statistical models
- Distributed query execution
- Cross-document inference

### Evolution Path

The Document Meta architecture is designed for future extension:

1. **Current**: Basic property patterns and simple indices
2. **Next**: Query compilation and optimisation
3. **Future**: Statistical inference as first-class citizen
   - Probabilistic indices
   - Learned access patterns
   - Automatic index selection
   - Cross-document correlations

The separation between wire-stable property indices and flexible lookup structures ensures the system can evolve without breaking compatibility.

