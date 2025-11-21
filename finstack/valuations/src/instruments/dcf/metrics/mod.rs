//! DCF-specific metrics (enterprise value, equity value, terminal value PV, and DV01).
//!
//! These metrics are registered under the `DCF` instrument type and integrate
//! with the unified DV01 framework used across valuations.

use crate::instruments::dcf::DiscountedCashFlow;
use crate::metrics::{MetricCalculator, MetricContext, MetricRegistry};
use finstack_core::Result;

/// Calculator for Enterprise Value metric.
pub struct EnterpriseValueCalculator;

impl MetricCalculator for EnterpriseValueCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let dcf = context
            .instrument
            .as_any()
            .downcast_ref::<DiscountedCashFlow>()
            .ok_or_else(|| {
                finstack_core::Error::Validation("Expected DiscountedCashFlow instrument".into())
            })?;

        let pv_explicit = dcf.calculate_pv_explicit_flows();
        let terminal_value = dcf.calculate_terminal_value();
        let pv_terminal = dcf.discount_terminal_value(terminal_value);

        Ok(pv_explicit + pv_terminal)
    }
}

/// Calculator for Equity Value metric.
pub struct EquityValueCalculator;

impl MetricCalculator for EquityValueCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let dcf = context
            .instrument
            .as_any()
            .downcast_ref::<DiscountedCashFlow>()
            .ok_or_else(|| {
                finstack_core::Error::Validation("Expected DiscountedCashFlow instrument".into())
            })?;

        // Calculate EV inline to avoid borrowing issues
        let pv_explicit = dcf.calculate_pv_explicit_flows();
        let terminal_value = dcf.calculate_terminal_value();
        let pv_terminal = dcf.discount_terminal_value(terminal_value);
        let enterprise_value = pv_explicit + pv_terminal;

        Ok(enterprise_value - dcf.net_debt)
    }
}

/// Calculator for Terminal Value PV metric.
pub struct TerminalValuePVCalculator;

impl MetricCalculator for TerminalValuePVCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let dcf = context
            .instrument
            .as_any()
            .downcast_ref::<DiscountedCashFlow>()
            .ok_or_else(|| {
                finstack_core::Error::Validation("Expected DiscountedCashFlow instrument".into())
            })?;

        let terminal_value = dcf.calculate_terminal_value();
        Ok(dcf.discount_terminal_value(terminal_value))
    }
}

/// Registers all DCF metrics to a registry.
///
/// Includes:
/// - Parallel DV01 (`MetricId::Dv01`)
/// - Bucketed DV01 (`MetricId::BucketedDv01`)
/// - Enterprise value (`MetricId::EnterpriseValue`)
/// - Equity value (`MetricId::EquityValue`)
/// - Terminal value PV (`MetricId::TerminalValuePV`)
pub fn register_dcf_metrics(registry: &mut MetricRegistry) {
    crate::register_metrics! {
        registry: registry,
        instrument: "DCF",
        metrics: [
            // Generic rate risk metrics via unified DV01 calculator
            (
                Dv01,
                crate::metrics::UnifiedDv01Calculator::<
                    crate::instruments::dcf::DiscountedCashFlow,
                >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())
            ),
            (
                BucketedDv01,
                crate::metrics::UnifiedDv01Calculator::<
                    crate::instruments::dcf::DiscountedCashFlow,
                >::new(crate::metrics::Dv01CalculatorConfig::key_rate())
            ),
            // Generic theta (rolls valuation date by configured period)
            (
                Theta,
                crate::metrics::GenericTheta::<crate::instruments::dcf::DiscountedCashFlow>::default()
            ),
            (EnterpriseValue, EnterpriseValueCalculator),
            (EquityValue, EquityValueCalculator),
            (TerminalValuePV, TerminalValuePVCalculator),
        ]
    }
}


