//! Swaption (option on interest rate swap) implementation with SABR volatility.

use crate::instruments::options::models::{SABRModel, SABRParameters};
use crate::instruments::options::OptionType;
use finstack_core::math::norm_cdf;
use crate::instruments::traits::Attributes;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::traits::Discount;
use finstack_core::money::Money;
use finstack_core::{Error, Result, F};

/// Swaption settlement type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwaptionSettlement { Physical, Cash }

/// Swaption exercise style
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwaptionExercise { European, Bermudan, American }

/// Swaption instrument
#[derive(Clone, Debug)]
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
    pub implied_vol: Option<F>,
    pub sabr_params: Option<SABRParameters>,
    pub attributes: Attributes,
}

impl Swaption {
    #[allow(clippy::too_many_arguments)]
    pub fn new_payer(
        id: impl Into<String>, notional: Money, strike_rate: F, expiry: Date, swap_start: Date, swap_end: Date,
        disc_id: &'static str, forward_id: &'static str, vol_id: &'static str,
    ) -> Self {
        Self {
            id: id.into(), option_type: OptionType::Call, notional, strike_rate, expiry, swap_start, swap_end,
            fixed_freq: Frequency::semi_annual(), float_freq: Frequency::quarterly(), day_count: DayCount::Thirty360,
            exercise: SwaptionExercise::European, settlement: SwaptionSettlement::Physical,
            disc_id, forward_id, vol_id, implied_vol: None, sabr_params: None, attributes: Attributes::default(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_receiver(
        id: impl Into<String>, notional: Money, strike_rate: F, expiry: Date, swap_start: Date, swap_end: Date,
        disc_id: &'static str, forward_id: &'static str, vol_id: &'static str,
    ) -> Self {
        Self {
            id: id.into(), option_type: OptionType::Put, notional, strike_rate, expiry, swap_start, swap_end,
            fixed_freq: Frequency::semi_annual(), float_freq: Frequency::quarterly(), day_count: DayCount::Thirty360,
            exercise: SwaptionExercise::European, settlement: SwaptionSettlement::Physical,
            disc_id, forward_id, vol_id, implied_vol: None, sabr_params: None, attributes: Attributes::default(),
        }
    }

    pub fn with_sabr(mut self, params: SABRParameters) -> Self { self.sabr_params = Some(params); self }

    pub(crate) fn swap_annuity(&self, disc: &dyn Discount) -> Result<F> {
        let base_date = disc.base_date();
        let mut annuity = 0.0;
        let sched = crate::cashflow::builder::build_dates(self.swap_start, self.swap_end, self.fixed_freq, StubKind::None, BusinessDayConvention::Following, None);
        let dates = sched.dates;
        if dates.len() < 2 { return Ok(0.0); }
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

    pub(crate) fn forward_swap_rate(&self, disc: &dyn Discount) -> Result<F> {
        let base_date = disc.base_date();
        let t_start = self.year_fraction(base_date, self.swap_start, self.day_count)?;
        let t_end = self.year_fraction(base_date, self.swap_end, self.day_count)?;
        let df_start = disc.df(t_start);
        let df_end = disc.df(t_end);
        let annuity = self.swap_annuity(disc)?;
        Ok((df_start - df_end) / annuity)
    }

    pub fn black_price(&self, disc: &dyn Discount, volatility: F) -> Result<Money> {
        let base_date = disc.base_date();
        let time_to_expiry = self.year_fraction(base_date, self.expiry, self.day_count)?;
        if time_to_expiry <= 0.0 { return Ok(Money::new(0.0, self.notional.currency())); }
        let forward_rate = self.forward_swap_rate(disc)?;
        let annuity = self.swap_annuity(disc)?;
        let variance = volatility.powi(2) * time_to_expiry;
        let d1 = ((forward_rate / self.strike_rate).ln() + 0.5 * variance) / variance.sqrt();
        let d2 = d1 - variance.sqrt();
        let value = match self.option_type {
            OptionType::Call => annuity * (forward_rate * norm_cdf(d1) - self.strike_rate * norm_cdf(d2)),
            OptionType::Put => annuity * (self.strike_rate * norm_cdf(-d2) - forward_rate * norm_cdf(-d1)),
        };
        Ok(Money::new(value * self.notional.amount(), self.notional.currency()))
    }

    pub fn sabr_price(&self, disc: &dyn Discount) -> Result<Money> {
        let sabr_params = self.sabr_params.as_ref().ok_or(Error::Internal)?;
        let model = SABRModel::new(sabr_params.clone());
        let base_date = disc.base_date();
        let time_to_expiry = self.year_fraction(base_date, self.expiry, self.day_count)?;
        if time_to_expiry <= 0.0 { return Ok(Money::new(0.0, self.notional.currency())); }
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
        let disc = curves.disc(s.disc_id)?;
        if s.sabr_params.is_some() {
            s.sabr_price(disc.as_ref())
        } else {
            let time_to_expiry = s.year_fraction(disc.base_date(), s.expiry, s.day_count)?;
            let vol = if let Some(impl_vol) = s.implied_vol { impl_vol } else {
                let vol_surface = curves.surface(s.vol_id)?; vol_surface.value_clamped(time_to_expiry, s.strike_rate)
            };
            s.black_price(disc.as_ref(), vol)
        }
    },
);


