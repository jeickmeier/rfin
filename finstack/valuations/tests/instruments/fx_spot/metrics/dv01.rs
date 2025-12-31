//! DV01 metric tests for FX Spot.
//!
//! FX Spot has no discount or forward curves, so generic DV01 returns 0.
//! These tests verify that FX Spot DV01 is consistently zero.

use super::super::common::*;
use finstack_core::types::InstrumentId;
use finstack_core::{
    currency::Currency, dates::Date, market_data::context::MarketContext, money::Money,
};
use finstack_valuations::{
    instruments::{FxSpot, Instrument},
    metrics::MetricId,
};

fn dv01_for(fx: FxSpot, as_of: Date) -> f64 {
    let market = MarketContext::new();
    let result = fx
        .price_with_metrics(&market, as_of, &[MetricId::Dv01])
        .expect("pricing with dv01 should succeed");
    *result.measures.get(MetricId::Dv01.as_str()).unwrap_or(&0.0)
}

#[test]
fn test_dv01_basic() {
    let fx = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(d(2025, 1, 17)); // T+2
    let dv01 = dv01_for(fx, test_date()); // 2025-01-15

    // FX Spot has no discount curves, so generic DV01 returns 0
    assert_eq!(dv01, 0.0, "FX Spot DV01 should be 0");
}

#[test]
fn test_dv01_zero_after_settlement() {
    let fx = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(d(2025, 1, 10)); // Past date
    let dv01 = dv01_for(fx, test_date());
    assert_eq!(dv01, 0.0, "FX Spot DV01 should be 0");
}

#[test]
fn test_dv01_on_settlement_date() {
    let settlement = test_date();
    let fx = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(settlement);
    let dv01 = dv01_for(fx, settlement);
    assert_eq!(dv01, 0.0, "FX Spot DV01 should be 0");
}

#[test]
fn test_dv01_always_zero() {
    // FX Spot DV01 is always zero regardless of notional, settlement, or rate
    let settlement = d(2025, 1, 17);

    let fx1 = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(settlement);
    let fx2 = eurusd_with_notional(2_000_000.0, 1.20).with_settlement(settlement);

    let dv01_1 = dv01_for(fx1, test_date());
    let dv01_2 = dv01_for(fx2, test_date());

    assert_eq!(dv01_1, 0.0, "FX Spot DV01 should be 0");
    assert_eq!(dv01_2, 0.0, "FX Spot DV01 should be 0");
}

#[test]
fn test_dv01_independent_of_time() {
    // DV01 is always zero regardless of settlement time
    let as_of = test_date(); // 2025-01-15

    let fx1 = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(d(2025, 1, 17)); // T+2
    let fx2 = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(d(2025, 2, 14)); // T+30

    let dv01_1 = dv01_for(fx1, as_of);
    let dv01_2 = dv01_for(fx2, as_of);

    assert_eq!(dv01_1, 0.0, "FX Spot DV01 should be 0");
    assert_eq!(dv01_2, 0.0, "FX Spot DV01 should be 0");
}

#[test]
fn test_dv01_independent_of_rate() {
    // DV01 is zero regardless of rate
    let settlement = d(2025, 1, 17);

    let fx1 = eurusd_with_notional(1_000_000.0, 1.10).with_settlement(settlement);
    let fx2 = eurusd_with_notional(1_000_000.0, 1.50).with_settlement(settlement);

    let dv01_1 = dv01_for(fx1, test_date());
    let dv01_2 = dv01_for(fx2, test_date());

    assert_eq!(dv01_1, 0.0, "FX Spot DV01 should be 0");
    assert_eq!(dv01_2, 0.0, "FX Spot DV01 should be 0");
}

#[test]
fn test_dv01_default_settlement_lag() {
    // DV01 is zero even with default settlement lag
    let fx = eurusd_with_notional(1_000_000.0, 1.20);
    let dv01 = dv01_for(fx, test_date());
    assert_eq!(dv01, 0.0, "FX Spot DV01 should be 0");
}

#[test]
fn test_dv01_custom_settlement_lag() {
    // DV01 is zero regardless of settlement lag
    let fx1 = FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD)
        .with_notional(Money::new(1_000_000.0, Currency::EUR))
        .unwrap()
        .with_rate(1.20)
        .with_settlement(d(2025, 1, 16)); // T+1

    let fx2 = FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD)
        .with_notional(Money::new(1_000_000.0, Currency::EUR))
        .unwrap()
        .with_rate(1.20)
        .with_settlement(d(2025, 1, 20)); // T+5

    let dv01_1 = dv01_for(fx1, test_date());
    let dv01_2 = dv01_for(fx2, test_date());

    assert_eq!(dv01_1, 0.0, "FX Spot DV01 should be 0");
    assert_eq!(dv01_2, 0.0, "FX Spot DV01 should be 0");
}

#[test]
fn test_dv01_various_currencies() {
    // DV01 is zero for all currency pairs
    let settlement = d(2025, 1, 17);

    // All with same notional and settlement
    let eur_fx = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(settlement);
    let gbp_fx = sample_gbpusd()
        .with_notional(Money::new(1_000_000.0, Currency::GBP))
        .unwrap()
        .with_rate(1.40)
        .with_settlement(settlement);

    let dv01_eur = dv01_for(eur_fx, test_date());
    let dv01_gbp = dv01_for(gbp_fx, test_date());

    assert_eq!(dv01_eur, 0.0, "FX Spot DV01 should be 0");
    assert_eq!(dv01_gbp, 0.0, "FX Spot DV01 should be 0");
}

#[test]
fn test_dv01_zero_notional() {
    let fx = sample_eurusd()
        .with_notional(Money::new(0.0, Currency::EUR))
        .unwrap()
        .with_rate(1.20)
        .with_settlement(d(2025, 1, 17));
    let dv01 = dv01_for(fx, test_date());
    assert_eq!(dv01, 0.0, "FX Spot DV01 should be 0");
}

#[test]
fn test_dv01_large_notional() {
    let fx = eurusd_with_notional(1_000_000_000.0, 1.20).with_settlement(d(2025, 1, 17));
    let dv01 = dv01_for(fx, test_date());
    assert_eq!(dv01, 0.0, "FX Spot DV01 should be 0");
}

#[test]
fn test_dv01_one_year_maturity() {
    let fx = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(d(2026, 1, 15)); // 1 year
    let dv01 = dv01_for(fx, test_date());
    assert_eq!(dv01, 0.0, "FX Spot DV01 should be 0");
}
