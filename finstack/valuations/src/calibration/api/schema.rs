//! Calibration Schema.
//!
//! Defines the JSON contract for plan-driven calibration.

use crate::calibration::config::{CalibrationConfig, CalibrationMethod, RatesStepConventions};
use crate::calibration::CalibrationReport;
use crate::market::quotes::market_quote::MarketQuote;
use finstack_core::config::ResultsMeta;
use finstack_core::currency::Currency;
use finstack_core::dates::BusinessDayConvention;
use finstack_core::dates::{Date, DayCount, Tenor};
use finstack_core::market_data::context::MarketContextState;
use finstack_core::market_data::term_structures::{
    NelsonSiegelModel, NsVariant, ParInterp, Seniority,
};
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use finstack_core::types::{CurveId, IndexId};
use finstack_core::HashMap;
use serde::{Deserialize, Serialize};

/// Schema version identifier for calibration API.
pub const CALIBRATION_SCHEMA: &str = "finstack.calibration";

/// Complete calibration result with market snapshot and diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationResult {
    /// Final calibrated market context (all curves, surfaces, scalars, etc.)
    pub final_market: MarketContextState,
    /// Merged plan-level calibration report.
    pub report: CalibrationReport,
    /// Per-step calibration reports keyed by step id.
    pub step_reports: std::collections::BTreeMap<String, CalibrationReport>,
    /// Results metadata (timestamp, version, rounding context, etc.).
    pub results_meta: ResultsMeta,
}

/// Top-level envelope for calibration results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationResultEnvelope {
    /// Schema version identifier (must be "finstack.calibration").
    pub schema: String,
    /// The calibration result.
    pub result: CalibrationResult,
}

impl CalibrationResultEnvelope {
    /// Create a new result envelope.
    pub fn new(result: CalibrationResult) -> Self {
        Self {
            schema: CALIBRATION_SCHEMA.to_string(),
            result,
        }
    }
}

/// Top-level envelope for calibration requests.
///
/// This is the outer-most structure for a calibration request. It includes
/// the schema version, the plan to execute, and an optional initial market state
/// to build upon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationEnvelope {
    /// Schema version identifier (must be [`CALIBRATION_SCHEMA`]).
    pub schema: String,
    /// The calibration plan containing steps and quote data.
    pub plan: CalibrationPlan,
    /// Optional initial market context (e.g., existing curves) to use as a baseline.
    #[serde(default)]
    pub initial_market: Option<MarketContextState>,
}

/// A calibration plan containing quote sets and execution steps.
///
/// A plan organizes market data into named sets and defines a sequence of
/// [`CalibrationStep`] to be executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationPlan {
    /// Unique identifier for the calibration plan.
    pub id: String,
    /// Optional human-readable description of the plan's purpose.
    #[serde(default)]
    pub description: Option<String>,
    /// Market data organized by set name (referenced by steps).
    pub quote_sets: HashMap<String, Vec<MarketQuote>>,
    /// Sequence of calibration steps to execute.
    pub steps: Vec<CalibrationStep>,
    /// Global settings for the calibration process.
    #[serde(default)]
    pub settings: CalibrationConfig,
}

/// A single step in the calibration process.
///
/// Each step targets the construction or update of a specific market object
/// (e.g., a yield curve) using a specified set of quotes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationStep {
    /// Unique identifier for the object being calibrated in this step.
    pub id: String,
    /// Reference to a named quote set in the parent plan.
    pub quote_set: String,
    /// Step-specific parameters and configuration.
    #[serde(flatten)]
    pub params: StepParams,
}

/// Polymorphic parameters for different calibration step types.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Student-t copula degrees of freedom calibration.
    StudentT(StudentTParams),

    /// Hull-White 1-factor model calibration.
    HullWhite(HullWhiteStepParams),

    /// SVI volatility surface calibration.
    SviSurface(SviSurfaceParams),

    /// Cross-currency basis curve calibration.
    XccyBasis(XccyBasisParams),

    /// Parametric (Nelson-Siegel / NSS) curve calibration.
    Parametric(ParametricCurveParams),
}

// =============================================================================
// Step Parameter Structs
// =============================================================================

/// Parameters for discount curve calibration step.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Optional turn-of-year adjustment applied to the constructed discount curve.
    ///
    /// When present, year-end funding jumps are modeled as additive forward rate
    /// step functions over the specified windows. The adjustment is applied as a
    /// post-calibration modification to the discount factors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toy_adjustment: Option<crate::calibration::config::ToyAdjustment>,

    /// Optional Hull-White curve ID for computing futures convexity adjustments.
    ///
    /// When present, the calibrator will look up pre-calibrated HW1F parameters
    /// (κ, σ) from the market context to compute convexity adjustments for any
    /// futures quotes that do not already have an explicit `convexity_adjustment`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hull_white_curve_id: Option<CurveId>,
}

/// Parameters for forward curve calibration step.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Optional CDS doc clause / market convention identifier.
    ///
    /// This selects the pricing/schedule conventions for the synthetic CDS instruments used
    /// during calibration. If omitted, the default for the currency is used.
    ///
    /// Examples (current built-ins):
    /// - `"IsdaNa"` (USD/CAD default)
    /// - `"IsdaEu"` (EUR/GBP/CHF default)
    /// - `"IsdaAs"` (JPY/AUD/NZD/HKD/SGD default)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc_clause: Option<String>,
}

/// Parameters for inflation curve calibration step.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Optional seasonal adjustment factors for deseasonalizing CPI observations.
    ///
    /// When provided, the calibrator will:
    /// 1. Deseasonalize input CPI levels using the monthly factors
    /// 2. Fit the smooth zero-coupon curve to deseasonalized levels
    /// 3. Reseasonalize the output CPI path
    ///
    /// Monthly adjustments are additive to log CPI level. They should approximately
    /// sum to zero over 12 months.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seasonal_factors: Option<SeasonalFactors>,
}

/// Monthly seasonal adjustment factors for inflation curves.
///
/// Used to deseasonalize CPI observations before fitting a smooth
/// zero-coupon inflation curve, then reseasonalize the output.
/// Monthly adjustments should approximately sum to zero.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeasonalFactors {
    /// Monthly adjustment factors (Jan=index 0 through Dec=index 11).
    /// These are additive adjustments to the log CPI level.
    pub monthly_adjustments: [f64; 12],
}

/// Parameters for volatility surface calibration step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VolSurfaceParams {
    /// Identifier for the volatility surface being built.
    pub surface_id: String,
    /// Base date for the surface.
    pub base_date: Date,
    /// Identifier for the underlying instrument.
    #[serde(alias = "underlying_id")]
    pub underlying_ticker: String,
    /// Model type.
    ///
    /// Note: currently supports SABR-only; set to `"SABR"` (case-insensitive).
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Optional floating index identifier used to resolve market swap conventions.
    ///
    /// Swaption forward/par rate calculations require swap schedule conventions (fixed frequency,
    /// day count, calendar, BDC) that are now indexed off a rate index conventions registry.
    ///
    /// If omitted, individual swaption quotes must provide `float_leg_conventions.index`.
    #[serde(default)]
    pub swap_index: Option<IndexId>,
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(default, alias = "payment_frequency")]
    pub frequency: Option<Tenor>,
    /// Day count convention for synthetic tranche premium accrual.
    #[serde(default)]
    pub day_count: Option<DayCount>,
    /// Business day convention for synthetic tranche schedule adjustments.
    #[serde(default, alias = "business_day_convention")]
    pub bdc: Option<BusinessDayConvention>,
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

/// Parameters for Student-t copula degrees of freedom calibration.
///
/// Calibrates the `df` (degrees of freedom) parameter of a Student-t copula
/// by repricing a market tranche upfront quote and minimizing the residual
/// using Brent root-finding.
///
/// # Fields
///
/// - `tranche_instrument_id`: Identifier for the reference tranche instrument
///   used as the calibration target.
/// - `base_correlation_curve_id`: Identifier for the pre-calibrated base
///   correlation curve (must already exist in the market context).
/// - `initial_df`: Starting guess for the degrees of freedom (e.g., 5.0).
/// - `df_bounds`: Feasible domain for `df` as `(lo, hi)`, e.g., `(2.1, 50.0)`.
/// - `correlation`: Market-implied flat correlation for the tranche.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StudentTParams {
    /// Identifier for the reference tranche instrument.
    pub tranche_instrument_id: String,
    /// Identifier for the pre-calibrated base correlation curve.
    pub base_correlation_curve_id: String,
    /// Discount curve identifier used to price the tranche.
    ///
    /// When omitted, calibration falls back to the only discount curve present
    /// in the market context for backward compatibility.
    #[serde(default)]
    pub discount_curve_id: Option<CurveId>,
    /// Starting guess for degrees of freedom (typically 4-10).
    #[serde(default = "default_student_t_initial_df")]
    pub initial_df: f64,
    /// Feasible domain for `df` as `(lower_bound, upper_bound)`.
    ///
    /// `df` must be > 2 for finite variance. Typical range: `(2.1, 50.0)`.
    #[serde(default = "default_student_t_df_bounds")]
    pub df_bounds: (f64, f64),
    /// Market-implied flat correlation for the tranche.
    #[serde(default = "default_student_t_correlation")]
    pub correlation: f64,
}

fn default_student_t_initial_df() -> f64 {
    5.0
}

fn default_student_t_df_bounds() -> (f64, f64) {
    (2.1, 50.0)
}

fn default_student_t_correlation() -> f64 {
    0.3
}

/// Parameters for Hull-White 1-factor model calibration step.
///
/// Calibrates κ (mean reversion) and σ (short rate volatility) by fitting
/// European swaption market prices using Jamshidian decomposition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HullWhiteStepParams {
    /// Discount curve ID (must already exist in market context).
    pub curve_id: CurveId,
    /// Currency for conventions.
    pub currency: Currency,
    /// Base date for the calibration.
    pub base_date: Date,
    /// Optional initial guess for mean reversion κ.
    #[serde(default)]
    pub initial_kappa: Option<f64>,
    /// Optional initial guess for short rate vol σ.
    #[serde(default)]
    pub initial_sigma: Option<f64>,
}

/// Parameters for SVI volatility surface calibration step.
///
/// Fits a Stochastic Volatility Inspired (SVI) parameterization per-expiry
/// to market-implied volatilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SviSurfaceParams {
    /// Identifier for the volatility surface being built.
    pub surface_id: String,
    /// Base date for the surface.
    pub base_date: Date,
    /// Underlying instrument ticker.
    pub underlying_ticker: String,
    /// Discount curve ID (optional).
    #[serde(default)]
    pub discount_curve_id: Option<CurveId>,
    /// Target expiries for calibration.
    #[serde(default)]
    pub target_expiries: Vec<f64>,
    /// Target strikes for calibration.
    #[serde(default)]
    pub target_strikes: Vec<f64>,
    /// Optional spot price override.
    #[serde(default)]
    pub spot_override: Option<f64>,
}

/// Volatility quoting convention for swaptions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AtmStrikeConvention {
    /// ATM = forward swap rate (standard market convention)
    #[default]
    SwapRate,
    /// ATM = par swap rate (same as forward for zero-cost swap)
    ParRate,
}

/// Interpolation method for SABR parameters across the expiry–tenor grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SabrInterpolationMethod {
    /// Bilinear interpolation in (expiry, tenor) over SABR parameters.
    #[default]
    Bilinear,
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

fn default_sabr_beta() -> f64 {
    0.5
}

// =============================================================================
// XCCY Basis Curve
// =============================================================================

/// Parameters for cross-currency basis curve calibration step.
///
/// Derives a foreign-currency discount curve from a domestic OIS curve,
/// FX spot rate, and cross-currency basis swap or FX forward quotes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct XccyBasisParams {
    /// Identifier for the foreign discount curve being built.
    pub curve_id: CurveId,
    /// Foreign currency being calibrated.
    pub currency: Currency,
    /// Base date for the curve.
    pub base_date: Date,
    /// FX spot rate (domestic per foreign).
    pub fx_spot: f64,
    /// Identifier for the pre-calibrated domestic discount curve.
    pub domestic_discount_id: CurveId,
    /// Calibration method to use.
    #[serde(default)]
    pub method: CalibrationMethod,
    /// Interpolation style for the foreign curve.
    #[serde(default = "default_interp_linear")]
    pub interpolation: InterpStyle,
    /// Extrapolation policy for the foreign curve.
    #[serde(default = "default_extrap_flat")]
    pub extrapolation: ExtrapolationPolicy,
    /// Step-level conventions for pricing and curve time axis.
    #[serde(default)]
    pub conventions: RatesStepConventions,
    /// Optional ID for the byproduct basis spread curve.
    #[serde(default)]
    pub basis_spread_curve_id: Option<CurveId>,
}

// =============================================================================
// Parametric (NS / NSS) Curve
// =============================================================================

/// Parameters for parametric curve calibration step.
///
/// Fits a Nelson-Siegel or Nelson-Siegel-Svensson yield curve model to
/// rate instrument quotes using global (Levenberg-Marquardt) optimization.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParametricCurveParams {
    /// Identifier for the parametric curve being built.
    pub curve_id: CurveId,
    /// Base date for the curve.
    pub base_date: Date,
    /// Nelson-Siegel variant (NS or NSS).
    pub model: NsVariant,
    /// Optional separate discount curve ID for multi-curve instrument pricing.
    #[serde(default)]
    pub discount_curve_id: Option<CurveId>,
    /// Optional initial parameter guesses.
    #[serde(default)]
    pub initial_params: Option<NelsonSiegelModel>,
}
