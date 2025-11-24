use std::hash::{Hash, Hasher};

/// Mix function for tiny open-addressed maps and bucket indexing.
#[inline]
pub fn mix(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E3779B97F4A7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// Compute 64-bit hash for a key with ahash.
#[inline]
pub fn hash64<K: Hash>(k: &K) -> u64 {
    let mut h = ahash::AHasher::default();
    k.hash(&mut h);
    h.finish()
}

/// Low 8 bits of hash used as fp8 bucket index.
#[inline]
pub fn fp8_from_hash(h: u64) -> u8 { (h & 0xFF) as u8 }

/// Middle 16 bits used as tag for fast in-bucket filter.
#[inline]
pub fn tag16_from_hash(h: u64) -> u16 { ((h >> 8) & 0xFFFF) as u16 }

/// Derive an fpN bucket index from the low `n_bits` of the hash.
#[inline]
pub fn fpn_from_hash(h: u64, n_bits: u32) -> usize { (h as usize) & ((1usize << n_bits) - 1) }

/// Derive a 16-bit tag from bits disjoint with the low `n_bits` used for fpN.
#[inline]
pub fn tag16_from_hash_disjoint(h: u64, n_bits: u32) -> u16 { ((h >> n_bits) & 0xFFFF) as u16 }

/// Derive an 8-bit tag from bits disjoint with the low `n_bits` used for fpN.
/// More cache-efficient than tag16 for small buckets, with acceptable collision rate.
#[inline]
pub fn tag8_from_hash_disjoint(h: u64, n_bits: u32) -> u8 { ((h >> n_bits) & 0xFF) as u8 }

/// Derive preferred slot index from hash using bits disjoint with bucket_bits and tag bits.
/// Returns a slot index within [0, slot_count) for deterministic probe start position.
#[inline]
pub fn preferred_slot_from_hash(h: u64, bucket_bits: u32, slot_bits: u32) -> usize {
    let shift = bucket_bits + 8; // Skip bucket + tag bits
    ((h >> shift) & ((1u64 << slot_bits) - 1)) as usize
}


