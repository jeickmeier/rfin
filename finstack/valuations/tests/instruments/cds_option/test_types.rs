//! Unit tests for CdsOption type construction and basic methods.

use super::common::*;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::instruments::cds_option::parameters::CdsOptionParams;
use finstack_valuations::instruments::cds_option::CdsOption;
use finstack_valuations::instruments::common::parameters::OptionType;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::CreditParams;
use finstack_valuations::pricer::InstrumentType;
use time::macros::date;

#[test]
fn test_cds_option_construction() {
    let _as_of = date!(2025 - 01 - 01);
    let expiry = date!(2026 - 01 - 01);
    let maturity = date!(2031 - 01 - 01);

    let option_params = CdsOptionParams::call(
        100.0,
        expiry,
        maturity,
        Money::new(10_000_000.0, Currency::USD),
    );
    let credit_params = CreditParams::corporate_standard("CORP", "HZ-CORP");

    let option = CdsOption::new(
        "TEST-CDSOPT",
        &option_params,
        &credit_params,
        "USD-OIS",
        "CDS-VOL",
    );

    assert_eq!(option.id(), "TEST-CDSOPT");
    assert_eq!(option.strike_spread_bp, 100.0);
    assert!(matches!(option.option_type, OptionType::Call));
    assert_eq!(option.expiry, expiry);
    assert_eq!(option.cds_maturity, maturity);
    assert_eq!(option.notional.amount(), 10_000_000.0);
    assert_eq!(option.notional.currency(), Currency::USD);
    assert_eq!(option.disc_id.as_str(), "USD-OIS");
    assert_eq!(option.credit_id.as_str(), "HZ-CORP");
    assert_eq!(option.vol_id.as_str(), "CDS-VOL");
}

#[test]
fn test_instrument_trait_implementation() {
    let as_of = date!(2025 - 01 - 01);
    let option = CdsOptionBuilder::new().build(as_of);

    assert_eq!(option.id(), "CDSOPT-TEST");
    assert_eq!(option.key(), InstrumentType::CDSOption);
}

#[test]
fn test_instrument_clone_box() {
    let as_of = date!(2025 - 01 - 01);
    let option = CdsOptionBuilder::new().build(as_of);

    let boxed = option.clone_box();
    assert_eq!(boxed.id(), option.id());
    assert_eq!(boxed.key(), option.key());
}

#[test]
fn test_single_name_option_defaults() {
    let as_of = date!(2025 - 01 - 01);
    let option = CdsOptionBuilder::new().build(as_of);

    assert!(!option.underlying_is_index);
    assert_eq!(option.index_factor, None);
    assert_eq!(option.forward_spread_adjust_bp, 0.0);
}

#[test]
fn test_index_option_construction() {
    let as_of = date!(2025 - 01 - 01);
    let option = CdsOptionBuilder::new()
        .as_index(0.88)
        .forward_adjust(15.0)
        .build(as_of);

    assert!(option.underlying_is_index);
    assert_eq!(option.index_factor, Some(0.88));
    assert_eq!(option.forward_spread_adjust_bp, 15.0);
}

#[test]
fn test_call_option() {
    let as_of = date!(2025 - 01 - 01);
    let option = CdsOptionBuilder::new().call().build(as_of);

    assert!(matches!(option.option_type, OptionType::Call));
}

#[test]
fn test_put_option() {
    let as_of = date!(2025 - 01 - 01);
    let option = CdsOptionBuilder::new().put().build(as_of);

    assert!(matches!(option.option_type, OptionType::Put));
}

#[test]
fn test_various_strikes() {
    let as_of = date!(2025 - 01 - 01);

    for strike in [25.0, 50.0, 100.0, 200.0, 500.0] {
        let option = CdsOptionBuilder::new().strike(strike).build(as_of);
        assert_eq!(option.strike_spread_bp, strike);
    }
}

#[test]
fn test_various_maturities() {
    let as_of = date!(2025 - 01 - 01);

    for expiry_months in [3, 6, 12, 24] {
        for cds_months in [36, 60, 84, 120] {
            let option = CdsOptionBuilder::new()
                .expiry_months(expiry_months)
                .cds_maturity_months(cds_months)
                .build(as_of);

            // Verify dates are correctly set
            assert!(option.expiry > as_of);
            assert!(option.cds_maturity > option.expiry);
        }
    }
}

#[test]
fn test_pricing_overrides() {
    let as_of = date!(2025 - 01 - 01);
    let mut option = CdsOptionBuilder::new().implied_vol(0.45).build(as_of);

    assert_eq!(option.pricing_overrides.implied_volatility, Some(0.45));

    // Test modification
    option.pricing_overrides.implied_volatility = Some(0.25);
    assert_eq!(option.pricing_overrides.implied_volatility, Some(0.25));
}

#[test]
fn test_attributes_access() {
    let as_of = date!(2025 - 01 - 01);
    let mut option = CdsOptionBuilder::new().build(as_of);

    // Test mutable access
    option
        .attributes_mut()
        .meta
        .insert("test_key".to_string(), "test_value".to_string());

    assert_eq!(
        option.attributes().meta.get("test_key"),
        Some(&"test_value".to_string())
    );
}
