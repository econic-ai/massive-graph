use std::sync::atomic::{AtomicU64, Ordering};

pub struct DeltaBloom {
    bits: Vec<AtomicU64>,
    mask: usize,
    k: u32,
}

impl DeltaBloom {
    pub fn with_capacity(capacity: usize, fpr: f64) -> Self {
        let m = (capacity.max(1) as f64 * (f64::ln(fpr).abs() / (f64::ln(2.0).powi(2)))).ceil() as usize;
        let m = m.next_power_of_two().max(64);
        let k = ((m as f64 / capacity.max(1) as f64) * f64::ln(2.0)).round().max(1.0) as u32;
        Self { bits: (0..(m/64)).map(|_| AtomicU64::new(0)).collect(), mask: m - 1, k }
    }

    #[inline]
    fn mix64(mut x: u64) -> u64 {
        // SplitMix64 mix function for cheap independent stream
        x ^= x >> 30;
        x = x.wrapping_mul(0xbf58476d1ce4e5b9);
        x ^= x >> 27;
        x = x.wrapping_mul(0x94d049bb133111eb);
        x ^ (x >> 31)
    }

    #[inline]
    fn double_hashes_from_seed(&self, seed: u64) -> (u64, u64) {
        let h1 = seed;
        let h2 = Self::mix64(seed) | 1;
        (h1, h2)
    }

    /// Insert a pre-hashed key using fused hashing path (lock-free, relaxed).
    pub fn insert_prehashed(&self, seed: u64) {
        let (h1, h2) = self.double_hashes_from_seed(seed);
        for i in 0..self.k {
            let h = h1.wrapping_add((i as u64).wrapping_mul(h2));
            let bit = (h as usize) & self.mask;
            let idx = bit >> 6;
            let off = bit & 63;
            self.bits[idx].fetch_or(1u64 << off, Ordering::Relaxed);
        }
    }

    /// Check membership using a pre-hashed seed (fused hashing, relaxed loads).
    pub fn might_contain_prehashed(&self, seed: u64) -> bool {
        let (h1, h2) = self.double_hashes_from_seed(seed);
        for i in 0..self.k {
            let h = h1.wrapping_add((i as u64).wrapping_mul(h2));
            let bit = (h as usize) & self.mask;
            let idx = bit >> 6;
            let off = bit & 63;
            if (self.bits[idx].load(Ordering::Relaxed) & (1u64 << off)) == 0 { return false; }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::hash::{Hash, Hasher};
    #[test]
    fn basic_bloom() {
        let b = DeltaBloom::with_capacity(1024, 0.01);
        let mut h = ahash::AHasher::default();
        123u64.hash(&mut h);
        let seed123 = h.finish();
        b.insert_prehashed(seed123);
        assert!(b.might_contain_prehashed(seed123));
        let mut h2 = ahash::AHasher::default();
        999u64.hash(&mut h2);
        assert!(!b.might_contain_prehashed(h2.finish()));
    }
}


