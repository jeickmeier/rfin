//! IR Future DV01 metric calculator.
//!
//! Approximates DV01 as: FaceValue × tau(period) × 1bp.
//! This aligns with exchange-traded rate futures where a 1bp change in the
//! implied rate translates linearly to P&L scaled by the contract face and
//! underlying accrual length.

use crate::instruments::ir_future::{InterestRateFuture, Position};
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

/// DV01 calculator for interest rate futures.
pub struct IrFutureDv01Calculator;

impl MetricCalculator for IrFutureDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let fut: &InterestRateFuture = context.instrument_as()?;

        let tau = fut
            .day_count
            .year_fraction(
                fut.period_start,
                fut.period_end,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);
        if tau == 0.0 {
            return Ok(0.0);
        }

        // Scale by contracts and apply position sign
        let contracts_scale = if fut.contract_specs.face_value != 0.0 {
            fut.notional.amount() / fut.contract_specs.face_value
        } else {
            1.0
        };
        let sign = match fut.position {
            Position::Long => 1.0,
            Position::Short => -1.0,
        };
        let dv01_per_contract = fut.contract_specs.face_value * tau * 1e-4;
        Ok(sign * contracts_scale * dv01_per_contract)
    }
}


