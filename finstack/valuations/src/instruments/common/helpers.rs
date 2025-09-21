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
    instrument: &dyn crate::instruments::common::traits::Instrument,
    curves: &MarketContext,
    as_of: Date,
    base_value: Money,
    metrics: &[crate::metrics::MetricId],
) -> finstack_core::Result<crate::results::ValuationResult> {
    // Create an owned clone for the Arc to avoid lifetime issues
    // This approach reduces generic monomorphization across compilation units
    let instrument_clone: Box<dyn crate::instruments::common::traits::Instrument> = instrument.clone_box();

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

    // Instrument-specific metadata stamping (non-invasive, opt-in by type)
    // CDSIndex: stamp step-in, effective dates, coupon-day info
    if instrument.instrument_type() == "CDSIndex" {
        if let Some(idx) = instrument
            .as_any()
            .downcast_ref::<crate::instruments::cds_index::CDSIndex>()
        {
            // Step-in date: T+1 (calendar day). For full business-day support, integrate calendars.
            let step_in = as_of + time::Duration::days(1);
            let effective = idx.premium.start;
            // Use documented ISDA standard coupon day
            let coupon_day =
                crate::instruments::cds::pricing::engine::isda_constants::STANDARD_COUPON_DAY;
            result
                .meta
                .custom
                .insert("step_in_date".to_string(), format!("{}", step_in));
            result
                .meta
                .custom
                .insert("effective_date".to_string(), format!("{}", effective));
            result
                .meta
                .custom
                .insert("coupon_day".to_string(), format!("{}", coupon_day));
            result
                .meta
                .custom
                .insert("use_isda_coupon_dates".to_string(), "true".to_string());
        }
    }
    Ok(result)
}

/// Ensure all money amounts in a collection share the same currency.
pub fn validate_currency_consistency(amounts: &[Money]) -> finstack_core::Result<()> {
    if amounts.is_empty() {
        return Ok(());
    }

    let expected_currency = amounts[0].currency();
    for amount in amounts.iter().skip(1) {
        if amount.currency() != expected_currency {
            return Err(finstack_core::Error::CurrencyMismatch {
                expected: expected_currency,
                actual: amount.currency(),
            });
        }
    }
    Ok(())
}
