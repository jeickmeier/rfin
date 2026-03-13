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
//! - standard provider implementations (e.g. `SimpleFxProvider`)
//!
//! # Examples
//! ```rust
//! use finstack_core::money::fx::{FxConversionPolicy, FxMatrix, FxProvider, FxQuery};
//! use finstack_core::money::fx::SimpleFxProvider;
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

mod providers;

/// Standard FX provider implementations.
pub use providers::{BumpedFxProvider, SimpleFxProvider};

use crate::currency::Currency;
use crate::dates::Date;
use lru::LruCache;
use parking_lot::Mutex;
use std::num::NonZeroUsize;
use std::sync::Arc;
// no duration needed in the simplified config

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
        let kind = if rate.is_nan() {
            crate::error::NonFiniteKind::NaN
        } else if rate.is_sign_positive() {
            crate::error::NonFiniteKind::PosInfinity
        } else {
            crate::error::NonFiniteKind::NegInfinity
        };
        return Err(crate::error::InputError::NonFiniteValue { kind }.into());
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

#[inline]
fn validate_fx_rate(from: Currency, to: Currency, rate: f64) -> crate::Result<f64> {
    if !rate.is_finite() || rate <= 0.0 {
        return Err(crate::error::InputError::InvalidFxRate { from, to, rate }.into());
    }
    Ok(rate)
}

/// Standard FX conversion strategies used to hint FX providers.
///
/// The policy tells a provider *how* the rate will be applied so it can decide
/// between spot, forward, or averaged sources.
///
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FxQuery {
    /// Source currency
    pub from: Currency,
    /// Target currency
    pub to: Currency,
    /// Applicable date for the rate
    pub on: Date,
    /// Conversion policy (defaults to CashflowDate)
    #[serde(default = "default_policy")]
    pub policy: FxConversionPolicy,
}

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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

/// Query-sensitive cache key for provider-observed FX rates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct QueryKey {
    from: Currency,
    to: Currency,
    on: Date,
    policy: FxConversionPolicy,
}

/// Configuration for [`FxMatrix`] behaviour.
///
/// Controls triangulation and caching.
///
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
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
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
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
/// # Required Methods
///
/// Implementors must provide:
/// - [`rate`](Self::rate): Look up an FX rate for a currency pair
///
/// # Implementation Guide
///
/// When implementing this trait:
/// 1. Return `1.0` when `from == to` (identity conversion)
/// 2. Consider supporting reciprocal lookups (if `A→B` exists, compute `B→A = 1/rate`)
/// 3. Validate rates are finite and positive before returning
/// 4. Use the `policy` hint to select between spot, forward, or averaged rates
///
/// # Errors
///
/// Implementations should return errors when:
/// - [`InputError::NotFound`](crate::error::InputError::NotFound): No rate available for the requested pair
/// - [`InputError::InvalidFxRate`](crate::error::InputError::InvalidFxRate): Rate is non-finite or non-positive
/// - [`InputError::NonFiniteValue`](crate::error::InputError::NonFiniteValue): Computed rate is NaN or infinity
///
/// # Examples
///
/// ## Using the trait
///
/// ```rust
/// use finstack_core::money::fx::{FxConversionPolicy, FxProvider};
/// use finstack_core::currency::Currency;
/// use finstack_core::dates::Date;
/// use time::Month;
///
/// fn convert_amount<P: FxProvider>(
///     provider: &P,
///     amount: f64,
///     from: Currency,
///     to: Currency,
///     on: Date,
/// ) -> finstack_core::Result<f64> {
///     let rate = provider.rate(from, to, on, FxConversionPolicy::CashflowDate)?;
///     Ok(amount * rate)
/// }
/// ```
///
/// ## Implementing the trait
///
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
    /// Return an FX rate to convert `from` → `to` applicable on `on` per `policy`.
    ///
    /// # Arguments
    ///
    /// * `from` - Source currency
    /// * `to` - Target currency
    /// * `on` - Valuation date for the rate lookup
    /// * `policy` - Hint for which rate type to use (spot, forward, average)
    ///
    /// # Returns
    ///
    /// The FX rate such that `amount_in_from * rate = amount_in_to`.
    ///
    /// # Errors
    ///
    /// Returns `Err` when:
    /// - No rate is available for the requested currency pair
    /// - The computed rate is non-finite or non-positive
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
    /// Explicit quotes inserted by callers or restored from serialized state.
    quotes: Mutex<LruCache<Pair, FxRate>>,
    /// Query-sensitive quotes observed from providers or triangulation.
    observed_quotes: Mutex<LruCache<QueryKey, FxRate>>,
    config: FxConfig,
}

/// Serializable state of an FxMatrix.
/// Contains the configuration and cached quotes that can be persisted and restored.
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
        let capacity = NonZeroUsize::new(FxConfig::default().cache_capacity)
            .unwrap_or_else(|| unreachable!("default FxConfig.cache_capacity is non-zero"));
        Self {
            provider,
            quotes: Mutex::new(LruCache::new(capacity)),
            observed_quotes: Mutex::new(LruCache::new(capacity)),
            config: FxConfig::default(),
        }
    }

    /// Create a new [`FxMatrix`] with custom configuration.
    ///
    /// # Deprecated
    ///
    /// Use [`try_with_config`](FxMatrix::try_with_config) instead, which validates
    /// the configuration and returns a `Result` rather than silently clamping
    /// `cache_capacity` to 1 on zero input:
    ///
    /// ```rust
    /// # use finstack_core::money::fx::{FxConfig, FxMatrix, FxProvider, FxConversionPolicy};
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::dates::Date;
    /// # use std::sync::Arc;
    /// # struct StaticFx;
    /// # impl FxProvider for StaticFx {
    /// #     fn rate(&self, _: Currency, _: Currency, _: Date, _: FxConversionPolicy) -> finstack_core::Result<f64> { Ok(1.0) }
    /// # }
    /// let mut cfg = FxConfig::default();
    /// cfg.cache_capacity = 128;
    /// let matrix = FxMatrix::try_with_config(Arc::new(StaticFx), cfg)
    ///     .expect("valid FX config");
    /// ```
    #[deprecated(
        since = "0.4.1",
        note = "Use `try_with_config` instead; `with_config` silently clamps \
                invalid `cache_capacity` to 1 rather than failing fast."
    )]
    pub fn with_config(provider: Arc<dyn FxProvider>, config: FxConfig) -> Self {
        let sanitized = FxConfig {
            cache_capacity: config.cache_capacity.max(1),
            ..config
        };
        let capacity = NonZeroUsize::new(sanitized.cache_capacity).unwrap_or(NonZeroUsize::MIN);
        let quotes = LruCache::new(capacity);
        let observed_quotes = LruCache::new(capacity);
        Self {
            provider,
            quotes: Mutex::new(quotes),
            observed_quotes: Mutex::new(observed_quotes),
            config: sanitized,
        }
    }

    /// Create a new [`FxMatrix`] with custom configuration, failing closed on invalid inputs.
    pub fn try_with_config(provider: Arc<dyn FxProvider>, config: FxConfig) -> crate::Result<Self> {
        if config.cache_capacity == 0 {
            return Err(crate::Error::Validation(
                "FxConfig.cache_capacity must be > 0".to_string(),
            ));
        }
        #[allow(deprecated)]
        Ok(Self::with_config(provider, config))
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

        // Check cache first. Explicit quotes are pair-global, while provider-observed
        // quotes are scoped by date/policy to avoid cross-query contamination.
        let (direct_opt, reciprocal_opt) = self.read_cached_pair_bidir(from, to);
        let (observed_direct_opt, observed_reciprocal_opt) =
            self.read_observed_pair_bidir(from, to, on, policy);

        if let Some(rate) = direct_opt {
            let rate = validate_fx_rate(from, to, rate)?;
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
        if let Some(rate) = observed_direct_opt {
            let rate = validate_fx_rate(from, to, rate)?;
            return Ok(FxRateResult {
                rate,
                triangulated: false,
            });
        }
        if let Some(r_rev) = observed_reciprocal_opt {
            return Ok(FxRateResult {
                rate: reciprocal_rate_or_err(r_rev, to, from)?,
                triangulated: false,
            });
        }

        // Try provider first
        match self.provider.rate(from, to, on, policy) {
            Ok(rate) => {
                let rate = validate_fx_rate(from, to, rate)?;
                self.insert_observed_quote(from, to, on, policy, rate);
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
    /// matrix.set_quote(Currency::GBP, Currency::USD, 1.3)
    ///     .expect("finite, positive explicit quote");
    /// let res = matrix.rate(FxQuery::new(
    ///     Currency::GBP,
    ///     Currency::USD,
    ///     Date::from_calendar_date(2024, Month::April, 1).expect("Valid date"),
    /// )).expect("FX rate lookup should succeed");
    /// assert_eq!(res.rate, 1.3);
    /// ```
    pub fn set_quote(&self, from: Currency, to: Currency, rate: FxRate) -> crate::Result<()> {
        let rate = validate_fx_rate(from, to, rate)?;
        self.insert_quote(from, to, rate);
        Ok(())
    }

    /// Seed multiple quotes at once.
    ///
    /// # Parameters
    /// - `quotes`: slice of `(from, to, rate)` tuples
    pub fn set_quotes(&self, quotes: &[(Currency, Currency, FxRate)]) -> crate::Result<()> {
        let mut map = self.quotes.lock();
        for &(from, to, rate) in quotes {
            validate_fx_rate(from, to, rate)?;
            map.put(Pair(from, to), rate);
        }
        Ok(())
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
        self.observed_quotes.lock().clear();
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
        let observed_quotes = self.observed_quotes.lock();
        quotes.len() + observed_quotes.len()
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
    /// let state = matrix.get_serializable_state();
    /// assert!(state.quotes.is_empty());
    /// ```
    pub fn get_serializable_state(&self) -> FxMatrixState {
        let quotes = self.quotes.lock();
        let mut quote_vec: Vec<(Currency, Currency, FxRate)> = quotes
            .iter()
            .map(|(pair, rate)| (pair.0, pair.1, *rate))
            .collect();
        // Deterministic snapshots: sort by pair key, not by LRU order.
        quote_vec.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
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
    /// let matrix = FxMatrix::new(Arc::new(StaticFx));
    /// let state = FxMatrixState { config: matrix.get_serializable_state().config, quotes: vec![] };
    /// matrix.load_from_state(&state).expect("valid snapshot state");
    /// ```
    pub fn load_from_state(&self, state: &FxMatrixState) -> crate::Result<()> {
        self.set_quotes(&state.quotes)
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
        let bumped = Self::try_with_config(bumped_provider, self.config)?;
        {
            let src = self.quotes.lock();
            let mut dst = bumped.quotes.lock();
            for (pair, rate) in src.iter() {
                if (pair.0 == from && pair.1 == to) || (pair.0 == to && pair.1 == from) {
                    continue;
                }
                dst.put(*pair, *rate);
            }
        }
        // Do not carry over provider-observed quotes. They may be date/policy-sensitive
        // or derived crosses that depend transitively on the bumped leg.
        // IMPORTANT: ensure the bumped pair is not shadowed by copied explicit quotes.
        bumped.set_quote(from, to, bumped_rate)?;

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
        use crate::error::InputError;

        let pivot = self.config.pivot_currency;

        // Try to get first leg: from -> pivot
        let a = match self.get_or_fetch(from, pivot, on, policy) {
            Ok(rate) => rate,
            Err(_) => {
                return Err(InputError::FxTriangulationFailed {
                    from,
                    to,
                    pivot,
                    missing_leg: format!("{from}->{pivot} rate not found"),
                }
                .into());
            }
        };

        // Try to get second leg: pivot -> to
        let b = match self.get_or_fetch(pivot, to, on, policy) {
            Ok(rate) => rate,
            Err(_) => {
                return Err(InputError::FxTriangulationFailed {
                    from,
                    to,
                    pivot,
                    missing_leg: format!("{pivot}->{to} rate not found"),
                }
                .into());
            }
        };

        let rate = a * b;
        let rate = validate_fx_rate(from, to, rate)?;
        self.insert_observed_quote(from, to, on, policy, rate);
        Ok(rate)
    }

    /// Insert an explicit provider quote
    fn insert_quote(&self, from: Currency, to: Currency, rate: FxRate) {
        // Internal insertion should never persist invalid rates.
        let checked = validate_fx_rate(from, to, rate);
        assert!(
            checked.is_ok(),
            "FxMatrix internal quote must be finite, positive (got {from}->{to}={rate})"
        );
        let mut quotes = self.quotes.lock();
        quotes.put(Pair(from, to), rate);
    }

    /// Insert a query-sensitive provider-observed quote.
    fn insert_observed_quote(
        &self,
        from: Currency,
        to: Currency,
        on: Date,
        policy: FxConversionPolicy,
        rate: FxRate,
    ) {
        let checked = validate_fx_rate(from, to, rate);
        assert!(
            checked.is_ok(),
            "FxMatrix observed quote must be finite, positive (got {from}->{to}={rate})"
        );
        let mut quotes = self.observed_quotes.lock();
        quotes.put(
            QueryKey {
                from,
                to,
                on,
                policy,
            },
            rate,
        );
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
        let (observed_direct_opt, observed_reciprocal_opt) =
            self.read_observed_pair_bidir(from, to, on, policy);
        // 1) Explicit quote wins
        if let Some(r) = direct_opt {
            return validate_fx_rate(from, to, r);
        }
        if let Some(r) = observed_direct_opt {
            return validate_fx_rate(from, to, r);
        }
        // 2) Try provider for direct
        if let Ok(r) = self.provider.rate(from, to, on, policy) {
            let r = validate_fx_rate(from, to, r)?;
            self.insert_observed_quote(from, to, on, policy, r);
            return Ok(r);
        }
        // 3) Reciprocal fallback if available
        if let Some(r_rev) = reciprocal_opt {
            return reciprocal_rate_or_err(r_rev, to, from);
        }
        if let Some(r_rev) = observed_reciprocal_opt {
            return reciprocal_rate_or_err(r_rev, to, from);
        }
        // 4) As last resort, propagate provider error
        let r = self.provider.rate(from, to, on, policy)?;
        let r = validate_fx_rate(from, to, r)?;
        self.insert_observed_quote(from, to, on, policy, r);
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
        let direct_key = Pair(from, to);
        let rev_key = Pair(to, from);

        let direct = quotes.get(&direct_key).copied().and_then(|r| {
            if r.is_finite() && r > 0.0 {
                Some(r)
            } else {
                // Purge invalid cached value.
                let _ = quotes.pop(&direct_key);
                None
            }
        });
        let rev = quotes.get(&rev_key).copied().and_then(|r| {
            if r.is_finite() && r > 0.0 {
                Some(r)
            } else {
                let _ = quotes.pop(&rev_key);
                None
            }
        });
        (direct, rev)
    }

    /// Read provider-observed cached quotes scoped to a specific date/policy query.
    #[inline]
    fn read_observed_pair_bidir(
        &self,
        from: Currency,
        to: Currency,
        on: Date,
        policy: FxConversionPolicy,
    ) -> (Option<FxRate>, Option<FxRate>) {
        let mut quotes = self.observed_quotes.lock();
        let direct_key = QueryKey {
            from,
            to,
            on,
            policy,
        };
        let rev_key = QueryKey {
            from: to,
            to: from,
            on,
            policy,
        };

        let direct = quotes.get(&direct_key).copied().and_then(|r| {
            if r.is_finite() && r > 0.0 {
                Some(r)
            } else {
                let _ = quotes.pop(&direct_key);
                None
            }
        });
        let rev = quotes.get(&rev_key).copied().and_then(|r| {
            if r.is_finite() && r > 0.0 {
                Some(r)
            } else {
                let _ = quotes.pop(&rev_key);
                None
            }
        });
        (direct, rev)
    }
}
