//! Equity DV01 metric calculator.
//!
//! Provides DV01 calculation for Equity instruments.
//!
//! # Note on Market Convention
//!
//! DV01 is not a standard risk metric for spot equities, as equities are
//! not fixed-income instruments. The primary risk metrics for equities are:
//! - Delta/Price risk (equity price sensitivity)
//! - Beta (market risk)
//! - Volatility/Vega (for option positions)
//!
//! This DV01 implementation provides a rough approximation of the present value
//! sensitivity to changes in discount rates, which may be useful for portfolio-level
//! aggregation but should not be considered a primary risk metric for equities.
//!
//! # Formula
//!
//! DV01 ≈ Position Value × 1bp
//!
//! This approximates the change in present value if the discount rate changes by 1bp,
//! assuming the equity is valued at present value with some discount rate applied.

use crate::constants::ONE_BASIS_POINT;
use crate::instruments::equity::Equity;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// DV01 calculator for Equity instruments (approximation for portfolio use).
pub struct EquityDv01Calculator;

impl MetricCalculator for EquityDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let _equity: &Equity = context.instrument_as()?;

        // For equity, DV01 is a rough approximation: position value × 1bp
        // This represents the approximate sensitivity to a 1bp change in discount rates
        // Note: This is NOT a standard equity risk metric; use for portfolio aggregation only
        let dv01 = context.base_value.amount() * ONE_BASIS_POINT;

        Ok(dv01)
    }
}
