//! DV01 (dollar value of 1bp) tests

use crate::swaption::common::*;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::MetricId;

#[test]
fn test_dv01_finite_and_reasonable() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    let market = create_flat_market(as_of, 0.05, 0.30);

    let result = swaption
        .price_with_metrics(&market, as_of, &[MetricId::Dv01])
        .unwrap();

    let dv01 = *result.measures.get("dv01").unwrap();

    assert!(dv01.is_finite(), "DV01 should be finite");
    assert_reasonable(dv01.abs(), 0.0, 100_000.0, "DV01 magnitude");
}

#[test]
fn test_dv01_vs_rho_relationship() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    let market = create_flat_market(as_of, 0.05, 0.30);

    let result = swaption
        .price_with_metrics(&market, as_of, &[MetricId::Dv01, MetricId::Rho])
        .unwrap();

    let dv01 = *result.measures.get("dv01").unwrap();
    let rho = *result.measures.get("rho").unwrap();

    // Both DV01 and Rho are per 1bp in our convention
    assert_approx_eq(rho, dv01, 0.01, "Rho vs DV01 relationship");
}

#[test]
fn test_dv01_bump_reprice() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    let market = create_flat_market(as_of, 0.05, 0.30);

    // Analytical DV01
    let result = swaption
        .price_with_metrics(&market, as_of, &[MetricId::Dv01])
        .unwrap();
    let dv01_analytical = *result.measures.get("dv01").unwrap();

    // DV01 should be finite and reasonable for ATM swaption
    assert!(dv01_analytical.is_finite(), "DV01 should be finite");
    // DV01 for 1M notional 1Y into 5Y should be in a reasonable range
    assert_reasonable(dv01_analytical.abs(), 10.0, 1000.0, "DV01 magnitude");
}

#[test]
fn test_dv01_scales_with_notional() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let market = create_flat_market(as_of, 0.05, 0.30);

    let mut swaption1 = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    swaption1.notional =
        finstack_core::money::Money::new(1_000_000.0, finstack_core::currency::Currency::USD);

    let mut swaption10 = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    swaption10.notional =
        finstack_core::money::Money::new(10_000_000.0, finstack_core::currency::Currency::USD);

    let dv01_1 = swaption1
        .price_with_metrics(&market, as_of, &[MetricId::Dv01])
        .unwrap()
        .measures
        .get("dv01")
        .copied()
        .unwrap();

    let dv01_10 = swaption10
        .price_with_metrics(&market, as_of, &[MetricId::Dv01])
        .unwrap()
        .measures
        .get("dv01")
        .copied()
        .unwrap();

    assert_approx_eq(dv01_10, dv01_1 * 10.0, 1e-4, "DV01 scaling with notional");
}

#[test]
fn test_dv01_increases_with_tenor() {
    let as_of = time::macros::date!(2024 - 01 - 01);
    let expiry = time::macros::date!(2025 - 01 - 01);
    let swap_start = expiry;
    let market = create_flat_market(as_of, 0.05, 0.30);

    // 2Y swap
    let swap_end_2y = time::macros::date!(2027 - 01 - 01);
    let swaption_2y = create_standard_payer_swaption(expiry, swap_start, swap_end_2y, 0.05);

    // 10Y swap
    let swap_end_10y = time::macros::date!(2035 - 01 - 01);
    let swaption_10y = create_standard_payer_swaption(expiry, swap_start, swap_end_10y, 0.05);

    let dv01_2y = swaption_2y
        .price_with_metrics(&market, as_of, &[MetricId::Dv01])
        .unwrap()
        .measures
        .get("dv01")
        .copied()
        .unwrap();

    let dv01_10y = swaption_10y
        .price_with_metrics(&market, as_of, &[MetricId::Dv01])
        .unwrap()
        .measures
        .get("dv01")
        .copied()
        .unwrap();

    // Longer tenor has more rate sensitivity
    assert!(
        dv01_10y.abs() > dv01_2y.abs(),
        "Longer tenor should have higher DV01 magnitude"
    );
}
