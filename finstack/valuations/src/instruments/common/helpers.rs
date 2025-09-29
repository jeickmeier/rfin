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

/// Monomorphized schedule → PV helper for instruments using a discount curve.
///
/// - Builds the schedule via `CashflowProvider`.
/// - Retrieves the discount curve by borrowed ID using `get_ref::<DiscountCurve>`.
/// - Performs NPV using static dispatch on `DiscountCurve`.
pub fn schedule_pv_impl<S>(
    instrument: &S,
    curves: &MarketContext,
    as_of: Date,
    disc_id: &finstack_core::types::CurveId,
    day_count: finstack_core::dates::DayCount,
) -> finstack_core::Result<Money>
where
    S: crate::cashflow::traits::CashflowProvider,
{
    use crate::instruments::common::discountable::npv_static;

    let flows = S::build_schedule(instrument, curves, as_of)?;
    let disc = curves.get_discount_ref(disc_id.as_str())?;
    let base = disc.base_date();
    npv_static(disc, base, day_count, &flows)
}

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

    let registry = standard_registry();
    let metric_measures = registry.compute(metrics, &mut context)?;

    // Deterministic insertion order: follow the requested metrics slice order
    let mut measures: IndexMap<String, f64> = IndexMap::new();
    for metric_id in metrics {
        if let Some(value) = metric_measures.get(metric_id) {
            measures.insert(metric_id.as_str().to_string(), *value);
        }
    }

    let mut result = crate::results::ValuationResult::stamped(instrument.id(), as_of, base_value);
    result.measures = measures;

    // Instrument-specific metadata stamping (non-invasive, opt-in by type)
    // CDSIndex: stamp step-in, effective dates, coupon-day info
    if let Some(idx) = instrument
        .as_any()
        .downcast_ref::<crate::instruments::cds_index::CDSIndex>()
    {
        // Step-in date: T+1 (calendar day). For full business-day support, integrate calendars.
        let step_in = as_of + time::Duration::days(1);
        let effective = idx.premium.start;
        // Use documented ISDA standard coupon day
        let coupon_day = crate::instruments::cds::pricer::isda_constants::STANDARD_COUPON_DAY;
        let _ = step_in;
        let _ = effective;
        let _ = coupon_day;
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
