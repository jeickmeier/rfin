//! Interest rate future metric calculators.
//!
//! Placeholder module to align with the `mod/metrics` layout used by other
//! fixed income instruments. Specific future-related metrics can be added later.

use crate::instruments::fixed_income::ir_future::InterestRateFuture;
use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::prelude::*;
use finstack_core::F;
use std::sync::Arc;

/// PV calculator for IR Future returning base value from context
pub struct IrFuturePvCalculator;

impl MetricCalculator for IrFuturePvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        Ok(context.base_value.amount())
    }
}

/// DV01 calculator for IR Future using face value × accrual × 1bp
pub struct IrFutureDv01Calculator;

impl MetricCalculator for IrFutureDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let fut: &InterestRateFuture = context.instrument_as()?;

        let tau = DiscountCurve::year_fraction(fut.period_start, fut.period_end, fut.day_count);
        if tau <= 0.0 {
            return Ok(0.0);
        }

        let dv01 = fut.contract_specs.face_value * tau * 1e-4;
        Ok(dv01)
    }
}

/// Registers interest rate future metrics.
///
/// Currently no specific metrics are defined; this function exists to
/// maintain a consistent registration surface across instruments.
pub fn register_ir_future_metrics(registry: &mut MetricRegistry) {
    registry
        .register_metric(MetricId::custom("pv"), Arc::new(IrFuturePvCalculator), &["InterestRateFuture"]) 
        .register_metric(MetricId::Dv01, Arc::new(IrFutureDv01Calculator), &["InterestRateFuture"]);
}


