# Document Type Architectures for Massive Graph

This document describes the specialized document type implementations built on top of the unified document architecture in Massive Graph. Each document type provides factory methods, validation, and helper functions while using the same underlying `Document` structure.

## Design Philosophy

All document types in Massive Graph follow the principle that **everything is a document**. Whether storing binary data, collaborative text files, or graph structures, they all use the same foundational architecture with properties and children. This unified approach:

- Eliminates the need for separate subsystems
- Ensures all types benefit from the same performance optimizations
- Maintains expressive power across different domains
- Simplifies the codebase through consistent patterns

## Document Type Overview

### 1. Binary Documents

**Purpose**: Storage of large binary data (videos, images, executables) with streaming capabilities.

**Architecture**:
```rust
Binary Document {
    doc_type: DocumentType::Binary,
    properties: {
        "filename" -> "video.mp4",
        "mime_type" -> "video/mp4", 
        "content" -> BinaryStream(chunks),
        "size" -> 1048576,
        "created_at" -> timestamp,
        "streaming" -> true,
    },
    children: [] // No children for binary documents
}
```

**Key Features**:
- **Chunk-based streaming** for large files
- **Range requests** for video seeking and partial downloads
- **Metadata properties** for MIME type, filename, etc.
- **Efficient append operations** for real-time data streams
- **Time-based queries** for accessing specific time ranges

**Use Cases**:
- Video and audio streaming with seek capabilities
- Large file storage with progressive download
- Real-time sensor data collection
- Binary log files with temporal access

**Performance Characteristics**:
- **Streaming**: O(1) append operations
- **Range queries**: O(log n) for timestamp-based access
- **Size overhead**: ~40 bytes metadata per chunk
- **Concurrency**: Lock-free reads during writes

### 2. Text Documents

**Purpose**: Simple text content with basic metadata, optimized for non-collaborative scenarios.

**Architecture**:
```rust
Text Document {
    doc_type: DocumentType::Text,
    properties: {
        "content" -> "My note content here",
        "length" -> 20,
        "language" -> "en",
        "content_type" -> "plain",
        "created_at" -> timestamp,
        "modified_at" -> timestamp,
    },
    children: [] // No children for simple text
}
```

**Key Features**:
- **Direct string storage** for simplicity
- **Language and encoding metadata** for internationalization
- **Content type specification** (plain, markdown, HTML)
- **Automatic length tracking** for quick size queries
- **Simple append/prepend operations**

**Use Cases**:
- Notes and documentation
- Configuration files
- Simple markup content
- Non-collaborative text editing

**Performance Characteristics**:
- **Access**: O(1) for full content retrieval
- **Updates**: O(n) for content replacement
- **Memory**: Minimal overhead, direct string storage
- **Suitable for**: Files under 100KB

### 3. TextFile Documents - Collaborative Architecture

**Purpose**: Sophisticated collaborative text editing with minimal conflict resolution.

This is the most complex document type, designed to solve the fundamental collaborative editing challenge: when lines are deleted, all subsequent line indices change, requiring cascade updates that are expensive and conflict-prone.

#### The Two-Layer Architecture

**Problem**: Traditional line-based editing uses sequential indices (0, 1, 2, 3...). When line 1 is deleted, lines 2, 3, 4... must all be renumbered, causing cascading updates and edit conflicts.

**Solution**: Separate stable line identifiers from display positions using a two-layer system:

1. **Line Content Layer**: Maps stable line IDs to content
2. **Line Index Layer**: Maps stable line IDs to display positions

**Data Structure**:
```rust
TextFile Document {
    doc_type: DocumentType::TextFile,
    properties: {
        "filename" -> "main.rs",
        "language" -> "rust",
        "encoding" -> "utf-8",
        
        // Layer 1: Stable line content
        "line_content" -> Object({
            1u16 -> "use std::collections::HashMap;",  // Line ID -> Content
            2u16 -> "",
            3u16 -> "fn main() {",
            42u16 -> "    let x = 42;",
        }),
        
        // Layer 2: Display position mapping  
        "line_index" -> Object({
            1u16 -> 0u16,      // Line ID -> Display Position
            2u16 -> 1000u16,   // Sparse positioning with gaps
            3u16 -> 2000u16,
            42u16 -> 3000u16,
        }),
        
        "next_line_id" -> 100u16,
        "line_count" -> 4,
        "active_cursors" -> Object({ user_id -> "5:10" }),
    },
    children: [] // No children for text files
}
```

#### Sparse Positioning System

**Core Innovation**: Instead of sequential positioning (0, 1, 2, 3...), use sparse positioning with intentional gaps (0, 1000, 2000, 3000...).

**Benefits**:
- **999 insertions possible** between any two lines before cascade needed
- **Most insertions are O(1)** - just find the midpoint gap
- **Deletions create reusable gaps** for future insertions
- **Natural accommodation** for collaborative editing patterns

**Gap Management**:
```rust
// Inserting between positions 1000 and 2000
new_position = 1000 + (2000 - 1000) / 2 = 1500

// Inserting between 1000 and 1001 (no gap)
// Triggers cascade update: shift positions >= 1001 by some amount
```

#### Collaborative Benefits

**No Content Cascades**: When lines are inserted/deleted, line content never moves. Only the index mapping changes.

**Minimal Index Updates**: 
- **Insert with gap**: O(1) - just add new mapping
- **Insert without gap**: O(k) where k = lines needing position updates
- **Delete**: O(1) - just remove mapping, creates gap
- **Edit content**: O(1) - direct update by stable line ID

**Stable References**: Line ID 42 always refers to the same content, regardless of display position changes.

**Client-Side Conflict Prevention**: Cursor tracking shows where users are editing to prevent conflicts before they occur.

#### Performance Analysis

**Lookup Patterns**:
- **"What's at line 5?"**: O(n) scan - mainly needed for initial file loading
- **"Edit line ID 42"**: O(1) hash lookup - used for ongoing operations
- **"Insert at position 3"**: O(1) or O(k) depending on gap availability

**Memory Overhead**:
- **Index overhead**: ~4 bytes per line (2 bytes line ID + 2 bytes position)  
- **Maximum file**: 65,535 lines (u16 line IDs)
- **Typical overhead**: ~260KB for maximum file size

**Optimized for Operation Frequency**:
- **Most common**: Edit existing line content (O(1))
- **Common**: Insert line with gap available (O(1))  
- **Uncommon**: Insert line requiring cascade (O(k))
- **Rare**: Random access by display position (O(n))

#### Use Cases

**Ideal for**:
- Collaborative code editing
- Shared documentation
- Real-time note-taking
- Wiki-style content editing

**Not suitable for**:
- Very large files (>65K lines)
- Non-collaborative editing (use Text documents instead)
- Binary content
- Files requiring frequent random access by line number

### 4. Graph Documents

**Purpose**: Container documents for graph structures, holding collections of nodes and edges as child documents.

**Architecture**:
```rust
Graph Document {
    doc_type: DocumentType::Graph,
    properties: {
        "name" -> "Knowledge Graph",
        "graph_type" -> "directed",
        "layout_algorithm" -> "force_directed",
        "node_count" -> 150,
        "edge_count" -> 300,
        "auto_layout" -> true,
        "created_at" -> timestamp,
        "modified_at" -> timestamp,
    },
    children: [node1_id, node2_id, edge1_id, edge2_id, ...]
}

Node Document {
    doc_type: DocumentType::Node,
    properties: {
        "label" -> "Person",
        "name" -> "Alice",
        "x" -> 100.0,
        "y" -> 200.0,
        "z" -> 0.0,
        "weight" -> 0.8,
    },
    children: [] // Nodes typically have no children
}

Edge Document {
    doc_type: DocumentType::Edge,
    properties: {
        "label" -> "knows",
        "weight" -> 0.7,
        "directed" -> true,
    },
    children: [source_node_id, target_node_id] // Edge connects these nodes
}
```

**Key Features**:
- **Graph containers** that organize nodes and edges
- **Type-based filtering** for efficient traversal
- **Layout algorithm specification** for visualization
- **Automatic count tracking** for nodes and edges
- **Metadata management** for graph properties

**Graph Operations**:
- **Add node**: Create node document, add to graph children, increment count
- **Add edge**: Create edge document with node children, add to graph children
- **Find neighbors**: Query edges with specific node children
- **Subgraph extraction**: Filter children by criteria
- **Layout calculation**: Use position properties for visualization

**Use Cases**:
- Social networks and relationship graphs
- Knowledge graphs and ontologies
- Workflow and process modeling
- Network topology representation
- Recommendation system graphs

## Architectural Benefits

### Unified Foundation

All document types share the same underlying architecture:

```rust
Document {
    header: DocumentHeader,     // ID, type, version, timestamps
    properties: AdaptiveMap,    // Key-value data storage
    children: Vec<ID16>,        // References to child documents
}
```

This enables:
- **Consistent performance optimizations** across all types
- **Uniform security and permissions** models
- **Shared infrastructure** for networking, storage, and synchronization
- **Simplified codebase** with predictable patterns

### Delta Propagation

All document types benefit from the same delta propagation system:
- **Real-time synchronization** between collaborators
- **Zero-copy networking** for efficient distribution
- **Conflict resolution** through property-level atomicity
- **Audit trails** with complete operation history

### Type-Specific Optimizations

While sharing the foundation, each type optimizes for its use case:
- **Binary**: Streaming and chunk-based access
- **Text**: Simple content operations
- **TextFile**: Collaborative editing with minimal conflicts
- **Graph**: Type-based traversal and layout algorithms

## Implementation Status

The core type implementations provide:

1. **Factory methods** for creating properly structured documents
2. **Validation functions** to ensure document integrity
3. **Helper methods** for common operations
4. **Metadata extraction** for convenient property access
5. **Comprehensive documentation** with examples and use cases

## Future Extensions

The unified architecture enables easy addition of new document types:

- **TableFile**: Collaborative spreadsheet editing
- **DiagramFile**: Real-time diagram collaboration  
- **DatabaseSchema**: Schema definitions with versioning
- **MLModel**: Machine learning model storage with parameters
- **Workflow**: Process definitions with state tracking

Each new type follows the same patterns while optimizing for specific requirements.

## Performance Characteristics Summary

| Document Type | Create | Read | Update | Delete | Collaboration |
|---------------|--------|------|--------|--------|---------------|
| Binary | O(1) | O(1) | O(1) append | O(1) | Streaming |
| Text | O(1) | O(1) | O(n) replace | O(1) | Basic |
| TextFile | O(n) | O(n) scan | O(1) line edit | O(1) | Advanced |
| Graph | O(1) | O(log n) | O(1) | O(1) | Type-filtered |

This architecture provides the foundation for a truly collaborative, real-time database where different data types can coexist and interact while maintaining optimal performance for their specific use cases. 