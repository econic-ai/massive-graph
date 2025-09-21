//! Simple bounded single-producer single-consumer ring buffer
//! - Lock-free, cache-friendly
//! - One producer thread, one consumer thread
//! - Capacity must be a power of two

use std::cell::UnsafeCell;
use std::mem::{size_of, MaybeUninit};
use std::sync::{atomic::{AtomicUsize, Ordering}, Arc};

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const CACHELINE: usize = 128;
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
const CACHELINE: usize = 64;
const PAD: usize = CACHELINE - size_of::<AtomicUsize>();

/// Bounded SPSC ring buffer
#[repr(C)]
pub struct SpscRing<T> {
    // Hot indices on separate cache lines
    head: AtomicUsize, // next write index (producer)
    _pad_head: [u8; PAD],
    tail: AtomicUsize, // next read index (consumer)
    _pad_tail: [u8; PAD],
    // Cold config/data
    capacity: usize,
    mask: usize,
    buffer: Box<[UnsafeCell<MaybeUninit<T>>]>,
}

unsafe impl<T: Send> Send for SpscRing<T> {}
unsafe impl<T: Send> Sync for SpscRing<T> {}

impl<T> SpscRing<T> {
    /// Create a new ring with capacity (must be power of two; will round up if not)
    #[inline(always)]
    pub fn with_capacity_pow2(capacity: usize) -> Self {
        let cap = capacity.next_power_of_two().max(2);
        let buffer = (0..cap)
            .map(|_| UnsafeCell::new(MaybeUninit::uninit()))
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self {
            head: AtomicUsize::new(0),
            _pad_head: [0u8; PAD],
            tail: AtomicUsize::new(0),
            _pad_tail: [0u8; PAD],
            capacity: cap,
            mask: cap - 1,
            buffer,
        }
    }

    /// Try to push an item. Returns Err(item) if full.
    #[inline(always)]
    pub fn push(&self, item: T) -> Result<(), T> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        if head.wrapping_sub(tail) == self.capacity {
            return Err(item);
        }

        let idx = head & self.mask;
        unsafe { (*self.buffer.get_unchecked(idx).get()) = MaybeUninit::new(item); }
        // Publish the write
        self.head.store(head.wrapping_add(1), Ordering::Release);
        Ok(())
    }

    /// Try to pop an item. Returns None if empty.
    #[inline(always)]
    pub fn pop(&self) -> Option<T> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);
        if tail == head {
            return None;
        }
        let idx = tail & self.mask;
        let value = unsafe { (*self.buffer.get_unchecked(idx).get()).assume_init_read() };
        // Publish the read
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        Some(value)
    }

    /// Returns true if the ring is full.
    #[inline(always)]
    pub fn is_full(&self) -> bool {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        head.wrapping_sub(tail) == self.capacity
    }

    /// Returns true if the ring is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);
        tail == head
    }
}

/// Batched single-producer handle. Commits head every B items.
pub struct SpscProducer<T, const B: usize> {
    ring: Arc<SpscRing<T>>,
    local_head: usize,
    published_head: usize,
    pending: usize,
}

/// Batched single-consumer handle. Commits tail every B items.
pub struct SpscConsumer<T, const B: usize> {
    ring: Arc<SpscRing<T>>,
    local_tail: usize,
    observed_head: usize,
    pending: usize,
}

impl<T> SpscRing<T> {
    /// Create batched producer/consumer handles with batch size B, owning an Arc to the ring.
    #[inline(always)]
    pub fn split_batched_owned<const B: usize>(ring: Arc<SpscRing<T>>) -> (SpscProducer<T, B>, SpscConsumer<T, B>) {
        let head = ring.head.load(Ordering::Relaxed);
        let tail = ring.tail.load(Ordering::Relaxed);
        (
            SpscProducer { ring: Arc::clone(&ring), local_head: head, published_head: head, pending: 0 },
            SpscConsumer { ring, local_tail: tail, observed_head: head, pending: 0 },
        )
    }
}

impl<T, const B: usize> SpscProducer<T, B> {
    
    /// Push an item. Returns Err(item) if full.
    #[inline(always)]
    pub fn push(&mut self, item: T) -> Result<(), T> {
        let tail = self.ring.tail.load(Ordering::Acquire);
        if self.local_head.wrapping_sub(tail) == self.ring.capacity { return Err(item); }
        let idx = self.local_head & self.ring.mask;
        unsafe { (*self.ring.buffer.get_unchecked(idx).get()) = MaybeUninit::new(item); }
        self.local_head = self.local_head.wrapping_add(1);
        self.pending += 1;
        if self.pending >= B {
            self.ring.head.store(self.local_head, Ordering::Release);
            self.published_head = self.local_head;
            self.pending = 0;
        }
        Ok(())
    }

    /// Flush the producer.
    #[inline(always)]
    pub fn flush(&mut self) {
        if self.pending > 0 {
            self.ring.head.store(self.local_head, Ordering::Release);
            self.published_head = self.local_head;
            self.pending = 0;
        }
    }
}

impl<T, const B: usize> Drop for SpscProducer<T, B> {
    fn drop(&mut self) { self.flush(); }
}

impl<T, const B: usize> SpscConsumer<T, B> {

    /// Pop an item. Returns None if empty.
    #[inline(always)]
    pub fn pop(&mut self) -> Option<T> {
        if self.local_tail == self.observed_head {
            // refresh observed head
            self.observed_head = self.ring.head.load(Ordering::Acquire);
            if self.local_tail == self.observed_head { return None; }
        }
        let idx = self.local_tail & self.ring.mask;
        let value = unsafe { (*self.ring.buffer.get_unchecked(idx).get()).assume_init_read() };
        self.local_tail = self.local_tail.wrapping_add(1);
        self.pending += 1;
        if self.pending >= B {
            self.ring.tail.store(self.local_tail, Ordering::Release);
            self.pending = 0;
        }
        Some(value)
    }

    /// Flush the consumer.
    #[inline(always)]
    pub fn flush(&mut self) {
        if self.pending > 0 {
            self.ring.tail.store(self.local_tail, Ordering::Release);
            self.pending = 0;
        }
    }
}

impl<T, const B: usize> Drop for SpscConsumer<T, B> {
    fn drop(&mut self) { self.flush(); }
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::sync::Arc;

	#[test]
	fn basic_push_pop_order() {
		let ring = SpscRing::with_capacity_pow2(4);
		assert!(ring.is_empty());
		for i in 0..4 { ring.push(i).unwrap(); }
		assert!(ring.is_full());
		for i in 0..4 { assert_eq!(ring.pop(), Some(i)); }
		assert!(ring.is_empty());
	}

	#[test]
	fn full_then_recover() {
		let ring = SpscRing::with_capacity_pow2(2);
		ring.push(1).unwrap();
		ring.push(2).unwrap();
		assert!(ring.is_full());
		assert_eq!(ring.push(3), Err(3));
		assert_eq!(ring.pop(), Some(1));
		assert!(ring.push(3).is_ok());
	}

	#[test]
	fn empty_pop_none() {
		let ring = SpscRing::<u32>::with_capacity_pow2(2);
		assert!(ring.is_empty());
		assert_eq!(ring.pop(), None);
	}

	#[test]
	fn wraparound_small_caps() {
		for cap in [2usize, 4, 8] {
			let ring = SpscRing::with_capacity_pow2(cap);
			let rounds = cap * 3;
			for i in 0..rounds {
				let _ = ring.push(i);
				let _ = ring.pop();
			}
			// Should be empty and consistent
			assert!(ring.is_empty());
		}
	}

	#[test]
	fn spsc_concurrency_order_and_count() {
		let ring = Arc::new(SpscRing::with_capacity_pow2(256));
		let total = 20_000usize;
		let prod = {
			let r = Arc::clone(&ring);
			std::thread::spawn(move || {
				for i in 0..total {
					// busy-wait push until it succeeds
					let mut v = i;
					loop { match r.push(v) { Ok(_) => break, Err(x) => { v = x; std::hint::spin_loop(); } } }
				}
			})
		};
		let cons = {
			let r = Arc::clone(&ring);
			std::thread::spawn(move || {
				let mut got = 0usize;
				let mut expected = 0usize;
				while got < total {
					if let Some(v) = r.pop() { assert_eq!(v, expected); expected += 1; got += 1; }
					else { std::hint::spin_loop(); }
				}
			})
		};
		let _ = prod.join();
		let _ = cons.join();
	}

	#[test]
	fn capacity_rounds_to_pow2() {
		let ring = SpscRing::<u8>::with_capacity_pow2(3);
		assert_eq!(ring.capacity, 4);
		let ring = SpscRing::<u8>::with_capacity_pow2(5);
		assert_eq!(ring.capacity, 8);
	}
}


