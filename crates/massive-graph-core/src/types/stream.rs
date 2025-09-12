// use std::sync::atomic::{AtomicBool, AtomicPtr};
// use super::StreamId;

// /// In-memory reference to immutable delta
// /// Used for building traversable chains
// pub struct DeltaRef {
//     pub ptr: *const WireDelta,  // Pointer to immutable delta
//     pub delta_id: ID8,           // Cached for fast ordering
//     pub timestamp: u64,          // Cached for time-based queries
//     pub next: AtomicPtr<DeltaRef>, // Next in chain (lock-free)
// }

// /// Lock-free append-only stream for deltas
// pub struct AppendOnlyDeltaStream {
//     pub head: *const DeltaRef,           // First delta (immutable after creation)
//     pub tail: AtomicPtr<DeltaRef>,      // Last delta for O(1) append
// }

// impl AppendOnlyDeltaStream {
//     /// Create new stream with initial delta
//     pub fn new(first: *const DeltaRef) -> Self {
//         Self {
//             head: first,
//             tail: AtomicPtr::new(first as *mut DeltaRef),
//         }
//     }
    
//     /// Append delta to stream atomically
//     pub fn append(&self, delta_ref: *mut DeltaRef) {
//         // Lock-free append via tail CAS
//     }
// }

use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr;
use crate::DocId;
use crate::types::delta::Delta;

/// Singly linked node for append-only streams.
pub struct Node<T> {
    /// Payload stored in the node (often a pointer into chunk memory for zero-copy).
    pub data: T,
    /// Next node pointer; null when this is the tail.
    pub next: AtomicPtr<Node<T>>,
}

impl<T> Node<T> {
    /// Create a node with `next = null`.
    pub fn new(data: T) -> Self {
        Self { data, next: AtomicPtr::new(ptr::null_mut()) }
    }

    /// Allocate a node on the heap and return a raw pointer.
    pub fn boxed(data: T) -> *mut Node<T> {
        Box::into_raw(Box::new(Self::new(data)))
    }

    /// Load next node pointer.
    pub fn next(&self) -> *mut Node<T> {
        self.next.load(Ordering::Acquire)
    }
}

/// Lock-free append-only stream for any type
#[repr(C)]
/// Lock-free append-only stream for any item type `T`.
pub struct AppendOnlyStream<T> {
    /// First node in the stream; never changes after initialization.
    head: *mut Node<T>,
    /// Tail node pointer for O(1) append progression.
    tail: AtomicPtr<Node<T>>,
}

impl<T> AppendOnlyStream<T> {
    /// Create a new stream with the given first node pointer.
    pub fn new(first: *mut Node<T>) -> Self {
        Self { head: first, tail: AtomicPtr::new(first) }
    }

    /// Return the head node pointer.
    pub fn head(&self) -> *mut Node<T> { self.head }

    /// Append a node using a Michael–Scott style enqueue.
    /// Safe for multiple producers; traversal remains lock-free.
    pub fn append(&self, new_node: *mut Node<T>) {
        // Ensure `new_node->next` is null before publishing.
        unsafe { (*new_node).next.store(ptr::null_mut(), Ordering::Relaxed) };
        loop {
            let tail = self.tail.load(Ordering::Acquire);
            let tail_next = unsafe { (*tail).next.load(Ordering::Acquire) };
            if tail_next.is_null() {
                // Try to link new_node at the observed tail.
                if unsafe { (*tail).next.compare_exchange(ptr::null_mut(), new_node, Ordering::AcqRel, Ordering::Acquire) }.is_ok() {
                    // Swing the tail forward (best-effort).
                    let _ = self.tail.compare_exchange(tail, new_node, Ordering::Release, Ordering::Relaxed);
                    return;
                }
            } else {
                // Help advance tail if another thread already linked a node.
                let _ = self.tail.compare_exchange(tail, tail_next, Ordering::Release, Ordering::Relaxed);
            }
        }
    }

    /// Iterator over nodes starting at head.
    pub fn iter(&self) -> StreamIter<T> { StreamIter { current: self.head } }

    /// Append a pre-linked chain of nodes `[first..=last]` in one operation.
    /// Assumes each node's `next` already points to the subsequent node, and `last->next` is ignored and reset to null.
    pub fn append_chain(&self, first: *mut Node<T>, last: *mut Node<T>) {
        if first.is_null() || last.is_null() { return; }
        // Ensure chain tail is null-terminated before publishing.
        unsafe { (*last).next.store(ptr::null_mut(), Ordering::Relaxed) };
        loop {
            let tail = self.tail.load(Ordering::Acquire);
            let tail_next = unsafe { (*tail).next.load(Ordering::Acquire) };
            if tail_next.is_null() {
                if unsafe { (*tail).next.compare_exchange(ptr::null_mut(), first, Ordering::AcqRel, Ordering::Acquire) }.is_ok() {
                    let _ = self.tail.compare_exchange(tail, last, Ordering::Release, Ordering::Relaxed);
                    return;
                }
            } else {
                let _ = self.tail.compare_exchange(tail, tail_next, Ordering::Release, Ordering::Relaxed);
            }
        }
    }

    /// Convenience: append many nodes by linking them locally first, then publishing once.
    pub fn append_many(&self, nodes: &[*mut Node<T>]) {
        if nodes.is_empty() { return; }
        // Link locally in-order for locality, then single publish.
        for w in nodes.windows(2) {
            let a = w[0]; let b = w[1];
            unsafe { (*a).next.store(b, Ordering::Relaxed) };
        }
        let first = nodes[0];
        let last = *nodes.last().unwrap();
        self.append_chain(first, last);
    }

    /// Build a per-document vector batch by scanning from `start` up to `max_scan` nodes.
    /// Returns the collected node pointers and the new cursor to resume from next time.
    /// TODO We can do this better - reusable vectors with specific capacity.
    pub fn build_doc_index_by<F>(&self, start: *mut Node<T>, target: DocId, max_scan: usize, doc_of: F) -> VectorisedStream<T>
    where
        F: Fn(&T) -> DocId,
    {
        let mut items: Vec<*mut Node<T>> = Vec::new();
        let mut cursor = if start.is_null() { self.head } else { start };
        let mut scanned = 0usize;
        while !cursor.is_null() && scanned < max_scan {
            let node_ref = unsafe { &*cursor };
            if doc_of(&node_ref.data) == target {
                items.push(cursor);
            }
            scanned += 1;
            cursor = node_ref.next.load(Ordering::Acquire);
        }
        VectorisedStream { items, cursor }
    }

    /// Build next batch (unfiltered) scanning from `start` up to `max_scan` nodes.
    pub fn build_next_batch(&self, start: *mut Node<T>, max_scan: usize) -> VectorisedStream<T> {
        let mut items: Vec<*mut Node<T>> = Vec::new();
        let mut cursor = if start.is_null() { self.head } else { start };
        let mut scanned = 0usize;
        while !cursor.is_null() && scanned < max_scan {
            items.push(cursor);
            scanned += 1;
            cursor = unsafe { (*cursor).next.load(Ordering::Acquire) };
        }
        VectorisedStream { items, cursor }
    }

    /// Build next batch (unfiltered) into an existing vector for capacity reuse.
    /// Returns the new cursor to resume from.
    pub fn build_next_batch_into(&self, start: *mut Node<T>, max_scan: usize, out: &mut Vec<*mut Node<T>>) -> *mut Node<T> {
        out.clear();
        let mut cursor = if start.is_null() { self.head } else { start };
        let mut scanned = 0usize;
        while !cursor.is_null() && scanned < max_scan {
            out.push(cursor);
            scanned += 1;
            cursor = unsafe { (*cursor).next.load(Ordering::Acquire) };
        }
        cursor
    }
}

/// Iterator type for traversing the stream.
pub struct StreamIter<T> { current: *mut Node<T> }

impl<T> Iterator for StreamIter<T> {
    type Item = *mut Node<T>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_null() { return None; }
        let node = self.current;
        self.current = unsafe { (*node).next.load(Ordering::Acquire) };
        Some(node)
    }
}

/// Result of building a vector index batch for a document.
pub struct VectorisedStream<T> {
    /// Collected node pointers matching the document.
    pub items: Vec<*mut Node<T>>,
    /// Cursor to resume the next batch scan from.
    pub cursor: *mut Node<T>,
}

/// Thin view for an immutable document version in chunk storage.
pub struct DocumentVersionRef<'a> {
    /// Zero-copy bytes representing the document version snapshot.
    pub bytes: &'a [u8],
}

/// Type aliases for clarity and reuse.
/// Delta Node
pub type DeltaNode<'a> = Node<Delta<'a>>;
/// Delta Stream
pub type DeltaStream<'a> = AppendOnlyStream<Delta<'a>>;
/// Vectorised Delta Stream
pub type VectorisedDeltaStream<'a> = VectorisedStream<Delta<'a>>;
/// Version Node
pub type VersionNode<'a> = Node<DocumentVersionRef<'a>>;
/// Version Stream
pub type VersionStream<'a> = AppendOnlyStream<DocumentVersionRef<'a>>;
/// Vectorised Version Stream
pub type VectorisedVersionStream<'a> = VectorisedStream<DocumentVersionRef<'a>>;