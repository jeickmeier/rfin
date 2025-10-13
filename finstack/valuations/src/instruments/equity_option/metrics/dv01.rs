//! Equity Option DV01 metric calculator.
//!
//! Provides DV01 calculation for Equity Option instruments:
//! DV01 ≈ Notional × Time to Expiry × 1bp

use crate::constants::ONE_BASIS_POINT;
use crate::instruments::equity_option::EquityOption;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// DV01 calculator for Equity Option instruments.
pub struct EquityOptionDv01Calculator;

impl MetricCalculator for EquityOptionDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &EquityOption = context.instrument_as()?;
        let as_of = context.as_of;

        if as_of >= option.expiry {
            return Ok(0.0);
        }

        // Simple DV01 approximation: Notional × Time to Expiry × 1bp
        let time_to_expiry = option
            .day_count
            .year_fraction(
                as_of,
                option.expiry,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);
        
        // Equity options use contract_size and strike price to determine notional exposure
        let notional_exposure = option.strike.amount() * option.contract_size;
        let dv01 = notional_exposure * time_to_expiry * ONE_BASIS_POINT;
        
        Ok(dv01)
    }
}
