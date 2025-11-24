use core::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

use crate::debug_log;

/// Default region capacity (in items) for new arenas.
/// This is a reasonable default for small allocations.
const DEFAULT_REGION_CAPACITY: usize = 4096;

/// Minimum region capacity to ensure efficient allocation.
const MIN_REGION_CAPACITY: usize = 64;

/// Epoch token representing a pinned GC epoch.
pub struct EpochToken {
    _private: (),
}

impl EpochToken {
    /// Pin the current epoch; values protected by this epoch must not be reclaimed.
    pub fn pin() -> Self { EpochToken { _private: () } }
}

/// Region identifier for arena-backed storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RegionId(pub u32);

/// Zero-copy handle to an immutable value in a non-moving, epoch-protected region.
/// SAFETY: `ptr` must remain valid for the lifetime of any active `EpochToken` that protects it.
#[derive(Copy, Clone)]
pub struct ZeroCopy<V> {
    /// Raw pointer to the immutable value.
    pub ptr: *const V,
    /// Epoch identifier protecting the allocation.
    pub epoch_id: u64,
    /// Region that owns the allocation.
    pub region_id: RegionId,
    _pd: PhantomData<V>,
}

// Handles are shareable across threads under epoch discipline.
unsafe impl<V: Send + Sync> Send for ZeroCopy<V> {}
unsafe impl<V: Send + Sync> Sync for ZeroCopy<V> {}

impl<V> ZeroCopy<V> {
    /// Create a new handle from a raw pointer, epoch and region. Caller guarantees safety.
    pub unsafe fn new_from_ptr(ptr: *const V, epoch_id: u64, region_id: RegionId) -> Self {
        ZeroCopy { ptr, epoch_id, region_id, _pd: PhantomData }
    }

    /// Borrow the underlying value; caller must ensure an active `EpochToken` protecting the region.
    #[inline]
    pub unsafe fn borrow<'a>(&self, _epoch: &'a EpochToken) -> &'a V { &*self.ptr }
}

/// Simple non-moving bump arena with deferred retire list (stubs for now).
pub struct Arena<T> {
    active_region: AtomicPtr<Region<T>>,   // current bump region
    region_cap: usize,                     // capacity in items per region
    regions: crossbeam::queue::SegQueue<*mut Region<T>>,
    retired: crossbeam::queue::SegQueue<*mut T>,
    total_allocated: AtomicUsize,          // Total bytes allocated (for diagnostics)
    alloc_count: AtomicUsize,              // Number of allocations performed
    retire_count: AtomicUsize,             // Number of retirements performed
}

struct Region<T> {
    data: *mut MaybeUninit<T>,
    cap: usize,
    head: AtomicUsize,
    _ownership: Box<[MaybeUninit<T>]>,
}

impl<T> Region<T> {
    fn new(cap: usize) -> Box<Self> {
        let mut vec_buf: Vec<MaybeUninit<T>> = Vec::with_capacity(cap);
        unsafe { vec_buf.set_len(cap); }
        for i in 0..cap { vec_buf[i] = MaybeUninit::<T>::uninit(); }
        let mut buf: Box<[MaybeUninit<T>]> = vec_buf.into_boxed_slice();
        let data = buf.as_mut_ptr();
        Box::new(Region { data, cap, head: AtomicUsize::new(0), _ownership: buf })
    }
}

impl<T> Arena<T> {
    /// Create a new arena with default region capacity.
    pub fn new() -> Self { 
        Self::with_region_capacity(DEFAULT_REGION_CAPACITY) 
    }

    /// Create a new arena with specified region capacity (in items of type T).
    /// Region capacity will be clamped to MIN_REGION_CAPACITY and rounded up to next power of two.
    pub fn with_region_capacity(region_cap: usize) -> Self {
        debug_log!("arena with_region_capacity region_cap({})", region_cap);
        Self {
            active_region: AtomicPtr::new(ptr::null_mut()),
            region_cap: region_cap.max(MIN_REGION_CAPACITY).next_power_of_two(),
            regions: crossbeam::queue::SegQueue::new(),
            retired: crossbeam::queue::SegQueue::new(),
            total_allocated: AtomicUsize::new(0),
            alloc_count: AtomicUsize::new(0),
            retire_count: AtomicUsize::new(0),
        }
    }
    
    /// Get total bytes allocated by this arena (for diagnostics)
    pub fn total_allocated_bytes(&self) -> usize {
        self.total_allocated.load(Ordering::Relaxed)
    }
    
    /// Get total number of allocations performed (for diagnostics)
    pub fn alloc_count(&self) -> usize {
        self.alloc_count.load(Ordering::Relaxed)
    }
    
    /// Get total number of retirements performed (for diagnostics)
    pub fn retire_count(&self) -> usize {
        self.retire_count.load(Ordering::Relaxed)
    }
    
    /// Get total number of regions allocated (for diagnostics)
    pub fn region_count(&self) -> usize {
        let mut count = 0;
        while self.regions.pop().is_some() {
            count += 1;
        }
        count
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
            self.regions.push(new_ptr);
        } else {
            unsafe { drop(Box::from_raw(new_ptr)); }
        }
    }

    #[inline]
    pub fn alloc_new(&self, value: T) -> *mut T {
        debug_log!("arena alloc_new");
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
                self.install_new_region(rptr);
            }
        }
    }

    /// Allocate space for N contiguous items (uninitialized).
    /// Returns a pointer to the first element.
    /// Caller is responsible for initializing all N elements.
    #[inline]
    pub fn alloc_array(&self, count: usize) -> *mut T {
        debug_log!("arena alloc_array count({})", count);
        if count == 0 { return ptr::null_mut(); }
        loop {
            let rptr = self.active_region.load(Ordering::Acquire);
            if rptr.is_null() {
                self.install_new_region(ptr::null_mut());
                continue;
            }
            let region = unsafe { &*rptr };
            let idx = region.head.fetch_add(count, Ordering::Relaxed);
            if idx + count <= region.cap {
                let slot = unsafe { region.data.add(idx) };
                return slot as *mut T;
            } else {
                // Not enough space in this region, install a new one
                self.install_new_region(rptr);
            }
        }
    }

    /// Allocate space for N contiguous items with specified alignment (uninitialized).
    /// Returns a properly aligned pointer to the first element.
    /// Caller is responsible for initializing all N elements.
    #[inline]
    pub fn alloc_array_aligned(&self, count: usize, align: usize) -> *mut T {
        debug_log!("arena alloc_array_aligned count({}) align({})", count, align);
        if count == 0 { return ptr::null_mut(); }
        loop {
            let rptr = self.active_region.load(Ordering::Acquire);
            if rptr.is_null() {
                self.install_new_region(ptr::null_mut());
                continue;
            }
            let region = unsafe { &*rptr };
            
            // Reserve space with extra padding for alignment
            let max_padding = (align - 1) / core::mem::size_of::<T>();
            let idx = region.head.fetch_add(count + max_padding, Ordering::Relaxed);
            
            if idx + count + max_padding <= region.cap {
                // Calculate aligned position from our reserved space
                let base_addr = unsafe { region.data.add(idx) } as usize;
                let aligned_addr = (base_addr + align - 1) & !(align - 1);
                let offset = (aligned_addr - base_addr) / core::mem::size_of::<T>();
                let slot = unsafe { region.data.add(idx + offset) };
                
                // Track allocation
                let bytes_allocated = count * core::mem::size_of::<T>();
                self.total_allocated.fetch_add(bytes_allocated, Ordering::Relaxed);
                self.alloc_count.fetch_add(1, Ordering::Relaxed);
                
                return slot as *mut T;
            } else {
                // Not enough space, install new region
                self.install_new_region(rptr);
            }
        }
    }

    #[inline]
    pub fn retire(&self, ptr_t: *mut T) {
        if !ptr_t.is_null() {
            self.retired.push(ptr_t);
            self.retire_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn drain_now(&self) {
        while let Some(ptr_t) = self.retired.pop() { unsafe { ptr::drop_in_place(ptr_t); } }
    }
}

// StoragePolicy removed: index is type-agnostic over V; ZeroCopy remains as a handle type.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zerocopy_borrow_roundtrip() {
        let arena = Arena::<u64>::new();
        let p = arena.alloc_new(99);
        let z = unsafe { ZeroCopy::new_from_ptr(p as *const u64, 1, RegionId(0)) };
        let epoch = EpochToken::pin();
        let v = unsafe { z.borrow(&epoch) };
        assert_eq!(*v, 99);
        arena.retire(p);
        arena.drain_now();
    }
}


