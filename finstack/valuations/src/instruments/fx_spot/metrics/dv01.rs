//! FX Spot DV01 metric calculator.
//!
//! For FX Spot, DV01 is typically negligible as the instrument has minimal interest rate sensitivity
//! (only from settlement lag). This implementation returns a small estimate based on settlement timing.
//!
//! Note: FX Spot is primarily sensitive to spot rate changes (FX01), not interest rates.

use crate::constants::ONE_BASIS_POINT;
use crate::instruments::fx_spot::FxSpot;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::prelude::*;
use finstack_core::Result;

/// DV01 calculator for FX Spot instruments.
///
/// Returns a small DV01 estimate based on settlement lag. For most FX Spot trades,
/// interest rate sensitivity is minimal and FX01 (spot rate sensitivity) is more relevant.
pub struct FxSpotDv01Calculator;

impl MetricCalculator for FxSpotDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let fx_spot: &FxSpot = context.instrument_as()?;
        let as_of = context.as_of;

        // Get settlement date or calculate from lag
        let settlement = fx_spot.settlement.unwrap_or_else(|| {
            let lag_days = fx_spot.settlement_lag_days.unwrap_or(2);
            as_of + time::Duration::days(lag_days.into())
        });

        if as_of >= settlement {
            return Ok(0.0);
        }

        // For FX Spot with short settlement (T+2), interest rate sensitivity is minimal
        // Approximate DV01 using time to settlement
        let time_to_settlement = finstack_core::dates::DayCount::Act360
            .year_fraction(
                as_of,
                settlement,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);

        let notional = fx_spot.notional.unwrap_or(Money::new(1.0, fx_spot.base));
        let dv01 = notional.amount() * time_to_settlement * ONE_BASIS_POINT;

        Ok(dv01)
    }
}
