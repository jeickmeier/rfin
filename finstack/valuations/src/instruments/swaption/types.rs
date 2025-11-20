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
            annuity
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
            _ => return Err(Error::Validation("Invalid frequency in cash annuity".into())),
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
