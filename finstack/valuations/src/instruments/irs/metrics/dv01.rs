//! IRS DV01 metric.
//!
//! Approximates the dollar value of a one basis point shift using the
//! annuity of the fixed leg and the instrument side to determine sign.

// Use the re-exported types from the parent module
use crate::instruments::irs::PayReceive;
use crate::instruments::InterestRateSwap;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};

/// DV01 calculator for IRS.
pub struct Dv01Calculator;

impl MetricCalculator for Dv01Calculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Annuity]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let irs: &InterestRateSwap = context.instrument_as()?;
        let annuity = context
            .computed
            .get(&MetricId::Annuity)
            .copied()
            .unwrap_or(0.0);
        let dv01_mag = annuity * irs.notional.amount() * 1e-4;
        let dv01 = match irs.side {
            PayReceive::ReceiveFixed => dv01_mag,
            PayReceive::PayFixed => -dv01_mag,
        };
        Ok(dv01)
    }
}
