#![cfg(feature = "slow")]
//! QuantLib parity tests for CDS Option pricing and Greeks.
//!
//! These tests validate that our CDS Option implementation follows the same
//! mathematical properties and conventions as QuantLib for options on CDS spreads.
//!
//! Reference: QuantLib test-suite/cdsoption.cpp
//!
//! ## QuantLib Property-Based Tests Covered:
//!
//! 1. **Black-76 Model Properties** (testBlack76):
//!    - Option value > 0 before expiry
//!    - ATM call ≈ ATM put at forward strike
//!    - Call value decreases with strike
//!    - Put value increases with strike
//!
//! 2. **Greeks Properties** (testGreeks):
//!    - Delta: call > 0, put < 0, bounded
//!    - Gamma: always > 0, peaks at ATM
//!    - Vega: always > 0, peaks at ATM
//!    - Theta: finite, typically negative for long positions
//!    - Rho: finite, reflects rate sensitivity
//!
//! 3. **Implied Volatility** (testImpliedVolatility):
//!    - Perfect round-trip: price -> IV -> price
//!    - Convergence from different initial guesses
//!    - Stability across moneyness (ITM/ATM/OTM)
//!
//! 4. **Forward Spread** (testForwardSpread):
//!    - Consistency with CDS par spread calculation
//!    - At-the-forward call/put parity
//!
//! 5. **Index Options** (testIndexOptions):
//!    - Linear scaling with index factor
//!    - Forward spread adjustment impact
//!
//! 6. **No-Arbitrage Bounds**:
//!    - Butterfly spread convexity
//!    - Digital spread bounds
//!    - Value bounds (0 <= V <= reasonable max)
//!
//! ## Implementation Notes:
//!
//! - Property-based rather than exact value comparison (no direct QuantLib integration)
//! - Uses f64 with documented tolerances; QuantLib uses double
//! - Follows Black-on-spreads convention (same as QuantLib)
//! - Implements ISDA-standard risky annuity calculation

use super::common::*;
use crate::finstack_test_utils as test_utils;
use finstack_core::currency::Currency;
use finstack_valuations::instruments::Instrument;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

// ============================================================================
// Test 1: Black-76 Model Properties (QuantLib testBlack76)
// ============================================================================

#[test]
fn test_quantlib_black76_positive_value() {
    // QuantLib property: options have positive value before expiry
    let as_of = date!(2024 - 12 - 20);
    let market = standard_market(as_of);

    for strike in [100.0, 200.0, 300.0] {
        let call = CDSOptionBuilder::new()
            .call()
            .strike(strike)
            .expiry_months(12)
            .cds_maturity_months(60)
            .implied_vol(0.30)
            .build(as_of);

        let put = CDSOptionBuilder::new()
            .put()
            .strike(strike)
            .expiry_months(12)
            .cds_maturity_months(60)
            .implied_vol(0.30)
            .build(as_of);

        assert_positive(
            call.value(&market, as_of).unwrap().amount(),
            &format!("Call value at strike {}", strike),
        );
        assert_positive(
            put.value(&market, as_of).unwrap().amount(),
            &format!("Put value at strike {}", strike),
        );
    }
}

#[test]
fn test_quantlib_black76_atf_call_put_parity() {
    // QuantLib property: at-the-forward, call ≈ put
    let as_of = date!(2024 - 12 - 20);
    let market = standard_market(as_of);

    // Get forward spread
    let temp_option = CDSOptionBuilder::new().build(as_of);
    let mut underlying = test_utils::cds_buy_protection(
        "CDS-FWD-TEMP",
        temp_option.notional,
        temp_option.strike_spread_bp,
        temp_option.expiry,
        temp_option.cds_maturity,
        temp_option.discount_curve_id.clone(),
        temp_option.credit_curve_id.clone(),
    )
    .expect("underlying CDS should build");
    underlying.protection.recovery_rate = temp_option.recovery_rate;
    let forward = underlying
        .price_with_metrics(&market, as_of, &[MetricId::ParSpread])
        .expect("par spread should compute")
        .measures[&MetricId::ParSpread];

    // Option struck at forward
    let call = CDSOptionBuilder::new()
        .call()
        .strike(forward)
        .expiry_months(12)
        .cds_maturity_months(60)
        .implied_vol(0.30)
        .build(as_of);

    let put = CDSOptionBuilder::new()
        .put()
        .strike(forward)
        .expiry_months(12)
        .cds_maturity_months(60)
        .implied_vol(0.30)
        .build(as_of);

    let call_pv = call.value(&market, as_of).unwrap().amount();
    let put_pv = put.value(&market, as_of).unwrap().amount();

    // QuantLib: ATF call should approximately equal ATF put
    let rel_diff = (call_pv - put_pv).abs() / call_pv.max(put_pv);
    assert!(
        rel_diff < 0.05,
        "ATF call ({}) should ≈ ATF put ({}), rel_diff={}",
        call_pv,
        put_pv,
        rel_diff
    );
}

#[test]
fn test_quantlib_black76_strike_monotonicity() {
    // QuantLib property: ∂C/∂K < 0, ∂P/∂K > 0
    let as_of = date!(2024 - 12 - 20);
    let market = standard_market(as_of);

    let strikes = [100.0, 150.0, 200.0, 250.0, 300.0];

    // Call values should decrease with strike
    let mut prev_call = f64::INFINITY;
    for &strike in &strikes {
        let call = CDSOptionBuilder::new()
            .call()
            .strike(strike)
            .expiry_months(12)
            .cds_maturity_months(60)
            .implied_vol(0.30)
            .build(as_of);

        let pv = call.value(&market, as_of).unwrap().amount();
        assert!(pv < prev_call, "Call value should decrease with strike");
        prev_call = pv;
    }

    // Put values should increase with strike
    let mut prev_put = 0.0;
    for &strike in &strikes {
        let put = CDSOptionBuilder::new()
            .put()
            .strike(strike)
            .expiry_months(12)
            .cds_maturity_months(60)
            .implied_vol(0.30)
            .build(as_of);

        let pv = put.value(&market, as_of).unwrap().amount();
        assert!(pv > prev_put, "Put value should increase with strike");
        prev_put = pv;
    }
}

// ============================================================================
// Test 2: Greeks Properties (QuantLib testGreeks)
// ============================================================================

#[test]
fn test_quantlib_greeks_delta_signs() {
    // QuantLib property: call delta > 0, put delta < 0
    let as_of = date!(2024 - 12 - 20);
    let market = standard_market(as_of);

    let call = CDSOptionBuilder::new()
        .call()
        .strike(200.0)
        .expiry_months(12)
        .cds_maturity_months(60)
        .implied_vol(0.30)
        .build(as_of);

    let put = CDSOptionBuilder::new()
        .put()
        .strike(200.0)
        .expiry_months(12)
        .cds_maturity_months(60)
        .implied_vol(0.30)
        .build(as_of);

    let call_delta = call.delta(&market, as_of).unwrap();
    let put_delta = put.delta(&market, as_of).unwrap();

    assert!(call_delta > 0.0, "Call delta should be positive");
    assert!(put_delta < 0.0, "Put delta should be negative");
}

#[test]
fn test_quantlib_greeks_gamma_positive() {
    // QuantLib property: gamma > 0 for all options
    let as_of = date!(2024 - 12 - 20);
    let market = standard_market(as_of);

    for strike in [100.0, 200.0, 300.0] {
        for option_type in [
            CDSOptionBuilder::new().call(),
            CDSOptionBuilder::new().put(),
        ] {
            let option = option_type
                .strike(strike)
                .expiry_months(12)
                .cds_maturity_months(60)
                .implied_vol(0.30)
                .build(as_of);

            let gamma = option.gamma(&market, as_of).unwrap();
            assert_non_negative(gamma, &format!("Gamma at strike {}", strike));
        }
    }
}

#[test]
fn test_quantlib_greeks_vega_positive() {
    // QuantLib property: vega > 0 for all options
    let as_of = date!(2024 - 12 - 20);
    let market = standard_market(as_of);

    for strike in [100.0, 200.0, 300.0] {
        let option = CDSOptionBuilder::new()
            .call()
            .strike(strike)
            .expiry_months(12)
            .cds_maturity_months(60)
            .implied_vol(0.30)
            .build(as_of);

        let vega = option.vega(&market, as_of).unwrap();
        assert_positive(vega, &format!("Vega at strike {}", strike));
    }
}

#[test]
fn test_quantlib_greeks_gamma_vega_peak_atm() {
    // QuantLib property: gamma and vega peak at ATM
    let as_of = date!(2024 - 12 - 20);
    let market = standard_market(as_of);

    let strikes = [100.0, 150.0, 200.0, 250.0, 300.0];
    let mut gammas = Vec::new();
    let mut vegas = Vec::new();

    for &strike in &strikes {
        let option = CDSOptionBuilder::new()
            .call()
            .strike(strike)
            .expiry_months(12)
            .cds_maturity_months(60)
            .implied_vol(0.30)
            .build(as_of);

        gammas.push(option.gamma(&market, as_of).unwrap());
        vegas.push(option.vega(&market, as_of).unwrap());
    }

    // Find max positions
    let max_gamma_idx = gammas
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .unwrap()
        .0;
    let max_vega_idx = vegas
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .unwrap()
        .0;

    // Max should not be at extremes (ITM or OTM), but near middle (ATM)
    assert!(
        max_gamma_idx > 0 && max_gamma_idx < strikes.len() - 1,
        "Gamma should peak near ATM, not at extremes"
    );
    assert!(
        max_vega_idx > 0 && max_vega_idx < strikes.len() - 1,
        "Vega should peak near ATM, not at extremes"
    );
}

#[test]
fn test_quantlib_greeks_finite() {
    // QuantLib property: all greeks are finite
    let as_of = date!(2024 - 12 - 20);
    let market = standard_market(as_of);

    let option = CDSOptionBuilder::new()
        .call()
        .strike(200.0)
        .expiry_months(12)
        .cds_maturity_months(60)
        .implied_vol(0.30)
        .build(as_of);

    let result = option
        .price_with_metrics(
            &market,
            as_of,
            &[
                MetricId::Delta,
                MetricId::Gamma,
                MetricId::Vega,
                MetricId::Theta,
                MetricId::Rho,
            ],
        )
        .unwrap();

    for (name, value) in &result.measures {
        assert_finite(*value, &format!("Greek: {}", name));
    }
}

// ============================================================================
// Test 3: Implied Volatility (QuantLib testImpliedVolatility)
// ============================================================================

#[test]
fn test_quantlib_iv_round_trip_atm() {
    // QuantLib property: perfect IV round-trip at ATM
    let as_of = date!(2024 - 12 - 20);
    let market = standard_market(as_of);
    let target_vol = 0.30;

    let option = CDSOptionBuilder::new()
        .call()
        .strike(200.0)
        .expiry_months(12)
        .cds_maturity_months(60)
        .implied_vol(target_vol)
        .build(as_of);

    let pv = option.value(&market, as_of).unwrap().amount();

    let mut option_solve = option.clone();
    option_solve.pricing_overrides.implied_volatility = None;
    let solved_iv = option_solve.implied_vol(&market, as_of, pv, None).unwrap();

    assert_approx_eq(solved_iv, target_vol, 1e-6, "IV round-trip ATM");
}

#[test]
fn test_quantlib_iv_round_trip_moneyness() {
    // QuantLib property: IV round-trip across moneyness spectrum
    let as_of = date!(2024 - 12 - 20);
    let market = standard_market(as_of);
    let target_vol = 0.35;

    for strike in [100.0, 150.0, 200.0, 250.0, 300.0] {
        let option = CDSOptionBuilder::new()
            .call()
            .strike(strike)
            .expiry_months(12)
            .cds_maturity_months(60)
            .implied_vol(target_vol)
            .build(as_of);

        let pv = option.value(&market, as_of).unwrap().amount();

        let mut option_solve = option.clone();
        option_solve.pricing_overrides.implied_volatility = None;
        let solved_iv = option_solve.implied_vol(&market, as_of, pv, None).unwrap();

        let tolerance = if !(150.0..=250.0).contains(&strike) {
            1e-4 // Looser for deep ITM/OTM
        } else {
            1e-6 // Tight for near-ATM
        };

        assert_approx_eq(
            solved_iv,
            target_vol,
            tolerance,
            &format!("IV round-trip strike {}", strike),
        );
    }
}

#[test]
fn test_quantlib_iv_convergence_from_different_guesses() {
    // QuantLib property: IV solver converges to same answer from different initial guesses
    let as_of = date!(2024 - 12 - 20);
    let market = standard_market(as_of);
    let true_vol = 0.28;

    let option = CDSOptionBuilder::new()
        .call()
        .strike(200.0)
        .expiry_months(12)
        .cds_maturity_months(60)
        .implied_vol(true_vol)
        .build(as_of);

    let pv = option.value(&market, as_of).unwrap().amount();

    let mut results = Vec::new();
    for guess in [0.10, 0.25, 0.50, 0.75] {
        let mut option_solve = option.clone();
        option_solve.pricing_overrides.implied_volatility = None;
        let solved = option_solve
            .implied_vol(&market, as_of, pv, Some(guess))
            .unwrap();
        results.push(solved);
    }

    // All results should be close to true_vol
    for (i, &iv) in results.iter().enumerate() {
        assert_approx_eq(
            iv,
            true_vol,
            1e-6,
            &format!("IV convergence iteration {}", i),
        );
    }
}

// ============================================================================
// Test 4: Forward Spread (QuantLib testForwardSpread)
// ============================================================================

#[test]
fn test_quantlib_forward_spread_positive() {
    // QuantLib property: forward spread should be positive for normal credits
    let as_of = date!(2024 - 12 - 20);
    let market = standard_market(as_of);

    let option = CDSOptionBuilder::new().build(as_of);
    let mut underlying = test_utils::cds_buy_protection(
        "CDS-FWD",
        option.notional,
        option.strike_spread_bp,
        option.expiry,
        option.cds_maturity,
        option.discount_curve_id.clone(),
        option.credit_curve_id.clone(),
    )
    .expect("underlying CDS should build");
    underlying.protection.recovery_rate = option.recovery_rate;
    let forward = underlying
        .price_with_metrics(&market, as_of, &[MetricId::ParSpread])
        .expect("par spread should compute")
        .measures[&MetricId::ParSpread];

    assert_positive(forward, "Forward spread");
    assert_in_range(forward, 50.0, 500.0, "Forward spread reasonableness");
}

#[test]
fn test_quantlib_forward_spread_atf_parity() {
    // QuantLib property: ATF call ≈ ATF put (forward parity)
    let as_of = date!(2024 - 12 - 20);
    let market = standard_market(as_of);

    let temp = CDSOptionBuilder::new().build(as_of);
    let mut underlying = test_utils::cds_buy_protection(
        "CDS-FWD-TEMP",
        temp.notional,
        temp.strike_spread_bp,
        temp.expiry,
        temp.cds_maturity,
        temp.discount_curve_id.clone(),
        temp.credit_curve_id.clone(),
    )
    .expect("underlying CDS should build");
    underlying.protection.recovery_rate = temp.recovery_rate;
    let forward = underlying
        .price_with_metrics(&market, as_of, &[MetricId::ParSpread])
        .expect("par spread should compute")
        .measures[&MetricId::ParSpread];

    let call = CDSOptionBuilder::new()
        .call()
        .strike(forward)
        .expiry_months(12)
        .cds_maturity_months(60)
        .implied_vol(0.30)
        .build(as_of);

    let put = CDSOptionBuilder::new()
        .put()
        .strike(forward)
        .expiry_months(12)
        .cds_maturity_months(60)
        .implied_vol(0.30)
        .build(as_of);

    let call_pv = call.value(&market, as_of).unwrap().amount();
    let put_pv = put.value(&market, as_of).unwrap().amount();

    assert_approx_eq(
        call_pv,
        put_pv,
        0.05,
        "ATF call-put parity (at forward spread)",
    );
}

// ============================================================================
// Test 5: Index Options (QuantLib testIndexOptions)
// ============================================================================

#[test]
fn test_quantlib_index_factor_linear_scaling() {
    // QuantLib property: PV scales linearly with index factor
    let as_of = date!(2024 - 12 - 20);
    let market = standard_market(as_of);

    let base = CDSOptionBuilder::new()
        .call()
        .strike(200.0)
        .expiry_months(12)
        .cds_maturity_months(60)
        .implied_vol(0.30)
        .with_index(1.0)
        .build(as_of);

    let base_pv = base.value(&market, as_of).unwrap().amount();

    for factor in [0.75, 0.85, 0.95] {
        let scaled = CDSOptionBuilder::new()
            .call()
            .strike(200.0)
            .expiry_months(12)
            .cds_maturity_months(60)
            .implied_vol(0.30)
            .with_index(factor)
            .build(as_of);

        let scaled_pv = scaled.value(&market, as_of).unwrap().amount();
        let expected = base_pv * factor;

        assert_approx_eq(
            scaled_pv,
            expected,
            0.001,
            &format!("Index factor scaling {}", factor),
        );
    }
}

#[test]
fn test_quantlib_index_forward_adjustment_direction() {
    // QuantLib property: positive adjustment increases call value, decreases put value
    let as_of = date!(2024 - 12 - 20);
    let market = standard_market(as_of);

    // Call: positive adjustment should increase value
    let call_base = CDSOptionBuilder::new()
        .call()
        .strike(200.0)
        .expiry_months(12)
        .cds_maturity_months(60)
        .implied_vol(0.30)
        .with_index(1.0)
        .forward_adjust(0.0)
        .build(as_of);

    let call_adj = CDSOptionBuilder::new()
        .call()
        .strike(200.0)
        .expiry_months(12)
        .cds_maturity_months(60)
        .implied_vol(0.30)
        .with_index(1.0)
        .forward_adjust(25.0)
        .build(as_of);

    let call_base_pv = call_base.value(&market, as_of).unwrap().amount();
    let call_adj_pv = call_adj.value(&market, as_of).unwrap().amount();

    assert!(
        call_adj_pv > call_base_pv,
        "Positive adjustment should increase call value"
    );

    // Put: positive adjustment should decrease value
    let put_base = CDSOptionBuilder::new()
        .put()
        .strike(200.0)
        .expiry_months(12)
        .cds_maturity_months(60)
        .implied_vol(0.30)
        .with_index(1.0)
        .forward_adjust(0.0)
        .build(as_of);

    let put_adj = CDSOptionBuilder::new()
        .put()
        .strike(200.0)
        .expiry_months(12)
        .cds_maturity_months(60)
        .implied_vol(0.30)
        .with_index(1.0)
        .forward_adjust(25.0)
        .build(as_of);

    let put_base_pv = put_base.value(&market, as_of).unwrap().amount();
    let put_adj_pv = put_adj.value(&market, as_of).unwrap().amount();

    assert!(
        put_adj_pv < put_base_pv,
        "Positive adjustment should decrease put value"
    );
}

// ============================================================================
// Test 6: No-Arbitrage Bounds (QuantLib convexity tests)
// ============================================================================

#[test]
fn test_quantlib_butterfly_no_arbitrage() {
    // QuantLib property: C(K1) + C(K3) >= 2*C(K2) for K1 < K2 < K3
    let as_of = date!(2024 - 12 - 20);
    let market = standard_market(as_of);

    let k1 = 100.0;
    let k2 = 200.0;
    let k3 = 300.0;

    let c1 = CDSOptionBuilder::new()
        .call()
        .strike(k1)
        .expiry_months(12)
        .cds_maturity_months(60)
        .implied_vol(0.30)
        .build(as_of)
        .value(&market, as_of)
        .unwrap()
        .amount();

    let c2 = CDSOptionBuilder::new()
        .call()
        .strike(k2)
        .expiry_months(12)
        .cds_maturity_months(60)
        .implied_vol(0.30)
        .build(as_of)
        .value(&market, as_of)
        .unwrap()
        .amount();

    let c3 = CDSOptionBuilder::new()
        .call()
        .strike(k3)
        .expiry_months(12)
        .cds_maturity_months(60)
        .implied_vol(0.30)
        .build(as_of)
        .value(&market, as_of)
        .unwrap()
        .amount();

    assert!(
        c1 + c3 >= 2.0 * c2 - 1e-6,
        "Butterfly no-arbitrage: C({}) + C({}) = {} < 2*C({}) = {}",
        k1,
        k3,
        c1 + c3,
        k2,
        2.0 * c2
    );
}

#[test]
fn test_quantlib_digital_spread_positive() {
    // QuantLib property: (C(K1) - C(K2))/(K2 - K1) >= 0 for K1 < K2
    let as_of = date!(2024 - 12 - 20);
    let market = standard_market(as_of);

    let k1 = 150.0;
    let k2 = 250.0;

    let c1 = CDSOptionBuilder::new()
        .call()
        .strike(k1)
        .expiry_months(12)
        .cds_maturity_months(60)
        .implied_vol(0.30)
        .build(as_of)
        .value(&market, as_of)
        .unwrap()
        .amount();

    let c2 = CDSOptionBuilder::new()
        .call()
        .strike(k2)
        .expiry_months(12)
        .cds_maturity_months(60)
        .implied_vol(0.30)
        .build(as_of)
        .value(&market, as_of)
        .unwrap()
        .amount();

    let spread_value = (c1 - c2) / (k2 - k1);

    assert!(
        spread_value >= -1e-6,
        "Digital spread value should be non-negative: {}",
        spread_value
    );
}

// ============================================================================
// Summary: Comprehensive Parity Check
// ============================================================================

#[test]
fn test_quantlib_comprehensive_properties() {
    // Comprehensive test covering all major QuantLib properties
    let as_of = date!(2024 - 12 - 20);
    let market = standard_market(as_of);

    let option = CDSOptionBuilder::new()
        .call()
        .strike(200.0)
        .expiry_months(12)
        .cds_maturity_months(60)
        .implied_vol(0.30)
        .notional(10_000_000.0, Currency::USD)
        .build(as_of);

    // Test all metrics
    let result = option
        .price_with_metrics(
            &market,
            as_of,
            &[
                MetricId::Delta,
                MetricId::Gamma,
                MetricId::Vega,
                MetricId::Theta,
                MetricId::Rho,
                MetricId::ImpliedVol,
            ],
        )
        .unwrap();

    // QuantLib property 1: Positive value
    assert_positive(result.value.amount(), "Option PV");

    // QuantLib property 2: All greeks finite
    for (name, value) in &result.measures {
        assert_finite(*value, &format!("Metric: {}", name));
    }

    // QuantLib property 3: Greek signs
    let delta = *result.measures.get("delta").unwrap();
    let gamma = *result.measures.get("gamma").unwrap();
    let vega = *result.measures.get("vega").unwrap();

    assert!(delta > 0.0, "Call delta should be positive");
    assert_non_negative(gamma, "Gamma");
    assert_positive(vega, "Vega");

    // QuantLib property 4: Implied vol round-trip
    let iv = *result.measures.get("implied_vol").unwrap();
    assert_approx_eq(iv, 0.30, 1e-6, "Implied vol round-trip");
}
