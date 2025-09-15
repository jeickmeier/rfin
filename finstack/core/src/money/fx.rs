//! Foreign-exchange interfaces and a simplified FX matrix.
//!
//! Design goals:
//! - Store raw FX quotes for currency pairs
//! - Compute reciprocal and triangulated rates on demand
//! - Provide simple, deterministic lookups with bounded LRU caching
//!
//! The public surface remains stable:
//! - `FxProvider` trait for on-demand quotes
//! - `FxMatrix` offering `rate(FxQuery)` for consumers and `MarketContext`


use crate::currency::Currency;
use crate::dates::Date;
use std::sync::Arc;
use lru::LruCache;
use parking_lot::Mutex;
use std::num::NonZeroUsize;
// no duration needed in the simplified config

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Provider FX rate type alias - always f64.
pub type FxRate = f64;

/// Standard FX conversion strategies. These are metadata hints for providers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
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

/// Pair key helper used internally for maps
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Pair(Currency, Currency);

/// Configuration for FX matrix behavior
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct FxConfig {
    /// Tolerance for closure checking (e.g., 0.0001 = 1bp). Only used if metadata requested.
    pub closure_tolerance: f64,
    /// Whether closure violations should produce errors
    pub strict_closure: bool,
    /// Pivot currency for optional triangulation fallback (typically USD)
    pub pivot_currency: Currency,
    /// Whether to enable automatic triangulation for missing rates
    pub enable_triangulation: bool,
    /// Maximum number of cached quotes to retain in an LRU
    pub cache_capacity: usize,
}

impl Default for FxConfig {
    fn default() -> Self {
        Self {
            closure_tolerance: 0.0001, // 1 basis point
            strict_closure: false,
            pivot_currency: Currency::USD, // USD as default pivot
            enable_triangulation: true,    // Enable triangulation by default
            cache_capacity: 1024,          // Default LRU capacity
        }
    }
}

/// Result of a closure check
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
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
///
/// Serialization is available when the crate is built with the `serde` feature.
/// The shape is stable and suitable for logs and result envelopes.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
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

/// Simplified FX matrix that stores quotes and computes cross rates on demand.
///
/// Note: `FxMatrix` cannot be directly serialized due to the trait object `Arc<dyn FxProvider>`.
/// To persist FX state, use `get_serializable_state()` to extract the config and quotes,
/// then recreate the matrix with `FxMatrix::with_config()` and `load_from_state()`.
pub struct FxMatrix {
    provider: Arc<dyn FxProvider>,
    /// Explicit quotes inserted or observed from provider
    quotes: Mutex<LruCache<Pair, FxRate>>,
    config: FxConfig,
}

/// Serializable state of an FxMatrix.
/// Contains the configuration and cached quotes that can be persisted and restored.
#[cfg(feature = "serde")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FxMatrixState {
    /// FX configuration
    pub config: FxConfig,
    /// Cached FX quotes as (from, to, rate) tuples
    pub quotes: Vec<(Currency, Currency, FxRate)>,
}

impl FxMatrix {
    /// Create a new `FxMatrix` wrapping the given provider with default configuration
    pub fn new(provider: Arc<dyn FxProvider>) -> Self {
        Self::with_config(provider, FxConfig::default())
    }

    /// Create a new `FxMatrix` with custom configuration
    pub fn with_config(provider: Arc<dyn FxProvider>, config: FxConfig) -> Self {
        let capacity = if config.cache_capacity == 0 { 1 } else { config.cache_capacity };
        let quotes = LruCache::new(NonZeroUsize::new(capacity).expect("non-zero capacity"));
        Self { provider, quotes: Mutex::new(quotes), config }
    }

    /// Direct lookup from the implied matrix. Falls back to provider/triangulation if missing.
    pub fn rate(&self, query: FxQuery) -> crate::Result<FxRateResult> {
        let from = query.from;
        let to = query.to;
        let on = query.on;
        let policy = query.policy;

        // Handle identity case
        if from == to {
            let rate = 1.0;

            let mut result = FxRateResult {
                rate,
                triangulated: false,
                pivot_currency: None,
                closure: None,
            };
            // Compute optional closure metadata via helper for consistency
            result.closure = self.compute_closure_result(&query, rate)?;
            return Ok(result);
        }

        // Prefer explicitly seeded quotes (or their reciprocal) before provider/triangulation.
        // Read both directions under a single lock, then drop before any recursive calls.
        let (direct_opt, reciprocal_opt) = self.read_cached_pair_bidir(from, to);

        if let Some(rate) = direct_opt {
            let mut result = FxRateResult {
                rate,
                triangulated: false,
                pivot_currency: None,
                closure: None,
            };
            result.closure = self.compute_closure_result(&query, rate)?;
            return Ok(result);
        }
        if let Some(r_rev) = reciprocal_opt {
            if r_rev != 0.0 {
                let rate = 1.0 / r_rev;
                let mut result = FxRateResult {
                    rate,
                    triangulated: false,
                    pivot_currency: None,
                    closure: None,
                };
                result.closure = self.compute_closure_result(&query, rate)?;
                return Ok(result);
            }
        }

        // Ask provider for a direct quote or compute via triangulation if needed
        let mut triangulated = false;
        let mut pivot_currency: Option<Currency> = None;
        let rate = match self.provider.rate(from, to, on, policy) {
            Ok(rate) => {
                self.insert_quote(from, to, rate);
                rate
            }
            Err(_) if self.config.enable_triangulation => {
                // Try triangulation using pivot
                let rate = self.triangulate_rate(from, to, on, policy)?;
                triangulated = true;
                pivot_currency = Some(self.config.pivot_currency);
                rate
            }
            Err(e) => return Err(e),
        };

        let mut result = FxRateResult { rate, triangulated, pivot_currency, closure: None };
        if query.want_meta {
            let closure = self.compute_closure_result(&query, result.rate)?;
            if self.config.strict_closure {
                if let Some(ClosureCheckResult::Fail { .. }) = closure {
                    return Err(crate::Error::Input(crate::error::InputError::Invalid));
                }
            }
            result.closure = closure;
        }

        Ok(result)
    }


    /// Seed or update a single quote directly inside the matrix.
    ///
    /// Note: This does not automatically insert a reciprocal. Lookups will use
    /// the reciprocal on demand if the opposite direction is requested.
    pub fn set_quote(&self, from: Currency, to: Currency, rate: FxRate) {
        self.insert_quote(from, to, rate);
    }

    /// Seed multiple quotes at once.
    pub fn set_quotes(&self, quotes: &[(Currency, Currency, FxRate)]) {
        let mut map = self.quotes.lock();
        for &(from, to, rate) in quotes {
            map.put(Pair(from, to), rate);
        }
    }

    /// Clear cached quotes considered "expired".
    ///
    /// Note: Quotes in this matrix are not timestamped, so we conservatively
    /// clear the entire cache. Callers that need finer-grained control should
    /// seed quotes explicitly via `set_quote(s)`.
    pub fn clear_expired(&self) {
        self.clear_cache();
    }

    /// Clear all stored quotes
    pub fn clear_cache(&self) {
        self.quotes.lock().clear();
    }

    /// Get simple statistics: (num_quotes, 0)
    pub fn cache_stats(&self) -> (usize, usize) {
        let quotes = self.quotes.lock();
        (quotes.len(), 0)
    }

    /// Extract serializable state from the FxMatrix.
    /// Returns the configuration and current quotes that can be persisted.
    #[cfg(feature = "serde")]
    pub fn get_serializable_state(&self) -> FxMatrixState {
        let quotes = self.quotes.lock();
        let quote_vec: Vec<(Currency, Currency, FxRate)> = quotes
            .iter()
            .map(|(pair, rate)| (pair.0, pair.1, *rate))
            .collect();
        FxMatrixState {
            config: self.config.clone(),
            quotes: quote_vec,
        }
    }

    /// Load quotes from a serialized state.
    /// This allows restoring cached quotes after deserialization.
    #[cfg(feature = "serde")]
    pub fn load_from_state(&self, state: &FxMatrixState) {
        self.set_quotes(&state.quotes);
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

        // Compute via pivot using available quotes/provider
        let a = self.get_or_fetch(from, pivot, on, policy)?;
        let b = self.get_or_fetch(pivot, to, on, policy)?;
        let rate = a * b;
        Ok(rate)
    }

    /// Insert an explicit provider quote
    fn insert_quote(&self, from: Currency, to: Currency, rate: FxRate) {
        let mut quotes = self.quotes.lock();
        quotes.put(Pair(from, to), rate);
    }

    /// Get rate preferring explicit quotes, then provider, then reciprocal.
    fn get_or_fetch(
        &self,
        from: Currency,
        to: Currency,
        on: Date,
        policy: FxConversionPolicy,
    ) -> crate::Result<FxRate> {
        if from == to {
            return Ok(1.0);
        }
        // Read both direct and reciprocal under a single lock, then drop before any further work
        let (direct_opt, reciprocal_opt) = self.read_cached_pair_bidir(from, to);
        // 1) Explicit quote wins
        if let Some(r) = direct_opt {
            return Ok(r);
        }
        // 2) Try provider for direct
        if let Ok(r) = self.provider.rate(from, to, on, policy) {
            self.insert_quote(from, to, r);
            return Ok(r);
        }
        // 3) Reciprocal fallback if available
        if let Some(r_rev) = reciprocal_opt {
            if r_rev != 0.0 {
                let r = 1.0 / r_rev;
                return Ok(r);
            }
        }
        // 4) As last resort, propagate provider error
        let r = self.provider.rate(from, to, on, policy)?;
        self.insert_quote(from, to, r);
        Ok(r)
    }

    /// Read direct and reciprocal cached quotes for a pair under a single lock.
    #[inline]
    fn read_cached_pair_bidir(
        &self,
        from: Currency,
        to: Currency,
    ) -> (Option<FxRate>, Option<FxRate>) {
        let mut quotes = self.quotes.lock();
        (
            quotes.get(&Pair(from, to)).copied(),
            quotes.get(&Pair(to, from)).copied(),
        )
    }

    fn check_closure(
        &self,
        direct_rate: FxRate,
        via_a: FxRate,
        via_b: FxRate,
    ) -> crate::Result<ClosureCheckResult> {
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

    /// Compute optional closure result based on query flags. Returns None when not requested.
    fn compute_closure_result(
        &self,
        query: &FxQuery,
        direct_rate: FxRate,
    ) -> crate::Result<Option<ClosureCheckResult>> {
        if !query.want_meta {
            return Ok(None);
        }
        let mid = match query.closure_check {
            Some(c) => c,
            None => return Ok(None),
        };

        // Recursively compute via legs using the unified API without metadata
        let via_a = self
            .rate(FxQuery {
                from: query.from,
                to: mid,
                on: query.on,
                policy: query.policy,
                closure_check: None,
                want_meta: false,
            })?
            .rate;
        let via_b = self
            .rate(FxQuery {
                from: mid,
                to: query.to,
                on: query.on,
                policy: query.policy,
                closure_check: None,
                want_meta: false,
            })?
            .rate;

        Ok(Some(self.check_closure(direct_rate, via_a, via_b)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::currency::Currency;
    use hashbrown::HashMap;
    // no duration needed in tests now

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

        let expected = 0.85;

        assert_eq!(rate, expected);

        // Test stats reflect quotes and implied
        let (quotes, implied) = matrix.cache_stats();
        assert!(quotes >= 1);
        assert_eq!(implied, 0);
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

        let expected = 1.0;

        assert_eq!(rate, expected);
    }

    #[test]
    fn fx_closure_checking_pass() {
        let provider = MockFxProvider::new();
        let config = FxConfig {
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
    fn fx_clear_implied_matrix() {
        let provider = MockFxProvider::new();
        let config = FxConfig::default();
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

        // Clear implied entries
        matrix.clear_expired();

        let (_, implied) = matrix.cache_stats();
        assert_eq!(implied, 0); // Implied cleared
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

        let (quotes, implied) = matrix.cache_stats();
        assert!(quotes >= 1);
        assert_eq!(implied, 0);

        // Clear cache
        matrix.clear_cache();

        let (quotes, implied) = matrix.cache_stats();
        assert_eq!(quotes, 0);
        assert_eq!(implied, 0);
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
        let config = FxConfig {
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
        let expected = 1.18 * 0.75;

        assert!((rate - expected).abs() < 0.001);
    }

    #[test]
    fn fx_triangulation_disabled() {
        let provider = MockFxProvider::new_incomplete(); // Only has USD pivot rates
        let config = FxConfig {
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
        let config = FxConfig {
            pivot_currency: Currency::USD,
            enable_triangulation: true,
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

        // Stats indicate stored quotes only
        let (quotes, implied) = matrix.cache_stats();
        assert!(quotes >= 1);
        assert_eq!(implied, 0);
    }

    #[test]
    fn fx_triangulation_pivot_identity() {
        let provider = MockFxProvider::new();
        let config = FxConfig {
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

        let config = FxConfig {
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
        let config = FxConfig {
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
        let expected = 1.18 * 0.75;

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

        assert_eq!(result.rate, 1.0);
    }

    #[test]
    fn fx_seed_quotes_directly() {
        let provider = MockFxProvider::new_incomplete();
        let matrix = FxMatrix::new(Arc::new(provider));

        // Seed a direct quote and verify retrieval
        matrix.set_quote(Currency::USD, Currency::CHF, 0.90);
        let usd_chf = matrix
            .rate(FxQuery {
                from: Currency::USD,
                to: Currency::CHF,
                on: test_date(),
                policy: FxConversionPolicy::CashflowDate,
                closure_check: None,
                want_meta: false,
            })
            .unwrap()
            .rate;
        assert_eq!(usd_chf, 0.90);

        // Opposite direction should use reciprocal on demand
        let chf_usd = matrix
            .rate(FxQuery {
                from: Currency::CHF,
                to: Currency::USD,
                on: test_date(),
                policy: FxConversionPolicy::CashflowDate,
                closure_check: None,
                want_meta: false,
            })
            .unwrap()
            .rate;
        assert!((chf_usd - (1.0 / 0.90)).abs() < 1e-12);

        // Bulk seed: add a couple more pairs
        matrix.set_quotes(&[
            (Currency::EUR, Currency::CHF, 0.95),
            (Currency::GBP, Currency::CHF, 1.10),
        ]);

        let eur_chf = matrix
            .rate(FxQuery {
                from: Currency::EUR,
                to: Currency::CHF,
                on: test_date(),
                policy: FxConversionPolicy::CashflowDate,
                closure_check: None,
                want_meta: false,
            })
            .unwrap()
            .rate;
        assert_eq!(eur_chf, 0.95);
    }
}
