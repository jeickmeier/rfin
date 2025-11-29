//! Implied volatility tests

use crate::swaption::common::*;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::MetricId;

#[test]
fn test_implied_vol_matches_surface() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let vol_input = 0.30;
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    let market = create_flat_market(as_of, 0.05, vol_input);

    let result = swaption
        .price_with_metrics(&market, as_of, &[MetricId::ImpliedVol])
        .unwrap();

    let implied_vol = *result.measures.get("implied_vol").unwrap();

    // Should recover the input vol from surface
    assert_approx_eq(
        implied_vol,
        vol_input,
        0.01,
        "Implied vol should match surface vol",
    );
}

#[test]
fn test_implied_vol_positive() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    let market = create_flat_market(as_of, 0.05, 0.25);

    let result = swaption
        .price_with_metrics(&market, as_of, &[MetricId::ImpliedVol])
        .unwrap();

    let implied_vol = *result.measures.get("implied_vol").unwrap();

    assert!(implied_vol > 0.0, "Implied vol should be positive");
    assert!(
        implied_vol < 5.0,
        "Implied vol should be reasonable (< 500%)"
    );
}

#[test]
fn test_implied_vol_inversion() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    let market = create_flat_market(as_of, 0.05, 0.40);

    // Solve for implied vol given market PV
    // The implied vol calculator solves: price_black(sigma) = target_pv
    // where target_pv is the base_value computed by the same pricing function.
    // Since both use identical pricing paths, inversion should be very precise.
    let result = swaption
        .price_with_metrics(&market, as_of, &[MetricId::ImpliedVol])
        .unwrap();

    let implied_vol = *result.measures.get("implied_vol").unwrap();

    // Should recover input vol precisely (within 1% = 100bp of vol)
    // The solver uses 1e-10 tolerance, so numerical precision is high.
    // Any remaining error comes from forward rate / annuity interpolation.
    assert_approx_eq(implied_vol, 0.40, 0.01, "Implied vol inversion");
}

#[test]
fn test_implied_vol_consistency_across_strikes() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let vol_input = 0.25;
    let market = create_flat_market(as_of, 0.05, vol_input);

    for strike in [0.03, 0.05, 0.07] {
        let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, strike);

        let result = swaption
            .price_with_metrics(&market, as_of, &[MetricId::ImpliedVol])
            .unwrap();

        let implied_vol = *result.measures.get("implied_vol").unwrap();

        // With flat surface, all strikes should recover same vol
        assert_approx_eq(
            implied_vol,
            vol_input,
            0.02,
            &format!("Implied vol at strike {}", strike),
        );
    }
}
