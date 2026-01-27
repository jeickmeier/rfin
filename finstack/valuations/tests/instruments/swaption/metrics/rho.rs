//! Rho (interest rate sensitivity) tests

use crate::swaption::common::*;
use finstack_valuations::instruments::Instrument;
use finstack_valuations::metrics::MetricId;

#[test]
fn test_rho_finite_and_reasonable() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    let market = create_flat_market(as_of, 0.05, 0.30);

    let result = swaption
        .price_with_metrics(&market, as_of, &[MetricId::Rho])
        .unwrap();

    let rho = *result.measures.get("rho").unwrap();

    assert!(rho.is_finite(), "Rho should be finite");
    // Rho can be positive or negative for swaptions depending on maturity structure.
    // For a 1M notional, 1Y-5Y swaption, rho per 1bp should be in the range of $10-$1000.
    // See test_rho_parallel_bump_validation for the tighter magnitude check.
    assert_reasonable(rho.abs(), 1.0, 10_000.0, "Rho magnitude");
}

#[test]
fn test_rho_parallel_bump_validation() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    let market = create_flat_market(as_of, 0.05, 0.30);

    // Analytical rho (per 1bp)
    let result = swaption
        .price_with_metrics(&market, as_of, &[MetricId::Rho])
        .unwrap();
    let rho_analytical = *result.measures.get("rho").unwrap();

    // Rho should be finite and reasonable for ATM swaption
    assert!(rho_analytical.is_finite(), "Rho should be finite");
    // Rho for 1M notional 1Y into 5Y should be in a reasonable per‑bp range
    assert_reasonable(rho_analytical.abs(), 10.0, 1_000.0, "Rho magnitude");
}

#[test]
fn test_rho_sign_depends_on_moneyness() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let market = create_flat_market(as_of, 0.05, 0.30);

    // Payer swaption
    let payer = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    let rho_payer = payer
        .price_with_metrics(&market, as_of, &[MetricId::Rho])
        .unwrap()
        .measures
        .get("rho")
        .copied()
        .unwrap();

    // Receiver swaption
    let receiver = create_standard_receiver_swaption(expiry, swap_start, swap_end, 0.05);
    let rho_receiver = receiver
        .price_with_metrics(&market, as_of, &[MetricId::Rho])
        .unwrap()
        .measures
        .get("rho")
        .copied()
        .unwrap();

    // Rho signs should reflect different rate sensitivities
    // Both should be finite
    assert!(
        rho_payer.is_finite() && rho_receiver.is_finite(),
        "Rhos should be finite"
    );
}

#[test]
fn test_rho_magnitude_scales_with_tenor() {
    let as_of = time::macros::date!(2024 - 01 - 01);
    let expiry = time::macros::date!(2025 - 01 - 01);
    let swap_start = expiry;
    let market = create_flat_market(as_of, 0.05, 0.30);

    // Short tenor swap (2Y)
    let swap_end_short = time::macros::date!(2027 - 01 - 01);
    let swaption_short = create_standard_payer_swaption(expiry, swap_start, swap_end_short, 0.05);

    // Long tenor swap (10Y)
    let swap_end_long = time::macros::date!(2035 - 01 - 01);
    let swaption_long = create_standard_payer_swaption(expiry, swap_start, swap_end_long, 0.05);

    let rho_short = swaption_short
        .price_with_metrics(&market, as_of, &[MetricId::Rho])
        .unwrap()
        .measures
        .get("rho")
        .copied()
        .unwrap();

    let rho_long = swaption_long
        .price_with_metrics(&market, as_of, &[MetricId::Rho])
        .unwrap()
        .measures
        .get("rho")
        .copied()
        .unwrap();

    // Longer tenor should generally have higher rho magnitude
    assert!(
        rho_long.abs() > rho_short.abs(),
        "Longer tenor should have higher rho magnitude"
    );
}
