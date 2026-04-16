//! Cache configuration with builder pattern.

use serde::{Deserialize, Serialize};

/// Configuration for the valuation cache.
///
/// Controls the maximum number of entries and memory usage before
/// LRU eviction kicks in. Both bounds are soft: eviction runs inline
/// on insert and removes approximately 10% of entries per pass.
///
/// # Defaults
///
/// | Parameter | Default |
/// |-----------|---------|
/// | `max_entries` | 10,000 |
/// | `max_memory_bytes` | 256 MB |
///
/// # Examples
///
/// ```
/// use finstack_valuations::cache::CacheConfig;
///
/// // Use defaults
/// let config = CacheConfig::default();
/// assert_eq!(config.max_entries(), 10_000);
///
/// // Builder pattern
/// let config = CacheConfig::builder()
///     .max_entries(5_000)
///     .max_memory_bytes(128 * 1024 * 1024)
///     .build();
/// assert_eq!(config.max_entries(), 5_000);
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Maximum number of entries before LRU eviction.
    max_entries: usize,
    /// Maximum estimated memory usage in bytes before eviction.
    max_memory_bytes: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            max_memory_bytes: 256 * 1024 * 1024,
        }
    }
}

impl CacheConfig {
    /// Create a builder for `CacheConfig`.
    pub fn builder() -> CacheConfigBuilder {
        CacheConfigBuilder::default()
    }

    /// Maximum number of entries before LRU eviction.
    pub fn max_entries(&self) -> usize {
        self.max_entries
    }

    /// Maximum estimated memory usage in bytes before eviction.
    pub fn max_memory_bytes(&self) -> usize {
        self.max_memory_bytes
    }
}

/// Builder for [`CacheConfig`].
#[derive(Debug)]
pub struct CacheConfigBuilder {
    max_entries: usize,
    max_memory_bytes: usize,
}

impl Default for CacheConfigBuilder {
    fn default() -> Self {
        let defaults = CacheConfig::default();
        Self {
            max_entries: defaults.max_entries,
            max_memory_bytes: defaults.max_memory_bytes,
        }
    }
}

impl CacheConfigBuilder {
    /// Set the maximum number of cached entries.
    pub fn max_entries(mut self, n: usize) -> Self {
        self.max_entries = n;
        self
    }

    /// Set the maximum estimated memory usage in bytes.
    pub fn max_memory_bytes(mut self, bytes: usize) -> Self {
        self.max_memory_bytes = bytes;
        self
    }

    /// Build the configuration.
    pub fn build(self) -> CacheConfig {
        CacheConfig {
            max_entries: self.max_entries,
            max_memory_bytes: self.max_memory_bytes,
        }
    }
}
