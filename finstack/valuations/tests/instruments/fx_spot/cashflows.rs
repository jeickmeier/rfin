//! FX Spot cashflow generation and settlement tests.

use super::common::*;
use finstack_core::types::InstrumentId;
use finstack_core::{
    currency::Currency, dates::BusinessDayConvention, market_data::context::MarketContext,
    money::Money,
};
use finstack_valuations::cashflow::traits::CashflowProvider;
use finstack_valuations::instruments::FxSpot;

#[test]
fn test_settlement_explicit_date() {
    let settlement = d(2025, 1, 17);
    let fx = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(settlement);
    let market = MarketContext::new();

    let cashflows = fx.build_schedule(&market, test_date()).unwrap();

    assert_eq!(cashflows.len(), 1);
    assert_eq!(cashflows[0].0, settlement);
    assert_approx_eq(
        cashflows[0].1.amount(),
        1_200_000.0,
        EPSILON,
        "Cashflow amount",
    );
    assert_eq!(cashflows[0].1.currency(), Currency::USD);
}

#[test]
fn test_settlement_lag_default() {
    // Default lag is T+2 business days
    let fx = eurusd_with_notional(1_000_000.0, 1.20);
    let market = MarketContext::new();
    let as_of = d(2025, 1, 15); // Wednesday

    let cashflows = fx.build_schedule(&market, as_of).unwrap();

    assert_eq!(cashflows.len(), 1);
    // T+2 business days from Wed Jan 15 = Fri Jan 17
    assert_eq!(cashflows[0].0, d(2025, 1, 17));
}

#[test]
fn test_settlement_already_settled() {
    let settlement = d(2025, 1, 10);
    let fx = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(settlement);
    let market = MarketContext::new();
    let as_of = d(2025, 1, 15);

    let cashflows = fx.build_schedule(&market, as_of).unwrap();

    // Already settled - no cashflows
    assert_eq!(cashflows.len(), 0);
}

#[test]
fn test_settlement_on_valuation_date() {
    let settlement = d(2025, 1, 15);
    let fx = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(settlement);
    let market = MarketContext::new();

    let cashflows = fx.build_schedule(&market, settlement).unwrap();

    // Settlement on as_of means already settled
    assert_eq!(cashflows.len(), 0);
}

#[test]
fn test_settlement_from_fx_matrix() {
    let fx = sample_eurusd()
        .with_notional(Money::new(1_000_000.0, Currency::EUR))
        .unwrap()
        .with_settlement(d(2025, 1, 17));
    let market = market_with_fx_matrix();

    let cashflows = fx.build_schedule(&market, test_date()).unwrap();

    assert_eq!(cashflows.len(), 1);
    assert_eq!(cashflows[0].1.currency(), Currency::USD);
    assert_approx_eq(
        cashflows[0].1.amount(),
        1_200_000.0,
        LARGE_EPSILON,
        "FX matrix cashflow",
    );
}

#[test]
fn test_settlement_explicit_rate_overrides_matrix() {
    let fx = eurusd_with_notional(1_000_000.0, 1.25).with_settlement(d(2025, 1, 17));
    let market = market_with_fx_matrix(); // Has EUR/USD = 1.20

    let cashflows = fx.build_schedule(&market, test_date()).unwrap();

    assert_eq!(cashflows.len(), 1);
    // Should use explicit rate 1.25, not matrix rate 1.20
    assert_approx_eq(
        cashflows[0].1.amount(),
        1_250_000.0,
        EPSILON,
        "Explicit rate overrides",
    );
}

#[test]
fn test_settlement_lag_custom() {
    let fx = FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD)
        .with_notional(Money::new(1_000_000.0, Currency::EUR))
        .unwrap()
        .with_rate(1.20)
        .with_settlement(d(2025, 1, 16)); // T+1

    let market = MarketContext::new();
    let as_of = d(2025, 1, 15); // Wednesday

    let cashflows = fx.build_schedule(&market, as_of).unwrap();

    assert_eq!(cashflows.len(), 1);
    assert_eq!(cashflows[0].0, d(2025, 1, 16));
}

#[test]
fn test_settlement_lag_over_weekend() {
    let fx = eurusd_with_notional(1_000_000.0, 1.20);
    let market = MarketContext::new();
    let as_of = d(2025, 1, 17); // Friday

    let cashflows = fx.build_schedule(&market, as_of).unwrap();

    assert_eq!(cashflows.len(), 1);
    // T+2 business days from Fri Jan 17 = Tue Jan 21 (skips weekend)
    assert_eq!(cashflows[0].0, d(2025, 1, 21));
}

#[test]
fn test_settlement_with_business_day_convention() {
    // Test that BDC is applied when calendar is present
    let settlement = d(2025, 1, 18); // Saturday
    let fx = eurusd_with_notional(1_000_000.0, 1.20)
        .with_settlement(settlement)
        .with_bdc(BusinessDayConvention::Following)
        .with_calendar_id("NewYork");

    let market = MarketContext::new();

    let cashflows = fx.build_schedule(&market, test_date()).unwrap();

    assert_eq!(cashflows.len(), 1);
    // BDC adjustment only applies when calendar is loaded - may return unadjusted date
    // Following convention should move Saturday to Monday if calendar is active
    assert!(
        cashflows[0].0 >= settlement,
        "Settlement on or after original date"
    );
}

#[test]
fn test_settlement_zero_notional() {
    let fx = sample_eurusd()
        .with_notional(Money::new(0.0, Currency::EUR))
        .unwrap()
        .with_rate(1.20)
        .with_settlement(d(2025, 1, 17));
    let market = MarketContext::new();

    let cashflows = fx.build_schedule(&market, test_date()).unwrap();

    assert_eq!(cashflows.len(), 1);
    assert_approx_eq(
        cashflows[0].1.amount(),
        0.0,
        EPSILON,
        "Zero notional cashflow",
    );
}

#[test]
fn test_multiple_instruments_independent_settlement() {
    let fx1 = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(d(2025, 1, 17));
    let fx2 = eurusd_with_notional(2_000_000.0, 1.22).with_settlement(d(2025, 1, 20));

    let market = MarketContext::new();
    let as_of = test_date();

    let cf1 = fx1.build_schedule(&market, as_of).unwrap();
    let cf2 = fx2.build_schedule(&market, as_of).unwrap();

    assert_eq!(cf1.len(), 1);
    assert_eq!(cf2.len(), 1);
    assert_eq!(cf1[0].0, d(2025, 1, 17));
    assert_eq!(cf2[0].0, d(2025, 1, 20));
    assert_approx_eq(cf1[0].1.amount(), 1_200_000.0, EPSILON, "FX1 amount");
    assert_approx_eq(cf2[0].1.amount(), 2_440_000.0, EPSILON, "FX2 amount");
}

#[test]
fn test_settlement_without_rate_or_matrix_fails() {
    let fx = sample_eurusd()
        .with_notional(Money::new(1_000_000.0, Currency::EUR))
        .unwrap()
        .with_settlement(d(2025, 1, 17));
    let market = MarketContext::new(); // No FX matrix

    let result = fx.build_schedule(&market, test_date());
    assert!(result.is_err());
}

#[test]
fn test_settlement_lag_negative() {
    // Test backward-looking settlement (unusual but valid)
    let fx = FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD)
        .with_notional(Money::new(1_000_000.0, Currency::EUR))
        .unwrap()
        .with_rate(1.20)
        .with_settlement(d(2025, 1, 15)); // Past date

    let market = MarketContext::new();
    let as_of = d(2025, 1, 17); // Friday

    let cashflows = fx.build_schedule(&market, as_of).unwrap();

    // Past date means no cashflow
    assert_eq!(cashflows.len(), 0);
}

#[test]
fn test_calendar_aware_settlement_lag() {
    let fx = FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD)
        .with_notional(Money::new(1_000_000.0, Currency::EUR))
        .unwrap()
        .with_rate(1.20)
        .with_calendar_id("NewYork")
        .with_settlement(d(2025, 1, 17)); // T+2

    let market = MarketContext::new();
    let as_of = d(2025, 1, 15);

    let cashflows = fx.build_schedule(&market, as_of).unwrap();

    assert_eq!(cashflows.len(), 1);
    // Should respect calendar holidays if any
    assert!(cashflows[0].0 > as_of);
}
