use std::alloc::{alloc, dealloc, Layout};
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::ptr;
use crossbeam_epoch as epoch;

use crate::debug_log;

/// Statistics for arena allocations
#[derive(Debug, Clone)]
pub struct ArenaStats {
    /// Total bytes allocated from regions
    pub bytes_allocated: usize,
    /// Total bytes allocated directly (oversized)
    pub bytes_large: usize,
    /// Number of region allocations
    pub num_regions: usize,
    /// Number of large (direct) allocations
    pub num_large: usize,
    /// Number of retired allocations
    pub num_retired: usize,
    /// Total bytes retired
    pub bytes_retired: usize,
}

impl ArenaStats {
    /// Create empty stats
    pub fn empty() -> Self {
        Self {
            bytes_allocated: 0,
            bytes_large: 0,
            num_regions: 0,
            num_large: 0,
            num_retired: 0,
            bytes_retired: 0,
        }
    }
    
    /// Total bytes (regions + large allocations)
    pub fn total_bytes(&self) -> usize {
        self.bytes_allocated + self.bytes_large
    }
}

/// Simple bump allocator arena for colocated [Buffer | Recs | TinyMap] allocations.
/// Handles both regular region-based allocations and oversized direct allocations.
pub struct Arena {
    /// Current region being allocated from
    current: AtomicPtr<Region>,
    /// Size of each region (bytes)
    region_size: usize,
    /// Total bytes allocated from regions
    bytes_allocated: AtomicUsize,
    /// Total bytes allocated directly (large allocations)
    bytes_large: AtomicUsize,
    /// Number of regions allocated
    num_regions: AtomicUsize,
    /// Number of large allocations
    num_large: AtomicUsize,
    /// Number of retirements
    num_retired: AtomicUsize,
    /// Total bytes retired
    bytes_retired: AtomicUsize,
}

struct Region {
    /// Pointer to the start of this region's memory
    start: *mut u8,
    /// Current allocation offset within this region
    offset: AtomicPtr<u8>,
    /// End of this region's memory
    end: *mut u8,
    /// Next region in the chain (for traversal during cleanup)
    next: AtomicPtr<Region>,
}

impl Arena {
    /// Create a new arena with specified region size (default 64KB)
    pub fn new(region_size: usize) -> Self {
        Arena {
            current: AtomicPtr::new(ptr::null_mut()),
            region_size,
            bytes_allocated: AtomicUsize::new(0),
            bytes_large: AtomicUsize::new(0),
            num_regions: AtomicUsize::new(0),
            num_large: AtomicUsize::new(0),
            num_retired: AtomicUsize::new(0),
            bytes_retired: AtomicUsize::new(0),
        }
    }
    
    /// Get current statistics snapshot
    pub fn stats(&self) -> ArenaStats {
        ArenaStats {
            bytes_allocated: self.bytes_allocated.load(Ordering::Relaxed),
            bytes_large: self.bytes_large.load(Ordering::Relaxed),
            num_regions: self.num_regions.load(Ordering::Relaxed),
            num_large: self.num_large.load(Ordering::Relaxed),
            num_retired: self.num_retired.load(Ordering::Relaxed),
            bytes_retired: self.bytes_retired.load(Ordering::Relaxed),
        }
    }
    
    /// Allocate bytes from the arena with proper alignment.
    /// Handles allocations larger than region_size by allocating directly.
    pub fn alloc_bytes(&self, size: usize, align: usize) -> *mut u8 {
        // debug_log!("arena alloc_bytes oversized allocation size({}) align({})", size, align);
        
        // Catch pathological allocations
        const MAX_REASONABLE_SIZE: usize = 50 * 1024 * 1024; // 50MB
        if size > MAX_REASONABLE_SIZE {
            panic!(
                "Arena::alloc_bytes PATHOLOGICAL ALLOCATION DETECTED!\n\
                 Requested size: {} bytes ({} MB)\n\
                 Align: {}\n\
                 Region size: {} bytes\n\
                 Current stats: regions={} bytes_allocated={} bytes_large={}",
                size,
                size / (1024 * 1024),
                align,
                self.region_size,
                self.num_regions.load(Ordering::Relaxed),
                self.bytes_allocated.load(Ordering::Relaxed),
                self.bytes_large.load(Ordering::Relaxed)
            );
        }

        if size > self.region_size {
            // Oversized allocation - allocate directly
            debug_log!("arena alloc_bytes oversized allocation size({}) align({})", size, align);
            return self.alloc_large(size, align);
        }
        
        loop {
            let region_ptr = self.current.load(Ordering::Acquire);
            
            if region_ptr.is_null() {
                // No region yet, allocate first one
                if self.try_allocate_new_region() {
                    continue;
                }
                return ptr::null_mut();
            }
            
            let region = unsafe { &*region_ptr };
            
            // Try to allocate from current region
            let current_ptr = region.offset.load(Ordering::Acquire);
            let aligned_ptr = align_up_ptr(current_ptr, align);
            let new_offset = unsafe { aligned_ptr.add(size) };
            
            // Check if allocation fits in current region
            if new_offset <= region.end {
                // Try to bump the offset
                if region.offset
                    .compare_exchange(
                        current_ptr,
                        new_offset,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    )
                    .is_ok()
                {
                    // Track this allocation
                    self.bytes_allocated.fetch_add(size, Ordering::Relaxed);
                    return aligned_ptr;
                }
                // CAS failed, retry
                continue;
            }
            
            // Current region is full, allocate new one
            if self.try_allocate_new_region() {
                continue;
            }
            
            return ptr::null_mut();
        }
    }
    
    /// Allocate a large object directly (not from regions)
    fn alloc_large(&self, size: usize, align: usize) -> *mut u8 {
        unsafe {
            let layout = Layout::from_size_align_unchecked(size, align);
            let ptr = alloc(layout);
            if !ptr.is_null() {
                self.bytes_large.fetch_add(size, Ordering::Relaxed);
                self.num_large.fetch_add(1, Ordering::Relaxed);
            }
            ptr
        }
    }
    
    /// Try to allocate and install a new region
    fn try_allocate_new_region(&self) -> bool {
        unsafe {
            let layout = Layout::from_size_align_unchecked(self.region_size, 64);
            let start = alloc(layout);
            if start.is_null() {
                return false;
            }
            
            let region = Box::into_raw(Box::new(Region {
                start,
                offset: AtomicPtr::new(start),
                end: start.add(self.region_size),
                next: AtomicPtr::new(ptr::null_mut()),
            }));
            
            // Try to install this region as current
            let old_current = self.current.load(Ordering::Acquire);
            (*region).next.store(old_current, Ordering::Release);
            
            if self.current
                .compare_exchange(
                    old_current,
                    region,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                // Successfully installed new region, track it
                self.num_regions.fetch_add(1, Ordering::Relaxed);
                true
            } else {
                // Another thread installed a region, free ours
                dealloc(start, layout);
                drop(Box::from_raw(region));
                true // Still succeeded (other thread's region is available)
            }
        }
    }
    
    /// Retire a pointer via epoch-based reclamation.
    /// Caller must provide size and alignment for proper deallocation.
    pub fn retire_ptr(&self, ptr: *mut u8, size: usize, align: usize, guard: &epoch::Guard) {
        if ptr.is_null() {
            return;
        }
        
        // Track retirement
        self.num_retired.fetch_add(1, Ordering::Relaxed);
        self.bytes_retired.fetch_add(size, Ordering::Relaxed);
        
        unsafe {
            guard.defer_unchecked(move || {
                let layout = Layout::from_size_align_unchecked(size, align);
                dealloc(ptr, layout);
            });
        }
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        use crate::debug_log;
        let stats = self.stats();

        
        unsafe {
            let mut current = self.current.load(Ordering::Acquire);
            let mut freed_regions = 0;
            while !current.is_null() {
                let region = Box::from_raw(current);
                let next = region.next.load(Ordering::Acquire);
                let layout = Layout::from_size_align_unchecked(self.region_size, 64);
                dealloc(region.start, layout);
                freed_regions += 1;
                current = next;
            }
            debug_log!(
                "Arena DROP: regions={}, freed_regions={}, bytes_allocated={} bytes_large={} num_large={} bytes_retired={}",
                stats.num_regions,
                freed_regions,
                stats.bytes_allocated,
                stats.bytes_large,
                stats.num_large,
                stats.bytes_retired
            );            
        }
    }
}

unsafe impl Send for Arena {}
unsafe impl Sync for Arena {}

/// Align a pointer up to the specified alignment
#[inline]
fn align_up_ptr(ptr: *mut u8, align: usize) -> *mut u8 {
    let addr = ptr as usize;
    let aligned = (addr + align - 1) & !(align - 1);
    aligned as *mut u8
}

/// Align a size up to the specified alignment
#[inline]
pub fn align_up(size: usize, align: usize) -> usize {
    (size + align - 1) & !(align - 1)
}

