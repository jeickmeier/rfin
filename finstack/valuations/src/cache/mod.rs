//! Valuation result caching with LRU eviction.
//!
//! Provides a content-addressed, memory-bounded cache for `ValuationResult`
//! values. Designed for use with Rayon-parallel portfolio pricing: multiple
//! worker threads can concurrently read from and insert into the cache.
//!
//! # Architecture
//!
//! The cache uses a content-addressed key (`CacheKey`) that hashes together
//! the instrument ID, market data version, per-curve versions, pricing model,
//! and requested metrics. Two pricings with the same key are guaranteed to
//! produce identical results.
//!
//! The storage layer (`ValuationCache`) wraps a `RwLock<HashMap>` with
//! approximate LRU eviction by access time. Eviction is triggered inline
//! on insert when either entry count or memory usage exceeds the configured
//! thresholds, removing approximately 10% of entries per pass.
//!
//! # Quick Start
//!
//! ```
//! use finstack_valuations::cache::{
//!     CacheConfig, CacheKey, CacheKeyInput, InstrumentFingerprint, ValuationCache,
//! };
//! use finstack_valuations::pricer::ModelKey;
//! use finstack_valuations::results::ValuationResult;
//! use finstack_core::currency::Currency;
//! use finstack_core::dates::create_date;
//! use finstack_core::money::Money;
//! use std::sync::Arc;
//! use time::Month;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let cache = ValuationCache::new(CacheConfig::default());
//! let as_of = create_date(2025, Month::January, 15)?;
//!
//! // Build a cache key.
//! //
//! // Use `InstrumentFingerprint::ImmutableById` only for instruments that
//! // cannot be mutated after construction. For mutable instruments, pass a
//! // non-zero content hash derived from the instrument body.
//! let key = CacheKey::new(&CacheKeyInput {
//!     instrument_id: "BOND-001",
//!     instrument_fingerprint: InstrumentFingerprint::ImmutableById,
//!     market_version: 1,
//!     curve_versions: &[],
//!     model_key: ModelKey::Discounting,
//!     metrics: &[],
//! });
//!
//! // Insert a result
//! let result = ValuationResult::stamped("BOND-001", as_of, Money::new(100.0, Currency::USD));
//! cache.insert(key.clone(), Arc::new(result));
//!
//! // Retrieve the cached result
//! let cached = cache.get(&key);
//! assert!(cached.is_some());
//!
//! // Check statistics
//! let stats = cache.stats().snapshot();
//! assert_eq!(stats.hits, 1);
//! # Ok(())
//! # }
//! ```
//!
//! # Module Layout
//!
//! - [`config`]: `CacheConfig` with builder pattern
//! - [`key`]: `CacheKey` and `CacheKeyInput` for content-addressed hashing
//! - [`store`]: `ValuationCache` with LRU eviction and thread-safe access
//! - [`stats`]: `CacheStats` (atomic) and `CacheStatsSnapshot` (plain)
//! - [`size`]: Memory size estimation for eviction decisions

mod config;
mod key;
pub(crate) mod size;
mod stats;
mod store;

pub use config::{CacheConfig, CacheConfigBuilder};
pub use key::{CacheKey, CacheKeyInput, InstrumentFingerprint};
pub use stats::{CacheStats, CacheStatsSnapshot};
pub use store::ValuationCache;
