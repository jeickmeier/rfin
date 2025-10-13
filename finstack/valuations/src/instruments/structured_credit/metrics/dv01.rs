//! Structured Credit DV01 metric calculator.
//!
//! Provides DV01 calculation for structured credit instruments using modified duration:
//! DV01 ≈ Price × Modified Duration × 1bp
//! Sign convention: positive for long positions (bond prices fall when rates rise).

use crate::constants::ONE_BASIS_POINT;
use crate::instruments::structured_credit::StructuredCredit;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

/// DV01 calculator for structured credit instruments.
pub struct StructuredCreditDv01Calculator;

impl MetricCalculator for StructuredCreditDv01Calculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::DurationMod, MetricId::DirtyPrice]
    }

    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let _structured_credit: &StructuredCredit = context.instrument_as()?;
        let _as_of = context.as_of;

        // For structured credit, we need to check if we're past legal maturity
        // This is a simplified check - in practice, we'd need more sophisticated logic
        // for different tranche maturities and payment schedules
        
        // Get modified duration and dirty price from computed metrics
        let modified_duration = context
            .computed
            .get(&MetricId::DurationMod)
            .copied()
            .unwrap_or(0.0);
        
        let dirty_price = context
            .computed
            .get(&MetricId::DirtyPrice)
            .copied()
            .unwrap_or(100.0); // Default to par if not available

        // DV01 = Price × Modified Duration × 1bp
        let dv01 = dirty_price * modified_duration * ONE_BASIS_POINT;
        
        Ok(dv01)
    }
}
