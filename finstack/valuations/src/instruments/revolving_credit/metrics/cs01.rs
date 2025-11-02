//! CS01 (credit spread sensitivity) metric for revolving credit facilities.

use crate::instruments::RevolvingCredit;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::DayCountCtx;

/// Calculator for CS01 (dollar value of a 1bp parallel shift in credit spread).
///
/// For revolving credit, this is equivalent to DV01 on the discount curve
/// since the discount curve incorporates credit risk.
///
/// Uses numerical differentiation: CS01 = (PV_down - PV_up) / 2 where
/// PV_up is computed with a +1bp spread bump and PV_down with -1bp.
#[derive(Debug, Default, Clone, Copy)]
pub struct Cs01Calculator;

impl MetricCalculator for Cs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let facility: &RevolvingCredit = context.instrument_as()?;
        let as_of = context.as_of;

        // Get base curves
        let disc = context.curves.get_discount_ref(facility.disc_id.as_str())?;
        let disc_dc = disc.day_count();

        // Generate cashflows
        let schedule =
            crate::instruments::revolving_credit::cashflows::generate_deterministic_cashflows(
                facility, as_of,
            )?;

        // Compute PV with spread bumps
        let bump_bp = 0.0001; // 1bp
        let mut npv_up = 0.0;
        let mut npv_down = 0.0;

        for cf in &schedule.flows {
            if cf.date <= as_of {
                continue;
            }

            let yf = disc_dc.year_fraction(disc.base_date(), cf.date, DayCountCtx::default())?;
            let df_base = disc.df(yf);

            // Apply spread bumps
            let df_up = df_base * (-bump_bp * yf).exp();
            let df_down = df_base * (bump_bp * yf).exp();

            npv_up += cf.amount.amount() * df_up;
            npv_down += cf.amount.amount() * df_down;
        }

        // CS01 magnitude per 1bp: use symmetric difference and return positive magnitude
        let cs01 = ((npv_down - npv_up) / 2.0).abs();

        Ok(cs01)
    }
}
