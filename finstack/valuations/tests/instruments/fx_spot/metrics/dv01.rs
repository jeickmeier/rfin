//! DV01 metric tests for FX Spot.
//!
//! FX Spot has no discount or forward curves, so generic DV01 returns 0.
//! These tests verify that FX Spot DV01 is consistently zero.

use super::super::common::*;
use finstack_core::types::InstrumentId;
use finstack_core::{currency::Currency, dates::Date, market_data::MarketContext, money::Money};
use finstack_valuations::{
    instruments::{
        common::traits::Instrument,
        fx_spot::FxSpot,
    },
    metrics::{traits::MetricCalculator, MetricContext, GenericParallelDv01},
};
use std::sync::Arc;

fn create_context(fx: FxSpot, as_of: Date) -> MetricContext {
    let market = MarketContext::new();
    let base_value = fx.npv(&market, as_of).unwrap();
    let instrument: Arc<dyn Instrument> = Arc::new(fx);
    MetricContext::new(instrument, Arc::new(market), as_of, base_value)
}

#[test]
fn test_dv01_basic() {
    let fx = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(d(2025, 1, 17)); // T+2
    let mut ctx = create_context(fx, test_date()); // 2025-01-15
    let calc = GenericParallelDv01::<FxSpot>::default();

    let dv01 = calc.calculate(&mut ctx).unwrap();

    // FX Spot has no discount curves, so generic DV01 returns 0
    assert_eq!(dv01, 0.0, "FX Spot DV01 should be 0");
}

#[test]
fn test_dv01_zero_after_settlement() {
    let fx = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(d(2025, 1, 10)); // Past date
    let mut ctx = create_context(fx, test_date());
    let calc = GenericParallelDv01::<FxSpot>::default();

    let dv01 = calc.calculate(&mut ctx).unwrap();
    assert_eq!(dv01, 0.0, "FX Spot DV01 should be 0");
}

#[test]
fn test_dv01_on_settlement_date() {
    let settlement = test_date();
    let fx = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(settlement);
    let mut ctx = create_context(fx, settlement);
    let calc = GenericParallelDv01::<FxSpot>::default();

    let dv01 = calc.calculate(&mut ctx).unwrap();
    assert_eq!(dv01, 0.0, "FX Spot DV01 should be 0");
}

#[test]
fn test_dv01_always_zero() {
    // FX Spot DV01 is always zero regardless of notional, settlement, or rate
    let calc = GenericParallelDv01::<FxSpot>::default();
    let settlement = d(2025, 1, 17);

    let fx1 = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(settlement);
    let fx2 = eurusd_with_notional(2_000_000.0, 1.20).with_settlement(settlement);

    let mut ctx1 = create_context(fx1, test_date());
    let mut ctx2 = create_context(fx2, test_date());

    let dv01_1 = calc.calculate(&mut ctx1).unwrap();
    let dv01_2 = calc.calculate(&mut ctx2).unwrap();

    assert_eq!(dv01_1, 0.0, "FX Spot DV01 should be 0");
    assert_eq!(dv01_2, 0.0, "FX Spot DV01 should be 0");
}

#[test]
fn test_dv01_independent_of_time() {
    // DV01 is always zero regardless of settlement time
    let calc = GenericParallelDv01::<FxSpot>::default();
    let as_of = test_date(); // 2025-01-15

    let fx1 = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(d(2025, 1, 17)); // T+2
    let fx2 = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(d(2025, 2, 14)); // T+30

    let mut ctx1 = create_context(fx1, as_of);
    let mut ctx2 = create_context(fx2, as_of);

    let dv01_1 = calc.calculate(&mut ctx1).unwrap();
    let dv01_2 = calc.calculate(&mut ctx2).unwrap();

    assert_eq!(dv01_1, 0.0, "FX Spot DV01 should be 0");
    assert_eq!(dv01_2, 0.0, "FX Spot DV01 should be 0");
}

#[test]
fn test_dv01_independent_of_rate() {
    // DV01 is zero regardless of rate
    let calc = GenericParallelDv01::<FxSpot>::default();
    let settlement = d(2025, 1, 17);

    let fx1 = eurusd_with_notional(1_000_000.0, 1.10).with_settlement(settlement);
    let fx2 = eurusd_with_notional(1_000_000.0, 1.50).with_settlement(settlement);

    let mut ctx1 = create_context(fx1, test_date());
    let mut ctx2 = create_context(fx2, test_date());

    let dv01_1 = calc.calculate(&mut ctx1).unwrap();
    let dv01_2 = calc.calculate(&mut ctx2).unwrap();

    assert_eq!(dv01_1, 0.0, "FX Spot DV01 should be 0");
    assert_eq!(dv01_2, 0.0, "FX Spot DV01 should be 0");
}

#[test]
fn test_dv01_default_settlement_lag() {
    // DV01 is zero even with default settlement lag
    let fx = eurusd_with_notional(1_000_000.0, 1.20);
    let mut ctx = create_context(fx, test_date());
    let calc = GenericParallelDv01::<FxSpot>::default();

    let dv01 = calc.calculate(&mut ctx).unwrap();
    assert_eq!(dv01, 0.0, "FX Spot DV01 should be 0");
}

#[test]
fn test_dv01_custom_settlement_lag() {
    // DV01 is zero regardless of settlement lag
    let calc = GenericParallelDv01::<FxSpot>::default();

    let fx1 = FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD)
        .try_with_notional(Money::new(1_000_000.0, Currency::EUR))
        .unwrap()
        .with_rate(1.20)
        .with_settlement(d(2025, 1, 16)); // T+1

    let fx2 = FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD)
        .try_with_notional(Money::new(1_000_000.0, Currency::EUR))
        .unwrap()
        .with_rate(1.20)
        .with_settlement(d(2025, 1, 20)); // T+5

    let mut ctx1 = create_context(fx1, test_date());
    let mut ctx2 = create_context(fx2, test_date());

    let dv01_1 = calc.calculate(&mut ctx1).unwrap();
    let dv01_2 = calc.calculate(&mut ctx2).unwrap();

    assert_eq!(dv01_1, 0.0, "FX Spot DV01 should be 0");
    assert_eq!(dv01_2, 0.0, "FX Spot DV01 should be 0");
}

#[test]
fn test_dv01_various_currencies() {
    // DV01 is zero for all currency pairs
    let calc = GenericParallelDv01::<FxSpot>::default();
    let settlement = d(2025, 1, 17);

    // All with same notional and settlement
    let eur_fx = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(settlement);
    let gbp_fx = sample_gbpusd()
        .try_with_notional(Money::new(1_000_000.0, Currency::GBP))
        .unwrap()
        .with_rate(1.40)
        .with_settlement(settlement);

    let mut eur_ctx = create_context(eur_fx, test_date());
    let mut gbp_ctx = create_context(gbp_fx, test_date());

    let dv01_eur = calc.calculate(&mut eur_ctx).unwrap();
    let dv01_gbp = calc.calculate(&mut gbp_ctx).unwrap();

    assert_eq!(dv01_eur, 0.0, "FX Spot DV01 should be 0");
    assert_eq!(dv01_gbp, 0.0, "FX Spot DV01 should be 0");
}

#[test]
fn test_dv01_zero_notional() {
    let fx = sample_eurusd()
        .try_with_notional(Money::new(0.0, Currency::EUR))
        .unwrap()
        .with_rate(1.20)
        .with_settlement(d(2025, 1, 17));
    let mut ctx = create_context(fx, test_date());
    let calc = GenericParallelDv01::<FxSpot>::default();

    let dv01 = calc.calculate(&mut ctx).unwrap();
    assert_eq!(dv01, 0.0, "FX Spot DV01 should be 0");
}

#[test]
fn test_dv01_large_notional() {
    let fx = eurusd_with_notional(1_000_000_000.0, 1.20).with_settlement(d(2025, 1, 17));
    let mut ctx = create_context(fx, test_date());
    let calc = GenericParallelDv01::<FxSpot>::default();

    let dv01 = calc.calculate(&mut ctx).unwrap();
    assert_eq!(dv01, 0.0, "FX Spot DV01 should be 0");
}

#[test]
fn test_dv01_one_year_maturity() {
    let fx = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(d(2026, 1, 15)); // 1 year
    let mut ctx = create_context(fx, test_date());
    let calc = GenericParallelDv01::<FxSpot>::default();

    let dv01 = calc.calculate(&mut ctx).unwrap();
    assert_eq!(dv01, 0.0, "FX Spot DV01 should be 0");
}
