//! Tests for graceful metric calculation error handling.
//!
//! Verifies that when a metric calculation fails or is missing,
//! the system continues computing other metrics and returns 0.0
//! for the failed metric.

use finstack_core::prelude::*;
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::{MetricContext, MetricId, MetricRegistry};
use std::sync::Arc;
use time::macros::date;

#[test]
fn test_missing_metric_returns_zero() {
    // Create a simple bond
    let bond = Bond::fixed(
        "TEST_BOND",
        Money::new(1000.0, Currency::USD),
        0.05, // 5% coupon
        date!(2024 - 01 - 01),
        date!(2025 - 01 - 01),
        "USD_OIS",
    );

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

    // Should not error
    assert!(result.is_ok(), "Should handle missing metric gracefully");

    let computed = result.unwrap();

    // Should return 0.0 for missing metric
    assert_eq!(
        computed.get(&MetricId::Ytm),
        Some(&0.0),
        "Missing metric should return 0.0"
    );
}

#[test]
fn test_partial_metric_failure_continues() {
    // Create a simple bond
    let bond = Bond::fixed(
        "TEST_BOND",
        Money::new(1000.0, Currency::USD),
        0.05,
        date!(2024 - 01 - 01),
        date!(2025 - 01 - 01),
        "USD_OIS",
    );

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
    let result = registry.compute(&metrics, &mut context);

    // Should not error
    assert!(
        result.is_ok(),
        "Should handle all missing metrics gracefully"
    );

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
    );

    // Create a minimal market context (might not have all curves needed)
    let market = MarketContext::new();

    let as_of = date!(2024 - 06 - 01);
    let base_value = Money::new(1000.0, Currency::USD);

    // Create context
    let instrument_arc: Arc<dyn Instrument> = Arc::new(bond);
    let mut context = MetricContext::new(instrument_arc, Arc::new(market), as_of, base_value);

    // Use standard registry which has bond metrics
    let registry = standard_registry();

    // Try to compute metrics - some might fail due to missing market data
    let metrics = vec![MetricId::Ytm, MetricId::DurationMac];
    let result = registry.compute(&metrics, &mut context);

    // Should not error even if calculations fail
    assert!(
        result.is_ok(),
        "Should handle calculation failures gracefully"
    );

    let computed = result.unwrap();

    // Should have results for all requested metrics (either computed or 0.0 fallback)
    assert_eq!(
        computed.len(),
        2,
        "Should return results for all requested metrics"
    );
}
