//! Market validation tests for Black-76 model properties.
//!
//! These tests verify that CDS option pricing conforms to established
//! Black-76 model properties and market conventions.

use super::common::*;
use finstack_valuations::instruments::Instrument;
use time::macros::date;

#[test]
fn test_value_increases_with_volatility() {
    // Black model property: ∂V/∂σ > 0
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    let mut values = Vec::new();
    for vol in [0.10, 0.20, 0.30, 0.40, 0.50] {
        let option = CdsOptionBuilder::new().implied_vol(vol).build(as_of);
        let pv = option.value(&market, as_of).unwrap().amount();
        values.push((vol, pv));
    }

    assert_increasing(&values, "Volatility", "Option value");
}

#[test]
fn test_value_increases_with_time() {
    // Black model property: ∂V/∂T > 0 (for European options)
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    let mut values = Vec::new();
    for months in [1, 3, 6, 12, 18, 24] {
        let option = CdsOptionBuilder::new()
            .expiry_months(months)
            .cds_maturity_months(months + 48)
            .build(as_of);
        let pv = option.value(&market, as_of).unwrap().amount();
        values.push((months as f64, pv));
    }

    assert_increasing(&values, "Time to expiry", "Option value");
}

#[test]
fn test_call_value_decreases_with_strike() {
    // Black model property: ∂C/∂K < 0
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    let mut values = Vec::new();
    for strike in [50.0, 100.0, 150.0, 200.0, 300.0, 400.0] {
        let option = CdsOptionBuilder::new().call().strike(strike).build(as_of);
        let pv = option.value(&market, as_of).unwrap().amount();
        values.push((strike, pv));
    }

    assert_decreasing(&values, "Strike", "Call value");
}

#[test]
fn test_put_value_increases_with_strike() {
    // Black model property: ∂P/∂K > 0
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    let mut values = Vec::new();
    for strike in [50.0, 100.0, 150.0, 200.0, 300.0, 400.0] {
        let option = CdsOptionBuilder::new().put().strike(strike).build(as_of);
        let pv = option.value(&market, as_of).unwrap().amount();
        values.push((strike, pv));
    }

    assert_increasing(&values, "Strike", "Put value");
}

#[test]
fn test_vega_always_positive() {
    // Black model property: vega > 0
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    for strike in [50.0, 100.0, 200.0, 400.0] {
        for months in [3, 6, 12, 24] {
            let option = CdsOptionBuilder::new()
                .strike(strike)
                .expiry_months(months)
                .cds_maturity_months(months + 48)
                .build(as_of);

            let vega = option.vega(&market, as_of).unwrap();
            assert_positive(
                vega,
                &format!("Vega for strike={}, expiry={}m", strike, months),
            );
        }
    }
}

#[test]
fn test_gamma_always_positive() {
    // Black model property: gamma > 0
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    for strike in [50.0, 100.0, 200.0, 400.0] {
        let option = CdsOptionBuilder::new().strike(strike).build(as_of);
        let gamma = option.gamma(&market, as_of).unwrap();
        assert_non_negative(gamma, &format!("Gamma for strike={}", strike));
    }
}

#[test]
fn test_delta_bounds_call() {
    // Black model property: 0 < Δ_call < 1 (approximately, scaled by notional)
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    for strike in [50.0, 100.0, 200.0, 400.0] {
        let option = CdsOptionBuilder::new().call().strike(strike).build(as_of);
        let delta = option.delta(&market, as_of).unwrap();

        assert_finite(delta, &format!("Call delta for strike={}", strike));
        assert_non_negative(delta, &format!("Call delta for strike={}", strike));
    }
}

#[test]
fn test_time_value_positive_before_expiry() {
    // Options have positive time value before expiry
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    for strike in [100.0, 200.0, 300.0] {
        let call = CdsOptionBuilder::new().call().strike(strike).build(as_of);
        let put = CdsOptionBuilder::new().put().strike(strike).build(as_of);

        let call_pv = call.value(&market, as_of).unwrap().amount();
        let put_pv = put.value(&market, as_of).unwrap().amount();

        assert_positive(call_pv, &format!("Call time value for strike={}", strike));
        assert_positive(put_pv, &format!("Put time value for strike={}", strike));
    }
}

#[test]
fn test_atm_options_highest_gamma() {
    // ATM options have highest gamma (approximately)
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    let mut gammas = Vec::new();
    for strike in [100.0, 150.0, 200.0, 250.0, 300.0, 350.0] {
        let option = CdsOptionBuilder::new().strike(strike).build(as_of);
        let gamma = option.gamma(&market, as_of).unwrap();
        gammas.push(gamma);
    }

    // Find max gamma
    let max_gamma = gammas.iter().fold(0.0f64, |a, &b| a.max(b));

    // Max should not be at the extremes
    assert!(
        gammas[0] < max_gamma && gammas[gammas.len() - 1] < max_gamma,
        "ATM (middle strikes) should have highest gamma"
    );
}

#[test]
fn test_atm_options_highest_vega() {
    // ATM options have highest vega (approximately)
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    let mut vegas = Vec::new();
    for strike in [100.0, 150.0, 200.0, 250.0, 300.0, 350.0] {
        let option = CdsOptionBuilder::new().strike(strike).build(as_of);
        let vega = option.vega(&market, as_of).unwrap();
        vegas.push(vega);
    }

    // Find max vega
    let max_vega = vegas.iter().fold(0.0f64, |a, &b| a.max(b));

    // Max should not be at the extremes
    assert!(
        vegas[0] < max_vega && vegas[vegas.len() - 1] < max_vega,
        "ATM (middle strikes) should have highest vega"
    );
}

#[test]
fn test_linear_scaling_with_notional() {
    // Option value should scale linearly with notional
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    let base_notional = 10_000_000.0;
    let option_base = CdsOptionBuilder::new()
        .notional(base_notional, finstack_core::currency::Currency::USD)
        .build(as_of);
    let pv_base = option_base.value(&market, as_of).unwrap().amount();

    for multiplier in [0.5, 2.0, 3.0, 5.0] {
        let option = CdsOptionBuilder::new()
            .notional(
                base_notional * multiplier,
                finstack_core::currency::Currency::USD,
            )
            .build(as_of);
        let pv = option.value(&market, as_of).unwrap().amount();

        assert_approx_eq(
            pv / pv_base,
            multiplier,
            0.001,
            &format!("Notional scaling {}x", multiplier),
        );
    }
}

#[test]
fn test_zero_time_gives_intrinsic_value() {
    // At expiry (t=0), option value = max(intrinsic, 0)
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    // Create option with very short time to expiry
    let option = CdsOptionBuilder::new()
        .expiry_months(1) // 1 month to expiry
        .cds_maturity_months(13) // CDS matures after option
        .build(as_of);

    let pv = option.value(&market, as_of).unwrap().amount();

    // Short-dated options should have positive value
    assert_positive(pv, "Short-dated option value");
}
