//! Tests for graceful metric calculation error handling.
//!
//! Verifies strict vs best-effort behavior.
//!
//! - In strict mode (default), unknown/missing metrics return errors.
//! - In best-effort mode, failures are coerced to `0.0` and the computation
//!   continues.

use finstack_core::currency::Currency;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::Error;
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::{MetricContext, MetricId, MetricRegistry};
use std::sync::Arc;
use time::macros::date;

#[test]
fn test_missing_metric_errors_in_strict_mode() {
    // Create a simple bond
    let bond = Bond::fixed(
        "TEST_BOND",
        Money::new(1000.0, Currency::USD),
        0.05, // 5% coupon
        date!(2024 - 01 - 01),
        date!(2025 - 01 - 01),
        "USD_OIS",
    ).unwrap();

    // Create a market context (empty for this test)
    let market = MarketContext::new();

    let as_of = date!(2024 - 06 - 01);
    let base_value = Money::new(1000.0, Currency::USD);

    // Create context
    let instrument_arc: Arc<dyn Instrument> = Arc::new(bond);
    let mut context = MetricContext::new(instrument_arc, Arc::new(market), as_of, base_value);

    // Create empty registry
    let registry = MetricRegistry::new();

    // Try to compute a metric that doesn't exist (e.g., Ytm)
    let metrics = vec![MetricId::Ytm];
    let result = registry.compute(&metrics, &mut context);

    match result {
        Err(Error::UnknownMetric { metric_id, .. }) => {
            assert_eq!(metric_id, "ytm");
        }
        Err(other) => panic!("unexpected error: {}", other),
        Ok(_) => panic!("expected strict mode to error on missing metric"),
    }
}

#[test]
fn test_missing_metrics_return_zero_in_best_effort_mode() {
    // Create a simple bond
    let bond = Bond::fixed(
        "TEST_BOND",
        Money::new(1000.0, Currency::USD),
        0.05,
        date!(2024 - 01 - 01),
        date!(2025 - 01 - 01),
        "USD_OIS",
    ).unwrap();

    // Create a market context
    let market = MarketContext::new();

    let as_of = date!(2024 - 06 - 01);
    let base_value = Money::new(1000.0, Currency::USD);

    // Create context
    let instrument_arc: Arc<dyn Instrument> = Arc::new(bond);
    let mut context = MetricContext::new(instrument_arc, Arc::new(market), as_of, base_value);

    // Create empty registry
    let registry = MetricRegistry::new();

    // Try to compute multiple metrics, none of which exist
    let metrics = vec![MetricId::Ytm, MetricId::DurationMac, MetricId::Dv01];
    let result = registry.compute_best_effort(&metrics, &mut context);

    assert!(result.is_ok(), "best-effort mode should never error");

    let computed = result.unwrap();

    // All should return 0.0
    assert_eq!(computed.len(), 3, "Should return all requested metrics");
    assert_eq!(computed.get(&MetricId::Ytm), Some(&0.0));
    assert_eq!(computed.get(&MetricId::DurationMac), Some(&0.0));
    assert_eq!(computed.get(&MetricId::Dv01), Some(&0.0));
}

#[test]
fn test_some_metrics_succeed_some_fail() {
    use finstack_valuations::metrics::standard_registry;

    // Create a simple bond
    let bond = Bond::fixed(
        "TEST_BOND",
        Money::new(1000.0, Currency::USD),
        0.05,
        date!(2024 - 01 - 01),
        date!(2025 - 01 - 01),
        "USD_OIS",
    ).unwrap();

    // Create a minimal market context (might not have all curves needed)
    let market = MarketContext::new();

    let as_of = date!(2024 - 06 - 01);
    let base_value = Money::new(1000.0, Currency::USD);

    // Create context
    let instrument_arc: Arc<dyn Instrument> = Arc::new(bond);
    let mut context = MetricContext::new(instrument_arc, Arc::new(market), as_of, base_value);

    // Use standard registry which has bond metrics
    let registry = standard_registry();

    // Try to compute metrics - they will fail in strict mode due to missing market data,
    // but should be coerced to 0.0 in best-effort mode.
    let metrics = vec![MetricId::Ytm, MetricId::DurationMac];
    let result = registry.compute_best_effort(&metrics, &mut context);

    assert!(result.is_ok(), "best-effort mode should never error");

    let computed = result.unwrap();

    // Should have results for all requested metrics (either computed or 0.0 fallback)
    assert_eq!(
        computed.len(),
        2,
        "Should return results for all requested metrics"
    );
}
