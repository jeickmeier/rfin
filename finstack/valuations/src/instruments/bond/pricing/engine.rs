use finstack_core::dates::Date;
use finstack_core::dates::DateExt;
use finstack_core::explain::{ExplainOpts, ExplanationTrace, TraceEntry};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

use crate::cashflow::traits::CashflowProvider;
// Discountable trait not required after switching to curve day-count path

use super::super::types::Bond;

/// Bond pricing engine providing core valuation methods.
pub struct BondEngine;

impl BondEngine {
    /// Price a bond using discount curve present value calculation.
    pub fn price(bond: &Bond, context: &MarketContext, as_of: Date) -> Result<Money> {
        Self::price_with_explanation(bond, context, as_of, ExplainOpts::disabled())
            .map(|(pv, _)| pv)
    }

    /// Price a bond with optional explanation trace.
    ///
    /// Returns the present value and an optional trace containing
    /// cashflow-level PV breakdown when explanation is enabled.
    pub fn price_with_explanation(
        bond: &Bond,
        context: &MarketContext,
        as_of: Date,
        explain: ExplainOpts,
    ) -> Result<(Money, Option<ExplanationTrace>)> {
        let flows = bond.build_schedule(context, as_of)?;
        let disc = context.get_discount(bond.discount_curve_id.as_str())?;
        // Discount using the curve's own day-count convention for time mapping.
        // Transform (date, amount) -> (df_on_date_curve(date) * amount) and sum.
        if flows.is_empty() {
            return Err(finstack_core::error::InputError::TooFewPoints.into());
        }
        let ccy = flows[0].1.currency();
        let mut total = Money::new(0.0, ccy);

        // Initialize explanation trace if requested
        let mut trace = if explain.enabled {
            Some(ExplanationTrace::new("pricing"))
        } else {
            None
        };

        // Settlement PV: start discounting from settlement date if provided
        let settle_date = if let Some(sd_u32) = bond.settlement_days {
            let sd: i32 = sd_u32 as i32;
            if let Some(id) = &bond.calendar_id {
                if let Some(cal) = finstack_core::dates::calendar::calendar_by_id(id) {
                    // Walk business days using the provided calendar
                    let mut d = as_of;
                    let mut remaining = sd;
                    let step = if remaining >= 0 { 1 } else { -1 };
                    while remaining != 0 {
                        d = d.saturating_add(time::Duration::days(step as i64));
                        if cal.is_business_day(d) {
                            remaining -= step;
                        }
                    }
                    finstack_core::dates::adjust(d, bond.bdc, cal)?
                } else {
                    as_of.add_weekdays(sd)
                }
            } else {
                as_of.add_weekdays(sd)
            }
        } else {
            as_of
        };
        // Pre-compute settle_date discount factor for correct theta
        let disc_dc = disc.day_count();
        let t_settle = disc_dc
            .year_fraction(
                disc.base_date(),
                settle_date,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let df_settle = disc.df(t_settle);

        for (d, amt) in &flows {
            if *d <= settle_date {
                continue;
            }
            // Discount from settle_date (which is derived from as_of)
            let t_d = disc_dc
                .year_fraction(
                    disc.base_date(),
                    *d,
                    finstack_core::dates::DayCountCtx::default(),
                )
                .unwrap_or(0.0);
            let df_d_abs = disc.df(t_d);
            let df = if df_settle != 0.0 {
                df_d_abs / df_settle
            } else {
                1.0
            };
            let pv_cf = *amt * df;
            total = (total + pv_cf)?;

            // Add trace entry if explanation is enabled
            if let Some(ref mut t) = trace {
                t.push(
                    TraceEntry::CashflowPV {
                        date: d.to_string(),
                        cashflow_amount: amt.amount(),
                        cashflow_currency: amt.currency().to_string(),
                        discount_factor: df,
                        pv_amount: pv_cf.amount(),
                        pv_currency: pv_cf.currency().to_string(),
                        curve_id: bond.discount_curve_id.to_string(),
                    },
                    explain.max_entries,
                );
            }
        }
        Ok((total, trace))
    }
}
