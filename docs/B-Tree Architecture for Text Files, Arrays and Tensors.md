# B-Tree Architecture for Collaborative Text Files, Arrays and Tensors

## Design Philosophy

Mutable ordered sequences present a fundamental challenge in concurrent systems: how to enable efficient insertions, deletions, and modifications whilst maintaining zero-copy reads and avoiding costly reindexing operations. Traditional approaches either require global locks, expensive memory copies, or complex conflict resolution algorithms. This architecture solves these challenges through a piece table built from immutable deltas, organised in a B-tree structure with copy-on-write semantics.

The key insight is that deltas, already stored immutably for network synchronisation, can serve double duty as the backing storage for piece tables. This eliminates an entire storage layer whilst providing natural audit trails, zero-copy operations, and lock-free concurrent access. The B-tree organisation ensures O(log n) access times even for very large sequences, whilst copy-on-write node updates enable multiple readers to work with consistent snapshots without any synchronisation.

## Core Architecture

### Piece Table of Deltas

The piece table architecture represents mutable sequences as ordered collections of immutable pieces. Rather than maintaining separate buffers for original and added content, pieces directly reference deltas stored in the chunk storage system. This unification means that every edit operation already has its data in the perfect format for both storage and network transmission.

```rust
struct PieceTable<T> {
    tree: Arc<BTree<Piece>>,
    total_length: usize,
    piece_count: usize,
}

struct Piece {
    source: PieceSource,
    start: usize,        // Starting position within source
    length: usize,       // Length of this piece
}

enum PieceSource {
    Original(Arc<Vec<u8>>),           // Original content if any
    Delta(ChunkId, SlotId),           // Reference to delta in storage
    Snapshot(ChunkId, SlotId),        // Flattened snapshot
}
```

**Benefits of Delta-Based Pieces**:
- Zero additional storage overhead - deltas serve both purposes
- Natural audit trail - piece table shows exact edit history
- Network optimised - pieces already in wire format
- Immutable data - enables zero-copy operations
- Replay capability - can reconstruct any historical version

### B-Tree Organisation

Linear piece tables require O(n) traversal to find a specific position, making them impractical for large documents or tensors. The B-tree structure reduces this to O(log n) by maintaining a balanced tree of pieces with cumulative position information.

```rust
struct BTreeNode {
    // Store lengths, not absolute positions
    pieces: Vec<Piece>,
    children: Vec<Arc<BTreeNode>>,
    
    // Cached metadata for fast traversal
    total_length: usize,
    piece_count: usize,
}
```

The B-tree maintains cumulative positions implicitly through its structure. Each node knows the total length of its subtree, enabling binary search to locate any position in O(log n) time. This approach avoids the problem of updating absolute positions after every insert or delete operation.

**Benefits of B-Tree Structure**:
- O(log n) position lookup vs O(n) for lists
- Efficient range queries for partial reads
- Natural batching of pieces in nodes
- Cache-friendly node size tuning
- Balanced tree maintains performance guarantees

### Immutable Nodes with Copy-on-Write

The most critical innovation for concurrent access is making B-tree nodes immutable. When an update occurs, rather than modifying nodes in place, the system creates new nodes along the path from the modification point to the root. Unchanged subtrees remain shared through Arc references.

```rust
struct ImmutableBTree {
    root: Arc<BTreeNode>,
}

impl ImmutableBTree {
    fn insert(&self, position: usize, piece: Piece) -> ImmutableBTree {
        // Creates new nodes only along the path to insertion point
        let new_root = self.copy_path_and_insert(self.root.clone(), position, piece);
        ImmutableBTree { root: Arc::new(new_root) }
    }
}
```

This path-copying approach means that readers working with a previous version of the tree continue to see a consistent, immutable structure even while writers create new versions. The ArcSwap at the document level enables atomic transitions between versions.

**Benefits of Immutable Nodes**:
- Lock-free concurrent reads - no synchronisation needed
- Consistent snapshots - readers never see partial updates
- Automatic garbage collection via Arc reference counting
- Natural versioning - old versions persist until unreferenced
- Memory efficient - only O(log n) nodes copied per update

## Text Operations

### Fundamental Operations

Text editing operations map elegantly to piece table modifications. The immutability of underlying data means that all operations create new piece configurations rather than modifying existing content.

**Insert Operation**: Splits an existing piece at the insertion point and adds a new piece referencing the inserted text's delta.

**Delete Operation**: Adjusts piece boundaries or removes pieces entirely, but never modifies the underlying delta data.

**Append Operation**: Simply adds a new piece to the end of the table, achieving O(1) complexity for this common operation.

**Replace Operation**: Combines delete and insert in a single atomic piece table update.

The beauty of this approach is that no operation requires reindexing subsequent content. A single character insertion at the beginning of a gigabyte file only creates a few new B-tree nodes and one new piece entry.

### Collaborative Editing

The piece table architecture naturally supports collaborative editing without complex conflict resolution algorithms. Since all data is immutable and operations create new piece tables, the system can use a simple "first writer wins" approach whilst maintaining consistency.

The zero-copy read guarantee is crucial here: readers accessing the document during an edit operation continue working with their snapshot of the piece table, completely unaware of concurrent modifications. When they need the latest version, they simply load the new piece table root—an atomic pointer swap.

### Concurrent Position Resolution

A fundamental challenge in collaborative editing is that insertions and deletions shift all subsequent positions. When User A inserts text at the beginning of a document, User B's cursor position becomes invalid. The piece table architecture addresses this through two complementary strategies:

**Dual Operation Types**: The system supports both index-based operations (for single-user scenarios) and anchor-based operations (for concurrent collaboration). Index-based operations use simple numeric positions, whilst anchor-based operations reference stable points that survive document mutations.

```rust
enum TextOperation {
    // Index-based - simple but affected by concurrent edits
    InsertAt { index: usize, text: String },
    
    // Anchor-based - stable across concurrent edits
    InsertAfter { anchor: AnchorId, offset: usize, text: String },
}
```

**Context Index**: A parallel B-tree structure maintains stable anchor points throughout the document. These anchors can represent lines, paragraphs, functions, or any semantic structure relevant to the document type. The Context Index provides O(log n) lookup of stable positions, enabling efficient resolution of anchor-based operations.

```rust
struct TextFile {
    pieces: Arc<BTree<Piece>>,           // The piece table
    context_index: Arc<BTree<Anchor>>,   // Stable reference points
}
```

This approach is particularly powerful for AI-assisted editing, where agents need stable references to semantic structures like functions, classes, or sections. An AI agent can target "the authentication function" via its anchor, regardless of how other parts of the document have changed. This enables multi-pass editing, semantic-aware transformations, and conflict-free collaboration between multiple AI agents and human users.

**Benefits for Collaboration**:
- No reindexing cascade on insertions/deletions
- Stable anchors survive concurrent edits
- Natural operation ordering through delta sequence
- AI agents can reliably target semantic structures
- Simple conflict resolution model
- Audit trail of all modifications

## Tensor and Array Applications

### Tensor Piece Structure

Large tensors and multi-dimensional arrays benefit enormously from the piece table approach. Rather than copying gigabytes of data for each operation, tensor modifications create new piece configurations that reference the same underlying immutable chunks.

```rust
struct TensorPieceTable {
    shape: Vec<usize>,
    tree: Arc<BTree<TensorPiece>>,
}

struct TensorPiece {
    source: PieceSource,
    start_coords: Vec<usize>,    // Starting position in each dimension
    shape: Vec<usize>,            // Shape of this piece
    stride: Vec<usize>,           // Memory layout information
}
```

### Zero-Copy Tensor Operations

The piece table architecture enables sophisticated tensor operations without data movement:

**Slicing**: Creates new pieces that reference subsets of existing data. A slice operation on a terabyte tensor completes in microseconds by creating a new piece table with adjusted boundaries.

**Transposition**: Rearranges dimensions in the piece metadata without touching data. The stride information handles the logical reordering.

**Reshaping**: Updates shape metadata whilst maintaining the same underlying data references.

**Incremental Updates**: Modifies specific regions by adding new pieces for changed areas whilst sharing unchanged regions.

### Machine Learning Optimisations

The architecture provides unique benefits for machine learning workloads:

**Gradient Storage**: Backward pass gradients become new pieces, enabling gradient checkpointing without duplicating forward pass data.

**Model Versioning**: Each training step creates a new piece table, providing free model checkpointing and rollback capabilities.

**Lazy Evaluation**: Operations build piece descriptions without immediate computation, enabling operation fusion and optimisation.

**Distributed Training**: Pieces can reference data on different nodes, enabling zero-copy distributed tensor operations.

**Benefits for ML Workloads**:
- Memory efficient model checkpointing
- Zero-copy data augmentation
- Natural gradient accumulation
- Efficient batch processing
- Version control for model weights

## Optimisation Strategies

### Periodic Flattening

Whilst the piece table structure is efficient, extreme fragmentation can degrade performance. After many edit operations, a document might consist of thousands of tiny pieces. Periodic flattening consolidates these pieces into a single contiguous delta, resetting the piece table to a single entry.

```rust
impl PieceTable {
    fn should_flatten(&self) -> bool {
        self.piece_count > 1000 ||
        self.total_length / self.piece_count < 10 ||
        self.edits_since_flatten > 1000
    }
    
    fn flatten(&self) -> PieceTable {
        // Materialise all pieces into single buffer
        // Store as new snapshot delta
        // Return new piece table with single piece
    }
}
```

Flattening occurs off the critical path—a background thread monitors fragmentation metrics and triggers flattening when thresholds are exceeded. The immutable architecture means that readers continue working with the fragmented version whilst flattening occurs.

**Benefits of Periodic Flattening**:
- Restores optimal cache locality
- Reduces tree depth and traversal time
- Creates natural checkpoint for recovery
- Improves network transmission efficiency
- Provides clean starting point for new edits

### Memory Management

The piece table architecture requires careful memory management to balance performance with resource usage:

**Piece Coalescing**: Adjacent pieces from the same source can be merged into a single piece, reducing overhead without flattening.

**Delta Retention**: Old deltas referenced only by historical piece tables can be archived or compressed based on retention policies.

**Reference Counting**: Arc automatically frees unreferenced pieces and nodes, but the system monitors reference counts to identify potential leaks.

**Memory Pressure Response**: Under memory pressure, the system can trigger aggressive flattening or evict cold piece tables to disk.

## Performance Characteristics

### Time Complexity

- **Random Access**: O(log n) where n is the number of pieces
- **Sequential Access**: O(1) amortised through iterator state
- **Insert/Delete**: O(log n) for tree traversal plus O(log n) node copies
- **Append**: O(1) when appending to last piece
- **Flatten**: O(n) but performed off critical path

### Space Complexity

- **Piece Overhead**: ~40 bytes per piece (source, start, length, metadata)
- **Node Overhead**: ~100 bytes per B-tree node
- **Path Copying**: O(log n) nodes copied per update
- **Version Storage**: Old versions consume only copied node space

### Concurrency Characteristics

- **Reader Concurrency**: Unlimited concurrent readers
- **Writer Throughput**: Single writer achieves full throughput
- **Lock-Free Reads**: Zero synchronisation overhead
- **Atomic Updates**: Version transitions via single pointer swap
- **Memory Ordering**: Relaxed ordering sufficient for most operations

## Design Trade-offs

### Accepted Overheads

1. **Path Copying Cost**: Each update copies O(log n) nodes
2. **Fragmentation Potential**: Many edits create many pieces
3. **Indirection Overhead**: Extra pointer chase to access data
4. **Memory for Old Versions**: Historical versions consume space

### Benefits Gained

1. **Zero-Copy Reads**: Readers never blocked or copying data
2. **No Reindexing**: Insertions don't cascade position updates
3. **Natural Versioning**: Every edit creates recoverable version
4. **Audit Trail**: Complete history in piece table structure
5. **Unified Storage**: Deltas serve both network and storage purposes
6. **Lock-Free Concurrency**: No synchronisation primitives needed
7. **Crash Recovery**: Immutable pieces enable simple recovery

This architecture represents a fundamental shift in how mutable ordered sequences are managed, trading small amounts of metadata overhead for massive gains in concurrency, reliability, and operational simplicity.