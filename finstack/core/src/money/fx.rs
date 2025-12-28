//! Foreign-exchange interfaces and a simplified FX matrix.
//!
//! Design goals:
//! - store raw FX quotes for currency pairs
//! - compute reciprocal and triangulated rates on demand
//! - provide deterministic lookups with bounded LRU caching
//!
//! The public surface remains stable:
//! - `FxProvider` trait for on-demand quotes
//! - `FxMatrix` offering `FxMatrix::rate` for consumers and `MarketContext`
//! - `providers` module with standard provider implementations
//!
//! # Examples
//! ```rust
//! use finstack_core::money::fx::{FxConversionPolicy, FxMatrix, FxProvider, FxQuery};
//! use finstack_core::money::fx::providers::SimpleFxProvider;
//! use finstack_core::currency::Currency;
//! use finstack_core::dates::Date;
//! use std::sync::Arc;
//! use time::Month;
//!
//! let provider = Arc::new(SimpleFxProvider::new());
//! provider.set_quote(Currency::EUR, Currency::USD, 1.1);
//!
//! let matrix = FxMatrix::new(provider.clone());
//! let date = Date::from_calendar_date(2024, Month::January, 5).expect("Valid date");
//! let res = matrix.rate(FxQuery::new(Currency::EUR, Currency::USD, date)).expect("FX rate lookup should succeed");
//! assert_eq!(res.rate, 1.1);
//! ```

/// Standard FX provider implementations.
pub mod providers;

use crate::currency::Currency;
use crate::dates::Date;
use lru::LruCache;
use parking_lot::Mutex;
use std::num::NonZeroUsize;
use std::sync::Arc;
// no duration needed in the simplified config

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Provider FX rate type alias - always f64.
pub type FxRate = f64;

/// Helper to compute reciprocal rate safely, checking for zero division.
///
/// Returns `1.0 / rate` if `rate != 0.0`, otherwise returns an error.
/// This consolidates the reciprocal logic used across FX providers and matrix lookups.
#[inline]
pub(crate) fn reciprocal_rate_or_err(
    rate: f64,
    from: Currency,
    to: Currency,
) -> crate::Result<f64> {
    if !rate.is_finite() {
        return Err(crate::error::InputError::NonFiniteValue {
            kind: if rate.is_nan() {
                "NaN".to_string()
            } else {
                "infinity".to_string()
            },
        }
        .into());
    }
    if rate != 0.0 {
        Ok(1.0 / rate)
    } else {
        Err(crate::error::InputError::NotFound {
            id: format!("FX:{from}->{to} (zero reciprocal)"),
        }
        .into())
    }
}

/// Standard FX conversion strategies used to hint FX providers.
///
/// The policy tells a provider *how* the rate will be applied so it can decide
/// between spot, forward, or averaged sources.
///
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

/// Simple FX rate query.
///
/// Contains only the essential parameters for currency conversion.
///
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
    /// Conversion policy (defaults to CashflowDate)
    #[cfg_attr(feature = "serde", serde(default = "default_policy"))]
    pub policy: FxConversionPolicy,
}

#[cfg(feature = "serde")]
fn default_policy() -> FxConversionPolicy {
    FxConversionPolicy::CashflowDate
}

impl FxQuery {
    /// Create a new FX query with default policy.
    pub fn new(from: Currency, to: Currency, on: Date) -> Self {
        Self {
            from,
            to,
            on,
            policy: FxConversionPolicy::CashflowDate,
        }
    }

    /// Create a new FX query with specific policy.
    pub fn with_policy(from: Currency, to: Currency, on: Date, policy: FxConversionPolicy) -> Self {
        Self {
            from,
            to,
            on,
            policy,
        }
    }
}

/// Metadata describing the policy applied by the provider.
///
/// Attach [`FxPolicyMeta`] to valuation results so auditors can understand how
/// FX conversions were sourced.
///
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

/// Configuration for [`FxMatrix`] behaviour.
///
/// Controls triangulation and caching.
///
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct FxConfig {
    /// Pivot currency for triangulation fallback (typically USD)
    pub pivot_currency: Currency,
    /// Whether to enable automatic triangulation for missing rates
    pub enable_triangulation: bool,
    /// Maximum number of cached quotes to retain in an LRU
    pub cache_capacity: usize,
}

impl Default for FxConfig {
    fn default() -> Self {
        Self {
            pivot_currency: Currency::USD,
            enable_triangulation: false, // Disabled by default - simpler
            cache_capacity: 256,         // Smaller cache - simpler
        }
    }
}

/// Result of an FX rate lookup with simple triangulation info.
///
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FxRateResult {
    /// The final FX rate
    pub rate: FxRate,
    /// Whether this rate was obtained via triangulation
    pub triangulated: bool,
}

/// Trait for obtaining FX rates.
///
/// Implementations can be as simple as hard-coded tables or as complex as
/// feed handlers. Providers should respect the supplied
/// [`FxConversionPolicy`].
///
/// # Examples
/// ```rust
/// use finstack_core::money::fx::{FxConversionPolicy, FxProvider};
/// use finstack_core::currency::Currency;
/// use finstack_core::dates::Date;
/// use time::Month;
///
/// struct StaticFx;
/// impl FxProvider for StaticFx {
///     fn rate(
///         &self,
///         _from: Currency,
///         _to: Currency,
///         _on: Date,
///         _policy: FxConversionPolicy,
///     ) -> finstack_core::Result<f64> {
///         Ok(1.25)
///     }
/// }
///
/// let trade_date = Date::from_calendar_date(2024, Month::January, 10).expect("Valid date");
/// let quote = StaticFx.rate(
///     Currency::EUR,
///     Currency::USD,
///     trade_date,
///     FxConversionPolicy::CashflowDate,
/// ).expect("FX rate lookup should succeed");
/// assert_eq!(quote, 1.25);
/// ```
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
/// Note: `FxMatrix` cannot be directly serialized due to the trait object
/// `Arc<dyn FxProvider>`. To persist FX state, use
/// [`FxMatrix::get_serializable_state`] to extract the config and quotes, then
/// recreate the matrix with [`FxMatrix::with_config`] and [`FxMatrix::load_from_state`].
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
    /// Create a new [`FxMatrix`] wrapping the given provider with the default configuration.
    ///
    /// # Parameters
    /// - `provider`: FX quote source implementing [`FxProvider`]
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::money::fx::{FxMatrix, FxProvider, FxConversionPolicy};
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::Date;
    /// use std::sync::Arc;
    /// use time::Month;
    ///
    /// struct StaticFx;
    /// impl FxProvider for StaticFx {
    ///     fn rate(
    ///         &self,
    ///         _from: Currency,
    ///         _to: Currency,
    ///         _on: Date,
    ///         _policy: FxConversionPolicy,
    ///     ) -> finstack_core::Result<f64> {
    ///         Ok(1.0)
    ///     }
    /// }
    ///
    /// let matrix = FxMatrix::new(Arc::new(StaticFx));
    /// assert_eq!(matrix.cache_stats(), 0);
    /// ```
    pub fn new(provider: Arc<dyn FxProvider>) -> Self {
        Self::with_config(provider, FxConfig::default())
    }

    /// Create a new [`FxMatrix`] with custom configuration.
    ///
    /// # Parameters
    /// - `provider`: FX quote source implementing [`FxProvider`]
    /// - `config`: tuning knobs controlling cache size and triangulation behaviour
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::money::fx::{FxConfig, FxMatrix, FxProvider, FxConversionPolicy};
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::Date;
    /// use std::sync::Arc;
    /// use time::Month;
    ///
    /// struct StaticFx;
    /// impl FxProvider for StaticFx {
    ///     fn rate(
    ///         &self,
    ///         _from: Currency,
    ///         _to: Currency,
    ///         _on: Date,
    ///         _policy: FxConversionPolicy,
    ///     ) -> finstack_core::Result<f64> {
    ///         Ok(1.0)
    ///     }
    /// }
    ///
    /// let mut cfg = FxConfig::default();
    /// cfg.cache_capacity = 128;
    /// let matrix = FxMatrix::with_config(Arc::new(StaticFx), cfg);
    /// assert_eq!(matrix.cache_stats(), 0);
    /// ```
    #[allow(clippy::expect_used)] // Capacity is set to at least 1 above, so always non-zero
    pub fn with_config(provider: Arc<dyn FxProvider>, config: FxConfig) -> Self {
        let capacity = if config.cache_capacity == 0 {
            1
        } else {
            config.cache_capacity
        };
        let quotes = LruCache::new(NonZeroUsize::new(capacity).expect("non-zero capacity"));
        Self {
            provider,
            quotes: Mutex::new(quotes),
            config,
        }
    }

    /// Access the underlying FX provider reference.
    pub fn provider(&self) -> Arc<dyn FxProvider> {
        Arc::clone(&self.provider)
    }

    /// Return the matrix configuration.
    pub fn config(&self) -> FxConfig {
        self.config
    }

    /// Look up an FX rate (with metadata) using caching and triangulation fallbacks.
    ///
    /// # Parameters
    /// - `query`: [`FxQuery`] describing the desired conversion
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::money::fx::{FxMatrix, FxProvider, FxConversionPolicy, FxQuery};
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::Date;
    /// use std::sync::Arc;
    /// use time::Month;
    ///
    /// struct StaticFx;
    /// impl FxProvider for StaticFx {
    ///     fn rate(
    ///         &self,
    ///         _from: Currency,
    ///         _to: Currency,
    ///         _on: Date,
    ///         _policy: FxConversionPolicy,
    ///     ) -> finstack_core::Result<f64> {
    ///         Ok(1.1)
    ///     }
    /// }
    ///
    /// let matrix = FxMatrix::new(Arc::new(StaticFx));
    /// let query = FxQuery::new(
    ///     Currency::EUR,
    ///     Currency::USD,
    ///     Date::from_calendar_date(2024, Month::March, 1).expect("Valid date"),
    /// );
    /// let result = matrix.rate(query).expect("FX rate lookup should succeed");
    /// assert!(result.rate > 1.0);
    /// ```
    pub fn rate(&self, query: FxQuery) -> crate::Result<FxRateResult> {
        let from = query.from;
        let to = query.to;
        let on = query.on;
        let policy = query.policy;

        // Handle identity case
        if from == to {
            return Ok(FxRateResult {
                rate: 1.0,
                triangulated: false,
            });
        }

        // Check cache first (both directions)
        let (direct_opt, reciprocal_opt) = self.read_cached_pair_bidir(from, to);

        if let Some(rate) = direct_opt {
            return Ok(FxRateResult {
                rate,
                triangulated: false,
            });
        }
        if let Some(r_rev) = reciprocal_opt {
            return Ok(FxRateResult {
                rate: reciprocal_rate_or_err(r_rev, to, from)?,
                triangulated: false,
            });
        }

        // Try provider first
        match self.provider.rate(from, to, on, policy) {
            Ok(rate) => {
                self.insert_quote(from, to, rate);
                Ok(FxRateResult {
                    rate,
                    triangulated: false,
                })
            }
            Err(_) if self.config.enable_triangulation => {
                // Try simple triangulation via pivot
                let rate = self.triangulate_rate(from, to, on, policy)?;
                Ok(FxRateResult {
                    rate,
                    triangulated: true,
                })
            }
            Err(e) => Err(e),
        }
    }

    /// Seed or update a single quote directly inside the matrix.
    ///
    /// Note: This does not automatically insert a reciprocal. Lookups will use
    /// the reciprocal on demand if the opposite direction is requested.
    ///
    /// # Parameters
    /// - `from`: base currency for the quote
    /// - `to`: quote currency
    /// - `rate`: raw FX rate (`from → to`)
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::money::fx::{FxMatrix, FxProvider, FxConversionPolicy, FxQuery};
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::Date;
    /// use std::sync::Arc;
    /// use time::Month;
    ///
    /// struct StaticFx;
    /// impl FxProvider for StaticFx {
    ///     fn rate(
    ///         &self,
    ///         _from: Currency,
    ///         _to: Currency,
    ///         _on: Date,
    ///         _policy: FxConversionPolicy,
    ///     ) -> finstack_core::Result<f64> {
    ///         Ok(1.2)
    ///     }
    /// }
    ///
    /// let matrix = FxMatrix::new(Arc::new(StaticFx));
    /// matrix.set_quote(Currency::GBP, Currency::USD, 1.3);
    /// let res = matrix.rate(FxQuery::new(
    ///     Currency::GBP,
    ///     Currency::USD,
    ///     Date::from_calendar_date(2024, Month::April, 1).expect("Valid date"),
    /// )).expect("FX rate lookup should succeed");
    /// assert_eq!(res.rate, 1.3);
    /// ```
    pub fn set_quote(&self, from: Currency, to: Currency, rate: FxRate) {
        self.insert_quote(from, to, rate);
    }

    /// Seed multiple quotes at once.
    ///
    /// # Parameters
    /// - `quotes`: slice of `(from, to, rate)` tuples
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
    /// seed quotes explicitly via [`FxMatrix::set_quote`].
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::money::fx::{FxConversionPolicy, FxMatrix, FxProvider};
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::dates::Date;
    /// # use std::sync::Arc;
    /// # use time::Month;
    /// # struct StaticFx;
    /// # impl FxProvider for StaticFx {
    /// #     fn rate(&self, _from: Currency, _to: Currency, _on: Date, _policy: FxConversionPolicy)
    /// #         -> finstack_core::Result<f64> { Ok(1.0) }
    /// # }
    /// let matrix = FxMatrix::new(Arc::new(StaticFx));
    /// matrix.clear_expired();
    /// ```
    pub fn clear_expired(&self) {
        self.clear_cache();
    }

    /// Clear all stored quotes.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::money::fx::{FxConversionPolicy, FxMatrix, FxProvider};
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::dates::Date;
    /// # use std::sync::Arc;
    /// # use time::Month;
    /// # struct StaticFx;
    /// # impl FxProvider for StaticFx {
    /// #     fn rate(&self, _from: Currency, _to: Currency, _on: Date, _policy: FxConversionPolicy)
    /// #         -> finstack_core::Result<f64> { Ok(1.0) }
    /// # }
    /// let matrix = FxMatrix::new(Arc::new(StaticFx));
    /// matrix.clear_cache();
    /// ```
    pub fn clear_cache(&self) {
        self.quotes.lock().clear();
    }

    /// Return cached quote count for quick diagnostics.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::money::fx::{FxConversionPolicy, FxMatrix, FxProvider};
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::dates::Date;
    /// # use std::sync::Arc;
    /// # use time::Month;
    /// # struct StaticFx;
    /// # impl FxProvider for StaticFx {
    /// #     fn rate(&self, _from: Currency, _to: Currency, _on: Date, _policy: FxConversionPolicy)
    /// #         -> finstack_core::Result<f64> { Ok(1.0) }
    /// # }
    /// let matrix = FxMatrix::new(Arc::new(StaticFx));
    /// assert_eq!(matrix.cache_stats(), 0);
    /// ```
    pub fn cache_stats(&self) -> usize {
        let quotes = self.quotes.lock();
        quotes.len()
    }

    /// Extract serializable state from the matrix.
    ///
    /// Returns the configuration and current quotes that can be persisted.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::money::fx::{FxMatrix, FxProvider, FxConversionPolicy};
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::dates::Date;
    /// # use std::sync::Arc;
    /// # use time::Month;
    /// # struct StaticFx;
    /// # impl FxProvider for StaticFx {
    /// #     fn rate(&self, _from: Currency, _to: Currency, _on: Date, _policy: FxConversionPolicy)
    /// #         -> finstack_core::Result<f64> { Ok(1.0) }
    /// # }
    /// let matrix = FxMatrix::new(Arc::new(StaticFx));
    /// # #[cfg(feature = "serde")]
    /// # {
    /// let state = matrix.get_serializable_state();
    /// assert!(state.quotes.is_empty());
    /// # }
    /// ```
    #[cfg(feature = "serde")]
    pub fn get_serializable_state(&self) -> FxMatrixState {
        let quotes = self.quotes.lock();
        let quote_vec: Vec<(Currency, Currency, FxRate)> = quotes
            .iter()
            .map(|(pair, rate)| (pair.0, pair.1, *rate))
            .collect();
        FxMatrixState {
            config: self.config,
            quotes: quote_vec,
        }
    }

    /// Load quotes from a serialized state.
    ///
    /// This allows restoring cached quotes after deserialization.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::money::fx::{FxMatrix, FxProvider, FxConversionPolicy, FxMatrixState};
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::dates::Date;
    /// # use std::sync::Arc;
    /// # use time::Month;
    /// # struct StaticFx;
    /// # impl FxProvider for StaticFx {
    /// #     fn rate(&self, _from: Currency, _to: Currency, _on: Date, _policy: FxConversionPolicy)
    /// #         -> finstack_core::Result<f64> { Ok(1.0) }
    /// # }
    /// # #[cfg(feature = "serde")]
    /// # {
    /// let matrix = FxMatrix::new(Arc::new(StaticFx));
    /// let state = FxMatrixState { config: matrix.get_serializable_state().config, quotes: vec![] };
    /// matrix.load_from_state(&state);
    /// # }
    /// ```
    #[cfg(feature = "serde")]
    pub fn load_from_state(&self, state: &FxMatrixState) {
        self.set_quotes(&state.quotes);
    }

    /// Create a new FX matrix with a bumped rate for a specific currency pair.
    ///
    /// This is useful for finite difference greek calculations where we need
    /// to bump FX spot while preserving all other market data. Creates a
    /// wrapper provider that overrides the specified rate.
    ///
    /// # Parameters
    /// - `from`: Base currency
    /// - `to`: Quote currency
    /// - `bump_pct`: Relative bump size (e.g., 0.01 for 1% increase)
    /// - `on`: Date for rate lookup (typically as_of date from valuation context)
    ///
    /// # Returns
    /// New FxMatrix with bumped rate
    ///
    /// # Errors
    /// Returns error if rate lookup fails
    pub fn with_bumped_rate(
        &self,
        from: Currency,
        to: Currency,
        bump_pct: f64,
        on: Date,
    ) -> crate::Result<Self> {
        // Get current rate
        let query = FxQuery::new(from, to, on);
        let current_rate = self.rate(query)?.rate;

        // Calculate bumped rate
        let bumped_rate = current_rate * (1.0 + bump_pct);

        // Create bumped provider
        use providers::BumpedFxProvider;
        use std::sync::Arc;
        let bumped_provider = Arc::new(BumpedFxProvider::new(
            self.provider.clone(),
            from,
            to,
            bumped_rate,
        ));

        // Create new FX matrix with same config and carry over cached quotes so lookups that
        // rely on seeded values keep working after the bump.
        let bumped = Self::with_config(bumped_provider, self.config);
        {
            let src = self.quotes.lock();
            let mut dst = bumped.quotes.lock();
            for (pair, rate) in src.iter() {
                dst.put(*pair, *rate);
            }
        }
        // IMPORTANT: ensure the bumped pair is not shadowed by copied cached quotes.
        // `FxMatrix::rate` consults the quote cache before consulting the provider.
        // Without overwriting the bumped pair here, callers that queried the original
        // matrix first would keep seeing the stale cached quote instead of the bumped quote.
        bumped.set_quote(from, to, bumped_rate);

        Ok(bumped)
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
        // Cache derived cross to avoid repeated recomputation
        self.insert_quote(from, to, rate);
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
            return reciprocal_rate_or_err(r_rev, to, from);
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
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::collections::HashMap;
    use crate::currency::Currency;
    // no duration needed in tests now

    // Mock FX provider for testing
    struct MockFxProvider {
        rates: HashMap<(Currency, Currency), f64>,
    }

    impl MockFxProvider {
        fn new() -> Self {
            let mut rates = HashMap::default();

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
            let mut rates = HashMap::default();

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
        Date::from_calendar_date(2023, Month::December, 15).expect("Valid test date")
    }

    #[test]
    fn fx_cache_basic_functionality() {
        let provider = MockFxProvider::new();
        let matrix = FxMatrix::new(Arc::new(provider));

        // Test basic rate retrieval
        let rate = matrix
            .rate(FxQuery::new(Currency::USD, Currency::EUR, test_date()))
            .expect("FX rate query should succeed in test")
            .rate;

        let expected = 0.85;

        assert_eq!(rate, expected);

        // Test stats reflect quotes
        let quotes = matrix.cache_stats();
        assert!(quotes >= 1);
    }

    #[test]
    fn fx_cache_identity_rates() {
        let provider = MockFxProvider::new();
        let matrix = FxMatrix::new(Arc::new(provider));

        let rate = matrix
            .rate(FxQuery::new(Currency::USD, Currency::USD, test_date()))
            .expect("FX rate query should succeed in test")
            .rate;

        let expected = 1.0;

        assert_eq!(rate, expected);
    }

    #[test]
    fn fx_basic_rate_lookup() {
        let provider = MockFxProvider::new();
        let config = FxConfig {
            enable_triangulation: true, // Enable for this test
            ..Default::default()
        };
        let matrix = FxMatrix::with_config(Arc::new(provider), config);

        let result = matrix
            .rate(FxQuery::new(Currency::USD, Currency::EUR, test_date()))
            .expect("FX rate query should succeed in test");

        // Should get a direct rate
        assert!(result.rate > 0.0);
        assert!(!result.triangulated);
    }

    #[test]
    fn fx_clear_implied_matrix() {
        let provider = MockFxProvider::new();
        let config = FxConfig::default();
        let matrix = FxMatrix::with_config(Arc::new(provider), config);

        // Get rate to populate cache
        let _rate1 = matrix
            .rate(FxQuery::new(Currency::USD, Currency::EUR, test_date()))
            .expect("FX rate query should succeed in test");

        // Clear implied entries
        matrix.clear_expired();

        let remaining = matrix.cache_stats();
        assert_eq!(remaining, 0); // Implied cleared
    }

    #[test]
    fn fx_cache_clear() {
        let provider = MockFxProvider::new();
        let matrix = FxMatrix::new(Arc::new(provider));

        // Populate cache
        let _rate = matrix
            .rate(FxQuery::new(Currency::USD, Currency::EUR, test_date()))
            .expect("FX rate query should succeed in test");

        let quotes = matrix.cache_stats();
        assert!(quotes >= 1);

        // Clear cache
        matrix.clear_cache();

        let quotes = matrix.cache_stats();
        assert_eq!(quotes, 0);
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
            ..Default::default()
        };
        let matrix = FxMatrix::with_config(Arc::new(provider), config);

        // Test EUR→GBP triangulation via USD: EUR→USD × USD→GBP
        let rate = matrix
            .rate(FxQuery::new(Currency::EUR, Currency::GBP, test_date()))
            .expect("FX rate query should succeed in test")
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
        let result = matrix.rate(FxQuery::new(Currency::EUR, Currency::GBP, test_date()));

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
            .rate(FxQuery::new(Currency::EUR, Currency::GBP, test_date()))
            .expect("FX rate query should succeed in test")
            .rate;

        // Second call should hit cache
        let rate2 = matrix
            .rate(FxQuery::new(Currency::EUR, Currency::GBP, test_date()))
            .expect("FX rate query should succeed in test")
            .rate;

        assert_eq!(rate1, rate2);

        // Stats indicate stored quotes only
        let quotes = matrix.cache_stats();
        assert!(quotes >= 1);
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
            .rate(FxQuery::new(Currency::USD, Currency::EUR, test_date()))
            .expect("FX rate query should succeed in test")
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
                let mut rates = HashMap::default();
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
        let result = matrix.rate(FxQuery::new(Currency::JPY, Currency::CAD, test_date()));

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
            .rate(FxQuery::new(Currency::USD, Currency::EUR, test_date()))
            .expect("FX rate query should succeed in test");

        assert!(!result.triangulated);

        let expected = 0.85;

        assert_eq!(result.rate, expected);

        // Test triangulated rate
        let result = matrix
            .rate(FxQuery::new(Currency::EUR, Currency::GBP, test_date()))
            .expect("FX rate query should succeed in test");

        assert!(result.triangulated);

        // Expected: 1.18 * 0.75 = 0.885
        let expected = 1.18 * 0.75;

        assert!((result.rate - expected).abs() < 0.001);

        // Test identity rate
        let result = matrix
            .rate(FxQuery::new(Currency::USD, Currency::USD, test_date()))
            .expect("FX rate query should succeed in test");

        assert!(!result.triangulated);

        assert_eq!(result.rate, 1.0);
    }

    #[test]
    fn fx_seed_quotes_directly() {
        let provider = MockFxProvider::new_incomplete();
        let matrix = FxMatrix::new(Arc::new(provider));

        // Seed a direct quote and verify retrieval
        matrix.set_quote(Currency::USD, Currency::CHF, 0.90);
        let usd_chf = matrix
            .rate(FxQuery::new(Currency::USD, Currency::CHF, test_date()))
            .expect("FX rate query should succeed in test")
            .rate;
        assert_eq!(usd_chf, 0.90);

        // Opposite direction should use reciprocal on demand
        let chf_usd = matrix
            .rate(FxQuery::new(Currency::CHF, Currency::USD, test_date()))
            .expect("FX rate query should succeed in test")
            .rate;
        assert!((chf_usd - (1.0 / 0.90)).abs() < 1e-12);

        // Bulk seed: add a couple more pairs
        matrix.set_quotes(&[
            (Currency::EUR, Currency::CHF, 0.95),
            (Currency::GBP, Currency::CHF, 1.10),
        ]);

        let eur_chf = matrix
            .rate(FxQuery::new(Currency::EUR, Currency::CHF, test_date()))
            .expect("FX rate query should succeed in test")
            .rate;
        assert_eq!(eur_chf, 0.95);
    }

    #[test]
    fn fx_bumped_matrix_preserves_seeded_quotes() {
        // Provider has no CHF quotes; we seed them manually.
        let provider = MockFxProvider::new_incomplete();
        let matrix = FxMatrix::new(Arc::new(provider));
        matrix.set_quote(Currency::USD, Currency::CHF, 0.90);

        // Ensure the seeded quote is available.
        let usd_chf = matrix
            .rate(FxQuery::new(Currency::USD, Currency::CHF, test_date()))
            .expect("FX rate query should succeed in test")
            .rate;
        assert_eq!(usd_chf, 0.90);

        // Bump EUR/USD; CHF quote should still be present in the bumped matrix.
        let bumped = matrix
            .with_bumped_rate(Currency::EUR, Currency::USD, 0.01, test_date())
            .expect("Bumped matrix construction should succeed in test");

        let usd_chf_bumped = bumped
            .rate(FxQuery::new(Currency::USD, Currency::CHF, test_date()))
            .expect("FX rate query should succeed in test")
            .rate;
        assert_eq!(usd_chf_bumped, 0.90);
    }
}
