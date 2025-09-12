//! Simple bounded single-producer single-consumer ring buffer
//! - Lock-free, cache-friendly
//! - One producer thread, one consumer thread
//! - Capacity must be a power of two

use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Pad to cache line to avoid false sharing on hot indices
#[repr(align(64))]
struct CachePad;

/// Bounded SPSC ring buffer
pub struct SpscRing<T> {
    _pad0: CachePad,
    capacity: usize,
    mask: usize,
    buffer: Box<[UnsafeCell<MaybeUninit<T>>]>,
    _pad1: CachePad,
    head: AtomicUsize, // next write index
    _pad2: CachePad,
    tail: AtomicUsize, // next read index
    _pad3: CachePad,
}

unsafe impl<T: Send> Send for SpscRing<T> {}
unsafe impl<T: Send> Sync for SpscRing<T> {}

impl<T> SpscRing<T> {
    /// Create a new ring with capacity (must be power of two; will round up if not)
    pub fn with_capacity_pow2(capacity: usize) -> Self {
        let cap = capacity.next_power_of_two().max(2);
        let buffer = (0..cap)
            .map(|_| UnsafeCell::new(MaybeUninit::uninit()))
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self {
            _pad0: CachePad,
            capacity: cap,
            mask: cap - 1,
            buffer,
            _pad1: CachePad,
            head: AtomicUsize::new(0),
            _pad2: CachePad,
            tail: AtomicUsize::new(0),
            _pad3: CachePad,
        }
    }

    /// Try to push an item. Returns Err(item) if full.
    pub fn push(&self, item: T) -> Result<(), T> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        if head.wrapping_sub(tail) == self.capacity {
            return Err(item);
        }

        let idx = head & self.mask;
        unsafe {
            (*self.buffer[idx].get()) = MaybeUninit::new(item);
        }
        // Publish the write
        self.head.store(head.wrapping_add(1), Ordering::Release);
        Ok(())
    }

    /// Try to pop an item. Returns None if empty.
    pub fn pop(&self) -> Option<T> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);
        if tail == head {
            return None;
        }
        let idx = tail & self.mask;
        let value = unsafe { (*self.buffer[idx].get()).assume_init_read() };
        // Publish the read
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        Some(value)
    }

    /// Returns true if the ring is full.
    pub fn is_full(&self) -> bool {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        head.wrapping_sub(tail) == self.capacity
    }

    /// Returns true if the ring is empty.
    pub fn is_empty(&self) -> bool {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);
        tail == head
    }
}


