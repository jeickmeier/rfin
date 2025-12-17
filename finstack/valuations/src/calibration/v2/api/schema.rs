//! Calibration v2 Schema.
//!
//! Defines the JSON contract for plan-driven calibration.

use crate::calibration::config::CalibrationConfig;
use crate::calibration::v2::domain::quotes::MarketQuote;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContextState;
use finstack_core::market_data::term_structures::Seniority;
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use finstack_core::types::{Currency, CurveId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Schema version identifier for calibration v2 API.
pub const CALIBRATION_SCHEMA_V2: &str = "finstack.calibration/2";

/// Top-level envelope for calibration requests.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationEnvelopeV2 {
    /// Schema version identifier (must be "finstack.calibration/2").
    pub schema: String,

    /// The calibration plan defining steps and quotes.
    pub plan: CalibrationPlanV2,

    /// Optional initial market state (curves, surfaces, scalars).
    /// If not provided, starts with an empty context.
    #[serde(default)]
    pub initial_market: Option<MarketContextState>,
}

/// A calibration plan containing quote sets and execution steps.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationPlanV2 {
    /// Unique identifier for this plan.
    pub id: String,

    /// Optional description.
    #[serde(default)]
    pub description: Option<String>,

    /// Named sets of market quotes.
    /// Steps reference these sets by name.
    pub quote_sets: HashMap<String, Vec<MarketQuote>>,

    /// Ordered list of calibration steps.
    pub steps: Vec<CalibrationStepV2>,

    /// Global calibration configuration (tolerances, bounds).
    #[serde(default)]
    pub settings: CalibrationConfig,
}

/// A single step in the calibration process.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationStepV2 {
    /// Unique identifier for this step.
    pub id: String,

    /// Name of the quote set to use from `plan.quote_sets`.
    pub quote_set: String,

    /// Step parameters defining the target and methodology.
    #[serde(flatten)]
    pub params: StepParams,
}

/// Polymorphic parameters for different calibration step types.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StepParams {
    /// Discount curve calibration.
    Discount(DiscountCurveParams),

    /// Forward curve calibration.
    Forward(ForwardCurveParams),

    /// Hazard curve calibration.
    Hazard(HazardCurveParams),

    /// Inflation curve calibration.
    Inflation(InflationCurveParams),

    /// Volatility surface calibration.
    VolSurface(VolSurfaceParams),

    /// Swaption volatility surface calibration.
    SwaptionVol(SwaptionVolParams),

    /// Base correlation calibration.
    BaseCorrelation(BaseCorrelationParams),
}

// =============================================================================
// Step Parameter Structs
// =============================================================================

/// Parameters for discount curve calibration step.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscountCurveParams {
    /// Identifier for the discount curve being built.
    pub curve_id: CurveId,
    /// Currency of the curve.
    pub currency: Currency,
    /// Base date for the curve.
    pub base_date: Date,
    /// Calibration method to use.
    #[serde(default)]
    pub method: CalibrationMethod,
    /// Interpolation style for the curve.
    #[serde(default = "default_interp_linear")]
    pub interpolation: InterpStyle,
    /// Extrapolation policy for the curve.
    #[serde(default = "default_extrap_flat")]
    pub extrapolation: ExtrapolationPolicy,
    /// Optional separate ID for pricing logic (defaults to curve_id).
    #[serde(default)]
    pub pricing_discount_id: Option<CurveId>,
    /// Optional forward curve ID for pricing (if needed).
    #[serde(default)]
    pub pricing_forward_id: Option<CurveId>,
}

/// Parameters for forward curve calibration step.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForwardCurveParams {
    /// Identifier for the forward curve being built.
    pub curve_id: CurveId,
    /// Currency of the curve.
    pub currency: Currency,
    /// Base date for the curve.
    pub base_date: Date,
    /// Tenor in years for the forward curve.
    pub tenor_years: f64,
    /// Identifier for the discount curve to use.
    pub discount_curve_id: CurveId,
    /// Calibration method to use.
    #[serde(default)]
    pub method: CalibrationMethod,
    /// Interpolation style for the curve.
    #[serde(default = "default_interp_linear")]
    pub interpolation: InterpStyle,
}

/// Parameters for hazard curve calibration step.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HazardCurveParams {
    /// Identifier for the hazard curve being built.
    pub curve_id: CurveId,
    /// Entity name.
    pub entity: String,
    /// Seniority of the debt.
    pub seniority: Seniority,
    /// Currency of the curve.
    pub currency: Currency,
    /// Base date for the curve.
    pub base_date: Date,
    /// Identifier for the discount curve to use.
    pub discount_curve_id: CurveId,
    /// Recovery rate assumption (defaults to 0.4).
    #[serde(default = "default_recovery_04")]
    pub recovery_rate: f64,
    /// Calibration method to use.
    #[serde(default)]
    pub method: CalibrationMethod,
    /// Interpolation style for the curve.
    #[serde(default = "default_interp_linear")]
    pub interpolation: InterpStyle,
}

/// Parameters for inflation curve calibration step.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InflationCurveParams {
    /// Identifier for the inflation curve being built.
    pub curve_id: CurveId,
    /// Currency of the curve.
    pub currency: Currency,
    /// Base date for the curve.
    pub base_date: Date,
    /// Identifier for the discount curve to use.
    pub discount_curve_id: CurveId,
    /// Reference index (e.g. "USA-CPI-U").
    pub index: String,
    /// Observation lag (e.g. "3M").
    pub observation_lag: String,
    /// Base CPI level at base_date.
    pub base_cpi: f64,
    /// Calibration method to use.
    #[serde(default)]
    pub method: CalibrationMethod,
    /// Interpolation style for the curve.
    #[serde(default = "default_interp_linear")]
    pub interpolation: InterpStyle,
}

/// Parameters for volatility surface calibration step.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VolSurfaceParams {
    /// Identifier for the volatility surface being built.
    pub surface_id: String,
    /// Base date for the surface.
    pub base_date: Date,
    /// Identifier for the underlying instrument.
    pub underlying_id: String,
    /// Model type: "Black", "Normal", "SABR", etc.
    pub model: String,
    /// Discount curve ID.
    #[serde(default)]
    pub discount_curve_id: Option<CurveId>,
    /// SABR Beta parameter.
    #[serde(default = "default_sabr_beta")]
    pub beta: f64,
    /// Target expiries for calibration.
    #[serde(default)]
    pub target_expiries: Vec<f64>,
    /// Target strikes for calibration.
    #[serde(default)]
    pub target_strikes: Vec<f64>,
    /// Optional spot price override.
    #[serde(default)]
    pub spot_override: Option<f64>,
    /// Optional dividend yield override.
    #[serde(default)]
    pub dividend_yield_override: Option<f64>,
}

/// Parameters for calibrating swaption volatility surfaces.
///
/// Defines the structure and conventions for building a volatility surface
/// from swaption quotes using the SABR model.
/// Parameters for calibrating swaption volatility surfaces.
///
/// Defines the structure and conventions for building a volatility surface
/// from swaption quotes using the SABR model.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SwaptionVolParams {
    /// Identifier for the volatility surface.
    pub surface_id: String,
    /// Base date for the calibration.
    pub base_date: Date,
    /// Discount curve identifier for pricing.
    pub discount_curve_id: CurveId,
    /// Optional forward curve identifier (if different from discount curve).
    #[serde(default)]
    pub forward_id: Option<String>,
    /// Currency for the swaption surface.
    pub currency: Currency,
    /// Volatility quoting convention (normal or lognormal).
    #[serde(default)]
    pub vol_convention: SwaptionVolConvention,
    /// ATM strike convention for swaptions.
    #[serde(default)]
    pub atm_convention: AtmStrikeConvention,
    /// SABR beta parameter (typically 0.0 for normal, 1.0 for lognormal).
    #[serde(default = "default_sabr_beta")]
    pub sabr_beta: f64,
    /// Target expiry times (in years) for the surface grid.
    #[serde(default)]
    pub target_expiries: Vec<f64>,
    /// Target tenor times (in years) for the surface grid.
    #[serde(default)]
    pub target_tenors: Vec<f64>,
    /// SABR parameter interpolation method between expiries/tenors.
    #[serde(default)]
    pub sabr_interpolation: SabrInterpolationMethod,
    /// Optional calendar identifier for date adjustments.
    #[serde(default)]
    pub calendar_id: Option<String>,
    /// Optional day count convention for fixed leg calculations.
    #[serde(default)]
    pub fixed_day_count: Option<DayCount>,
}

/// Parameters for calibrating base correlation curves.
///
/// Defines the structure for building a base correlation curve from
/// CDS tranche quotes with different detachment points.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaseCorrelationParams {
    /// Credit index identifier (e.g., CDX, iTraxx).
    pub index_id: String,
    /// Series number of the credit index.
    pub series: u16,
    /// Maturity of the tranches in years.
    pub maturity_years: f64,
    /// Base date for the calibration.
    pub base_date: Date,
    /// Discount curve identifier for pricing.
    pub discount_curve_id: CurveId,
    /// Detachment points (as percentages) for the tranches.
    #[serde(default)]
    pub detachment_points: Vec<f64>,
    /// Whether to use IMM dates for coupon schedules.
    #[serde(default)]
    pub use_imm_dates: bool,
}

/// Volatility quoting convention for swaptions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwaptionVolConvention {
    /// Normal (absolute) volatility in basis points
    Normal,
    /// Lognormal (Black) volatility as percentage
    #[default]
    Lognormal,
    /// Shifted lognormal for negative rates
    ShiftedLognormal {
        /// Shift amount for negative rate handling
        shift: f64,
    },
}

/// ATM strike convention for swaptions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AtmStrikeConvention {
    /// ATM = forward swap rate (standard market convention)
    #[default]
    SwapRate,
    /// ATM = par swap rate (same as forward for zero-cost swap)
    ParRate,
}

/// Interpolation method for SABR parameters across the expiry–tenor grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SabrInterpolationMethod {
    /// Bilinear interpolation in (expiry, tenor) over SABR parameters.
    #[default]
    Bilinear,
}

/// Calibration methodology choice.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationMethod {
    /// Sequential bootstrapping method.
    #[default]
    Bootstrap,
    /// Global optimization method.
    Global,
}

// Defaults
fn default_interp_linear() -> InterpStyle {
    InterpStyle::Linear
}

fn default_extrap_flat() -> ExtrapolationPolicy {
    ExtrapolationPolicy::FlatZero
}

fn default_recovery_04() -> f64 {
    0.4
}

#[allow(dead_code)]
fn default_sabr_beta() -> f64 {
    0.5
}

#[allow(dead_code)]
fn default_true() -> bool {
    true
}
