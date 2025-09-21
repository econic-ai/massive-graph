# Segmented Array Stream — Implementation Instructions

**You are implementing a lock-free(ish) segmented append-only array with multi-writer append and multi-cursor reads.** The design is generic over `T` (e.g., `ChunkRef<C>`); no lifetimes required in the core types. Only fill in implementations; the structs already exist.

> File location: `massive-graph-core/src/structures/segmented_stream.rs` (or similar)

---

## 1) Context & Goals

* Provide an **append-only** segmented array (“stream”) that stores items in fixed-size **pages** (`PAGE_SIZE` defined in crate constants).
* **Many writers** may append concurrently; **many readers/cursors** scan concurrently from various positions.
* **Reads never block** and observe only fully-published entries.
* **No epochs** and **no drop** logic for `T` (assume trivial drop or externally managed lifetimes). Use `MaybeUninit<T>` inside pages.
* Page memory may grow unbounded for now; pool reuse is optional but preferred.

---

## 2) Structures (already defined)

* `Stream<T>` with:

  * `pages: ArcSwap<Vec<Page<T>>>` (optional global list; use only if needed),
  * `active_page: AtomicPtr<Page<T>>` (current append target),
  * `sequence: AtomicU32` (can be unused initially),
  * `pool: Option<PagePool<T>>` (ring-style page pool).
* `Page<T>` with `entries: [MaybeUninit<T>; PAGE_SIZE]`, `claimed`, `committed`, `next`.
* `Cursor<T>` with `page: AtomicPtr<Page<T>>`, `index: AtomicU32` (shape only; can expose user cursors as light wrappers).
* `PagePool<T>` ring buffer (bounded MPMC) — implement minimal push/pop.

**Do not change public shapes.**

---

## 3) Invariants & Memory Ordering (critical)

* **Slot publication:** writer writes `entries[i]` then increments `committed` with **Release**. Readers load `committed` with **Acquire** and only read `entries[..committed]`.
* **Slot reservation:** writers `i = claimed.fetch_add(1, Relaxed)` to get a slot; if `i >= PAGE_SIZE`, switch to next page.
* **Page linking:** initialize new page fully (`claimed=0, committed=0, next=null`) then link via `curr.next.store(new, Release)` and `active_page.store(new, Release)`.
* Readers traverse `next` via **Acquire** loads.

---

## 4) Append Path (multi-writer)

1. Load `active_page` (Acquire).
2. Try `i = claimed.fetch_add(1, Relaxed)`; if `i < PAGE_SIZE`:

   * Write `entries[i]` (use `ptr::write`/`MaybeUninit::write`).
   * `committed.fetch_add(1, Release)`; return handle.
3. Else (page full):

   * Allocate or pop a page from `PagePool` (if `Some`).
   * Initialize it; link it from the old page with `next.store(new, Release)` (only **one** writer should succeed in linking; others detect `next` non-null and reuse it).
   * `active_page.store(new, Release)`.
   * Retry append on the new page.

> Use a **CAS** on `curr.next` to ensure a single link (first writer wins). Others read `next` and proceed. Keep spinning to the linked page; no locks.

---

## 5) Read Path (cursors)

* A cursor keeps `(page_ptr, idx)` locally (no atomics needed inside the reader loop; the struct uses atomics but readers may copy to locals).
* Steps:

  1. `cap = page.committed.load(Acquire)`; if `idx < cap`, read `&entries[idx]` (assume-init ref) and advance.
  2. If `idx == cap` and `cap < PAGE_SIZE`, the reader is **caught up** at tail; return `None` / yield.
  3. If `cap == PAGE_SIZE`, hop to `next = page.next.load(Acquire)`; if null, yield; else switch `page_ptr = next`, set `idx = 0`, continue.
* Provide a **batch** read helper that returns `&[T]` from `idx..cap` for the current page to minimize per-item overhead.

---

## 6) Page Pool (ring) — minimal MPMC

Implement a bounded ring with power-of-two `capacity`:

* `tail` is producer index, `head` is consumer index.
* `push(page_ptr)`: reserve `t = tail.fetch_add(1, Relaxed)`, slot = `t & (cap-1)`; CAS slot from null → page; on full, either drop or back off (keep simple).
* `pop()`: reserve `h = head.fetch_add(1, Relaxed)`, slot = `h & (cap-1)`; swap out non-null page or return null if empty.
* Zero the page state (`claimed=0, committed=0, next=null`) before reuse. (No drop of entries required.)

If ring complexity is high, stub with `crossbeam_queue::SegQueue<*mut Page<T>>` (feature-flag) but keep the ring API.

---

## 7) Minimal Public API to Expose

* `Stream::<T>::new(...) -> Self` (allocate first page; set `active_page`).
* `append(&self, item: T) -> Result<(), StreamError>` (multi-writer path above).
* `CursorHandle`:

  * `fn new_at_head(&self) -> CursorHandle<T>`
  * `fn next(&mut self) -> Option<&T>`
  * `fn next_batch(&mut self) -> &[T]`

> Keep API small and generic; return references with appropriate lifetimes obtained from page borrows (internally safe due to publication ordering).

---

## 8) Safety & Edge Cases

* Writers must never increment `committed` past `PAGE_SIZE`.
* Only set `next` **once** (use CAS). If another writer already linked a page, use that one.
* Readers may observe partially-filled last page; treat `cap < PAGE_SIZE` as tail.
* No global sequence, no EOF/closed logic in this pass.
* Assume `T: Send + Sync` and trivially droppable or externally managed; no per-entry drop on reuse.

---

## 9) Tests (what to write)

1. **Single-thread basics**

   * Append `N > PAGE_SIZE` items; verify page chaining and ordering.
   * Cursor from head: iterate all; `next_batch` returns contiguous slices per page.
2. **Multi-writer correctness**

   * Spawn `W` writer tasks each appending `M` items; verify total `W*M` items readable in order per-page (no gaps/dups within page; cross-page order per-writer may interleave, which is fine).
   * Ensure exactly one `next` link per boundary (use counters/flags to assert no double-link).
3. **Tail behavior**

   * Reader at tail sees `cap < PAGE_SIZE` and yields; after more appends, reader resumes and gets new items.
4. **Pool reuse (if enabled)**

   * Push/pop pages through `PagePool`; confirm fields reset and appends succeed.
5. **Memory ordering**

   * Fuzz test with heavy contention: asserts that readers never read uninitialized memory (e.g., fill entries with sentinel values prior to write and check visibility constraints).

> Use `loom` (optional) for a reduced model test of the publish protocol (claimed→write→committed; next linking). Provide conditional compilation if you include it.

---

## 10) Benchmarks (optional skeleton)

* Micro-bench: single-writer throughput (appends/sec) for varying `PAGE_SIZE`.
* Multi-writer throughput with/without pool reuse.
* Reader `next_batch` throughput vs `next()` item-by-item.

---

## 11) Deliverables & Acceptance

* `Stream<T>` appends concurrently without panics; readers never block and never observe uninitialized entries.
* Tests in `structures/segmented_stream_tests.rs` passing locally.
* Minimal docs explaining the Release/Acquire points and the single-link guarantee.

**Keep the implementation minimal and robust.** Prefer straightforward atomics over clever tricks. No extra features beyond this scope.
