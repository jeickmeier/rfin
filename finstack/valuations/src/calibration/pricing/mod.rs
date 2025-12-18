//! Calibration pricing infrastructure.
//!
//! This module defines the conventions, pricer, and factories required to value
//! financial instruments during the calibration process. It ensures that
//! market-standard pricing logic is consistently applied across all adapters.
//!
//! # Submodules
//! - [`pricer`]: The core [`CalibrationPricer`] that handles multi-curve context.
//! - [`convention_resolution`]: Logic for mapping high-level strings to concrete conventions.
//! - [`quote_factory`]: Converts market quotes into concrete instrument objects.
//! - [`convexity`]: Utilities for futures convexity adjustments.

/// Convention resolution for pricing (turns quote conventions + market defaults into effective inputs).
pub(crate) mod convention_resolution;
mod convexity;
mod pricer;
pub(crate) mod quote_factory;

use finstack_core::dates::{BusinessDayConvention, DayCount};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Step-level conventions for rates calibration (discount and forward curves).
///
/// This is a Bloomberg/FinCad-style design: curve construction uses a small set of
/// *step-level* conventions (e.g., curve time-axis day count), while individual
/// quotes can still override instrument conventions via `InstrumentConventions`.
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RatesStepConventions {
    /// Day count used to map dates to year fractions for curve knot times.
    #[serde(default)]
    #[cfg_attr(feature = "ts_export", ts(type = "string | null"))]
    pub curve_day_count: Option<DayCount>,

    /// Optional pricer-level settlement lag override (business days).
    #[serde(default)]
    pub settlement_days: Option<i32>,

    /// Optional pricer-level calendar identifier override.
    #[serde(default)]
    pub calendar_id: Option<String>,

    /// Optional pricer-level business day convention override.
    #[serde(default)]
    #[cfg_attr(feature = "ts_export", ts(type = "string | null"))]
    pub business_day_convention: Option<BusinessDayConvention>,

    /// Allow calendar-day fallback when the requested calendar is missing.
    #[serde(default)]
    pub allow_calendar_fallback: Option<bool>,

    /// Whether instruments start at settlement (true for discount curves).
    #[serde(default)]
    pub use_settlement_start: Option<bool>,

    /// Enable vendor-style strict pricing in this step.
    ///
    /// When enabled, calibration will fail fast if required pricing conventions are
    /// not explicitly provided (either via these step-level conventions or via the
    /// quote/leg `InstrumentConventions`). This avoids hidden currency-based defaults
    /// and improves vendor-matching determinism.
    #[serde(default)]
    pub strict_pricing: Option<bool>,

    /// Step-level default payment delay (business days) used when quotes do not specify one.
    ///
    /// In strict pricing mode, this must be explicitly provided unless the instrument's
    /// conventions (e.g., overnight RFR index rules) supply a deterministic value.
    #[serde(default)]
    pub default_payment_delay_days: Option<i32>,

    /// Step-level default reset lag (business days) used when quotes do not specify one.
    ///
    /// In strict pricing mode, this must be explicitly provided unless the instrument's
    /// conventions (e.g., overnight RFR index rules) supply a deterministic value.
    #[serde(default)]
    pub default_reset_lag_days: Option<i32>,

    /// Optional convexity parameters for futures pricing in this step.
    #[serde(default)]
    #[cfg_attr(feature = "ts_export", ts(type = "Record<string, unknown> | null"))]
    pub convexity_params: Option<ConvexityParameters>,

    /// Enforce discount-curve separation (reject non-OIS forward-dependent quotes).
    ///
    /// Default is `false` to preserve backwards compatibility; enable to match
    /// vendor-style strict validation.
    #[serde(default)]
    pub enforce_discount_separation: Option<bool>,
}

pub use convexity::{
    calculate_convexity_adjustment, default_convexity_params, estimate_rate_volatility,
    ho_lee_convexity, ConvexityParameters, VolatilitySource,
};
pub use pricer::{CalibrationPricer, RatesQuoteUseCase};
