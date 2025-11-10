//! Minimal Perfect Hash (MPH) indexer using a BBHash-style multi-level construction.
//! This file is self-contained and compiles (no unmatched braces).
//!
//! Goals:
//! - Deterministic build over a fixed key set S
//! - `eval(&K) -> usize` with O(1) time, no allocs/locks
//! - Bits/key on the order of a few (not micro-optimized here)
//! - No external crates beyond `ahash`

use ahash::AHasher;
use std::hash::Hasher;

/// Public trait from your codebase - pure compile-time polymorphism, no dynamic dispatch.
pub trait MphIndexer<K: Clone>: Send + Sync + Clone {
    /// Evaluate the hash function for a key, returning a slot index.
    fn eval(&self, key: &K) -> usize;
    
    /// Build a new indexer from a key set (used during publish to rebuild for changed keys).
    /// Returns a new instance of the same indexer type (monomorphized at compile time).
    fn build(keys: &[K]) -> Self where K: core::hash::Hash, Self: Sized;
}

/// BBHash configuration
#[derive(Clone, Copy)]
pub struct BbhConfig {
    /// oversizing factor per level (>=1.0). Typical 1.1 .. 2.0
    pub gamma: f64,
    /// maximum number of levels (safety valve)
    pub max_levels: usize,
}
impl Default for BbhConfig {
    fn default() -> Self { Self { gamma: 1.3, max_levels: 32 } }
}

#[derive(Clone)]
struct Level {
    seed: u64,
    m: usize,               // bins in this level
    off: usize,             // flat offset within concatenated levels
    uniq: Vec<u64>,         // bitset: bins hit by exactly 1 key
}

#[inline]
fn bitset(len_bits: usize) -> Vec<u64> { vec![0u64; (len_bits + 63) >> 6] }
#[inline]
fn bs_set(bs: &mut [u64], idx: usize) { bs[idx >> 6] |= 1u64 << (idx & 63); }
#[inline]
fn bs_get(bs: &[u64], idx: usize) -> bool { (bs[idx >> 6] >> (idx & 63)) & 1 == 1 }

#[inline]
fn hash64<K: core::hash::Hash>(k: &K, salt: u64) -> u64 {
    let mut h = AHasher::default();
    h.write_u64(salt);
    k.hash(&mut h);
    h.finish()
}

/// Minimal Perfect Hash indexer (BBHash-style multi-level peeling)
#[derive(Clone)]
pub struct BBHashIndexer<K> {
    n: usize,
    levels: Vec<Level>,
    trans: Vec<usize>,          // flat (level.off + bin) -> dense [0..n)
    _pk: core::marker::PhantomData<K>,
}

impl<K: core::hash::Hash + Send + Sync> BBHashIndexer<K> {
    /// Build from a set of unique keys.
    pub fn build(keys: &[K], cfg: BbhConfig) -> Self {
        assert!(!keys.is_empty());
        let n = keys.len();
        let mut remaining: Vec<usize> = (0..n).collect();
        let mut levels: Vec<Level> = Vec::new();
        let mut off = 0usize;
        let mut salt = 0x9E37_79B9_7F4A_7C15u64;

        while !remaining.is_empty() {
            // choose bins for this level
            let m = ((remaining.len() as f64) * cfg.gamma).ceil() as usize;
            let mut occ = bitset(m);
            let mut uniq = bitset(m);

            // count hits per bin via (occ, uniq) trick
            for &ix in &remaining {
                let h = hash64(&keys[ix], salt);
                let b = (h as usize) % m;
                if !bs_get(&occ, b) {
                    bs_set(&mut occ, b);      // first hit
                    bs_set(&mut uniq, b);     // currently unique
                } else {
                    // second or more hit clears uniq
                    let w = b >> 6; let bit = 1u64 << (b & 63);
                    uniq[w] &= !bit;
                }
            }

            // collect singletons
            let mut singles: Vec<usize> = Vec::new();
            for &ix in &remaining {
                let h = hash64(&keys[ix], salt);
                let b = (h as usize) % m;
                if bs_get(&uniq, b) { singles.push(ix); }
            }

            if singles.is_empty() {
                // reseed and retry this level
                salt = salt.rotate_left(17) ^ 0xD1B5_4A32_D192_ED03u64;
                if levels.len() >= cfg.max_levels { panic!("BBHash build failed: too many levels"); }
                continue;
            }

            // finalize level (store uniq only)
            levels.push(Level { seed: salt, m, off, uniq });

            // remove singles from remaining
            let last = levels.last().unwrap();
            let mut next: Vec<usize> = Vec::with_capacity(remaining.len() - singles.len());
            for ix in remaining.into_iter() {
                let h = hash64(&keys[ix], last.seed);
                let b = (h as usize) % last.m;
                if !bs_get(&last.uniq, b) { next.push(ix); }
            }
            remaining = next;
            off += m;
            salt = salt.rotate_left(21) ^ 0xA24B_1C66_3CF1_357Du64;
        }

        // compact to exactly n outputs: build translation from flat space to [0..n)
        let total_m: usize = levels.iter().map(|l| l.m).sum();
        let mut trans = vec![usize::MAX; total_m];
        let mut dense = 0usize;
        'outer: for lvl in &levels {
            for b in 0..lvl.m {
                if bs_get(&lvl.uniq, b) {
                    let flat = lvl.off + b;
                    trans[flat] = dense;
                    dense += 1;
                    if dense == n { break 'outer; }
                }
            }
        }
        assert_eq!(dense, n, "BBHash: failed to assign n outputs");

        Self { n, levels, trans, _pk: core::marker::PhantomData }
    }
}

impl<K: Clone + core::hash::Hash + Send + Sync + 'static> MphIndexer<K> for BBHashIndexer<K> {
    #[inline]
    fn eval(&self, key: &K) -> usize {
        for lvl in &self.levels {
            let h = hash64(key, lvl.seed);
            let b = (h as usize) % lvl.m;
            if bs_get(&lvl.uniq, b) {
                return self.trans[lvl.off + b];
            }
        }
        // For non-members, map deterministically into [0, n)
        // This ensures eval is always in-bounds and stable across runs
        let last = self.levels.last().unwrap();
        (hash64(key, last.seed) as usize) % self.n
    }
    
    fn build(keys: &[K]) -> Self {
        BBHashIndexer::build(keys, Default::default())
    }
}

// === Example: integrate during snapshot build ===
// let mph = BBHashIndexer::<K>::build(&base_keys, Default::default());
// let indexer = ArcIndexer(Arc::new(mph));
// let idx = indexer.eval(&base_keys[i]); // 0..n-1