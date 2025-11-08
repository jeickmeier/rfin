//! DV01 metric for term loans.

use crate::instruments::TermLoan;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::DayCountCtx;

#[derive(Debug, Default, Clone, Copy)]
pub struct Dv01Calculator;

impl MetricCalculator for Dv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let loan: &TermLoan = context.instrument_as()?;
        let as_of = context.as_of;

        // Discount curve
        let disc = context
            .curves
            .get_discount_ref(loan.discount_curve_id.as_str())?;
        let disc_dc = disc.day_count();

        // Generate cashflows
        let schedule = crate::instruments::term_loan::cashflows::generate_cashflows(
            loan,
            &context.curves,
            as_of,
        )?;

        // PV with +/- 1bp discount bumps
        let bump_bp = 0.0001;
        let mut pv_up = 0.0;
        let mut pv_down = 0.0;
        for cf in &schedule.flows {
            if cf.date <= as_of {
                continue;
            }
            let t = disc_dc.year_fraction(disc.base_date(), cf.date, DayCountCtx::default())?;
            let df_base = disc.df(t);
            let df_up = df_base * (-bump_bp * t).exp();
            let df_down = df_base * (bump_bp * t).exp();
            pv_up += cf.amount.amount() * df_up;
            pv_down += cf.amount.amount() * df_down;
        }
        Ok((pv_up - pv_down) / 2.0)
    }
}
