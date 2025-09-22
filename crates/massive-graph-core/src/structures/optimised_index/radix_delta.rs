use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, AtomicU8, AtomicUsize, Ordering};
use std::cell::UnsafeCell;
use std::ptr;
use crate::structures::optimised_index::arena::Arena;

#[inline]
fn fp8<K: Hash>(k: &K) -> u8 {
    let mut h = ahash::AHasher::default();
    k.hash(&mut h);
    (h.finish() & 0xFF) as u8
}

#[inline]
fn fp8_from_seed(seed: u64) -> u8 { (seed & 0xFF) as u8 }

#[inline]
fn hash_u64<K: Hash>(k: &K) -> u64 {
    let mut h = ahash::AHasher::default();
    k.hash(&mut h);
    h.finish()
}

const FP_BUSY: u8 = 0xFF; // reserved busy marker

struct Bucket<K: Clone, V> {
    // Keys: written only while ctrl == FP_BUSY
    keys: Vec<UnsafeCell<Option<K>>>,
    // Values: raw pointers managed by value_arena; null = empty/tombstone
    vals: Vec<AtomicPtr<V>>,           
    // Control bytes: 0 empty, FP_BUSY in-progress, else fingerprint (1..=254)
    ctrl: Vec<AtomicU8>,
    cap: usize,
    len: AtomicUsize,
}

impl<K: Clone, V: Clone> Bucket<K, V> {
    fn with_capacity(cap: usize) -> Self {
        let mut keys = Vec::with_capacity(cap);
        let mut vals = Vec::with_capacity(cap);
        let mut ctrl = Vec::with_capacity(cap);
        for _ in 0..cap {
            keys.push(UnsafeCell::new(None));
            vals.push(AtomicPtr::new(ptr::null_mut()));
            ctrl.push(AtomicU8::new(0));
        }
        Self { keys, vals, ctrl, cap, len: AtomicUsize::new(0) }
    }
}

/// Two-level radix (directory) with immutable buckets swapped atomically (COW per write)
pub struct RadixDelta<K: Clone, V> {
    dir: Vec<AtomicPtr<Bucket<K, V>>>,
    dir_mask: usize,
    bucket_cap: usize,
    arena: Arc<Arena<Bucket<K, V>>>,
    value_arena: Arc<Arena<V>>,
}

/// Borrowed-state result for delta lookups.
pub enum DeltaState<'a, T> { Hit(&'a T), Tombstone, Miss }

impl<K: Clone, V: Clone> RadixDelta<K, V>
where
    K: Eq + Hash + Clone,
{
    pub fn with_params(dir_bits: usize, bucket_cap: usize) -> Self {
        let dir_len = 1usize << dir_bits.min(20).max(1);
        let mut dir = Vec::with_capacity(dir_len);
        let arena = Arc::new(Arena::new());
        let value_arena = Arc::new(Arena::new());
        let cap = bucket_cap.max(8).next_power_of_two();
        for _ in 0..dir_len {
            let ptr = arena.alloc_new(Bucket::with_capacity(cap));
            dir.push(AtomicPtr::new(ptr));
        }
        Self { dir, dir_mask: dir_len - 1, bucket_cap: cap, arena, value_arena }
    }

    pub fn new() -> Self { Self::with_params(12, 32) }

    #[inline(always)]
    fn bucket_index(&self, k: &K) -> usize { (hash_u64(k) as usize) & self.dir_mask }

    #[inline(always)]
    fn bucket_index_from_hash(&self, h: u64) -> usize { (h as usize) & self.dir_mask }

    /// Upsert in-place using per-slot atomics; avoids cloning the entire bucket.
    pub fn upsert(&self, key: K, val: V) {
        let seed = hash_u64(&key);
        let bidx = self.bucket_index_from_hash(seed);
        let head = self.dir[bidx].load(Ordering::Acquire);
        let b = unsafe { &*head };
        let cap = b.cap;
        let mut fp = fp8_from_seed(seed);
        if fp == 0 { fp = 1; }
        if fp == FP_BUSY { fp = 0xFE; }
        let mut idx = (seed as usize) & (cap - 1);
        let mut dist = 0usize;
        loop {
            let slot = &b.ctrl[idx];
            let cur = slot.load(Ordering::Acquire);
            if cur == 0 {
                if slot.compare_exchange(0, FP_BUSY, Ordering::AcqRel, Ordering::Relaxed).is_ok() {
                    unsafe { *b.keys[idx].get() = Some(key.clone()); }
                    let p = self.value_arena.alloc_new(val);
                    b.vals[idx].store(p, Ordering::Release);
                    slot.store(fp, Ordering::Release);
                    b.len.fetch_add(1, Ordering::Relaxed);
                    return;
                } else {
                    continue;
                }
            }
            if cur != FP_BUSY && cur == fp {
                let kref = unsafe { &*b.keys[idx].get() };
                if let Some(k) = kref.as_ref() { if k == &key {
                    let newp = self.value_arena.alloc_new(val);
                    let old = b.vals[idx].swap(newp, Ordering::AcqRel);
                    if !old.is_null() { self.value_arena.retire(old); }
                    return;
                } }
            }
            idx = (idx + 1) & (cap - 1);
            dist += 1;
            if dist > cap { return; }
        }
    }

    /// Tombstone (delete) by nulling pointer; keep key/ctrl to retain OA continuity.
    pub fn delete(&self, key: &K) {
        let seed = hash_u64(key);
        let bidx = self.bucket_index_from_hash(seed);
        let head = self.dir[bidx].load(Ordering::Acquire);
        let b = unsafe { &*head };
        let cap = b.cap;
        let mut fp = fp8_from_seed(seed);
        if fp == 0 { fp = 1; }
        if fp == FP_BUSY { fp = 0xFE; }
        let mut idx = (seed as usize) & (cap - 1);
        let mut dist = 0usize;
        while dist < cap {
            let slot_fp = b.ctrl[idx].load(Ordering::Acquire);
            if slot_fp == 0 { return; }
            if slot_fp != FP_BUSY && slot_fp == fp {
                let kref = unsafe { &*b.keys[idx].get() };
                if let Some(k) = kref.as_ref() { if k == key {
                    let old = b.vals[idx].swap(ptr::null_mut(), Ordering::AcqRel);
                    if !old.is_null() { self.value_arena.retire(old); }
                    return;
                }}
            }
            idx = (idx + 1) & (cap - 1);
            dist += 1;
        }
    }

    /// Get overlay value if present (compat path cloning V to Arc<V>); None indicates tombstone.
    pub fn get(&self, key: &K) -> Option<Option<Arc<V>>> {
        let seed = hash_u64(key);
        match self.get_ref_hashed(key, seed) {
            DeltaState::Hit(v) => Some(Some(Arc::new(v.clone()))),
            DeltaState::Tombstone => Some(None),
            DeltaState::Miss => None,
        }
    }

    /// Get overlay using a precomputed 64-bit hash seed; avoids re-hashing the key.
    pub fn get_hashed(&self, key: &K, seed: u64) -> Option<Option<Arc<V>>> {
        match self.get_ref_hashed(key, seed) {
            DeltaState::Hit(v) => Some(Some(Arc::new(v.clone()))),
            DeltaState::Tombstone => Some(None),
            DeltaState::Miss => None,
        }
    }

    /// Borrowed get (no clone/Arc) using key.
    #[inline(always)]
    pub fn get_ref(&self, key: &K) -> DeltaState<V> {
        let seed = hash_u64(key);
        self.get_ref_hashed(key, seed)
    }

    /// Borrowed get (no clone/Arc) with precomputed seed.
    #[inline(always)]
    pub fn get_ref_hashed(&self, key: &K, seed: u64) -> DeltaState<V> {
        let bidx = self.bucket_index_from_hash(seed);
        let bptr = self.dir[bidx].load(Ordering::Acquire);
        let b = unsafe { &*bptr };
        let cap = b.cap;
        let mut fp = fp8_from_seed(seed);
        if fp == 0 { fp = 1; }
        if fp == FP_BUSY { fp = 0xFE; }
        let mut idx = (seed as usize) & (cap - 1);
        let mut dist = 0usize;
        while dist < cap {
            let slot_fp = b.ctrl[idx].load(Ordering::Acquire);
            if slot_fp == 0 { return DeltaState::Miss; }
            if slot_fp != FP_BUSY && slot_fp == fp {
                let kref = unsafe { &*b.keys[idx].get() };
                if let Some(k) = kref.as_ref() { if k == key {
                    let vptr = b.vals[idx].load(Ordering::Acquire);
                    if vptr.is_null() { return DeltaState::Tombstone; }
                    let vref: &V = unsafe { &*vptr };
                    return DeltaState::Hit(vref);
                } }
            }
            idx = (idx + 1) & (cap - 1);
            dist += 1;
        }
        DeltaState::Miss
    }

    /// Clear all overlay entries (bench/testing aid).
    pub fn clear_all(&self) {
        for aptr in &self.dir {
            let prev = aptr.swap(self.arena.alloc_new(Bucket::with_capacity(self.bucket_cap)), Ordering::AcqRel);
            if !prev.is_null() {
                let b = unsafe { &*prev };
                for v in &b.vals {
                    let p = v.load(Ordering::Acquire);
                    if !p.is_null() { self.value_arena.retire(p); }
                }
            }
            self.arena.retire(prev);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_upsert_get_delete() {
        let d: RadixDelta<u64, u64> = RadixDelta::new();
        assert!(matches!(d.get_ref(&1), DeltaState::Miss));
        d.upsert(1, 10);
        match d.get_ref(&1) { DeltaState::Hit(&v) => assert_eq!(v, 10), _ => panic!("expected hit"), }
        d.delete(&1);
        assert!(matches!(d.get_ref(&1), DeltaState::Tombstone));
    }
}


