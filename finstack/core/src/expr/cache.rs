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
pub const DEFAULT_CACHE_CAPACITY: usize = 1024;

/// Cached result for an expression evaluation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CachedResult {
    /// Scalar result backed by Arc slice to avoid clones on conversion.
    Scalar(Arc<[f64]>),
}

impl CachedResult {
    /// Borrow the cached scalar payload as a slice.
    pub fn as_scalar_slice(&self) -> &[f64] {
        match self {
            CachedResult::Scalar(shared) => shared.as_ref(),
        }
    }

    /// Get as scalar vector.
    pub fn as_scalar(&self) -> crate::Result<Vec<f64>> {
        Ok(self.as_scalar_slice().to_vec())
    }

    /// Length of the cached payload.
    pub fn len(&self) -> usize {
        match self {
            CachedResult::Scalar(shared) => shared.len(),
        }
    }

    /// Estimate memory usage in bytes.
    pub fn memory_size(&self) -> usize {
        match self {
            CachedResult::Scalar(shared) => shared.len() * std::mem::size_of::<f64>(),
        }
    }
}

/// Cache entry with metadata.
#[derive(Debug, Clone)]
struct CacheEntry {
    /// The cached result.
    result: CachedResult,
    /// When this entry was last accessed.
    /// In WASM, this is a dummy value since Instant::now() is not available.
    #[cfg(target_arch = "wasm32")]
    last_access: u64, // Use counter instead of Instant in WASM
    #[cfg(not(target_arch = "wasm32"))]
    last_access: std::time::Instant,
    /// How many times this entry has been accessed.
    access_count: usize,
    /// Memory size in bytes.
    size: usize,
    /// Result length for cache compatibility checks.
    len: usize,
}

/// LRU cache for expression results with memory budget management.
#[derive(Debug)]
pub struct ExpressionCache {
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
pub struct CacheStats {
    /// Total cache hits.
    pub hits: usize,
    /// Total cache misses.
    pub misses: usize,
    /// Total evictions due to memory pressure.
    pub evictions: usize,
    /// Current number of entries.
    pub entries: usize,
    /// Current memory usage in bytes.
    pub memory_usage: usize,
}

impl ExpressionCache {
    /// Create a new expression cache with the given memory budget.
    pub fn with_budget(max_memory_mb: usize) -> Self {
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
    pub fn for_plan(plan: &ExecutionPlan, budget_mb: usize) -> Self {
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
    pub fn get(&mut self, node_id: u64, len: usize) -> Option<CachedResult> {
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
                // Update access metadata
                #[cfg(target_arch = "wasm32")]
                {
                    entry.last_access += 1; // Increment counter in WASM
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    entry.last_access = std::time::Instant::now();
                }
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
    pub fn put(&mut self, node_id: u64, result: CachedResult) -> bool {
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
            #[cfg(target_arch = "wasm32")]
            last_access: 0, // Dummy value in WASM (not used for LRU - lru crate uses insertion order)
            #[cfg(not(target_arch = "wasm32"))]
            last_access: std::time::Instant::now(),
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

    /// Clear all cached entries.
    pub fn clear(&mut self) {
        self.cache.clear();
        self.current_memory = 0;
        self.stats = CacheStats::default();
    }

    /// Get current cache statistics.
    pub fn stats(&self) -> CacheStats {
        self.stats.clone()
    }

    /// Check if a node result is cached.
    pub fn contains(&self, node_id: u64) -> bool {
        self.cache.contains(&node_id)
    }

    /// Calculate cache hit ratio.
    pub fn hit_ratio(&self) -> f64 {
        let total = self.stats.hits + self.stats.misses;
        if total == 0 {
            0.0
        } else {
            self.stats.hits as f64 / total as f64
        }
    }
}

/// Global cache manager with thread-safe access.
#[derive(Debug, Clone)]
pub struct CacheManager {
    /// The underlying cache, protected by Mutex for thread safety.
    cache: Arc<Mutex<ExpressionCache>>,
}

impl CacheManager {
    /// Create a new cache manager.
    pub fn new(budget_mb: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(ExpressionCache::with_budget(budget_mb))),
        }
    }

    /// Create cache manager optimized for an execution plan.
    pub fn for_plan(plan: &ExecutionPlan, budget_mb: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(ExpressionCache::for_plan(plan, budget_mb))),
        }
    }

    /// Get a cached result.
    pub fn get(&self, node_id: u64, len: usize) -> Option<CachedResult> {
        self.cache.lock().get(node_id, len)
    }

    /// Store a result in the cache.
    pub fn put(&self, node_id: u64, result: CachedResult) -> bool {
        self.cache.lock().put(node_id, result)
    }

    /// Check if a result is cached.
    pub fn contains(&self, node_id: u64) -> bool {
        self.cache.lock().contains(node_id)
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        self.cache.lock().stats()
    }

    /// Get cache hit ratio.
    pub fn hit_ratio(&self) -> f64 {
        self.cache.lock().hit_ratio()
    }

    /// Clear the cache.
    pub fn clear(&self) {
        self.cache.lock().clear();
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
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
        assert_eq!(retrieved.as_scalar().expect("Value should be scalar"), data);

        // Check stats
        let stats = cache.stats();
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

        assert!(!cache.contains(1)); // Entry 1 should be evicted
        assert!(cache.contains(2)); // Entry 2 should remain

        let stats = cache.stats();
        assert_eq!(stats.evictions, 1);
    }
}
