//! DV01 metric tests for FX Spot.

use super::super::common::*;
use finstack_core::types::InstrumentId;
use finstack_core::{currency::Currency, dates::Date, market_data::MarketContext, money::Money};
use finstack_valuations::{
    instruments::{
        common::traits::Instrument,
        fx_spot::{metrics::dv01::FxSpotDv01Calculator, FxSpot},
    },
    metrics::{traits::MetricCalculator, MetricContext},
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
    let calc = FxSpotDv01Calculator;

    let dv01 = calc.calculate(&mut ctx).unwrap();

    // DV01 = notional * time_to_settlement * 1bp
    // Time to settlement ≈ 2/360 (Act/360)
    // DV01 ≈ 1_000_000 * (2/360) * 0.0001 ≈ 0.5556
    assert!(dv01 > 0.0, "DV01 should be positive");
    assert_approx_eq(dv01, 0.5556, 0.01, "DV01 approximation");
}

#[test]
fn test_dv01_zero_after_settlement() {
    let fx = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(d(2025, 1, 10)); // Past date
    let mut ctx = create_context(fx, test_date());
    let calc = FxSpotDv01Calculator;

    let dv01 = calc.calculate(&mut ctx).unwrap();
    assert_approx_eq(dv01, 0.0, EPSILON, "DV01 zero after settlement");
}

#[test]
fn test_dv01_on_settlement_date() {
    let settlement = test_date();
    let fx = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(settlement);
    let mut ctx = create_context(fx, settlement);
    let calc = FxSpotDv01Calculator;

    let dv01 = calc.calculate(&mut ctx).unwrap();
    assert_approx_eq(dv01, 0.0, EPSILON, "DV01 zero on settlement");
}

#[test]
fn test_dv01_scales_with_notional() {
    let calc = FxSpotDv01Calculator;
    let settlement = d(2025, 1, 17);

    let fx1 = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(settlement);
    let fx2 = eurusd_with_notional(2_000_000.0, 1.20).with_settlement(settlement);
    let fx3 = eurusd_with_notional(5_000_000.0, 1.20).with_settlement(settlement);

    let mut ctx1 = create_context(fx1, test_date());
    let mut ctx2 = create_context(fx2, test_date());
    let mut ctx3 = create_context(fx3, test_date());

    let dv01_1 = calc.calculate(&mut ctx1).unwrap();
    let dv01_2 = calc.calculate(&mut ctx2).unwrap();
    let dv01_3 = calc.calculate(&mut ctx3).unwrap();

    // DV01 should scale linearly with notional
    assert_approx_eq(dv01_2 / dv01_1, 2.0, 0.01, "2x notional => 2x DV01");
    assert_approx_eq(dv01_3 / dv01_1, 5.0, 0.01, "5x notional => 5x DV01");
}

#[test]
fn test_dv01_scales_with_time() {
    let calc = FxSpotDv01Calculator;
    let as_of = test_date(); // 2025-01-15

    let fx1 = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(d(2025, 1, 17)); // T+2
    let fx2 = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(d(2025, 2, 14)); // T+30
    let fx3 = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(d(2025, 7, 15)); // T+180

    let mut ctx1 = create_context(fx1, as_of);
    let mut ctx2 = create_context(fx2, as_of);
    let mut ctx3 = create_context(fx3, as_of);

    let dv01_1 = calc.calculate(&mut ctx1).unwrap();
    let dv01_2 = calc.calculate(&mut ctx2).unwrap();
    let dv01_3 = calc.calculate(&mut ctx3).unwrap();

    // Longer time => larger DV01
    assert!(dv01_2 > dv01_1, "T+30 DV01 > T+2 DV01");
    assert!(dv01_3 > dv01_2, "T+180 DV01 > T+30 DV01");

    // Approximate scaling: DV01 ∝ time
    assert_approx_eq(dv01_2 / dv01_1, 30.0 / 2.0, 1.0, "30-day vs 2-day");
    assert_approx_eq(dv01_3 / dv01_1, 180.0 / 2.0, 5.0, "180-day vs 2-day");
}

#[test]
fn test_dv01_independent_of_rate() {
    // DV01 depends on notional and time, not on FX rate
    let calc = FxSpotDv01Calculator;
    let settlement = d(2025, 1, 17);

    let fx1 = eurusd_with_notional(1_000_000.0, 1.10).with_settlement(settlement);
    let fx2 = eurusd_with_notional(1_000_000.0, 1.50).with_settlement(settlement);

    let mut ctx1 = create_context(fx1, test_date());
    let mut ctx2 = create_context(fx2, test_date());

    let dv01_1 = calc.calculate(&mut ctx1).unwrap();
    let dv01_2 = calc.calculate(&mut ctx2).unwrap();

    assert_approx_eq(dv01_1, dv01_2, 0.001, "DV01 independent of rate");
}

#[test]
fn test_dv01_default_settlement_lag() {
    // Without explicit settlement, should use default T+2
    let fx = eurusd_with_notional(1_000_000.0, 1.20);
    let mut ctx = create_context(fx, test_date());
    let calc = FxSpotDv01Calculator;

    let dv01 = calc.calculate(&mut ctx).unwrap();
    assert!(dv01 > 0.0, "DV01 positive with default lag");
}

#[test]
fn test_dv01_custom_settlement_lag() {
    let calc = FxSpotDv01Calculator;

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

    // T+5 should have higher DV01 than T+1
    assert!(dv01_2 > dv01_1, "Longer lag => higher DV01");
}

#[test]
fn test_dv01_various_currencies() {
    let calc = FxSpotDv01Calculator;
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

    // Same notional and time => same DV01
    assert_approx_eq(dv01_eur, dv01_gbp, 0.001, "DV01 currency independence");
}

#[test]
fn test_dv01_zero_notional() {
    let fx = sample_eurusd()
        .try_with_notional(Money::new(0.0, Currency::EUR))
        .unwrap()
        .with_rate(1.20)
        .with_settlement(d(2025, 1, 17));
    let mut ctx = create_context(fx, test_date());
    let calc = FxSpotDv01Calculator;

    let dv01 = calc.calculate(&mut ctx).unwrap();
    assert_approx_eq(dv01, 0.0, EPSILON, "Zero notional => zero DV01");
}

#[test]
fn test_dv01_large_notional() {
    let fx = eurusd_with_notional(1_000_000_000.0, 1.20).with_settlement(d(2025, 1, 17));
    let mut ctx = create_context(fx, test_date());
    let calc = FxSpotDv01Calculator;

    let dv01 = calc.calculate(&mut ctx).unwrap();
    assert!(dv01 > 0.0, "DV01 positive for large notional");
    // Should be roughly 1000x the DV01 for 1M notional
    assert_approx_eq(dv01, 555.6, 10.0, "Large notional DV01");
}

#[test]
fn test_dv01_one_year_maturity() {
    let fx = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(d(2026, 1, 15)); // 1 year
    let mut ctx = create_context(fx, test_date());
    let calc = FxSpotDv01Calculator;

    let dv01 = calc.calculate(&mut ctx).unwrap();

    // DV01 = 1M * 1 year * 1bp = 1M * 1.0 * 0.0001 = 100
    assert_approx_eq(dv01, 100.0, 5.0, "One year DV01");
}
