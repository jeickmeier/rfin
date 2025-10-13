//! Deposit DV01 metric calculator.
//!
//! Provides DV01 calculation for deposit instruments:
//! DV01 ≈ Notional × tau(start, end) × DF(end) × 1bp
//! Sign convention: positive for deposits (receiving fixed rate).

use crate::constants::ONE_BASIS_POINT;
use crate::instruments::deposit::Deposit;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::Result;

/// DV01 calculator for deposits.
pub struct DepositDv01Calculator;

impl MetricCalculator for DepositDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let deposit: &Deposit = context.instrument_as()?;
        let as_of = context.as_of;

        if as_of >= deposit.end {
            return Ok(0.0);
        }

        let disc = context.curves.get_discount_ref(deposit.disc_id.as_str())?;
        let base = disc.base_date();

        // Accrual period
        let tau = deposit
            .day_count
            .year_fraction(
                deposit.start,
                deposit.end,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);
        if tau == 0.0 {
            return Ok(0.0);
        }

        // Discount factor to end date
        let df_end = DiscountCurve::df_on(disc, base, deposit.end, deposit.day_count);
        
        // DV01 = Notional × tau × DF(end) × 1bp
        let dv01 = deposit.notional.amount() * tau * df_end * ONE_BASIS_POINT;
        
        Ok(dv01)
    }
}
