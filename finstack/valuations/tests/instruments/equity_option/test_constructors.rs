//! Tests for equity option constructors and builders.

use super::helpers::*;
use finstack_core::currency::Currency;
use finstack_core::dates::DayCount;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::equity::equity_option::EquityOption;
use finstack_valuations::instruments::market::{ExerciseStyle, OptionType};
use finstack_valuations::instruments::Attributes;
use finstack_valuations::instruments::{PricingOverrides, SettlementType};
use finstack_valuations::test_utils;
use time::macros::date;

#[test]
fn test_builder_creates_valid_option() {
    let expiry = date!(2025 - 12 - 31);

    let option = EquityOption::builder()
        .id(InstrumentId::new("TEST_CALL"))
        .underlying_ticker("AAPL".to_string())
        .strike(Money::new(150.0, Currency::USD))
        .option_type(OptionType::Call)
        .exercise_style(ExerciseStyle::European)
        .expiry(expiry)
        .contract_size(100.0)
        .day_count(DayCount::Act365F)
        .settlement(SettlementType::Cash)
        .discount_curve_id(CurveId::new(DISC_ID))
        .spot_id(SPOT_ID.to_string())
        .vol_surface_id(CurveId::new(VOL_ID))
        .div_yield_id_opt(Some(DIV_ID.to_string()))
        .pricing_overrides(PricingOverrides::default())
        .attributes(Attributes::new())
        .build()
        .unwrap();

    assert_eq!(option.id.as_str(), "TEST_CALL");
    assert_eq!(option.strike.amount(), 150.0);
    assert_eq!(option.option_type, OptionType::Call);
    assert_eq!(option.contract_size, 100.0);
}

#[test]
fn test_european_call_convenience_constructor() {
    let expiry = date!(2025 - 12 - 31);
    let notional = Money::new(1_000_000.0, Currency::USD);

    let call =
        test_utils::equity_option_european_call("SPX-CALL", "SPX", 4500.0, expiry, notional, 100.0)
            .unwrap();

    assert_eq!(call.option_type, OptionType::Call);
    assert_eq!(call.exercise_style, ExerciseStyle::European);
    assert_eq!(call.strike.amount(), 4500.0);
    assert_eq!(call.contract_size, 100.0);
    assert_eq!(call.settlement, SettlementType::Cash);
}

#[test]
fn test_european_put_convenience_constructor() {
    let expiry = date!(2025 - 12 - 31);
    let notional = Money::new(1_000_000.0, Currency::USD);

    let put =
        test_utils::equity_option_european_put("SPX-PUT", "SPX", 4200.0, expiry, notional, 100.0)
            .unwrap();

    assert_eq!(put.option_type, OptionType::Put);
    assert_eq!(put.exercise_style, ExerciseStyle::European);
    assert_eq!(put.strike.amount(), 4200.0);
    assert_eq!(put.contract_size, 100.0);
}

#[test]
fn test_american_call_convenience_constructor() {
    let expiry = date!(2025 - 12 - 31);
    let notional = Money::new(1_000_000.0, Currency::USD);

    let call =
        test_utils::equity_option_american_call("SPX-AMER", "SPX", 4500.0, expiry, notional, 100.0)
            .unwrap();

    assert_eq!(call.exercise_style, ExerciseStyle::American);
    assert_eq!(call.option_type, OptionType::Call);
}

#[test]
fn test_contract_size_variations() {
    let expiry = date!(2025 - 12 - 31);
    let notional = Money::new(1_000_000.0, Currency::USD);

    // Standard contract
    let standard =
        test_utils::equity_option_european_call("STD", "SPX", 100.0, expiry, notional, 100.0)
            .unwrap();
    assert_eq!(standard.contract_size, 100.0);

    // Mini contract
    let mini =
        test_utils::equity_option_european_call("MINI", "SPX", 100.0, expiry, notional, 10.0)
            .unwrap();
    assert_eq!(mini.contract_size, 10.0);

    // Custom size
    let custom =
        test_utils::equity_option_european_call("CUSTOM", "SPX", 100.0, expiry, notional, 50.0)
            .unwrap();
    assert_eq!(custom.contract_size, 50.0);
}

#[test]
fn test_settlement_type_variations() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);

    let mut cash_settled = create_call(as_of, expiry, 100.0);
    cash_settled.settlement = SettlementType::Cash;
    assert_eq!(cash_settled.settlement, SettlementType::Cash);

    let mut physical_settled = create_call(as_of, expiry, 100.0);
    physical_settled.settlement = SettlementType::Physical;
    assert_eq!(physical_settled.settlement, SettlementType::Physical);
}

#[test]
fn test_exercise_style_variations() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);

    let mut european = create_call(as_of, expiry, 100.0);
    european.exercise_style = ExerciseStyle::European;
    assert_eq!(european.exercise_style, ExerciseStyle::European);

    let mut american = create_call(as_of, expiry, 100.0);
    american.exercise_style = ExerciseStyle::American;
    assert_eq!(american.exercise_style, ExerciseStyle::American);
}

#[test]
fn test_convenience_constructors_use_notional_currency() {
    let expiry = date!(2025 - 12 - 31);
    let notional = Money::new(1_000_000.0, Currency::EUR);

    let option =
        test_utils::equity_option_european_call("EUR-OPT", "SX5E", 4000.0, expiry, notional, 100.0)
            .unwrap();

    // Strike currency matches notional currency
    assert_eq!(option.strike.currency(), Currency::EUR);
}
