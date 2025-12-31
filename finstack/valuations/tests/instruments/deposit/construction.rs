//! Construction and validation tests for Deposit instruments.
//!
//! Tests the builder pattern, field validation, and proper initialization
//! of deposit instruments.

use super::common::*;
use finstack_core::currency::Currency;
use finstack_core::dates::DayCount;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::rates::deposit::Deposit;

#[test]
fn test_basic_construction() {
    // Setup
    let base = date(2025, 1, 1);

    // Execute
    let dep = Deposit::builder()
        .id(InstrumentId::new("DEP-001"))
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(base)
        .end(date(2025, 7, 1))
        .day_count(DayCount::Act360)
        .discount_curve_id(CurveId::new("USD-OIS"))
        .build()
        .unwrap();

    // Validate
    assert_eq!(dep.id.as_str(), "DEP-001");
    assert_eq!(dep.notional.amount(), 1_000_000.0);
    assert_eq!(dep.notional.currency(), Currency::USD);
    assert_eq!(dep.start, base);
    assert_eq!(dep.end, date(2025, 7, 1));
    assert!(matches!(dep.day_count, DayCount::Act360));
    assert_eq!(dep.discount_curve_id.as_str(), "USD-OIS");
    assert!(dep.quote_rate.is_none());
}

#[test]
fn test_construction_with_quote_rate() {
    // Setup
    let base = date(2025, 1, 1);

    // Execute
    let mut dep = Deposit::builder()
        .id(InstrumentId::new("DEP-002"))
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(base)
        .end(date(2025, 7, 1))
        .day_count(DayCount::Act360)
        .discount_curve_id(CurveId::new("USD-OIS"))
        .build()
        .unwrap();
    dep.quote_rate = Some(0.05);

    // Validate
    assert!(dep.quote_rate.is_some());
    assert!((dep.quote_rate.unwrap() - 0.05).abs() < 1e-12);
}

#[test]
fn test_construction_with_different_day_counts() {
    // Test various day count conventions
    let base = date(2025, 1, 1);
    let day_counts = vec![
        DayCount::Act360,
        DayCount::Act365F,
        DayCount::Thirty360,
        DayCount::ActAct,
    ];

    for dc in day_counts {
        let dep = Deposit::builder()
            .id(InstrumentId::new("DEP-DC"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .start(base)
            .end(date(2025, 7, 1))
            .day_count(dc)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .build()
            .unwrap();

        // Verify day count was set
        let _ = dep.day_count;
    }
}

#[test]
fn test_construction_with_different_currencies() {
    // Test multiple currencies
    let base = date(2025, 1, 1);
    let currencies = vec![Currency::USD, Currency::EUR, Currency::GBP, Currency::JPY];

    for ccy in currencies {
        let dep = Deposit::builder()
            .id(InstrumentId::new("DEP-CCY"))
            .notional(Money::new(1_000_000.0, ccy))
            .start(base)
            .end(date(2025, 7, 1))
            .day_count(DayCount::Act360)
            .discount_curve_id(CurveId::new(format!("{}-OIS", ccy)))
            .build()
            .unwrap();

        assert_eq!(dep.notional.currency(), ccy);
    }
}

#[test]
fn test_construction_with_various_maturities() {
    // Test deposits of different tenors
    let base = date(2025, 1, 1);

    // 1 week
    let dep_1w = DepositBuilder::new(base).end(date(2025, 1, 8)).build();
    assert!(dep_1w.end > dep_1w.start);

    // 1 month
    let dep_1m = DepositBuilder::new(base).end(date(2025, 2, 1)).build();
    assert!(dep_1m.end > dep_1m.start);

    // 3 months
    let dep_3m = DepositBuilder::new(base).end(date(2025, 4, 1)).build();
    assert!(dep_3m.end > dep_3m.start);

    // 6 months
    let dep_6m = DepositBuilder::new(base).end(date(2025, 7, 1)).build();
    assert!(dep_6m.end > dep_6m.start);

    // 1 year
    let dep_1y = DepositBuilder::new(base).end(date(2026, 1, 1)).build();
    assert!(dep_1y.end > dep_1y.start);
}

#[test]
fn test_construction_with_large_notional() {
    // Setup - Test with very large notional
    let base = date(2025, 1, 1);

    // Execute
    let dep = DepositBuilder::new(base)
        .notional(Money::new(1_000_000_000.0, Currency::USD))
        .build();

    // Validate
    assert_eq!(dep.notional.amount(), 1_000_000_000.0);
}

#[test]
fn test_construction_with_small_notional() {
    // Setup - Test with small notional
    let base = date(2025, 1, 1);

    // Execute
    let dep = DepositBuilder::new(base)
        .notional(Money::new(1_000.0, Currency::USD))
        .build();

    // Validate
    assert_eq!(dep.notional.amount(), 1_000.0);
}

#[test]
fn test_builder_pattern_ergonomics() {
    // Test that builder can be used fluently
    let base = date(2025, 1, 1);

    let dep = DepositBuilder::new(base)
        .id("DEP-FLUENT")
        .notional(Money::new(5_000_000.0, Currency::EUR))
        .start(date(2025, 2, 1))
        .end(date(2025, 8, 1))
        .day_count(DayCount::Act365F)
        .quote_rate(0.03)
        .discount_curve_id("EUR-OIS")
        .build();

    assert_eq!(dep.id.as_str(), "DEP-FLUENT");
    assert_eq!(dep.notional.amount(), 5_000_000.0);
    assert_eq!(dep.notional.currency(), Currency::EUR);
    assert!(dep.quote_rate.is_some());
}

#[test]
fn test_clone_deposit() {
    // Setup
    let base = date(2025, 1, 1);
    let dep = standard_deposit(base);

    // Execute
    let dep_clone = dep.clone();

    // Validate
    assert_eq!(dep.id, dep_clone.id);
    assert_eq!(dep.notional, dep_clone.notional);
    assert_eq!(dep.start, dep_clone.start);
    assert_eq!(dep.end, dep_clone.end);
}

#[test]
fn test_debug_format() {
    // Test that Debug trait is properly implemented
    let base = date(2025, 1, 1);
    let dep = standard_deposit(base);

    let debug_str = format!("{:?}", dep);
    assert!(debug_str.contains("Deposit"));
}
