//! Caching system for expression evaluation results.
//!
//! Provides LRU-based caching for intermediate expression results to avoid
//! recomputation when evaluating complex DAGs with shared sub-expressions.
//! Cache size is configurable and memory usage is tracked.

use super::dag::ExecutionPlan;
use lru::LruCache;
use parking_lot::Mutex;
use std::num::NonZeroUsize;
use std::sync::Arc;

/// Default capacity for expression cache (number of entries).
pub(crate) const DEFAULT_CACHE_CAPACITY: usize = 1024;

/// Cached result for an expression evaluation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum CachedResult {
    /// Scalar result backed by Arc slice to avoid clones on conversion.
    Scalar(Arc<[f64]>),
}

impl CachedResult {
    /// Borrow the cached scalar payload as a slice.
    pub(crate) fn as_scalar_slice(&self) -> &[f64] {
        match self {
            CachedResult::Scalar(shared) => shared.as_ref(),
        }
    }

    /// Length of the cached payload.
    pub(crate) fn len(&self) -> usize {
        match self {
            CachedResult::Scalar(shared) => shared.len(),
        }
    }

    /// Estimate memory usage in bytes.
    pub(crate) fn memory_size(&self) -> usize {
        match self {
            CachedResult::Scalar(shared) => shared.len() * std::mem::size_of::<f64>(),
        }
    }
}

/// Cache entry with metadata.
///
/// LRU ordering is handled by the underlying `lru` crate via insertion/access
/// order — no wall-clock timestamp is used, keeping cache behaviour
/// deterministic across runs.
#[derive(Debug, Clone)]
struct CacheEntry {
    /// The cached result.
    result: CachedResult,
    /// How many times this entry has been accessed.
    access_count: usize,
    /// Memory size in bytes.
    size: usize,
    /// Result length for cache compatibility checks.
    len: usize,
}

/// LRU cache for expression results with memory budget management.
#[derive(Debug)]
pub(crate) struct ExpressionCache {
    /// The actual cache storage using lru crate.
    cache: LruCache<u64, CacheEntry>,
    /// Maximum memory budget in bytes.
    max_memory: usize,
    /// Current memory usage.
    current_memory: usize,
    /// Cache hit/miss statistics.
    stats: CacheStats,
}

/// Cache performance statistics.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct CacheStats {
    /// Total cache hits.
    pub(crate) hits: usize,
    /// Total cache misses.
    pub(crate) misses: usize,
    /// Total evictions due to memory pressure.
    pub(crate) evictions: usize,
    /// Current number of entries.
    pub(crate) entries: usize,
    /// Current memory usage in bytes.
    pub(crate) memory_usage: usize,
}

impl ExpressionCache {
    /// Create a new expression cache with the given memory budget.
    pub(crate) fn with_budget(max_memory_mb: usize) -> Self {
        Self::with_budget_and_capacity(max_memory_mb, DEFAULT_CACHE_CAPACITY)
    }

    /// Create a new expression cache with custom memory budget and capacity.
    pub(crate) fn with_budget_and_capacity(max_memory_mb: usize, capacity: usize) -> Self {
        // Use NonZeroUsize::MIN (1) as fallback if capacity is 0
        let capacity = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::MIN);
        Self {
            cache: LruCache::new(capacity),
            max_memory: max_memory_mb * 1024 * 1024, // Convert MB to bytes
            current_memory: 0,
            stats: CacheStats::default(),
        }
    }

    /// Create cache optimized for the given execution plan.
    pub(crate) fn for_plan(plan: &ExecutionPlan, budget_mb: usize) -> Self {
        // Use the estimated entries from the plan as the initial capacity (at least 64)
        let estimated_entries = plan.cache_strategy.cache_nodes.len().max(64);
        // SAFETY: max(64) guarantees non-zero, but use MIN as defensive fallback
        let capacity = NonZeroUsize::new(estimated_entries).unwrap_or(NonZeroUsize::MIN);
        Self {
            cache: LruCache::new(capacity),
            max_memory: budget_mb * 1024 * 1024, // Convert MB to bytes
            current_memory: 0,
            stats: CacheStats::default(),
        }
    }

    /// Get a cached result if available and matching the requested length.
    pub(crate) fn get(&mut self, node_id: u64, len: usize) -> Option<CachedResult> {
        if self
            .cache
            .peek(&node_id)
            .is_none_or(|entry| entry.len != len)
        {
            self.record_miss();
            return None;
        }

        match self.cache.get_mut(&node_id) {
            Some(entry) => {
                entry.access_count += 1;
                self.stats.hits += 1;
                Some(entry.result.clone())
            }
            None => {
                debug_assert!(
                    false,
                    "ExpressionCache::get peek succeeded but get_mut failed for node {node_id}"
                );
                self.record_miss();
                None
            }
        }
    }

    /// Store a result in the cache.
    pub(crate) fn put(&mut self, node_id: u64, result: CachedResult) -> bool {
        let size = result.memory_size();
        let len = result.len();

        // Check if we need to make space for memory budget
        while self.current_memory + size > self.max_memory && !self.cache.is_empty() {
            if !self.evict_lru() {
                // Couldn't evict anything, item too large for budget
                return false;
            }
        }

        // Handle existing entry replacement
        if let Some(old_entry) = self.cache.peek(&node_id) {
            self.current_memory -= old_entry.size;
        }

        // Create new entry
        let entry = CacheEntry {
            result,
            access_count: 1,
            size,
            len,
        };

        // Insert will handle LRU eviction if capacity is exceeded
        if let Some(evicted) = self.cache.put(node_id, entry) {
            // The LRU cache evicted an entry due to capacity limit
            self.current_memory -= evicted.size;
            self.stats.evictions += 1;
        }

        self.current_memory += size;

        // Update stats
        self.stats.entries = self.cache.len();
        self.stats.memory_usage = self.current_memory;

        true
    }

    #[inline]
    fn record_miss(&mut self) {
        self.stats.misses += 1;
    }

    /// Evict the least recently used entry.
    fn evict_lru(&mut self) -> bool {
        if let Some((_, entry)) = self.cache.pop_lru() {
            self.current_memory -= entry.size;
            self.stats.evictions += 1;
            self.stats.entries = self.cache.len();
            self.stats.memory_usage = self.current_memory;
            true
        } else {
            false
        }
    }
}

/// Global cache manager with thread-safe access.
#[derive(Debug, Clone)]
pub(crate) struct CacheManager {
    /// The underlying cache, protected by Mutex for thread safety.
    cache: Arc<Mutex<ExpressionCache>>,
}

impl CacheManager {
    /// Create a new cache manager.
    pub(crate) fn new(budget_mb: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(ExpressionCache::with_budget(budget_mb))),
        }
    }

    /// Create cache manager optimized for an execution plan.
    pub(crate) fn for_plan(plan: &ExecutionPlan, budget_mb: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(ExpressionCache::for_plan(plan, budget_mb))),
        }
    }

    /// Get a cached result.
    pub(crate) fn get(&self, node_id: u64, len: usize) -> Option<CachedResult> {
        self.cache.lock().get(node_id, len)
    }

    /// Store a result in the cache.
    pub(crate) fn put(&self, node_id: u64, result: CachedResult) -> bool {
        self.cache.lock().put(node_id, result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_basic_operations() {
        let mut cache = ExpressionCache::with_budget(10); // 10MB budget

        let data = vec![1.0, 2.0, 3.0];
        let result = CachedResult::Scalar(Arc::from(data.clone().into_boxed_slice()));

        // Store and retrieve
        assert!(cache.put(1, result));
        let retrieved = cache
            .get(1, data.len())
            .expect("Value should exist after put");
        assert_eq!(retrieved.as_scalar_slice(), data.as_slice());

        // Check stats
        let stats = &cache.stats;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.entries, 1);
    }

    #[test]
    fn cache_lru_eviction() {
        // Create cache with very small budget (64KB = 0.0625MB)
        // Each large_data item is ~80KB (10000 * 8 bytes), so only one will fit
        let mut cache = ExpressionCache {
            cache: LruCache::new(NonZeroUsize::new(10).expect("10 should be non-zero")), // Small capacity for testing
            max_memory: 65536, // 64KB in bytes
            current_memory: 0,
            stats: CacheStats {
                hits: 0,
                misses: 0,
                evictions: 0,
                entries: 0,
                memory_usage: 0,
            },
        };

        // This should force eviction
        let large_data: Vec<f64> = (0..10000).map(|i| i as f64).collect();
        let result1 = CachedResult::Scalar(Arc::from(large_data.clone().into_boxed_slice()));
        let result2 = CachedResult::Scalar(Arc::from(large_data.clone().into_boxed_slice()));

        assert!(cache.put(1, result1));
        assert!(cache.put(2, result2)); // Should evict entry 1

        assert!(!cache.cache.contains(&1)); // Entry 1 should be evicted
        assert!(cache.cache.contains(&2)); // Entry 2 should remain

        let stats = &cache.stats;
        assert_eq!(stats.evictions, 1);
    }
}
