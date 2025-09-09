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
    

    // Create an owned clone for the Arc to avoid lifetime issues
    // This approach reduces generic monomorphization across compilation units
    let instrument_clone: Box<dyn crate::instruments::traits::InstrumentLike> = instrument.clone_box();

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

// Deprecated generic version for backward compatibility.
// Use `build_with_metrics_dyn` instead to avoid coverage metadata conflicts.
