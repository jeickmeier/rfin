//! Numeric/rounding configuration for finstack-core.
//!
//! This module defines configuration types and helper functions that operate on
//! an explicit `FinstackConfig` passed by the caller. There is no global
//! configuration state; call sites must provide the configuration they wish to
//! apply.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use hashbrown::HashMap;

/// Rounding modes supported by the library.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum RoundingMode {
    /// Banker's rounding (ties to even).
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

/// Configuration container. Extend as needed.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FinstackConfig {
    /// Detailed rounding policy (ingest/output scales by currency).
    pub rounding: RoundingPolicy,
}

impl Default for FinstackConfig {
    fn default() -> Self {
        Self {
            rounding: RoundingPolicy::default(),
        }
    }
}

/// Policy mapping to determine decimal places for each currency at ingest/output.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CurrencyScalePolicy {
    /// Default scale (decimal places) when currency not present in overrides.
    pub default_scale: u32,
    /// Explicit currency overrides for scale.
    pub overrides: HashMap<crate::currency::Currency, u32>,
}

impl Default for CurrencyScalePolicy {
    fn default() -> Self {
        Self {
            default_scale: 2,
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

/// Snapshot of active rounding settings for stamping in results.
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

/// Numeric engine mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum NumericMode {
    /// Floating-point f64 engine.
    F64,
    /// Decimal-128 (rust_decimal) engine.
    Decimal128,
}

/// Result metadata commonly stamped into envelopes.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ResultsMeta {
    /// Numeric engine mode used to produce the results.
    pub numeric_mode: NumericMode,
    /// Rounding context snapshot applied to IO boundaries.
    pub rounding: RoundingContext,
    // Reserved for future: parallel flag, seeds, etc.
}

/// Compute the effective output scale for a currency.
pub fn output_scale_for(cfg: &FinstackConfig, ccy: crate::currency::Currency) -> u32 {
    if let Some(&s) = cfg.rounding.output_scale.overrides.get(&ccy) {
        return s;
    }
    cfg.rounding.output_scale.default_scale
}

/// Compute the effective ingest scale for a currency.
pub fn ingest_scale_for(cfg: &FinstackConfig, ccy: crate::currency::Currency) -> u32 {
    if let Some(&s) = cfg.rounding.ingest_scale.overrides.get(&ccy) {
        return s;
    }
    cfg.rounding.ingest_scale.default_scale
}

/// Build a snapshot of the current rounding context from a config.
pub fn rounding_context_from(cfg: &FinstackConfig) -> RoundingContext {
    RoundingContext {
        mode: cfg.rounding.mode,
        ingest_scale_by_ccy: cfg.rounding.ingest_scale.overrides.clone(),
        output_scale_by_ccy: cfg.rounding.output_scale.overrides.clone(),
        version: 1,
    }
}

/// Obtain current numeric mode.
pub fn numeric_mode() -> NumericMode {
    #[cfg(feature = "decimal128")]
    {
        NumericMode::Decimal128
    }
    #[cfg(not(feature = "decimal128"))]
    {
        NumericMode::F64
    }
}

/// Construct a `ResultsMeta` snapshot for stamping into result envelopes.
pub fn results_meta(cfg: &FinstackConfig) -> ResultsMeta {
    ResultsMeta {
        numeric_mode: numeric_mode(),
        rounding: rounding_context_from(cfg),
    }
}

// No unit tests here rely on global configuration anymore.
