//! Core swaption pricing engine.
//!
//! Provides deterministic valuation for vanilla swaptions using:
//! - Black (lognormal) model with surface volatilities
//! - SABR-implied volatilities via `SABRModel` when parameters are supplied
//!
//! Heavy numerics are kept here to isolate pricing policy from instrument data shapes.

use crate::instruments::common::models::{SABRModel, SABRParameters};
use crate::instruments::swaption::types::Swaption;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind};
use finstack_core::market_data::traits::Discounting;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::{Error, Result, F};

/// Swaption pricing engine. Stateless wrapper.
pub struct SwaptionPricer;

impl Default for SwaptionPricer {
    fn default() -> Self {
        Self
    }
}

impl SwaptionPricer {
    /// Compute instrument NPV dispatching to SABR or Black as configured on the instrument.
    pub fn npv(&self, s: &Swaption, curves: &MarketContext, as_of: Date) -> Result<Money> {
        let disc = curves
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
            s.disc_id,
        )?;
        if s.sabr_params.is_some() {
            return self.price_sabr(s, disc, as_of);
        }
        let time_to_expiry = self.year_fraction(as_of, s.expiry, s.day_count)?;
        let vol = if let Some(impl_vol) = s.pricing_overrides.implied_volatility {
            impl_vol
        } else {
            let vol_surface = curves.surface_ref(s.vol_id)?;
            vol_surface.value_clamped(time_to_expiry, s.strike_rate)
        };
        self.price_black(s, disc, vol, as_of)
    }

    /// Black (lognormal) model PV using forward swap rate and annuity.
    pub fn price_black(
        &self,
        s: &Swaption,
        disc: &dyn Discounting,
        volatility: F,
        as_of: Date,
    ) -> Result<Money> {
        let time_to_expiry = self.year_fraction(as_of, s.expiry, s.day_count)?;
        if time_to_expiry <= 0.0 {
            return Ok(Money::new(0.0, s.notional.currency()));
        }
        let forward_rate = self.forward_swap_rate(s, disc, as_of)?;
        let annuity = self.swap_annuity(s, disc, as_of)?;
        let variance = volatility.powi(2) * time_to_expiry;

        // Use stable handling if variance is near zero
        if variance <= 0.0 || !variance.is_finite() {
            return Ok(Money::new(0.0, s.notional.currency()));
        }
        let sqrt_var = variance.sqrt();
        let d1 = ((forward_rate / s.strike_rate).ln() + 0.5 * variance) / sqrt_var;
        let d2 = d1 - sqrt_var;
        let value = match s.option_type {
            crate::instruments::common::parameters::OptionType::Call => {
                annuity
                    * (forward_rate * finstack_core::math::norm_cdf(d1)
                        - s.strike_rate * finstack_core::math::norm_cdf(d2))
            }
            crate::instruments::common::parameters::OptionType::Put => {
                annuity
                    * (s.strike_rate * finstack_core::math::norm_cdf(-d2)
                        - forward_rate * finstack_core::math::norm_cdf(-d1))
            }
        };
        Ok(Money::new(
            value * s.notional.amount(),
            s.notional.currency(),
        ))
    }

    /// SABR-implied volatility PV via Black price.
    pub fn price_sabr(&self, s: &Swaption, disc: &dyn Discounting, as_of: Date) -> Result<Money> {
        let params: &SABRParameters = s.sabr_params.as_ref().ok_or(Error::Internal)?;
        let model = SABRModel::new(params.clone());
        let time_to_expiry = self.year_fraction(as_of, s.expiry, s.day_count)?;
        if time_to_expiry <= 0.0 {
            return Ok(Money::new(0.0, s.notional.currency()));
        }
        let forward_rate = self.forward_swap_rate(s, disc, as_of)?;
        let sabr_vol = model.implied_volatility(forward_rate, s.strike_rate, time_to_expiry)?;
        self.price_black(s, disc, sabr_vol, as_of)
    }

    /// Utility: compute year fraction using instrument's day count in a stable way.
    #[inline]
    pub fn year_fraction(&self, start: Date, end: Date, dc: DayCount) -> Result<F> {
        dc.year_fraction(start, end, finstack_core::dates::DayCountCtx::default())
    }

    /// Discounted fixed-leg PV01 (annuity) of the underlying swap schedule.
    pub fn swap_annuity(&self, s: &Swaption, disc: &dyn Discounting, as_of: Date) -> Result<F> {
        let mut annuity = 0.0;
        let sched = crate::cashflow::builder::build_dates(
            s.swap_start,
            s.swap_end,
            s.fixed_freq,
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
            let t = self.year_fraction(as_of, d, s.day_count)?;
            let accrual = self.year_fraction(prev, d, s.day_count)?;
            let df = disc.df(t);
            annuity += accrual * df;
            prev = d;
        }
        Ok(annuity)
    }

    /// Forward par swap rate implied by discount factors and annuity.
    pub fn forward_swap_rate(
        &self,
        s: &Swaption,
        disc: &dyn Discounting,
        as_of: Date,
    ) -> Result<F> {
        let t_start = self.year_fraction(as_of, s.swap_start, s.day_count)?;
        let t_end = self.year_fraction(as_of, s.swap_end, s.day_count)?;
        let df_start = disc.df(t_start);
        let df_end = disc.df(t_end);
        let annuity = self.swap_annuity(s, disc, as_of)?;
        Ok((df_start - df_end) / annuity)
    }
}
