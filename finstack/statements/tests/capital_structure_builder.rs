//! Capital structure builder integration tests.
#![allow(clippy::expect_used, clippy::panic)]

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_statements::builder::{ModelBuilder, NeedPeriods};
use finstack_statements::types::DebtInstrumentSpec;
use time::Month;

#[test]
fn test_add_bond() {
    let issue = Date::from_calendar_date(2025, Month::January, 15).expect("valid date");
    let maturity = Date::from_calendar_date(2030, Month::January, 15).expect("valid date");

    let builder = ModelBuilder::<NeedPeriods>::new("test")
        .periods("2025Q1..2025Q2", None)
        .expect("valid period range")
        .add_bond(
            "BOND-001",
            Money::new(1_000_000.0, Currency::USD),
            0.05,
            issue,
            maturity,
            "USD-OIS",
        )
        .expect("valid bond");
    let model = builder.build().expect("valid model");

    let cs = model
        .capital_structure
        .as_ref()
        .expect("capital_structure should exist");
    assert_eq!(cs.debt_instruments.len(), 1);

    match &cs.debt_instruments[0] {
        DebtInstrumentSpec::Bond { id, .. } => {
            assert_eq!(id, "BOND-001");
        }
        _ => panic!("Expected Bond variant"),
    }
}

#[test]
fn test_add_swap() {
    let start = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
    let maturity = Date::from_calendar_date(2030, Month::January, 1).expect("valid date");

    let builder = ModelBuilder::<NeedPeriods>::new("test")
        .periods("2025Q1..2025Q2", None)
        .expect("valid period range")
        .add_swap(
            "SWAP-001",
            Money::new(5_000_000.0, Currency::USD),
            0.04,
            start,
            maturity,
            "USD-OIS",
            "USD-SOFR-3M",
        )
        .expect("valid swap");
    let model = builder.build().expect("valid model");

    let cs = model
        .capital_structure
        .as_ref()
        .expect("capital_structure should exist");
    assert_eq!(cs.debt_instruments.len(), 1);

    match &cs.debt_instruments[0] {
        DebtInstrumentSpec::Swap { id, .. } => {
            assert_eq!(id, "SWAP-001");
        }
        _ => panic!("Expected Swap variant"),
    }
}

#[test]
fn test_add_multiple_instruments() {
    let issue = Date::from_calendar_date(2025, Month::January, 15).expect("valid date");
    let maturity = Date::from_calendar_date(2030, Month::January, 15).expect("valid date");

    let builder = ModelBuilder::<NeedPeriods>::new("test")
        .periods("2025Q1..2025Q2", None)
        .expect("valid period range")
        .add_bond(
            "BOND-001",
            Money::new(1_000_000.0, Currency::USD),
            0.05,
            issue,
            maturity,
            "USD-OIS",
        )
        .expect("valid bond")
        .add_bond(
            "BOND-002",
            Money::new(2_000_000.0, Currency::USD),
            0.06,
            issue,
            maturity,
            "USD-OIS",
        )
        .expect("valid bond");
    let model = builder.build().expect("valid model");

    let cs = model
        .capital_structure
        .as_ref()
        .expect("capital_structure should exist");
    assert_eq!(cs.debt_instruments.len(), 2);
}

#[test]
fn test_add_custom_debt() {
    let builder = ModelBuilder::<NeedPeriods>::new("test")
        .periods("2025Q1..2025Q2", None)
        .expect("valid period range")
        .add_custom_debt(
            "TL-A",
            serde_json::json!({
                "type": "term_loan",
                "notional": 10_000_000.0,
                "currency": "USD",
            }),
        );
    let model = builder.build().expect("valid model");

    let cs = model
        .capital_structure
        .as_ref()
        .expect("capital_structure should exist");
    assert_eq!(cs.debt_instruments.len(), 1);

    match &cs.debt_instruments[0] {
        DebtInstrumentSpec::Generic { id, .. } => {
            assert_eq!(id, "TL-A");
        }
        _ => panic!("Expected Generic variant"),
    }
}
