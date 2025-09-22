//! Utilities for instrument pricing and metrics assembly.
//!
//! Contains helpers shared across instrument implementations, notably the
//! function to assemble a `ValuationResult` with computed metrics.

use crate::metrics::{standard_registry, MetricContext};
use crate::cashflow::traits::CashflowProvider;
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
    let instrument_clone: Box<dyn crate::instruments::common::traits::Instrument> =
        instrument.clone_box();

    let mut context = MetricContext::new(
        Arc::from(instrument_clone),
        Arc::new(curves.clone()),
        as_of,
        base_value,
    );

    // Allow instrument to pre-populate context (cashflows, curve IDs, day count, resolvers)
    // This keeps the engine generic while enabling instrument-specific caching hints.
    let instrument_for_prepare = Arc::clone(&context.instrument);
    instrument_for_prepare.prepare_metric_context(&mut context)?;

    // Simple per-instrument context hints to improve generic engine defaults
    // without requiring custom per-instrument assembly logic.
    // These hints are best-effort and only fill missing fields.
    // Bond
    if let Some(bond) = context
        .instrument
        .as_any()
        .downcast_ref::<crate::instruments::bond::Bond>()
    {
        if context.cashflows.is_none() {
            if let Ok(flows) = <crate::instruments::bond::Bond as CashflowProvider>::build_schedule(
                bond,
                &context.curves,
                context.as_of,
            ) {
                context.cashflows = Some(flows);
            }
        }
        if context.discount_curve_id.is_none() {
            context.discount_curve_id = Some(bond.disc_id.clone());
        }
        if context.day_count.is_none() {
            context.day_count = Some(bond.schedule.dc);
        }
    }

    // Interest Rate Swap
    if let Some(irs) = context
        .instrument
        .as_any()
        .downcast_ref::<crate::instruments::irs::InterestRateSwap>()
    {
        if context.cashflows.is_none() {
            if let Ok(flows) = <crate::instruments::irs::InterestRateSwap as CashflowProvider>::build_schedule(
                irs,
                &context.curves,
                context.as_of,
            ) {
                context.cashflows = Some(flows);
            }
        }
        if context.discount_curve_id.is_none() {
            context.discount_curve_id = Some(finstack_core::types::CurveId::new(irs.fixed.disc_id));
        }
        if context.day_count.is_none() {
            context.day_count = Some(irs.fixed.schedule.dc);
        }
    }

    let registry = standard_registry();
    // If no metrics explicitly requested, compute all applicable metrics for the instrument type
    let metric_measures = if metrics.is_empty() {
        registry.compute_all(&mut context)?
    } else {
        registry.compute(metrics, &mut context)?
    };

    // Deterministic insertion order: follow the requested metrics slice order
    let mut measures: IndexMap<String, finstack_core::F> = IndexMap::new();
    if metrics.is_empty() {
        // Deterministic order: sort by metric name for stability when computing all
        let mut pairs: Vec<(String, finstack_core::F)> = metric_measures
            .iter()
            .map(|(k, v)| (k.as_str().to_string(), *v))
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, value) in pairs {
            measures.insert(name, value);
        }
    } else {
        // Preserve caller-provided order
        for metric_id in metrics {
            if let Some(value) = metric_measures.get(metric_id) {
                measures.insert(metric_id.as_str().to_string(), *value);
            }
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
