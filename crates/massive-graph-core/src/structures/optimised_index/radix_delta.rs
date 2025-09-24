use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, AtomicU16, AtomicUsize, Ordering};
use std::cell::UnsafeCell;
use std::mem::{MaybeUninit, size_of};
use std::ptr;
use crate::structures::optimised_index::arena::Arena;
use crate::types::ID16;

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

// Slot state encoded in high 8 bits of meta; fingerprint in low 8 bits
const STATE_NONE: u8 = 0x00;
const STATE_BUSY: u8 = 0x01;
const STATE_PTR: u8 = 0x02;
const STATE_INLINE: u8 = 0x03;
const STATE_TOMBSTONE: u8 = 0x04;

#[repr(align(64))]
struct Slot<K: Clone, V> {
    key: UnsafeCell<Option<K>>,        // None until published
    val_ptr: AtomicPtr<V>,             // used when state == PTR
    inline: UnsafeCell<MaybeUninit<V>>, // used when state == INLINE
    meta: AtomicU16,                   // [state:8 | fp:8]
}

impl<K: Clone, V> Slot<K, V> {
    #[inline]
    fn load_meta(&self, ord: Ordering) -> u16 { self.meta.load(ord) }
    #[inline]
    fn meta_state(meta: u16) -> u8 { (meta >> 8) as u8 }
    #[inline]
    fn meta_fp(meta: u16) -> u8 { (meta & 0xFF) as u8 }
    #[inline]
    fn pack_meta(state: u8, fp: u8) -> u16 { ((state as u16) << 8) | (fp as u16) }
}

struct Bucket<K: Clone, V> {
    slots: Vec<Slot<K, V>>,            // Array-of-Structs for locality
    cap: usize,
    len: AtomicUsize,
}

impl<K: Clone, V: Clone> Bucket<K, V> {
    fn with_capacity(cap: usize) -> Self {
        let mut slots = Vec::with_capacity(cap);
        for _ in 0..cap {
            slots.push(Slot {
                key: UnsafeCell::new(None),
                val_ptr: AtomicPtr::new(ptr::null_mut()),
                inline: UnsafeCell::new(MaybeUninit::uninit()),
                meta: AtomicU16::new(Slot::<K, V>::pack_meta(STATE_NONE, 0)),
            });
        }
        Self { slots, cap, len: AtomicUsize::new(0) }
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

    pub fn new() -> Self { Self::with_params(16, 32) }

    #[inline(always)]
    fn dir_bits(&self) -> usize { (self.dir_mask + 1).trailing_zeros() as usize }

    #[inline(always)]
    fn bucket_index_from_hash(&self, h: u64) -> usize {
        let bits = self.dir_bits().min(64);
        let hi = if bits == 0 { 0 } else { (h >> (64 - bits)) as usize };
        hi & self.dir_mask
    }

    /// Upsert: lock-free linear probing; claim empty slot or update in place.
    pub fn upsert(&self, key: K, val: V) {
        let seed = hash_u64(&key);
        let bidx = self.bucket_index_from_hash(seed);
        let head = self.dir[bidx].load(Ordering::Acquire);
        let b = unsafe { &*head };
        let cap = b.cap;
        let mut fp = fp8_from_seed(seed); if fp == 0 { fp = 1; }
        let mut idx = (seed as usize) & (cap - 1);
        let mut dist = 0usize;
        loop {
            let slot = &b.slots[idx];
            let meta = slot.load_meta(Ordering::Acquire);
            let state = Slot::<K, V>::meta_state(meta);
            let sfp = Slot::<K, V>::meta_fp(meta);
            if state == STATE_NONE || state == STATE_TOMBSTONE {
                // Claim by moving to BUSY if still in the same observed state
                if slot
                    .meta
                    .compare_exchange(meta, Slot::<K, V>::pack_meta(STATE_BUSY, 0), Ordering::AcqRel, Ordering::Relaxed)
                    .is_ok()
                {
                    unsafe { *slot.key.get() = Some(key.clone()); }
                    if size_of::<V>() <= 24 {
                        unsafe { (*slot.inline.get()).write(val); }
                        slot.meta
                            .store(Slot::<K, V>::pack_meta(STATE_INLINE, fp), Ordering::Release);
                    } else {
                        let p = self.value_arena.alloc_new(val);
                        slot.val_ptr.store(p, Ordering::Release);
                        slot.meta
                            .store(Slot::<K, V>::pack_meta(STATE_PTR, fp), Ordering::Release);
                    }
                    b.len.fetch_add(1, Ordering::Relaxed);
                    return;
                } else {
                    continue;
                }
            } else if (state == STATE_PTR || state == STATE_INLINE) && sfp == fp {
                let kref = unsafe { &*slot.key.get() };
                if let Some(k) = kref.as_ref() {
                    if k == &key {
                        if state == STATE_INLINE {
                            unsafe { (*slot.inline.get()).write(val); }
                        } else {
                            let newp = self.value_arena.alloc_new(val);
                            let old = slot.val_ptr.swap(newp, Ordering::AcqRel);
                            if !old.is_null() {
                                self.value_arena.retire(old);
                            }
                        }
                        return;
                    }
                }
            }
            idx = (idx + 1) & (cap - 1);
            dist += 1;
            if dist > cap { break; }
        }
        // If we arrive here, table/bucket is effectively full
        return;
    }

    /// Tombstone (delete) by nulling pointer; keep key/ctrl to retain OA continuity.
    pub fn delete(&self, key: &K) {
        let seed = hash_u64(key);
        let bidx = self.bucket_index_from_hash(seed);
        let head = self.dir[bidx].load(Ordering::Acquire);
        let b = unsafe { &*head };
        let cap = b.cap;
        let mut fp = fp8_from_seed(seed); if fp == 0 { fp = 1; }
        let mut idx = (seed as usize) & (cap - 1);
        let mut dist = 0usize;
        while dist < cap {
            let slot = &b.slots[idx];
            let meta = slot.load_meta(Ordering::Acquire);
            let state = Slot::<K, V>::meta_state(meta);
            let sfp = Slot::<K, V>::meta_fp(meta);
            if state == STATE_NONE { return; }
            if (state == STATE_PTR || state == STATE_INLINE) && sfp == fp {
                let kref = unsafe { &*slot.key.get() };
                if let Some(k) = kref.as_ref() {
                    if k == key {
                        if state == STATE_PTR {
                            let old = slot.val_ptr.swap(ptr::null_mut(), Ordering::AcqRel);
                            if !old.is_null() { self.value_arena.retire(old); }
                        }
                        slot.meta
                            .store(Slot::<K, V>::pack_meta(STATE_TOMBSTONE, sfp), Ordering::Release);
                        return;
                    }
                }
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
        let mut fp = fp8_from_seed(seed); if fp == 0 { fp = 1; }
        let mut idx = (seed as usize) & (cap - 1);
        let mut dist = 0usize;
        while dist < cap {
            let slot = &b.slots[idx];
            let meta = slot.load_meta(Ordering::Acquire);
            let state = Slot::<K, V>::meta_state(meta);
            let sfp = Slot::<K, V>::meta_fp(meta);
            if state == STATE_NONE { return DeltaState::Miss; }
            if (state == STATE_PTR || state == STATE_INLINE) && sfp == fp {
                let kref = unsafe { &*slot.key.get() };
                if let Some(k) = kref.as_ref() {
                    if k == key {
                        if state == STATE_INLINE {
                            let vref: &V = unsafe { &*(*slot.inline.get()).as_ptr() };
                            return DeltaState::Hit(vref);
                        } else {
                            let vptr = slot.val_ptr.load(Ordering::Acquire);
                            if vptr.is_null() { return DeltaState::Tombstone; }
                            let vref: &V = unsafe { &*vptr };
                            return DeltaState::Hit(vref);
                        }
                    }
                }
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
                for slot in &b.slots {
                    let p = slot.val_ptr.load(Ordering::Acquire);
                    if !p.is_null() { self.value_arena.retire(p); }
                }
            }
            self.arena.retire(prev);
        }
    }
}

// Removed ID16 specialization; unified high-bit directory via bucket_index_from_hash
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


