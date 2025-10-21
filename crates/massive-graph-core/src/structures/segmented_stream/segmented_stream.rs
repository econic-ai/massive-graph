use std::{
    mem::MaybeUninit,
    ptr,
    sync::atomic::{AtomicPtr, AtomicU32, AtomicU64, Ordering},
    sync::Arc,
};

use crossbeam_epoch::{self as epoch, Atomic, Owned, Shared};
use std::sync::atomic::AtomicUsize;
use std::sync::Mutex;
use std::thread::JoinHandle;

// Refer to PAGE_SIZE directly in const to avoid unused import warnings

/// Default number of entries per page for the segmented stream.
/// For tests, use a small page size to avoid allocating massive arrays.
#[cfg(not(test))]
pub const DEFAULT_PAGE_SIZE: usize = crate::constants::PAGE_SIZE;
#[cfg(test)]
pub const DEFAULT_PAGE_SIZE: usize = 64 ;

/// Append-only segmented stream of items stored in fixed-size pages.
/// Multi-writer append; multi-reader cursors; lock-free reads.
pub struct SegmentedStream<T> {
    /// Directory of all pages to keep Arc references alive.
    pages: Mutex<Vec<Arc<Page<T>>>>,
    /// The current page used for appends (tail of the list).
    active_page: Atomic<Page<T>>,
    /// Optional fixed-size page pool and recycler.
    pub pool: Option<StreamPagePool<T>>,
    /// Number of entries per page (configurable).
    page_size: usize,
    /// Threshold for pre-linking next page (page_size / 2).
    link_ahead: u32,
    /// Global page sequence counter.
    next_page_seq: AtomicU64,
}

/// Page is a 64-byte aligned struct with dynamically-sized entries.
#[repr(C, align(64))]
pub struct Page<T> {
    /// Sequence number of this page (monotonically increasing).
    page_seq: AtomicU64,
    /// Number of slots reserved by writers in this page.
    /// Writers `fetch_add(1)` to claim a slot index in [0..page_size).
    claimed: AtomicU32,
    /// Number of initialized entries visible to readers (<= claimed).
    committed: AtomicU32,
    /// Link to the next page. Set exactly once, after full init of the new page.
    next: AtomicPtr<Page<T>>,
    /// Entry storage (dynamically allocated, uninitialized until published).
    entries: Box<[MaybeUninit<T>]>,
}

/// Cursor is a pointer to a page and an index into the page.
#[allow(dead_code)]
pub struct Cursor<T> {
    page: Arc<Page<T>>,
    index: u32,
}

/// Opaque index into the segmented stream identifying a specific entry.
///
/// The index is stable for the life of the underlying page and slot.
/// Compaction routines must not move entries that are still referenced by
/// any published base or by a live radix descriptor.
#[derive(Debug)]
pub struct StreamIndex<T> {
    /// Raw pointer to the page containing the entry.
    pub(crate) page: *const Page<T>,
    /// Slot index within the page.
    pub(crate) idx: u32,
}

// Manual implementations to avoid requiring T: Copy
impl<T> Copy for StreamIndex<T> {}

impl<T> Clone for StreamIndex<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Default for StreamIndex<T> {
    fn default() -> Self {
        Self {
            page: std::ptr::null(),
            idx: 0,
        }
    }
}

// Question becmes, how do you get the memrory benefits of a page that is no longer referenced?
// I think that we need to have a way to recycle the page
// when it is no longer referenced.
// I think that we need to have a way to recycle the page

/// Errors returned by the stream API.
#[derive(Debug)]
pub enum StreamError {
    /// Unexpected allocation failure.
    AllocationFailed,
}

impl<T> Page<T> {
    /// Allocate and initialize a new empty page with the specified size and sequence number.
    fn new(page_size: usize, page_seq: u64) -> Box<Self> {
        // Allocate entries array on the heap
        let entries: Box<[MaybeUninit<T>]> = (0..page_size)
            .map(|_| MaybeUninit::uninit())
            .collect::<Vec<_>>()
            .into_boxed_slice();
        
        Box::new(Page {
            page_seq: AtomicU64::new(page_seq),
            claimed: AtomicU32::new(0),
            committed: AtomicU32::new(0),
            next: AtomicPtr::new(ptr::null_mut()),
            entries,
        })
    }

    /// Reset bookkeeping fields for reuse (does not drop entries).
    #[inline]
    fn reset_for_reuse(&self, new_seq: u64) {
        self.page_seq.store(new_seq, Ordering::Relaxed);
        self.claimed.store(0, Ordering::Relaxed);
        self.committed.store(0, Ordering::Relaxed);
        self.next.store(ptr::null_mut(), Ordering::Relaxed);
    }
    
    /// Get the page size (number of entries).
    #[inline]
    fn size(&self) -> usize {
        self.entries.len()
    }
}

impl<T> SegmentedStream<T> {
    /// Create a new stream with a single initial page using the default page size.
    pub fn new() -> Self {
        Self::with_page_size(DEFAULT_PAGE_SIZE)
    }
    
    /// Create a new stream with a custom page size.
    pub fn with_page_size(page_size: usize) -> Self {
        let link_ahead = (page_size / 2) as u32;
        // let first: Arc<Page<T>> = Arc::from(Page::<T>::new(page_size, 0));
        let first_page_box: Box<Page<T>> = Page::<T>::new(page_size, 0);
        let first_arc: Arc<Page<T>> = Arc::from(first_page_box);
        let first_ptr = Arc::as_ptr(&first_arc);


        // let first_ptr = Arc::into_raw(first.clone());
        let mut pages_vec = Vec::new();
        pages_vec.push(first_arc);
        
        // DIAGNOSTIC TEST: Initialize active_page with null instead of the actual Page
        // This tests if creating Atomic<Page> instances with real data causes the 256MB allocation
        
        SegmentedStream {
            pages: Mutex::new(pages_vec),
            // active_page: Atomic::from(Owned::from(unsafe { Box::from_raw(first_ptr as *mut Page<T>) })),
            // active_page: Atomic::null(), // NULL instead of Page
            active_page: Atomic::from(Owned::from(unsafe { 
                // This creates a NEW owned pointer from the Arc's data
                // We need to ensure the Arc stays alive
                Box::from_raw(first_ptr as *mut Page<T>)
            })),            
            pool: None,
            page_size,
            link_ahead,
            next_page_seq: AtomicU64::new(1),
        }
    }

    /// Create a new stream with a provided page pool using the default page size.
    pub fn with_pool(pool: StreamPagePool<T>) -> Self {
        Self::with_pool_and_page_size(pool, DEFAULT_PAGE_SIZE)
    }
    
    /// Create a new stream with a provided page pool and custom page size.
    pub fn with_pool_and_page_size(pool: StreamPagePool<T>, page_size: usize) -> Self {
        let link_ahead = (page_size / 2) as u32;
        let first_page_box: Box<Page<T>> = Page::<T>::new(page_size, 0);
        let first_arc: Arc<Page<T>> = Arc::from(first_page_box);
        let first_ptr = Arc::as_ptr(&first_arc);
        // let first_ptr = Arc::into_raw(first.clone());
        let mut pages_vec = Vec::new();
        pages_vec.push(first_arc);
        
        SegmentedStream {
            pages: Mutex::new(pages_vec),
            // active_page: Atomic::null(), // NULL instead of Page
            active_page: Atomic::from(Owned::from(unsafe { 
                // This creates a NEW owned pointer from the Arc's data
                // We need to ensure the Arc stays alive
                Box::from_raw(first_ptr as *mut Page<T>)
            })),                
            pool: Some(pool),
            page_size,
            link_ahead,
            next_page_seq: AtomicU64::new(1),
        }
    }

    // /// Create a new stream using a pool with pages pre-reset; avoids reset on hot path.
    // pub fn with_prereset_pool(pool: StreamPagePool<T>) -> Self {
    //     let first = Arc::from(Page::<T>::new());
    //     SegmentedStream {
    //         pages: ArcSwap::new(Arc::new(Vec::<Arc<Page<T>>>::new())),
    //         active_page: ArcSwap::from(Arc::clone(&first)),
    //         pool: Some(pool),
    //     }
    // }

    /// Create a new stream, attach a pool, and enable recycler (no gc head tracking).
    // pub fn with_pool_and_recycler(
    //     mut pool: StreamPagePool<T>,
    //     ready_capacity: usize,
    //     dirty_capacity: usize,
    // ) -> Self
    // where
    //     T: Send + Sync + 'static,
    // {
    //     let first: Arc<Page<T>> = Arc::from(Page::<T>::new());
    //     // initialize stream
    //     let stream = SegmentedStream {
    //         pages: ArcSwap::new(Arc::new(Vec::<Arc<Page<T>>>::new())),
    //         active_page: ArcSwap::from(Arc::clone(&first)),
    //         pool: None,
    //     };
    //     // start recycler (no gc head advancement)
    //     pool = pool.with_recycler(ready_capacity, dirty_capacity);
    //     SegmentedStream { pool: Some(pool), ..stream }
    // }

    /// Append one item to the stream using the multi-writer append path.
    pub fn append(&self, item: T) -> Result<(), StreamError> {
        let guard = epoch::pin();
        let mut page_shared = self.active_page.load(Ordering::Acquire, &guard);
        
        loop {
            let page = unsafe { page_shared.as_ref().unwrap() };
            let idx = page.claimed.fetch_add(1, Ordering::Relaxed);
            
            // Pre-link next page at LINK_AHEAD to reduce contention
            if idx == self.link_ahead {
                let next_ptr_probe = page.next.load(Ordering::Relaxed);
                if next_ptr_probe.is_null() {
                    let new_page = self.alloc_page_arc();
                    let new_page_ptr = Arc::as_ptr(&new_page) as *mut Page<T>;
                    if page.next.compare_exchange(ptr::null_mut(), new_page_ptr, Ordering::Release, Ordering::Relaxed).is_ok() {
                        // Add to pages Vec to keep Arc alive
                        if let Ok(mut pages) = self.pages.lock() {
                            pages.push(new_page.clone());
                        }
                        let _ = Arc::into_raw(new_page);
                    } else {
                        drop(new_page);
                    }
                }
            }
            
            if (idx as usize) < page.size() {
                // Write then publish
                unsafe {
                    let slot = page.entries.get_unchecked(idx as usize) as *const _ as *mut MaybeUninit<T>;
                    slot.write(MaybeUninit::new(item));
                    page.committed.fetch_add(1, Ordering::Release);
                }
                return Ok(());
            }
    
            // Page full: follow or link next, and update active_page if we're the one who fills it
            let next_ptr = page.next.load(Ordering::Acquire);
            if next_ptr.is_null() {
                // Try to link a new page
                let new_page = self.alloc_page_arc();
                let new_page_ptr = Arc::as_ptr(&new_page) as *mut Page<T>;
                
                match page.next.compare_exchange(
                    ptr::null_mut(), 
                    new_page_ptr, 
                    Ordering::Release, 
                    Ordering::Acquire
                ) {
                    Ok(_) => {
                        // Successfully linked new page
                        // Add to pages Vec to keep Arc alive
                        if let Ok(mut pages) = self.pages.lock() {
                            pages.push(new_page.clone());
                        }
                        // Update active_page NOW (when page is full)
                        let new_page_box = unsafe { Box::from_raw(Arc::into_raw(new_page) as *mut Page<T>) };
                        self.active_page.store(Owned::from(new_page_box), Ordering::Release);
                        // Move to new page
                        page_shared = self.active_page.load(Ordering::Acquire, &guard);
                    }
                    Err(existing_ptr) => {
                        // Another thread already linked a page
                        drop(new_page);
                        // Follow existing link
                        page_shared = unsafe { Shared::from(existing_ptr as *const _) };
                    }
                }
            } else {
                // Follow existing link
                page_shared = unsafe { Shared::from(next_ptr as *const _) };
            }
        }
    }

    /// Append one item and return its `StreamIndex` upon publish.
    pub fn append_with_index(&self, item: T) -> Result<StreamIndex<T>, StreamError> {
        let guard = epoch::pin();
        let mut page_shared = self.active_page.load(Ordering::Acquire, &guard);
        
        loop {
            let page = unsafe { page_shared.as_ref().unwrap() };
            let idx = page.claimed.fetch_add(1, Ordering::Relaxed);
            
            // Pre-link next page at LINK_AHEAD to reduce contention
            if idx == self.link_ahead {
                let next_ptr_probe = page.next.load(Ordering::Relaxed);
                if next_ptr_probe.is_null() {
                    let new_page = self.alloc_page_arc();
                    let new_page_ptr = Arc::as_ptr(&new_page) as *mut Page<T>;
                    if page.next.compare_exchange(ptr::null_mut(), new_page_ptr, Ordering::Release, Ordering::Relaxed).is_ok() {
                        // Add to pages Vec to keep Arc alive
                        if let Ok(mut pages) = self.pages.lock() {
                            pages.push(new_page.clone());
                        }
                        let _ = Arc::into_raw(new_page);
                    } else {
                        drop(new_page);
                    }
                }
            }
            
            if (idx as usize) < page.size() {
                unsafe {
                    let slot = page.entries.get_unchecked(idx as usize) as *const _ as *mut MaybeUninit<T>;
                    slot.write(MaybeUninit::new(item));
                    page.committed.fetch_add(1, Ordering::Release);
                }
                let page_ptr = page as *const Page<T>;
                return Ok(StreamIndex { page: page_ptr, idx });
            }
            
            // Page full: follow or link next, and update active_page if we're the one who fills it
            let next_ptr = page.next.load(Ordering::Acquire);
            if next_ptr.is_null() {
                // Try to link a new page
                let new_page = self.alloc_page_arc();
                let new_page_ptr = Arc::as_ptr(&new_page) as *mut Page<T>;
                
                match page.next.compare_exchange(
                    ptr::null_mut(),
                    new_page_ptr,
                    Ordering::Release,
                    Ordering::Acquire,
                ) {
                    Ok(_) => {
                        // Successfully linked new page
                        // Add to pages Vec to keep Arc alive
                        if let Ok(mut pages) = self.pages.lock() {
                            pages.push(new_page.clone());
                        }
                        // Update active_page NOW (when page is full)
                        let new_page_box = unsafe { Box::from_raw(Arc::into_raw(new_page) as *mut Page<T>) };
                        self.active_page.store(Owned::from(new_page_box), Ordering::Release);
                        // Move to new page
                        page_shared = self.active_page.load(Ordering::Acquire, &guard);
                    }
                    Err(existing_ptr) => {
                        // Another thread already linked a page
                        drop(new_page);
                        // Follow existing link
                        page_shared = unsafe { Shared::from(existing_ptr as *const _) };
                    }
                }
            } else {
                // Follow existing link
                page_shared = unsafe { Shared::from(next_ptr as *const _) };
            }
        }
    }

    /// Resolve a `StreamIndex` to an immutable reference if the entry is still present.
    #[inline]
    pub fn resolve_ref<'a>(&'a self, idx: &StreamIndex<T>) -> Option<&'a T> {
        // SAFETY: The page lifetime is managed by leaked Arcs in the linked list.
        // This returns None if the slot was never committed.
        let page = unsafe { &*idx.page };
        let cap = page.committed.load(Ordering::Acquire);
        if idx.idx < cap {
            let ptr_t = unsafe { page.entries.get_unchecked(idx.idx as usize) as *const _ as *const T };
            Some(unsafe { &*ptr_t })
        } else {
            None
        }
    }

    /// Resolve a `StreamIndex` to `&T` without checking committed.
    #[inline]
    pub fn resolve_ref_unchecked<'a>(&'a self, idx: &StreamIndex<T>) -> &'a T {
        // SAFETY: Caller must ensure the referenced entry was committed before exposure
        // and that pages will not be reclaimed for the duration of the borrow (e.g., via epochs).
        let page = unsafe { &*idx.page };
        let ptr_t = unsafe { page.entries.get_unchecked(idx.idx as usize) as *const _ as *const T };
        unsafe { &*ptr_t }
    }

    /// Allocate a fresh page or pop one from the pool if available.
    #[inline]
    fn alloc_page_arc(&self) -> Arc<Page<T>> {
        let seq = self.next_page_seq.fetch_add(1, Ordering::Relaxed);
        if let Some(pool) = self.pool.as_ref() {
            if let Some(p) = pool.pop_ready() {
                p.reset_for_reuse(seq);
                return p;
            }
        }
        Arc::from(Page::<T>::new(self.page_size, seq))
    }

}

impl<T> Cursor<T> {
    /// Create a new cursor positioned at the head of the stream.
    pub fn new_at_head(stream: &SegmentedStream<T>) -> Self {
        // Get the first page from the pages Vec
        let pages = stream.pages.lock().unwrap();
        let head = Arc::clone(&pages[0]);
        drop(pages);
        
        Cursor {
            page: head,
            index: 0,
        }
    }

    /// Read the next item if available; hops pages automatically.
    pub fn next<'a>(&'a mut self) -> Option<&'a T> {
        let mut idx = self.index;

        loop {
            let (cap, res_opt, next_ptr) = unsafe {
                let page = &*self.page;
                let cap = page.committed.load(Ordering::Acquire);
                if idx < cap {
                    let ptr_t = page.entries.get_unchecked(idx as usize) as *const _ as *const T;
                    let r: &T = &*ptr_t;
                    (cap, Some(r), ptr::null_mut())
                } else {
                    // Advisory prefetch if we're near the end of a page
                    // No explicit prefetch to avoid unstable intrinsics
                    let n = page.next.load(Ordering::Acquire);
                    (cap, None, n)
                }
            };
            if let Some(r) = res_opt {
                // Advance local index and persist.
                idx += 1;
                self.index = idx;
                return Some(r);
            }
            // Check if we're at the tail of the current page
            let page_size = unsafe { (*Arc::as_ptr(&self.page)).size() };
            if cap < page_size as u32 {
                // Tail of current page; no more items yet.
                return None;
            }
            // Page full; try to hop to next.
            if next_ptr.is_null() {
                return None;
            }
            // SAFETY: The page is kept alive by the pages Vec
            // Reconstruct Arc from raw pointer (doesn't increment count)
            let next_arc = unsafe { Arc::from_raw(next_ptr) };
            // Clone to keep our reference, then forget the reconstructed Arc
            self.page = Arc::clone(&next_arc);
            std::mem::forget(next_arc);
            idx = 0;
            self.index = 0;
        }
    }

    /// Return a batch slice of initialized items in the current page.
    /// If none are available, returns an empty slice. Hops on next call.
    pub fn next_batch<'a>(&'a mut self) -> &'a [T] {
        let idx = self.index;
        let (cap, slice_opt, next_ptr) = unsafe {
            let page = &*self.page;
            let cap = page.committed.load(Ordering::Acquire);
            if idx < cap {
                let len = (cap - idx) as usize;
                let base = page.entries.get_unchecked(idx as usize) as *const _ as *const T;
                let slice_ref = std::slice::from_raw_parts(base, len);
                (cap, Some(slice_ref), ptr::null_mut())
            } else {
                let n = page.next.load(Ordering::Acquire);
                (cap, None, n)
            }
        };
        if let Some(slice) = slice_opt {
            self.index = cap;
            return slice;
        } else {
            let page_size = unsafe { (*Arc::as_ptr(&self.page)).size() };
            if cap == page_size as u32 {
                // Full page and consumed; try to hop and expose empty slice for now.
                if !next_ptr.is_null() {
                    // SAFETY: The page is kept alive by the pages Vec
                    // Reconstruct Arc from raw pointer (doesn't increment count)
                    let next = unsafe { Arc::from_raw(next_ptr) };
                    // Clone to keep our reference, then forget the reconstructed Arc
                    self.page = Arc::clone(&next);
                    std::mem::forget(next);
                    self.index = 0;
                }
                &[]
            } else {
                // Tail without new items.
                &[]
            }
        }
    }
}

impl<T> Drop for SegmentedStream<T> {
    fn drop(&mut self) {
        // let dropped = STREAM_DROPPED.fetch_add(1, Ordering::Relaxed) + 1;
        
        // // Print stats every 10,000 drops
        // if dropped % 10000 == 0 {
        //     let created = STREAM_CREATED.load(Ordering::Relaxed);
        //     eprintln!(
        //         "SegmentedStream stats: created={}, dropped={}, active={}",
        //         created, dropped, created - dropped
        //     );
        // }
        
        // The active_page Atomic<Page<T>> will drop here
        // This is where epoch may defer the Page to garbage collection
        // With 522K rapid drops, this could cause epoch's internal structures to grow
    }
}

/// Fixed-size, lock-free(ish) page pool with round-robin allocation.
pub struct StreamPagePool<T> {
    pages: Vec<Arc<Page<T>>>,
    next: AtomicUsize,
    // Ready ring for recycled pages
    ready: Option<Arc<Ring<T>>>,
    // Dirty ring for full pages pending recycle
    dirty: Option<Arc<Ring<T>>>,
    recycler: Option<JoinHandle<()>>,
}

impl<T> StreamPagePool<T> {
    /// Create a pool with `capacity` pre-initialized pages.
    pub fn with_capacity(capacity: usize) -> Self {
        let mut pages = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            pages.push(Arc::from(Page::<T>::new(64, 0)));
        }
        StreamPagePool { pages, next: AtomicUsize::new(0), ready: None, dirty: None, recycler: None }
    }

    /// Returns a page in round-robin order. Always Some for capacity > 0.
    #[inline]
    pub fn get(&self) -> Option<Arc<Page<T>>> {
        let cap = self.pages.len();
        if cap == 0 { return None; }
        let idx = self.next.fetch_add(1, Ordering::Relaxed) % cap;
        Some(Arc::clone(&self.pages[idx]))
    }

    /// Initialize recycler rings and background thread.
    pub fn with_recycler(mut self, ready_capacity: usize, dirty_capacity: usize) -> Self
    where
        T: Send + Sync + 'static,
    {
        self.ready = Some(Arc::new(Ring::new(ready_capacity)));
        self.dirty = Some(Arc::new(Ring::new(dirty_capacity)));
        let ready = Arc::clone(self.ready.as_ref().unwrap());
        let dirty = Arc::clone(self.dirty.as_ref().unwrap());
        let handle = std::thread::spawn(move || {
            loop {
                if let Some(mut_ptr) = dirty.pop_ptr() {
                    // Reconstruct Arc ownership
                    let page_arc: Arc<Page<T>> = unsafe { Arc::from_raw(mut_ptr) };
                    // Check eligibility: full page
                    let page_size = page_arc.size();
                    if page_arc.committed.load(Ordering::Acquire) == page_size as u32 {
                        // Reset with a new sequence number (will be set by alloc_page_arc)
                        page_arc.reset_for_reuse(0);
                        // Move to ready ring
                        let _ = ready.push_arc(page_arc);
                    } else {
                        // Not ready yet, requeue and back off
                        let raw = Arc::into_raw(page_arc) as *mut Page<T>;
                        let _ = dirty.push_ptr(raw);
                        std::thread::yield_now();
                    }
                } else {
                    std::thread::sleep(std::time::Duration::from_micros(50));
                }
            }
        });
        self.recycler = Some(handle);
        self
    }

    /// Start a background prefiller that keeps the ready ring topped up by allocating new pages.
    pub fn with_prefiller(mut self, ready_capacity: usize) -> Self
    where
        T: Send + Sync + 'static,
    {
        let ready_capacity = ready_capacity.min(100).next_power_of_two().max(1);
        self.ready = Some(Arc::new(Ring::new(ready_capacity)));
        let ready = Arc::clone(self.ready.as_ref().unwrap());
        let handle = std::thread::spawn(move || {
            loop {
                // Fill as many slots as available up to a batch cap
                let available = ready.capacity().saturating_sub(ready.len());
                if available == 0 {
                    std::thread::sleep(std::time::Duration::from_micros(50));
                    continue;
                }
                let batch = available.min(8);
                for _ in 0..batch {
                    let page = Arc::from(Page::<T>::new(64, 0));
                    if !ready.push_arc(page) { break; }
                }
                // Yield to allow consumers to make progress
                std::thread::yield_now();
            }
        });
        self.recycler = Some(handle);
        self
    }

    #[inline]
    fn push_dirty_arc(&self, page: Arc<Page<T>>) {
        if let Some(dirty) = self.dirty.as_ref() {
            let raw = Arc::into_raw(page) as *mut Page<T>;
            let _ = dirty.push_ptr(raw);
        }
    }

    #[inline]
    fn pop_ready(&self) -> Option<Arc<Page<T>>> {
        if let Some(ready) = self.ready.as_ref() {
            if let Some(ptr) = ready.pop_ptr() {
                let arc = unsafe { Arc::from_raw(ptr) };
                return Some(arc);
            }
        }
        None
    }
}

/// Bounded MPSC ring storing owned Arc<Page<T>> as raw pointers.
pub struct Ring<T> {
    slots: Vec<AtomicPtr<Page<T>>>,
    head: AtomicUsize, // consumer
    tail: AtomicUsize, // producer
    mask: usize,
    cap: usize,
    count: AtomicUsize,
}

impl<T> Ring<T> {
    fn new(capacity: usize) -> Self {
        assert!(capacity.is_power_of_two(), "capacity must be power of two");
        let mut slots = Vec::with_capacity(capacity);
        for _ in 0..capacity { slots.push(AtomicPtr::new(ptr::null_mut())); }
        Ring {
            slots,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            mask: capacity - 1,
            cap: capacity,
            count: AtomicUsize::new(0),
        }
    }

    #[inline]
    fn push_arc(&self, page: Arc<Page<T>>) -> bool {
        let raw = Arc::into_raw(page) as *mut Page<T>;
        self.push_ptr(raw)
    }

    #[inline]
    fn push_ptr(&self, ptr_page: *mut Page<T>) -> bool {
        let tail = self.tail.fetch_add(1, Ordering::Relaxed);
        let idx = tail & self.mask;
        let slot = &self.slots[idx];
        // Try to store if empty
        if slot.compare_exchange(ptr::null_mut(), ptr_page, Ordering::Release, Ordering::Relaxed).is_ok() {
            self.count.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            // Failed; ring full. Drop the pushed page to avoid leak.
            unsafe { let _ = Arc::from_raw(ptr_page); }
            false
        }
    }

    #[inline]
    fn pop_ptr(&self) -> Option<*mut Page<T>> {
        let head = self.head.fetch_add(1, Ordering::Relaxed);
        let idx = head & self.mask;
        let slot = &self.slots[idx];
        let ptr_cur = slot.swap(ptr::null_mut(), Ordering::Acquire);
        if ptr_cur.is_null() {
            None
        } else {
            self.count.fetch_sub(1, Ordering::Relaxed);
            Some(ptr_cur)
        }
    }

    #[inline]
    fn len(&self) -> usize { self.count.load(Ordering::Relaxed) }

    #[inline]
    fn capacity(&self) -> usize { self.cap }
}
