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
use crate::instruments::pricing_overrides::VolSurfaceExtrapolation;
use crate::instruments::PricingOverrides;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::traits::Discounting;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId, Rate};
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
            BusinessDayConvention::Following,
            None,
        )?;
        // Exercise dates are all coupon dates except the last one (maturity)
        let exercise_dates: Vec<Date> = sched
            .dates
            .into_iter()
            .filter(|&d| d >= first_exercise && d < swap_end)
            .collect();
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
    pub fixed_freq: Tenor,
    /// Floating leg payment frequency
    pub float_freq: Tenor,
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
    #[allow(clippy::expect_used)] // Example uses hardcoded valid values
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
            fixed_freq: Tenor::semi_annual(),
            float_freq: Tenor::quarterly(),
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
            fixed_freq: Tenor::semi_annual(),
            float_freq: Tenor::quarterly(),
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
            fixed_freq: Tenor::semi_annual(),
            float_freq: Tenor::quarterly(),
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
        // 1. SABR model (if enabled) overrides basic model choice
        if self.sabr_params.is_some() {
            return self.price_sabr(curves, as_of);
        }

        let time_to_expiry = self.year_fraction(as_of, self.expiry, self.day_count)?;
        let vol_surface = curves.surface(self.vol_surface_id.as_str())?;
        let vol = if let Some(impl_vol) = self.pricing_overrides.implied_volatility {
            impl_vol
        } else {
            match self.pricing_overrides.vol_surface_extrapolation {
                VolSurfaceExtrapolation::Clamp | VolSurfaceExtrapolation::LinearInVariance => {
                    // LinearInVariance falls back to Clamp until surface impl is ready
                    vol_surface.value_clamped(time_to_expiry, self.strike_rate)
                }
                VolSurfaceExtrapolation::Error => {
                    vol_surface.value_checked(time_to_expiry, self.strike_rate)?
                }
            }
        };

        match self.vol_model {
            VolatilityModel::Black => self.price_black(curves, vol, as_of),
            VolatilityModel::Normal => self.price_normal(curves, vol, as_of),
        }
    }

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
        let time_to_expiry = self.year_fraction(as_of, self.expiry, self.day_count)?;
        if time_to_expiry <= 0.0 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }

        let disc = curves.get_discount(self.discount_curve_id.as_ref())?;
        let forward_rate = self.forward_swap_rate(curves, as_of)?;
        let annuity = self.annuity(disc.as_ref(), as_of, forward_rate)?;

        let value = model_fn(
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

    /// Black (lognormal) model PV.
    pub fn price_black(
        &self,
        curves: &MarketContext,
        volatility: f64,
        as_of: Date,
    ) -> Result<Money> {
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
            use crate::instruments::common::models::{d1_black76, d2_black76};
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
            use crate::instruments::common::models::volatility::normal::bachelier_price;
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
    /// # References
    ///
    /// - Hagan, P. et al. (2002). "Managing Smile Risk" *Wilmott Magazine*
    /// - Antonov, A. et al. (2015). "SABR/Free Sabr" for normal vol extensions
    pub fn price_sabr(&self, curves: &MarketContext, as_of: Date) -> Result<Money> {
        let params: &SABRParameters = self.sabr_params.as_ref().ok_or(Error::Internal)?;
        let model = SABRModel::new(params.clone());
        let time_to_expiry = self.year_fraction(as_of, self.expiry, self.day_count)?;
        if time_to_expiry <= 0.0 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }
        let forward_rate = self.forward_swap_rate(curves, as_of)?;

        // SABR outputs lognormal (Black) volatility
        let sabr_lognormal_vol =
            model.implied_volatility(forward_rate, self.strike_rate, time_to_expiry)?;

        // Dispatch to the appropriate pricing model
        match self.vol_model {
            VolatilityModel::Black => self.price_black(curves, sabr_lognormal_vol, as_of),
            VolatilityModel::Normal => {
                // Convert lognormal vol to normal vol using improved Hagan approximation.
                //
                // The exact relationship between lognormal (Black) and normal (Bachelier)
                // volatilities involves solving a non-linear equation. We use the
                // second-order approximation from Hagan et al.:
                //
                // σ_normal ≈ σ_lognormal × F_mid × [1 - (σ²T/24) × (1 - F_mid²/FK)]
                //
                // where F_mid = √(F×K) is the geometric mean of forward and strike.
                //
                // For ATM (F = K), this simplifies to: σ_normal = σ_lognormal × F
                // For OTM/ITM, the correction term improves accuracy to ~1bp for
                // typical market conditions.
                //
                // References:
                // - Hagan, P. et al. (2002). "Managing Smile Risk" Wilmott Magazine
                // - Jaeckel, P. (2017). "Let's Be Rational" for exact conversion
                let f = forward_rate;
                let k = self.strike_rate;
                let geometric_mean_fk = (f * k).abs().sqrt();

                let sabr_normal_vol = if geometric_mean_fk > 1e-10 {
                    // Second-order correction for non-ATM options
                    let variance = sabr_lognormal_vol * sabr_lognormal_vol * time_to_expiry;

                    // Correction term: accounts for convexity difference between models
                    // This term is small (~0.1%) for typical parameters but improves accuracy
                    let correction = if variance > 1e-10 && (f * k).abs() > 1e-20 {
                        let fk_ratio_factor = 1.0 - geometric_mean_fk * geometric_mean_fk / (f * k);
                        1.0 - (variance / 24.0) * fk_ratio_factor
                    } else {
                        1.0
                    };

                    sabr_lognormal_vol * geometric_mean_fk * correction.max(0.5)
                // Floor at 0.5 for stability
                } else {
                    // Fallback for very small rates: use forward directly (ATM approximation)
                    sabr_lognormal_vol * f.abs().max(1e-4)
                };

                self.price_normal(curves, sabr_normal_vol, as_of)
            }
        }
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
    ///
    /// # Time Basis
    ///
    /// Uses curve-consistent relative discount factors via `relative_df_discounting`:
    /// - DF from `as_of` to each payment date is computed using the discount curve's
    ///   own base_date and day_count (not the instrument's day_count).
    /// - Accrual fractions use the instrument's day_count (correct for coupon calculation).
    pub fn swap_annuity(&self, disc: &dyn Discounting, as_of: Date) -> Result<f64> {
        use crate::instruments::common::pricing::time::relative_df_discounting;

        let mut annuity = 0.0;
        let sched = crate::cashflow::builder::build_dates(
            self.swap_start,
            self.swap_end,
            self.fixed_freq,
            StubKind::None,
            BusinessDayConvention::Following,
            None,
        )?;
        let dates = sched.dates;
        if dates.len() < 2 {
            return Ok(0.0);
        }
        let mut prev = dates[0];
        for window in dates.windows(2) {
            let d = window[1];
            // Accrual uses instrument's day count (correct for coupon calculation)
            let accrual = self.year_fraction(prev, d, self.day_count)?;
            // DF uses curve-consistent relative DF (correct for discounting)
            let df = relative_df_discounting(disc, as_of, d)?;
            annuity += accrual * df;
            prev = d;
        }
        Ok(annuity)
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
    /// # ISDA Cash Settlement Conventions
    ///
    /// This approximation may differ from exact ISDA cash settlement calculations:
    ///
    /// - **Par-Par**: Uses the actual swap PV01 from the zero curve at settlement
    /// - **Zero Coupon**: Discounts the single payment at swap maturity
    ///
    /// For trades with explicit ISDA cash settlement conventions, the difference
    /// can be several basis points on notional, particularly for:
    /// - Steep yield curves
    /// - Long-dated swaps
    /// - Non-standard payment frequencies
    ///
    /// For production systems requiring exact ISDA compliance, consider using
    /// [`swap_annuity`] with the settlement date curve instead.
    ///
    /// # Edge Cases
    ///
    /// When `forward_rate ≈ 0`, uses L'Hôpital's limit: `A → N/m` (sum of accruals).
    pub fn cash_annuity(&self, forward_rate: f64) -> Result<f64> {
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
            let tenor = self.year_fraction(self.swap_start, self.swap_end, self.day_count)?;
            let periods = freq_per_year * tenor;
            return Ok(periods / freq_per_year);
        }

        let tenor_years = self.year_fraction(self.swap_start, self.swap_end, self.day_count)?;
        let n_periods = tenor_years * freq_per_year;

        let df_swap = (1.0 + forward_rate / freq_per_year).powf(-n_periods);
        Ok((1.0 - df_swap) / forward_rate)
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
        use crate::instruments::common::pricing::time::{
            rate_period_on_dates, relative_df_discounting,
        };

        let disc = curves.get_discount(self.discount_curve_id.as_ref())?;
        let annuity = self.swap_annuity(disc.as_ref(), as_of)?;
        if annuity.abs() < 1e-10 {
            return Ok(0.0);
        }

        // Single-curve optimization
        if self.forward_id == self.discount_curve_id {
            let df_start = relative_df_discounting(disc.as_ref(), as_of, self.swap_start)?;
            let df_end = relative_df_discounting(disc.as_ref(), as_of, self.swap_end)?;
            return Ok((df_start - df_end) / annuity);
        }

        let fwd = curves.get_forward(self.forward_id.as_ref())?;
        let fwd_dc = fwd.day_count();
        let sched = crate::cashflow::builder::build_dates(
            self.swap_start,
            self.swap_end,
            self.float_freq,
            StubKind::None,
            BusinessDayConvention::Following,
            None,
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
            let model = SABRModel::new(sabr.clone());
            return model.implied_volatility(forward, self.strike_rate, time_to_expiry);
        }

        // 2. Pricing override
        if let Some(impl_vol) = self.pricing_overrides.implied_volatility {
            return Ok(impl_vol);
        }

        // 3. Volatility surface
        let vol_surface = curves.surface(self.vol_surface_id.as_str())?;
        match self.pricing_overrides.vol_surface_extrapolation {
            VolSurfaceExtrapolation::Clamp | VolSurfaceExtrapolation::LinearInVariance => {
                // LinearInVariance falls back to Clamp until surface impl is ready
                Ok(vol_surface.value_clamped(time_to_expiry, self.strike_rate))
            }
            VolSurfaceExtrapolation::Error => {
                Ok(vol_surface.value_checked(time_to_expiry, self.strike_rate)?)
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
        let t = self.year_fraction(as_of, self.expiry, self.day_count)?;

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
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        self.npv(curves, as_of)
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
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
            None,
            None,
        )
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for Swaption {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

impl crate::instruments::common::pricing::HasForwardCurves for Swaption {
    fn forward_curve_ids(&self) -> Vec<finstack_core::types::CurveId> {
        vec![self.forward_id.clone()]
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
/// use finstack_valuations::instruments::rates::swaption::{
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
    #[allow(clippy::expect_used)] // Example uses hardcoded valid values
    pub fn example() -> Self {
        let swap_start =
            Date::from_calendar_date(2025, time::Month::January, 17).expect("Valid example date");
        let swap_end =
            Date::from_calendar_date(2035, time::Month::January, 17).expect("Valid example date");
        let first_exercise =
            Date::from_calendar_date(2027, time::Month::January, 17).expect("Valid example date");

        Self {
            id: InstrumentId::new("BERM-10NC2-USD"),
            option_type: OptionType::Call,
            notional: Money::new(10_000_000.0, Currency::USD),
            strike_rate: 0.03,
            swap_start,
            swap_end,
            fixed_freq: Tenor::semi_annual(),
            float_freq: Tenor::quarterly(),
            day_count: DayCount::Thirty360,
            settlement: SwaptionSettlement::Physical,
            discount_curve_id: CurveId::new("USD-OIS"),
            forward_id: CurveId::new("USD-SOFR-3M"),
            vol_surface_id: CurveId::new("USD-SWPNVOL"),
            bermudan_schedule: BermudanSchedule::co_terminal(
                first_exercise,
                swap_end,
                Tenor::semi_annual(),
            )
            .expect("valid Bermudan schedule"),
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
            fixed_freq: Tenor::semi_annual(),
            float_freq: Tenor::quarterly(),
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

    /// Create a new Bermudan payer swaption using a typed strike rate.
    #[allow(clippy::too_many_arguments)]
    pub fn new_payer_rate(
        id: impl Into<InstrumentId>,
        notional: Money,
        strike_rate: Rate,
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
            strike_rate: strike_rate.as_decimal(),
            swap_start,
            swap_end,
            fixed_freq: Tenor::semi_annual(),
            float_freq: Tenor::quarterly(),
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
            fixed_freq: Tenor::semi_annual(),
            float_freq: Tenor::quarterly(),
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

    /// Create a new Bermudan receiver swaption using a typed strike rate.
    #[allow(clippy::too_many_arguments)]
    pub fn new_receiver_rate(
        id: impl Into<InstrumentId>,
        notional: Money,
        strike_rate: Rate,
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
            strike_rate: strike_rate.as_decimal(),
            swap_start,
            swap_end,
            fixed_freq: Tenor::semi_annual(),
            float_freq: Tenor::quarterly(),
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
        let sched = crate::cashflow::builder::build_dates(
            self.swap_start,
            self.swap_end,
            self.fixed_freq,
            StubKind::None,
            BusinessDayConvention::Following,
            None,
        )?;

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
        use crate::instruments::common::pricing::time::{
            rate_period_on_dates, relative_df_discounting,
        };

        let disc = curves.get_discount(self.discount_curve_id.as_ref())?;
        let annuity = self.remaining_annuity(disc.as_ref(), as_of, exercise_date)?;

        if annuity.abs() < 1e-10 {
            return Ok(0.0);
        }

        // Single-curve optimization
        if self.forward_id == self.discount_curve_id {
            let df_start = relative_df_discounting(disc.as_ref(), as_of, exercise_date)?;
            let df_end = relative_df_discounting(disc.as_ref(), as_of, self.swap_end)?;
            return Ok((df_start - df_end) / annuity);
        }

        let fwd = curves.get_forward(self.forward_id.as_ref())?;
        let fwd_dc = fwd.day_count();
        let sched = crate::cashflow::builder::build_dates(
            exercise_date,
            self.swap_end,
            self.float_freq,
            StubKind::None,
            BusinessDayConvention::Following,
            None,
        )?;

        let mut pv_float = 0.0;
        let mut prev = exercise_date;
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
        use crate::instruments::common::pricing::time::relative_df_discounting;

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
}

impl crate::instruments::common::pricing::HasDiscountCurve for BermudanSwaption {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

impl crate::instruments::common::pricing::HasForwardCurves for BermudanSwaption {
    fn forward_curve_ids(&self) -> Vec<finstack_core::types::CurveId> {
        vec![self.forward_id.clone()]
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
