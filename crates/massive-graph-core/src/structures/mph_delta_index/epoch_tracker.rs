/// Global tracker for epoch defer statistics
use std::sync::atomic::{AtomicUsize, Ordering};

/// Global counters for epoch defer tracking
pub struct EpochStats {
    /// Total defers registered (incremented when we call defer_*)
    defers_registered: AtomicUsize,
    /// Guards pinned (incremented on epoch::pin())
    guards_pinned: AtomicUsize,
    /// Flushes called (incremented on guard.flush())
    flushes_called: AtomicUsize,
    /// RadixIndex instances created
    radix_created: AtomicUsize,
    /// RadixIndex instances dropped
    radix_dropped: AtomicUsize,
}

impl EpochStats {
    pub const fn new() -> Self {
        Self {
            defers_registered: AtomicUsize::new(0),
            guards_pinned: AtomicUsize::new(0),
            flushes_called: AtomicUsize::new(0),
            radix_created: AtomicUsize::new(0),
            radix_dropped: AtomicUsize::new(0),
        }
    }

    #[inline]
    pub fn register_defer(&self) {
        self.defers_registered.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn register_pin(&self) {
        self.guards_pinned.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn register_flush(&self) {
        self.flushes_called.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn register_radix_create(&self) {
        self.radix_created.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn register_radix_drop(&self) {
        self.radix_dropped.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> EpochSnapshot {
        EpochSnapshot {
            defers_registered: self.defers_registered.load(Ordering::Relaxed),
            guards_pinned: self.guards_pinned.load(Ordering::Relaxed),
            flushes_called: self.flushes_called.load(Ordering::Relaxed),
            radix_created: self.radix_created.load(Ordering::Relaxed),
            radix_dropped: self.radix_dropped.load(Ordering::Relaxed),
        }
    }

    pub fn reset(&self) {
        self.defers_registered.store(0, Ordering::Relaxed);
        self.guards_pinned.store(0, Ordering::Relaxed);
        self.flushes_called.store(0, Ordering::Relaxed);
        self.radix_created.store(0, Ordering::Relaxed);
        self.radix_dropped.store(0, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EpochSnapshot {
    pub defers_registered: usize,
    pub guards_pinned: usize,
    pub flushes_called: usize,
    pub radix_created: usize,
    pub radix_dropped: usize,
}

impl EpochSnapshot {
    pub fn print_summary(&self) {
        eprintln!("=== Epoch Statistics ===");
        eprintln!("  Defers registered: {}", self.defers_registered);
        eprintln!("  Guards pinned:     {}", self.guards_pinned);
        eprintln!("  Flushes called:    {}", self.flushes_called);
        eprintln!("  RadixIndex created: {}", self.radix_created);
        eprintln!("  RadixIndex dropped: {}", self.radix_dropped);
        eprintln!("  Active RadixIndex:  {}", self.radix_created.saturating_sub(self.radix_dropped));
        eprintln!("  Pending defers estimate: {}", self.estimate_pending());
        eprintln!();
    }

    /// Rough estimate of pending defers
    /// Assumes each flush processes some percentage of defers
    fn estimate_pending(&self) -> usize {
        // Very rough heuristic - actual epoch behavior is more complex
        let processed_estimate = self.flushes_called.saturating_mul(100);
        self.defers_registered.saturating_sub(processed_estimate)
    }
}

/// Global singleton
pub static EPOCH_STATS: EpochStats = EpochStats::new();

/// Wrapper for epoch::pin() that tracks statistics
#[inline]
pub fn tracked_pin() -> crossbeam_epoch::Guard {
    EPOCH_STATS.register_pin();
    crossbeam_epoch::pin()
}

/// Track a defer call
#[inline]
pub fn track_defer() {
    EPOCH_STATS.register_defer();
}

