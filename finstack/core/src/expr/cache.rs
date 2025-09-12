//! Caching system for expression evaluation results.
//!
//! This module provides a multi-level cache for intermediate expression
//! results to avoid recomputation in complex DAGs with shared sub-expressions.

use super::dag::ExecutionPlan;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};

/// Cached result for an expression evaluation.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CachedResult {
    /// Scalar result backed by Arc slice to avoid clones on conversion.
    Scalar(Arc<[f64]>),
}

impl CachedResult {
    /// Get as scalar vector.
    pub fn as_scalar(&self) -> crate::Result<Vec<f64>> {
        match self {
            CachedResult::Scalar(shared) => Ok(shared.as_ref().to_vec()),
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
    last_access: std::time::Instant,
    /// How many times this entry has been accessed.
    access_count: usize,
    /// Memory size in bytes.
    size: usize,
}

/// LRU cache for expression results with memory budget management.
#[derive(Debug)]
pub struct ExpressionCache {
    /// The actual cache storage.
    entries: HashMap<u64, CacheEntry>,
    /// Access order for LRU eviction.
    access_order: VecDeque<u64>,
    /// Maximum memory budget in bytes.
    max_memory: usize,
    /// Current memory usage.
    current_memory: usize,
    /// Cache hit/miss statistics.
    stats: CacheStats,
}

/// Cache performance statistics.
#[derive(Debug, Clone, Default]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
        Self {
            entries: HashMap::new(),
            access_order: VecDeque::new(),
            max_memory: max_memory_mb * 1024 * 1024, // Convert MB to bytes
            current_memory: 0,
            stats: CacheStats::default(),
        }
    }

    /// Create cache optimized for the given execution plan.
    pub fn for_plan(plan: &ExecutionPlan, budget_mb: usize) -> Self {
        let mut cache = Self::with_budget(budget_mb);

        // Pre-allocate space for nodes that are likely to be cached
        let estimated_entries = plan.cache_strategy.cache_nodes.len();
        cache.entries.reserve(estimated_entries);

        cache
    }

    /// Get a cached result if available.
    pub fn get(&mut self, node_id: u64) -> Option<CachedResult> {
        if let Some(entry) = self.entries.get_mut(&node_id) {
            // Update access metadata
            entry.last_access = std::time::Instant::now();
            entry.access_count += 1;

            // Move to end of access order (most recently used)
            if let Some(pos) = self.access_order.iter().position(|&id| id == node_id) {
                self.access_order.remove(pos);
            }
            self.access_order.push_back(node_id);

            self.stats.hits += 1;
            Some(entry.result.clone())
        } else {
            self.stats.misses += 1;
            None
        }
    }

    /// Store a result in the cache.
    pub fn put(&mut self, node_id: u64, result: CachedResult) -> bool {
        let size = result.memory_size();

        // Check if we need to make space
        while self.current_memory + size > self.max_memory && !self.access_order.is_empty() {
            if !self.evict_lru() {
                // Couldn't evict anything, item too large
                return false;
            }
        }

        // Remove existing entry if present
        if let Some(old_entry) = self.entries.remove(&node_id) {
            self.current_memory -= old_entry.size;
            if let Some(pos) = self.access_order.iter().position(|&id| id == node_id) {
                self.access_order.remove(pos);
            }
        }

        // Insert new entry
        let entry = CacheEntry {
            result,
            last_access: std::time::Instant::now(),
            access_count: 1,
            size,
        };

        self.entries.insert(node_id, entry);
        self.access_order.push_back(node_id);
        self.current_memory += size;

        // Update stats
        self.stats.entries = self.entries.len();
        self.stats.memory_usage = self.current_memory;

        true
    }

    /// Evict the least recently used entry.
    fn evict_lru(&mut self) -> bool {
        if let Some(node_id) = self.access_order.pop_front() {
            if let Some(entry) = self.entries.remove(&node_id) {
                self.current_memory -= entry.size;
                self.stats.evictions += 1;
                self.stats.entries = self.entries.len();
                self.stats.memory_usage = self.current_memory;
                return true;
            }
        }
        false
    }

    /// Clear all cached entries.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.access_order.clear();
        self.current_memory = 0;
        self.stats = CacheStats::default();
    }

    /// Get current cache statistics.
    pub fn stats(&self) -> CacheStats {
        self.stats.clone()
    }

    /// Check if a node result is cached.
    pub fn contains(&self, node_id: u64) -> bool {
        self.entries.contains_key(&node_id)
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
    /// The underlying cache, protected by RwLock for thread safety.
    cache: Arc<RwLock<ExpressionCache>>,
}

impl CacheManager {
    /// Create a new cache manager.
    pub fn new(budget_mb: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(ExpressionCache::with_budget(budget_mb))),
        }
    }

    /// Create cache manager optimized for an execution plan.
    pub fn for_plan(plan: &ExecutionPlan, budget_mb: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(ExpressionCache::for_plan(plan, budget_mb))),
        }
    }

    /// Get a cached result.
    pub fn get(&self, node_id: u64) -> Option<CachedResult> {
        self.cache.write().ok()?.get(node_id)
    }

    /// Store a result in the cache.
    pub fn put(&self, node_id: u64, result: CachedResult) -> bool {
        self.cache
            .write()
            .map(|mut c| c.put(node_id, result))
            .unwrap_or(false)
    }

    /// Check if a result is cached.
    pub fn contains(&self, node_id: u64) -> bool {
        self.cache
            .read()
            .map(|c| c.contains(node_id))
            .unwrap_or(false)
    }

    /// Get cache statistics.
    pub fn stats(&self) -> Option<CacheStats> {
        self.cache.read().ok().map(|c| c.stats())
    }

    /// Get cache hit ratio.
    pub fn hit_ratio(&self) -> f64 {
        self.cache.read().map(|c| c.hit_ratio()).unwrap_or(0.0)
    }

    /// Clear the cache.
    pub fn clear(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
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
        let retrieved = cache.get(1).unwrap();
        assert_eq!(retrieved.as_scalar().unwrap(), data);

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
            entries: HashMap::new(),
            access_order: VecDeque::new(),
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
