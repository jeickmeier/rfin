//! Payer vs Receiver swaption tests

use crate::swaption::common::*;
use finstack_valuations::instruments::internal::InstrumentExt as Instrument;
use finstack_valuations::metrics::MetricId;

#[test]
fn test_payer_receiver_symmetry() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let market = create_flat_market(as_of, 0.05, 0.30);
    let forward = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05)
        .forward_swap_rate(&market, as_of)
        .unwrap();

    let payer = create_standard_payer_swaption(expiry, swap_start, swap_end, forward);
    let receiver = create_standard_receiver_swaption(expiry, swap_start, swap_end, forward);

    let pv_payer = payer.value(&market, as_of).unwrap().amount();
    let pv_receiver = receiver.value(&market, as_of).unwrap().amount();

    // At ATM (strike = forward), payer and receiver should have similar values
    assert_approx_eq(pv_payer, pv_receiver, 0.05, "ATM payer-receiver symmetry");
}

#[test]
fn test_payer_benefits_from_rate_increase() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let strike = 0.05;

    // Low rate environment
    let market_low = create_flat_market(as_of, 0.03, 0.30);
    // High rate environment
    let market_high = create_flat_market(as_of, 0.07, 0.30);

    let payer = create_standard_payer_swaption(expiry, swap_start, swap_end, strike);

    let pv_low = payer.value(&market_low, as_of).unwrap().amount();
    let pv_high = payer.value(&market_high, as_of).unwrap().amount();

    // Payer swaption is more valuable when rates are higher
    assert!(pv_high > pv_low, "Payer should benefit from rate increase");
}

#[test]
fn test_receiver_benefits_from_rate_decrease() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let strike = 0.05;

    // Low rate environment
    let market_low = create_flat_market(as_of, 0.03, 0.30);
    // High rate environment
    let market_high = create_flat_market(as_of, 0.07, 0.30);

    let receiver = create_standard_receiver_swaption(expiry, swap_start, swap_end, strike);

    let pv_low = receiver.value(&market_low, as_of).unwrap().amount();
    let pv_high = receiver.value(&market_high, as_of).unwrap().amount();

    // Receiver swaption is more valuable when rates are lower
    assert!(
        pv_low > pv_high,
        "Receiver should benefit from rate decrease"
    );
}

#[test]
fn test_delta_signs_opposite() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let strike = 0.05;
    let market = create_flat_market(as_of, 0.05, 0.30);

    let payer = create_standard_payer_swaption(expiry, swap_start, swap_end, strike);
    let receiver = create_standard_receiver_swaption(expiry, swap_start, swap_end, strike);

    let delta_payer = payer
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Delta],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .unwrap()
        .measures
        .get("delta")
        .copied()
        .unwrap();

    let delta_receiver = receiver
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Delta],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .unwrap()
        .measures
        .get("delta")
        .copied()
        .unwrap();

    // Deltas should have opposite signs
    assert!(delta_payer > 0.0, "Payer delta should be positive");
    assert!(delta_receiver < 0.0, "Receiver delta should be negative");
}

#[test]
fn test_vega_same_sign() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let strike = 0.05;
    let market = create_flat_market(as_of, 0.05, 0.30);

    let payer = create_standard_payer_swaption(expiry, swap_start, swap_end, strike);
    let receiver = create_standard_receiver_swaption(expiry, swap_start, swap_end, strike);

    let vega_payer = payer
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Vega],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .unwrap()
        .measures
        .get("vega")
        .copied()
        .unwrap();

    let vega_receiver = receiver
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Vega],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .unwrap()
        .measures
        .get("vega")
        .copied()
        .unwrap();

    // Both long options → both have positive vega
    assert!(vega_payer > 0.0, "Payer vega should be positive");
    assert!(vega_receiver > 0.0, "Receiver vega should be positive");

    // ATM vegas should be similar
    assert_approx_eq(
        vega_payer,
        vega_receiver,
        0.10,
        "ATM vegas should be similar",
    );
}

#[test]
fn test_gamma_same_sign() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let strike = 0.05;
    let market = create_flat_market(as_of, 0.05, 0.30);

    let payer = create_standard_payer_swaption(expiry, swap_start, swap_end, strike);
    let receiver = create_standard_receiver_swaption(expiry, swap_start, swap_end, strike);

    let gamma_payer = payer
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Gamma],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .unwrap()
        .measures
        .get("gamma")
        .copied()
        .unwrap();

    let gamma_receiver = receiver
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Gamma],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .unwrap()
        .measures
        .get("gamma")
        .copied()
        .unwrap();

    // Both long options → both have positive gamma
    assert!(gamma_payer >= 0.0, "Payer gamma should be non-negative");
    assert!(
        gamma_receiver >= 0.0,
        "Receiver gamma should be non-negative"
    );
}
