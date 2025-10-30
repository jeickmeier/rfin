//! IRS floating leg PV metric.
//!
//! Discounts floating coupons projected from a forward curve, including
//! any quoted spread in basis points.
//! Only includes future cashflows (payment date > as_of date).

use crate::instruments::InterestRateSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::Date;

/// PV of the floating leg of an IRS.
pub struct FloatLegPvCalculator;

impl MetricCalculator for FloatLegPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let irs: &InterestRateSwap = context.instrument_as()?;
        let as_of = context.as_of;

        let disc = context.curves.get_discount(&irs.float.disc_id)?;
        let base = disc.base_date();
        let disc_dc = disc.day_count();
        
        // For OIS swaps (same discount curve for both legs), use discount-only pricing
        // to be consistent with the main IRS pricer
        if irs.float.disc_id == irs.fixed.disc_id {
            // OIS swap: use discount-only method
            let t_as_of = disc_dc
                .year_fraction(base, as_of, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);
            let df_as_of = disc.df(t_as_of);

            let t_start = disc_dc
                .year_fraction(base, irs.float.start, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);
            let t_end = disc_dc
                .year_fraction(base, irs.float.end, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);

            let df_start_abs = disc.df(t_start);
            let df_end_abs = disc.df(t_end);
            let df_start = if df_as_of != 0.0 {
                df_start_abs / df_as_of
            } else {
                1.0
            };
            let df_end = if df_as_of != 0.0 {
                df_end_abs / df_as_of
            } else {
                1.0
            };
            
            let mut pv = irs.notional.amount() * (df_start - df_end);
            
            // Add spread contribution if any
            if irs.float.spread_bp != 0.0 {
                let sched = crate::cashflow::builder::build_dates(
                    irs.float.start,
                    irs.float.end,
                    irs.float.freq,
                    irs.float.stub,
                    irs.float.bdc,
                    irs.float.calendar_id.as_deref(),
                );
                let dates: Vec<Date> = sched.dates;
                
                if dates.len() >= 2 {
                    let mut prev = dates[0];
                    let mut spread_pv = 0.0;
                    for &d in &dates[1..] {
                        if d <= as_of {
                            prev = d;
                            continue;
                        }
                        
                        let alpha = irs.float.dc
                            .year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())
                            .unwrap_or(0.0);
                        let t_d = disc_dc
                            .year_fraction(base, d, finstack_core::dates::DayCountCtx::default())
                            .unwrap_or(0.0);
                        let df_d_abs = disc.df(t_d);
                        let df = if df_as_of != 0.0 {
                            df_d_abs / df_as_of
                        } else {
                            1.0
                        };
                        spread_pv += alpha * df;
                        prev = d;
                    }
                    pv += irs.notional.amount() * (irs.float.spread_bp * 1e-4) * spread_pv;
                }
            }
            
            return Ok(pv);
        }
        
        // Non-OIS swap: use forward curve for pricing
        let fwd = context.curves.get_forward(&irs.float.fwd_id)?;

        let sched = crate::cashflow::builder::build_dates(
            irs.float.start,
            irs.float.end,
            irs.float.freq,
            irs.float.stub,
            irs.float.bdc,
            irs.float.calendar_id.as_deref(),
        );
        let dates: Vec<Date> = sched.dates;
        if dates.len() < 2 {
            return Ok(0.0);
        }

        // Pre-compute as_of discount factor for correct discounting
        let t_as_of = disc_dc
            .year_fraction(base, as_of, finstack_core::dates::DayCountCtx::default())
            .unwrap_or(0.0);
        let df_as_of = disc.df(t_as_of);

        let mut pv = 0.0;
        let mut prev = dates[0];
        for &d in &dates[1..] {
            // Only include future cashflows
            if d <= as_of {
                prev = d;
                continue;
            }

            let t1 = irs
                .float
                .dc
                .year_fraction(base, prev, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);
            let t2 = irs
                .float
                .dc
                .year_fraction(base, d, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);
            let yf = irs
                .float
                .dc
                .year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);

            // Only call rate_period if t1 < t2 to avoid date ordering errors
            let f = if t2 > t1 {
                fwd.rate_period(t1, t2)
            } else {
                0.0
            };
            let rate = f + (irs.float.spread_bp * 1e-4);
            let coupon = irs.notional.amount() * rate * yf;

            // Discount from as_of for correct theta and seasoned swap handling
            let t_d = disc_dc
                .year_fraction(base, d, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);
            let df_d_abs = disc.df(t_d);
            let df = if df_as_of != 0.0 {
                df_d_abs / df_as_of
            } else {
                1.0
            };

            pv += coupon * df;
            prev = d;
        }
        Ok(pv)
    }
}
