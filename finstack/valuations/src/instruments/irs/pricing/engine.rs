//! Core IRS pricing engine and helpers.
//!
//! Provides deterministic present value calculation for a vanilla
//! fixed-for-floating interest rate swap. The engine uses the instrument
//! day-counts for accrual and the discount curve's own date helpers for
//! discounting to ensure policy visibility and currency safety.
//!
//! PV = sign × (PV_fixed − PV_float) with sign determined by `PayReceive`.

use crate::instruments::irs::types::{InterestRateSwap, PayReceive};
use finstack_core::dates::calendar::calendar_by_id;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;
use finstack_core::{dates::DayCountCtx, dates::ScheduleBuilder};

/// Common IRS pricing engine providing core calculation methods.
pub struct IrsEngine;

impl IrsEngine {
    /// Calculates the present value of an IRS by composing leg PVs.
    pub fn pv(irs: &InterestRateSwap, context: &MarketContext) -> Result<Money> {
        let disc = context.get_discount_ref(irs.fixed.disc_id.as_ref())?;
        // Attempt to resolve a forward curve for the float leg. If absent and the float leg
        // references the same discounting curve (OIS case), fall back to an efficient
        // discount-only computation for the floating leg.
        let pv_fixed = irs.pv_fixed_leg(disc)?;
        let pv_float = match context.get_forward_ref(irs.float.fwd_id.as_ref()) {
            Ok(fwd) => irs.pv_float_leg(disc, fwd)?,
            Err(_) => {
                // OIS fallback: forward curve not found. If the float leg uses the same
                // discount curve as the fixed leg, we can value the floating leg using
                // discount factors only: PV_float = N × (P(0, T_start) - P(0, T_end))
                // plus the spread annuity when spread is non-zero.
                if irs.float.disc_id == irs.fixed.disc_id {
                    // Base PV without spread
                    let df_start = DiscountCurve::df_on_date_curve(disc, irs.float.start);
                    let df_end = DiscountCurve::df_on_date_curve(disc, irs.float.end);
                    let mut pv = irs.notional.amount() * (df_start - df_end);

                    // Add spread contribution if any: N × sum_i( spread × alpha_i × DF(T_i) )
                    if irs.float.spread_bp != 0.0 {
                        // Build coupon schedule using the float leg payment frequency and conventions
                        let builder = ScheduleBuilder::new(irs.float.start, irs.float.end)
                            .frequency(irs.float.freq)
                            .stub_rule(irs.float.stub);
                        let sched_dates: Vec<_> = if let Some(id) = irs.float.calendar_id {
                            if let Some(cal) = calendar_by_id(id) {
                                builder
                                    .adjust_with(irs.float.bdc, cal)
                                    .build()
                                    .unwrap()
                                    .into_iter()
                                    .collect()
                            } else {
                                builder.build().unwrap().into_iter().collect()
                            }
                        } else {
                            builder.build().unwrap().into_iter().collect()
                        };

                        if sched_dates.len() >= 2 {
                            let mut prev = sched_dates[0];
                            let mut annuity = 0.0;
                            for &d in &sched_dates[1..] {
                                let alpha = irs
                                    .float
                                    .dc
                                    .year_fraction(prev, d, DayCountCtx::default())
                                    .unwrap_or(0.0);
                                let df = DiscountCurve::df_on_date_curve(disc, d);
                                annuity += alpha * df;
                                prev = d;
                            }
                            pv += irs.notional.amount() * (irs.float.spread_bp * 1e-4) * annuity;
                        }
                    }
                    Money::new(pv, irs.notional.currency())
                } else {
                    // Not OIS and forward curve missing: return the error to guide callers
                    // to provide a forward curve.
                    return Err(context
                        .get_forward_ref(irs.float.fwd_id.as_ref())
                        .err()
                        .unwrap());
                }
            }
        };

        let npv = match irs.side {
            PayReceive::PayFixed => (pv_float - pv_fixed)?,
            PayReceive::ReceiveFixed => (pv_fixed - pv_float)?,
        };
        Ok(npv)
    }
}
