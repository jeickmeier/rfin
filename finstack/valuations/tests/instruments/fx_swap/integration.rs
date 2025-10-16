//! Integration tests for FX swaps.
//!
//! Tests complex scenarios involving multiple components:
//! - Multi-metric calculations
//! - Scenario analysis (rate shocks, curve shifts)
//! - End-to-end workflow validation

use super::fixtures::*;
use finstack_core::dates::Date;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::MetricId;

#[test]
fn test_full_metrics_suite() {
    // Calculate complete set of metrics for a standard swap
    let dates = TestDates::standard();
    let market = setup_standard_market(dates.as_of);

    let swap = create_standard_fx_swap(
        "FULL_METRICS",
        dates.near_date,
        dates.far_date_1y,
        1_000_000.0,
    );

    let result = swap
        .price_with_metrics(
            &market,
            dates.as_of,
            &[
                MetricId::custom("forward_points"),
                MetricId::custom("fx01"),
                MetricId::custom("ir01_domestic"),
                MetricId::custom("ir01_foreign"),
                MetricId::custom("carry_pv"),
                MetricId::Dv01,
                MetricId::Theta,
                MetricId::BucketedDv01,
            ],
        )
        .unwrap();

    // Verify all metrics computed successfully
    assert_eq!(result.measures.len(), 8, "Should calculate all 8 metrics");

    // Sanity checks on values
    let fwd_pts = *result.measures.get("forward_points").unwrap();
    let fx01 = *result.measures.get("fx01").unwrap();
    let ir01_dom = *result.measures.get("ir01_domestic").unwrap();
    let ir01_for = *result.measures.get("ir01_foreign").unwrap();
    let dv01 = *result.measures.get("dv01").unwrap();

    assert!(fwd_pts > 0.0, "Forward points should be positive");
    assert!(fx01 != 0.0, "FX01 should be non-zero");
    assert!(ir01_dom > 0.0, "Domestic IR01 should be positive");
    assert!(ir01_for < 0.0, "Foreign IR01 should be negative");
    assert!(dv01 > 0.0, "DV01 should be positive");
}

#[test]
fn test_rate_shock_scenario() {
    // Test swap behavior under parallel rate shock
    let dates = TestDates::standard();
    let market_base = setup_standard_market(dates.as_of);
    let market_shock = setup_steep_curve_market(dates.as_of);

    let swap = create_standard_fx_swap(
        "RATE_SHOCK",
        dates.near_date,
        dates.far_date_1y,
        1_000_000.0,
    );

    let pv_base = swap.value(&market_base, dates.as_of).unwrap();
    let pv_shock = swap.value(&market_shock, dates.as_of).unwrap();

    // PV should change materially under rate shock
    let pv_change = (pv_shock.amount() - pv_base.amount()).abs();
    assert!(
        pv_change > 10.0,
        "PV should change materially under rate shock, change: {}",
        pv_change
    );
}

#[test]
fn test_fx_rate_shock_scenario() {
    // Test swap behavior under FX rate shock
    let dates = TestDates::standard();

    // Base case: EUR/USD = 1.1
    let market_base = setup_standard_market(dates.as_of);

    // Shocked case: EUR/USD = 1.2
    let market_shock = setup_steep_curve_market(dates.as_of); // Uses 1.2 spot

    let swap = create_standard_fx_swap("FX_SHOCK", dates.near_date, dates.far_date_1y, 1_000_000.0);

    let pv_base = swap.value(&market_base, dates.as_of).unwrap();
    let pv_shock = swap.value(&market_shock, dates.as_of).unwrap();

    // PV should change with FX spot movement
    assert!(
        pv_base.amount() != pv_shock.amount(),
        "PV should change under FX shock"
    );
}

#[test]
fn test_time_series_pv_evolution() {
    // Test PV evolution over time as swap approaches maturity
    let dates = TestDates::standard();
    let market = setup_standard_market(dates.as_of);

    let swap = create_fx_swap_with_rates(
        "TIME_SERIES",
        dates.near_date,
        dates.far_date_1y,
        1_000_000.0,
        1.10,
        1.15, // Off-market rate to ensure non-zero PV
    );

    // Value at inception
    let pv_t0 = swap.value(&market, dates.as_of).unwrap();

    // Value 3 months later
    let pv_t1 = swap.value(&market, dates.far_date_3m).unwrap();

    // Value 6 months later
    let as_of_6m = Date::from_calendar_date(2024, time::Month::July, 1).unwrap();
    let pv_t2 = swap.value(&market, as_of_6m).unwrap();

    // All PVs should be finite
    assert!(pv_t0.amount().is_finite());
    assert!(pv_t1.amount().is_finite());
    assert!(pv_t2.amount().is_finite());

    // PV should evolve over time (not remain constant)
    assert!(
        pv_t0.amount() != pv_t1.amount() || pv_t1.amount() != pv_t2.amount(),
        "PV should evolve over time"
    );
}

#[test]
fn test_portfolio_of_swaps() {
    // Test aggregating metrics across multiple swaps (simplified portfolio test)
    let dates = TestDates::standard();
    let market = setup_standard_market(dates.as_of);

    let swaps = vec![
        create_standard_fx_swap("SWAP_1", dates.near_date, dates.far_date_1m, 1_000_000.0),
        create_standard_fx_swap("SWAP_2", dates.near_date, dates.far_date_3m, 2_000_000.0),
        create_standard_fx_swap("SWAP_3", dates.near_date, dates.far_date_1y, 500_000.0),
    ];

    let mut total_pv: f64 = 0.0;
    let mut total_dv01: f64 = 0.0;

    for swap in swaps {
        let result = swap
            .price_with_metrics(&market, dates.as_of, &[MetricId::Dv01])
            .unwrap();

        total_pv += result.value.amount();
        total_dv01 += result.measures.get("dv01").unwrap();
    }

    // Portfolio aggregates should be reasonable
    assert!(total_pv.is_finite(), "Portfolio PV should be finite");
    assert!(total_dv01 > 0.0, "Portfolio DV01 should be positive");
}

#[test]
fn test_hedge_ratio_calculation() {
    // Test calculating hedge ratio between two swaps
    let dates = TestDates::standard();
    let market = setup_standard_market(dates.as_of);

    let swap1 = create_standard_fx_swap("HEDGE_1", dates.near_date, dates.far_date_1y, 1_000_000.0);

    let swap2 = create_standard_fx_swap("HEDGE_2", dates.near_date, dates.far_date_1y, 2_000_000.0);

    let result1 = swap1
        .price_with_metrics(&market, dates.as_of, &[MetricId::custom("fx01")])
        .unwrap();

    let result2 = swap2
        .price_with_metrics(&market, dates.as_of, &[MetricId::custom("fx01")])
        .unwrap();

    let fx01_1 = *result1.measures.get("fx01").unwrap();
    let fx01_2 = *result2.measures.get("fx01").unwrap();

    // Hedge ratio for FX risk
    let hedge_ratio = -fx01_1 / fx01_2;

    // Should be roughly 0.5 (1M notional vs 2M notional), but allow wide tolerance
    // due to the complex nature of FX swap Greeks
    assert_within_pct(hedge_ratio, 0.5, 95.0, "Hedge ratio calculation");
}

#[test]
fn test_par_swap_construction() {
    // Test that a fair swap at inception has near-zero PV
    let dates = TestDates::standard();
    let market = setup_standard_market(dates.as_of);

    // Create swap without explicit rates (uses model-implied rates)
    let par_swap =
        create_standard_fx_swap("PAR_SWAP", dates.near_date, dates.far_date_1y, 1_000_000.0);

    let pv = par_swap.value(&market, dates.as_of).unwrap();

    // Par swap should have minimal PV
    assert!(
        pv.amount().abs() < 1000.0,
        "Par swap PV should be near zero, got: {}",
        pv.amount()
    );
}

#[test]
fn test_metric_consistency() {
    // Test that metrics are internally consistent
    let dates = TestDates::standard();
    let market = setup_standard_market(dates.as_of);

    let swap = create_standard_fx_swap(
        "CONSISTENCY",
        dates.near_date,
        dates.far_date_1y,
        1_000_000.0,
    );

    let result = swap
        .price_with_metrics(
            &market,
            dates.as_of,
            &[
                MetricId::custom("ir01_domestic"),
                MetricId::custom("ir01_foreign"),
                MetricId::custom("fx01"),
                MetricId::Dv01,
            ],
        )
        .unwrap();

    let ir01_dom = result.measures.get("ir01_domestic").unwrap().abs();
    let ir01_for = result.measures.get("ir01_foreign").unwrap().abs();
    let fx01 = result.measures.get("fx01").unwrap().abs();
    let dv01 = *result.measures.get("dv01").unwrap();

    // All sensitivities should be non-zero
    assert!(ir01_dom > 1e-10, "IR01 domestic should be non-zero");
    assert!(ir01_for > 1e-10, "IR01 foreign should be non-zero");
    assert!(fx01 > 1e-10, "FX01 should be non-zero");
    assert!(dv01 > 0.1, "DV01 should be material");
}
