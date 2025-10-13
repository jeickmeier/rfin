//! Equity DV01 metric calculator.
//!
//! Provides DV01 calculation for Equity instruments:
//! DV01 ≈ Position Value × 1bp
//! Sign convention: positive for long positions.

use crate::constants::ONE_BASIS_POINT;
use crate::instruments::equity::Equity;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// DV01 calculator for Equity instruments.
pub struct EquityDv01Calculator;

impl MetricCalculator for EquityDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let _equity: &Equity = context.instrument_as()?;
        
        // For equity, DV01 is simply the position value × 1bp
        // This represents the sensitivity to a 1bp change in the discount rate
        let dv01 = context.base_value.amount() * ONE_BASIS_POINT;
        
        Ok(dv01)
    }
}
