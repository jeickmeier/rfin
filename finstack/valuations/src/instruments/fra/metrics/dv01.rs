//! FRA DV01 metric calculator.
//!
//! Provides an analytic approximation for FRA DV01:
//! DV01 ≈ Notional × tau(start, end) × DF(start) × 1bp
//! Sign convention: receive-fixed → positive; pay-fixed → negative.

use crate::instruments::fra::ForwardRateAgreement;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

/// Analytic DV01 for FRAs.
pub struct FraDv01Calculator;

impl MetricCalculator for FraDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let fra: &ForwardRateAgreement = context.instrument_as()?;

        let disc = context.curves.get_discount_ref(fra.disc_id.as_str())?;
        let base = disc.base_date();

        // Accrual over the FRA period (instrument basis)
        let tau = fra
            .day_count
            .year_fraction(
                fra.start_date,
                fra.end_date,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);
        if tau == 0.0 {
            return Ok(0.0);
        }

        // Discount factor to settlement date (start of accrual)
        let df_start = DiscountCurve::df_on(disc, base, fra.start_date, fra.day_count);
        let dv01 = fra.notional.amount() * tau * df_start * 1e-4;

        Ok(if fra.pay_fixed { -dv01 } else { dv01 })
    }
}
