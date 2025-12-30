//! Numeric precision, rounding rules, and IO metadata for `finstack-core`.
//!
//! FinStack opts into *explicit* configuration: there is no global state and all
//! helpers in this module expect a [`FinstackConfig`] supplied by the caller. The
//! config captures per-currency rounding scales, the active numeric engine, and
//! helper utilities for stamping outputs.
//!
//! # When to use
//! - to determine ingest/output decimal scales for [`crate::money::Money`]
//! - to stamp result envelopes with the numeric configuration via [`results_meta`]
//! - to snapshot the rounding policy that produced an output
//!
//! # Examples
//! ```rust
//! use finstack_core::config::{FinstackConfig, RoundingMode};
//! use finstack_core::currency::Currency;
//!
//! let mut cfg = FinstackConfig::default();
//! cfg.rounding.mode = RoundingMode::AwayFromZero;
//! cfg.rounding.output_scale.overrides.insert(Currency::JPY, 0);
//! cfg.rounding.ingest_scale.overrides.insert(Currency::JPY, 0);
//!
//! let usd_scale = cfg.output_scale(Currency::USD);
//! let jpy_ingest = cfg.ingest_scale(Currency::JPY);
//!
//! assert_eq!(usd_scale, 2);
//! assert_eq!(jpy_ingest, 0);
//! ```
//!
//! ## Numeric mode
//!
//! The engine currently operates in a single numeric mode: [`NumericMode::F64`].
//! To make this explicit and avoid unnecessary function calls, the active mode
//! is exposed as a constant: [`NUMERIC_MODE`]. Future releases may introduce
//! additional modes (e.g., alternative numeric strategies) or feature-gated switching; in that case
//! the constant will remain stable and reflect the compile-time choice.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use std::collections::BTreeMap;

/// Rounding modes supported by the library.
///
/// The variants mirror the most common conventions found in pricing engines.
///
/// # Examples
/// ```rust
/// use finstack_core::config::{FinstackConfig, RoundingMode};
///
/// let mut cfg = FinstackConfig::default();
/// cfg.rounding.mode = RoundingMode::TowardZero;
/// assert!(matches!(cfg.rounding.mode, RoundingMode::TowardZero));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RoundingMode {
    /// Banker's rounding (ties to even).
    #[default]
    Bankers,
    /// Round halves away from zero.
    AwayFromZero,
    /// Round toward zero (truncate).
    TowardZero,
    /// Round toward negative infinity.
    Floor,
    /// Round toward positive infinity.
    Ceil,
}

// Default derived above

/// Global numeric configuration supplied to valuation components.
///
/// The configuration owns two [`CurrencyScalePolicy`] maps (ingest/output) and
/// the active [`RoundingMode`]. It can be customised at startup and threaded
/// through pricing calculations.
///
/// # Examples
/// ```rust
/// use finstack_core::config::{FinstackConfig, RoundingMode};
/// use finstack_core::currency::Currency;
///
/// let mut cfg = FinstackConfig::default();
/// cfg.rounding.mode = RoundingMode::Bankers;
/// cfg.rounding.output_scale.overrides.insert(Currency::CHF, 2);
/// assert_eq!(cfg.output_scale(Currency::CHF), 2);
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FinstackConfig {
    /// Detailed rounding policy (ingest/output scales by currency).
    pub rounding: RoundingPolicy,
    /// Numerical tolerance settings for floating-point comparisons.
    #[serde(default)]
    pub tolerances: ToleranceConfig,
    /// Optional module-specific configuration sections (versioned, namespaced keys).
    ///
    /// Keys follow `{crate}.{domain}.v{N}`, e.g., `valuations.calibration.v2`.
    /// Values are validated by the owning crate's strict serde schema.
    #[serde(default, skip_serializing_if = "ConfigExtensions::is_empty")]
    pub extensions: ConfigExtensions,
}
// Default derived above

/// Versioned, namespaced extension map carried alongside the core config.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ConfigExtensions {
    #[serde(flatten)]
    pub(crate) inner: BTreeMap<String, JsonValue>,
}

impl ConfigExtensions {
    /// Returns true if no extension sections are present.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get a section by key.
    #[inline]
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        self.inner.get(key)
    }

    /// Insert or replace a section by key.
    #[inline]
    pub fn insert(&mut self, key: impl Into<String>, value: JsonValue) -> Option<JsonValue> {
        self.inner.insert(key.into(), value)
    }
}

/// Policy mapping that determines decimal places for each currency at ingest/output.
///
/// The policy stores currency-specific overrides only.
///
/// # Examples
/// ```rust
/// use finstack_core::config::{CurrencyScalePolicy, FinstackConfig};
/// use finstack_core::currency::Currency;
/// use std::collections::BTreeMap;
///
/// let mut cfg = FinstackConfig::default();
/// cfg.rounding.output_scale = CurrencyScalePolicy {
///     overrides: BTreeMap::from([(Currency::KWD, 3)]),
/// };
///
/// assert_eq!(cfg.output_scale(Currency::KWD), 3);
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CurrencyScalePolicy {
    /// Explicit currency overrides for scale.
    pub overrides: BTreeMap<crate::currency::Currency, u32>,
}

/// Full rounding policy used at IO boundaries and normalization steps.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RoundingPolicy {
    /// Rounding behaviour to apply when mapping fractional values to a scale.
    pub mode: RoundingMode,
    /// Decimal places applied when normalizing inbound values by currency.
    pub ingest_scale: CurrencyScalePolicy,
    /// Decimal places used at output/serialization for each currency.
    pub output_scale: CurrencyScalePolicy,
}

/// Numerical tolerance configuration for floating-point comparisons.
///
/// Provides configurable epsilon values for zero-checks in rate calculations
/// and generic floating-point comparisons. These defaults are chosen to balance
/// numerical stability with practical precision requirements.
///
/// # Examples
/// ```rust
/// use finstack_core::config::ToleranceConfig;
///
/// let mut tol = ToleranceConfig::default();
/// assert_eq!(tol.rate_epsilon, 1e-12);
///
/// // Customize for stricter rate comparisons
/// tol.rate_epsilon = 1e-14;
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToleranceConfig {
    /// Epsilon for rate comparisons (default: 1e-12).
    ///
    /// Used when comparing interest rates, yields, and other small ratios.
    #[serde(default = "default_rate_epsilon")]
    pub rate_epsilon: f64,
    /// Epsilon for generic floating-point comparisons (default: 1e-10).
    ///
    /// Used for general numerical comparisons where higher tolerance is acceptable.
    #[serde(default = "default_generic_epsilon")]
    pub generic_epsilon: f64,
}

fn default_rate_epsilon() -> f64 {
    1e-12
}

fn default_generic_epsilon() -> f64 {
    1e-10
}

impl Default for ToleranceConfig {
    fn default() -> Self {
        Self {
            rate_epsilon: default_rate_epsilon(),
            generic_epsilon: default_generic_epsilon(),
        }
    }
}

/// Snapshot of active rounding settings used for result stamping.
///
/// Instances are typically produced via [`rounding_context_from`] and persisted
/// alongside valuation results.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoundingContext {
    /// Active rounding mode.
    pub mode: RoundingMode,
    /// Ingest scale map snapshot by currency code.
    pub ingest_scale_by_ccy: BTreeMap<crate::currency::Currency, u32>,
    /// Output scale map snapshot by currency code.
    pub output_scale_by_ccy: BTreeMap<crate::currency::Currency, u32>,
    /// Tolerance settings snapshot for floating-point comparisons.
    #[serde(default)]
    pub tolerances: ToleranceConfig,
    /// Schema version for forward compatibility.
    pub version: u32,
}

/// Zero-kind classification for tolerance checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ZeroKind {
    /// Money-like magnitudes (use currency scale).
    Money(crate::currency::Currency),
    /// Interest rates or small numeric ratios.
    Rate,
    /// Generic floating-point comparisons.
    Generic,
}

impl Default for RoundingContext {
    fn default() -> Self {
        rounding_context_from(&FinstackConfig::default())
    }
}

impl RoundingContext {
    /// Effective output scale for the provided currency within this context.
    #[inline]
    pub fn output_scale(&self, ccy: crate::currency::Currency) -> u32 {
        if let Some(&s) = self.output_scale_by_ccy.get(&ccy) {
            return s;
        }
        ccy.decimals() as u32
    }

    /// Money epsilon derived from the currency output scale (half ULP at that scale).
    #[inline]
    pub fn money_epsilon(&self, ccy: crate::currency::Currency) -> f64 {
        let scale = self.output_scale(ccy) as i32;
        // Half of one unit in the last place at the configured scale.
        0.5 * 10f64.powi(-scale)
    }

    /// Returns true if a money amount (in the given currency) is effectively zero under this context.
    #[inline]
    pub fn is_effectively_zero_money(&self, amount: f64, ccy: crate::currency::Currency) -> bool {
        amount.abs() <= self.money_epsilon(ccy)
    }

    /// Returns true if a floating value is effectively zero for the specified kind.
    ///
    /// Uses tolerance values from the context's [`ToleranceConfig`]:
    /// - Rate: uses [`ToleranceConfig::rate_epsilon`] (default: 1e-12)
    /// - Generic: uses [`ToleranceConfig::generic_epsilon`] (default: 1e-10)
    /// - Money: uses [`Self::money_epsilon`] (derived from currency scale)
    #[inline]
    pub fn is_effectively_zero(&self, x: f64, kind: ZeroKind) -> bool {
        match kind {
            ZeroKind::Money(ccy) => self.is_effectively_zero_money(x, ccy),
            ZeroKind::Rate => x.abs() <= self.tolerances.rate_epsilon,
            ZeroKind::Generic => x.abs() <= self.tolerances.generic_epsilon,
        }
    }
}

/// Numeric engine mode compiled into the crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum NumericMode {
    /// Floating-point f64 engine.
    F64,
}

/// Metadata bundle that accompanies valuation outputs.
///
/// The metadata is intentionally small so it can be attached to reports and
/// downstream data stores for reproducibility and audit trails.
///
/// # Examples
/// ```rust
/// use finstack_core::config::{results_meta, FinstackConfig, NumericMode};
///
/// let meta = results_meta(&FinstackConfig::default());
/// assert_eq!(meta.numeric_mode, NumericMode::F64);
/// assert!(meta.timestamp.is_none()); // deterministic by default
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResultsMeta {
    /// Numeric engine mode used to produce the results.
    pub numeric_mode: NumericMode,
    /// Rounding context snapshot applied to IO boundaries.
    pub rounding: RoundingContext,
    /// Optional FX policy applied by the computing layer (human-readable key).
    #[serde(default)]
    pub fx_policy_applied: Option<String>,
    /// Timestamp when result was computed (ISO 8601 format).
    /// Useful for audit trails and reproducibility.
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        with = "time::serde::iso8601::option"
    )]
    pub timestamp: Option<time::OffsetDateTime>,
    /// Finstack library version used to produce the result.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub version: Option<String>,
}

impl Default for ResultsMeta {
    fn default() -> Self {
        results_meta(&FinstackConfig::default())
    }
}

impl FinstackConfig {
    /// Effective output scale for a currency. Falls back to ISO-4217 `Currency::decimals()`.
    pub fn output_scale(&self, ccy: crate::currency::Currency) -> u32 {
        if let Some(&s) = self.rounding.output_scale.overrides.get(&ccy) {
            return s;
        }
        ccy.decimals() as u32
    }

    /// Effective ingest scale for a currency. Falls back to a high precision default
    /// to preserve inputs unless explicitly overridden (min of 6 decimals).
    pub fn ingest_scale(&self, ccy: crate::currency::Currency) -> u32 {
        if let Some(&s) = self.rounding.ingest_scale.overrides.get(&ccy) {
            return s;
        }
        core::cmp::max(6, ccy.decimals() as u32)
    }
}

/// Build a snapshot of the current rounding context from a config.
///
/// The snapshot captures the concrete overrides used during valuation and can
/// later be embedded in [`ResultsMeta`].
///
/// # Examples
/// ```rust
/// use finstack_core::config::{rounding_context_from, FinstackConfig};
///
/// let ctx = rounding_context_from(&FinstackConfig::default());
/// assert_eq!(ctx.version, 1);
/// ```
pub fn rounding_context_from(cfg: &FinstackConfig) -> RoundingContext {
    RoundingContext {
        mode: cfg.rounding.mode,
        ingest_scale_by_ccy: cfg.rounding.ingest_scale.overrides.clone(),
        output_scale_by_ccy: cfg.rounding.output_scale.overrides.clone(),
        tolerances: cfg.tolerances,
        version: 1,
    }
}

/// Active numeric mode used by the engine.
pub const NUMERIC_MODE: NumericMode = NumericMode::F64;

/// Construct a [`ResultsMeta`] snapshot for stamping into result envelopes.
///
/// Convenience wrapper that combines [`NUMERIC_MODE`] and
/// [`rounding_context_from`], without a timestamp (deterministic).
///
/// # Examples
/// ```rust
/// use finstack_core::config::{results_meta, FinstackConfig, NumericMode};
///
/// let cfg = FinstackConfig::default();
/// let meta = results_meta(&cfg);
/// assert_eq!(meta.numeric_mode, NumericMode::F64);
/// assert!(meta.timestamp.is_none());
/// ```
pub fn results_meta(cfg: &FinstackConfig) -> ResultsMeta {
    results_meta_with_timestamp(cfg, None)
}

/// Construct a [`ResultsMeta`] snapshot and stamp a timestamp of "now".
///
/// Use this at user-facing IO boundaries and audit trails. For deterministic outputs
/// (golden tests, reproducible snapshots), prefer [`results_meta`].
pub fn results_meta_now(cfg: &FinstackConfig) -> ResultsMeta {
    // With `wasm-bindgen` feature enabled in `time` crate, `now_utc()` works on WASM too.
    results_meta_with_timestamp(cfg, Some(time::OffsetDateTime::now_utc()))
}

/// Construct a [`ResultsMeta`] snapshot with an explicitly provided timestamp.
///
/// This is useful for deterministic injection (tests) or when a higher-level layer
/// controls timestamping.
pub fn results_meta_with_timestamp(
    cfg: &FinstackConfig,
    timestamp: Option<time::OffsetDateTime>,
) -> ResultsMeta {
    ResultsMeta {
        numeric_mode: NUMERIC_MODE,
        rounding: rounding_context_from(cfg),
        fx_policy_applied: None,
        timestamp,
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
    }
}

// No unit tests here rely on global configuration anymore.
