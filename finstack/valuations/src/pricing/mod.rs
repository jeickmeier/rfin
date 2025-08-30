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
    use std::sync::Arc;

    let mut context = MetricContext::new(
        Arc::new(instrument),
        Arc::new(curves.clone()),
        as_of,
        base_value,
    );

    let registry = standard_registry();
    let metric_measures = registry.compute(metrics, &mut context)?;

    let measures: hashbrown::HashMap<String, finstack_core::F> = metric_measures
        .into_iter()
        .map(|(k, v)| (k.as_str().to_string(), v))
        .collect();

    let mut result = crate::pricing::result::ValuationResult::stamped(
        context.instrument.id().to_string(),
        as_of,
        base_value,
    );
    result.measures = measures;
    Ok(result)
}
