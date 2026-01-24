//! FX option instrument construction and trait implementation tests.
//!
//! Tests builders, convenience constructors, and trait implementations.

use super::helpers::*;
use finstack_core::currency::Currency;
use finstack_core::dates::DayCount;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::fx::fx_option::{FxOption, FxOptionParams};
use finstack_valuations::instruments::market::{ExerciseStyle, OptionType};
use finstack_valuations::instruments::FxUnderlyingParams;
use finstack_valuations::instruments::{Attributes, Instrument};
use finstack_valuations::instruments::{PricingOverrides, SettlementType};
use finstack_valuations::test_utils;
use time::macros::date;

#[test]
fn test_builder_pattern_creates_valid_option() {
    // Arrange & Act
    let option = FxOption::builder()
        .id(InstrumentId::new("TEST_CALL"))
        .base_currency(Currency::EUR)
        .quote_currency(Currency::USD)
        .strike(1.20)
        .option_type(OptionType::Call)
        .exercise_style(ExerciseStyle::European)
        .expiry(date!(2025 - 01 - 01))
        .day_count(DayCount::Act365F)
        .notional(Money::new(1_000_000.0, Currency::EUR))
        .settlement(SettlementType::Cash)
        .domestic_discount_curve_id(CurveId::new("USD-OIS"))
        .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
        .vol_surface_id(CurveId::new("EURUSD-VOL"))
        .pricing_overrides(PricingOverrides::default())
        .attributes(Attributes::new())
        .build();

    // Assert
    assert!(option.is_ok(), "Builder should create valid option");
    let opt = option.unwrap();
    assert_eq!(opt.id.as_str(), "TEST_CALL");
    assert_eq!(opt.strike, 1.20);
    assert_eq!(opt.option_type, OptionType::Call);
}

#[test]
fn test_european_call_convenience_constructor() {
    // Arrange & Act
    let call = test_utils::fx_option_european_call(
        "EUR_USD_CALL",
        Currency::EUR,
        Currency::USD,
        1.20,
        date!(2025 - 01 - 01),
        Money::new(1_000_000.0, Currency::EUR),
        CurveId::new("EURUSD-VOL"),
    )
    .unwrap();

    // Assert
    assert_eq!(call.id.as_str(), "EUR_USD_CALL");
    assert_eq!(call.option_type, OptionType::Call);
    assert_eq!(call.exercise_style, ExerciseStyle::European);
    assert_eq!(call.settlement, SettlementType::Cash);
    assert_eq!(call.strike, 1.20);
    assert_eq!(call.notional.amount(), 1_000_000.0);
    assert_eq!(call.notional.currency(), Currency::EUR);
}

#[test]
fn test_european_put_convenience_constructor() {
    // Arrange & Act
    let put = test_utils::fx_option_european_put(
        "EUR_USD_PUT",
        Currency::EUR,
        Currency::USD,
        1.20,
        date!(2025 - 01 - 01),
        Money::new(1_000_000.0, Currency::EUR),
        CurveId::new("EURUSD-VOL"),
    )
    .unwrap();

    // Assert
    assert_eq!(put.id.as_str(), "EUR_USD_PUT");
    assert_eq!(put.option_type, OptionType::Put);
    assert_eq!(put.exercise_style, ExerciseStyle::European);
    assert_eq!(put.settlement, SettlementType::Cash);
}

#[test]
fn test_new_with_parameter_structs() {
    // Arrange
    let option_params = FxOptionParams::new(
        1.20,
        date!(2025 - 01 - 01),
        OptionType::Call,
        Money::new(1_000_000.0, Currency::EUR),
    );
    let underlying_params = FxUnderlyingParams::usd_eur();

    // Act
    let option = FxOption::new(
        "TEST_OPTION",
        &option_params,
        &underlying_params,
        "EURUSD-VOL",
    );

    // Assert
    assert_eq!(option.id.as_str(), "TEST_OPTION");
    assert_eq!(option.base_currency, underlying_params.base_currency);
    assert_eq!(option.quote_currency, underlying_params.quote_currency);
    assert_eq!(option.strike, option_params.strike);
    assert_eq!(option.option_type, option_params.option_type);
}

#[test]
fn test_instrument_trait_id() {
    // Arrange
    let call = build_call_option(
        date!(2024 - 01 - 01),
        date!(2025 - 01 - 01),
        1.20,
        1_000_000.0,
    );

    // Act & Assert
    assert_eq!(call.id(), "FX_CALL_TEST");
}

#[test]
fn test_instrument_trait_key() {
    // Arrange
    let call = build_call_option(
        date!(2024 - 01 - 01),
        date!(2025 - 01 - 01),
        1.20,
        1_000_000.0,
    );

    // Act
    use finstack_valuations::pricer::InstrumentType;
    let key = call.key();

    // Assert
    assert_eq!(key, InstrumentType::FxOption);
}

#[test]
fn test_instrument_trait_as_any() {
    // Arrange
    let call = build_call_option(
        date!(2024 - 01 - 01),
        date!(2025 - 01 - 01),
        1.20,
        1_000_000.0,
    );

    // Act
    let any = call.as_any();

    // Assert: Should be able to downcast
    assert!(any.downcast_ref::<FxOption>().is_some());
}

#[test]
fn test_instrument_trait_attributes() {
    // Arrange
    let mut call = build_call_option(
        date!(2024 - 01 - 01),
        date!(2025 - 01 - 01),
        1.20,
        1_000_000.0,
    );

    // Act: Set and get attributes
    call.attributes_mut()
        .meta
        .insert("test_key".to_string(), "test_value".to_string());
    let value = call.attributes().meta.get("test_key");

    // Assert
    assert_eq!(value.map(|s| s.as_str()), Some("test_value"));
}

#[test]
fn test_instrument_trait_clone_box() {
    // Arrange
    let call = build_call_option(
        date!(2024 - 01 - 01),
        date!(2025 - 01 - 01),
        1.20,
        1_000_000.0,
    );

    // Act
    let boxed = call.clone_box();

    // Assert
    assert_eq!(boxed.id(), call.id());
}

#[test]
fn test_instrument_trait_value() {
    // Arrange
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());

    // Act: Call via trait
    let pv = call.value(&market, as_of).unwrap();

    // Assert
    assert!(pv.amount() > 0.0);
    assert_eq!(pv.currency(), QUOTE);
}

#[test]
fn test_npv_method() {
    // Arrange
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());

    // Act
    let pv = call.npv(&market, as_of).unwrap();

    // Assert
    assert!(pv.amount() > 0.0);
    assert_eq!(pv.currency(), QUOTE);
}

#[test]
fn test_value_and_npv_equivalent() {
    // Arrange
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());

    // Act
    let pv1 = call.value(&market, as_of).unwrap();
    let pv2 = call.npv(&market, as_of).unwrap();

    // Assert: Should be identical
    assert_eq!(pv1.amount(), pv2.amount());
    assert_eq!(pv1.currency(), pv2.currency());
}

#[test]
fn test_compute_greeks_method() {
    // Arrange
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());

    // Act
    let greeks = call.compute_greeks(&market, as_of).unwrap();

    // Assert
    assert!(greeks.delta.is_finite());
    assert!(greeks.gamma >= 0.0);
    assert!(greeks.vega > 0.0);
}

#[test]
fn test_implied_vol_method() {
    // Arrange
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());

    // Act
    let pv = call.npv(&market, as_of).unwrap();
    let iv = call.implied_vol(&market, as_of, pv.amount(), None).unwrap();

    // Assert
    assert_approx_eq(iv, 0.15, 1e-6, 1e-6, "IV method should recover market vol");
}

#[test]
fn test_calculator_method() {
    // Arrange
    let call = build_call_option(
        date!(2024 - 01 - 01),
        date!(2025 - 01 - 01),
        1.20,
        1_000_000.0,
    );

    // Act
    let calc = call.calculator();

    // Assert: Should create a new calculator
    // (just verify it exists and has default config)
    assert_eq!(calc.config.theta_days_per_year, 365.0);
}

#[test]
fn test_pricing_overrides_applied() {
    // Arrange
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let mut call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);

    // Override vol to 30%
    call.pricing_overrides.implied_volatility = Some(0.30);

    let market = build_market_context(as_of, MarketParams::atm()); // Market vol is 15%
    let calc = call.calculator();

    // Act
    let (_spot, _r_d, _r_f, sigma, _t) = calc.collect_inputs(&call, &market, as_of).unwrap();

    // Assert: Should use override vol
    assert_eq!(sigma, 0.30, "Should use override vol");
}

#[test]
fn test_attributes_are_mutable() {
    // Arrange
    let mut call = build_call_option(
        date!(2024 - 01 - 01),
        date!(2025 - 01 - 01),
        1.20,
        1_000_000.0,
    );

    // Act
    call.attributes_mut()
        .meta
        .insert("trader".to_string(), "Alice".to_string());
    call.attributes_mut()
        .meta
        .insert("book".to_string(), "FX_OPTIONS".to_string());

    // Assert
    assert_eq!(
        call.attributes().meta.get("trader").map(|s| s.as_str()),
        Some("Alice")
    );
    assert_eq!(
        call.attributes().meta.get("book").map(|s| s.as_str()),
        Some("FX_OPTIONS")
    );
}

#[test]
fn test_clone_preserves_all_fields() {
    // Arrange
    let original = build_call_option(
        date!(2024 - 01 - 01),
        date!(2025 - 01 - 01),
        1.20,
        1_000_000.0,
    );

    // Act
    let cloned = original.clone();

    // Assert
    assert_eq!(cloned.id, original.id);
    assert_eq!(cloned.strike, original.strike);
    assert_eq!(cloned.option_type, original.option_type);
    assert_eq!(cloned.expiry, original.expiry);
    assert_eq!(cloned.notional.amount(), original.notional.amount());
}

#[test]
fn test_debug_trait_implemented() {
    // Arrange
    let call = build_call_option(
        date!(2024 - 01 - 01),
        date!(2025 - 01 - 01),
        1.20,
        1_000_000.0,
    );

    // Act: Format as debug string
    let debug_str = format!("{:?}", call);

    // Assert: Should contain key fields
    assert!(debug_str.contains("FxOption"));
    assert!(debug_str.contains("strike"));
}

#[test]
fn test_settlement_types() {
    // Arrange & Act: Create with physical settlement
    let mut call = build_call_option(
        date!(2024 - 01 - 01),
        date!(2025 - 01 - 01),
        1.20,
        1_000_000.0,
    );
    call.settlement = SettlementType::Physical;

    // Assert
    assert_eq!(call.settlement, SettlementType::Physical);

    // Change to cash
    call.settlement = SettlementType::Cash;
    assert_eq!(call.settlement, SettlementType::Cash);
}

#[test]
fn test_exercise_styles() {
    // Arrange: Test different exercise styles can be set
    let mut call = build_call_option(
        date!(2024 - 01 - 01),
        date!(2025 - 01 - 01),
        1.20,
        1_000_000.0,
    );

    // Act & Assert: European (default)
    assert_eq!(call.exercise_style, ExerciseStyle::European);

    // Change to American (though not priced differently yet)
    call.exercise_style = ExerciseStyle::American;
    assert_eq!(call.exercise_style, ExerciseStyle::American);
}
