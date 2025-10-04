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
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::traits::Discounting;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::{Error, Result};

use super::parameters::SwaptionParams;

/// Swaption settlement type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwaptionSettlement {
    Physical,
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
pub enum SwaptionExercise {
    European,
    Bermudan,
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
pub struct Swaption {
    pub id: InstrumentId,
    pub option_type: OptionType,
    pub notional: Money,
    pub strike_rate: f64,
    pub expiry: Date,
    pub swap_start: Date,
    pub swap_end: Date,
    pub fixed_freq: Frequency,
    pub float_freq: Frequency,
    pub day_count: DayCount,
    pub exercise: SwaptionExercise,
    pub settlement: SwaptionSettlement,
    pub disc_id: CurveId,
    pub forward_id: CurveId,
    pub vol_id: &'static str,
    pub pricing_overrides: PricingOverrides,
    pub sabr_params: Option<SABRParameters>,
    pub attributes: Attributes,
}

impl Swaption {
    /// Create a new payer swaption using parameter structs.
    pub fn new_payer(
        id: impl Into<InstrumentId>,
        params: &SwaptionParams,
        disc_id: impl Into<CurveId>,
        forward_id: impl Into<CurveId>,
        vol_id: &'static str,
    ) -> Self {
        Self {
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
            disc_id: disc_id.into(),
            forward_id: forward_id.into(),
            vol_id,
            pricing_overrides: PricingOverrides::default(),
            sabr_params: None,
            attributes: Attributes::default(),
        }
    }

    /// Create a new receiver swaption using parameter structs.
    pub fn new_receiver(
        id: impl Into<InstrumentId>,
        params: &SwaptionParams,
        disc_id: impl Into<CurveId>,
        forward_id: impl Into<CurveId>,
        vol_id: &'static str,
    ) -> Self {
        Self {
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
            disc_id: disc_id.into(),
            forward_id: forward_id.into(),
            vol_id,
            pricing_overrides: PricingOverrides::default(),
            sabr_params: None,
            attributes: Attributes::default(),
        }
    }

    /// Attach SABR parameters to enable SABR-implied volatility pricing.
    pub fn with_sabr(mut self, params: SABRParameters) -> Self {
        self.sabr_params = Some(params);
        self
    }

    // ============================================================================
    // Pricing Methods (moved from engine for direct access)
    // ============================================================================

    /// Compute instrument NPV dispatching to SABR or Black as configured on the instrument.
    pub fn npv(&self, curves: &MarketContext, as_of: Date) -> Result<Money> {
        let disc = curves.get_discount_ref(self.disc_id.as_ref())?;
        if self.sabr_params.is_some() {
            return self.price_sabr(disc, as_of);
        }
        let time_to_expiry = self.year_fraction(as_of, self.expiry, self.day_count)?;
        let vol = if let Some(impl_vol) = self.pricing_overrides.implied_volatility {
            impl_vol
        } else {
            let vol_surface = curves.surface_ref(self.vol_id)?;
            vol_surface.value_clamped(time_to_expiry, self.strike_rate)
        };
        self.price_black(disc, vol, as_of)
    }

    /// Black (lognormal) model PV using forward swap rate and annuity.
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
        let annuity = self.swap_annuity(disc, as_of)?;

        // Use stable handling if volatility is near zero
        if volatility <= 0.0 || !volatility.is_finite() {
            return Ok(Money::new(0.0, self.notional.currency()));
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

    /// SABR-implied volatility PV via Black price.
    pub fn price_sabr(&self, disc: &dyn Discounting, as_of: Date) -> Result<Money> {
        let params: &SABRParameters = self.sabr_params.as_ref().ok_or(Error::Internal)?;
        let model = SABRModel::new(params.clone());
        let time_to_expiry = self.year_fraction(as_of, self.expiry, self.day_count)?;
        if time_to_expiry <= 0.0 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }
        let forward_rate = self.forward_swap_rate(disc, as_of)?;
        let sabr_vol = model.implied_volatility(forward_rate, self.strike_rate, time_to_expiry)?;
        self.price_black(disc, sabr_vol, as_of)
    }

    /// Utility: compute year fraction using instrument's day count in a stable way.
    #[inline]
    pub fn year_fraction(&self, start: Date, end: Date, dc: DayCount) -> Result<f64> {
        dc.year_fraction(start, end, finstack_core::dates::DayCountCtx::default())
    }

    /// Discounted fixed-leg PV01 (annuity) of the underlying swap schedule.
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

    /// Forward par swap rate implied by discount factors and annuity.
    pub fn forward_swap_rate(&self, disc: &dyn Discounting, as_of: Date) -> Result<f64> {
        let t_start = self.year_fraction(as_of, self.swap_start, self.day_count)?;
        let t_end = self.year_fraction(as_of, self.swap_end, self.day_count)?;
        let df_start = disc.df(t_start);
        let df_end = disc.df(t_end);
        let annuity = self.swap_annuity(disc, as_of)?;
        Ok((df_start - df_end) / annuity)
    }
}

impl_instrument!(
    Swaption,
    crate::pricer::InstrumentType::Swaption,
    "Swaption",
    pv = |s, curves, as_of| {
        // Call the instrument's own npv method
        s.npv(curves, as_of)
    },
);

impl crate::instruments::common::HasDiscountCurve for Swaption {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.disc_id
    }
}
