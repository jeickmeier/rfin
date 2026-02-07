mod common;

use common::*;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_portfolio::types::{Entity, DUMMY_ENTITY_ID};
use finstack_portfolio::{Portfolio, PortfolioBuilder, PortfolioError, Position, PositionUnit};
use finstack_valuations::instruments::rates::deposit::Deposit;
use std::sync::Arc;

#[test]
fn getters_and_tag_filters() {
    let as_of = base_date();

    let dep = Deposit::builder()
        .id("D".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(as_of)
        .end(as_of)
        .day_count(finstack_core::dates::DayCount::Act360)
        .discount_curve_id("USD".into())
        .build()
        .unwrap();

    let p = Position::new("P", "E", "D", Arc::new(dep), 1.0, PositionUnit::Units)
        .unwrap()
        .with_tag("sector", "Tech");

    let portfolio = PortfolioBuilder::new("P")
        .base_ccy(Currency::USD)
        .as_of(as_of)
        .entity(Entity::new("E"))
        .position(p)
        .build()
        .unwrap();

    assert!(portfolio.get_position("P").is_some());
    assert_eq!(portfolio.positions_for_entity("E").len(), 1);
    assert_eq!(portfolio.positions_with_tag("sector", "Tech").len(), 1);
}

#[test]
fn validate_unknown_entity_fails() {
    let as_of = base_date();

    let dep = Deposit::builder()
        .id("D".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(as_of)
        .end(as_of)
        .day_count(finstack_core::dates::DayCount::Act360)
        .discount_curve_id("USD".into())
        .build()
        .unwrap();

    let p = Position::new("P", "UNKNOWN", "D", Arc::new(dep), 1.0, PositionUnit::Units).unwrap();

    let mut portfolio = Portfolio::new("P", Currency::USD, as_of);
    portfolio.positions.push(p);
    portfolio.rebuild_index();

    let err = portfolio.validate().unwrap_err();
    match err {
        PortfolioError::UnknownEntity { .. } => {}
        other => panic!("unexpected error: {:?}", other),
    }
}

#[test]
fn builder_required_fields_and_dummy_auto_create() {
    let as_of = base_date();

    let dep = Deposit::builder()
        .id("D".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(as_of)
        .end(as_of)
        .day_count(finstack_core::dates::DayCount::Act360)
        .discount_curve_id("USD".into())
        .build()
        .unwrap();

    let p = Position::new(
        "P",
        DUMMY_ENTITY_ID,
        "D",
        Arc::new(dep),
        1.0,
        PositionUnit::Units,
    )
    .unwrap();

    // Missing base_ccy
    assert!(PortfolioBuilder::new("P").as_of(as_of).build().is_err());
    // Missing as_of
    assert!(PortfolioBuilder::new("P")
        .base_ccy(Currency::USD)
        .build()
        .is_err());

    // Dummy should be auto-created because position references it
    let portfolio = PortfolioBuilder::new("P")
        .base_ccy(Currency::USD)
        .as_of(as_of)
        .position(p)
        .build()
        .unwrap();
    assert!(portfolio.has_dummy_entity());
}
