use std::{
    mem::MaybeUninit,
    ptr,
    sync::atomic::{AtomicPtr, AtomicU32, Ordering},
    sync::Arc,
};

use arc_swap::ArcSwap;
use std::sync::atomic::AtomicUsize;
use std::thread::JoinHandle;

// Refer to PAGE_SIZE directly in const to avoid unused import warnings

/// Number of entries per page for the segmented stream.
/// For tests, use a small page size to avoid allocating massive arrays on the stack
/// during `Page::new()` initialization.
#[cfg(test)]
const ENTRIES_PER_PAGE: usize = 64;
#[cfg(not(test))]
const ENTRIES_PER_PAGE: usize = crate::constants::PAGE_SIZE;


const LINK_AHEAD: u32 = (ENTRIES_PER_PAGE / 2) as u32;

/// Append-only segmented stream of items stored in fixed-size pages.
/// Multi-writer append; multi-reader cursors; lock-free reads.
pub struct SegmentedStream<T> {
    /// Optional directory of pages (unused in this minimal pass).
    #[allow(dead_code)]
    pages: ArcSwap<Vec<Arc<Page<T>>>>,
    /// The current page used for appends (tail of the list).
    active_page: ArcSwap<Page<T>>, 
    /// Optional fixed-size page pool and recycler.
    pub pool: Option<StreamPagePool<T>>,
}

/// Page is a 64-byte aligned array of entries.
#[repr(C, align(64))]
pub struct Page<T> {
    /// Number of slots reserved by writers in this page.
    /// Writers `fetch_add(1)` to claim a slot index in [0..ENTRIES_PER_PAGE).
    claimed: AtomicU32,
    /// Number of initialized entries visible to readers (<= claimed).
    committed: AtomicU32,
    /// Link to the next page. Set exactly once, after full init of the new page.
    next: AtomicPtr<Page<T>>,
    /// Number of active readers currently positioned on this page.
    readers: AtomicU32,
    /// Entry storage (uninitialized until published).
    entries: [MaybeUninit<T>; ENTRIES_PER_PAGE],
}

/// Cursor is a pointer to a page and an index into the page.
#[allow(dead_code)]
pub struct Cursor<T> {
    page: Arc<Page<T>>,
    index: u32,
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
    /// Allocate and initialize a new empty page.
    fn new() -> Box<Self> {
        // Allocate the Page on the heap uninitialized to avoid large stack frames
        // with big entry arrays. Initialize fields in place.
        let mut page = Box::<Page<T>>::new_uninit();
        unsafe {
            let ptr_page = page.as_mut_ptr();
            // Initialize scalar/atomic fields
            ptr::addr_of_mut!((*ptr_page).claimed).write(AtomicU32::new(0));
            ptr::addr_of_mut!((*ptr_page).committed).write(AtomicU32::new(0));
            ptr::addr_of_mut!((*ptr_page).next).write(AtomicPtr::new(ptr::null_mut()));
            ptr::addr_of_mut!((*ptr_page).readers).write(AtomicU32::new(0));
            // The entries field is [MaybeUninit<T>; N]; leaving it uninitialized is valid.
            page.assume_init()
        }
    }

    /// Reset bookkeeping fields for reuse (does not drop entries).
    #[inline]
    fn reset_for_reuse(&self) {
        self.claimed.store(0, Ordering::Relaxed);
        self.committed.store(0, Ordering::Relaxed);
        self.next.store(ptr::null_mut(), Ordering::Relaxed);
        self.readers.store(0, Ordering::Relaxed);
    }
}

impl<T> SegmentedStream<T> {
    /// Create a new stream with a single initial page.
    pub fn new() -> Self {
        let first: Arc<Page<T>> = Arc::from(Page::<T>::new());
        SegmentedStream {
            pages: ArcSwap::new(Arc::new(Vec::<Arc<Page<T>>>::new())),
            active_page: ArcSwap::from(Arc::clone(&first)),
            pool: None,
        }
    }

    /// Create a new stream with a provided page pool.
    pub fn with_pool(pool: StreamPagePool<T>) -> Self {
        let first = Arc::from(Page::<T>::new());
        SegmentedStream {
            pages: ArcSwap::new(Arc::new(Vec::<Arc<Page<T>>>::new())),
            active_page: ArcSwap::from(Arc::clone(&first)),
            pool: Some(pool),
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
        let curr = self.active_page.load_full(); // Arc<Page<T>>
        // let mut page: *const Page<T> = &*curr;
        let mut page: &Page<T> = &*curr;
        
        loop {
            let idx = page.claimed.fetch_add(1, Ordering::Relaxed);
            // Link ahead at half capacity to reduce contention at boundary
            if idx == LINK_AHEAD {
                let next_ptr_probe = page.next.load(Ordering::Relaxed);
                if next_ptr_probe.is_null() {
                    let new_page = self.alloc_page_arc();
                    let new_page_ptr = Arc::as_ptr(&new_page) as *mut Page<T>;
                    if page.next.compare_exchange(ptr::null_mut(), new_page_ptr, Ordering::Release, Ordering::Relaxed).is_ok() {
                        // Hint: update active_page; relaxed suffices as it's a hint only
                        self.active_page.store(new_page.clone());
                        let _ = Arc::into_raw(new_page);
                    } else {
                        drop(new_page);
                    }
                }
            }
            if (idx as usize) < ENTRIES_PER_PAGE {
                // Write then publish
                unsafe {
                    let slot = page.entries.get_unchecked(idx as usize) as *const _ as *mut MaybeUninit<T>;
                    slot.write(MaybeUninit::new(item));
                    page.committed.fetch_add(1, Ordering::Release);
                }
                return Ok(());
            }
    
            // Page full: follow or link next
            let next_ptr = page.next.load(Ordering::Relaxed);
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
                        // Successfully linked new page - only this thread updates active_page
                        self.active_page.store(new_page.clone());
                        // Leak one strong count for the raw pointer stored in next
                        let _ = Arc::into_raw(new_page);
                        // Enqueue current full page for recycling if pool has recycler
                        if let Some(pool) = self.pool.as_ref() {
                            pool.push_dirty_arc(Arc::clone(&curr));
                        }
                        page = unsafe { &*new_page_ptr };
                    }
                    Err(existing_ptr) => {
                        // Another thread already linked a page
                        // We still own new_page, so just drop it normally
                        drop(new_page);
                        // Use the existing page
                        page = unsafe { &*existing_ptr };
                    }
                }
            } else {
                // Follow existing link
                page = unsafe { &*next_ptr };
            }
        }
    }

    /// Allocate a fresh page or pop one from the pool if available.
    #[inline]
    fn alloc_page_arc(&self) -> Arc<Page<T>> {
        if let Some(pool) = self.pool.as_ref() {
            if let Some(p) = pool.pop_ready() { return p; }
        }
        Arc::from(Page::<T>::new())
    }

}

impl<T> Cursor<T> {
    /// Create a new cursor positioned at the head of the stream.
    pub fn new_at_head(stream: &SegmentedStream<T>) -> Self {
        let head = stream.active_page.load_full();
        head.readers.fetch_add(1, Ordering::Relaxed);
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
            if cap < ENTRIES_PER_PAGE as u32 {
                // Tail of current page; no more items yet.
                return None;
            }
            // Page full; try to hop to next.
            if next_ptr.is_null() {
                return None;
            }
            // SAFETY: The page is kept alive by the previous page holding an Arc reference to it
            // Decrement readers on old page, increment on new
            self.page.readers.fetch_sub(1, Ordering::Relaxed);
            let next_arc = unsafe { Arc::from_raw(next_ptr) };
            next_arc.readers.fetch_add(1, Ordering::Relaxed);
            self.page = next_arc;
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
        } else if cap == ENTRIES_PER_PAGE as u32 {
            // Full page and consumed; try to hop and expose empty slice for now.
            if !next_ptr.is_null() {
                // SAFETY: The page is kept alive by the previous page holding an Arc reference to it
                // Decrement readers on old page, increment on new
                self.page.readers.fetch_sub(1, Ordering::Relaxed);
                let next = unsafe { Arc::from_raw(next_ptr) };
                next.readers.fetch_add(1, Ordering::Relaxed);
                self.page = next;
                self.index = 0;
            }
            &[]
        } else {
            // Tail without new items.
            &[]
        }
    }
}

impl<T> Drop for Cursor<T> {
    fn drop(&mut self) {
        self.page.readers.fetch_sub(1, Ordering::Relaxed);
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
            pages.push(Arc::from(Page::<T>::new()));
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
                    // Check eligibility: full and no readers
                    if page_arc.committed.load(Ordering::Acquire) == ENTRIES_PER_PAGE as u32
                        && page_arc.readers.load(Ordering::Relaxed) == 0
                    {
                        page_arc.reset_for_reuse();
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
                    let page = Arc::from(Page::<T>::new());
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // NOTE: For basic tests we avoid forcing page rollover because ENTRIES_PER_PAGE
    // maps to PAGE_SIZE. Advanced rollover tests are provided as ignored skeletons below.

    #[test]
    fn stream_new_initial_state() {
        // Goal: Ensure a new stream has a single empty page with next == null
        let s: SegmentedStream<u32> = SegmentedStream::new();
        let head = s.active_page.load_full();
        assert_eq!(head.claimed.load(Ordering::Relaxed), 0);
        assert_eq!(head.committed.load(Ordering::Relaxed), 0);
        assert!(head.next.load(Ordering::Acquire).is_null());
    }

    #[test]
    fn single_writer_append_and_read_order() {
        // Goal: Appending a few items makes them visible to a cursor in order
        let s: SegmentedStream<u32> = SegmentedStream::new();
        assert!(s.append(10).is_ok());
        assert!(s.append(20).is_ok());
        assert!(s.append(30).is_ok());

        let mut c = Cursor::new_at_head(&s);
        // Call next() sequentially and copy values to avoid holding borrows across calls
        let mut got = Vec::new();
        if let Some(v) = c.next() { got.push(*v); }
        if let Some(v) = c.next() { got.push(*v); }
        if let Some(v) = c.next() { got.push(*v); }
        assert_eq!(got, vec![10, 20, 30]);

        // Tail: no more items yet
        assert!(c.next().is_none());
    }

    #[test]
    fn next_batch_basic() {
        // Goal: next_batch returns the committed suffix from current index
        let s: SegmentedStream<u32> = SegmentedStream::new();
        for v in [1_u32, 2, 3] {
            s.append(v).unwrap();
        }
        let mut c = Cursor::new_at_head(&s);
        let batch = c.next_batch();
        assert_eq!(batch, &[1, 2, 3]);
        // Subsequent call at tail yields empty slice
        let batch2 = c.next_batch();
        assert!(batch2.is_empty());
    }

    // ---------- Skeletons for advanced tests (ignored for now) ----------

    #[test]
    fn page_boundary_and_linking_single_link_guarantee() {
        // Goal: Force rollover with small test ENTRIES_PER_PAGE, verify linking and counters
        let s: SegmentedStream<u32> = SegmentedStream::new();
        // Fill the first page exactly
        for i in 0..(ENTRIES_PER_PAGE as u32) {
            s.append(i).unwrap();
        }
        // First page should be full; next is still null until the next append attempts to link
        let head = s.active_page.load_full();
        assert_eq!(head.committed.load(Ordering::Acquire), ENTRIES_PER_PAGE as u32);
        // Trigger linking by appending a few more
        s.append(100).unwrap();
        s.append(101).unwrap();

        let next_ptr = head.next.load(Ordering::Acquire);
        assert!(!next_ptr.is_null(), "next page should be linked after rollover");
        let next_page = unsafe { &*next_ptr };
        // The new page should have at least the two appended items committed
        assert!(next_page.committed.load(Ordering::Acquire) >= 2);

        // active_page should now point to the new page (same address)
        let active = s.active_page.load_full();
        let active_ptr = Arc::as_ptr(&active) as *const Page<u32> as *mut Page<u32>;
        assert_eq!(active_ptr, next_ptr, "active_page should be the newly linked page");
    }

    #[test]
    fn multi_writer_correctness_no_gaps_no_dups() {
        // Goal: Spawn multiple writers, append concurrently, ensure total count and no gaps per page
        let writers = 8usize;
        let per = 200usize;
        let total = writers * per;

        let s: Arc<SegmentedStream<u64>> = Arc::new(SegmentedStream::new());

        let mut handles = Vec::new();
        for w in 0..writers {
            let s_cloned = Arc::clone(&s);
            handles.push(std::thread::spawn(move || {
                for i in 0..(per as u64) {
                    // Encode writer and sequence to enable exact multiset check
                    let v: u64 = ((w as u64) << 32) | i;
                    s_cloned.append(v).unwrap();
                }
            }));
        }
        for h in handles { let _ = h.join(); }

        // Read all items back
        let mut c = Cursor::new_at_head(&s);
        let mut items: Vec<u64> = Vec::with_capacity(total);
        while let Some(v) = c.next() { items.push(*v); }
        assert_eq!(items.len(), total);

        // Validate multiset equality
        let mut expected: Vec<u64> = Vec::with_capacity(total);
        for w in 0..writers { for i in 0..(per as u64) { expected.push(((w as u64) << 32) | i); } }
        items.sort_unstable();
        expected.sort_unstable();
        assert_eq!(items, expected);

        // Per-page invariants: claimed >= committed; full pages have committed == ENTRIES_PER_PAGE
        let head = s.active_page.load_full();
        let mut page_ref: &Page<u64> = &*head;
        loop {
            let claimed = page_ref.claimed.load(Ordering::Relaxed);
            let committed = page_ref.committed.load(Ordering::Acquire);
            assert!(claimed >= committed);
            if committed < ENTRIES_PER_PAGE as u32 {
                // Tail page may be partial; must be the last page
                assert!(page_ref.next.load(Ordering::Acquire).is_null());
                break;
            }
            let next_ptr = page_ref.next.load(Ordering::Acquire);
            if next_ptr.is_null() { break; }
            // SAFETY: pages are backed by leaked Arc pointers; deref is valid
            page_ref = unsafe { &*next_ptr };
        }
    }

    #[test]
    fn tail_behavior_and_resume_after_more_appends() {
        // Goal: Cursor at tail yields None, then after more appends returns items
        let s: SegmentedStream<u32> = SegmentedStream::new();
        s.append(1).unwrap();
        s.append(2).unwrap();
        let mut c = Cursor::new_at_head(&s);
        assert_eq!(c.next().copied(), Some(1));
        assert_eq!(c.next().copied(), Some(2));
        // At tail now
        assert!(c.next().is_none());
        // Append more and ensure the cursor can continue
        s.append(3).unwrap();
        s.append(4).unwrap();
        assert_eq!(c.next().copied(), Some(3));
        assert_eq!(c.next().copied(), Some(4));
    }

    #[test]
    fn batch_read_hops_to_next_page_on_full() {
        // Goal: next_batch returns full first page, then empty while hopping, then next page slice
        let s: SegmentedStream<u32> = SegmentedStream::new();
        let total = ENTRIES_PER_PAGE + 3;
        for i in 0..(total as u32) {
            s.append(i).unwrap();
        }
        let mut c = Cursor::new_at_head(&s);
        let b1 = c.next_batch();
        assert_eq!(b1.len(), ENTRIES_PER_PAGE);
        // Next call should perform hop and return empty slice
        let b2 = c.next_batch();
        assert!(b2.is_empty());
        // Third call should expose remaining items from the next page
        let b3 = c.next_batch();
        assert_eq!(b3.len(), 3);
    }

    #[test]
    fn pool_prefiller_fills_ring() {
        // Prefiller should fill up to capacity and allow pops
        let pool = StreamPagePool::<u32>::with_capacity(0).with_prefiller(8);
        let ready = pool.ready.as_ref().unwrap().clone();
        // wait until some pages are available
        let mut spins = 0;
        while ready.len() < 4 && spins < 100 {
            std::thread::sleep(Duration::from_millis(10));
            spins += 1;
        }
        assert!(ready.len() >= 1, "prefiller did not produce pages");
        // Pop a few and ensure they are valid
        for _ in 0..ready.len().min(3) {
            if let Some(ptr) = ready.pop_ptr() {
                // reconstruct Arc and let it drop
                let _arc = unsafe { Arc::from_raw(ptr) };
            }
        }
    }

    #[test]
    fn stream_uses_ready_ring_without_prefiller() {
        // Manually prefill ready ring and verify SegmentedStream consumes it on rollover
        let mut pool = StreamPagePool::<u32>::with_capacity(0);
        let ready = Arc::new(Ring::new(2));
        // prefill two pages
        let _ = ready.push_arc(Arc::from(Page::<u32>::new()));
        let _ = ready.push_arc(Arc::from(Page::<u32>::new()));
        // install ready ring into pool
        pool.ready = Some(ready.clone());

        let s = SegmentedStream::with_pool(pool);
        // Fill one page, then force rollover consuming one ready page
        for i in 0..(ENTRIES_PER_PAGE as u32) { s.append(i).unwrap(); }
        let before = ready.len();
        s.append(9999).unwrap(); // rollover
        // Give a tiny moment for pop to reflect
        std::thread::sleep(Duration::from_millis(1));
        let after = ready.len();
        assert!(before >= 1, "expected at least one ready page before rollover");
        assert!(after <= before.saturating_sub(1), "expected ready ring to be consumed by 1 on rollover (before={}, after={})", before, after);
    }

    #[test]
    #[ignore]
    fn active_page_update_only_by_cas_winner() {
        // Goal: Ensure only the thread that links next updates active_page
        unimplemented!("observe active_page pointer evolution under contention");
    }

    #[test]
    #[ignore]
    fn pool_reuse_resets_page_fields() {
        // Goal: When pool is enabled, recycled pages reset claimed/committed/next
        unimplemented!("enable pool, recycle pages, then reuse and assert resets");
    }
}