# Memory Storage Architecture v2

## Design Philosophy

The memory storage architecture implements a unified chunk-based system for all immutable data types, with specialized handling for different access patterns. The core principle is **write-once, read-many** - data is allocated once in chunk memory and never moves, enabling true zero-copy network transmission and eliminating synchronization overhead.

The architecture distinguishes between:
- **Persistent data** (Document Headers, Wire Snapshots, Deltas) - must survive restarts
- **Ephemeral structures** (Linked lists, Stream references) - rebuilt on startup
- **Append-only streams** - optimized for sequential access and propagation
- **User isolation** - Separate storage spaces per user for security and performance

## Core Chunk Architecture

The chunk system uses document-specific allocation to ensure sequential storage of deltas, enabling single-read zero-copy propagation for optimal performance.

```rust
pub struct ChunkStorage {
    // ALL chunks for each document (active + sealed)
    document_chunks: DashMap<DocId, ArcSwap<Vec<Arc<Chunk>>>>,
    
    // Current active chunk per document for fast allocation
    active_chunks: DashMap<DocId, Arc<Chunk>>,
    
    // Pre-allocated chunk pools by size for atomic swap
    chunk_pools: [SegQueue<Arc<Chunk>>; 4],  // One pool per ChunkSize variant
    
    // Global sequence counter
    sequence: AtomicU64,
}

pub struct Chunk {
    document_id: Option<DocId>,     // Which document (None for pool chunks)
    data: Vec<u8>,                  // Variable size, set at creation
    used: AtomicUsize,              // Bytes allocated
    capacity: usize,                // Initial capacity
    state: AtomicU8,                // ChunkState as atomic
    created_at: u64,
    next: Option<Arc<Chunk>>,       // Chain for large documents
}

pub struct ChunkRef {
    document_id: DocId,             // Which document
    chunk_id: u64,                  // Unique chunk identifier  
    offset: u32,                    // Offset within chunk
    length: u32,                    // Length of data
}

#[repr(u8)]
pub enum ChunkState {
    Active = 0,      // Currently receiving writes
    Sealed = 1,      // No more writes, can persist
    Persisted = 2,   // Written to disk
    Archived = 3,    // Moved to cold storage
}

#[repr(usize)]
pub enum ChunkSize {
    Tiny = 65_536,        // 64KB - Very slow documents
    Small = 1_048_576,    // 1MB - Normal documents  
    Medium = 4_194_304,   // 4MB - Active documents
    Large = 16_777_216,   // 16MB - Streams/hot documents
}
```

### Document-Specific Chunking Benefits

By allocating chunks per document rather than mixing documents:

1. **Sequential reads** - All deltas for a document are contiguous
2. **Zero-copy propagation** - Send entire delta sequence in one syscall
3. **Cache efficiency** - CPU prefetcher works optimally
4. **Adaptive sizing** - Right-sized chunks based on document activity

### Length-Prefixed Encoding

Data within chunks uses length-prefix encoding for optimal memory utilization:

```
Chunk Memory Layout (Single Document):
[Length:4][Header][Delta][Length:4][Header][Delta][Length:4][Header][Delta]...
     ↑_____________________________________________________________↑
                    All deltas for one document - sequential!

Benefits:
- Single memory region for zero-copy send
- Perfect cache locality for document operations  
- No fragmentation from mixed documents
- 100% memory utilization within chunk
```

## User Storage Space

Multi-user environments benefit from isolated storage spaces per user, providing the highest level of data organization:

```rust
pub struct UserStorageSpace {
    user_id: ID16,
    
    // Separate storage for different data types
    documents: ChunkStorage,    // Document headers + snapshots
    deltas: ChunkStorage,        // All deltas (document-specific chunks)
    
    // Lock-free indexes - one DashMap per document type
    indexes: [DashMap<DocId, ChunkRef>; NUM_DOC_TYPES],
    
    // Version snapshots index
    version_index: DashMap<(DocId, VersionId), ChunkRef>,
    
    // Metadata
    created_at: u64,
    total_bytes_used: AtomicU64,
    quota_bytes: u64,
}

impl UserStorageSpace {
    /// Get the index for a specific document type
    fn get_index(&self, doc_type: DocumentType) -> &DashMap<DocId, ChunkRef> {
        &self.indexes[doc_type as usize]
    }
    
    /// Allocate space for a delta
    fn allocate_delta(&self, doc_id: DocId, header: &[u8], delta: &[u8]) -> ChunkRef {
        self.deltas.allocate(doc_id, header, delta)
    }
    
    /// Look up a document header
    fn get_document(&self, doc_type: DocumentType, doc_id: DocId) -> Option<ChunkRef> {
        self.get_index(doc_type)
            .get(&doc_id)
            .map(|entry| *entry)
    }
}
```

### Benefits of User Isolation

```rust
struct UserStorageSpace {
    user_id: UserId,
    documents: ChunkStorage,  // User's document headers
    deltas: ChunkStorage,     // User's deltas
    streams: ChunkStorage,    // User's streams
}
```

### Benefits of User Isolation

1. **Security**: User data physically separated in memory
2. **Performance**: No cross-user contention on chunks
3. **Cache Locality**: User operations stay in same memory regions
4. **Resource Limits**: Per-user quotas easily enforced
5. **Cleanup**: User deletion frees entire storage space
6. **Debugging**: Issues isolated to specific user's chunks

## Document Headers

Document headers are the foundation for system recovery. They contain all information needed to reconstruct a document after restart. Headers are approximately 53 bytes each, stored once and never modified.

**Recovery Strategy**: 
- On restart, scan chunks for document headers
- Each header provides stream IDs to reconstruct document history
- Headers use the same chunk storage system with length-prefix encoding

## Delta Storage

Deltas are stored with server-generated headers contiguously for zero-copy propagation. Three header types exist based on requirements:

- **LightDeltaHeader** (16 bytes) - For streams where ordering can be estimated by timestamp
- **OrderedDeltaHeader** (32 bytes) - Standard header with ID-based chaining for order guarantees  
- **SecureOrderedDeltaHeader** (64 bytes) - Includes cryptographic proof for lineage verification

### Storage Layout

Headers and deltas are stored contiguously for zero-copy network transmission:

```
Memory Layout:
[Length:4][LightHeader:16][Delta:N]
[Length:4][OrderedHeader:32][Delta:N]
[Length:4][SecureHeader:64][Delta:N]

Zero-copy send: sendmsg(fd, chunk.data[offset..], MSG_ZEROCOPY)
```

The header type is encoded in the first byte, allowing receivers to determine header size. This design ensures single-syscall transmission of both header and delta.

## Wire Format Snapshots

Wire format snapshots are periodic materialized views of documents at specific versions. These snapshots can range from a few KB for simple JSON documents to several MB for complex tensors or graphs.

**Purpose**: 
- Avoid replaying entire delta history
- Load latest snapshot, then apply only subsequent deltas
- Created off critical path by maintenance workers
- Stored using the same chunk system with ChunkRef pointing to the wire bytes

## Streams (Runtime Linked Lists)

Streams are generic, type-agnostic linked lists used for sequential access and propagation. They are ephemeral structures rebuilt from persistent storage on restart.

```rust
/// Generic stream storage (type-agnostic)
pub struct Stream {
    stream_id: u64,
    head: *const Node,           // First node
    tail: AtomicPtr<Node>,       // Last node for O(1) append
    last_processed: AtomicPtr<Node>, // Processing cursor
}

/// Runtime node - not persisted
struct Node {
    data_ref: ChunkRef,         // Points to any data type in chunk
    next: AtomicPtr<Node>,      // Next in stream
}
```

**Key Properties**:
- Type-agnostic - can store deltas, versions, text, binary data
- Linked list rebuilt on startup from persistent chunks
- Used for propagation and sequential access
- Lock-free append via atomic tail pointer
- Direct memory pointers for O(1) traversal

**Use Cases**:
- Delta streams for document history
- Version streams for snapshot sequences
- Text/Binary streams for real-time data (video, chat)
- Event streams for audit logs

## Allocation Implementation

Fast-path allocation using relaxed memory ordering for maximum performance:

```rust
impl ChunkStorage {
    fn allocate(&self, header: &[u8], delta: &[u8]) -> ChunkRef {
        let total_size = 4 + header.len() + delta.len();  // 4 bytes for length prefix
        
        // Fast path - single atomic load
        let chunk = self.active_chunk.load();
        
        // Claim space with relaxed ordering (1ns)
        let offset = chunk.used.fetch_add(total_size, Ordering::Relaxed);
        
        if offset + total_size > CHUNK_DATA_SIZE {
            return self.allocate_slow_path(header, delta);  // Chunk full
        }
        
        // Write length prefix and data
        unsafe {
            let ptr = chunk.data.as_ptr().add(offset);
            
            // Write length prefix
            ptr::copy_nonoverlapping(
                &(header.len() + delta.len()) as *const u32 as *const u8,
                ptr,
                4
            );
            
            // Write header
            ptr::copy_nonoverlapping(header.as_ptr(), ptr.add(4), header.len());
            
            // Write delta
            ptr::copy_nonoverlapping(
                delta.as_ptr(),
                ptr.add(4 + header.len()),
                delta.len()
            );
        }
        
        ChunkRef {
            chunk_idx: chunk.chunk_idx,
            offset: offset as u32,
            length: total_size as u32,
        }
    }
}
```

### Memory Ordering

Using `Ordering::Relaxed` for allocation provides:
- **~1ns atomic operations** vs 3-5ns for stricter orderings
- **No cache synchronization** between cores
- **Safe for our use case** - we only need unique offsets, not ordering

## Chunk Lifecycle and Persistence

```rust
enum ChunkState {
    Active,      // Currently receiving writes
    Sealed,      // No more writes, can persist
    Persisted,   // Written to disk
    Archived,    // Moved to cold storage
}

impl ChunkStorage {
    /// Persist sealed chunks to disk
    async fn persist_chunk(&self, chunk: &Chunk) -> Result<()> {
        let path = format!("chunks/{}.dat", chunk.chunk_idx);
        let used = chunk.used.load(Ordering::Acquire);
        tokio::fs::write(&path, &chunk.data[..used]).await?;
        Ok(())
    }
    
    /// Load chunk from disk on demand
    async fn load_chunk(&self, chunk_idx: u32) -> Result<Chunk> {
        let path = format!("chunks/{}.dat", chunk_idx);
        let data = tokio::fs::read(&path).await?;
        // Reconstruct chunk...
    }
}
```

## Traversal Without Slots

Length-prefixed encoding enables sequential traversal without slot arrays:

```rust
impl Chunk {
    fn iterate(&self) -> impl Iterator<Item = &[u8]> {
        let mut offset = 0;
        let used = self.used.load(Ordering::Acquire);
        
        std::iter::from_fn(move || {
            if offset >= used {
                return None;
            }
            
            // Read length prefix
            let length = u32::from_le_bytes(
                self.data[offset..offset+4].try_into().ok()?
            ) as usize;
            
            offset += 4;
            let data = &self.data[offset..offset + length];
            offset += length;
            
            Some(data)
        })
    }
}
```

## Performance Characteristics

| Operation | Throughput | Latency | Notes |
|-----------|------------|---------|-------|
| Chunk Allocation | 100M ops/sec | ~60ns | With relaxed ordering |
| Delta Storage | 50M ops/sec | ~120ns | Header + delta allocation |
| Stream Append | 100M ops/sec | ~10ns | Atomic pointer swap |
| Zero-copy Send | Network limited | ~0ns | Direct from chunk |
| Chunk Persist | 1GB/sec | Async | Background operation |
| Sequential Scan | Memory bandwidth | ~2GB/sec | Length-prefix traversal |

### Allocation Cost Breakdown
```
Load active chunk:   5ns   // ArcSwap load
Claim space:        1ns   // fetch_add(Relaxed)
Write length:       5ns   // 4 bytes
Write header:      20ns   // ~32 bytes
Write delta:       30ns   // ~300 bytes
-----------------------
Total:             ~60ns
```

## Design Trade-offs

**Accepted Overhead**:
- Length prefixes (4 bytes per allocation)
- No O(1) random access by index
- Rebuild cost for linked structures on restart

**Benefits Gained**:
- 100% memory utilization potential
- True zero-copy from allocation to network
- No synchronization on read path
- Simple recovery model
- User isolation for multi-tenant safety

## Future Work

1. **Stream Storage** - Optimized linked lists per document
2. **Index Structures** - HashMap<DocId, ChunkRef> for O(1) lookup
3. **Large Value Handling** - Strategy for multi-MB documents
4. **Garbage Collection** - Reclaim space from old snapshots