use crossbeam::queue::SegQueue;

/// Minimal generic arena for deferred reclamation.
/// Allocates objects on the heap and supports retiring pointers for later free.
pub struct Arena<T> {
    retire: SegQueue<*mut T>,
}

impl<T> Arena<T> {
    /// Create a new arena.
    pub fn new() -> Self { Self { retire: SegQueue::new() } }

    /// Allocate a new object and return a raw pointer.
    #[inline]
    pub fn alloc_new(&self, value: T) -> *mut T { Box::into_raw(Box::new(value)) }

    /// Retire a raw pointer for later reclamation.
    #[inline]
    pub fn retire(&self, ptr: *mut T) { if !ptr.is_null() { self.retire.push(ptr); } }

    /// Drain retired objects immediately (tests/benches). In prod, call periodically.
    pub fn drain_now(&self) {
        while let Some(ptr) = self.retire.pop() { unsafe { drop(Box::from_raw(ptr)); } }
    }
}


