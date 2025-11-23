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

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use hashbrown::HashMap;

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FinstackConfig {
    /// Detailed rounding policy (ingest/output scales by currency).
    pub rounding: RoundingPolicy,
}
// Default derived above

/// Policy mapping that determines decimal places for each currency at ingest/output.
///
/// The policy stores currency-specific overrides only.
///
/// # Examples
/// ```rust
/// use finstack_core::config::{CurrencyScalePolicy, FinstackConfig};
/// use finstack_core::currency::Currency;
/// use hashbrown::HashMap;
///
/// let mut cfg = FinstackConfig::default();
/// cfg.rounding.output_scale = CurrencyScalePolicy {
///     overrides: HashMap::from([(Currency::KWD, 3)]),
/// };
///
/// assert_eq!(cfg.output_scale(Currency::KWD), 3);
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CurrencyScalePolicy {
    /// Explicit currency overrides for scale.
    pub overrides: HashMap<crate::currency::Currency, u32>,
}

impl Default for CurrencyScalePolicy {
    fn default() -> Self {
        Self {
            overrides: HashMap::new(),
        }
    }
}

/// Full rounding policy used at IO boundaries and normalization steps.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RoundingPolicy {
    /// Rounding behaviour to apply when mapping fractional values to a scale.
    pub mode: RoundingMode,
    /// Decimal places applied when normalizing inbound values by currency.
    pub ingest_scale: CurrencyScalePolicy,
    /// Decimal places used at output/serialization for each currency.
    pub output_scale: CurrencyScalePolicy,
}

impl Default for RoundingPolicy {
    fn default() -> Self {
        Self {
            mode: RoundingMode::Bankers,
            ingest_scale: CurrencyScalePolicy::default(),
            output_scale: CurrencyScalePolicy::default(),
        }
    }
}

/// Snapshot of active rounding settings used for result stamping.
///
/// Instances are typically produced via [`rounding_context_from`] and persisted
/// alongside valuation results.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RoundingContext {
    /// Active rounding mode.
    pub mode: RoundingMode,
    /// Ingest scale map snapshot by currency code.
    pub ingest_scale_by_ccy: HashMap<crate::currency::Currency, u32>,
    /// Output scale map snapshot by currency code.
    pub output_scale_by_ccy: HashMap<crate::currency::Currency, u32>,
    /// Schema version for forward compatibility.
    pub version: u32,
}

/// Zero-kind classification for tolerance checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
    /// Heuristics:
    /// - Rate: 1e-12
    /// - Generic: 1e-10
    /// - Money: uses [`money_epsilon`]
    #[inline]
    pub fn is_effectively_zero(&self, x: f64, kind: ZeroKind) -> bool {
        match kind {
            ZeroKind::Money(ccy) => self.is_effectively_zero_money(x, ccy),
            ZeroKind::Rate => x.abs() <= 1e-12,
            ZeroKind::Generic => x.abs() <= 1e-10,
        }
    }
}

/// Numeric engine mode compiled into the crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
/// assert!(meta.timestamp.is_some());
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ResultsMeta {
    /// Numeric engine mode used to produce the results.
    pub numeric_mode: NumericMode,
    /// Rounding context snapshot applied to IO boundaries.
    pub rounding: RoundingContext,
    /// Optional FX policy applied by the computing layer (human-readable key).
    #[cfg_attr(feature = "serde", serde(default))]
    pub fx_policy_applied: Option<String>,
    /// Timestamp when result was computed (ISO 8601 format).
    /// Useful for audit trails and reproducibility.
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "Option::is_none",
            default,
            with = "time::serde::iso8601::option"
        )
    )]
    pub timestamp: Option<time::OffsetDateTime>,
    /// Finstack library version used to produce the result.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
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
        version: 1,
    }
}

/// Active numeric mode used by the engine.
pub const NUMERIC_MODE: NumericMode = NumericMode::F64;

/// Construct a [`ResultsMeta`] snapshot for stamping into result envelopes.
///
/// Convenience wrapper that combines [`NUMERIC_MODE`] and
/// [`rounding_context_from`], with automatic timestamping.
///
/// # Examples
/// ```rust
/// use finstack_core::config::{results_meta, FinstackConfig, NumericMode};
///
/// let cfg = FinstackConfig::default();
/// let meta = results_meta(&cfg);
/// assert_eq!(meta.numeric_mode, NumericMode::F64);
/// assert!(meta.timestamp.is_some());
/// ```
pub fn results_meta(cfg: &FinstackConfig) -> ResultsMeta {
    // Generate ISO 8601 timestamp
    // With `wasm-bindgen` feature enabled in `time` crate, `now_utc()` works on WASM too.
    let timestamp = Some(time::OffsetDateTime::now_utc());

    ResultsMeta {
        numeric_mode: NUMERIC_MODE,
        rounding: rounding_context_from(cfg),
        fx_policy_applied: None,
        timestamp,
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
    }
}

// No unit tests here rely on global configuration anymore.
