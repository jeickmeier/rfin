//! Utilities for instrument pricing and metrics assembly.
//!
//! Contains helpers shared across instrument implementations, notably the
//! function to assemble a `ValuationResult` with computed metrics.

use crate::metrics::{standard_registry, MetricContext};
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use indexmap::IndexMap;
use std::sync::Arc;

/// Shared helper to build a ValuationResult with a set of metrics.
///
/// Centralizes the repeated pattern across instruments to compute base value,
/// build metric context, compute metrics and stamp a result.
///
/// This function uses trait objects to avoid generic monomorphization across
/// compilation units, which can cause coverage metadata mismatches.
pub fn build_with_metrics_dyn(
    instrument: &dyn crate::instruments::traits::InstrumentLike,
    curves: &MarketContext,
    as_of: Date,
    base_value: Money,
    metrics: &[crate::metrics::MetricId],
) -> finstack_core::Result<crate::results::ValuationResult> {
    use crate::instruments::*;

    // Create an owned clone for the Arc to avoid lifetime issues
    // This approach reduces generic monomorphization across compilation units
    let instrument_clone: Box<dyn crate::instruments::traits::InstrumentLike> = {
        // Fixed Income instruments
        if let Some(bond) = instrument.as_any().downcast_ref::<Bond>() {
            Box::new(bond.clone())
        } else if let Some(loan) = instrument.as_any().downcast_ref::<Loan>() {
            Box::new(loan.clone())
        } else if let Some(irs) = instrument.as_any().downcast_ref::<InterestRateSwap>() {
            Box::new(irs.clone())
        } else if let Some(cds) = instrument.as_any().downcast_ref::<CreditDefaultSwap>() {
            Box::new(cds.clone())
        } else if let Some(convertible) = instrument.as_any().downcast_ref::<ConvertibleBond>() {
            Box::new(convertible.clone())
        } else if let Some(deposit) = instrument.as_any().downcast_ref::<Deposit>() {
            Box::new(deposit.clone())
        } else if let Some(inflation_bond) =
            instrument.as_any().downcast_ref::<InflationLinkedBond>()
        {
            Box::new(inflation_bond.clone())
        } else if let Some(fx_spot) = instrument.as_any().downcast_ref::<FxSpot>() {
            Box::new(fx_spot.clone())
        } else if let Some(fx_swap) = instrument.as_any().downcast_ref::<FxSwap>() {
            Box::new(fx_swap.clone())

        // Equity instruments
        } else if let Some(equity) = instrument.as_any().downcast_ref::<Equity>() {
            Box::new(equity.clone())

        // Options
        } else if let Some(equity_option) = instrument.as_any().downcast_ref::<EquityOption>() {
            Box::new(equity_option.clone())
        } else if let Some(fx_option) = instrument.as_any().downcast_ref::<FxOption>() {
            Box::new(fx_option.clone())
        } else if let Some(credit_option) = instrument.as_any().downcast_ref::<CreditOption>() {
            Box::new(credit_option.clone())
        } else if let Some(ir_option) = instrument.as_any().downcast_ref::<InterestRateOption>() {
            Box::new(ir_option.clone())
        } else if let Some(swaption) = instrument.as_any().downcast_ref::<Swaption>() {
            Box::new(swaption.clone())
        } else {
            return Err(finstack_core::error::InputError::NotFound {
                id: format!(
                    "unsupported instrument type for metrics computation: {}",
                    instrument.instrument_type()
                ),
            }
            .into());
        }
    };

    let mut context = MetricContext::new(
        Arc::from(instrument_clone),
        Arc::new(curves.clone()),
        as_of,
        base_value,
    );

    let registry = standard_registry();
    let metric_measures = registry.compute(metrics, &mut context)?;

    // Deterministic insertion order: follow the requested metrics slice order
    let mut measures: IndexMap<String, finstack_core::F> = IndexMap::new();
    for metric_id in metrics {
        if let Some(value) = metric_measures.get(metric_id) {
            measures.insert(metric_id.as_str().to_string(), *value);
        }
    }

    let mut result = crate::results::ValuationResult::stamped(instrument.id(), as_of, base_value);
    result.measures = measures;
    Ok(result)
}

/// Deprecated generic version for backward compatibility.
/// Use `build_with_metrics_dyn` instead to avoid coverage metadata conflicts.
#[deprecated(
    since = "0.3.0",
    note = "Use build_with_metrics_dyn to avoid coverage metadata conflicts"
)]
pub fn build_with_metrics<I>(
    instrument: I,
    curves: &MarketContext,
    as_of: Date,
    base_value: Money,
    metrics: &[crate::metrics::MetricId],
) -> finstack_core::Result<crate::results::ValuationResult>
where
    I: crate::instruments::traits::InstrumentLike + Clone + 'static,
{
    build_with_metrics_dyn(&instrument, curves, as_of, base_value, metrics)
}


