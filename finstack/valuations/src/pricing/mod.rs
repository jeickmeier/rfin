//! Pricing-related utilities and interfaces.

pub mod discountable;
pub mod npv;
pub mod result;

/// Shared helper to build a ValuationResult with a set of metrics.
///
/// This centralizes the repeated pattern across instruments to compute
/// base value, build metric context, compute metrics and stamp a result.
pub fn build_with_metrics(
    instrument: crate::instruments::Instrument,
    curves: &finstack_core::market_data::multicurve::CurveSet,
    as_of: finstack_core::dates::Date,
    base_value: finstack_core::money::Money,
    metrics: &[crate::metrics::MetricId],
) -> finstack_core::Result<crate::pricing::result::ValuationResult> {
    use crate::metrics::{standard_registry, MetricContext};
    use indexmap::IndexMap;
    use std::sync::Arc;

    let mut context = MetricContext::new(
        Arc::new(instrument),
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

    let mut result = crate::pricing::result::ValuationResult::stamped(
        context.instrument.id().to_string(),
        as_of,
        base_value,
    );
    result.measures = measures;
    Ok(result)
}
