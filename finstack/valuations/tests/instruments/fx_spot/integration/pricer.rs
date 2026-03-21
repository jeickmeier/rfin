//! Pricer integration tests for FX Spot.

use super::super::common::*;
use finstack_core::{currency::Currency, market_data::context::MarketContext, money::Money};
use finstack_valuations::{
    instruments::equity::Equity,
    instruments::{fx::fx_spot::FxSpotPricer, internal::InstrumentExt as Instrument},
    pricer::{InstrumentType, ModelKey, Pricer},
};

#[test]
fn test_pricer_key() {
    let pricer = FxSpotPricer::new();
    let key = pricer.key();

    assert_eq!(key.instrument, InstrumentType::FxSpot);
    assert_eq!(key.model, ModelKey::Discounting);
}

#[test]
fn test_pricer_price_dyn() {
    let fx = eurusd_with_notional(1_000_000.0, 1.20);
    let pricer = FxSpotPricer::new();
    let market = MarketContext::new();
    let as_of =
        finstack_core::dates::Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

    let instrument: &dyn Instrument = &fx;
    let result = pricer.price_dyn(instrument, &market, as_of).unwrap();

    assert_eq!(result.instrument_id, "EURUSD");
    assert_approx_eq(result.value.amount(), 1_200_000.0, EPSILON, "PV");
}

#[test]
fn test_pricer_with_various_instruments() {
    let pricer = FxSpotPricer::new();
    let market = MarketContext::new();

    let instruments: Vec<Box<dyn Instrument>> = vec![
        Box::new(eurusd_with_notional(1_000_000.0, 1.20)),
        Box::new(
            sample_gbpusd()
                .with_notional(Money::new(500_000.0, Currency::GBP))
                .unwrap()
                .with_rate(1.40)
                .expect("test rate"),
        ),
        Box::new(
            sample_usdjpy()
                .with_notional(Money::new(100_000.0, Currency::USD))
                .unwrap()
                .with_rate(110.0)
                .expect("test rate"),
        ),
    ];

    let as_of =
        finstack_core::dates::Date::from_calendar_date(2024, time::Month::January, 1).unwrap();
    for inst in instruments {
        let result = pricer.price_dyn(inst.as_ref(), &market, as_of);
        assert!(result.is_ok(), "Pricer should price all FX instruments");
    }
}

#[test]
fn test_pricer_wrong_instrument_type_fails() {
    let pricer = FxSpotPricer::new();
    let market = MarketContext::new();
    let as_of =
        finstack_core::dates::Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

    let equity = Equity::new("AAPL", "AAPL", Currency::USD)
        .with_price(100.0)
        .with_shares(10.0);

    let result = pricer.price_dyn(&equity, &market, as_of);
    assert!(result.is_err(), "Pricer should reject non-FX instruments");
}

#[test]
fn test_pricer_default_constructor() {
    let pricer = FxSpotPricer;
    let key = pricer.key();

    assert_eq!(key.instrument, InstrumentType::FxSpot);
}

#[test]
fn test_pricer_consistent_with_instrument_value() {
    let fx = eurusd_with_notional(2_500_000.0, 1.22);
    let pricer = FxSpotPricer::new();
    let market = MarketContext::new();
    let as_of =
        finstack_core::dates::Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

    // Price via pricer
    let pricer_result = pricer.price_dyn(&fx, &market, as_of).unwrap();

    // Price via instrument
    let instrument_value = fx.value(&market, test_date()).unwrap();

    assert_approx_eq(
        pricer_result.value.amount(),
        instrument_value.amount(),
        EPSILON,
        "Pricer and instrument values match",
    );
}

#[test]
fn test_pricer_with_fx_matrix() {
    let fx = sample_eurusd()
        .with_notional(Money::new(1_000_000.0, Currency::EUR))
        .unwrap();
    let pricer = FxSpotPricer::new();
    let market = market_with_fx_matrix(); // EUR/USD = 1.20
    let as_of =
        finstack_core::dates::Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

    let result = pricer.price_dyn(&fx, &market, as_of).unwrap();

    assert_approx_eq(
        result.value.amount(),
        1_200_000.0,
        LARGE_EPSILON,
        "FX matrix pricing",
    );
}

#[test]
fn test_pricer_valuation_result_structure() {
    let fx = eurusd_with_notional(1_000_000.0, 1.20);
    let pricer = FxSpotPricer::new();
    let market = MarketContext::new();
    let as_of =
        finstack_core::dates::Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

    let result = pricer.price_dyn(&fx, &market, as_of).unwrap();

    assert_eq!(result.instrument_id, "EURUSD");
    assert_eq!(result.value.currency(), Currency::USD);
    assert!(result.measures.is_empty(), "No metrics by default");
}
