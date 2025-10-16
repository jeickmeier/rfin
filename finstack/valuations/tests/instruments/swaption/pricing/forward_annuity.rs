//! Forward swap rate and annuity calculation tests

use crate::swaption::common::*;

#[test]
fn test_forward_swap_rate_calculation() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let strike = 0.05;
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, strike);

    let market = create_flat_market(as_of, 0.05, 0.30);
    let disc = market.get_discount_ref("USD_OIS").unwrap();

    let forward = swaption.forward_swap_rate(disc, as_of).unwrap();

    // For flat 5% curve, forward should be close to 5%
    assert_approx_eq(forward, 0.05, 0.001, "Forward swap rate");
}

#[test]
fn test_swap_annuity_positive() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let strike = 0.05;
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, strike);

    let market = create_flat_market(as_of, 0.05, 0.30);
    let disc = market.get_discount_ref("USD_OIS").unwrap();

    let annuity = swaption.swap_annuity(disc, as_of).unwrap();

    // 5Y quarterly swap should have annuity around 4.5-4.8 (20 periods * ~0.24 each)
    assert_reasonable(annuity, 3.0, 6.0, "Swap annuity");
}

#[test]
fn test_annuity_increases_with_tenor() {
    let (as_of, expiry, swap_start, _) = standard_dates();
    let market = create_flat_market(as_of, 0.05, 0.30);
    let disc = market.get_discount_ref("USD_OIS").unwrap();

    // 2Y swap
    let swap_end_2y = time::macros::date!(2027 - 01 - 01);
    let swaption_2y = create_standard_payer_swaption(expiry, swap_start, swap_end_2y, 0.05);
    let annuity_2y = swaption_2y.swap_annuity(disc, as_of).unwrap();

    // 5Y swap
    let swap_end_5y = time::macros::date!(2030 - 01 - 01);
    let swaption_5y = create_standard_payer_swaption(expiry, swap_start, swap_end_5y, 0.05);
    let annuity_5y = swaption_5y.swap_annuity(disc, as_of).unwrap();

    // 10Y swap
    let swap_end_10y = time::macros::date!(2035 - 01 - 01);
    let swaption_10y = create_standard_payer_swaption(expiry, swap_start, swap_end_10y, 0.05);
    let annuity_10y = swaption_10y.swap_annuity(disc, as_of).unwrap();

    assert!(annuity_2y < annuity_5y, "5Y annuity should exceed 2Y");
    assert!(annuity_5y < annuity_10y, "10Y annuity should exceed 5Y");
}

#[test]
fn test_year_fraction_act360() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);

    let yf = swaption
        .year_fraction(as_of, expiry, swaption.day_count)
        .unwrap();

    // 1 year ≈ 1.0 under Act/360
    assert_approx_eq(yf, 1.0, 0.02, "Year fraction for 1Y");
}

#[test]
fn test_forward_rate_consistency() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);

    // Test with different curve levels
    for rate in [0.02, 0.05, 0.08] {
        let market = create_flat_market(as_of, rate, 0.30);
        let disc = market.get_discount_ref("USD_OIS").unwrap();

        let forward = swaption.forward_swap_rate(disc, as_of).unwrap();

        // Forward should be close to flat curve rate
        assert_approx_eq(
            forward,
            rate,
            0.005,
            &format!("Forward rate at {}%", rate * 100.0),
        );
    }
}
