//! Swaption (option on interest rate swap) implementation with SABR volatility.
//!
//! This module defines the `Swaption` data structure and integrates with the
//! common instrument trait via `impl_instrument!`. All pricing math is
//! implemented in the `pricing/` submodule; metrics are provided in the
//! `metrics/` submodule. The type exposes helper methods for forward swap
//! rate, annuity, and day-count based year fractions that reuse core library
//! functionality.

use crate::instruments::common_impl::helpers::year_fraction;
use crate::instruments::common_impl::models::{
    SABRModel, SABRParameters as InternalSabrParameters,
};
use crate::instruments::common_impl::parameters::OptionType;
use crate::instruments::common_impl::traits::Attributes;
use crate::instruments::pricing_overrides::VolSurfaceExtrapolation;
use crate::instruments::PricingOverrides;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::surfaces::VolSurfaceAxis;
use finstack_core::market_data::traits::Discounting;
use finstack_core::money::Money;
use finstack_core::types::{CalendarId, CurveId, InstrumentId};
use finstack_core::{Error, Result};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

use super::parameters::SwaptionParams;
use crate::impl_instrument_base;

/// Volatility model for pricing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum VolatilityModel {
    /// Black (Lognormal) model (1976)
    #[default]
    Black,
    /// Bachelier (Normal) model
    Normal,
}

/// Public SABR parameters for swaption volatility modeling.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SABRParameters {
    /// Initial volatility (alpha)
    pub alpha: f64,
    /// CEV exponent (beta) - typically 0 to 1
    pub beta: f64,
    /// Volatility of volatility (nu/volvol)
    pub nu: f64,
    /// Correlation between asset and volatility (rho)
    pub rho: f64,
    /// Shift parameter for handling negative rates (optional)
    pub shift: Option<f64>,
}

impl SABRParameters {
    /// Create new SABR parameters with validation.
    pub fn new(alpha: f64, beta: f64, nu: f64, rho: f64) -> Result<Self> {
        let _ = InternalSabrParameters::new(alpha, beta, nu, rho)?;
        Ok(Self {
            alpha,
            beta,
            nu,
            rho,
            shift: None,
        })
    }

    /// Create new SABR parameters with a shift for negative rates.
    pub fn new_with_shift(alpha: f64, beta: f64, nu: f64, rho: f64, shift: f64) -> Result<Self> {
        let _ = InternalSabrParameters::new_with_shift(alpha, beta, nu, rho, shift)?;
        Ok(Self {
            alpha,
            beta,
            nu,
            rho,
            shift: Some(shift),
        })
    }

    /// Create SABR parameters with equity market standard (beta=1.0).
    pub fn equity_standard(alpha: f64, nu: f64, rho: f64) -> Result<Self> {
        let _ = InternalSabrParameters::equity_standard(alpha, nu, rho)?;
        Ok(Self {
            alpha,
            beta: 1.0,
            nu,
            rho,
            shift: None,
        })
    }

    /// Create SABR parameters with interest rate market standard (beta=0.5).
    pub fn rates_standard(alpha: f64, nu: f64, rho: f64) -> Result<Self> {
        let _ = InternalSabrParameters::rates_standard(alpha, nu, rho)?;
        Ok(Self {
            alpha,
            beta: 0.5,
            nu,
            rho,
            shift: None,
        })
    }

    /// Create SABR parameters with normal model convention (beta=0.0).
    pub fn normal(alpha: f64, nu: f64, rho: f64) -> Result<Self> {
        let _ = InternalSabrParameters::normal(alpha, nu, rho)?;
        Ok(Self {
            alpha,
            beta: 0.0,
            nu,
            rho,
            shift: None,
        })
    }

    /// Create SABR parameters with lognormal model convention (beta=1.0).
    pub fn lognormal(alpha: f64, nu: f64, rho: f64) -> Result<Self> {
        let _ = InternalSabrParameters::lognormal(alpha, nu, rho)?;
        Ok(Self {
            alpha,
            beta: 1.0,
            nu,
            rho,
            shift: None,
        })
    }

    pub(crate) fn to_internal(&self) -> Result<InternalSabrParameters> {
        match self.shift {
            Some(shift) => InternalSabrParameters::new_with_shift(
                self.alpha, self.beta, self.nu, self.rho, shift,
            ),
            None => InternalSabrParameters::new(self.alpha, self.beta, self.nu, self.rho),
        }
    }
}

impl std::fmt::Display for VolatilityModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VolatilityModel::Black => write!(f, "black"),
            VolatilityModel::Normal => write!(f, "normal"),
        }
    }
}

impl std::str::FromStr for VolatilityModel {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let normalized = s.to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "black" | "lognormal" | "black76" => Ok(Self::Black),
            "normal" | "bachelier" => Ok(Self::Normal),
            other => Err(format!(
                "Unknown volatility model: '{}'. Valid: black, normal",
                other
            )),
        }
    }
}

/// Swaption settlement method
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum SwaptionSettlement {
    /// Physical settlement (enter into underlying swap)
    Physical,
    /// Cash settlement (receive NPV of swap)
    Cash,
}

/// Cash settlement annuity method for cash-settled swaptions.
///
/// Different methods exist for calculating the annuity factor used in cash settlement:
///
/// # Market Background
///
/// When a swaption is cash-settled, the payoff is:
/// ```text
/// Payoff = Annuity × max(S - K, 0)  [for payer]
/// ```
///
/// The choice of annuity method affects the settlement amount and can result
/// in differences of several basis points on notional for steep curves.
///
/// # ⚠️ Production Recommendation
///
/// For production systems requiring ISDA compliance, use [`IsdaParPar`](Self::IsdaParPar):
///
/// ```rust,ignore
/// let swaption = Swaption::example()
///     .with_cash_settlement_method(CashSettlementMethod::IsdaParPar);
/// ```
///
/// The default `ParYield` method is a fast approximation suitable for:
/// - Quick calculations and screening
/// - Flat yield curve environments
/// - Short-dated swaptions where precision is less critical
///
/// # References
///
/// - ISDA 2006 Definitions, Section 18.2
/// - "Interest Rate Models" by Brigo & Mercurio, Chapter 6
/// - Bloomberg VCUB/SWPM: Uses ISDA Par-Par for production
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CashSettlementMethod {
    /// Par yield approximation using flat forward rate.
    ///
    /// ```text
    /// A = (1 - (1 + S/m)^(-N)) / S
    /// ```
    ///
    /// This is a closed-form approximation that assumes the forward swap rate
    /// is a constant discount rate. Fast but less accurate for steep curves.
    ///
    /// **Note**: This was the legacy default. As of the market standards audit,
    /// [`IsdaParPar`](Self::IsdaParPar) is now the default for ISDA compliance.
    ParYield,

    /// ISDA Par-Par method using actual swap annuity from discount curve.
    ///
    /// ```text
    /// A = Σ τ_i × DF(t_i)
    /// ```
    ///
    /// Uses the actual market discount factors to compute the annuity,
    /// matching the PV01 of the underlying swap. This is the most accurate
    /// method and matches professional library implementations.
    ///
    /// # ✅ Default (ISDA Compliant)
    ///
    /// This is the default method, matching professional library implementations
    /// (Bloomberg VCUB/SWPM, QuantLib). Suitable for:
    /// - Production pricing requiring ISDA compliance
    /// - Steep yield curve environments
    /// - Long-dated swaptions (> 5Y into > 10Y swap)
    /// - Trade confirmation matching
    /// - Any situation where cash settlement valuation precision matters
    #[default]
    IsdaParPar,

    /// Zero coupon method discounting the single payment to swap maturity.
    ///
    /// ```text
    /// A = τ × DF(T_swap)
    /// ```
    ///
    /// Rarely used in modern markets; included for completeness.
    ZeroCoupon,
}

impl std::fmt::Display for CashSettlementMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CashSettlementMethod::ParYield => write!(f, "par_yield"),
            CashSettlementMethod::IsdaParPar => write!(f, "isda_par_par"),
            CashSettlementMethod::ZeroCoupon => write!(f, "zero_coupon"),
        }
    }
}

impl std::str::FromStr for CashSettlementMethod {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let normalized = s.to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "par_yield" | "paryield" => Ok(Self::ParYield),
            "isda_par_par" | "isdaparpar" | "par_par" => Ok(Self::IsdaParPar),
            "zero_coupon" | "zerocoupon" => Ok(Self::ZeroCoupon),
            other => Err(format!(
                "Unknown cash settlement method: '{}'. Valid: par_yield, isda_par_par, zero_coupon",
                other
            )),
        }
    }
}

impl std::fmt::Display for SwaptionSettlement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SwaptionSettlement::Physical => write!(f, "physical"),
            SwaptionSettlement::Cash => write!(f, "cash"),
        }
    }
}

impl std::str::FromStr for SwaptionSettlement {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let normalized = s.to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "physical" => Ok(SwaptionSettlement::Physical),
            "cash" => Ok(SwaptionSettlement::Cash),
            other => Err(format!("Unknown swaption settlement: {}", other)),
        }
    }
}

/// Swaption exercise style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum SwaptionExercise {
    /// European exercise (only at expiry)
    #[default]
    European,
    /// Bermudan exercise (at discrete dates)
    Bermudan,
    /// American exercise (any time before expiry)
    American,
}

impl std::fmt::Display for SwaptionExercise {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SwaptionExercise::European => write!(f, "european"),
            SwaptionExercise::Bermudan => write!(f, "bermudan"),
            SwaptionExercise::American => write!(f, "american"),
        }
    }
}

impl std::str::FromStr for SwaptionExercise {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let normalized = s.to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "european" => Ok(SwaptionExercise::European),
            "bermudan" => Ok(SwaptionExercise::Bermudan),
            "american" => Ok(SwaptionExercise::American),
            other => Err(format!("Unknown swaption exercise: {}", other)),
        }
    }
}

// ============================================================================
// Bermudan Swaption Types
// ============================================================================

/// Bermudan exercise schedule specification.
///
/// Defines the exercise dates and constraints for a Bermudan swaption.
/// Exercise dates are typically aligned with swap coupon dates.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BermudanSchedule {
    /// Exercise dates (must be sorted, typically on swap coupon dates)
    pub exercise_dates: Vec<Date>,
    /// Lockout period end (no exercise before this date)
    pub lockout_end: Option<Date>,
    /// Notice period in business days before exercise
    pub notice_days: u32,
}

impl BermudanSchedule {
    /// Create a new Bermudan schedule with the given exercise dates.
    ///
    /// # Arguments
    /// * `exercise_dates` - Exercise dates (will be sorted)
    pub fn new(mut exercise_dates: Vec<Date>) -> Self {
        exercise_dates.sort();
        Self {
            exercise_dates,
            lockout_end: None,
            notice_days: 0,
        }
    }

    /// Create schedule with lockout period.
    pub fn with_lockout(mut self, lockout_end: Date) -> Self {
        self.lockout_end = Some(lockout_end);
        self
    }

    /// Create schedule with notice period.
    pub fn with_notice_days(mut self, days: u32) -> Self {
        self.notice_days = days;
        self
    }

    /// Generate co-terminal exercise dates from swap schedule.
    ///
    /// Creates exercise dates on each fixed leg payment date from `first_exercise`
    /// to `swap_end`, excluding the final payment date (swap maturity).
    ///
    /// # Arguments
    /// * `first_exercise` - First allowed exercise date
    /// * `swap_end` - Swap maturity date
    /// * `fixed_freq` - Fixed leg payment frequency
    pub fn co_terminal(
        first_exercise: Date,
        swap_end: Date,
        fixed_freq: Tenor,
    ) -> finstack_core::Result<Self> {
        let sched = crate::cashflow::builder::build_dates(
            first_exercise,
            swap_end,
            fixed_freq,
            StubKind::None,
            BusinessDayConvention::ModifiedFollowing, // Market standard per ISDA
            false,
            0,
            crate::cashflow::builder::calendar::WEEKENDS_ONLY_ID,
        )?;
        // Exercise dates are all coupon dates except the last one (maturity),
        // but always include the first_exercise date when it is before swap_end.
        let mut exercise_dates: Vec<Date> = Vec::new();
        if first_exercise < swap_end {
            exercise_dates.push(first_exercise);
        }
        exercise_dates.extend(
            sched
                .dates
                .into_iter()
                .filter(|&d| d > first_exercise && d < swap_end),
        );
        Ok(Self::new(exercise_dates))
    }

    /// Get effective exercise dates (filtered by lockout).
    pub fn effective_dates(&self) -> Vec<Date> {
        match self.lockout_end {
            Some(lockout) => self
                .exercise_dates
                .iter()
                .filter(|&&d| d > lockout)
                .copied()
                .collect(),
            None => self.exercise_dates.clone(),
        }
    }

    /// Convert exercise dates to year fractions from a given date.
    pub fn exercise_times(&self, as_of: Date, day_count: DayCount) -> Result<Vec<f64>> {
        let ctx = finstack_core::dates::DayCountCtx::default();
        self.effective_dates()
            .iter()
            .map(|&d| day_count.year_fraction(as_of, d, ctx))
            .collect()
    }

    /// Number of exercise opportunities.
    pub fn num_exercises(&self) -> usize {
        self.effective_dates().len()
    }
}

/// Co-terminal vs non-co-terminal Bermudan exercise.
///
/// This distinction affects pricing methodology and calibration:
/// - Co-terminal: All exercise dates lead to the same swap end date
/// - Non-co-terminal: Each exercise date may have a different remaining swap tenor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum BermudanType {
    /// All exercise dates lead to same swap end date (most common)
    #[default]
    CoTerminal,
    /// Exercise dates may have different swap end dates
    NonCoTerminal,
}

impl std::fmt::Display for BermudanType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BermudanType::CoTerminal => write!(f, "co_terminal"),
            BermudanType::NonCoTerminal => write!(f, "non_co_terminal"),
        }
    }
}

impl std::str::FromStr for BermudanType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let normalized = s.to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "co_terminal" | "coterminal" => Ok(Self::CoTerminal),
            "non_co_terminal" | "noncoterminal" => Ok(Self::NonCoTerminal),
            other => Err(format!(
                "Unknown Bermudan type: '{}'. Valid: co_terminal, non_co_terminal",
                other
            )),
        }
    }
}

/// Swaption instrument
#[derive(
    Clone, Debug, finstack_valuations_macros::FinancialBuilder, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
pub struct Swaption {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Option type (payer or receiver swaption)
    pub option_type: OptionType,
    /// Notional amount of underlying swap
    pub notional: Money,
    /// Strike (fixed rate on underlying swap)
    #[serde(alias = "strike_rate")]
    pub strike: Decimal,
    /// Option expiry date
    pub expiry: Date,
    /// Underlying swap start date
    pub swap_start: Date,
    /// Underlying swap end date
    pub swap_end: Date,
    /// Fixed leg payment frequency
    pub fixed_freq: Tenor,
    /// Floating leg payment frequency
    pub float_freq: Tenor,
    /// Day count convention
    pub day_count: DayCount,
    /// Exercise style (European, Bermudan, American). Defaults to European.
    #[serde(default, alias = "exercise")]
    #[builder(default)]
    pub exercise_style: SwaptionExercise,
    /// Settlement method (physical or cash)
    pub settlement: SwaptionSettlement,
    /// Cash settlement annuity method (only used when settlement = Cash).
    ///
    /// - `ParYield` (default): Fast approximation using flat forward rate
    /// - `IsdaParPar`: Uses actual swap annuity from discount curve (ISDA compliant)
    /// - `ZeroCoupon`: Discounts to swap maturity (rarely used)
    #[serde(default)]
    pub cash_settlement_method: CashSettlementMethod,
    /// Volatility model (Black or Normal)
    #[serde(default)]
    pub vol_model: VolatilityModel,
    /// Discount curve ID for present value calculations
    pub discount_curve_id: CurveId,
    /// Forward curve ID for floating rate projections
    #[serde(alias = "forward_id")]
    pub forward_curve_id: CurveId,
    /// Volatility surface ID for option pricing
    pub vol_surface_id: CurveId,
    /// Holiday calendar ID for schedule generation.
    ///
    /// Controls business day adjustment and payment date calculation for the
    /// underlying swap schedule. When `None`, uses weekends-only calendar
    /// (no holiday adjustments). For production use, set to the appropriate
    /// currency calendar (e.g., `"nyse"` for USD, `"target"` for EUR).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let swaption = Swaption::example()
    ///     .with_calendar("nyse");
    /// ```
    #[serde(default)]
    pub calendar_id: Option<CalendarId>,
    /// Pricing overrides (manual price, yield, spread)
    #[serde(default)]
    #[builder(default)]
    pub pricing_overrides: PricingOverrides,
    /// Optional SABR volatility model parameters
    pub sabr_params: Option<SABRParameters>,
    /// Attributes for scenario selection and grouping
    #[serde(default)]
    #[builder(default)]
    pub attributes: Attributes,
}

impl Swaption {
    pub(crate) fn strike_f64(&self) -> Result<f64> {
        self.strike.to_f64().ok_or_else(|| {
            Error::Validation("Swaption strike could not be converted to f64".to_string())
        })
    }

    /// Create a canonical example swaption for testing and documentation.
    ///
    /// Returns a 1Y x 5Y payer swaption (1 year to expiry, 5 year swap tenor).
    #[allow(clippy::expect_used)] // Example uses hardcoded valid values
    pub fn example() -> Self {
        Self {
            id: InstrumentId::new("SWPN-1Yx5Y-USD"),
            option_type: OptionType::Call,
            notional: Money::new(10_000_000.0, Currency::USD),
            strike: Decimal::try_from(0.03).expect("valid decimal"),
            expiry: Date::from_calendar_date(2027, time::Month::January, 15)
                .expect("Valid example date"),
            swap_start: Date::from_calendar_date(2027, time::Month::January, 17)
                .expect("Valid example date"),
            swap_end: Date::from_calendar_date(2032, time::Month::January, 17)
                .expect("Valid example date"),
            fixed_freq: Tenor::semi_annual(),
            float_freq: Tenor::quarterly(),
            day_count: DayCount::Thirty360,
            exercise_style: SwaptionExercise::European,
            settlement: SwaptionSettlement::Cash,
            cash_settlement_method: CashSettlementMethod::default(),
            vol_model: VolatilityModel::Black,
            discount_curve_id: CurveId::new("USD-OIS"),
            forward_curve_id: CurveId::new("USD-OIS"),
            vol_surface_id: CurveId::new("USD-SWPNVOL"),
            calendar_id: None,
            pricing_overrides: PricingOverrides::default(),
            sabr_params: None,
            attributes: Attributes::new(),
        }
    }

    /// Create a new payer swaption using parameter structs.
    pub fn new_payer(
        id: impl Into<InstrumentId>,
        params: &SwaptionParams,
        discount_curve_id: impl Into<CurveId>,
        forward_curve_id: impl Into<CurveId>,
        vol_surface_id: impl Into<CurveId>,
    ) -> Self {
        let mut s = Self {
            id: id.into(),
            option_type: OptionType::Call,
            notional: params.notional,
            strike: params.strike,
            expiry: params.expiry,
            swap_start: params.swap_start,
            swap_end: params.swap_end,
            fixed_freq: Tenor::semi_annual(),
            float_freq: Tenor::quarterly(),
            day_count: DayCount::Thirty360,
            exercise_style: SwaptionExercise::European,
            settlement: SwaptionSettlement::Physical,
            cash_settlement_method: CashSettlementMethod::default(),
            discount_curve_id: discount_curve_id.into(),
            forward_curve_id: forward_curve_id.into(),
            vol_surface_id: vol_surface_id.into(),
            calendar_id: None,
            pricing_overrides: PricingOverrides::default(),
            sabr_params: None,
            attributes: Attributes::default(),
            vol_model: Default::default(),
        };
        if let Some(f) = params.fixed_freq {
            s.fixed_freq = f;
        }
        if let Some(f) = params.float_freq {
            s.float_freq = f;
        }
        if let Some(dc) = params.day_count {
            s.day_count = dc;
        }
        if let Some(vm) = params.vol_model {
            s.vol_model = vm;
        }
        s
    }

    /// Create a new receiver swaption using parameter structs.
    pub fn new_receiver(
        id: impl Into<InstrumentId>,
        params: &SwaptionParams,
        discount_curve_id: impl Into<CurveId>,
        forward_curve_id: impl Into<CurveId>,
        vol_surface_id: impl Into<CurveId>,
    ) -> Self {
        let mut s = Self {
            id: id.into(),
            option_type: OptionType::Put,
            notional: params.notional,
            strike: params.strike,
            expiry: params.expiry,
            swap_start: params.swap_start,
            swap_end: params.swap_end,
            fixed_freq: Tenor::semi_annual(),
            float_freq: Tenor::quarterly(),
            day_count: DayCount::Thirty360,
            exercise_style: SwaptionExercise::European,
            settlement: SwaptionSettlement::Physical,
            cash_settlement_method: CashSettlementMethod::default(),
            discount_curve_id: discount_curve_id.into(),
            forward_curve_id: forward_curve_id.into(),
            vol_surface_id: vol_surface_id.into(),
            calendar_id: None,
            pricing_overrides: PricingOverrides::default(),
            sabr_params: None,
            attributes: Attributes::default(),
            vol_model: Default::default(),
        };
        if let Some(f) = params.fixed_freq {
            s.fixed_freq = f;
        }
        if let Some(f) = params.float_freq {
            s.float_freq = f;
        }
        if let Some(dc) = params.day_count {
            s.day_count = dc;
        }
        if let Some(vm) = params.vol_model {
            s.vol_model = vm;
        }
        s
    }

    /// Attach SABR parameters to enable SABR-implied volatility pricing.
    pub fn with_sabr(mut self, params: SABRParameters) -> Self {
        self.sabr_params = Some(params);
        self
    }

    /// Override the exercise style (default: European).
    pub fn with_exercise_style(mut self, style: SwaptionExercise) -> Self {
        self.exercise_style = style;
        self
    }

    /// Override the settlement type (default: Physical).
    pub fn with_settlement(mut self, settlement: SwaptionSettlement) -> Self {
        self.settlement = settlement;
        self
    }

    /// Override the option type (Call = payer, Put = receiver).
    pub fn with_option_type(mut self, option_type: OptionType) -> Self {
        self.option_type = option_type;
        self
    }

    /// Set the holiday calendar for schedule generation.
    ///
    /// # Arguments
    /// * `calendar_id` - Calendar ID registered in `CalendarRegistry`
    ///   (e.g., `"nyse"` for USD, `"target"` for EUR)
    pub fn with_calendar(mut self, calendar_id: impl Into<CalendarId>) -> Self {
        self.calendar_id = Some(calendar_id.into());
        self
    }

    /// Resolve the effective calendar ID for schedule generation.
    ///
    /// Returns the user-configured calendar or falls back to weekends-only.
    fn effective_calendar_id(&self) -> &str {
        self.calendar_id
            .as_deref()
            .unwrap_or(crate::cashflow::builder::calendar::WEEKENDS_ONLY_ID)
    }

    /// Set the cash settlement annuity method.
    ///
    /// Only affects pricing when `settlement` is `SwaptionSettlement::Cash`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use finstack_valuations::instruments::rates::swaption::{Swaption, CashSettlementMethod};
    ///
    /// // Create a cash-settled swaption with ISDA Par-Par settlement
    /// let swaption = Swaption::example()
    ///     .with_cash_settlement_method(CashSettlementMethod::IsdaParPar);
    /// ```
    pub fn with_cash_settlement_method(mut self, method: CashSettlementMethod) -> Self {
        self.cash_settlement_method = method;
        self
    }

    // ============================================================================
    // Pricing Methods (moved from engine for direct access)
    // ============================================================================

    /// Helper for common pricing logic
    fn price_model_base<F>(
        &self,
        curves: &MarketContext,
        volatility: f64,
        as_of: Date,
        model_fn: F,
    ) -> Result<Money>
    where
        F: Fn(f64, f64, f64, f64, f64) -> f64, // forward, strike, vol, t, annuity -> value
    {
        let time_to_expiry = year_fraction(self.day_count, as_of, self.expiry)?;
        if time_to_expiry <= 0.0 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }

        let disc = curves.get_discount(self.discount_curve_id.as_ref())?;
        let forward_rate = self.forward_swap_rate(curves, as_of)?;
        let annuity = self.annuity(disc.as_ref(), as_of, forward_rate)?;
        let strike = self.strike_f64()?;

        let value = model_fn(forward_rate, strike, volatility, time_to_expiry, annuity);

        Ok(Money::new(
            value * self.notional.amount(),
            self.notional.currency(),
        ))
    }

    /// Black (lognormal) model PV.
    pub fn price_black(
        &self,
        curves: &MarketContext,
        volatility: f64,
        as_of: Date,
    ) -> Result<Money> {
        let time_to_expiry = year_fraction(self.day_count, as_of, self.expiry)?;
        if time_to_expiry <= 0.0 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }

        let strike = self.strike_f64()?;
        let forward = self.forward_swap_rate(curves, as_of)?;
        if forward <= 0.0 || strike <= 0.0 {
            return Err(finstack_core::Error::Validation(format!(
                "Black swaption pricing requires positive forward and strike, got forward={} strike={}",
                forward, strike
            )));
        }

        self.price_model_base(curves, volatility, as_of, |fwd, strike, vol, t, annuity| {
            // Use stable handling if volatility is near zero
            if vol <= 0.0 || !vol.is_finite() {
                // Intrinsic value
                let val = match self.option_type {
                    OptionType::Call => (fwd - strike).max(0.0),
                    OptionType::Put => (strike - fwd).max(0.0),
                };
                return val * annuity;
            }

            // Use centralized Black76 helpers for forward-based pricing
            use crate::instruments::common_impl::models::{d1_black76, d2_black76};
            let d1 = d1_black76(fwd, strike, vol, t);
            let d2 = d2_black76(fwd, strike, vol, t);

            match self.option_type {
                OptionType::Call => {
                    annuity
                        * (fwd * finstack_core::math::norm_cdf(d1)
                            - strike * finstack_core::math::norm_cdf(d2))
                }
                OptionType::Put => {
                    annuity
                        * (strike * finstack_core::math::norm_cdf(-d2)
                            - fwd * finstack_core::math::norm_cdf(-d1))
                }
            }
        })
    }

    /// Bachelier (normal) model PV.
    pub fn price_normal(
        &self,
        curves: &MarketContext,
        volatility: f64,
        as_of: Date,
    ) -> Result<Money> {
        self.price_model_base(curves, volatility, as_of, |fwd, strike, vol, t, annuity| {
            use crate::instruments::common_impl::models::volatility::normal::bachelier_price;
            bachelier_price(self.option_type, fwd, strike, vol, t, annuity)
        })
    }

    /// SABR-implied volatility PV with model-aware pricing.
    ///
    /// The SABR formula (Hagan 2002) outputs lognormal (Black) volatility by default.
    /// When `vol_model == Normal`, we convert the lognormal vol to approximate
    /// normal (Bachelier) vol using the standard approximation:
    ///
    /// ```text
    /// σ_normal ≈ σ_lognormal × forward × (1 - ε) where ε is a small correction
    /// ```
    ///
    /// For ATM options, this approximation is exact. For OTM/ITM options,
    /// the approximation is accurate to within a few basis points for typical
    /// market conditions.
    ///
    /// # Negative Rates
    ///
    /// When SABR `shift` is set, the lognormal-to-normal conversion operates on
    /// shifted rates (F + shift, K + shift) which are guaranteed positive.
    /// Without a shift, non-positive rates fall back to a crude approximation.
    /// For negative-rate currencies (EUR, JPY, CHF), always use shifted SABR
    /// via [`SABRParameters::new_with_shift`].
    ///
    /// # References
    ///
    /// - Hagan, P. et al. (2002). "Managing Smile Risk" *Wilmott Magazine*
    /// - Antonov, A. et al. (2015). "SABR/Free Sabr" for normal vol extensions
    pub fn price_sabr(&self, curves: &MarketContext, as_of: Date) -> Result<Money> {
        let params = self.sabr_params.as_ref().ok_or(Error::Internal)?;
        let model = SABRModel::new(params.to_internal()?);
        let time_to_expiry = year_fraction(self.day_count, as_of, self.expiry)?;
        if time_to_expiry <= 0.0 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }
        let forward_rate = self.forward_swap_rate(curves, as_of)?;
        let strike = self.strike_f64()?;

        // SABR outputs lognormal (Black) volatility
        let sabr_lognormal_vol = model.implied_volatility(forward_rate, strike, time_to_expiry)?;

        // Dispatch to the appropriate pricing model
        match self.vol_model {
            VolatilityModel::Black => self.price_black(curves, sabr_lognormal_vol, as_of),
            VolatilityModel::Normal => {
                let sabr_normal_vol = lognormal_to_normal_vol(
                    sabr_lognormal_vol,
                    forward_rate,
                    strike,
                    time_to_expiry,
                    params.shift,
                );
                self.price_normal(curves, sabr_normal_vol, as_of)
            }
        }
    }

    /// Calculate annuity based on settlement type and cash settlement method.
    ///
    /// # Settlement Types
    ///
    /// - **Physical**: Always uses `swap_annuity()` (actual PV01 from discount curve)
    /// - **Cash**: Uses the method specified by `cash_settlement_method`:
    ///   - `ParYield`: Closed-form approximation (fast, less accurate for steep curves)
    ///   - `IsdaParPar`: Actual swap annuity from discount curve (ISDA compliant)
    ///   - `ZeroCoupon`: Single discount to swap maturity (rarely used)
    pub fn annuity(&self, disc: &dyn Discounting, as_of: Date, forward_rate: f64) -> Result<f64> {
        match self.settlement {
            SwaptionSettlement::Physical => self.swap_annuity(disc, as_of),
            SwaptionSettlement::Cash => match self.cash_settlement_method {
                CashSettlementMethod::ParYield => self.cash_annuity_par_yield(forward_rate),
                CashSettlementMethod::IsdaParPar => self.swap_annuity(disc, as_of),
                CashSettlementMethod::ZeroCoupon => self.cash_annuity_zero_coupon(disc, as_of),
            },
        }
    }

    /// Discounted fixed-leg PV01 (annuity) of the underlying swap schedule (Physical Settlement).
    ///
    /// # Time Basis
    ///
    /// Uses curve-consistent relative discount factors via `relative_df_discounting`:
    /// - DF from `as_of` to each payment date is computed using the discount curve's
    ///   own base_date and day_count (not the instrument's day_count).
    /// - Accrual fractions use the instrument's day_count (correct for coupon calculation).
    pub fn swap_annuity(&self, disc: &dyn Discounting, as_of: Date) -> Result<f64> {
        use crate::instruments::common_impl::pricing::time::relative_df_discounting;
        use finstack_core::math::NeumaierAccumulator;

        let mut annuity = NeumaierAccumulator::new();
        let sched = crate::cashflow::builder::build_dates(
            self.swap_start,
            self.swap_end,
            self.fixed_freq,
            StubKind::None,
            BusinessDayConvention::ModifiedFollowing, // Market standard per ISDA
            false,
            0,
            self.effective_calendar_id(),
        )?;
        let dates = sched.dates;
        if dates.len() < 2 {
            return Ok(0.0);
        }
        let mut prev = dates[0];
        for window in dates.windows(2) {
            let d = window[1];
            // Accrual uses instrument's day count (correct for coupon calculation)
            let accrual = year_fraction(self.day_count, prev, d)?;
            // DF uses curve-consistent relative DF (correct for discounting)
            let df = relative_df_discounting(disc, as_of, d)?;
            annuity.add(accrual * df);
            prev = d;
        }
        Ok(annuity.total())
    }

    /// Cash settlement annuity using par yield approximation.
    ///
    /// # Formula
    ///
    /// ```text
    /// A = (1 - (1 + S/m)^(-N)) / S
    /// ```
    ///
    /// where:
    /// - S = forward swap rate (settlement rate)
    /// - m = payment frequency per year
    /// - N = total number of payment periods
    ///
    /// # Approximation Notes
    ///
    /// This formula assumes:
    /// 1. **Flat forward rate**: The swap rate S is used as a constant discount rate
    ///    across all periods. This is an approximation when the yield curve is not flat.
    /// 2. **Equal periods**: All accrual periods are assumed equal (no stubs).
    ///
    /// For production systems requiring exact ISDA compliance, use
    /// `cash_settlement_method: CashSettlementMethod::IsdaParPar` which delegates
    /// to `swap_annuity`.
    ///
    /// # Edge Cases
    ///
    /// When `forward_rate ≈ 0`, uses L'Hôpital's limit: `A → N/m` (sum of accruals).
    pub fn cash_annuity_par_yield(&self, forward_rate: f64) -> Result<f64> {
        let freq_per_year = match self.fixed_freq.unit {
            finstack_core::dates::TenorUnit::Months if self.fixed_freq.count > 0 => {
                12.0 / self.fixed_freq.count as f64
            }
            finstack_core::dates::TenorUnit::Days if self.fixed_freq.count > 0 => {
                365.0 / self.fixed_freq.count as f64
            }
            finstack_core::dates::TenorUnit::Years if self.fixed_freq.count > 0 => {
                1.0 / self.fixed_freq.count as f64
            }
            finstack_core::dates::TenorUnit::Weeks if self.fixed_freq.count > 0 => {
                52.0 / self.fixed_freq.count as f64
            }
            _ => {
                return Err(Error::Validation(
                    "Invalid frequency in cash annuity".into(),
                ))
            }
        };

        if forward_rate.abs() < 1e-8 {
            // L'Hopital's limit for S -> 0: A = N/m (sum of accruals)
            // We need number of periods.
            let tenor = year_fraction(self.day_count, self.swap_start, self.swap_end)?;
            let periods = freq_per_year * tenor;
            return Ok(periods / freq_per_year);
        }

        let tenor_years = year_fraction(self.day_count, self.swap_start, self.swap_end)?;
        let n_periods = tenor_years * freq_per_year;

        let df_swap = (1.0 + forward_rate / freq_per_year).powf(-n_periods);
        Ok((1.0 - df_swap) / forward_rate)
    }

    /// Cash settlement annuity using zero coupon method.
    ///
    /// # Formula
    ///
    /// ```text
    /// A = τ × DF(T_swap)
    /// ```
    ///
    /// where:
    /// - τ = total swap tenor as year fraction
    /// - DF(T_swap) = discount factor to swap maturity
    ///
    /// This method treats the entire swap as a single zero-coupon payment
    /// at maturity. Rarely used in modern markets; included for completeness.
    pub fn cash_annuity_zero_coupon(&self, disc: &dyn Discounting, as_of: Date) -> Result<f64> {
        use crate::instruments::common_impl::pricing::time::relative_df_discounting;

        let tenor = year_fraction(self.day_count, self.swap_start, self.swap_end)?;
        let df = relative_df_discounting(disc, as_of, self.swap_end)?;
        Ok(tenor * df)
    }

    /// Forward par swap rate implied by float-leg PV and fixed-leg annuity.
    ///
    /// # Time Basis
    ///
    /// Uses curve-consistent time mapping:
    /// - Discount factors use the discount curve's own base_date/day_count
    /// - Forward rates use the forward curve's own base_date/day_count
    ///
    /// # Formula
    ///
    /// ```text
    /// S = PV_float / Annuity
    /// ```
    ///
    /// where:
    /// - PV_float = Σ (accrual_i × forward_i × DF_i)
    /// - Annuity = Σ (accrual_i × DF_i) for all fixed leg payments.
    pub fn forward_swap_rate(&self, curves: &MarketContext, as_of: Date) -> Result<f64> {
        use crate::instruments::common_impl::pricing::time::{
            rate_period_on_dates, relative_df_discounting,
        };

        let disc = curves.get_discount(self.discount_curve_id.as_ref())?;
        let annuity = self.swap_annuity(disc.as_ref(), as_of)?;
        if annuity.abs() < 1e-10 {
            return Ok(0.0);
        }

        // Single-curve optimization
        if self.forward_curve_id == self.discount_curve_id {
            let df_start = relative_df_discounting(disc.as_ref(), as_of, self.swap_start)?;
            let df_end = relative_df_discounting(disc.as_ref(), as_of, self.swap_end)?;
            return Ok((df_start - df_end) / annuity);
        }

        let fwd = curves.get_forward(self.forward_curve_id.as_ref())?;
        let fwd_dc = fwd.day_count();
        let sched = crate::cashflow::builder::build_dates(
            self.swap_start,
            self.swap_end,
            self.float_freq,
            StubKind::None,
            BusinessDayConvention::ModifiedFollowing, // Market standard per ISDA
            false,
            0,
            self.effective_calendar_id(),
        )?;

        let mut pv_float = 0.0;
        let mut prev = self.swap_start;
        for &d in sched.dates.iter().skip(1) {
            let accrual =
                fwd_dc.year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())?;
            let fwd_rate = rate_period_on_dates(fwd.as_ref(), prev, d)?;
            let df = relative_df_discounting(disc.as_ref(), as_of, d)?;
            pv_float += accrual * fwd_rate * df;
            prev = d;
        }

        Ok(pv_float / annuity)
    }

    /// Resolve volatility from SABR parameters, pricing override, or volatility surface.
    ///
    /// This consolidates the volatility resolution logic used by Greek calculators.
    /// Priority order:
    /// 1. SABR model parameters (if set)
    /// 2. Pricing override implied volatility (if set)
    /// 3. Volatility surface lookup
    ///
    /// # Arguments
    /// * `curves` - Market context containing volatility surfaces
    /// * `forward` - Forward swap rate
    /// * `time_to_expiry` - Time to option expiry in years
    ///
    /// # Returns
    /// Resolved volatility value
    pub fn resolve_volatility(
        &self,
        curves: &MarketContext,
        forward: f64,
        time_to_expiry: f64,
    ) -> Result<f64> {
        // 1. SABR model (highest priority)
        if let Some(sabr) = &self.sabr_params {
            let model = SABRModel::new(sabr.to_internal()?);
            return model.implied_volatility(forward, self.strike_f64()?, time_to_expiry);
        }

        // 2. Pricing override
        if let Some(impl_vol) = self.pricing_overrides.market_quotes.implied_volatility {
            return Ok(impl_vol);
        }

        // 3. Volatility surface
        let vol_surface = curves.get_surface(self.vol_surface_id.as_str())?;
        vol_surface.require_secondary_axis(VolSurfaceAxis::Strike)?;
        let strike = self.strike_f64()?;
        match self
            .pricing_overrides
            .model_config
            .vol_surface_extrapolation
        {
            VolSurfaceExtrapolation::Clamp | VolSurfaceExtrapolation::LinearInVariance => {
                // LinearInVariance falls back to Clamp until surface impl is ready
                Ok(vol_surface.value_clamped(time_to_expiry, strike))
            }
            VolSurfaceExtrapolation::Error => {
                Ok(vol_surface.value_checked(time_to_expiry, strike)?)
            }
        }
    }

    /// Pre-compute common Greek calculation inputs.
    ///
    /// Returns `None` if the option has expired (time_to_expiry <= 0).
    /// This consolidates the setup logic shared across delta, gamma, vega, and rho calculators.
    ///
    /// # Arguments
    /// * `curves` - Market context containing curves and surfaces
    /// * `as_of` - Valuation date
    ///
    /// # Returns
    /// `Some(GreekInputs)` containing forward, annuity, sigma, and time to expiry,
    /// or `None` if the option has expired.
    pub fn greek_inputs(&self, curves: &MarketContext, as_of: Date) -> Result<Option<GreekInputs>> {
        let disc = curves.get_discount(self.discount_curve_id.as_ref())?;
        if as_of >= self.expiry {
            return Ok(None);
        }
        let t = year_fraction(self.day_count, as_of, self.expiry)?;

        if t <= 0.0 {
            return Ok(None);
        }

        let forward = self.forward_swap_rate(curves, as_of)?;
        let annuity = self.annuity(disc.as_ref(), as_of, forward)?;
        let sigma = self.resolve_volatility(curves, forward, t)?;

        Ok(Some(GreekInputs {
            forward,
            annuity,
            sigma,
            time_to_expiry: t,
        }))
    }
}

/// Pre-computed inputs for Greek calculations.
///
/// This struct contains the common values needed by delta, gamma, vega,
/// and other Greek calculators, avoiding redundant computation.
#[derive(Debug, Clone, Copy)]
pub struct GreekInputs {
    /// Forward swap rate
    pub forward: f64,
    /// Swap annuity (PV01 or cash annuity depending on settlement)
    pub annuity: f64,
    /// Resolved volatility (from SABR, override, or surface)
    pub sigma: f64,
    /// Time to option expiry in years
    pub time_to_expiry: f64,
}

impl crate::instruments::common_impl::traits::Instrument for Swaption {
    impl_instrument_base!(crate::pricer::InstrumentType::Swaption);

    fn value(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::pricing_overrides::VolSurfaceExtrapolation;

        // 1. SABR model (if enabled) overrides basic model choice
        if self.sabr_params.is_some() {
            return self.price_sabr(curves, as_of);
        }

        let time_to_expiry = year_fraction(self.day_count, as_of, self.expiry)?;
        let vol_surface = curves.get_surface(self.vol_surface_id.as_str())?;
        vol_surface.require_secondary_axis(VolSurfaceAxis::Strike)?;
        let strike = self.strike_f64()?;
        let vol = if let Some(impl_vol) = self.pricing_overrides.market_quotes.implied_volatility {
            impl_vol
        } else {
            match self
                .pricing_overrides
                .model_config
                .vol_surface_extrapolation
            {
                VolSurfaceExtrapolation::Clamp | VolSurfaceExtrapolation::LinearInVariance => {
                    // LinearInVariance falls back to Clamp until surface impl is ready
                    vol_surface.value_clamped(time_to_expiry, strike)
                }
                VolSurfaceExtrapolation::Error => {
                    vol_surface.value_checked(time_to_expiry, strike)?
                }
            }
        };

        match self.vol_model {
            VolatilityModel::Black => self.price_black(curves, vol, as_of),
            VolatilityModel::Normal => self.price_normal(curves, vol, as_of),
        }
    }

    fn expiry(&self) -> Option<finstack_core::dates::Date> {
        Some(self.expiry)
    }

    fn effective_start_date(&self) -> Option<finstack_core::dates::Date> {
        Some(self.swap_start)
    }

    fn scenario_overrides_mut(
        &mut self,
    ) -> Option<&mut crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&mut self.pricing_overrides)
    }

    fn scenario_overrides(
        &self,
    ) -> Option<&crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&self.pricing_overrides)
    }
}

// Implement CurveDependencies for DV01 calculator
impl crate::instruments::common_impl::traits::CurveDependencies for Swaption {
    fn curve_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::traits::InstrumentCurves> {
        crate::instruments::common_impl::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .forward(self.forward_curve_id.clone())
            .build()
    }
}

// ============================================================================
// Bermudan Swaption Instrument
// ============================================================================

/// Bermudan swaption with multiple exercise dates.
///
/// A Bermudan swaption gives the holder the right to enter into an interest rate
/// swap at any of a set of predetermined exercise dates. This is the most common
/// type of exotic swaption in the market, used extensively for:
///
/// - Callable bond hedging
/// - Mortgage prepayment risk management
/// - Structured product hedging
///
/// # Pricing Methods
///
/// Bermudan swaptions require numerical methods for pricing:
/// - **Hull-White Tree**: Industry standard, calibrated to swaption volatility
/// - **LSMC**: Longstaff-Schwartz Monte Carlo for validation
///
/// # Example
///
/// ```rust,no_run
/// use finstack_valuations::instruments::rates::swaption::{
///     BermudanSwaption, BermudanSchedule, BermudanType, SwaptionSettlement,
/// };
///
/// // Create a 10NC2 (10-year swap, callable after 2 years)
/// let swaption = BermudanSwaption::example();
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BermudanSwaption {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Option type (payer = Call, receiver = Put)
    pub option_type: OptionType,
    /// Notional amount of underlying swap
    pub notional: Money,
    /// Strike (fixed rate on underlying swap)
    #[serde(alias = "strike_rate")]
    pub strike: Decimal,
    /// Underlying swap start date (first accrual start)
    pub swap_start: Date,
    /// Underlying swap end date (final payment)
    pub swap_end: Date,
    /// Fixed leg payment frequency
    pub fixed_freq: Tenor,
    /// Floating leg payment frequency
    pub float_freq: Tenor,
    /// Day count convention for fixed leg
    pub day_count: DayCount,
    /// Settlement method (physical or cash)
    pub settlement: SwaptionSettlement,
    /// Discount curve ID for present value calculations
    pub discount_curve_id: CurveId,
    /// Forward curve ID for floating rate projections
    #[serde(alias = "forward_id")]
    pub forward_curve_id: CurveId,
    /// Volatility surface ID for calibration
    pub vol_surface_id: CurveId,
    /// Bermudan exercise schedule
    pub bermudan_schedule: BermudanSchedule,
    /// Co-terminal or non-co-terminal exercise
    pub bermudan_type: BermudanType,
    /// Holiday calendar ID for schedule generation.
    ///
    /// Controls business day adjustment for the underlying swap schedule.
    /// When `None`, uses weekends-only calendar. For production use, set to
    /// the appropriate currency calendar (e.g., `"nyse"` for USD).
    #[serde(default)]
    pub calendar_id: Option<CalendarId>,
    /// Pricing overrides (manual price, yield, spread)
    #[serde(default)]
    pub pricing_overrides: PricingOverrides,
    /// Attributes for scenario selection and grouping
    #[serde(default)]
    /// Attributes for scenario selection and tagging
    pub attributes: Attributes,
}

impl BermudanSwaption {
    /// Create a canonical example Bermudan swaption for testing.
    ///
    /// Returns a 10NC2 payer swaption (10-year swap, callable quarterly after 2 years).
    #[allow(clippy::expect_used)] // Example uses hardcoded valid values
    pub fn example() -> Self {
        let swap_start =
            Date::from_calendar_date(2027, time::Month::January, 17).expect("Valid example date");
        let swap_end =
            Date::from_calendar_date(2037, time::Month::January, 17).expect("Valid example date");
        let first_exercise =
            Date::from_calendar_date(2029, time::Month::January, 17).expect("Valid example date");

        Self {
            id: InstrumentId::new("BERM-10NC2-USD"),
            option_type: OptionType::Call,
            notional: Money::new(10_000_000.0, Currency::USD),
            strike: Decimal::try_from(0.03).expect("valid decimal"),
            swap_start,
            swap_end,
            fixed_freq: Tenor::semi_annual(),
            float_freq: Tenor::quarterly(),
            day_count: DayCount::Thirty360,
            settlement: SwaptionSettlement::Physical,
            discount_curve_id: CurveId::new("USD-OIS"),
            forward_curve_id: CurveId::new("USD-OIS"),
            vol_surface_id: CurveId::new("USD-SWPNVOL"),
            bermudan_schedule: BermudanSchedule::co_terminal(
                first_exercise,
                swap_end,
                Tenor::semi_annual(),
            )
            .expect("valid Bermudan schedule"),
            bermudan_type: BermudanType::CoTerminal,
            calendar_id: None,
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::new(),
        }
    }

    /// Create a new Bermudan payer swaption (right to pay fixed).
    ///
    /// Returns an error if the strike value is not representable as `Decimal` (e.g., NaN or Inf).
    #[allow(clippy::too_many_arguments)]
    pub fn new_payer(
        id: impl Into<InstrumentId>,
        notional: Money,
        strike: f64,
        swap_start: Date,
        swap_end: Date,
        bermudan_schedule: BermudanSchedule,
        discount_curve_id: impl Into<CurveId>,
        forward_curve_id: impl Into<CurveId>,
        vol_surface_id: impl Into<CurveId>,
    ) -> finstack_core::Result<Self> {
        Ok(Self {
            id: id.into(),
            option_type: OptionType::Call,
            notional,
            strike: crate::utils::decimal::f64_to_decimal(strike, "strike")?,
            swap_start,
            swap_end,
            fixed_freq: Tenor::semi_annual(),
            float_freq: Tenor::quarterly(),
            day_count: DayCount::Thirty360,
            settlement: SwaptionSettlement::Physical,
            discount_curve_id: discount_curve_id.into(),
            forward_curve_id: forward_curve_id.into(),
            vol_surface_id: vol_surface_id.into(),
            bermudan_schedule,
            bermudan_type: BermudanType::CoTerminal,
            calendar_id: None,
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::default(),
        })
    }

    /// Create a new Bermudan receiver swaption (right to receive fixed).
    ///
    /// Returns an error if the strike value is not representable as `Decimal` (e.g., NaN or Inf).
    #[allow(clippy::too_many_arguments)]
    pub fn new_receiver(
        id: impl Into<InstrumentId>,
        notional: Money,
        strike: f64,
        swap_start: Date,
        swap_end: Date,
        bermudan_schedule: BermudanSchedule,
        discount_curve_id: impl Into<CurveId>,
        forward_curve_id: impl Into<CurveId>,
        vol_surface_id: impl Into<CurveId>,
    ) -> finstack_core::Result<Self> {
        Ok(Self {
            id: id.into(),
            option_type: OptionType::Put,
            notional,
            strike: crate::utils::decimal::f64_to_decimal(strike, "strike")?,
            swap_start,
            swap_end,
            fixed_freq: Tenor::semi_annual(),
            float_freq: Tenor::quarterly(),
            day_count: DayCount::Thirty360,
            settlement: SwaptionSettlement::Physical,
            discount_curve_id: discount_curve_id.into(),
            forward_curve_id: forward_curve_id.into(),
            vol_surface_id: vol_surface_id.into(),
            bermudan_schedule,
            bermudan_type: BermudanType::CoTerminal,
            calendar_id: None,
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::default(),
        })
    }

    /// Set fixed leg frequency.
    pub fn with_fixed_freq(mut self, freq: Tenor) -> Self {
        self.fixed_freq = freq;
        self
    }

    /// Set floating leg frequency.
    pub fn with_float_freq(mut self, freq: Tenor) -> Self {
        self.float_freq = freq;
        self
    }

    /// Set day count convention.
    pub fn with_day_count(mut self, dc: DayCount) -> Self {
        self.day_count = dc;
        self
    }

    /// Set settlement method.
    pub fn with_settlement(mut self, settlement: SwaptionSettlement) -> Self {
        self.settlement = settlement;
        self
    }

    /// Set Bermudan type (co-terminal or non-co-terminal).
    pub fn with_bermudan_type(mut self, bermudan_type: BermudanType) -> Self {
        self.bermudan_type = bermudan_type;
        self
    }

    /// Set the holiday calendar for schedule generation.
    pub fn with_calendar(mut self, calendar_id: impl Into<CalendarId>) -> Self {
        self.calendar_id = Some(calendar_id.into());
        self
    }

    /// Resolve the effective calendar ID for schedule generation.
    fn effective_calendar_id(&self) -> &str {
        self.calendar_id
            .as_deref()
            .unwrap_or(crate::cashflow::builder::calendar::WEEKENDS_ONLY_ID)
    }

    /// Get the first exercise date.
    pub fn first_exercise(&self) -> Option<Date> {
        self.bermudan_schedule.effective_dates().first().copied()
    }

    /// Get the last exercise date.
    pub fn last_exercise(&self) -> Option<Date> {
        self.bermudan_schedule.effective_dates().last().copied()
    }

    /// Calculate time to first exercise in years.
    pub fn time_to_first_exercise(&self, as_of: Date) -> Result<f64> {
        match self.first_exercise() {
            Some(first) => {
                if as_of >= first {
                    return Ok(0.0);
                }
                self.day_count.year_fraction(
                    as_of,
                    first,
                    finstack_core::dates::DayCountCtx::default(),
                )
            }
            None => Err(Error::Validation("No exercise dates".into())),
        }
    }

    /// Calculate time to swap maturity in years.
    pub fn time_to_maturity(&self, as_of: Date) -> Result<f64> {
        if as_of >= self.swap_end {
            return Ok(0.0);
        }
        self.day_count.year_fraction(
            as_of,
            self.swap_end,
            finstack_core::dates::DayCountCtx::default(),
        )
    }

    /// Get exercise dates as year fractions from valuation date.
    pub fn exercise_times(&self, as_of: Date) -> Result<Vec<f64>> {
        self.bermudan_schedule.exercise_times(as_of, self.day_count)
    }

    /// Build the underlying swap payment schedule.
    ///
    /// Returns (payment_dates, accrual_fractions) for the fixed leg.
    pub fn build_swap_schedule(&self, _as_of: Date) -> Result<(Vec<Date>, Vec<f64>)> {
        let periods = crate::cashflow::builder::periods::build_periods(
            crate::cashflow::builder::periods::BuildPeriodsParams {
                start: self.swap_start,
                end: self.swap_end,
                frequency: self.fixed_freq,
                stub: StubKind::None,
                bdc: BusinessDayConvention::ModifiedFollowing, // Market standard per ISDA
                calendar_id: self.effective_calendar_id(),
                end_of_month: false,
                day_count: self.day_count,
                payment_lag_days: 0,
                reset_lag_days: None,
            },
        )?;

        if periods.is_empty() {
            return Err(Error::Validation(
                "Swap schedule has fewer than 2 dates".into(),
            ));
        }

        let dates: Vec<Date> = periods.iter().map(|p| p.payment_date).collect();
        let accruals: Vec<f64> = periods.iter().map(|p| p.accrual_year_fraction).collect();

        Ok((dates, accruals))
    }

    /// Convert payment dates to year fractions.
    pub fn payment_times(&self, as_of: Date) -> Result<Vec<f64>> {
        let (dates, _) = self.build_swap_schedule(as_of)?;
        let ctx = finstack_core::dates::DayCountCtx::default();
        dates
            .iter()
            .map(|&d| self.day_count.year_fraction(as_of, d, ctx))
            .collect()
    }

    pub(crate) fn strike_f64(&self) -> Result<f64> {
        self.strike.to_f64().ok_or_else(|| {
            Error::Validation("BermudanSwaption strike could not be converted to f64".into())
        })
    }

    /// Forward swap rate at a given exercise date (multi-curve).
    ///
    /// For co-terminal swaptions, the swap always matures at `swap_end`.
    /// For non-co-terminal, each exercise date may have different remaining tenor.
    ///
    /// # Time Basis
    ///
    /// Uses curve-consistent time mapping:
    /// - Discount factors use the discount curve's own base_date/day_count
    /// - Forward rates use the forward curve's own base_date/day_count
    pub fn forward_swap_rate(
        &self,
        curves: &MarketContext,
        as_of: Date,
        exercise_date: Date,
    ) -> Result<f64> {
        use crate::instruments::common_impl::pricing::time::{
            rate_period_on_dates, relative_df_discounting,
        };

        let disc = curves.get_discount(self.discount_curve_id.as_ref())?;
        let annuity = self.remaining_annuity(disc.as_ref(), as_of, exercise_date)?;

        if annuity.abs() < 1e-10 {
            return Ok(0.0);
        }

        // Single-curve optimization
        if self.forward_curve_id == self.discount_curve_id {
            let df_start = relative_df_discounting(disc.as_ref(), as_of, exercise_date)?;
            let df_end = relative_df_discounting(disc.as_ref(), as_of, self.swap_end)?;
            return Ok((df_start - df_end) / annuity);
        }

        let fwd = curves.get_forward(self.forward_curve_id.as_ref())?;
        let fwd_dc = fwd.day_count();
        let periods = crate::cashflow::builder::periods::build_periods(
            crate::cashflow::builder::periods::BuildPeriodsParams {
                start: exercise_date,
                end: self.swap_end,
                frequency: self.float_freq,
                stub: StubKind::None,
                bdc: BusinessDayConvention::ModifiedFollowing, // Market standard per ISDA
                calendar_id: self.effective_calendar_id(),
                end_of_month: false,
                day_count: fwd_dc,
                payment_lag_days: 0,
                reset_lag_days: None,
            },
        )?;

        let mut pv_float = 0.0;
        for period in periods {
            let fwd_rate =
                rate_period_on_dates(fwd.as_ref(), period.accrual_start, period.accrual_end)?;
            let df = relative_df_discounting(disc.as_ref(), as_of, period.payment_date)?;
            pv_float += period.accrual_year_fraction * fwd_rate * df;
        }

        Ok(pv_float / annuity)
    }

    /// Calculate annuity for remaining swap payments after exercise date.
    ///
    /// # Time Basis
    ///
    /// Uses curve-consistent relative discount factors:
    /// - DF from `as_of` to each payment date computed using the discount curve's
    ///   own base_date and day_count.
    /// - Accrual fractions use the instrument's day_count (correct for coupon calculation).
    pub fn remaining_annuity(
        &self,
        disc: &dyn Discounting,
        as_of: Date,
        exercise_date: Date,
    ) -> Result<f64> {
        use crate::instruments::common_impl::pricing::time::relative_df_discounting;

        let (dates, accruals) = self.build_swap_schedule(as_of)?;

        let mut annuity = 0.0;
        for (d, tau) in dates.iter().zip(accruals.iter()) {
            if *d > exercise_date {
                let df = relative_df_discounting(disc, as_of, *d)?;
                annuity += tau * df;
            }
        }

        Ok(annuity)
    }

    /// Convert to European swaption for the first exercise date.
    ///
    /// Useful for calibration and testing.
    pub fn to_european(&self) -> Result<Swaption> {
        let first_ex = self
            .first_exercise()
            .ok_or_else(|| Error::Validation("No exercise dates".into()))?;

        Ok(Swaption {
            id: InstrumentId::new(format!("{}-EURO", self.id.as_str())),
            option_type: self.option_type,
            notional: self.notional,
            strike: self.strike,
            expiry: first_ex,
            swap_start: first_ex,
            swap_end: self.swap_end,
            fixed_freq: self.fixed_freq,
            float_freq: self.float_freq,
            day_count: self.day_count,
            exercise_style: SwaptionExercise::European,
            settlement: self.settlement,
            cash_settlement_method: CashSettlementMethod::default(),
            vol_model: VolatilityModel::Black,
            discount_curve_id: self.discount_curve_id.clone(),
            forward_curve_id: self.forward_curve_id.clone(),
            vol_surface_id: self.vol_surface_id.clone(),
            calendar_id: self.calendar_id.clone(),
            pricing_overrides: self.pricing_overrides.clone(),
            sabr_params: None,
            attributes: self.attributes.clone(),
        })
    }
}

impl crate::instruments::common_impl::traits::Instrument for BermudanSwaption {
    impl_instrument_base!(crate::pricer::InstrumentType::BermudanSwaption);

    fn value(
        &self,
        _curves: &finstack_core::market_data::context::MarketContext,
        _as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        // Bermudan swaptions require tree or MC pricing - delegate to pricer
        Err(Error::Validation(
            "BermudanSwaption requires tree or LSMC pricing via BermudanSwaptionPricer".into(),
        ))
    }

    fn price_with_metrics(
        &self,
        _curves: &finstack_core::market_data::context::MarketContext,
        _as_of: finstack_core::dates::Date,
        _metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        Err(Error::Validation(
            "BermudanSwaption requires tree or LSMC pricing via BermudanSwaptionPricer".into(),
        ))
    }

    fn effective_start_date(&self) -> Option<finstack_core::dates::Date> {
        Some(self.swap_start)
    }

    fn scenario_overrides_mut(
        &mut self,
    ) -> Option<&mut crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&mut self.pricing_overrides)
    }

    fn scenario_overrides(
        &self,
    ) -> Option<&crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&self.pricing_overrides)
    }
}

impl crate::instruments::common_impl::traits::CurveDependencies for BermudanSwaption {
    fn curve_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::traits::InstrumentCurves> {
        crate::instruments::common_impl::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .forward(self.forward_curve_id.clone())
            .build()
    }
}

/// Convert lognormal (Black) volatility to normal (Bachelier) volatility.
///
/// Uses the Brenner-Subrahmanyam (1988) / Hagan (2002) approximation with
/// second-order correction. When a SABR shift is provided, the conversion
/// operates on shifted rates (F + shift, K + shift), ensuring positivity
/// even for negative-rate environments.
///
/// # Arguments
///
/// * `sigma_ln` - Lognormal (Black) volatility
/// * `forward` - Forward swap rate
/// * `strike` - Strike rate
/// * `time_to_expiry` - Time to option expiry in years
/// * `shift` - Optional SABR shift for negative rate handling
///
/// # Formula
///
/// For ATM (F = K):
/// ```text
/// σ_normal ≈ σ_lognormal × F_eff × [1 - σ²T/24]
/// ```
///
/// For general F ≠ K:
/// ```text
/// σ_normal ≈ σ_lognormal × (F_eff - K_eff) / ln(F_eff/K_eff)
///             × [1 - σ²T/24 × (1 - ln²(F_eff/K_eff)/12)]
/// ```
///
/// where F_eff = F + shift, K_eff = K + shift when shift is provided.
///
/// # References
///
/// - Brenner, M. & Subrahmanyam, M.G. (1988). "A Simple Formula to Compute
///   the Implied Standard Deviation"
/// - Hagan, P. et al. (2002). "Managing Smile Risk" Wilmott Magazine
/// - Jaeckel, P. (2017). "Let's Be Rational" for exact conversion
fn lognormal_to_normal_vol(
    sigma_ln: f64,
    forward: f64,
    strike: f64,
    time_to_expiry: f64,
    shift: Option<f64>,
) -> f64 {
    // Apply shift to ensure positive rates for the lognormal-to-normal mapping.
    // Shifted SABR models define F_eff = F + shift, K_eff = K + shift where
    // shift is chosen so that both are positive (e.g., shift = 3% for EUR).
    let (f, k) = match shift {
        Some(s) => (forward + s, strike + s),
        None => (forward, strike),
    };

    let variance = sigma_ln * sigma_ln * time_to_expiry;

    if f <= 0.0 || k <= 0.0 {
        // Without shift, non-positive rates can't use the lognormal approximation.
        // Fall back to linear approximation using the arithmetic mean of absolute
        // values. This is crude and will produce unreliable normal vols -- callers
        // should supply a SABR shift for negative-rate currencies instead.
        //
        // WARNING: This fallback is inherently unreliable. For negative-rate
        // currencies (EUR, JPY, CHF), always configure `SABRParameters.shift`
        // so that F + shift and K + shift are positive.
        let effective_level = ((f.abs() + k.abs()) / 2.0).max(1e-6);
        return sigma_ln * effective_level;
    }

    let log_fk = (f / k).ln();

    // Moneyness-adjusted forward level
    // For ATM: limit of (F-K)/ln(F/K) as K→F is F
    // For non-ATM: this gives the "effective" forward for normal vol
    let effective_forward = if log_fk.abs() < 1e-8 {
        // Near ATM: use Taylor expansion to avoid 0/0
        // (F-K)/ln(F/K) ≈ F × [1 - ln(F/K)/2 + ln(F/K)²/12 - ...]
        f * (1.0 - log_fk / 2.0 + log_fk * log_fk / 12.0)
    } else {
        (f - k) / log_fk
    };

    // Second-order correction from Hagan (2002):
    // The correction accounts for the difference in convexity between
    // lognormal and normal models. For typical parameters this is ~0.1-1%.
    //
    // Correction = 1 - σ²T/24 × [1 - (1/12)(ln(F/K))²]
    //
    // For extreme parameters (σ²T > 12), the raw correction becomes negative.
    // We floor at 0.5 to keep the result positive and bounded. This floor only
    // activates for unrealistic combinations (e.g., 80% vol + 30Y tenor) where
    // the second-order approximation itself has broken down anyway.
    let moneyness_factor = 1.0 - log_fk * log_fk / 12.0;
    let correction = if variance > 1e-10 {
        let raw = 1.0 - (variance / 24.0) * moneyness_factor;
        raw.max(0.5)
    } else {
        1.0
    };

    sigma_ln * effective_forward * correction
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    //! Tests for SABR-to-normal vol conversion.
    //!
    //! The conversion formula is validated against:
    //! 1. ATM limit: σ_N ≈ σ_LN × F (simple approximation)
    //! 2. Non-ATM: σ_N ≈ σ_LN × (F-K)/ln(F/K) × [1 - σ²T/24 × (1 - ln²(F/K)/12)]
    //! 3. Convergence: as K → F, the formula converges to the ATM limit

    use super::lognormal_to_normal_vol;

    /// Test the lognormal-to-normal vol conversion formula at ATM.
    ///
    /// At ATM (F = K), the formula should give:
    /// σ_N ≈ σ_LN × F × (1 - σ_LN²T/24)
    #[test]
    fn test_lognormal_to_normal_vol_atm() {
        let f: f64 = 0.03; // 3% forward rate
        let sigma_ln: f64 = 0.20; // 20% lognormal vol
        let t: f64 = 1.0; // 1 year

        // Expected: σ_N ≈ σ_LN × F × (1 - σ²T/24)
        let correction = 1.0 - (sigma_ln * sigma_ln * t) / 24.0;
        let expected_sigma_n = sigma_ln * f * correction;

        let computed_sigma_n = lognormal_to_normal_vol(sigma_ln, f, f, t, None);

        // Should be very close at ATM
        assert!(
            (computed_sigma_n - expected_sigma_n).abs() < 1e-10,
            "ATM vol conversion failed: computed={:.6}, expected={:.6}",
            computed_sigma_n,
            expected_sigma_n
        );
    }

    /// Test the lognormal-to-normal vol conversion formula for OTM options.
    #[test]
    fn test_lognormal_to_normal_vol_otm() {
        let f: f64 = 0.03; // 3% forward rate
        let k: f64 = 0.04; // 4% strike (OTM call / ITM put)
        let sigma_ln: f64 = 0.20; // 20% lognormal vol
        let t: f64 = 1.0; // 1 year

        let sigma_n = lognormal_to_normal_vol(sigma_ln, f, k, t, None);

        // Normal vol should be positive and reasonable
        assert!(sigma_n > 0.0, "Normal vol should be positive");
        // Normal vol for rates is typically in bp terms (0.001 = 10bp)
        // For 20% lognormal vol on 3% rates, expect ~60bp = 0.006
        assert!(
            sigma_n > 0.002 && sigma_n < 0.02,
            "Normal vol {} seems unreasonable for 20% lognormal on 3% rates",
            sigma_n
        );
    }

    /// Test that the formula converges smoothly as K → F (no discontinuity).
    #[test]
    fn test_lognormal_to_normal_vol_convergence() {
        let f: f64 = 0.03;
        let sigma_ln: f64 = 0.20;
        let t: f64 = 1.0;

        // Compute at exactly ATM
        let sigma_n_atm = lognormal_to_normal_vol(sigma_ln, f, f, t, None);

        // Compute at K very close to F
        for delta in [1e-6_f64, 1e-8_f64, 1e-10_f64] {
            let k = f * (1.0 + delta);
            let sigma_n = lognormal_to_normal_vol(sigma_ln, f, k, t, None);

            // Should converge to ATM value
            let diff = (sigma_n - sigma_n_atm).abs();
            assert!(
                diff < delta * 10.0,
                "Convergence failure at delta={}: diff={:.2e}",
                delta,
                diff
            );
        }
    }

    /// Test that the correction factor stays in reasonable bounds.
    #[test]
    fn test_correction_factor_bounds() {
        // High vol, long maturity: correction should be floored near 0.5
        let f: f64 = 0.03;
        let sigma_ln: f64 = 0.80; // 80% vol (extreme)
        let t: f64 = 30.0; // 30 years

        let sigma_n = lognormal_to_normal_vol(sigma_ln, f, f, t, None);

        // Even with extreme parameters, result should be positive and bounded
        assert!(sigma_n > 0.0, "Normal vol should be positive");

        // The correction should floor near 0.5, so normal vol ≈ σ_LN × F × 0.5
        let approx_floor = sigma_ln * f * 0.5;
        assert!(
            sigma_n >= approx_floor * 0.9, // Allow some tolerance from hard floor at 0.5
            "Correction floor should prevent unreasonably low vol: got {}, expected >= {}",
            sigma_n,
            approx_floor * 0.9
        );
    }

    /// Test shifted SABR lognormal-to-normal conversion for negative rates.
    ///
    /// With a shift, negative rates become positive in the shifted domain,
    /// allowing the standard lognormal-to-normal approximation to apply.
    #[test]
    fn test_lognormal_to_normal_vol_shifted_negative_rates() {
        // EUR-like scenario: negative forward and strike
        let f: f64 = -0.005; // -0.5% forward rate
        let k: f64 = -0.003; // -0.3% strike
        let sigma_ln: f64 = 0.30; // 30% lognormal vol (on shifted rates)
        let t: f64 = 1.0;
        let shift = 0.03; // 3% shift (standard for EUR)

        let sigma_n = lognormal_to_normal_vol(sigma_ln, f, k, t, Some(shift));

        // With shift: F_eff = -0.5% + 3% = 2.5%, K_eff = -0.3% + 3% = 2.7%
        // Both positive, so standard approximation applies
        assert!(sigma_n > 0.0, "Normal vol should be positive with shift");

        // For 30% lognormal vol on ~2.5% shifted rates, expect ~75bp = 0.0075
        assert!(
            sigma_n > 0.003 && sigma_n < 0.02,
            "Shifted normal vol {} seems unreasonable",
            sigma_n
        );

        // Without shift, should still produce a positive result (fallback)
        let sigma_n_no_shift = lognormal_to_normal_vol(sigma_ln, f, k, t, None);
        assert!(
            sigma_n_no_shift > 0.0,
            "Fallback should produce positive vol"
        );
    }

    /// Test that shifted conversion is consistent with unshifted for positive rates.
    #[test]
    fn test_shifted_vs_unshifted_positive_rates() {
        let f: f64 = 0.03;
        let k: f64 = 0.035;
        let sigma_ln: f64 = 0.20;
        let t: f64 = 1.0;

        // With zero shift, should give same result as no shift
        let sigma_n_none = lognormal_to_normal_vol(sigma_ln, f, k, t, None);
        let sigma_n_zero = lognormal_to_normal_vol(sigma_ln, f, k, t, Some(0.0));

        assert!(
            (sigma_n_none - sigma_n_zero).abs() < 1e-12,
            "Zero shift should match no shift: none={}, zero={}",
            sigma_n_none,
            sigma_n_zero
        );
    }
}
