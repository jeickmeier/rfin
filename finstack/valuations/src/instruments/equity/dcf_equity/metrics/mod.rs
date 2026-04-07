//! DCF-specific metrics (enterprise value, equity value, terminal value PV, DV01,
//! price-per-share, and diluted shares).
//!
//! These metrics are registered under the `DCF` instrument type and integrate
//! with the unified DV01 framework used across valuations.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::equity::dcf_equity::DiscountedCashFlow;
use crate::metrics::{MetricCalculator, MetricContext, MetricRegistry};
use finstack_core::Result;

/// Helper: downcast to [`DiscountedCashFlow`] or return a validation error.
fn downcast_dcf(context: &MetricContext) -> Result<&DiscountedCashFlow> {
    context
        .instrument
        .as_any()
        .downcast_ref::<DiscountedCashFlow>()
        .ok_or_else(|| {
            finstack_core::Error::Validation("Expected DiscountedCashFlow instrument".into())
        })
}

/// Calculator for Enterprise Value metric.
///
/// Computes EV from PV components directly (before equity bridge / discounts).
/// Uses market-curve discounting when available, otherwise falls back to WACC.
struct EnterpriseValueCalculator;

impl MetricCalculator for EnterpriseValueCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let dcf = downcast_dcf(context)?;
        // Compute EV directly from discounted PV components (before equity bridge / discounts).
        let terminal_value = dcf.calculate_terminal_value()?;

        if let Ok(discount_curve) = context.curves.get_discount(&dcf.discount_curve_id) {
            let pv_explicit: f64 = dcf
                .flows
                .iter()
                .map(|(date, amount)| {
                    let years = dcf.discount_years(dcf.valuation_date, *date);
                    amount * discount_curve.df(years)
                })
                .sum();

            let pv_terminal = if let Some((terminal_date, _)) = dcf.flows.last() {
                let years = dcf.discount_years(dcf.valuation_date, *terminal_date);
                terminal_value * discount_curve.df(years)
            } else {
                0.0
            };

            Ok(pv_explicit + pv_terminal)
        } else {
            let pv_explicit = dcf.calculate_pv_explicit_flows();
            let pv_terminal = dcf.discount_terminal_value(terminal_value)?;
            Ok(pv_explicit + pv_terminal)
        }
    }
}

/// Calculator for Equity Value metric.
///
/// Consistent with the `value()` method: applies equity bridge and valuation discounts.
struct EquityValueCalculator;

impl MetricCalculator for EquityValueCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let dcf = downcast_dcf(context)?;
        let equity = dcf.value(context.curves.as_ref(), context.as_of)?;
        Ok(equity.amount())
    }
}

/// Calculator for Terminal Value PV metric.
///
/// Uses market-curve discounting when available (same convention as EV metric),
/// otherwise falls back to WACC discounting.
struct TerminalValuePVCalculator;

impl MetricCalculator for TerminalValuePVCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let dcf = downcast_dcf(context)?;
        let terminal_value = dcf.calculate_terminal_value()?;
        if let Ok(discount_curve) = context.curves.get_discount(&dcf.discount_curve_id) {
            if let Some((terminal_date, _)) = dcf.flows.last() {
                let years = dcf.discount_years(dcf.valuation_date, *terminal_date);
                Ok(terminal_value * discount_curve.df(years))
            } else {
                Ok(0.0)
            }
        } else {
            dcf.discount_terminal_value(terminal_value)
        }
    }
}

/// Calculator for Equity Price Per Share metric.
///
/// Returns equity value / diluted shares using the treasury stock method.
/// Returns `NaN` if `shares_outstanding` is not set.
struct EquityPricePerShareCalculator;

impl MetricCalculator for EquityPricePerShareCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let dcf = downcast_dcf(context)?;
        let equity = dcf.value(context.curves.as_ref(), context.as_of)?;
        Ok(dcf
            .equity_value_per_share(equity.amount())
            .unwrap_or(f64::NAN))
    }
}

/// Calculator for diluted share count metric.
///
/// Returns diluted shares via treasury stock method.
/// Returns `NaN` if `shares_outstanding` is not set.
struct EquitySharesCalculator;

impl MetricCalculator for EquitySharesCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let dcf = downcast_dcf(context)?;
        let equity = dcf.value(context.curves.as_ref(), context.as_of)?;
        Ok(dcf.diluted_shares(equity.amount()).unwrap_or(f64::NAN))
    }
}

/// Registers all DCF metrics to a registry.
///
/// Includes:
/// - Parallel DV01 (`MetricId::Dv01`)
/// - Bucketed DV01 (`MetricId::BucketedDv01`)
/// - Theta (`MetricId::Theta`)
/// - Enterprise value (`MetricId::EnterpriseValue`)
/// - Equity value (`MetricId::EquityValue`)
/// - Terminal value PV (`MetricId::TerminalValuePV`)
/// - Equity price per share (`MetricId::EquityPricePerShare`)
/// - Diluted shares (`MetricId::EquityShares`)
pub(crate) fn register_dcf_metrics(registry: &mut MetricRegistry) {
    use crate::pricer::InstrumentType;
    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::DCF,
        metrics: [
            // Generic rate risk metrics via unified DV01 calculator
            (
                Dv01,
                crate::metrics::UnifiedDv01Calculator::<
                    crate::instruments::equity::dcf_equity::DiscountedCashFlow,
                >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())
            ),
            (
                BucketedDv01,
                crate::metrics::UnifiedDv01Calculator::<
                    crate::instruments::equity::dcf_equity::DiscountedCashFlow,
                >::new(crate::metrics::Dv01CalculatorConfig::triangular_key_rate())
            ),
            (EnterpriseValue, EnterpriseValueCalculator),
            (EquityValue, EquityValueCalculator),
            (TerminalValuePV, TerminalValuePVCalculator),
            (EquityPricePerShare, EquityPricePerShareCalculator),
            (EquityShares, EquitySharesCalculator),
        ]
    }
}
