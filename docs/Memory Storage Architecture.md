# Memory Storage Architecture

## Design Philosophy

The memory storage architecture prioritises zero-blocking allocation and memory stability to support millions of operations per second. Network threads must never wait when storing data, and once stored, data never moves in memory, enabling true zero-copy network transmission. The system achieves this through wait-free atomic operations and chunked pre-allocation, accepting some memory overhead for predictable performance.

This chunk-based architecture is designed for immutable data types - primarily deltas and large immutable values like tensors. Streams, being ordered lists of deltas, use a linked structure referencing these immutable chunks.

## Core Architecture

### Storage Structure

```rust
struct ChunkStorage {
    chunks: RwLock<Vec<Arc<Chunk>>>,      // All chunks (rarely locked)
    active_chunk: ArcSwap<Chunk>,         // Current chunk for allocation
    next_chunk: ArcSwap<Option<Chunk>>,   // Pre-created by maintenance
    threshold_triggered: AtomicBool,      // Signal to maintenance thread
}

struct Chunk {
    // Contiguous storage
    data: Box<[u8]>,  // 16MB pre-allocated buffer
    data_used: AtomicUsize,
    
    // Parallel array of slot information
    slots: Box<[SlotInfo; CHUNK_SIZE]>,  // 64K slots
    slots_claimed: AtomicU32,
    
    // Parallel array of mutable state
    states: Box<[SlotState; CHUNK_SIZE]>,
    
    // Chunk metadata
    chunk_id: u64,
    created_at: u64,
}

const CHUNK_SIZE: usize = 65536;  // 64K slots per chunk
const CHUNK_DATA_SIZE: usize = 16 * 1024 * 1024;  // 16MB data per chunk
```

### Slot Information and State

```rust
struct SlotInfo {
    offset: u32,      // Where in 'data' this item starts
    length: u32,      // Total length
    slot_type: u8,    // Delta, StreamChunk, Snapshot, etc.
    flags: u8,        // Tombstone, compressed, etc.
    reserved: u16,    // Padding/future use
}

struct SlotState {
    status: AtomicU8,       // NOT_PROCESSED, PROCESSING, PROCESSED, INVALID
    owner_id: AtomicU8,     // Which worker/stream owns this
    ref_count: AtomicU16,   // Reference counting for cleanup
    sequence: AtomicU64,    // Global sequence number
}

enum SlotType {
    Delta = 0,
    StreamChunk = 1,
    DocumentSnapshot = 2,
    LargeValue = 3,
    TempValue = 4,
}
```

## Memory Layout Diagram

```
Chunk Memory Layout:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Data Array (Contiguous Storage):
┌────────────────────────────────────────────────────────────┐
│[Delta₁][StreamChunk₁][Snapshot₁][LargeValue₁][Free...]    │ 16MB
└────────────────────────────────────────────────────────────┘
     ↑         ↑            ↑           ↑
     │         │            │           │
  offset:0  offset:352  offset:4896  offset:12053
     
Slots Array (Fixed-Size Metadata):
┌────────────────────────────────────────────────────────────┐
│ [DELTA,352] [STREAM,4544] [SNAP,7157] [LARGE,8192] [...]   │ 64K slots
└────────────────────────────────────────────────────────────┘
     Slot 0      Slot 1        Slot 2      Slot 3

States Array (Mutable State Tracking):
┌────────────────────────────────────────────────────────────┐
│ [PROCESSED] [ACTIVE] [PROCESSED] [TEMP] [...]              │ 64K states
└────────────────────────────────────────────────────────────┘

Wire Transmission (Zero-Copy):
                 Request for Slot 1
                      ↓
    ┌─────────────────────────────────┐
    │ &data[352..4896] → Direct Send   │ Zero-copy to network!
    └─────────────────────────────────┘
```

## Data Type Storage Strategies

### Delta Storage

Deltas are stored with their headers contiguously for atomic wire transmission:

```rust
struct DeltaStorage {
    // Delta = Header (32 or 64 bytes) + Payload
    // Stored contiguously in chunk for zero-copy send
}
```

**Allocation Pattern**:
```rust
fn allocate_delta(&self, header: &[u8], payload: &[u8]) -> SlotHandle {
    let total_size = header.len() + payload.len();
    let offset = chunk.data_used.fetch_add(total_size, Ordering::Relaxed);
    
    // Write header and payload contiguously
    unsafe {
        ptr::copy_nonoverlapping(header.as_ptr(), data_ptr, header.len());
        ptr::copy_nonoverlapping(payload.as_ptr(), data_ptr + header.len(), payload.len());
    }
}
```

**Key Properties**:
- Headers and payloads stored together for atomic transmission
- Never modified after writing (immutable)
- Direct network send from chunk memory

### Stream Storage - Ordered Delta Lists

Streams (TextStream, BinaryStream, DocumentStream) are fundamentally ordered lists of deltas. Rather than storing data separately, streams maintain doubly-linked lists of delta references:

```rust
struct Stream {
    stream_id: StreamId,
    stream_type: StreamType,
    head: Option<Arc<DeltaRef>>,  // First delta in stream
    tail: Option<Arc<DeltaRef>>,  // Last delta for O(1) append
    count: AtomicU64,
}

struct DeltaRef {
    chunk_id: u64,
    slot_id: usize,
    next: Option<Arc<DeltaRef>>,     // Next delta in stream
    prev: Option<Weak<DeltaRef>>,    // Previous delta (weak to prevent cycles)
    sequence: u64,                   // Position in stream
}

enum StreamType {
    TextStream,      // Text deltas (append text chunks)
    BinaryStream,    // Binary deltas (append binary chunks)  
    DocumentStream,  // Document reference deltas
    AuditStream,     // Audit log of all operations
}
```

**Key Properties**:
- Streams ARE sequences of deltas, not separate data
- Doubly-linked for bidirectional traversal
- Each delta is immutable once written
- Append is O(1) via tail pointer
- Backward traversal for seeking/replay
- Perfect for audit logs - natural order preservation
- Zero-copy iteration through delta chain
- Old deltas retained until stream truncated

**Stream Operations**:
```rust
// Appending to a stream creates a delta and links it
fn append_to_stream(&mut self, content: &[u8]) {
    let delta = create_append_delta(content);
    let delta_ref = Arc::new(store_delta(delta));  // Stores in chunk
    
    // Link bidirectionally
    if let Some(tail) = &mut self.tail {
        delta_ref.prev = Some(Arc::downgrade(tail));
        tail.next = Some(delta_ref.clone());
    }
    self.tail = Some(delta_ref);
}

// Traverse backward from any point
fn traverse_backward(delta_ref: &DeltaRef) {
    if let Some(prev_weak) = &delta_ref.prev {
        if let Some(prev) = prev_weak.upgrade() {
            // Process previous delta
        }
    }
}
```

### Mutable Ordered Sequences (TextFiles, Strings, Tensors)

These types require special handling for concurrent read/write access with zero-copy guarantees. They use a piece table architecture built from immutable deltas:

**Core Strategy**:
- Piece tables reference immutable deltas stored in chunks
- B-tree structure for O(log n) position lookups
- Copy-on-write tree nodes for lock-free concurrent access
- Periodic flattening into snapshot deltas for performance

**Key Benefits**:
- Zero-copy reads during concurrent writes
- No reindexing on insert/delete operations
- Natural audit trail through delta references
- Memory efficiency through shared immutable chunks

See "Piece Table Architecture" document for detailed implementation.

## Zero-Copy Wire Transfer

All stored types support zero-copy transmission:

```rust
struct WireRef<'a> {
    wire_bytes: &'a [u8],  // Direct reference into chunk
    slot_type: SlotType,
    chunk_ref: &'a Chunk,
    slot_id: usize,
}
```

## Allocation Flow

Wait-free allocation works identically for all types:

```rust
fn allocate(&self, data: &[u8], slot_type: SlotType) -> SlotHandle {
    loop {
        let chunk = self.active_chunk.load();
        let offset = chunk.data_used.fetch_add(data.len(), Ordering::Relaxed);
        
        if offset + data.len() > CHUNK_DATA_SIZE {
            self.transition_to_next_chunk();
            continue;
        }
        
        let slot_id = chunk.slots_claimed.fetch_add(1, Ordering::Relaxed);
        // Store data and return handle
    }
}
```

## Chunk Management

### Proactive Chunk Creation

Maintenance thread pre-creates chunks at 70% capacity to prevent allocation stalls.

### Chunk Lifecycle

- **Active**: Currently receiving allocations
- **Sealed**: No new allocations, but data still referenced
- **Archived**: Can be persisted to disk
- **Reclaimable**: All references dropped, can be freed

## Performance Characteristics

### Allocation Performance
- **Allocation**: ~100M ops/sec via wait-free fetch-and-add
- **No blocking**: Writers never wait on locks
- **Predictable latency**: Single atomic operation
- **Natural backpressure**: Chunk exhaustion signals overload

### Memory Efficiency
- **16MB chunks**: Fits in L3 cache
- **64K slots**: Balances overhead with allocation frequency
- **10-20% tombstones acceptable**: Worth it for zero contention
- **Unified storage**: All types use same infrastructure

### Wire Transmission
- **True zero-copy**: Direct memory references
- **Vectored I/O support**: Batch transmission
- **Type-agnostic**: Same mechanism for all data types
- **Cache-friendly**: Sequential memory access

## Design Trade-offs

### Accepted Overheads
1. **Memory pre-allocation**: 16MB per chunk upfront
2. **Tombstone waste**: Failed allocations leave gaps
3. **Slot metadata overhead**: 16 bytes per slot
4. **Type discrimination**: 1 byte per slot for type

### Benefits Gained
1. **Never blocks**: Wait-free guarantees
2. **Memory stability**: Data never moves
3. **Zero-copy networking**: Direct transmission
4. **Unified architecture**: One system for all types
5. **Natural flow control**: Backpressure mechanism