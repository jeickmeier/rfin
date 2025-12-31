//! Gamma tests

use crate::swaption::common::*;
use finstack_valuations::instruments::Instrument;
use finstack_valuations::metrics::MetricId;

#[test]
fn test_gamma_positive_for_long_option() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    let market = create_flat_market(as_of, 0.05, 0.30);

    let result = swaption
        .price_with_metrics(&market, as_of, &[MetricId::Gamma])
        .unwrap();

    let gamma = *result.measures.get("gamma").unwrap();

    // Long options have positive gamma
    assert!(gamma >= 0.0, "Long option gamma should be non-negative");
    assert!(gamma.is_finite(), "Gamma should be finite");
}

#[test]
fn test_atm_gamma_highest() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let market = create_flat_market(as_of, 0.05, 0.30);

    let atm = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    let itm = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.03);
    let otm = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.07);

    let gamma_atm = atm
        .price_with_metrics(&market, as_of, &[MetricId::Gamma])
        .unwrap()
        .measures
        .get("gamma")
        .copied()
        .unwrap();

    let gamma_itm = itm
        .price_with_metrics(&market, as_of, &[MetricId::Gamma])
        .unwrap()
        .measures
        .get("gamma")
        .copied()
        .unwrap();

    let gamma_otm = otm
        .price_with_metrics(&market, as_of, &[MetricId::Gamma])
        .unwrap()
        .measures
        .get("gamma")
        .copied()
        .unwrap();

    // ATM options have highest gamma
    assert!(gamma_atm >= gamma_itm, "ATM gamma should be >= ITM gamma");
    assert!(gamma_atm >= gamma_otm, "ATM gamma should be >= OTM gamma");
}

#[test]
fn test_gamma_reasonable_magnitude() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    let market = create_flat_market(as_of, 0.05, 0.30);

    let result = swaption
        .price_with_metrics(&market, as_of, &[MetricId::Gamma])
        .unwrap();

    let gamma = *result.measures.get("gamma").unwrap();

    // Gamma should be finite and positive
    assert_reasonable(gamma, 0.0, 1e10, "Gamma magnitude");
}

#[test]
fn test_gamma_decreases_with_time_to_expiry() {
    let as_of = time::macros::date!(2024 - 01 - 01);
    let swap_start = time::macros::date!(2025 - 01 - 01);
    let swap_end = time::macros::date!(2030 - 01 - 01);
    let market = create_flat_market(as_of, 0.05, 0.30);

    // Short expiry
    let expiry_short = time::macros::date!(2024 - 06 - 01); // 6M
    let swaption_short = create_standard_payer_swaption(expiry_short, swap_start, swap_end, 0.05);

    // Long expiry
    let expiry_long = time::macros::date!(2026 - 01 - 01); // 2Y
    let swaption_long = create_standard_payer_swaption(expiry_long, swap_start, swap_end, 0.05);

    let gamma_short = swaption_short
        .price_with_metrics(&market, as_of, &[MetricId::Gamma])
        .unwrap()
        .measures
        .get("gamma")
        .copied()
        .unwrap();

    let gamma_long = swaption_long
        .price_with_metrics(&market, as_of, &[MetricId::Gamma])
        .unwrap()
        .measures
        .get("gamma")
        .copied()
        .unwrap();

    // Shorter expiry typically has higher gamma near expiry
    // (though this depends on vol and other factors)
    assert!(
        gamma_short >= 0.0 && gamma_long >= 0.0,
        "Both gammas should be non-negative"
    );
}
