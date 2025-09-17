//! Swaption (option on interest rate swap) implementation with SABR volatility.

use crate::instruments::common::{MarketRefs, PricingOverrides};
use crate::instruments::options::models::{SABRModel, SABRParameters};
use crate::instruments::options::OptionType;
use crate::instruments::traits::Attributes;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::traits::Discounting;
use finstack_core::math::norm_cdf;
use finstack_core::money::Money;
use finstack_core::{Error, Result, F};

use super::parameters::SwaptionParams;

/// Swaption settlement type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwaptionSettlement {
    Physical,
    Cash,
}

/// Swaption exercise style
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwaptionExercise {
    European,
    Bermudan,
    American,
}

/// Swaption instrument
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
pub struct Swaption {
    pub id: String,
    pub option_type: OptionType,
    pub notional: Money,
    pub strike_rate: F,
    pub expiry: Date,
    pub swap_start: Date,
    pub swap_end: Date,
    pub fixed_freq: Frequency,
    pub float_freq: Frequency,
    pub day_count: DayCount,
    pub exercise: SwaptionExercise,
    pub settlement: SwaptionSettlement,
    pub disc_id: &'static str,
    pub forward_id: &'static str,
    pub vol_id: &'static str,
    pub pricing_overrides: PricingOverrides,
    pub sabr_params: Option<SABRParameters>,
    pub attributes: Attributes,
}

impl Swaption {
    /// Create a new payer swaption using parameter structs
    pub fn new_payer(
        id: impl Into<String>,
        params: &SwaptionParams,
        market_refs: &MarketRefs,
    ) -> Self {
        let forward_id = market_refs
            .fwd_id
            .as_ref()
            .expect("Forward curve required for swaptions");
        let vol_id = market_refs
            .vol_id
            .as_ref()
            .expect("Volatility surface required for swaptions");

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
            disc_id: Box::leak(market_refs.disc_id.to_string().into_boxed_str()),
            forward_id: Box::leak(forward_id.to_string().into_boxed_str()),
            vol_id: Box::leak(vol_id.to_string().into_boxed_str()),
            pricing_overrides: PricingOverrides::default(),
            sabr_params: None,
            attributes: Attributes::default(),
        }
    }

    /// Create a new receiver swaption using parameter structs
    pub fn new_receiver(
        id: impl Into<String>,
        params: &SwaptionParams,
        market_refs: &MarketRefs,
    ) -> Self {
        let forward_id = market_refs
            .fwd_id
            .as_ref()
            .expect("Forward curve required for swaptions");
        let vol_id = market_refs
            .vol_id
            .as_ref()
            .expect("Volatility surface required for swaptions");

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
            disc_id: Box::leak(market_refs.disc_id.to_string().into_boxed_str()),
            forward_id: Box::leak(forward_id.to_string().into_boxed_str()),
            vol_id: Box::leak(vol_id.to_string().into_boxed_str()),
            pricing_overrides: PricingOverrides::default(),
            sabr_params: None,
            attributes: Attributes::default(),
        }
    }

    pub fn with_sabr(mut self, params: SABRParameters) -> Self {
        self.sabr_params = Some(params);
        self
    }

    pub(crate) fn swap_annuity(&self, disc: &dyn Discounting) -> Result<F> {
        let base_date = disc.base_date();
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
            let t = self.year_fraction(base_date, d, self.day_count)?;
            let accrual = self.year_fraction(prev, d, self.day_count)?;
            let df = disc.df(t);
            annuity += accrual * df;
            prev = d;
        }
        Ok(annuity)
    }

    pub(crate) fn forward_swap_rate(&self, disc: &dyn Discounting) -> Result<F> {
        let base_date = disc.base_date();
        let t_start = self.year_fraction(base_date, self.swap_start, self.day_count)?;
        let t_end = self.year_fraction(base_date, self.swap_end, self.day_count)?;
        let df_start = disc.df(t_start);
        let df_end = disc.df(t_end);
        let annuity = self.swap_annuity(disc)?;
        Ok((df_start - df_end) / annuity)
    }

    pub fn black_price(&self, disc: &dyn Discounting, volatility: F) -> Result<Money> {
        let base_date = disc.base_date();
        let time_to_expiry = self.year_fraction(base_date, self.expiry, self.day_count)?;
        if time_to_expiry <= 0.0 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }
        let forward_rate = self.forward_swap_rate(disc)?;
        let annuity = self.swap_annuity(disc)?;
        let variance = volatility.powi(2) * time_to_expiry;
        let d1 = ((forward_rate / self.strike_rate).ln() + 0.5 * variance) / variance.sqrt();
        let d2 = d1 - variance.sqrt();
        let value = match self.option_type {
            OptionType::Call => {
                annuity * (forward_rate * norm_cdf(d1) - self.strike_rate * norm_cdf(d2))
            }
            OptionType::Put => {
                annuity * (self.strike_rate * norm_cdf(-d2) - forward_rate * norm_cdf(-d1))
            }
        };
        Ok(Money::new(
            value * self.notional.amount(),
            self.notional.currency(),
        ))
    }

    pub fn sabr_price(&self, disc: &dyn Discounting) -> Result<Money> {
        let sabr_params = self.sabr_params.as_ref().ok_or(Error::Internal)?;
        let model = SABRModel::new(sabr_params.clone());
        let base_date = disc.base_date();
        let time_to_expiry = self.year_fraction(base_date, self.expiry, self.day_count)?;
        if time_to_expiry <= 0.0 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }
        let forward_rate = self.forward_swap_rate(disc)?;
        let sabr_vol = model.implied_volatility(forward_rate, self.strike_rate, time_to_expiry)?;
        self.black_price(disc, sabr_vol)
    }

    pub(crate) fn year_fraction(&self, start: Date, end: Date, dc: DayCount) -> Result<F> {
        dc.year_fraction(start, end, finstack_core::dates::DayCountCtx::default())
    }
}

impl_instrument!(
    Swaption,
    "Swaption",
    pv = |s, curves, _as_of| {
        let disc = curves
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                s.disc_id,
            )?;
        if s.sabr_params.is_some() {
            s.sabr_price(disc)
        } else {
            let time_to_expiry = s.year_fraction(disc.base_date(), s.expiry, s.day_count)?;
            let vol = if let Some(impl_vol) = s.pricing_overrides.implied_volatility {
                impl_vol
            } else {
                let vol_surface = curves.surface_ref(s.vol_id)?;
                vol_surface.value_clamped(time_to_expiry, s.strike_rate)
            };
            s.black_price(disc, vol)
        }
    },
);
