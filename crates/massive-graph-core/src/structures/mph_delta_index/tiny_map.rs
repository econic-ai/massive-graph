/// Immutable read-only snapshot for ultra-fast lock-free lookups and iteration.
/// Memory is arena-allocated as part of [Buffer | Recs | TinyMap] colocated structure.
/// Uses raw pointers into arena memory - no Box ownership.
#[repr(C)]
pub struct ReadTinyMap {
    /// Number of active entries
    len: usize,
    /// Points to tags array (sorted u8 fingerprints)
    tags: *const u8,
    /// Points to slots array (u16, properly aligned)
    slots: *const u16,
    /// Points to active_slots array (u16, sorted by slot index)
    active_slots: *const u16,
}

unsafe impl Send for ReadTinyMap {}
unsafe impl Sync for ReadTinyMap {}

impl ReadTinyMap {
    /// Get the number of entries in the map.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }
    
    /// Build TinyMap directly into provided arena memory.
    /// Input: base_ptr points to arena-allocated memory, items contain entries to store
    /// 
    /// Memory layout: [tags(u8*len) | padding | slots(u16*len) | padding | active_slots(u16*len)]
    /// 
    /// Returns ReadTinyMap with pointers into the arena allocation.
    pub fn new_in_place(base_ptr: *mut u8, items: Vec<(u8, u64, usize, u8)>) -> Self {
        // Filter tombstones - only keep upserts
        let mut valid: Vec<_> = items.into_iter()
            .filter(|(_, _, _, kind)| *kind == 0)
            .collect();
        
        let len = valid.len();
        
        if len == 0 {
            return ReadTinyMap {
                len: 0,
                tags: core::ptr::null(),
                slots: core::ptr::null(),
                active_slots: core::ptr::null(),
            };
        }
        
        // Sort by tag8 for linear scan
        valid.sort_unstable_by_key(|(tag, _, _, _)| *tag);
        
        unsafe {
            // Write tags at base_ptr
            let tags_ptr = base_ptr;
            for (i, (tag, _, _, _)) in valid.iter().enumerate() {
                *tags_ptr.add(i) = *tag;
            }
            
            // Write slots (u16-aligned)
            let slots_offset = super::arena::align_up(len, 2);
            let slots_ptr = base_ptr.add(slots_offset) as *mut u16;
            for (i, (_, _, slot_idx, _)) in valid.iter().enumerate() {
                *slots_ptr.add(i) = *slot_idx as u16;
            }
            
            // Write active_slots (sorted by slot index)
            let mut active: Vec<u16> = valid.iter().map(|(_, _, s, _)| *s as u16).collect();
            active.sort_unstable();
            let active_offset = slots_offset + (len * 2);
            let active_offset_aligned = super::arena::align_up(active_offset, 2);
            let active_ptr = base_ptr.add(active_offset_aligned) as *mut u16;
            for (i, &slot) in active.iter().enumerate() {
                *active_ptr.add(i) = slot;
            }
            
            ReadTinyMap {
                len,
                tags: tags_ptr,
                slots: slots_ptr,
                active_slots: active_ptr,
            }
        }
    }
    
    /// Calculate the total size needed for a TinyMap with `len` entries.
    /// Used by arena allocator to determine allocation size.
    pub fn calculate_size(len: usize) -> usize {
        if len == 0 {
            return 0;
        }
        let tags_size = len;
        let slots_offset = super::arena::align_up(tags_size, 2);
        let active_offset = slots_offset + (len * 2);
        let active_offset_aligned = super::arena::align_up(active_offset, 2);
        active_offset_aligned + (len * 2)
    }

    /// Get count of entries.
    #[inline]
    pub fn count(&self) -> usize {
        self.len
    }
    
    /// Get active slots sorted by slot index for sequential iteration.
    #[inline]
    pub fn slots(&self) -> &[u16] {
        if self.len == 0 {
            &[]
        } else {
            unsafe { core::slice::from_raw_parts(self.active_slots, self.len) }
        }
    }
    
    /// Get direct access to tags array (sorted 8-bit fingerprints).
    #[inline]
    fn tags(&self) -> &[u8] {
        if self.len == 0 {
            &[]
        } else {
            unsafe { core::slice::from_raw_parts(self.tags, self.len) }
        }
    }
    
    /// Get direct access to slot indices (permuted to match tags).
    #[inline]
    fn slot_indices(&self) -> &[u16] {
        if self.len == 0 {
            &[]
        } else {
            unsafe { core::slice::from_raw_parts(self.slots, self.len) }
        }
    }
    
    /// Iterate over all buffer slot indices (standard iterator, no filtering).
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = u16> + '_ {
        self.slots().iter().copied()
    }
    
    /// Iterate over buffer slot indices matching a specific 8-bit tag using binary search.
    /// Since tags are sorted, this finds the range of matching tags efficiently.
    /// 8-bit tags provide 2x better cache efficiency vs 16-bit.
    #[inline]
    pub fn iter_tags(&self, tag8: u8) -> impl Iterator<Item = u16> + '_ {
        let tags = self.tags();
        let slots = self.slot_indices();
        
        // Binary search to find range of matching tags
        let start = tags.partition_point(|&t| t < tag8);
        
        // Check if tag exists
        let (start, end) = if start >= tags.len() || tags[start] != tag8 {
            // Tag not found - empty range
            (0, 0)
        } else {
            // Find end of matching range
            let end = tags[start..].partition_point(|&t| t == tag8) + start;
            (start, end)
        };
        
        // Yield corresponding slots
        (start..end).map(move |i| slots[i])
    }
    
    /// Iterate over buffer slot indices matching a specific 8-bit tag (linear scan fallback).
    /// Used for benchmarking comparison with binary search.
    /// 8-bit tags allow 2x SIMD width: 16x u8 vs 8x u16 per NEON instruction.
    #[inline]
    pub fn iter_tags_linear(&self, tag8: u8) -> TagMatchIter<'_> {
        TagMatchIter {
            tags: self.tags(),
            slots: self.slot_indices(),
            target_tag: tag8,
            pos: 0,
        }
    }
}

// Note: ReadTinyMap does NOT own its memory - it points into arena allocations.
// The arena is responsible for cleanup via epoch-based retirement.
// When the containing Buffer is retired, the entire [Buffer | Recs | TinyMap] allocation is freed.

/// Zero-allocation iterator over buffer slot indices matching a specific 8-bit tag.
/// Filters during iteration for optimal performance. Handles tag8 collisions correctly.
pub struct TagMatchIter<'a> {
    tags: &'a [u8],
    slots: &'a [u16],
    target_tag: u8,
    pos: usize,
}

impl<'a> Iterator for TagMatchIter<'a> {
    type Item = u16;
    
    #[inline]
    fn next(&mut self) -> Option<u16> {
        // Linear scan with SIMD auto-vectorization (16x u8 per instruction on ARM64)
        while self.pos < self.tags.len() {
            let i = self.pos;
            self.pos += 1;
            if self.tags[i] == self.target_tag {
                return Some(self.slots[i]);
            }
        }
        None
    }
}

