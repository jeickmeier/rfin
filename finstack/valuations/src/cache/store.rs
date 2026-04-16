//! Thread-safe, memory-bounded valuation result cache.
//!
//! Uses `RwLock<HashMap>` for concurrent read/write access. Reads acquire
//! a shared lock; writes (insert, evict) acquire an exclusive lock. This
//! is sufficient for the typical pricing workload where reads dominate
//! during cache-hit scenarios.
//!
//! Eviction is approximate LRU: when capacity thresholds are exceeded,
//! the oldest-accessed entries are removed. Eviction runs inline on
//! insert to keep the implementation simple and deterministic.

use super::config::CacheConfig;
use super::key::CacheKey;
use super::size::estimate_result_size;
use super::stats::CacheStats;
use crate::results::ValuationResult;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

/// Cache entry wrapping a result with access metadata.
struct CacheEntry {
    /// Shared reference to the cached valuation result.
    result: Arc<ValuationResult>,
    /// Estimated heap size in bytes for eviction decisions.
    estimated_size: usize,
    /// Monotonic access counter for LRU ordering.
    last_access: u64,
}

/// Thread-safe, memory-bounded valuation result cache.
///
/// Each pricing thread can look up and insert results through shared
/// `&self` references. The cache maintains approximate LRU eviction
/// to bound both entry count and memory usage.
///
/// # Thread Safety
///
/// All operations use interior mutability via `RwLock`. Read operations
/// (`get`) acquire a write lock to update access timestamps. Insert and
/// eviction operations also acquire a write lock. This is acceptable for
/// the pricing workload where cache operations are fast relative to
/// actual instrument pricing.
///
/// # Examples
///
/// ```
/// use finstack_valuations::cache::{CacheConfig, CacheKey, CacheKeyInput, ValuationCache};
/// use finstack_valuations::pricer::ModelKey;
/// use finstack_valuations::results::ValuationResult;
/// use finstack_core::currency::Currency;
/// use finstack_core::dates::create_date;
/// use finstack_core::money::Money;
/// use std::sync::Arc;
/// use time::Month;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let cache = ValuationCache::new(CacheConfig::default());
/// let as_of = create_date(2025, Month::January, 15)?;
///
/// let key = CacheKey::new(&CacheKeyInput {
///     instrument_id: "BOND-001",
///     market_version: 1,
///     curve_versions: &[],
///     model_key: ModelKey::Discounting,
///     metrics: &[],
/// });
///
/// // Miss on first lookup
/// assert!(cache.get(&key).is_none());
///
/// // Insert a result
/// let result = ValuationResult::stamped("BOND-001", as_of, Money::new(100.0, Currency::USD));
/// cache.insert(key.clone(), Arc::new(result));
///
/// // Hit on second lookup
/// assert!(cache.get(&key).is_some());
/// assert_eq!(cache.stats().entry_count(), 1);
/// # Ok(())
/// # }
/// ```
pub struct ValuationCache {
    map: RwLock<HashMap<CacheKey, CacheEntry>>,
    config: CacheConfig,
    stats: CacheStats,
    /// Monotonic counter for LRU ordering.
    access_counter: AtomicU64,
}

impl ValuationCache {
    /// Create a new cache with the given configuration.
    pub fn new(config: CacheConfig) -> Self {
        Self {
            map: RwLock::new(HashMap::with_capacity(config.max_entries())),
            config,
            stats: CacheStats::default(),
            access_counter: AtomicU64::new(0),
        }
    }

    /// Look up a cached valuation result.
    ///
    /// Returns `Some(Arc<ValuationResult>)` on hit, updating LRU
    /// access time. Returns `None` on miss.
    pub fn get(&self, key: &CacheKey) -> Option<Arc<ValuationResult>> {
        self.stats.lookups.fetch_add(1, Ordering::Relaxed);

        // We need a write lock to update the access timestamp.
        let mut map = self.map.write().ok()?;
        match map.get_mut(key) {
            Some(entry) => {
                entry.last_access = self.access_counter.fetch_add(1, Ordering::Relaxed);
                self.stats.hits.fetch_add(1, Ordering::Relaxed);
                Some(Arc::clone(&entry.result))
            }
            None => {
                self.stats.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    /// Insert a valuation result into the cache.
    ///
    /// If the cache exceeds capacity after insertion, the least
    /// recently accessed entries are evicted until within bounds.
    pub fn insert(&self, key: CacheKey, result: Arc<ValuationResult>) {
        let estimated_size = estimate_result_size(&result);

        let entry = CacheEntry {
            result,
            estimated_size,
            last_access: self.access_counter.fetch_add(1, Ordering::Relaxed),
        };

        {
            let mut map = match self.map.write() {
                Ok(m) => m,
                Err(_) => return, // poisoned lock: skip insert
            };
            map.insert(key, entry);
            self.stats.entries.store(map.len(), Ordering::Relaxed);
        }

        self.stats
            .memory_bytes
            .fetch_add(estimated_size, Ordering::Relaxed);

        self.maybe_evict();
    }

    /// Invalidate all entries whose instrument_id matches.
    ///
    /// Used when an instrument's specification changes (rare).
    pub fn invalidate_instrument(&self, instrument_id: &str) {
        let mut map = match self.map.write() {
            Ok(m) => m,
            Err(_) => return,
        };
        map.retain(|_, entry| entry.result.instrument_id != instrument_id);
        self.stats.entries.store(map.len(), Ordering::Relaxed);
    }

    /// Clear the entire cache. Resets entry count and memory tracking.
    pub fn clear(&self) {
        let mut map = match self.map.write() {
            Ok(m) => m,
            Err(_) => return,
        };
        map.clear();
        self.stats.entries.store(0, Ordering::Relaxed);
        self.stats.memory_bytes.store(0, Ordering::Relaxed);
    }

    /// Current cache statistics.
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Current number of entries in the cache.
    pub fn len(&self) -> usize {
        self.map.read().map_or(0, |m| m.len())
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Create a snapshot of the cache for what-if analysis.
    ///
    /// The snapshot shares `Arc<ValuationResult>` pointers with the
    /// original cache (cheap clone). Mutations to the snapshot do not
    /// affect the original.
    pub fn snapshot(&self) -> ValuationCache {
        let map = match self.map.read() {
            Ok(m) => m,
            Err(_) => {
                return ValuationCache::new(self.config.clone());
            }
        };

        let mut new_map = HashMap::with_capacity(map.len());
        for (key, entry) in map.iter() {
            new_map.insert(
                key.clone(),
                CacheEntry {
                    result: Arc::clone(&entry.result),
                    estimated_size: entry.estimated_size,
                    last_access: entry.last_access,
                },
            );
        }

        ValuationCache {
            map: RwLock::new(new_map),
            config: self.config.clone(),
            stats: CacheStats::default(),
            access_counter: AtomicU64::new(self.access_counter.load(Ordering::Relaxed)),
        }
    }

    /// Evict least-recently-accessed entries if over capacity.
    fn maybe_evict(&self) {
        let entries = self.stats.entries.load(Ordering::Relaxed);
        let memory = self.stats.memory_bytes.load(Ordering::Relaxed);

        if entries <= self.config.max_entries() && memory <= self.config.max_memory_bytes() {
            return;
        }

        // Evict approximately 10% of entries per pass to amortize overhead.
        let evict_count = entries / 10;
        if evict_count == 0 {
            return;
        }

        let mut map = match self.map.write() {
            Ok(m) => m,
            Err(_) => return,
        };

        // Collect all access times, sort, find eviction cutoff.
        let mut access_times: Vec<(CacheKey, u64)> = map
            .iter()
            .map(|(k, e)| (k.clone(), e.last_access))
            .collect();
        access_times.sort_unstable_by_key(|(_, t)| *t);

        let mut evicted = 0u64;
        for (key, _) in access_times.iter().take(evict_count) {
            if let Some(entry) = map.remove(key) {
                self.stats
                    .memory_bytes
                    .fetch_sub(entry.estimated_size, Ordering::Relaxed);
                evicted += 1;
            }
        }

        self.stats.evictions.fetch_add(evicted, Ordering::Relaxed);
        self.stats.entries.store(map.len(), Ordering::Relaxed);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::cache::{CacheKeyInput, CacheStatsSnapshot};
    use crate::pricer::ModelKey;
    use finstack_core::currency::Currency;
    use finstack_core::dates::create_date;
    use finstack_core::money::Money;
    use time::Month;

    fn make_result(id: &str) -> ValuationResult {
        let as_of = create_date(2025, Month::January, 15).expect("valid date");
        ValuationResult::stamped(id, as_of, Money::new(100.0, Currency::USD))
    }

    fn make_key(instrument_id: &str, version: u64) -> CacheKey {
        CacheKey::new(&CacheKeyInput {
            instrument_id,
            market_version: version,
            curve_versions: &[],
            model_key: ModelKey::Discounting,
            metrics: &[],
        })
    }

    #[test]
    fn insert_and_get() {
        let cache = ValuationCache::new(CacheConfig::default());
        let key = make_key("BOND-001", 1);
        let result = Arc::new(make_result("BOND-001"));

        // Miss
        assert!(cache.get(&key).is_none());
        assert_eq!(cache.stats().misses(), 1);

        // Insert
        cache.insert(key.clone(), result);
        assert_eq!(cache.len(), 1);

        // Hit
        let cached = cache.get(&key);
        assert!(cached.is_some());
        assert_eq!(cache.stats().hits(), 1);
        assert_eq!(cached.expect("just checked").instrument_id, "BOND-001");
    }

    #[test]
    fn cache_miss_with_different_version() {
        let cache = ValuationCache::new(CacheConfig::default());
        let key_v1 = make_key("BOND-001", 1);
        let key_v2 = make_key("BOND-001", 2);
        let result = Arc::new(make_result("BOND-001"));

        cache.insert(key_v1, result);
        assert!(cache.get(&key_v2).is_none());
    }

    #[test]
    fn lru_eviction_by_entry_count() {
        let config = CacheConfig::builder().max_entries(5).build();
        let cache = ValuationCache::new(config);

        // Insert 10 entries
        for i in 0..10 {
            let key = make_key(&format!("BOND-{i:03}"), 1);
            cache.insert(key, Arc::new(make_result(&format!("BOND-{i:03}"))));
        }

        // Eviction should have kicked in, reducing below 10.
        assert!(
            cache.len() < 10,
            "cache should have evicted some entries, got {}",
            cache.len()
        );
        assert!(
            cache.stats().evictions() > 0,
            "should have recorded evictions"
        );
    }

    #[test]
    fn lru_preserves_recently_accessed() {
        let config = CacheConfig::builder().max_entries(5).build();
        let cache = ValuationCache::new(config);

        // Insert 5 entries
        for i in 0..5 {
            let key = make_key(&format!("BOND-{i:03}"), 1);
            cache.insert(key, Arc::new(make_result(&format!("BOND-{i:03}"))));
        }

        // Access entry 0 to make it "recently used"
        let key_0 = make_key("BOND-000", 1);
        let _ = cache.get(&key_0);

        // Insert more entries to trigger eviction
        for i in 5..10 {
            let key = make_key(&format!("BOND-{i:03}"), 1);
            cache.insert(key, Arc::new(make_result(&format!("BOND-{i:03}"))));
        }

        // Entry 0 should still be present (recently accessed)
        assert!(
            cache.get(&key_0).is_some(),
            "recently accessed entry should survive eviction"
        );
    }

    #[test]
    fn memory_bounded_eviction() {
        // Set a very small memory limit
        let config = CacheConfig::builder()
            .max_entries(1000)
            .max_memory_bytes(500)
            .build();
        let cache = ValuationCache::new(config);

        // Insert entries until memory limit exceeded
        for i in 0..20 {
            let key = make_key(&format!("BOND-{i:03}"), 1);
            cache.insert(key, Arc::new(make_result(&format!("BOND-{i:03}"))));
        }

        // Should have evicted some entries
        assert!(cache.len() < 20, "should have evicted entries for memory");
    }

    #[test]
    fn clear_resets_cache() {
        let cache = ValuationCache::new(CacheConfig::default());
        let key = make_key("BOND-001", 1);
        cache.insert(key.clone(), Arc::new(make_result("BOND-001")));
        assert_eq!(cache.len(), 1);

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.get(&key).is_none());
        assert_eq!(cache.stats().entry_count(), 0);
    }

    #[test]
    fn invalidate_instrument() {
        let cache = ValuationCache::new(CacheConfig::default());

        // Insert two entries for different instruments
        let key1 = make_key("BOND-001", 1);
        let key2 = make_key("BOND-002", 1);
        cache.insert(key1.clone(), Arc::new(make_result("BOND-001")));
        cache.insert(key2.clone(), Arc::new(make_result("BOND-002")));
        assert_eq!(cache.len(), 2);

        // Invalidate one instrument
        cache.invalidate_instrument("BOND-001");
        assert_eq!(cache.len(), 1);
        assert!(cache.get(&key1).is_none());
        assert!(cache.get(&key2).is_some());
    }

    #[test]
    fn snapshot_shares_data_but_is_independent() {
        let cache = ValuationCache::new(CacheConfig::default());
        let key = make_key("BOND-001", 1);
        cache.insert(key.clone(), Arc::new(make_result("BOND-001")));

        // Take snapshot
        let snap = cache.snapshot();
        assert_eq!(snap.len(), 1);
        assert!(snap.get(&key).is_some());

        // Modify original -- snapshot unaffected
        cache.clear();
        assert_eq!(cache.len(), 0);
        assert_eq!(snap.len(), 1);
    }

    #[test]
    fn stats_tracking() {
        let cache = ValuationCache::new(CacheConfig::default());
        let key = make_key("BOND-001", 1);

        // Miss
        let _ = cache.get(&key);
        assert_eq!(cache.stats().lookups(), 1);
        assert_eq!(cache.stats().misses(), 1);
        assert_eq!(cache.stats().hits(), 0);

        // Insert
        cache.insert(key.clone(), Arc::new(make_result("BOND-001")));

        // Hit
        let _ = cache.get(&key);
        assert_eq!(cache.stats().lookups(), 2);
        assert_eq!(cache.stats().hits(), 1);
        assert_eq!(cache.stats().misses(), 1);

        // Snapshot
        let snap = cache.stats().snapshot();
        assert_eq!(snap.lookups, 2);
        assert_eq!(snap.hits, 1);
        assert_eq!(snap.misses, 1);
        assert!(snap.hit_rate() > 0.0);
    }

    #[test]
    fn stats_snapshot_hit_rate() {
        let snap = CacheStatsSnapshot {
            lookups: 100,
            hits: 75,
            misses: 25,
            evictions: 0,
            entries: 50,
            memory_bytes: 0,
        };
        assert!((snap.hit_rate() - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn stats_snapshot_zero_lookups() {
        let snap = CacheStatsSnapshot {
            lookups: 0,
            hits: 0,
            misses: 0,
            evictions: 0,
            entries: 0,
            memory_bytes: 0,
        };
        assert!((snap.hit_rate() - 0.0).abs() < f64::EPSILON);
    }
}
