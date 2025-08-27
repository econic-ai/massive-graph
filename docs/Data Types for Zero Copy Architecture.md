# Data Types for Zero Copy Architecture

## Core Design Philosophy
The data architecture prioritises zero-copy operations to minimise memory allocations and data movement. This is critical for achieving millions of operations per second throughput. The challenge lies in balancing mutable operations with stable memory addresses - zero-copy requires memory stability, whilst document updates inherently want to change data. Our solution employs different strategies for different data patterns, optimising each for its specific use case.

The key innovation is that documents themselves are stored in wire-ready format. Rather than maintaining separate in-memory and wire representations, each property value is stored pre-encoded. This eliminates conversion overhead and enables true zero-copy network transmission - we simply iterate the document properties and send the raw bytes directly to the network.

## Document Types
Documents in Massive Graph are not limited to JSON-like structures. Different document types optimise for different data models whilst sharing the same underlying property index system:

```rust
enum DocumentType {
    Json = 0,        // Flattened key-value properties
    Stream = 1,      // Text/Binary/Document streams for media
    TextFile = 2,    // Line-based text for code collaboration
    Binary = 3,      // Raw bytes (videos, images, etc.)
    Graph = 4,       // Nodes and edges with properties
    Tensor = 5,      // N-dimensional arrays (embeddings, weights)
    Table = 6,       // Columnar data for analytics
    TimeSeries = 7,  // Time-indexed data points
    Geospatial = 8,  // Spatial data with coordinates
    Event = 9,       // Append-only audit logs
}
```

Each document type uses the property index system differently:
- **JSON**: Flattened paths like "user.profile.name" → PropertyId
- **Graph**: Node/edge references like "node.123.label", "edge.456.weight"
- **Tensor**: Dimensional data like "shape.0", "data.chunk.0"
- **Stream**: Sequential chunks "chunk.0", "chunk.1" with append-only semantics
- **Table**: Column-based "col.age.row.0", "col.name.row.0"

This flexibility allows Massive Graph to efficiently handle diverse data models whilst maintaining the zero-copy architecture and property index benefits across all document types.

## Document Structure

Documents in Massive Graph are built around a unified value system where any value can contain other values, enabling natural nesting and composition. A document can BE a stream, or it can be a JSON-like structure that CONTAINS streams, binaries, or other complex types as properties.

- **Root value system**: Document has a root value that can be any type
- **Natural nesting**: Maps can contain streams, arrays can contain maps, etc.
- **Wire-ready storage**: Values stored in wire-encodable format
- **Version tracking**: Each value change tracked with sequential versions

This unified approach means a document that IS a binary file and a document with a binary file property both use the same Value enum, just at different levels of the hierarchy.

## Property Index System
- **Append-only per-document registry**: Each document maintains its own property mappings
- **Pattern-based templates**: Handle unbounded structures like arrays, maps, time-series
- **PropertyId encoding**: 0-127 single byte, 128-32K two bytes via high-bit flag
- **Wire stability**: Append-only ensures deltas remain valid

The property index has evolved from a simple global mapping to a sophisticated per-document meta structure. Rather than exhausting a global namespace with array elements or map keys, each document defines patterns like `users[*].name` that can represent unlimited elements whilst using a single registry entry. This approach dramatically reduces registry size whilst maintaining delta efficiency. See Section 2 for detailed architecture.

## Data Type Strategies

### Atomic Primitives
- **Direct atomic operations**: Integers, booleans use compare-and-swap
- **No versioning needed**: AtomicU64, AtomicBool, etc.
- **Lock-free updates**: Natural atomic operations without wrapper

### Strings
- **Copy-on-Write**: New Arc allocation for each change
- **Atomic pointer swap**: Readers always see consistent state
- **No locks required**: Arc reference counting handles lifecycle

### Streams (Text/Binary/Document)
- **Append-only delta list**: Never modify existing chunks
- **Zero reallocation**: New deltas just append to list
- **Natural audit trail**: Document streams preserve full history
- **Wire format**: Send delta list for reconstruction

Streams recognise that different text patterns require different optimisation strategies. Standard text fields that change occasionally benefit from CoW simplicity, whilst streams like chat logs, audit trails, or activity feeds would suffer from CoW's memory amplification on large texts. Streams store the raw delta operations and reconstruct the full content only when needed.

## Array Handling

Arrays in the Value system are naturally handled through the Value::Array variant for simple arrays, or through patterns for complex indexed access:

- **Simple arrays**: Value::Array(Vec<VersionedValue>) for lists of values
- **Pattern-based access**: For sparse or very large arrays, patterns like `users[*]` in a Map
- **Replace is O(1)**: Direct value update in array or map
- **Append is O(1)**: Push to Vec or add to Map with next index

Arrays can contain any value type, including other arrays or maps, enabling multi-dimensional structures. The choice between Array and pattern-based Map storage depends on the access patterns and sparsity of the data.

## Versioned Value Structure

The core of the document system is the Value enum, which provides a unified type system for all data in Massive Graph:

```rust
struct VersionedValue {
    version: u64,              // Sequential version number
    value: Arc<Value>,         // The actual value, Arc for zero-copy sharing
}

enum Value {
    // Primitives
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(Arc<String>),
    Binary(Arc<Vec<u8>>),
    
    // Collections  
    Map(HashMap<PropertyId, VersionedValue>),
    Array(Vec<VersionedValue>),
    
    // Streams (append-only)
    TextStream(Vec<Arc<String>>),      // Chunks of text
    BinaryStream(Vec<Arc<Vec<u8>>>),   // Binary chunks  
    DocumentStream(Vec<DocumentId>),    // References to other docs
    
    // Geospatial
    Coordinate(f64, f64),              // Lat/lon pair
    Point3D(f64, f64, f64),            // X,Y,Z coordinate
    Polygon(Vec<Coordinate>),           // Closed shape
    LineString(Vec<Coordinate>),        // Path/route
    
    // Temporal
    Timestamp(u64),                     // Unix timestamp
    Duration(u64),                      // Time span in ms
    DateRange(u64, u64),               // Start/end timestamps
    
    // References
    DocumentRef(DocumentId),
    PropertyRef(PropertyId),
    DeltaRef(DeltaId),
    
    // Binary/Media (with metadata in parent Map if needed)
    Image(Arc<Vec<u8>>),
    Video(Arc<Vec<u8>>),
    Audio(Arc<Vec<u8>>),
    
    // Math/ML
    Vector(Arc<Vec<f32>>),             // 1D array of floats
    Matrix { rows: usize, cols: usize, data: Arc<Vec<f32>> },
    Tensor { shape: Vec<usize>, data: Arc<Vec<f32>> },
    
    // Special
}
```

This Value enum enables documents to have flexible structure - a document can be:
- A simple string (Document with root = Value::String)
- A stream (Document with root = Value::BinaryStream)
- A complex nested structure (Document with root = Value::Map containing other values)
- Any other value type

Each value change increments the version number, providing a total order of operations. The Arc allows multiple readers to share the same immutable data, with old versions naturally garbage collected when no longer referenced.

## Memory Lifecycle
- **Arc reference counting**: Automatic cleanup when refs drop to zero
- **No explicit version management**: Old versions cleaned up naturally
- **Copy-on-write safety**: Writers never affect active readers
- **Lazy document loading**: Documents materialised only when accessed

The architecture ensures memory safety without locks for read operations. Writers create new Arc-wrapped values, atomically swap pointers, and old values are automatically freed when all readers finish. This provides a clean separation between read and write paths without synchronisation overhead.

## Document Structure Definition
```rust
struct Document {
    // Document type determines how to interpret the root value
    doc_type: DocumentType,
    
    // The root value - can be ANY Value type
    root: VersionedValue,
    
    // Document metadata and indices
    meta: DocumentMeta,
    
    // Document state (integrated with activity tracking)
    state: DocumentState,
    
    // Document metadata
    doc_id: DocumentId,
    created_at: AtomicU64,
    last_modified: AtomicU64,
}

struct DocumentState {
    // State flags using bitflags in single atomic
    flags: AtomicU32,  // IDLE, DELTA_LOCKED, WIRE_STALE, etc.
    
    // Activity tracking for hot/cold classification
    delta_count: AtomicU32,
    last_access: AtomicU64,
    
    // Hot document dedicated queue (allocated on promotion)
    hot_queue: Option<MPSCQueue<Delta>>,
}
```

The Document structure is simply a container with metadata around a root value. The root value can be any type from the Value enum - a document that IS a stream has root = Value::BinaryStream, while a document with stream properties has root = Value::Map containing stream values. This unified approach eliminates the need for separate storage strategies per document type.

## Wire Transmission Format

The ValueType enum is the wire format representation of the Value enum. There is a one-to-one correspondence between Value variants and ValueType discriminants, ensuring seamless serialization and deserialization:

```rust
// Wire format type discriminants - matches Value enum variants exactly
enum ValueType {
    // Primitives
    Null = 0,
    Bool = 1,
    Integer = 2,
    Float = 3,
    String = 4,
    Binary = 5,
    
    // Collections
    Map = 6,
    Array = 7,
    
    // Streams
    TextStream = 8,      // Append-only text (chat, logs)
    BinaryStream = 9,    // Append-only binary (media chunks)
    DocumentStream = 10, // Append-only document references (audit trail)
    
    // Geospatial
    Coordinate = 11,     // Lat/lon pair
    Point3D = 12,        // X,Y,Z coordinate
    Polygon = 13,        // Closed shape
    LineString = 14,     // Path/route
    
    // Temporal
    Timestamp = 15,      // Point in time
    Duration = 16,       // Time span
    DateRange = 17,      // Start/end pair
    
    // References
    DocumentRef = 18,    // Reference to another document
    PropertyRef = 19,    // Reference to a property path
    DeltaRef = 20,       // Reference to a delta (for audit)
    
    // Binary/Media
    Image = 21,          // Image with metadata
    Video = 22,          // Video with metadata
    Audio = 23,          // Audio with metadata
    
    // Math/ML
    Vector = 24,         // 1D array of floats (embeddings)
    Matrix = 25,         // 2D array
    Tensor = 26,         // N-dimensional array
    
    // Special
}

struct WireDocument<'a> {
    // Fixed header with document metadata
    header: [u8; 32],  // doc_id, version, timestamp, root_value_type
    
    // The root value serialized
    root_data: &'a [u8],  // Borrowed reference for zero-copy
}

struct WireValue<'a> {
    value_type: u8,        // ValueType enum discriminant
    length: [u8; 4],       // Length of value bytes
    data: &'a [u8],        // Zero-copy reference to actual data
}
```

The relationship between Value and ValueType is fundamental: when serializing a Value for network transmission, its variant determines the ValueType discriminant in the wire format. This ensures receivers can correctly reconstruct the Value from the wire bytes without additional metadata. The wire format is self-describing through this type system.