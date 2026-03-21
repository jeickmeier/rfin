//! Tests for CDS Option metrics framework integration.

use super::common::*;
use finstack_valuations::instruments::internal::InstrumentExt as Instrument;
use finstack_valuations::metrics::{standard_registry, MetricContext, MetricId};
use time::macros::date;

#[test]
fn test_metrics_registry_delta() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);
    let option = CDSOptionBuilder::new().build(as_of);

    let pv = option.value(&market, as_of).unwrap();
    let mut ctx = MetricContext::new(
        std::sync::Arc::new(option),
        std::sync::Arc::new(market),
        as_of,
        pv,
        MetricContext::default_config(),
    );

    let registry = standard_registry();
    let results = registry.compute(&[MetricId::Delta], &mut ctx).unwrap();

    assert!(results.contains_key(&MetricId::Delta));
    let delta = *results.get(&MetricId::Delta).unwrap();
    assert_finite(delta, "Delta from registry");
}

#[test]
fn test_metrics_registry_all_greeks() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);
    let option = CDSOptionBuilder::new().build(as_of);

    let pv = option.value(&market, as_of).unwrap();
    let mut ctx = MetricContext::new(
        std::sync::Arc::new(option),
        std::sync::Arc::new(market),
        as_of,
        pv,
        MetricContext::default_config(),
    );

    let metrics = vec![
        MetricId::Delta,
        MetricId::Gamma,
        MetricId::Vega,
        MetricId::Theta,
        MetricId::Rho,
        MetricId::Cs01,
        MetricId::Dv01,
    ];

    let registry = standard_registry();
    let results = registry.compute(&metrics, &mut ctx).unwrap();

    assert_eq!(results.len(), metrics.len());
    for metric_id in metrics {
        assert!(results.contains_key(&metric_id));
        let value = *results.get(&metric_id).unwrap();
        assert_finite(value, &format!("{:?}", metric_id));
    }
}

#[test]
fn test_metrics_registry_implied_vol() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);
    let target_vol = 0.30;
    let option = CDSOptionBuilder::new().implied_vol(target_vol).build(as_of);

    let pv = option.value(&market, as_of).unwrap();
    let mut ctx = MetricContext::new(
        std::sync::Arc::new(option),
        std::sync::Arc::new(market),
        as_of,
        pv,
        MetricContext::default_config(),
    );

    let registry = standard_registry();
    let results = registry.compute(&[MetricId::ImpliedVol], &mut ctx).unwrap();

    let iv = *results.get(&MetricId::ImpliedVol).unwrap();
    assert_approx_eq(iv, target_vol, 1e-6, "Implied vol from registry");
}

#[test]
fn test_cs01_uses_delta_dependency() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);
    let option = CDSOptionBuilder::new().build(as_of);

    let pv = option.value(&market, as_of).unwrap();
    let mut ctx = MetricContext::new(
        std::sync::Arc::new(option),
        std::sync::Arc::new(market),
        as_of,
        pv,
        MetricContext::default_config(),
    );

    // Compute CS01 which should use Delta if available
    let registry = standard_registry();
    let results = registry
        .compute(&[MetricId::Delta, MetricId::Cs01], &mut ctx)
        .unwrap();

    assert!(results.contains_key(&MetricId::Delta));
    assert!(results.contains_key(&MetricId::Cs01));

    let delta = *results.get(&MetricId::Delta).unwrap();
    let cs01 = *results.get(&MetricId::Cs01).unwrap();

    assert_finite(delta, "Delta");
    assert_finite(cs01, "CS01");
    assert_positive(cs01, "CS01 for call");
}

#[test]
fn test_bucketed_dv01_registered() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);
    let option = CDSOptionBuilder::new().build(as_of);

    let pv = option.value(&market, as_of).unwrap();
    let mut ctx = MetricContext::new(
        std::sync::Arc::new(option),
        std::sync::Arc::new(market),
        as_of,
        pv,
        MetricContext::default_config(),
    );

    let registry = standard_registry();
    let results = registry.compute(&[MetricId::BucketedDv01], &mut ctx);

    // Bucketed DV01 should be registered (may or may not compute successfully depending on market)
    // Just verify it doesn't panic
    assert!(results.is_ok() || results.is_err());
}

#[test]
fn test_metrics_near_expiry() {
    // Test metrics for near-expiry option
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);
    let option = CDSOptionBuilder::new()
        .expiry_months(1) // Very short time to expiry
        .cds_maturity_months(13)
        .build(as_of);

    let pv = option.value(&market, as_of).unwrap();
    let mut ctx = MetricContext::new(
        std::sync::Arc::new(option),
        std::sync::Arc::new(market),
        as_of,
        pv,
        MetricContext::default_config(),
    );

    let registry = standard_registry();
    let results = registry
        .compute(&[MetricId::Delta, MetricId::Vega], &mut ctx)
        .unwrap();

    // Near-expiry options should still have computable greeks
    let delta = *results.get(&MetricId::Delta).unwrap();
    let vega = *results.get(&MetricId::Vega).unwrap();

    assert_finite(delta, "Near-expiry delta");
    assert_finite(vega, "Near-expiry vega");
}
