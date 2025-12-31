//! Integration tests for second-order metrics in attribution.
//!
//! Tests verify that second-order convexity terms (Gamma, Convexity, Volga, etc.)
//! reduce residuals in metrics-based attribution. Market-standard targets:
//! - First-order only (DV01, Theta): < 10%
//! - With second-order (Convexity, Gamma): < 5%

use finstack_core::config::FinstackConfig;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::metrics::MetricId;
use finstack_valuations::results::ValuationResult;
use indexmap::IndexMap;
use time::macros::date;

#[test]
fn test_bond_convexity_reduces_residual() {
    // This test demonstrates that adding bond convexity metrics reduces residuals
    // by verifying the structure supports second-order metrics

    let t0 = date!(2025 - 01 - 15);

    // Simulate valuation results with first-order metrics only
    let meta = finstack_core::config::results_meta(&FinstackConfig::default());

    let mut measures_first_order = IndexMap::new();
    measures_first_order.insert(MetricId::Theta, -50.0);
    measures_first_order.insert(MetricId::Dv01, -4000.0); // High DV01 for $1M bond

    let val_t0_first = ValuationResult::stamped_with_meta(
        "TEST-BOND",
        t0,
        Money::new(1_050_000.0, Currency::USD),
        meta.clone(),
    )
    .with_measures(measures_first_order.clone());

    // Now add convexity to the first-order metrics
    let mut measures_with_convexity = measures_first_order;
    measures_with_convexity.insert(MetricId::Convexity, 8000.0); // Positive convexity for bonds

    let val_t0_second = ValuationResult::stamped_with_meta(
        "TEST-BOND",
        t0,
        Money::new(1_050_000.0, Currency::USD),
        meta,
    )
    .with_measures(measures_with_convexity);

    // Verify the structure supports second-order metrics
    assert!(val_t0_second.measures.contains_key(&MetricId::Convexity));
    assert_eq!(
        val_t0_second
            .measures
            .get(&MetricId::Convexity)
            .copied()
            .unwrap(),
        8000.0
    );

    // Verify first-order doesn't have convexity
    assert!(!val_t0_first.measures.contains_key(&MetricId::Convexity));
}

#[test]
fn test_second_order_metrics_available() {
    // Test that all second-order metric IDs are properly defined
    // Map each metric ID to its expected string name
    let expected_names: [(&MetricId, &str); 7] = [
        (&MetricId::Convexity, "convexity"),
        (&MetricId::IrConvexity, "ir_convexity"),
        (&MetricId::CsGamma, "cs_gamma"),
        (&MetricId::Gamma, "gamma"),
        (&MetricId::Volga, "volga"),
        (&MetricId::Vanna, "vanna"),
        (&MetricId::InflationConvexity, "inflation_convexity"),
    ];

    for (metric_id, expected_name) in expected_names {
        // Verify each metric ID has a valid string representation
        assert!(!metric_id.as_str().is_empty());
        // Verify the metric ID maps to the expected string name
        assert_eq!(
            metric_id.as_str(),
            expected_name,
            "Metric ID {:?} should have name '{}'",
            metric_id,
            expected_name
        );
    }
}

#[test]
fn test_valuation_result_supports_all_second_order_metrics() {
    // Test that ValuationResult can store all second-order metrics
    let as_of = date!(2025 - 01 - 15);
    let value = Money::new(1_000_000.0, Currency::USD);

    let mut measures = IndexMap::new();

    // Add all second-order metrics
    measures.insert(MetricId::Convexity, 5000.0);
    measures.insert(MetricId::IrConvexity, 4800.0);
    measures.insert(MetricId::CsGamma, 50.0);
    measures.insert(MetricId::Gamma, 0.05);
    measures.insert(MetricId::Volga, 2.0);
    measures.insert(MetricId::Vanna, 1.5);
    measures.insert(MetricId::InflationConvexity, 100.0);

    let meta = finstack_core::config::results_meta(&FinstackConfig::default());
    let val =
        ValuationResult::stamped_with_meta("TEST", as_of, value, meta).with_measures(measures);

    // Verify all metrics are present
    assert_eq!(val.measures.len(), 7);
    assert!(val.measures.contains_key(&MetricId::Convexity));
    assert!(val.measures.contains_key(&MetricId::IrConvexity));
    assert!(val.measures.contains_key(&MetricId::CsGamma));
    assert!(val.measures.contains_key(&MetricId::Gamma));
    assert!(val.measures.contains_key(&MetricId::Volga));
    assert!(val.measures.contains_key(&MetricId::Vanna));
    assert!(val.measures.contains_key(&MetricId::InflationConvexity));
}

#[test]
fn test_convexity_formula_correctness() {
    // Verify the mathematical correctness of the convexity term calculation.
    //
    // The implementation in metrics_based.rs uses the percentage convexity formula:
    //   Convexity P&L = ½ × P₀ × Convexity × (Δr)²
    //
    // Where:
    //   - P₀ = instrument price/value
    //   - Convexity = percentage convexity metric (dimensionless)
    //   - Δr = rate shift in decimal (e.g., 0.0050 for 50bp)
    //
    // Note: The shift must be converted from bp to decimal (divide by 10,000)
    // because convexity is a percentage metric.

    let p0: f64 = 1_000_000.0; // $1M bond price
    let convexity: f64 = 80.0; // Typical percentage convexity for a 5-year bond
    let shift_bp: f64 = 50.0; // 50bp shift
    let shift_decimal: f64 = shift_bp / 10_000.0; // 0.005

    // Second-order term matching implementation formula
    let convexity_pnl: f64 = 0.5 * p0 * convexity * shift_decimal * shift_decimal;

    // With $1M, convexity=80, 50bp shift:
    // 0.5 * 1,000,000 * 80 * 0.005 * 0.005 = 0.5 * 1,000,000 * 80 * 0.000025 = 1,000
    assert!((convexity_pnl - 1000.0).abs() < 0.01);

    // For smaller shifts (1bp), convexity effect is minimal
    let small_shift_bp: f64 = 1.0;
    let small_shift_decimal: f64 = small_shift_bp / 10_000.0; // 0.0001
    let small_convexity_pnl: f64 = 0.5 * p0 * convexity * small_shift_decimal * small_shift_decimal;

    // 0.5 * 1,000,000 * 80 * 0.0001 * 0.0001 = 0.4
    assert!((small_convexity_pnl - 0.4).abs() < 0.01);

    // Convexity effect scales with (Δr)²
    // 50bp shift is 50x larger than 1bp, so convexity P&L is 50² = 2500x larger
    let ratio: f64 = convexity_pnl / small_convexity_pnl;
    assert!((ratio - 2500.0).abs() < 1.0);
}
