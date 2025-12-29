//! Unit tests for CdsOptionParams builder and validation.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::instruments::cds_option::parameters::CdsOptionParams;
use finstack_valuations::instruments::common::parameters::OptionType;
use time::macros::date;

#[test]
fn test_call_constructor() {
    let expiry = date!(2025 - 12 - 31);
    let maturity = date!(2030 - 12 - 31);
    let notional = Money::new(10_000_000.0, Currency::USD);

    let params =
        CdsOptionParams::call(100.0, expiry, maturity, notional).expect("valid call params");

    assert_eq!(params.strike_spread_bp, 100.0);
    assert_eq!(params.expiry, expiry);
    assert_eq!(params.cds_maturity, maturity);
    assert_eq!(params.notional.amount(), 10_000_000.0);
    assert_eq!(params.notional.currency(), Currency::USD);
    assert!(matches!(params.option_type, OptionType::Call));
    assert!(!params.underlying_is_index);
    assert_eq!(params.index_factor, None);
    assert_eq!(params.forward_spread_adjust_bp, 0.0);
}

#[test]
fn test_put_constructor() {
    let expiry = date!(2025 - 12 - 31);
    let maturity = date!(2030 - 12 - 31);
    let notional = Money::new(5_000_000.0, Currency::EUR);

    let params = CdsOptionParams::put(150.0, expiry, maturity, notional).expect("valid put params");

    assert_eq!(params.strike_spread_bp, 150.0);
    assert!(matches!(params.option_type, OptionType::Put));
    assert_eq!(params.notional.currency(), Currency::EUR);
}

#[test]
fn test_index_option_builder() {
    let expiry = date!(2025 - 12 - 31);
    let maturity = date!(2030 - 12 - 31);
    let notional = Money::new(10_000_000.0, Currency::USD);

    let params = CdsOptionParams::call(100.0, expiry, maturity, notional)
        .expect("valid call params")
        .as_index(0.85)
        .expect("valid index factor");

    assert!(params.underlying_is_index);
    assert_eq!(params.index_factor, Some(0.85));
}

#[test]
fn test_forward_spread_adjustment() {
    let expiry = date!(2025 - 12 - 31);
    let maturity = date!(2030 - 12 - 31);
    let notional = Money::new(10_000_000.0, Currency::USD);

    let params = CdsOptionParams::call(100.0, expiry, maturity, notional)
        .expect("valid call params")
        .as_index(0.90)
        .expect("valid index factor")
        .with_forward_spread_adjust_bp(25.0);

    assert_eq!(params.forward_spread_adjust_bp, 25.0);
    assert!(params.underlying_is_index);
}

#[test]
fn test_chained_builders() {
    let expiry = date!(2025 - 06 - 30);
    let maturity = date!(2028 - 06 - 30);
    let notional = Money::new(20_000_000.0, Currency::GBP);

    let params = CdsOptionParams::put(200.0, expiry, maturity, notional)
        .expect("valid put params")
        .as_index(0.75)
        .expect("valid index factor")
        .with_forward_spread_adjust_bp(-10.0);

    assert!(matches!(params.option_type, OptionType::Put));
    assert_eq!(params.strike_spread_bp, 200.0);
    assert!(params.underlying_is_index);
    assert_eq!(params.index_factor, Some(0.75));
    assert_eq!(params.forward_spread_adjust_bp, -10.0);
}

#[test]
fn test_various_strikes() {
    let expiry = date!(2026 - 01 - 01);
    let maturity = date!(2031 - 01 - 01);
    let notional = Money::new(10_000_000.0, Currency::USD);

    for strike in [25.0, 50.0, 100.0, 200.0, 500.0, 1000.0] {
        let params =
            CdsOptionParams::call(strike, expiry, maturity, notional).expect("valid call params");
        assert_eq!(params.strike_spread_bp, strike);
    }
}

#[test]
fn test_various_currencies() {
    let expiry = date!(2026 - 01 - 01);
    let maturity = date!(2031 - 01 - 01);

    for currency in [Currency::USD, Currency::EUR, Currency::GBP, Currency::JPY] {
        let notional = Money::new(10_000_000.0, currency);
        let params =
            CdsOptionParams::call(100.0, expiry, maturity, notional).expect("valid call params");
        assert_eq!(params.notional.currency(), currency);
    }
}
