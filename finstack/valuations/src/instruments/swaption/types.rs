//! Swaption (option on interest rate swap) implementation with SABR volatility.
//!
//! This module defines the `Swaption` data structure and integrates with the
//! common instrument trait via `impl_instrument!`. All pricing math is
//! implemented in the `pricing/` submodule; metrics are provided in the
//! `metrics/` submodule. The type exposes helper methods for forward swap
//! rate, annuity, and day-count based year fractions that reuse core library
//! functionality.

use crate::instruments::common::models::{SABRModel, SABRParameters};
use crate::instruments::common::parameters::OptionType;
use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::traits::Discounting;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::{Error, Result};

use super::parameters::SwaptionParams;

/// Volatility model for pricing
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum VolatilityModel {
    /// Black (Lognormal) model (1976)
    #[default]
    Black,
    /// Bachelier (Normal) model
    Normal,
}

impl std::fmt::Display for VolatilityModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VolatilityModel::Black => write!(f, "black"),
            VolatilityModel::Normal => write!(f, "normal"),
        }
    }
}

/// Swaption settlement type
/// Swaption settlement method
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SwaptionSettlement {
    /// Physical settlement (enter into underlying swap)
    Physical,
    /// Cash settlement (receive NPV of swap)
    Cash,
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
        match s.to_ascii_lowercase().as_str() {
            "physical" => Ok(SwaptionSettlement::Physical),
            "cash" => Ok(SwaptionSettlement::Cash),
            other => Err(format!("Unknown swaption settlement: {}", other)),
        }
    }
}

/// Swaption exercise style
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SwaptionExercise {
    /// European exercise (only at expiry)
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
        match s.to_ascii_lowercase().as_str() {
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
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    pub fn co_terminal(first_exercise: Date, swap_end: Date, fixed_freq: Frequency) -> Self {
        let sched = crate::cashflow::builder::build_dates(
            first_exercise,
            swap_end,
            fixed_freq,
            StubKind::None,
            BusinessDayConvention::Following,
            None,
        );
        // Exercise dates are all coupon dates except the last one (maturity)
        let exercise_dates: Vec<Date> = sched
            .dates
            .into_iter()
            .filter(|&d| d >= first_exercise && d < swap_end)
            .collect();
        Self::new(exercise_dates)
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
            BermudanType::CoTerminal => write!(f, "co-terminal"),
            BermudanType::NonCoTerminal => write!(f, "non-co-terminal"),
        }
    }
}

/// Swaption instrument
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Swaption {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Option type (payer or receiver swaption)
    pub option_type: OptionType,
    /// Notional amount of underlying swap
    pub notional: Money,
    /// Strike rate (fixed rate on underlying swap)
    pub strike_rate: f64,
    /// Option expiry date
    pub expiry: Date,
    /// Underlying swap start date
    pub swap_start: Date,
    /// Underlying swap end date
    pub swap_end: Date,
    /// Fixed leg payment frequency
    pub fixed_freq: Frequency,
    /// Floating leg payment frequency
    pub float_freq: Frequency,
    /// Day count convention
    pub day_count: DayCount,
    /// Exercise style (European, Bermudan, American)
    pub exercise: SwaptionExercise,
    /// Settlement method (physical or cash)
    pub settlement: SwaptionSettlement,
    /// Volatility model (Black or Normal)
    #[cfg_attr(feature = "serde", serde(default))]
    pub vol_model: VolatilityModel,
    /// Discount curve ID for present value calculations
    pub discount_curve_id: CurveId,
    /// Forward curve ID for floating rate projections
    pub forward_id: CurveId,
    /// Volatility surface ID for option pricing
    pub vol_surface_id: CurveId,
    /// Pricing overrides (manual price, yield, spread)
    pub pricing_overrides: PricingOverrides,
    /// Optional SABR volatility model parameters
    pub sabr_params: Option<SABRParameters>,
    /// Attributes for scenario selection and grouping
    pub attributes: Attributes,
}

impl Swaption {
    /// Create a canonical example swaption for testing and documentation.
    ///
    /// Returns a 1Y x 5Y payer swaption (1 year to expiry, 5 year swap tenor).
    pub fn example() -> Self {
        Self {
            id: InstrumentId::new("SWPN-1Yx5Y-USD"),
            option_type: OptionType::Call,
            notional: Money::new(10_000_000.0, Currency::USD),
            strike_rate: 0.03,
            expiry: Date::from_calendar_date(2025, time::Month::January, 15)
                .expect("Valid example date"),
            swap_start: Date::from_calendar_date(2025, time::Month::January, 17)
                .expect("Valid example date"),
            swap_end: Date::from_calendar_date(2030, time::Month::January, 17)
                .expect("Valid example date"),
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::quarterly(),
            day_count: DayCount::Thirty360,
            exercise: SwaptionExercise::European,
            settlement: SwaptionSettlement::Cash,
            vol_model: VolatilityModel::Black,
            discount_curve_id: CurveId::new("USD-OIS"),
            forward_id: CurveId::new("USD-SOFR-3M"),
            vol_surface_id: CurveId::new("USD-SWPNVOL"),
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
        forward_id: impl Into<CurveId>,
        vol_surface_id: impl Into<CurveId>,
    ) -> Self {
        let mut s = Self {
            id: id.into(),
            option_type: OptionType::Call,
            notional: params.notional,
            strike_rate: params.strike_rate,
            expiry: params.expiry,
            swap_start: params.swap_start,
            swap_end: params.swap_end,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::quarterly(),
            day_count: DayCount::Thirty360,
            exercise: SwaptionExercise::European,
            settlement: SwaptionSettlement::Physical,
            discount_curve_id: discount_curve_id.into(),
            forward_id: forward_id.into(),
            vol_surface_id: vol_surface_id.into(),
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
        s
    }

    /// Create a new receiver swaption using parameter structs.
    pub fn new_receiver(
        id: impl Into<InstrumentId>,
        params: &SwaptionParams,
        discount_curve_id: impl Into<CurveId>,
        forward_id: impl Into<CurveId>,
        vol_surface_id: impl Into<CurveId>,
    ) -> Self {
        let mut s = Self {
            id: id.into(),
            option_type: OptionType::Put,
            notional: params.notional,
            strike_rate: params.strike_rate,
            expiry: params.expiry,
            swap_start: params.swap_start,
            swap_end: params.swap_end,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::quarterly(),
            day_count: DayCount::Thirty360,
            exercise: SwaptionExercise::European,
            settlement: SwaptionSettlement::Physical,
            discount_curve_id: discount_curve_id.into(),
            forward_id: forward_id.into(),
            vol_surface_id: vol_surface_id.into(),
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
        s
    }

    /// Attach SABR parameters to enable SABR-implied volatility pricing.
    pub fn with_sabr(mut self, params: SABRParameters) -> Self {
        self.sabr_params = Some(params);
        self
    }

    // ============================================================================
    // Pricing Methods (moved from engine for direct access)
    // ============================================================================

    /// Compute instrument NPV dispatching to SABR, Black, or Normal as configured.
    pub fn npv(&self, curves: &MarketContext, as_of: Date) -> Result<Money> {
        let disc = curves.get_discount_ref(self.discount_curve_id.as_ref())?;

        // 1. SABR model (if enabled) overrides basic model choice
        if self.sabr_params.is_some() {
            return self.price_sabr(disc, as_of);
        }

        let time_to_expiry = self.year_fraction(as_of, self.expiry, self.day_count)?;
        let vol_surface = curves.surface_ref(self.vol_surface_id.as_str())?;
        let vol = if let Some(impl_vol) = self.pricing_overrides.implied_volatility {
            impl_vol
        } else {
            vol_surface.value_clamped(time_to_expiry, self.strike_rate)
        };

        match self.vol_model {
            VolatilityModel::Black => self.price_black(disc, vol, as_of),
            VolatilityModel::Normal => self.price_normal(disc, vol, as_of),
        }
    }

    /// Black (lognormal) model PV.
    pub fn price_black(
        &self,
        disc: &dyn Discounting,
        volatility: f64,
        as_of: Date,
    ) -> Result<Money> {
        let time_to_expiry = self.year_fraction(as_of, self.expiry, self.day_count)?;
        if time_to_expiry <= 0.0 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }
        let forward_rate = self.forward_swap_rate(disc, as_of)?;
        let annuity = self.annuity(disc, as_of, forward_rate)?;

        // Use stable handling if volatility is near zero
        if volatility <= 0.0 || !volatility.is_finite() {
            // Intrinsic value
            let val = match self.option_type {
                OptionType::Call => (forward_rate - self.strike_rate).max(0.0),
                OptionType::Put => (self.strike_rate - forward_rate).max(0.0),
            };
            return Ok(Money::new(
                val * annuity * self.notional.amount(),
                self.notional.currency(),
            ));
        }

        // Use centralized Black76 helpers for forward-based pricing
        use crate::instruments::common::models::{d1_black76, d2_black76};
        let d1 = d1_black76(forward_rate, self.strike_rate, volatility, time_to_expiry);
        let d2 = d2_black76(forward_rate, self.strike_rate, volatility, time_to_expiry);

        let value = match self.option_type {
            OptionType::Call => {
                annuity
                    * (forward_rate * finstack_core::math::norm_cdf(d1)
                        - self.strike_rate * finstack_core::math::norm_cdf(d2))
            }
            OptionType::Put => {
                annuity
                    * (self.strike_rate * finstack_core::math::norm_cdf(-d2)
                        - forward_rate * finstack_core::math::norm_cdf(-d1))
            }
        };
        Ok(Money::new(
            value * self.notional.amount(),
            self.notional.currency(),
        ))
    }

    /// Bachelier (normal) model PV.
    pub fn price_normal(
        &self,
        disc: &dyn Discounting,
        volatility: f64,
        as_of: Date,
    ) -> Result<Money> {
        let time_to_expiry = self.year_fraction(as_of, self.expiry, self.day_count)?;
        if time_to_expiry <= 0.0 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }

        let forward_rate = self.forward_swap_rate(disc, as_of)?;
        let annuity = self.annuity(disc, as_of, forward_rate)?;

        use crate::instruments::common::models::volatility::normal::bachelier_price;

        let value = bachelier_price(
            self.option_type,
            forward_rate,
            self.strike_rate,
            volatility,
            time_to_expiry,
            annuity,
        );

        Ok(Money::new(
            value * self.notional.amount(),
            self.notional.currency(),
        ))
    }

    /// SABR-implied volatility PV via Black price (default).
    /// Note: If SABR is calibrated to Normal vols, this needs to be adjusted to use price_normal.
    /// Current implementation assumes SABR -> Lognormal Vol.
    pub fn price_sabr(&self, disc: &dyn Discounting, as_of: Date) -> Result<Money> {
        let params: &SABRParameters = self.sabr_params.as_ref().ok_or(Error::Internal)?;
        let model = SABRModel::new(params.clone());
        let time_to_expiry = self.year_fraction(as_of, self.expiry, self.day_count)?;
        if time_to_expiry <= 0.0 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }
        let forward_rate = self.forward_swap_rate(disc, as_of)?;
        // TODO: Check if SABR model supports Normal vol output. Assuming Lognormal for now.
        let sabr_vol = model.implied_volatility(forward_rate, self.strike_rate, time_to_expiry)?;
        self.price_black(disc, sabr_vol, as_of)
    }

    /// Utility: compute year fraction using instrument's day count in a stable way.
    #[inline]
    pub fn year_fraction(&self, start: Date, end: Date, dc: DayCount) -> Result<f64> {
        dc.year_fraction(start, end, finstack_core::dates::DayCountCtx::default())
    }

    /// Calculate annuity based on settlement type.
    /// Physical -> PV01 of swap
    /// Cash -> Cash Annuity (Par Yield)
    pub fn annuity(&self, disc: &dyn Discounting, as_of: Date, forward_rate: f64) -> Result<f64> {
        match self.settlement {
            SwaptionSettlement::Physical => self.swap_annuity(disc, as_of),
            SwaptionSettlement::Cash => self.cash_annuity(forward_rate),
        }
    }

    /// Discounted fixed-leg PV01 (annuity) of the underlying swap schedule (Physical Settlement).
    pub fn swap_annuity(&self, disc: &dyn Discounting, as_of: Date) -> Result<f64> {
        let mut annuity = 0.0;
        let sched = crate::cashflow::builder::build_dates(
            self.swap_start,
            self.swap_end,
            self.fixed_freq,
            StubKind::None,
            BusinessDayConvention::Following,
            None,
        );
        let dates = sched.dates;
        if dates.len() < 2 {
            return Ok(0.0);
        }
        let mut prev = dates[0];
        for &d in &dates[1..] {
            let t = self.year_fraction(as_of, d, self.day_count)?;
            let accrual = self.year_fraction(prev, d, self.day_count)?;
            let df = disc.df(t);
            annuity += accrual * df;
            prev = d;
        }
        Ok(annuity)
    }

    /// Cash settlement annuity (Par Yield approximation).
    /// A = (1 - (1 + S/m)^(-N)) / S
    /// where S = forward rate, m = frequency, N = number of payments
    pub fn cash_annuity(&self, forward_rate: f64) -> Result<f64> {
        let freq_per_year = match self.fixed_freq {
            Frequency::Months(m) if m > 0 => 12.0 / (m as f64),
            Frequency::Days(d) if d > 0 => 365.0 / (d as f64),
            _ => {
                return Err(Error::Validation(
                    "Invalid frequency in cash annuity".into(),
                ))
            }
        };

        if forward_rate.abs() < 1e-8 {
            // L'Hopital's limit for S -> 0: A = N/m (sum of accruals)
            // We need number of periods.
            let tenor = self.year_fraction(self.swap_start, self.swap_end, self.day_count)?;
            let periods = freq_per_year * tenor;
            return Ok(periods / freq_per_year);
        }

        let tenor_years = self.year_fraction(self.swap_start, self.swap_end, self.day_count)?;
        let n_periods = tenor_years * freq_per_year;

        let df_swap = (1.0 + forward_rate / freq_per_year).powf(-n_periods);
        Ok((1.0 - df_swap) / forward_rate)
    }

    /// Forward par swap rate implied by discount factors and annuity.
    pub fn forward_swap_rate(&self, disc: &dyn Discounting, as_of: Date) -> Result<f64> {
        let t_start = self.year_fraction(as_of, self.swap_start, self.day_count)?;
        let t_end = self.year_fraction(as_of, self.swap_end, self.day_count)?;
        let df_start = disc.df(t_start);
        let df_end = disc.df(t_end);
        let annuity = self.swap_annuity(disc, as_of)?;
        if annuity.abs() < 1e-10 {
            return Ok(0.0);
        }
        Ok((df_start - df_end) / annuity)
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
            let model = SABRModel::new(sabr.clone());
            return model.implied_volatility(forward, self.strike_rate, time_to_expiry);
        }

        // 2. Pricing override
        if let Some(impl_vol) = self.pricing_overrides.implied_volatility {
            return Ok(impl_vol);
        }

        // 3. Volatility surface
        let vol_surface = curves.surface_ref(self.vol_surface_id.as_str())?;
        Ok(vol_surface.value_clamped(time_to_expiry, self.strike_rate))
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
    pub fn greek_inputs(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<Option<GreekInputs>> {
        let disc = curves.get_discount_ref(self.discount_curve_id.as_ref())?;
        let t = self.year_fraction(as_of, self.expiry, self.day_count)?;

        if t <= 0.0 {
            return Ok(None);
        }

        let forward = self.forward_swap_rate(disc, as_of)?;
        let annuity = self.annuity(disc, as_of, forward)?;
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
#[derive(Clone, Copy, Debug)]
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

impl crate::instruments::common::traits::Instrument for Swaption {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::Swaption
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn crate::instruments::common::traits::Instrument> {
        Box::new(self.clone())
    }

    fn value(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        self.npv(curves, as_of)
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(curves.clone()),
            as_of,
            base_value,
            metrics,
        )
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for Swaption {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

// Implement CurveDependencies for DV01 calculator
impl crate::instruments::common::traits::CurveDependencies for Swaption {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .forward(self.forward_id.clone())
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
/// use finstack_valuations::instruments::swaption::{
///     BermudanSwaption, BermudanSchedule, BermudanType, SwaptionSettlement,
/// };
///
/// // Create a 10NC2 (10-year swap, callable after 2 years)
/// let swaption = BermudanSwaption::example();
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct BermudanSwaption {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Option type (payer = Call, receiver = Put)
    pub option_type: OptionType,
    /// Notional amount of underlying swap
    pub notional: Money,
    /// Strike rate (fixed rate on underlying swap)
    pub strike_rate: f64,
    /// Underlying swap start date (first accrual start)
    pub swap_start: Date,
    /// Underlying swap end date (final payment)
    pub swap_end: Date,
    /// Fixed leg payment frequency
    pub fixed_freq: Frequency,
    /// Floating leg payment frequency
    pub float_freq: Frequency,
    /// Day count convention for fixed leg
    pub day_count: DayCount,
    /// Settlement method (physical or cash)
    pub settlement: SwaptionSettlement,
    /// Discount curve ID for present value calculations
    pub discount_curve_id: CurveId,
    /// Forward curve ID for floating rate projections
    pub forward_id: CurveId,
    /// Volatility surface ID for calibration
    pub vol_surface_id: CurveId,
    /// Bermudan exercise schedule
    pub bermudan_schedule: BermudanSchedule,
    /// Co-terminal or non-co-terminal exercise
    pub bermudan_type: BermudanType,
    /// Pricing overrides (manual price, yield, spread)
    #[cfg_attr(feature = "serde", serde(default))]
    pub pricing_overrides: PricingOverrides,
    /// Attributes for scenario selection and grouping
    #[cfg_attr(feature = "serde", serde(default))]
    pub attributes: Attributes,
}

impl BermudanSwaption {
    /// Create a canonical example Bermudan swaption for testing.
    ///
    /// Returns a 10NC2 payer swaption (10-year swap, callable quarterly after 2 years).
    pub fn example() -> Self {
        let swap_start = Date::from_calendar_date(2025, time::Month::January, 17)
            .expect("Valid example date");
        let swap_end = Date::from_calendar_date(2035, time::Month::January, 17)
            .expect("Valid example date");
        let first_exercise = Date::from_calendar_date(2027, time::Month::January, 17)
            .expect("Valid example date");

        Self {
            id: InstrumentId::new("BERM-10NC2-USD"),
            option_type: OptionType::Call,
            notional: Money::new(10_000_000.0, Currency::USD),
            strike_rate: 0.03,
            swap_start,
            swap_end,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::quarterly(),
            day_count: DayCount::Thirty360,
            settlement: SwaptionSettlement::Physical,
            discount_curve_id: CurveId::new("USD-OIS"),
            forward_id: CurveId::new("USD-SOFR-3M"),
            vol_surface_id: CurveId::new("USD-SWPNVOL"),
            bermudan_schedule: BermudanSchedule::co_terminal(
                first_exercise,
                swap_end,
                Frequency::semi_annual(),
            ),
            bermudan_type: BermudanType::CoTerminal,
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::new(),
        }
    }

    /// Create a new Bermudan payer swaption (right to pay fixed).
    #[allow(clippy::too_many_arguments)]
    pub fn new_payer(
        id: impl Into<InstrumentId>,
        notional: Money,
        strike_rate: f64,
        swap_start: Date,
        swap_end: Date,
        bermudan_schedule: BermudanSchedule,
        discount_curve_id: impl Into<CurveId>,
        forward_id: impl Into<CurveId>,
        vol_surface_id: impl Into<CurveId>,
    ) -> Self {
        Self {
            id: id.into(),
            option_type: OptionType::Call,
            notional,
            strike_rate,
            swap_start,
            swap_end,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::quarterly(),
            day_count: DayCount::Thirty360,
            settlement: SwaptionSettlement::Physical,
            discount_curve_id: discount_curve_id.into(),
            forward_id: forward_id.into(),
            vol_surface_id: vol_surface_id.into(),
            bermudan_schedule,
            bermudan_type: BermudanType::CoTerminal,
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::default(),
        }
    }

    /// Create a new Bermudan receiver swaption (right to receive fixed).
    #[allow(clippy::too_many_arguments)]
    pub fn new_receiver(
        id: impl Into<InstrumentId>,
        notional: Money,
        strike_rate: f64,
        swap_start: Date,
        swap_end: Date,
        bermudan_schedule: BermudanSchedule,
        discount_curve_id: impl Into<CurveId>,
        forward_id: impl Into<CurveId>,
        vol_surface_id: impl Into<CurveId>,
    ) -> Self {
        Self {
            id: id.into(),
            option_type: OptionType::Put,
            notional,
            strike_rate,
            swap_start,
            swap_end,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::quarterly(),
            day_count: DayCount::Thirty360,
            settlement: SwaptionSettlement::Physical,
            discount_curve_id: discount_curve_id.into(),
            forward_id: forward_id.into(),
            vol_surface_id: vol_surface_id.into(),
            bermudan_schedule,
            bermudan_type: BermudanType::CoTerminal,
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::default(),
        }
    }

    /// Set fixed leg frequency.
    pub fn with_fixed_freq(mut self, freq: Frequency) -> Self {
        self.fixed_freq = freq;
        self
    }

    /// Set floating leg frequency.
    pub fn with_float_freq(mut self, freq: Frequency) -> Self {
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
            Some(first) => self
                .day_count
                .year_fraction(as_of, first, finstack_core::dates::DayCountCtx::default()),
            None => Err(Error::Validation("No exercise dates".into())),
        }
    }

    /// Calculate time to swap maturity in years.
    pub fn time_to_maturity(&self, as_of: Date) -> Result<f64> {
        self.day_count
            .year_fraction(as_of, self.swap_end, finstack_core::dates::DayCountCtx::default())
    }

    /// Get exercise dates as year fractions from valuation date.
    pub fn exercise_times(&self, as_of: Date) -> Result<Vec<f64>> {
        self.bermudan_schedule.exercise_times(as_of, self.day_count)
    }

    /// Build the underlying swap payment schedule.
    ///
    /// Returns (payment_dates, accrual_fractions) for the fixed leg.
    pub fn build_swap_schedule(&self, _as_of: Date) -> Result<(Vec<Date>, Vec<f64>)> {
        let sched = crate::cashflow::builder::build_dates(
            self.swap_start,
            self.swap_end,
            self.fixed_freq,
            StubKind::None,
            BusinessDayConvention::Following,
            None,
        );

        let dates: Vec<Date> = sched.dates.iter().skip(1).copied().collect();
        let ctx = finstack_core::dates::DayCountCtx::default();

        let mut accruals = Vec::with_capacity(dates.len());
        let mut prev = self.swap_start;
        for &d in &dates {
            let tau = self.day_count.year_fraction(prev, d, ctx)?;
            accruals.push(tau);
            prev = d;
        }

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

    /// Forward swap rate at a given exercise date (using discount curve).
    ///
    /// For co-terminal swaptions, the swap always matures at `swap_end`.
    /// For non-co-terminal, each exercise date may have different remaining tenor.
    pub fn forward_swap_rate(
        &self,
        disc: &dyn Discounting,
        as_of: Date,
        exercise_date: Date,
    ) -> Result<f64> {
        let ctx = finstack_core::dates::DayCountCtx::default();

        // Swap starts at exercise date
        let t_start = self.day_count.year_fraction(as_of, exercise_date, ctx)?;
        let t_end = self.day_count.year_fraction(as_of, self.swap_end, ctx)?;

        let df_start = disc.df(t_start);
        let df_end = disc.df(t_end);

        // Calculate annuity for remaining swap
        let annuity = self.remaining_annuity(disc, as_of, exercise_date)?;

        if annuity.abs() < 1e-10 {
            return Ok(0.0);
        }

        Ok((df_start - df_end) / annuity)
    }

    /// Calculate annuity for remaining swap payments after exercise date.
    pub fn remaining_annuity(
        &self,
        disc: &dyn Discounting,
        as_of: Date,
        exercise_date: Date,
    ) -> Result<f64> {
        let (dates, accruals) = self.build_swap_schedule(as_of)?;
        let ctx = finstack_core::dates::DayCountCtx::default();

        let mut annuity = 0.0;
        for (d, tau) in dates.iter().zip(accruals.iter()) {
            if *d > exercise_date {
                let t = self.day_count.year_fraction(as_of, *d, ctx)?;
                annuity += tau * disc.df(t);
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
            strike_rate: self.strike_rate,
            expiry: first_ex,
            swap_start: first_ex,
            swap_end: self.swap_end,
            fixed_freq: self.fixed_freq,
            float_freq: self.float_freq,
            day_count: self.day_count,
            exercise: SwaptionExercise::European,
            settlement: self.settlement,
            vol_model: VolatilityModel::Black,
            discount_curve_id: self.discount_curve_id.clone(),
            forward_id: self.forward_id.clone(),
            vol_surface_id: self.vol_surface_id.clone(),
            pricing_overrides: self.pricing_overrides.clone(),
            sabr_params: None,
            attributes: self.attributes.clone(),
        })
    }
}

impl crate::instruments::common::traits::Instrument for BermudanSwaption {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::BermudanSwaption
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn crate::instruments::common::traits::Instrument> {
        Box::new(self.clone())
    }

    fn value(
        &self,
        _curves: &finstack_core::market_data::MarketContext,
        _as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        // Bermudan swaptions require tree or MC pricing - delegate to pricer
        Err(Error::Validation(
            "BermudanSwaption requires tree or LSMC pricing via BermudanSwaptionPricer".into(),
        ))
    }

    fn price_with_metrics(
        &self,
        _curves: &finstack_core::market_data::MarketContext,
        _as_of: finstack_core::dates::Date,
        _metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        Err(Error::Validation(
            "BermudanSwaption requires tree or LSMC pricing via BermudanSwaptionPricer".into(),
        ))
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for BermudanSwaption {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

impl crate::instruments::common::traits::CurveDependencies for BermudanSwaption {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .forward(self.forward_id.clone())
            .build()
    }
}
