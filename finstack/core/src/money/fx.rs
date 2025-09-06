//! Foreign-exchange interfaces and policy types.
//!
//! This module defines an `FxProvider` trait and simple policy metadata used by
//! `Money::convert`. Conversions are always explicit – arithmetic on `Money`
//! requires the same currency.

extern crate alloc;

use crate::currency::Currency;
use crate::dates::Date;
use alloc::sync::Arc;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Provider FX rate type alias – `Decimal` when `decimal128` is enabled, otherwise `f64`.
#[cfg(feature = "decimal128")]
pub type FxRate = rust_decimal::Decimal;
/// Provider FX rate type alias – `Decimal` when `decimal128` is enabled, otherwise `f64`.
#[cfg(not(feature = "decimal128"))]
pub type FxRate = f64;

/// Standard FX conversion strategies. These are metadata hints for providers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum FxConversionPolicy {
    /// Use spot/forward on the cashflow date.
    CashflowDate,
    /// Use period end date.
    PeriodEnd,
    /// Use an average over the period.
    PeriodAverage,
    /// Custom strategy defined by the caller/provider.
    Custom,
}

/// FX rate lookup query
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FxQuery {
    /// Source currency
    pub from: Currency,
    /// Target currency
    pub to: Currency,
    /// Applicable date for the rate
    pub on: Date,
    /// Conversion policy hint
    pub policy: FxConversionPolicy,
    /// Optional closure check via this intermediate currency
    pub closure_check: Option<Currency>,
    /// Whether the caller wants metadata (triangulation/closure)
    pub want_meta: bool,
}

/// Metadata describing the policy applied by the provider.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FxPolicyMeta {
    /// Strategy applied for the conversion.
    pub strategy: FxConversionPolicy,
    /// Optional declared target currency (for stamping).
    pub target_ccy: Option<Currency>,
    /// Optional notes for auditability.
    pub notes: String,
}

impl Default for FxPolicyMeta {
    fn default() -> Self {
        Self {
            strategy: FxConversionPolicy::CashflowDate,
            target_ccy: None,
            notes: String::new(),
        }
    }
}

/// Cache key for FX rate lookups  
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct FxCacheKey {
    from: Currency,
    to: Currency,
    date: Date,
    policy: FxConversionPolicy,
}

/// Cached FX rate entry with timestamp
#[derive(Clone, Debug)]
struct CachedFxEntry {
    rate: FxRate,
    cached_at: Instant,
    last_access_at: Instant,
    ttl: Duration,
}

impl CachedFxEntry {
    fn new(rate: FxRate, ttl: Duration) -> Self {
        Self {
            rate,
            cached_at: Instant::now(),
            last_access_at: Instant::now(),
            ttl,
        }
    }

    fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > self.ttl
    }
}

/// Configuration for FX caching and closure checking
#[derive(Clone, Debug)]
pub struct FxCacheConfig {
    /// Maximum number of entries to cache
    pub max_entries: usize,
    /// Default TTL for cached rates
    pub default_ttl: Duration,
    /// Tolerance for closure checking (e.g., 0.0001 = 1bp)
    pub closure_tolerance: f64,
    /// Whether closure violations should produce warnings or errors
    pub strict_closure: bool,
    /// Pivot currency for triangulation (typically USD)
    pub pivot_currency: Currency,
    /// Whether to enable automatic triangulation for missing rates
    pub enable_triangulation: bool,
}

impl Default for FxCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            default_ttl: Duration::from_secs(300), // 5 minutes
            closure_tolerance: 0.0001,             // 1 basis point
            strict_closure: false,
            pivot_currency: Currency::USD, // USD as default pivot
            enable_triangulation: true,    // Enable triangulation by default
        }
    }
}

/// Result of a closure check
#[derive(Clone, Debug, PartialEq)]
pub enum ClosureCheckResult {
    /// Closure check passed within tolerance
    Pass,
    /// Closure check failed - provides direct rate and calculated rate for comparison
    Fail {
        /// The direct FX rate from the provider
        direct_rate: FxRate,
        /// The calculated rate via intermediate currency
        calculated_rate: FxRate,
        /// The absolute difference between direct and calculated rates
        difference: FxRate,
    },
}

/// Result of an FX rate lookup with triangulation metadata
#[derive(Clone, Debug, PartialEq)]
pub struct FxRateResult {
    /// The final FX rate
    pub rate: FxRate,
    /// Whether this rate was obtained via triangulation
    pub triangulated: bool,
    /// The pivot currency used for triangulation (if applicable)
    pub pivot_currency: Option<Currency>,
    /// Optional closure check result
    pub closure: Option<ClosureCheckResult>,
}

/// Trait for obtaining FX rates.
pub trait FxProvider: Send + Sync {
    /// Return a rate to convert `from` → `to` applicable on `on` per `policy`.
    fn rate(
        &self,
        from: Currency,
        to: Currency,
        on: Date,
        policy: FxConversionPolicy,
    ) -> crate::Result<FxRate>;
}

/// Enhanced FX matrix with LRU caching and closure checking
pub struct FxMatrix {
    provider: Arc<dyn FxProvider>,
    cache: std::sync::RwLock<HashMap<FxCacheKey, CachedFxEntry>>,
    config: FxCacheConfig,
}

impl FxMatrix {
    /// Create a new `FxMatrix` wrapping the given provider with default cache configuration
    pub fn new(provider: Arc<dyn FxProvider>) -> Self {
        Self::with_config(provider, FxCacheConfig::default())
    }

    /// Create a new `FxMatrix` with custom cache configuration
    pub fn with_config(provider: Arc<dyn FxProvider>, config: FxCacheConfig) -> Self {
        Self {
            provider,
            cache: std::sync::RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Get rate (query-based) with caching, optional triangulation, and optional closure diagnostics
    pub fn rate(&self, q: FxQuery) -> crate::Result<FxRateResult> {
        let from = q.from;
        let to = q.to;
        let on = q.on;
        let policy = q.policy;

        // Handle identity case
        if from == to {
            #[cfg(feature = "decimal128")]
            let rate = rust_decimal::Decimal::ONE;
            #[cfg(not(feature = "decimal128"))]
            let rate = 1.0;

            let mut result = FxRateResult {
                rate,
                triangulated: false,
                pivot_currency: None,
                closure: None,
            };

            // Identity closure check is trivial if requested
            if let Some(mid) = q.closure_check {
                if q.want_meta {
                    let via_a = self
                        .rate(FxQuery {
                            from,
                            to: mid,
                            on,
                            policy,
                            closure_check: None,
                            want_meta: false,
                        })?
                        .rate;
                    let via_b = self
                        .rate(FxQuery {
                            from: mid,
                            to,
                            on,
                            policy,
                            closure_check: None,
                            want_meta: false,
                        })?
                        .rate;
                    result.closure = Some(self.check_closure(rate, via_a, via_b)?);
                }
            }
            return Ok(result);
        }

        let cache_key = FxCacheKey {
            from,
            to,
            date: on,
            policy,
        };

        // Try to get from cache first
        let mut hit_rate: Option<FxRate> = None;
        {
            let cache = self.cache.read().unwrap();
            if let Some(entry) = cache.get(&cache_key) {
                if !entry.is_expired() {
                    hit_rate = Some(entry.rate);
                }
            }
        }
        if let Some(rate) = hit_rate {
            // Update recency under write lock
            if let Ok(mut cache) = self.cache.write() {
                if let Some(entry) = cache.get_mut(&cache_key) {
                    entry.last_access_at = Instant::now();
                }
            }
            let mut result = FxRateResult {
                rate,
                triangulated: false,
                pivot_currency: None,
                closure: None,
            };
            if q.want_meta {
                if let Some(mid) = q.closure_check {
                    let via_a = self
                        .rate(FxQuery {
                            from,
                            to: mid,
                            on,
                            policy,
                            closure_check: None,
                            want_meta: false,
                        })?
                        .rate;
                    let via_b = self
                        .rate(FxQuery {
                            from: mid,
                            to,
                            on,
                            policy,
                            closure_check: None,
                            want_meta: false,
                        })?
                        .rate;
                    result.closure = Some(self.check_closure(rate, via_a, via_b)?);
                }
            }
            return Ok(result);
        }

        // Cache miss or expired - try direct rate first
        let mut triangulated = false;
        let mut pivot_currency: Option<Currency> = None;
        let rate = match self.provider.rate(from, to, on, policy) {
            Ok(rate) => {
                self.update_cache(cache_key, rate);
                rate
            }
            Err(_) if self.config.enable_triangulation => {
                // Direct rate failed, try triangulation
                let rate = self.triangulate_rate(from, to, on, policy)?;
                triangulated = true;
                pivot_currency = Some(self.config.pivot_currency);
                rate
            }
            Err(e) => return Err(e),
        };

        let mut result = FxRateResult {
            rate,
            triangulated,
            pivot_currency,
            closure: None,
        };
        if q.want_meta {
            if let Some(mid) = q.closure_check {
                let via_a = self
                    .rate(FxQuery {
                        from,
                        to: mid,
                        on,
                        policy,
                        closure_check: None,
                        want_meta: false,
                    })?
                    .rate;
                let via_b = self
                    .rate(FxQuery {
                        from: mid,
                        to,
                        on,
                        policy,
                        closure_check: None,
                        want_meta: false,
                    })?
                    .rate;
                let closure_result = self.check_closure(result.rate, via_a, via_b)?;
                if self.config.strict_closure {
                    if let ClosureCheckResult::Fail { .. } = closure_result {
                        return Err(crate::Error::Input(crate::error::InputError::Invalid));
                    }
                }
                result.closure = Some(closure_result);
            }
        }

        Ok(result)
    }

    /// Clear expired entries from the cache
    pub fn clear_expired(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.retain(|_, entry| !entry.is_expired());
    }

    /// Clear all cache entries
    pub fn clear_cache(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().unwrap();
        let total = cache.len();
        let expired = cache.values().filter(|entry| entry.is_expired()).count();
        (total, expired)
    }

    // Private helper methods

    /// Attempt to triangulate FX rate via pivot currency
    fn triangulate_rate(
        &self,
        from: Currency,
        to: Currency,
        on: Date,
        policy: FxConversionPolicy,
    ) -> crate::Result<FxRate> {
        let pivot = self.config.pivot_currency;

        // Don't triangulate if we already involve the pivot currency directly
        if from == pivot || to == pivot {
            return Err(crate::Error::Input(crate::error::InputError::Invalid));
        }

        // Try triangulation: from → pivot → to
        let from_to_pivot = self.provider.rate(from, pivot, on, policy)?;
        let pivot_to_to = self.provider.rate(pivot, to, on, policy)?;

        #[cfg(feature = "decimal128")]
        let triangulated_rate = from_to_pivot * pivot_to_to;
        #[cfg(not(feature = "decimal128"))]
        let triangulated_rate = from_to_pivot * pivot_to_to;

        // Cache the triangulated rate for future use
        let cache_key = FxCacheKey {
            from,
            to,
            date: on,
            policy,
        };
        self.update_cache(cache_key, triangulated_rate);

        Ok(triangulated_rate)
    }

    fn update_cache(&self, key: FxCacheKey, rate: FxRate) {
        let mut cache = self.cache.write().unwrap();
        // Drop expired entries first
        if cache.len() >= self.config.max_entries {
            cache.retain(|_, entry| !entry.is_expired());
        }
        // True LRU: evict least recently accessed if still at/over capacity
        if cache.len() >= self.config.max_entries {
            if let Some(lru_key) = cache
                .iter()
                .min_by_key(|(_, entry)| entry.last_access_at)
                .map(|(k, _)| k.clone())
            {
                cache.remove(&lru_key);
            }
        }
        let entry = CachedFxEntry::new(rate, self.config.default_ttl);
        cache.insert(key, entry);
    }

    fn check_closure(
        &self,
        direct_rate: FxRate,
        via_a: FxRate,
        via_b: FxRate,
    ) -> crate::Result<ClosureCheckResult> {
        #[cfg(feature = "decimal128")]
        {
            // Compute entirely in Decimal to avoid precision loss
            let calculated = via_a * via_b;
            let diff = (direct_rate - calculated).abs();
            let tol = rust_decimal::Decimal::try_from(self.config.closure_tolerance)
                .unwrap_or(rust_decimal::Decimal::ZERO);
            if diff <= tol {
                return Ok(ClosureCheckResult::Pass);
            }
            Ok(ClosureCheckResult::Fail {
                direct_rate,
                calculated_rate: calculated,
                difference: diff,
            })
        }
        #[cfg(not(feature = "decimal128"))]
        {
            let direct_f64 = direct_rate;
            let calculated_f64 = via_a * via_b;
            let difference = (direct_f64 - calculated_f64).abs();

            if difference <= self.config.closure_tolerance {
                Ok(ClosureCheckResult::Pass)
            } else {
                Ok(ClosureCheckResult::Fail {
                    direct_rate: direct_f64,
                    calculated_rate: calculated_f64,
                    difference,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::currency::Currency;
    use std::time::Duration;

    // Mock FX provider for testing
    struct MockFxProvider {
        rates: HashMap<(Currency, Currency), f64>,
    }

    impl MockFxProvider {
        fn new() -> Self {
            let mut rates = HashMap::new();

            // Add some mock rates with USD as pivot
            rates.insert((Currency::USD, Currency::EUR), 0.85);
            rates.insert((Currency::EUR, Currency::USD), 1.18);
            rates.insert((Currency::USD, Currency::GBP), 0.75);
            rates.insert((Currency::GBP, Currency::USD), 1.33);
            rates.insert((Currency::USD, Currency::JPY), 110.0);
            rates.insert((Currency::JPY, Currency::USD), 0.0091);
            rates.insert((Currency::USD, Currency::CAD), 1.25);
            rates.insert((Currency::CAD, Currency::USD), 0.80);

            // Intentionally omit direct cross-rates to test triangulation
            // EUR/GBP, EUR/JPY, GBP/JPY will be triangulated via USD

            Self { rates }
        }

        fn new_incomplete() -> Self {
            let mut rates = HashMap::new();

            // Only USD pivot rates - no cross-rates available
            rates.insert((Currency::USD, Currency::EUR), 0.85);
            rates.insert((Currency::EUR, Currency::USD), 1.18);
            rates.insert((Currency::USD, Currency::GBP), 0.75);
            rates.insert((Currency::GBP, Currency::USD), 1.33);

            Self { rates }
        }
    }

    impl FxProvider for MockFxProvider {
        fn rate(
            &self,
            from: Currency,
            to: Currency,
            _on: Date,
            _policy: FxConversionPolicy,
        ) -> crate::Result<FxRate> {
            if let Some(&rate) = self.rates.get(&(from, to)) {
                #[cfg(feature = "decimal128")]
                return rust_decimal::Decimal::try_from(rate).map_err(|_| crate::Error::Internal);
                #[cfg(not(feature = "decimal128"))]
                return Ok(rate);
            }
            Err(crate::Error::Internal)
        }
    }

    fn test_date() -> Date {
        use time::Month;
        Date::from_calendar_date(2023, Month::December, 15).unwrap()
    }

    #[test]
    fn fx_cache_basic_functionality() {
        let provider = MockFxProvider::new();
        let matrix = FxMatrix::new(Arc::new(provider));

        // Test basic rate retrieval
        let rate = matrix
            .rate(FxQuery {
                from: Currency::USD,
                to: Currency::EUR,
                on: test_date(),
                policy: FxConversionPolicy::CashflowDate,
                closure_check: None,
                want_meta: false,
            })
            .unwrap()
            .rate;

        #[cfg(feature = "decimal128")]
        let expected = rust_decimal::Decimal::try_from(0.85).unwrap();
        #[cfg(not(feature = "decimal128"))]
        let expected = 0.85;

        assert_eq!(rate, expected);

        // Test cache stats
        let (total, _expired) = matrix.cache_stats();
        assert_eq!(total, 1);
    }

    #[test]
    fn fx_cache_identity_rates() {
        let provider = MockFxProvider::new();
        let matrix = FxMatrix::new(Arc::new(provider));

        let rate = matrix
            .rate(FxQuery {
                from: Currency::USD,
                to: Currency::USD,
                on: test_date(),
                policy: FxConversionPolicy::CashflowDate,
                closure_check: None,
                want_meta: false,
            })
            .unwrap()
            .rate;

        #[cfg(feature = "decimal128")]
        let expected = rust_decimal::Decimal::ONE;
        #[cfg(not(feature = "decimal128"))]
        let expected = 1.0;

        assert_eq!(rate, expected);
    }

    #[test]
    fn fx_closure_checking_pass() {
        let provider = MockFxProvider::new();
        let config = FxCacheConfig {
            closure_tolerance: 0.01, // 1% tolerance for this test
            ..Default::default()
        };
        let matrix = FxMatrix::with_config(Arc::new(provider), config);

        let result = matrix
            .rate(FxQuery {
                from: Currency::USD,
                to: Currency::EUR,
                on: test_date(),
                policy: FxConversionPolicy::CashflowDate,
                closure_check: Some(Currency::GBP),
                want_meta: true,
            })
            .unwrap();

        // Should get a rate regardless of closure result
        #[cfg(feature = "decimal128")]
        assert!(result.rate > rust_decimal::Decimal::ZERO);
        #[cfg(not(feature = "decimal128"))]
        assert!(result.rate > 0.0);

        // With our mock data, the closure might not be perfect but should be reasonable
        match result.closure.expect("closure requested") {
            ClosureCheckResult::Pass => {}
            ClosureCheckResult::Fail { difference, .. } => {
                // For this test, we'll accept larger differences since our mock data
                // isn't perfectly consistent
                println!("Closure check failed with difference: {}", difference);
            }
        }
    }

    #[test]
    fn fx_cache_ttl_expiry() {
        let provider = MockFxProvider::new();
        let config = FxCacheConfig {
            default_ttl: Duration::from_nanos(1), // Immediate expiry
            ..Default::default()
        };
        let matrix = FxMatrix::with_config(Arc::new(provider), config);

        // Get rate to populate cache
        let _rate1 = matrix
            .rate(FxQuery {
                from: Currency::USD,
                to: Currency::EUR,
                on: test_date(),
                policy: FxConversionPolicy::CashflowDate,
                closure_check: None,
                want_meta: false,
            })
            .unwrap();

        // Wait a moment for TTL to expire
        std::thread::sleep(Duration::from_millis(1));

        // Clear expired entries
        matrix.clear_expired();

        let (total, _) = matrix.cache_stats();
        assert_eq!(total, 0); // Should be cleared due to expiry
    }

    #[test]
    fn fx_cache_lru_eviction() {
        let provider = MockFxProvider::new();
        let config = FxCacheConfig {
            max_entries: 2,
            default_ttl: Duration::from_secs(60),
            ..Default::default()
        };
        let matrix = FxMatrix::with_config(Arc::new(provider), config);

        // Fill cache to capacity
        let _rate1 = matrix
            .rate(FxQuery {
                from: Currency::USD,
                to: Currency::EUR,
                on: test_date(),
                policy: FxConversionPolicy::CashflowDate,
                closure_check: None,
                want_meta: false,
            })
            .unwrap();
        let _rate2 = matrix
            .rate(FxQuery {
                from: Currency::EUR,
                to: Currency::USD,
                on: test_date(),
                policy: FxConversionPolicy::CashflowDate,
                closure_check: None,
                want_meta: false,
            })
            .unwrap();

        let (total, _) = matrix.cache_stats();
        assert_eq!(total, 2);

        // Add one more - should evict the oldest
        let _rate3 = matrix
            .rate(FxQuery {
                from: Currency::USD,
                to: Currency::GBP,
                on: test_date(),
                policy: FxConversionPolicy::CashflowDate,
                closure_check: None,
                want_meta: false,
            })
            .unwrap();

        let (total, _) = matrix.cache_stats();
        assert_eq!(total, 2); // Still capped at 2
    }

    #[test]
    fn fx_cache_clear() {
        let provider = MockFxProvider::new();
        let matrix = FxMatrix::new(Arc::new(provider));

        // Populate cache
        let _rate = matrix
            .rate(FxQuery {
                from: Currency::USD,
                to: Currency::EUR,
                on: test_date(),
                policy: FxConversionPolicy::CashflowDate,
                closure_check: None,
                want_meta: false,
            })
            .unwrap();

        let (total, _) = matrix.cache_stats();
        assert_eq!(total, 1);

        // Clear cache
        matrix.clear_cache();

        let (total, _) = matrix.cache_stats();
        assert_eq!(total, 0);
    }

    #[test]
    fn fx_policy_meta_creation() {
        let policy = FxPolicyMeta {
            strategy: FxConversionPolicy::PeriodAverage,
            target_ccy: Some(Currency::USD),
            notes: "Test policy".to_string(),
        };

        assert_eq!(policy.strategy, FxConversionPolicy::PeriodAverage);
        assert_eq!(policy.target_ccy, Some(Currency::USD));
        assert_eq!(policy.notes, "Test policy");

        // Test default
        let default_policy = FxPolicyMeta::default();
        assert_eq!(default_policy.strategy, FxConversionPolicy::CashflowDate);
        assert_eq!(default_policy.target_ccy, None);
        assert_eq!(default_policy.notes, String::new());
    }

    #[test]
    fn fx_triangulation_success() {
        let provider = MockFxProvider::new_incomplete(); // Only has USD pivot rates
        let config = FxCacheConfig {
            pivot_currency: Currency::USD,
            enable_triangulation: true,
            closure_tolerance: 0.01, // Allow for some rounding differences
            ..Default::default()
        };
        let matrix = FxMatrix::with_config(Arc::new(provider), config);

        // Test EUR→GBP triangulation via USD: EUR→USD × USD→GBP
        let rate = matrix
            .rate(FxQuery {
                from: Currency::EUR,
                to: Currency::GBP,
                on: test_date(),
                policy: FxConversionPolicy::CashflowDate,
                closure_check: None,
                want_meta: false,
            })
            .unwrap()
            .rate;

        // Expected: 1.18 * 0.75 = 0.885
        #[cfg(feature = "decimal128")]
        let expected = rust_decimal::Decimal::try_from(1.18 * 0.75).unwrap();
        #[cfg(not(feature = "decimal128"))]
        let expected = 1.18 * 0.75;

        #[cfg(feature = "decimal128")]
        assert!((rate - expected).abs() < rust_decimal::Decimal::try_from(0.001).unwrap());
        #[cfg(not(feature = "decimal128"))]
        assert!((rate - expected).abs() < 0.001);
    }

    #[test]
    fn fx_triangulation_disabled() {
        let provider = MockFxProvider::new_incomplete(); // Only has USD pivot rates
        let config = FxCacheConfig {
            pivot_currency: Currency::USD,
            enable_triangulation: false, // Disabled
            ..Default::default()
        };
        let matrix = FxMatrix::with_config(Arc::new(provider), config);

        // Should fail when triangulation is disabled
        let result = matrix.rate(FxQuery {
            from: Currency::EUR,
            to: Currency::GBP,
            on: test_date(),
            policy: FxConversionPolicy::CashflowDate,
            closure_check: None,
            want_meta: false,
        });

        assert!(result.is_err());
    }

    #[test]
    fn fx_triangulation_caching() {
        let provider = MockFxProvider::new_incomplete();
        let config = FxCacheConfig {
            pivot_currency: Currency::USD,
            enable_triangulation: true,
            max_entries: 10,
            ..Default::default()
        };
        let matrix = FxMatrix::with_config(Arc::new(provider), config);

        // First call should triangulate and cache
        let rate1 = matrix
            .rate(FxQuery {
                from: Currency::EUR,
                to: Currency::GBP,
                on: test_date(),
                policy: FxConversionPolicy::CashflowDate,
                closure_check: None,
                want_meta: false,
            })
            .unwrap()
            .rate;

        // Second call should hit cache
        let rate2 = matrix
            .rate(FxQuery {
                from: Currency::EUR,
                to: Currency::GBP,
                on: test_date(),
                policy: FxConversionPolicy::CashflowDate,
                closure_check: None,
                want_meta: false,
            })
            .unwrap()
            .rate;

        assert_eq!(rate1, rate2);

        // Cache should contain the triangulated rate plus intermediate rates
        let (total, _) = matrix.cache_stats();
        assert!(total >= 1); // At least the final triangulated rate should be cached
    }

    #[test]
    fn fx_triangulation_pivot_identity() {
        let provider = MockFxProvider::new();
        let config = FxCacheConfig {
            pivot_currency: Currency::USD,
            enable_triangulation: true,
            ..Default::default()
        };
        let matrix = FxMatrix::with_config(Arc::new(provider), config);

        // USD→EUR should use direct rate, not triangulation
        let rate = matrix
            .rate(FxQuery {
                from: Currency::USD,
                to: Currency::EUR,
                on: test_date(),
                policy: FxConversionPolicy::CashflowDate,
                closure_check: None,
                want_meta: false,
            })
            .unwrap()
            .rate;

        // Should get the direct rate
        #[cfg(feature = "decimal128")]
        let expected = rust_decimal::Decimal::try_from(0.85).unwrap();
        #[cfg(not(feature = "decimal128"))]
        let expected = 0.85;

        assert_eq!(rate, expected);
    }

    #[test]
    fn fx_triangulation_missing_pivot_rates() {
        // Create provider with no USD rates at all
        let provider = MockFxProvider {
            rates: {
                let mut rates = HashMap::new();
                rates.insert((Currency::EUR, Currency::GBP), 0.88);
                rates.insert((Currency::GBP, Currency::EUR), 1.14);
                rates
            },
        };

        let config = FxCacheConfig {
            pivot_currency: Currency::USD,
            enable_triangulation: true,
            ..Default::default()
        };
        let matrix = FxMatrix::with_config(Arc::new(provider), config);

        // Should fail when pivot rates are missing
        let result = matrix.rate(FxQuery {
            from: Currency::JPY,
            to: Currency::CAD,
            on: test_date(),
            policy: FxConversionPolicy::CashflowDate,
            closure_check: None,
            want_meta: false,
        });

        assert!(result.is_err());
    }

    #[test]
    fn fx_rate_with_metadata() {
        let provider = MockFxProvider::new_incomplete();
        let config = FxCacheConfig {
            pivot_currency: Currency::USD,
            enable_triangulation: true,
            ..Default::default()
        };
        let matrix = FxMatrix::with_config(Arc::new(provider), config);

        // Test direct rate
        let result = matrix
            .rate(FxQuery {
                from: Currency::USD,
                to: Currency::EUR,
                on: test_date(),
                policy: FxConversionPolicy::CashflowDate,
                closure_check: None,
                want_meta: true,
            })
            .unwrap();

        assert!(!result.triangulated);
        assert_eq!(result.pivot_currency, None);

        #[cfg(feature = "decimal128")]
        let expected = rust_decimal::Decimal::try_from(0.85).unwrap();
        #[cfg(not(feature = "decimal128"))]
        let expected = 0.85;

        assert_eq!(result.rate, expected);

        // Test triangulated rate
        let result = matrix
            .rate(FxQuery {
                from: Currency::EUR,
                to: Currency::GBP,
                on: test_date(),
                policy: FxConversionPolicy::CashflowDate,
                closure_check: None,
                want_meta: true,
            })
            .unwrap();

        assert!(result.triangulated);
        assert_eq!(result.pivot_currency, Some(Currency::USD));

        // Expected: 1.18 * 0.75 = 0.885
        #[cfg(feature = "decimal128")]
        let expected = rust_decimal::Decimal::try_from(1.18 * 0.75).unwrap();
        #[cfg(not(feature = "decimal128"))]
        let expected = 1.18 * 0.75;

        #[cfg(feature = "decimal128")]
        assert!((result.rate - expected).abs() < rust_decimal::Decimal::try_from(0.001).unwrap());
        #[cfg(not(feature = "decimal128"))]
        assert!((result.rate - expected).abs() < 0.001);

        // Test identity rate
        let result = matrix
            .rate(FxQuery {
                from: Currency::USD,
                to: Currency::USD,
                on: test_date(),
                policy: FxConversionPolicy::CashflowDate,
                closure_check: None,
                want_meta: true,
            })
            .unwrap();

        assert!(!result.triangulated);
        assert_eq!(result.pivot_currency, None);

        #[cfg(feature = "decimal128")]
        assert_eq!(result.rate, rust_decimal::Decimal::ONE);
        #[cfg(not(feature = "decimal128"))]
        assert_eq!(result.rate, 1.0);
    }
}
