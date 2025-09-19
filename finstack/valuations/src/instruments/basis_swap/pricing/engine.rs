//! Core basis swap pricing engine and shared helpers.

use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;
use finstack_core::F;

/// Common basis swap pricing engine.
pub struct BasisEngine;

/// Parameters for floating leg PV calculation.
#[derive(Debug, Clone)]
pub struct FloatLegParams<'a> {
    /// Period schedule for this leg.
    pub schedule: &'a crate::cashflow::builder::schedule_utils::PeriodSchedule,
    /// Notional amount for the leg.
    pub notional: Money,
    /// Discount curve identifier.
    pub disc_id: &'a str,
    /// Forward curve identifier.
    pub fwd_id: &'a str,
    /// Day count for accrual.
    pub accrual_dc: DayCount,
    /// Spread in decimal (e.g., 0.0005 for 5 bp).
    pub spread: F,
    /// Base date for forward/discount time conversion.
    pub base_date: Date,
}

impl BasisEngine {
    /// Present value of a generic floating leg.
    pub fn pv_float_leg(
        params: FloatLegParams,
        context: &MarketContext,
        valuation_date: Date,
    ) -> Result<Money> {
        // Curves
        let disc = context.get_ref::<
            finstack_core::market_data::term_structures::discount_curve::DiscountCurve,
        >(params.disc_id)?;
        let fwd = context.get_ref::<
            finstack_core::market_data::term_structures::forward_curve::ForwardCurve,
        >(params.fwd_id)?;

        let mut pv = 0.0;
        let currency = params.notional.currency();
        let dc_ctx = DayCountCtx::default();

        // Iterate periods
        for i in 1..params.schedule.dates.len() {
            let period_start = params.schedule.dates[i - 1];
            let period_end = params.schedule.dates[i];

            // Skip past periods
            if period_end <= valuation_date {
                continue;
            }

            // Forward rate for the accrual period using Act/360 time from base date
            let t_start = DayCount::Act360.year_fraction(params.base_date, period_start, dc_ctx)?;
            let t_end = DayCount::Act360.year_fraction(params.base_date, period_end, dc_ctx)?;
            let forward_rate = fwd.rate_period(t_start, t_end);

            // Total rate (add spread)
            let total_rate = forward_rate + params.spread;

            // Accrual year fraction
            let year_frac = params
                .accrual_dc
                .year_fraction(period_start, period_end, dc_ctx)?;

            // Payment
            let payment = params.notional.amount() * total_rate * year_frac;

            // Discount factor to payment date (t_end consistent with fwd time base)
            let df = disc.df(t_end);
            pv += payment * df;
        }

        Ok(Money::new(pv, currency))
    }

    /// Discounted accrual sum for a leg (no notional multiplier)
    pub fn annuity_for_leg(
        schedule: &crate::cashflow::builder::schedule_utils::PeriodSchedule,
        accrual_dc: DayCount,
        disc_curve_id: &str,
        curves: &MarketContext,
    ) -> Result<F> {
        let disc = curves.get_ref::<
            finstack_core::market_data::term_structures::discount_curve::DiscountCurve,
        >(disc_curve_id)?;
        let base = disc.base_date();

        let mut annuity = 0.0;
        let mut prev = schedule.dates[0];
        for &d in &schedule.dates[1..] {
            let yf = accrual_dc.year_fraction(prev, d, DayCountCtx::default())?;
            let t_pay = accrual_dc.year_fraction(base, d, DayCountCtx::default())?;
            let df = disc.df(t_pay);
            annuity += yf * df;
            prev = d;
        }
        Ok(annuity)
    }
}


