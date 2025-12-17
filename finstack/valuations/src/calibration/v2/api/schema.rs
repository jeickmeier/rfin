//! Calibration v2 Schema.
//!
//! Defines the JSON contract for plan-driven calibration.

use crate::calibration::config::CalibrationConfig;
use crate::calibration::v2::domain::pricing::ConvexityParameters;
use crate::calibration::v2::domain::quotes::MarketQuote;
use finstack_core::dates::BusinessDayConvention;
use finstack_core::dates::{Date, DayCount, Tenor};
use finstack_core::market_data::context::MarketContextState;
use finstack_core::market_data::term_structures::{ParInterp, Seniority};
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use finstack_core::types::{Currency, CurveId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Schema version identifier for calibration v2 API.
pub const CALIBRATION_SCHEMA_V2: &str = "finstack.calibration/2";

/// Top-level envelope for calibration requests.
#[derive(Clone, Debug, Serialize, Deserialize)]
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

/// Step-level conventions for rates calibration (discount and forward curves).
///
/// This is a Bloomberg/FinCad-style design: curve construction uses a small set of
/// *step-level* conventions (e.g., curve time-axis day count), while individual
/// quotes can still override instrument conventions via `InstrumentConventions`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RatesStepConventions {
    /// Day count used to map dates to year fractions for curve knot times.
    #[serde(default)]
    pub curve_day_count: Option<DayCount>,

    /// Optional pricer-level settlement lag override (business days).
    #[serde(default)]
    pub settlement_days: Option<i32>,

    /// Optional pricer-level calendar identifier override.
    #[serde(default)]
    pub calendar_id: Option<String>,

    /// Optional pricer-level business day convention override.
    #[serde(default)]
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
    pub convexity_params: Option<ConvexityParameters>,

    /// Enforce discount-curve separation (reject non-OIS forward-dependent quotes).
    ///
    /// Default is `false` to preserve backwards compatibility; enable to match
    /// vendor-style strict validation.
    #[serde(default)]
    pub enforce_discount_separation: Option<bool>,
}

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

    /// Step-level conventions for pricing and curve time axis.
    #[serde(default)]
    pub conventions: RatesStepConventions,
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

    /// Step-level conventions for pricing and curve time axis.
    #[serde(default)]
    pub conventions: RatesStepConventions,
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
    /// Notional used to price synthetic CDS instruments during calibration.
    ///
    /// Calibration normalizes residuals by notional, so this is typically left as
    /// the unit-notional default unless you have a specific reason to change it.
    #[serde(default = "default_unit_notional")]
    pub notional: f64,
    /// Calibration method to use.
    #[serde(default)]
    pub method: CalibrationMethod,
    /// Interpolation style for the curve.
    #[serde(default = "default_interp_linear")]
    pub interpolation: InterpStyle,

    /// Interpolation method for par spreads reported by the calibrated curve.
    ///
    /// Note: this is used for *quoting/interpolation of stored par spreads* and does not affect
    /// survival no-arbitrage, which is enforced via non-negative hazards and the curve's
    /// internal log-linear survival interpolation.
    #[serde(default = "default_par_interp_linear")]
    pub par_interp: ParInterp,
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
    ///
    /// This controls the index publication lag used for instruments referencing this curve
    /// when no `InflationIndex` fixings series is provided in the market context.
    pub observation_lag: String,
    /// Base CPI level used as the curve's reference CPI at t=0.
    ///
    /// When calibrating ZCIS curves in a curve-only context, this is typically the latest
    /// known CPI fixing, i.e. CPI at `base_date - observation_lag` (not CPI at `base_date`).
    pub base_cpi: f64,
    /// Notional used to price synthetic inflation swaps during calibration.
    ///
    /// Calibration normalizes residuals by notional, so this is typically left as
    /// the unit-notional default unless you have a specific reason to change it.
    #[serde(default = "default_unit_notional")]
    pub notional: f64,
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
    /// Model type.
    ///
    /// Note: v2 currently supports SABR-only; set to `"SABR"` (case-insensitive).
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
    /// Extrapolation policy for SABR parameter interpolation across expiries.
    ///
    /// This controls how the adapter behaves when `target_expiries` extend beyond
    /// the expiries that were successfully calibrated from market quotes.
    #[serde(default)]
    pub expiry_extrapolation: SurfaceExtrapolationPolicy,
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
    /// Reporting tolerance used to determine calibration success.
    ///
    /// This is distinct from `plan.settings.tolerance` (solver tolerance). For swaption-vol
    /// calibration, success should reflect whether the fitted smile residuals are within a
    /// market-appropriate tolerance (e.g., 10–20 vol bps), not machine epsilon.
    #[serde(default)]
    pub vol_tolerance: Option<f64>,

    /// Solver tolerance used inside the SABR calibration routines.
    ///
    /// This is an algorithmic convergence tolerance (not a market quoting tolerance).
    /// If unset, the SABR calibrator default is used.
    #[serde(default)]
    pub sabr_tolerance: Option<f64>,

    /// Extrapolation policy used when interpolating SABR parameters across the
    /// expiry–tenor grid for target points that do not have a directly calibrated bucket.
    #[serde(default)]
    pub sabr_extrapolation: SurfaceExtrapolationPolicy,

    /// Allow deterministic fallbacks when SABR grid corners are missing during interpolation.
    ///
    /// If `false` (default), missing corner buckets are treated as an error instead of
    /// silently substituting a nearby bucket.
    #[serde(default)]
    pub allow_sabr_missing_bucket_fallback: bool,
}

/// Extrapolation policy for volatility surface construction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceExtrapolationPolicy {
    /// Reject out-of-bounds targets with an explicit error (vendor-matching).
    #[default]
    Error,
    /// Clamp targets to the nearest boundary (flat extrapolation).
    Clamp,
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
    /// Currency used for synthetic tranche pricing.
    pub currency: Currency,
    /// Notional used to price synthetic tranches during calibration.
    ///
    /// Calibration can be expressed in upfront % terms, so this is typically left
    /// as unit-notional unless you have a specific reason to change it.
    #[serde(default = "default_unit_notional")]
    pub notional: f64,
    /// Payment frequency for synthetic tranches (e.g., quarterly).
    #[serde(default)]
    pub payment_frequency: Option<Tenor>,
    /// Day count convention for synthetic tranche premium accrual.
    #[serde(default)]
    pub day_count: Option<DayCount>,
    /// Business day convention for synthetic tranche schedule adjustments.
    #[serde(default)]
    pub business_day_convention: Option<BusinessDayConvention>,
    /// Optional calendar identifier for schedule generation and date adjustments.
    #[serde(default)]
    pub calendar_id: Option<String>,
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
    /// Normal (absolute) volatility quoted in **basis points**.
    ///
    /// Example: `50.0` means 50bp = `0.0050` in internal model units.
    Normal,
    /// Lognormal (Black) volatility quoted as **percentage**.
    ///
    /// Example: `20.0` means 20% = `0.20` in internal model units.
    #[default]
    Lognormal,
    /// Shifted lognormal (Black) volatility quoted as **percentage**, with an explicit shift.
    ///
    /// Example: `20.0` means 20% = `0.20` in internal model units.
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

fn default_par_interp_linear() -> ParInterp {
    ParInterp::Linear
}

fn default_extrap_flat() -> ExtrapolationPolicy {
    ExtrapolationPolicy::FlatForward
}

fn default_recovery_04() -> f64 {
    0.4
}

fn default_unit_notional() -> f64 {
    1.0
}

#[allow(dead_code)]
fn default_sabr_beta() -> f64 {
    0.5
}
