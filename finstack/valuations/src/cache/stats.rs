//! Cache statistics tracking with atomic counters.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// Thread-safe cache statistics using atomic counters.
///
/// All counters are updated with `Ordering::Relaxed` since exact
/// real-time accuracy is not required for monitoring purposes.
/// Use [`CacheStats::snapshot`] for a consistent point-in-time view.
#[derive(Debug)]
pub struct CacheStats {
    /// Total cache lookups.
    pub(crate) lookups: AtomicU64,
    /// Cache hits (result returned without repricing).
    pub(crate) hits: AtomicU64,
    /// Cache misses (repricing required).
    pub(crate) misses: AtomicU64,
    /// Total entries evicted since creation.
    pub(crate) evictions: AtomicU64,
    /// Current number of cached entries.
    pub(crate) entries: AtomicUsize,
    /// Current estimated memory usage in bytes.
    pub(crate) memory_bytes: AtomicUsize,
}

impl Default for CacheStats {
    fn default() -> Self {
        Self {
            lookups: AtomicU64::new(0),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            entries: AtomicUsize::new(0),
            memory_bytes: AtomicUsize::new(0),
        }
    }
}

impl CacheStats {
    /// Hit rate as a fraction in `[0.0, 1.0]`.
    ///
    /// Returns `0.0` when no lookups have been performed.
    pub fn hit_rate(&self) -> f64 {
        let total = self.lookups.load(Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }
        self.hits.load(Ordering::Relaxed) as f64 / total as f64
    }

    /// Total number of lookups performed.
    pub fn lookups(&self) -> u64 {
        self.lookups.load(Ordering::Relaxed)
    }

    /// Total number of cache hits.
    pub fn hits(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    /// Total number of cache misses.
    pub fn misses(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }

    /// Total number of evicted entries.
    pub fn evictions(&self) -> u64 {
        self.evictions.load(Ordering::Relaxed)
    }

    /// Current number of entries in the cache.
    pub fn entry_count(&self) -> usize {
        self.entries.load(Ordering::Relaxed)
    }

    /// Current estimated memory usage in bytes.
    pub fn memory_bytes(&self) -> usize {
        self.memory_bytes.load(Ordering::Relaxed)
    }

    /// Snapshot the current statistics as a plain struct (no atomics).
    ///
    /// Useful for serialization, logging, and reporting.
    pub fn snapshot(&self) -> CacheStatsSnapshot {
        CacheStatsSnapshot {
            lookups: self.lookups.load(Ordering::Relaxed),
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
            entries: self.entries.load(Ordering::Relaxed),
            memory_bytes: self.memory_bytes.load(Ordering::Relaxed),
        }
    }
}

/// Non-atomic snapshot of cache statistics for reporting.
///
/// All fields are plain integers, suitable for serialization and logging.
///
/// # Examples
///
/// ```
/// use finstack_valuations::cache::CacheStatsSnapshot;
///
/// let snap = CacheStatsSnapshot {
///     lookups: 100,
///     hits: 80,
///     misses: 20,
///     evictions: 5,
///     entries: 50,
///     memory_bytes: 1024 * 1024,
/// };
/// assert!((snap.hit_rate() - 0.8).abs() < f64::EPSILON);
/// assert!((snap.memory_mb() - 1.0).abs() < 0.01);
/// ```
#[derive(Clone, Debug, serde::Serialize)]
pub struct CacheStatsSnapshot {
    /// Total cache lookups.
    pub lookups: u64,
    /// Cache hits.
    pub hits: u64,
    /// Cache misses.
    pub misses: u64,
    /// Total evictions.
    pub evictions: u64,
    /// Current entry count.
    pub entries: usize,
    /// Current estimated memory in bytes.
    pub memory_bytes: usize,
}

impl CacheStatsSnapshot {
    /// Hit rate as a fraction in `[0.0, 1.0]`.
    pub fn hit_rate(&self) -> f64 {
        if self.lookups == 0 {
            return 0.0;
        }
        self.hits as f64 / self.lookups as f64
    }

    /// Memory usage in megabytes.
    pub fn memory_mb(&self) -> f64 {
        self.memory_bytes as f64 / (1024.0 * 1024.0)
    }
}
