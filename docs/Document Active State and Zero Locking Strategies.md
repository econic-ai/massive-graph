# Document Active State and Zero Locking Strategies

### Design Philosophy
The architecture achieves zero-locking for document data through exclusive document ownership. Each document belongs to exactly one worker thread, determined by consistent hashing of the document ID. This eliminates synchronisation overhead for data access - no mutexes or locks needed for document properties. State management uses atomic bitflags for efficient state transitions and monitoring, providing clean state machine semantics without the overhead of full locking.

### Hash-Based Document Ownership
```rust
fn get_document_owner(doc_id: DocumentId, num_workers: usize) -> usize {
    hash(doc_id) % num_workers  // Consistent assignment
}
```

This simple hash function ensures:
- **Deterministic routing**: Same document always goes to same worker
- **Even distribution**: Documents spread uniformly across workers
- **Zero coordination**: No runtime negotiation needed
- **Cache locality**: Document stays in one worker's CPU cache

### Document Structure with Atomic State
Since documents are exclusively owned by one worker, we use simple data structures for properties whilst maintaining atomic state flags:

```rust
struct Document {
    // No locks needed for data - single-threaded access only
    doc_type: DocumentType,
    props: HashMap<PropertyId, VersionedValue>,  // Simple HashMap, not DashMap
    meta: DocumentMeta,
    state: DocumentState,
    
    // Regular fields for metadata
    created_at: u64,
    last_modified: u64,
}

struct DocumentState {
    // State flags using bitflags in single atomic
    flags: AtomicU32,  // Efficient state checks and transitions
    
    // Activity tracking with atomics for clean updates
    delta_count: AtomicU32,
    last_access: AtomicU64,
    access_pattern: AtomicU8,
}

bitflags! {
    struct DocFlags: u32 {
        const IDLE              = 0b00000000;
        const PROCESSING        = 0b00000001;  // Currently applying delta
        const NEEDS_COMPACTION  = 0b00000010;  // Properties need cleanup
        const NEEDS_REINDEX     = 0b00000100;  // Indices need rebuild
        const HOT_DOCUMENT      = 0b00001000;  // High traffic document
        const EVICTING          = 0b00010000;  // Being removed from memory
        const DIRTY             = 0b00100000;  // Has unapplied changes
        const SNAPSHOT_READY    = 0b01000000;  // Read snapshot available
    }
}

struct VersionedValue {
    version: u64,
    data: Arc<Vec<u8>>,  // Arc for safe reader access
}
```

Using bitflags provides atomic multi-state operations. We can check multiple conditions or perform complex state transitions in a single atomic operation, eliminating race conditions. This uses only 4 bytes per document versus 32-64 bytes for separate atomic booleans with padding.

### State Management with Bitflags
Atomic bitflags provide efficient state management even with single-threaded ownership:

```rust
impl Worker {
    fn process_delta(&mut self, delta: Delta) {
        let doc = self.documents.entry(delta.doc_id)
            .or_insert_with(Document::new);
        
        // Atomic state transition - clean and efficient
        let prev_flags = doc.state.flags.fetch_or(DocFlags::PROCESSING.bits(), Ordering::Acquire);
        
        // Check if already processing (shouldn't happen with single owner, but safe)
        if prev_flags & DocFlags::PROCESSING.bits() != 0 {
            return; // Already processing
        }
        
        // Apply delta - no locks needed for data access
        match delta.operation {
            Op::Set(prop_id, value) => {
                doc.props.insert(prop_id, VersionedValue {
                    version: doc.next_version(),
                    data: Arc::new(value),
                });
            }
            Op::Delete(prop_id) => {
                doc.props.remove(&prop_id);
            }
        }
        
        // Update activity tracking
        doc.state.delta_count.fetch_add(1, Ordering::Relaxed);
        doc.state.last_access.store(current_timestamp(), Ordering::Relaxed);
        
        // Clear processing flag and check if maintenance needed
        let delta_count = doc.state.delta_count.load(Ordering::Relaxed);
        let mut new_flags = DocFlags::IDLE.bits();
        
        if delta_count % 1000 == 0 {
            new_flags |= DocFlags::NEEDS_COMPACTION.bits();
        }
        
        if delta_count > HOT_THRESHOLD {
            new_flags |= DocFlags::HOT_DOCUMENT.bits();
        }
        
        doc.state.flags.store(new_flags, Ordering::Release);
    }
}
```

The atomic operations ensure clean state transitions and enable lock-free monitoring from other threads (like metrics collectors) whilst maintaining single-threaded data access.

### Read Path Without Blocking
Readers access document properties through the Arc references in VersionedValue:

```rust
impl Worker {
    fn handle_read(&self, doc_id: DocumentId, prop_id: PropertyId) -> Option<Arc<Vec<u8>>> {
        let doc = self.documents.get(&doc_id)?;
        
        // Check if document is ready for reading
        let flags = doc.state.flags.load(Ordering::Acquire);
        if flags & DocFlags::EVICTING.bits() != 0 {
            return None; // Document being evicted
        }
        
        let versioned_value = doc.props.get(&prop_id)?;
        Some(versioned_value.data.clone())  // Clone Arc, not data
    }
}

// Reader thread receives Arc<Vec<u8>> and can process without blocking
fn process_read(data: Arc<Vec<u8>>) {
    // Safe to read even if writer updates document
    // Writer creates new Arc, doesn't modify this one
    let bytes = &*data;
    // Process bytes...
}
```

The Arc mechanism provides natural read-write isolation:
- **Writers create new Arcs**: Never modify existing data
- **Readers hold Arc references**: Data remains valid until dropped
- **No synchronisation needed**: Arc's reference counting is thread-safe
- **Eventual consistency**: Readers might see slightly stale data (microseconds old)

### Monitoring and Metrics
The atomic state flags enable efficient monitoring without interfering with workers:

```rust
impl Monitor {
    fn collect_metrics(&self, workers: &[Worker]) {
        for worker in workers {
            for doc in worker.documents.values() {
                let flags = doc.state.flags.load(Ordering::Relaxed);
                
                // Count documents in various states
                if flags & DocFlags::PROCESSING.bits() != 0 {
                    self.processing_count.fetch_add(1, Ordering::Relaxed);
                }
                if flags & DocFlags::HOT_DOCUMENT.bits() != 0 {
                    self.hot_count.fetch_add(1, Ordering::Relaxed);
                }
                if flags & DocFlags::NEEDS_COMPACTION.bits() != 0 {
                    self.needs_maintenance.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }
}
```

Monitoring threads can safely inspect document states without locks or coordination with workers.

### Memory Lifecycle Without Locks
The ownership model with atomic state tracking simplifies memory management:

```rust
impl Worker {
    fn compact_document(&mut self, doc_id: DocumentId) {
        let doc = &mut self.documents[&doc_id];
        
        let flags = doc.state.flags.load(Ordering::Acquire);
        if flags & DocFlags::NEEDS_COMPACTION.bits() == 0 {
            return; // No compaction needed
        }
        
        // Set compacting state
        doc.state.flags.fetch_or(DocFlags::PROCESSING.bits(), Ordering::Acquire);
        
        // Compact properties - single-threaded access to data
        for (prop_id, value) in &mut doc.props {
            // Arc automatically frees old data when no readers remain
            // No need for hazard pointers or RCU
        }
        
        // Clear compaction and processing flags
        doc.state.flags.fetch_and(
            !(DocFlags::NEEDS_COMPACTION | DocFlags::PROCESSING).bits(), 
            Ordering::Release
        );
    }
}
```

Arc reference counting provides automatic memory safety. Old versions are freed when all reader references drop, without requiring explicit synchronisation or hazard pointers.

### Performance Characteristics
The hybrid approach provides optimal performance:
- **No locks for data access**: Full single-threaded speed for properties
- **Atomic state management**: Clean state machine with 1-2ns overhead
- **No cache coherence for data**: Properties stay in worker's cache
- **Minimal coherence for state**: Only 4-byte atomic flags shared
- **Perfect scaling**: Workers never wait for each other on data access

This architecture achieves the best of both worlds: single-threaded simplicity for data access with clean atomic state management for coordination and monitoring.
