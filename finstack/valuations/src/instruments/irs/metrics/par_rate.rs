//! IRS par rate metric.
//!
//! Computes the fixed rate that sets the swap PV to zero given curves.
//! Uses float-leg PV divided by notional times fixed-leg annuity.

use crate::instruments::irs::InterestRateSwap;
use crate::instruments::irs::types::ParRateMethod;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::Date;
use finstack_core::market_data::term_structures::{
    discount_curve::DiscountCurve, forward_curve::ForwardCurve,
};
use finstack_core::F;

/// Par rate calculator for IRS.
pub struct ParRateCalculator;

impl MetricCalculator for ParRateCalculator {
    fn dependencies(&self) -> &[MetricId] { &[MetricId::Annuity] }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let irs: &InterestRateSwap = context.instrument_as()?;

        let disc = context
            .curves
            .get::<DiscountCurve>(irs.fixed.disc_id)?;
        let base = disc.base_date();

        let method = irs.fixed.par_method.unwrap_or(ParRateMethod::ForwardBased);
        match method {
            ParRateMethod::ForwardBased => {
                // float PV / (N * annuity)
                let fwd = context
                    .curves
                    .get::<ForwardCurve>(irs.float.fwd_id)?;

                let annuity = context.computed.get(&MetricId::Annuity).copied().unwrap_or(0.0);
                if annuity == 0.0 { return Ok(0.0); }

                let fs = crate::cashflow::builder::build_dates(
                    irs.float.start,
                    irs.float.end,
                    irs.float.freq,
                    irs.float.stub,
                    irs.float.bdc,
                    irs.float.calendar_id,
                );
                let schedule: Vec<Date> = fs.dates;
                if schedule.len() < 2 { return Ok(0.0); }

                let mut pv = 0.0;
                let mut prev = schedule[0];
                for &d in &schedule[1..] {
                    let t1 = irs.float.dc.year_fraction(base, prev, finstack_core::dates::DayCountCtx::default()).unwrap_or(0.0);
                    let t2 = irs.float.dc.year_fraction(base, d, finstack_core::dates::DayCountCtx::default()).unwrap_or(0.0);
                    let yf = irs.float.dc.year_fraction(prev, d, finstack_core::dates::DayCountCtx::default()).unwrap_or(0.0);
                    let f = fwd.rate_period(t1, t2);
                    let rate = f + (irs.float.spread_bp * 1e-4);
                    let coupon = irs.notional.amount() * rate * yf;
                    let df = disc.df_on_date_curve(d);
                    pv += coupon * df;
                    prev = d;
                }

                Ok(pv / irs.notional.amount() / annuity)
            }
            ParRateMethod::DiscountRatio => {
                // (P(0,T0) - P(0,Tn)) / Sum alpha_i P(0,Ti)
                let sched = crate::cashflow::builder::build_dates(
                    irs.fixed.start,
                    irs.fixed.end,
                    irs.fixed.freq,
                    irs.fixed.stub,
                    irs.fixed.bdc,
                    irs.fixed.calendar_id,
                );
                let dates: Vec<Date> = sched.dates;
                if dates.len() < 2 { return Ok(0.0); }

                // Numerator: P(0,T0) - P(0,Tn)
                let p0 = disc.df_on_date_curve(dates[0]);
                let pn = disc.df_on_date_curve(*dates.last().unwrap());
                let num = p0 - pn;

                // Denominator: Sum alpha_i P(0,Ti)
                let mut den = 0.0;
                let mut prev = dates[0];
                for &d in &dates[1..] {
                    let alpha = irs
                        .fixed
                        .dc
                        .year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())
                        .unwrap_or(0.0);
                    let p = disc.df_on_date_curve(d);
                    den += alpha * p;
                    prev = d;
                }
                if den == 0.0 { return Ok(0.0); }
                Ok(num / den)
            }
        }
    }
}


