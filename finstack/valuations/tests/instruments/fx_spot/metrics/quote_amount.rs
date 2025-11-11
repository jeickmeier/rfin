//! Quote amount metric tests.

use super::super::common::*;
use finstack_core::{currency::Currency, dates::Date, market_data::MarketContext, money::Money};
use finstack_valuations::{
    metrics::GenericPv,
    instruments::{
        common::traits::Instrument,
        fx_spot::FxSpot,
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
fn test_quote_amount_basic() {
    let fx = eurusd_with_notional(1_000_000.0, 1.20);
    let mut ctx = create_context(fx, test_date());
    let calc = GenericPv;

    let amount = calc.calculate(&mut ctx).unwrap();
    assert_approx_eq(amount, 1_200_000.0, EPSILON, "Quote amount");
}

#[test]
fn test_quote_amount_equals_base_value() {
    let fx = eurusd_with_notional(2_000_000.0, 1.22);
    let ctx = create_context(fx, test_date());
    let calc = GenericPv;

    let mut ctx_mut = ctx;
    let quote_amt = calc.calculate(&mut ctx_mut).unwrap();
    let base_value = ctx_mut.base_value.amount();

    assert_approx_eq(quote_amt, base_value, EPSILON, "Equals base_value");
}

#[test]
fn test_quote_amount_default_notional() {
    let fx = sample_eurusd().with_rate(1.18);
    let mut ctx = create_context(fx, test_date());
    let calc = GenericPv;

    let amount = calc.calculate(&mut ctx).unwrap();
    assert_approx_eq(amount, 1.18, EPSILON, "Default notional quote amount");
}

#[test]
fn test_quote_amount_various_rates() {
    let calc = GenericPv;

    let rates = vec![0.5, 1.0, 1.2, 1.5, 2.0, 100.0];
    let notional = 1_000_000.0;

    for rate in rates {
        let fx = eurusd_with_notional(notional, rate);
        let mut ctx = create_context(fx, test_date());

        let quote_amt = calc.calculate(&mut ctx).unwrap();
        let expected = notional * rate;

        assert_approx_eq(
            quote_amt,
            expected,
            EPSILON,
            &format!("Quote amount for rate {}", rate),
        );
    }
}

#[test]
fn test_quote_amount_various_currencies() {
    let calc = GenericPv;

    // GBPUSD
    let gbp_fx = sample_gbpusd()
        .try_with_notional(Money::new(500_000.0, Currency::GBP))
        .unwrap()
        .with_rate(1.40);
    let mut gbp_ctx = create_context(gbp_fx, test_date());
    assert_approx_eq(
        calc.calculate(&mut gbp_ctx).unwrap(),
        700_000.0,
        EPSILON,
        "GBP/USD quote amount",
    );

    // USDJPY
    let jpy_fx = sample_usdjpy()
        .try_with_notional(Money::new(100_000.0, Currency::USD))
        .unwrap()
        .with_rate(110.0);
    let mut jpy_ctx = create_context(jpy_fx, test_date());
    assert_approx_eq(
        calc.calculate(&mut jpy_ctx).unwrap(),
        11_000_000.0,
        EPSILON,
        "USD/JPY quote amount",
    );
}

#[test]
fn test_quote_amount_zero_notional() {
    let fx = sample_eurusd()
        .try_with_notional(Money::new(0.0, Currency::EUR))
        .unwrap()
        .with_rate(1.20);
    let mut ctx = create_context(fx, test_date());
    let calc = GenericPv;

    let amount = calc.calculate(&mut ctx).unwrap();
    assert_approx_eq(amount, 0.0, EPSILON, "Zero notional");
}

#[test]
fn test_quote_amount_large_notional() {
    let fx = eurusd_with_notional(1_000_000_000.0, 1.20);
    let mut ctx = create_context(fx, test_date());
    let calc = GenericPv;

    let amount = calc.calculate(&mut ctx).unwrap();
    assert_approx_eq(amount, 1_200_000_000.0, 1.0, "Large notional");
}

#[test]
fn test_quote_amount_independence_from_date() {
    let calc = GenericPv;
    let fx = eurusd_with_notional(1_000_000.0, 1.20);

    let mut ctx1 = create_context(fx.clone(), d(2025, 1, 15));
    let mut ctx2 = create_context(fx.clone(), d(2025, 6, 15));
    let mut ctx3 = create_context(fx, d(2026, 1, 15));

    let amount1 = calc.calculate(&mut ctx1).unwrap();
    let amount2 = calc.calculate(&mut ctx2).unwrap();
    let amount3 = calc.calculate(&mut ctx3).unwrap();

    // With explicit rate, quote amount should be date-independent
    assert_approx_eq(amount1, amount2, EPSILON, "Date independence 1");
    assert_approx_eq(amount1, amount3, EPSILON, "Date independence 2");
    assert_approx_eq(amount1, 1_200_000.0, EPSILON, "Expected value");
}

#[test]
fn test_quote_amount_conversion_relationship() {
    // Verify: quote_amount = base_amount * spot_rate
    use finstack_valuations::instruments::fx_spot::metrics::base_amount::BaseAmountCalculator;

    let fx = eurusd_with_notional(1_234_567.0, 1.23456);
    let mut ctx = create_context(fx.clone(), test_date());

    let base_calc = BaseAmountCalculator;
    let quote_calc = GenericPv;

    let base_amt = base_calc.calculate(&mut ctx).unwrap();
    let quote_amt = quote_calc.calculate(&mut ctx).unwrap();
    let spot_rate = fx.spot_rate.unwrap();

    let expected_quote = base_amt * spot_rate;
    assert_approx_eq(
        quote_amt,
        expected_quote,
        1e-2, // Relaxed tolerance for Decimal arithmetic
        "Conversion relationship",
    );
}
