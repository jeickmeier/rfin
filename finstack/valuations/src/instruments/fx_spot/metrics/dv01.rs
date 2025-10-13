//! FX Spot DV01 metric calculator.
//!
//! Provides DV01 calculation for FX Spot instruments:
//! DV01 ≈ Notional × Time to Settlement × 1bp
//! Sign convention: positive for long positions.

use crate::constants::ONE_BASIS_POINT;
use crate::instruments::fx_spot::FxSpot;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::prelude::*;
use finstack_core::Result;

/// DV01 calculator for FX Spot instruments.
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

        // Simple DV01 approximation: Notional × Time to Settlement × 1bp
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
