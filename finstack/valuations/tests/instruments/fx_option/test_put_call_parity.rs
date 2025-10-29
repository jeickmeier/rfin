//! Market standard test: Put-Call Parity for FX options.
//!
//! Validates that the Garman-Kohlhagen model satisfies put-call parity:
//! C - P = S * exp(-r_f * T) - K * exp(-r_d * T)
//!
//! This is a fundamental no-arbitrage relationship that all FX option
//! pricing models must satisfy.

use super::helpers::*;
use finstack_valuations::instruments::fx_option::FxOptionCalculator;
use time::macros::date;

#[ignore = "QuantLib parity: comprehensive validation"]
#[test]
fn test_put_call_parity_atm() {
    // Arrange: ATM call and put
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let strike = 1.20;

    let call = build_call_option(as_of, expiry, strike, 1_000_000.0);
    let put = build_put_option(as_of, expiry, strike, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());
    let calc = FxOptionCalculator::new();

    // Act
    let call_pv = calc.npv(&call, &market, as_of).unwrap();
    let put_pv = calc.npv(&put, &market, as_of).unwrap();

    let (spot, r_d, r_f, _, t) = calc.collect_inputs(&call, &market, as_of).unwrap();

    // Put-call parity: C - P = S * exp(-r_f * T) - K * exp(-r_d * T)
    let notional = call.notional.amount();
    let lhs = call_pv.amount() - put_pv.amount();
    let rhs = notional * (spot * (-r_f * t).exp() - strike * (-r_d * t).exp());

    // Assert
    assert_approx_eq(lhs, rhs, 1e-6, 1.0, "Put-call parity should hold ATM");
}

#[ignore = "QuantLib parity: comprehensive validation"]
#[test]
fn test_put_call_parity_itm() {
    // Arrange: ITM call (OTM put)
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let strike = 1.10; // ITM for call (spot = 1.20)

    let call = build_call_option(as_of, expiry, strike, 1_000_000.0);
    let put = build_put_option(as_of, expiry, strike, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());
    let calc = FxOptionCalculator::new();

    // Act
    let call_pv = calc.npv(&call, &market, as_of).unwrap();
    let put_pv = calc.npv(&put, &market, as_of).unwrap();

    let (spot, r_d, r_f, _, t) = calc.collect_inputs(&call, &market, as_of).unwrap();

    let notional = call.notional.amount();
    let lhs = call_pv.amount() - put_pv.amount();
    let rhs = notional * (spot * (-r_f * t).exp() - strike * (-r_d * t).exp());

    // Assert
    assert_approx_eq(lhs, rhs, 1e-6, 1.0, "Put-call parity should hold ITM");
}

#[ignore = "QuantLib parity: comprehensive validation"]
#[test]
fn test_put_call_parity_otm() {
    // Arrange: OTM call (ITM put)
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let strike = 1.35; // OTM for call (spot = 1.20)

    let call = build_call_option(as_of, expiry, strike, 1_000_000.0);
    let put = build_put_option(as_of, expiry, strike, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());
    let calc = FxOptionCalculator::new();

    // Act
    let call_pv = calc.npv(&call, &market, as_of).unwrap();
    let put_pv = calc.npv(&put, &market, as_of).unwrap();

    let (spot, r_d, r_f, _, t) = calc.collect_inputs(&call, &market, as_of).unwrap();

    let notional = call.notional.amount();
    let lhs = call_pv.amount() - put_pv.amount();
    let rhs = notional * (spot * (-r_f * t).exp() - strike * (-r_d * t).exp());

    // Assert
    assert_approx_eq(lhs, rhs, 1e-6, 1.0, "Put-call parity should hold OTM");
}

#[ignore = "QuantLib parity: comprehensive validation"]
#[test]
fn test_put_call_parity_high_vol() {
    // Arrange: High volatility environment
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let strike = 1.20;

    let call = build_call_option(as_of, expiry, strike, 1_000_000.0);
    let put = build_put_option(as_of, expiry, strike, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::high_vol());
    let calc = FxOptionCalculator::new();

    // Act
    let call_pv = calc.npv(&call, &market, as_of).unwrap();
    let put_pv = calc.npv(&put, &market, as_of).unwrap();

    let (spot, r_d, r_f, _, t) = calc.collect_inputs(&call, &market, as_of).unwrap();

    let notional = call.notional.amount();
    let lhs = call_pv.amount() - put_pv.amount();
    let rhs = notional * (spot * (-r_f * t).exp() - strike * (-r_d * t).exp());

    // Assert: Parity holds regardless of vol level
    assert_approx_eq(
        lhs,
        rhs,
        1e-6,
        1.0,
        "Put-call parity should hold at high vol",
    );
}

#[ignore = "QuantLib parity: comprehensive validation"]
#[test]
fn test_put_call_parity_steep_carry() {
    // Arrange: Steep interest rate differential
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let strike = 1.20;

    let call = build_call_option(as_of, expiry, strike, 1_000_000.0);
    let put = build_put_option(as_of, expiry, strike, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::steep_carry());
    let calc = FxOptionCalculator::new();

    // Act
    let call_pv = calc.npv(&call, &market, as_of).unwrap();
    let put_pv = calc.npv(&put, &market, as_of).unwrap();

    let (spot, r_d, r_f, _, t) = calc.collect_inputs(&call, &market, as_of).unwrap();

    let notional = call.notional.amount();
    let lhs = call_pv.amount() - put_pv.amount();
    let rhs = notional * (spot * (-r_f * t).exp() - strike * (-r_d * t).exp());

    // Assert: Parity holds with carry
    assert_approx_eq(
        lhs,
        rhs,
        1e-6,
        1.0,
        "Put-call parity should hold with steep carry",
    );
}

#[ignore = "QuantLib parity: comprehensive validation"]
#[test]
fn test_put_call_parity_short_dated() {
    // Arrange: 1M option
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2024 - 02 - 01);
    let strike = 1.20;

    let call = build_call_option(as_of, expiry, strike, 1_000_000.0);
    let put = build_put_option(as_of, expiry, strike, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());
    let calc = FxOptionCalculator::new();

    // Act
    let call_pv = calc.npv(&call, &market, as_of).unwrap();
    let put_pv = calc.npv(&put, &market, as_of).unwrap();

    let (spot, r_d, r_f, _, t) = calc.collect_inputs(&call, &market, as_of).unwrap();

    let notional = call.notional.amount();
    let lhs = call_pv.amount() - put_pv.amount();
    let rhs = notional * (spot * (-r_f * t).exp() - strike * (-r_d * t).exp());

    // Assert
    assert_approx_eq(
        lhs,
        rhs,
        1e-6,
        1.0,
        "Put-call parity should hold for short dated",
    );
}

#[ignore = "QuantLib parity: comprehensive validation"]
#[test]
fn test_put_call_parity_long_dated() {
    // Arrange: 5Y option
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2029 - 01 - 01);
    let strike = 1.20;

    let call = build_call_option(as_of, expiry, strike, 1_000_000.0);
    let put = build_put_option(as_of, expiry, strike, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());
    let calc = FxOptionCalculator::new();

    // Act
    let call_pv = calc.npv(&call, &market, as_of).unwrap();
    let put_pv = calc.npv(&put, &market, as_of).unwrap();

    let (spot, r_d, r_f, _, t) = calc.collect_inputs(&call, &market, as_of).unwrap();

    let notional = call.notional.amount();
    let lhs = call_pv.amount() - put_pv.amount();
    let rhs = notional * (spot * (-r_f * t).exp() - strike * (-r_d * t).exp());

    // Assert
    assert_approx_eq(
        lhs,
        rhs,
        1e-5,
        10.0,
        "Put-call parity should hold for long dated",
    );
}

#[ignore = "QuantLib parity: comprehensive validation"]
#[test]
fn test_put_call_parity_different_notionals() {
    // Arrange: Various notional sizes
    for notional in [100_000.0, 1_000_000.0, 10_000_000.0] {
        let as_of = date!(2024 - 01 - 01);
        let expiry = date!(2025 - 01 - 01);
        let strike = 1.20;

        let call = build_call_option(as_of, expiry, strike, notional);
        let put = build_put_option(as_of, expiry, strike, notional);
        let market = build_market_context(as_of, MarketParams::atm());
        let calc = FxOptionCalculator::new();

        // Act
        let call_pv = calc.npv(&call, &market, as_of).unwrap();
        let put_pv = calc.npv(&put, &market, as_of).unwrap();

        let (spot, r_d, r_f, _, t) = calc.collect_inputs(&call, &market, as_of).unwrap();

        let lhs = call_pv.amount() - put_pv.amount();
        let rhs = notional * (spot * (-r_f * t).exp() - strike * (-r_d * t).exp());

        // Assert
        let tol = notional * 1e-6;
        assert_approx_eq(
            lhs,
            rhs,
            1e-6,
            tol,
            &format!("Put-call parity at notional {}", notional),
        );
    }
}
