mod common;

use crate::common::*;
use finstack_core::prelude::*;
use finstack_portfolio::types::{Entity, DUMMY_ENTITY_ID};
use finstack_portfolio::{PortfolioBuilder, PortfolioError, Position, PositionUnit};
use finstack_valuations::instruments::deposit::Deposit;
use std::sync::Arc;

#[test]
fn cross_currency_conversion_uses_fx_matrix() {
    let as_of = base_date();

    // EUR deposit valued with USD base; FX = 1.10 EUR→USD
    let dep = Deposit::builder()
        .id("DEP_EUR".into())
        .notional(Money::new(1_000_000.0, Currency::EUR))
        .start(as_of)
        .end(as_of)
        .day_count(finstack_core::dates::DayCount::Act360)
        .disc_id("EUR".into())
        .build()
        .unwrap();

    let position = Position::new(
        "POS_EUR",
        DUMMY_ENTITY_ID,
        "DEP_EUR",
        Arc::new(dep),
        1.0,
        PositionUnit::Units,
    );

    let portfolio = PortfolioBuilder::new("P")
        .base_ccy(Currency::USD)
        .as_of(as_of)
        .position(position)
        .build()
        .unwrap();

    // Market with EUR curve and FX
    let market = market_with_eur_and_fx(1.10);
    // Sanity: base currency is USD
    assert_eq!(portfolio.base_ccy, Currency::USD);

    let config = FinstackConfig::default();
    let valuation = finstack_portfolio::value_portfolio(&portfolio, &market, &config).unwrap();

    // With zero-rate curve and same-day start/end, PV ~= notional; FX applied to convert to USD
    let pos_val = valuation.get_position_value("POS_EUR").unwrap();
    assert_eq!(pos_val.value_native.currency(), Currency::EUR);
    assert_eq!(pos_val.value_base.currency(), Currency::USD);
}

#[test]
fn missing_fx_matrix_errors_for_cross_currency() {
    let as_of = base_date();

    // EUR deposit, portfolio base USD, but no FX in market
    let dep = Deposit::builder()
        .id("DEP_EUR".into())
        .notional(Money::new(1_000_000.0, Currency::EUR))
        .start(as_of)
        .end(as_of)
        .day_count(finstack_core::dates::DayCount::Act360)
        .disc_id("EUR".into())
        .build()
        .unwrap();

    let position = Position::new(
        "POS_EUR",
        DUMMY_ENTITY_ID,
        "DEP_EUR",
        Arc::new(dep),
        1.0,
        PositionUnit::Units,
    );

    let portfolio = PortfolioBuilder::new("P")
        .base_ccy(Currency::USD)
        .as_of(as_of)
        .position(position)
        .build()
        .unwrap();

    // Market has only EUR curve, no FX
    let market = market_with_eur();
    let config = FinstackConfig::default();
    let err = finstack_portfolio::value_portfolio(&portfolio, &market, &config).unwrap_err();

    match err {
        PortfolioError::MissingMarketData(msg) => assert!(msg.contains("FX")),
        other => panic!("unexpected error: {:?}", other),
    }
}

#[test]
fn quantity_scaling_and_entity_totals() {
    let as_of = base_date();

    let dep = Deposit::builder()
        .id("DEP_USD".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(as_of)
        .end(as_of)
        .day_count(finstack_core::dates::DayCount::Act360)
        .disc_id("USD".into())
        .build()
        .unwrap();

    // Short position (negative quantity)
    let position = Position::new(
        "POS_SHORT",
        "E1",
        "DEP_USD",
        Arc::new(dep),
        -2.0,
        PositionUnit::Units,
    );

    let portfolio = PortfolioBuilder::new("P")
        .base_ccy(Currency::USD)
        .as_of(as_of)
        .entity(Entity::new("E1"))
        .position(position)
        .build()
        .unwrap();

    let market = market_with_usd();
    let config = FinstackConfig::default();
    let valuation = finstack_portfolio::value_portfolio(&portfolio, &market, &config).unwrap();

    let pv = valuation.get_position_value("POS_SHORT").unwrap();
    assert!(pv.value_native.amount().is_sign_negative());
    assert!(valuation.get_entity_value(&"E1".to_string()).is_some());
}
