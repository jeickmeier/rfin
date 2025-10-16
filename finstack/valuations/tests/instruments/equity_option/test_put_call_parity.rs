//! Tests for put-call parity relationships.
//!
//! Put-call parity for European options:
//! C - P = S*e^(-qT) - K*e^(-rT)
//!
//! Where:
//! - C = call value
//! - P = put value
//! - S = spot price
//! - K = strike
//! - r = risk-free rate
//! - q = dividend yield
//! - T = time to expiry

use super::helpers::*;
use finstack_valuations::instruments::common::traits::Instrument;
use time::macros::date;

#[test]
fn test_put_call_parity_atm() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let strike = 100.0;
    let spot = 100.0;
    let rate = 0.05;
    let div_yield = 0.02;

    let call = create_call(as_of, expiry, strike);
    let put = create_put(as_of, expiry, strike);

    let market = build_standard_market(as_of, spot, 0.25, rate, div_yield);

    let call_pv = call.value(&market, as_of).unwrap().amount();
    let put_pv = put.value(&market, as_of).unwrap().amount();

    // C - P = (S*e^(-qT) - K*e^(-rT)) * contract_size
    let t = 1.0_f64; // 1 year
    let forward_spot = spot * (-div_yield * t).exp() as f64;
    let pv_strike = strike * (-rate * t).exp() as f64;
    let expected_diff = (forward_spot - pv_strike) * call.contract_size;

    let actual_diff = call_pv - put_pv;

    assert_approx_eq_tol(
        actual_diff,
        expected_diff,
        10.0, // Allow $10 tolerance for numerical precision
        "Put-call parity ATM",
    );
}

#[test]
fn test_put_call_parity_itm_call() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let strike = 90.0;
    let spot = 100.0;
    let rate = 0.05;
    let div_yield = 0.0;

    let call = create_call(as_of, expiry, strike);
    let put = create_put(as_of, expiry, strike);

    let market = build_standard_market(as_of, spot, 0.25, rate, div_yield);

    let call_pv = call.value(&market, as_of).unwrap().amount();
    let put_pv = put.value(&market, as_of).unwrap().amount();

    let t = 1.0_f64;
    let forward_spot = spot * (-div_yield * t).exp() as f64;
    let pv_strike = strike * (-rate * t).exp() as f64;
    let expected_diff = (forward_spot - pv_strike) * call.contract_size;

    let actual_diff = call_pv - put_pv;

    assert_approx_eq_tol(actual_diff, expected_diff, 10.0, "Put-call parity ITM call");
}

#[test]
fn test_put_call_parity_otm_call() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let strike = 110.0;
    let spot = 100.0;
    let rate = 0.05;
    let div_yield = 0.0;

    let call = create_call(as_of, expiry, strike);
    let put = create_put(as_of, expiry, strike);

    let market = build_standard_market(as_of, spot, 0.25, rate, div_yield);

    let call_pv = call.value(&market, as_of).unwrap().amount();
    let put_pv = put.value(&market, as_of).unwrap().amount();

    let t = 1.0_f64;
    let forward_spot = spot * (-div_yield * t).exp() as f64;
    let pv_strike = strike * (-rate * t).exp() as f64;
    let expected_diff = (forward_spot - pv_strike) * call.contract_size;

    let actual_diff = call_pv - put_pv;

    assert_approx_eq_tol(actual_diff, expected_diff, 10.0, "Put-call parity OTM call");
}

#[test]
fn test_put_call_parity_short_dated() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2024 - 04 - 01); // 3 months
    let strike = 100.0;
    let spot = 100.0;
    let rate = 0.05;
    let div_yield = 0.02;

    let call = create_call(as_of, expiry, strike);
    let put = create_put(as_of, expiry, strike);

    let market = build_standard_market(as_of, spot, 0.25, rate, div_yield);

    let call_pv = call.value(&market, as_of).unwrap().amount();
    let put_pv = put.value(&market, as_of).unwrap().amount();

    let t = 0.25; // 3 months
    let forward_spot = spot * (-div_yield * t).exp() as f64;
    let pv_strike = strike * (-rate * t).exp() as f64;
    let expected_diff = (forward_spot - pv_strike) * call.contract_size;

    let actual_diff = call_pv - put_pv;

    assert_approx_eq_tol(
        actual_diff,
        expected_diff,
        5.0,
        "Put-call parity short dated",
    );
}

#[test]
fn test_put_call_parity_long_dated() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2026 - 01 - 01); // 2 years
    let strike = 100.0;
    let spot = 100.0;
    let rate = 0.05;
    let div_yield = 0.02;

    let call = create_call(as_of, expiry, strike);
    let put = create_put(as_of, expiry, strike);

    let market = build_standard_market(as_of, spot, 0.25, rate, div_yield);

    let call_pv = call.value(&market, as_of).unwrap().amount();
    let put_pv = put.value(&market, as_of).unwrap().amount();

    let t = 2.0; // 2 years
    let forward_spot = spot * (-div_yield * t).exp() as f64;
    let pv_strike = strike * (-rate * t).exp() as f64;
    let expected_diff = (forward_spot - pv_strike) * call.contract_size;

    let actual_diff = call_pv - put_pv;

    assert_approx_eq_tol(
        actual_diff,
        expected_diff,
        20.0, // Larger tolerance for longer dates
        "Put-call parity long dated",
    );
}

#[test]
fn test_put_call_parity_with_high_dividends() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let strike = 100.0;
    let spot = 100.0;
    let rate = 0.05;
    let div_yield = 0.08; // High dividend

    let call = create_call(as_of, expiry, strike);
    let put = create_put(as_of, expiry, strike);

    let market = build_standard_market(as_of, spot, 0.25, rate, div_yield);

    let call_pv = call.value(&market, as_of).unwrap().amount();
    let put_pv = put.value(&market, as_of).unwrap().amount();

    let t = 1.0_f64;
    let forward_spot = spot * (-div_yield * t).exp() as f64;
    let pv_strike = strike * (-rate * t).exp() as f64;
    let expected_diff = (forward_spot - pv_strike) * call.contract_size;

    let actual_diff = call_pv - put_pv;

    assert_approx_eq_tol(
        actual_diff,
        expected_diff,
        15.0,
        "Put-call parity with high dividends",
    );
}

#[test]
fn test_put_call_parity_zero_rates() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let strike = 100.0;
    let spot = 100.0;
    let rate = 0.0;
    let div_yield = 0.0;

    let call = create_call(as_of, expiry, strike);
    let put = create_put(as_of, expiry, strike);

    let market = build_standard_market(as_of, spot, 0.25, rate, div_yield);

    let call_pv = call.value(&market, as_of).unwrap().amount();
    let put_pv = put.value(&market, as_of).unwrap().amount();

    // With zero rates and dividends: C - P = (S - K) * contract_size
    let expected_diff = (spot - strike) * call.contract_size;

    let actual_diff = call_pv - put_pv;

    assert_approx_eq_tol(
        actual_diff,
        expected_diff,
        5.0,
        "Put-call parity zero rates",
    );
}

#[test]
fn test_put_call_parity_across_volatilities() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let strike = 100.0;
    let spot = 100.0;
    let rate = 0.05;
    let div_yield = 0.02;

    let call = create_call(as_of, expiry, strike);
    let put = create_put(as_of, expiry, strike);

    let t = 1.0_f64;
    let forward_spot = spot * (-div_yield * t).exp() as f64;
    let pv_strike = strike * (-rate * t).exp() as f64;
    let expected_diff = (forward_spot - pv_strike) * call.contract_size;

    // Test across different volatilities
    for vol in [0.10, 0.20, 0.30, 0.50, 0.80] {
        let market = build_standard_market(as_of, spot, vol, rate, div_yield);

        let call_pv = call.value(&market, as_of).unwrap().amount();
        let put_pv = put.value(&market, as_of).unwrap().amount();

        let actual_diff = call_pv - put_pv;

        assert_approx_eq_tol(
            actual_diff,
            expected_diff,
            10.0,
            &format!("Put-call parity at vol={}", vol),
        );
    }
}

#[test]
fn test_put_call_parity_across_spot_levels() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let strike = 100.0;
    let rate = 0.05;
    let div_yield = 0.02;

    let call = create_call(as_of, expiry, strike);
    let put = create_put(as_of, expiry, strike);

    let t = 1.0;

    // Test across different spot levels
    for spot in [80.0, 90.0, 100.0, 110.0, 120.0] {
        let market = build_standard_market(as_of, spot, 0.25, rate, div_yield);

        let call_pv = call.value(&market, as_of).unwrap().amount();
        let put_pv = put.value(&market, as_of).unwrap().amount();

        let forward_spot = spot * (-div_yield * t).exp() as f64;
        let pv_strike = strike * (-rate * t).exp() as f64;
        let expected_diff = (forward_spot - pv_strike) * call.contract_size;

        let actual_diff = call_pv - put_pv;

        assert_approx_eq_tol(
            actual_diff,
            expected_diff,
            10.0,
            &format!("Put-call parity at spot={}", spot),
        );
    }
}
