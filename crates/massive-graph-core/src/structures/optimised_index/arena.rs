use crossbeam::queue::SegQueue;
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

/// Bump-pointer arena with deferred reclamation.
/// - Fast alloc: atomic bump in current region, no malloc per object
/// - Deferred free: pointers retired and dropped later
pub struct Arena<T> {
    active_region: AtomicPtr<Region<T>>,   // current bump region
    region_cap: usize,                     // capacity in items per region
    regions: SegQueue<*mut Region<T>>,     // owns all regions for drop
    retire: SegQueue<*mut T>,              // retired object pointers
}

struct Region<T> {
    data: *mut MaybeUninit<T>,            // contiguous storage for T
    cap: usize,                           // number of T slots
    head: AtomicUsize,                    // next index to allocate
    ownership: Box<[MaybeUninit<T>]>,     // owns the memory
}

impl<T> Region<T> {
    fn new(cap: usize) -> Box<Self> {
        let mut vec_buf: Vec<MaybeUninit<T>> = Vec::with_capacity(cap);
        unsafe { vec_buf.set_len(cap); }
        for i in 0..cap { vec_buf[i] = MaybeUninit::<T>::uninit(); }
        let mut buf: Box<[MaybeUninit<T>]> = vec_buf.into_boxed_slice();
        let data = buf.as_mut_ptr();
        Box::new(Region { data, cap, head: AtomicUsize::new(0), ownership: buf })
    }
}

impl<T> Arena<T> {
    /// Create a new arena with default region capacity (items per region).
    pub fn new() -> Self { Self::with_region_capacity(4096) }

    /// Create a new arena with a specific region capacity (items per region).
    pub fn with_region_capacity(region_cap: usize) -> Self {
        Self {
            active_region: AtomicPtr::new(ptr::null_mut()),
            region_cap: region_cap.max(64).next_power_of_two(),
            regions: SegQueue::new(),
            retire: SegQueue::new(),
        }
    }

    #[inline]
    fn install_new_region(&self, expect: *mut Region<T>) {
        let new_region = Region::new(self.region_cap);
        let new_ptr = Box::into_raw(new_region);
        if self
            .active_region
            .compare_exchange(expect, new_ptr, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
        {
            // keep ownership to drop later
            self.regions.push(new_ptr);
        } else {
            // someone else installed; drop ours
            unsafe { drop(Box::from_raw(new_ptr)); }
        }
    }

    /// Allocate a new object (bump-pointer). Returns a raw pointer to initialized T.
    #[inline]
    pub fn alloc_new(&self, value: T) -> *mut T {
        loop {
            let rptr = self.active_region.load(Ordering::Acquire);
            if rptr.is_null() {
                self.install_new_region(ptr::null_mut());
                continue;
            }
            let region = unsafe { &*rptr };
            let idx = region.head.fetch_add(1, Ordering::Relaxed);
            if idx < region.cap {
                let slot = unsafe { region.data.add(idx) };
                let tptr = slot as *mut T;
                unsafe { ptr::write(tptr, value); }
                return tptr;
            } else {
                // region full: try to install a new one
                self.install_new_region(rptr);
            }
        }
    }

    /// Retire a raw pointer for later reclamation.
    #[inline]
    pub fn retire(&self, ptr: *mut T) { if !ptr.is_null() { self.retire.push(ptr); } }

    /// Drain retired objects and (optionally) regions. We only drop retired objects here.
    pub fn drain_now(&self) {
        while let Some(ptr) = self.retire.pop() { unsafe { ptr::drop_in_place(ptr); } }
    }
}


