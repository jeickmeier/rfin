//! Pricer integration tests for FX Spot.

use super::super::common::*;
use finstack_core::{currency::Currency, market_data::MarketContext, money::Money};
use finstack_valuations::{
    instruments::{common::traits::Instrument, fx_spot::SimpleFxSpotDiscountingPricer},
    pricer::{InstrumentType, ModelKey, Pricer},
};

#[test]
fn test_pricer_key() {
    let pricer = SimpleFxSpotDiscountingPricer::new();
    let key = pricer.key();

    assert_eq!(key.instrument, InstrumentType::FxSpot);
    assert_eq!(key.model, ModelKey::Discounting);
}

#[test]
fn test_pricer_price_dyn() {
    let fx = eurusd_with_notional(1_000_000.0, 1.20);
    let pricer = SimpleFxSpotDiscountingPricer::new();
    let market = MarketContext::new();

    let instrument: &dyn Instrument = &fx;
    let result = pricer.price_dyn(instrument, &market).unwrap();

    assert_eq!(result.instrument_id, "EURUSD");
    assert_approx_eq(result.value.amount(), 1_200_000.0, EPSILON, "PV");
}

#[test]
fn test_pricer_with_various_instruments() {
    let pricer = SimpleFxSpotDiscountingPricer::new();
    let market = MarketContext::new();

    let instruments: Vec<Box<dyn Instrument>> = vec![
        Box::new(eurusd_with_notional(1_000_000.0, 1.20)),
        Box::new(
            sample_gbpusd()
                .try_with_notional(Money::new(500_000.0, Currency::GBP))
                .unwrap()
                .with_rate(1.40),
        ),
        Box::new(
            sample_usdjpy()
                .try_with_notional(Money::new(100_000.0, Currency::USD))
                .unwrap()
                .with_rate(110.0),
        ),
    ];

    for inst in instruments {
        let result = pricer.price_dyn(inst.as_ref(), &market);
        assert!(result.is_ok(), "Pricer should price all FX instruments");
    }
}

// Disabled: test needs proper Deposit builder setup
// #[test]
// fn test_pricer_wrong_instrument_type_fails() {
//     // Test that the pricer rejects wrong instrument types
//     let pricer = SimpleFxSpotDiscountingPricer::new();
//     let market = MarketContext::new();
//
//     // TODO: Create a different instrument type properly
//     // Should fail with type mismatch
// }

#[test]
fn test_pricer_default_constructor() {
    let pricer = SimpleFxSpotDiscountingPricer;
    let key = pricer.key();

    assert_eq!(key.instrument, InstrumentType::FxSpot);
}

#[test]
fn test_pricer_consistent_with_instrument_value() {
    let fx = eurusd_with_notional(2_500_000.0, 1.22);
    let pricer = SimpleFxSpotDiscountingPricer::new();
    let market = MarketContext::new();

    // Price via pricer
    let pricer_result = pricer.price_dyn(&fx, &market).unwrap();

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
        .try_with_notional(Money::new(1_000_000.0, Currency::EUR))
        .unwrap();
    let pricer = SimpleFxSpotDiscountingPricer::new();
    let market = market_with_fx_matrix(); // EUR/USD = 1.20

    let result = pricer.price_dyn(&fx, &market).unwrap();

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
    let pricer = SimpleFxSpotDiscountingPricer::new();
    let market = MarketContext::new();

    let result = pricer.price_dyn(&fx, &market).unwrap();

    assert_eq!(result.instrument_id, "EURUSD");
    assert_eq!(result.value.currency(), Currency::USD);
    assert!(result.measures.is_empty(), "No metrics by default");
}
