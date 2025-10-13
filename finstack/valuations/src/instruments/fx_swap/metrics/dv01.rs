//! FX Swap DV01 metric calculator.
//!
//! Provides DV01 calculation for FX swap instruments:
//! DV01 ≈ Notional × tau(near, far) × DF(far) × 1bp
//! Sign convention: positive for paying fixed (receiving floating).

use crate::constants::ONE_BASIS_POINT;
use crate::instruments::fx_swap::FxSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::Result;

/// DV01 calculator for FX swaps.
pub struct FxSwapDv01Calculator;

impl MetricCalculator for FxSwapDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let fx_swap: &FxSwap = context.instrument_as()?;
        let as_of = context.as_of;

        if as_of >= fx_swap.far_date {
            return Ok(0.0);
        }

        let disc = context
            .curves
            .get_discount_ref(fx_swap.domestic_disc_id.as_str())?;
        let base = disc.base_date();

        // Use domestic curve day count convention
        let day_count = disc.day_count();

        // Accrual period from near to far date
        let tau = day_count
            .year_fraction(
                fx_swap.near_date,
                fx_swap.far_date,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);
        if tau == 0.0 {
            return Ok(0.0);
        }

        // Discount factor to far date
        let df_far = DiscountCurve::df_on(disc, base, fx_swap.far_date, day_count);

        // DV01 = Notional × tau × DF(far) × 1bp
        let dv01 = fx_swap.base_notional.amount() * tau * df_far * ONE_BASIS_POINT;

        Ok(dv01)
    }
}
