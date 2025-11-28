mod common;

use common::*;
use finstack_core::prelude::*;
use finstack_portfolio::types::Entity;
use finstack_portfolio::{PortfolioBuilder, Position, PositionUnit};
use finstack_valuations::instruments::deposit::Deposit;
use std::sync::Arc;
use time::Duration;

#[test]
fn test_position_spec_roundtrip() {
    let as_of = base_date();
    let end_date = as_of + Duration::days(30);

    let deposit = Deposit::builder()
        .id("DEP_1M".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(as_of)
        .end(end_date)
        .day_count(finstack_core::dates::DayCount::Act360)
        .discount_curve_id("USD".into())
        .build()
        .unwrap();

    let position = Position::new(
        "POS_001",
        "ENTITY_A",
        "DEP_1M",
        Arc::new(deposit),
        1.5,
        PositionUnit::Units,
    )
    .unwrap()
    .with_tag("rating", "AAA")
    .with_tag("sector", "Banking");

    // Convert to spec
    let spec = position.to_spec();

    // Verify spec contains all position data
    assert_eq!(spec.position_id, "POS_001");
    assert_eq!(spec.entity_id, "ENTITY_A");
    assert_eq!(spec.instrument_id, "DEP_1M");
    assert_eq!(spec.quantity, 1.5);
    assert_eq!(spec.unit, PositionUnit::Units);
    assert_eq!(spec.tags.get("rating"), Some(&"AAA".to_string()));
    assert_eq!(spec.tags.get("sector"), Some(&"Banking".to_string()));

    // Note: instrument_spec may be None if Deposit doesn't implement to_instrument_json()
    // This is expected until we implement the conversion for all instrument types
}

#[test]
fn test_portfolio_spec_serialization() {
    let as_of = base_date();
    let end_date = as_of + Duration::days(30);

    let dep1 = Deposit::builder()
        .id("DEP_1".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(as_of)
        .end(end_date)
        .day_count(finstack_core::dates::DayCount::Act360)
        .discount_curve_id("USD".into())
        .build()
        .unwrap();

    let dep2 = Deposit::builder()
        .id("DEP_2".into())
        .notional(Money::new(500_000.0, Currency::USD))
        .start(as_of)
        .end(end_date)
        .day_count(finstack_core::dates::DayCount::Act360)
        .discount_curve_id("USD".into())
        .build()
        .unwrap();

    let pos1 = Position::new(
        "POS_001",
        "ENTITY_A",
        "DEP_1",
        Arc::new(dep1),
        1.0,
        PositionUnit::Units,
    )
    .unwrap()
    .with_tag("rating", "AAA");

    let pos2 = Position::new(
        "POS_002",
        "ENTITY_B",
        "DEP_2",
        Arc::new(dep2),
        2.0,
        PositionUnit::Units,
    )
    .unwrap()
    .with_tag("rating", "AA");

    let portfolio = PortfolioBuilder::new("TEST_PORTFOLIO")
        .name("Test Portfolio")
        .base_ccy(Currency::USD)
        .as_of(as_of)
        .entity(Entity::new("ENTITY_A"))
        .entity(Entity::new("ENTITY_B"))
        .position(pos1)
        .position(pos2)
        .tag("strategy", "fixed_income")
        .build()
        .unwrap();

    // Convert to spec
    let spec = portfolio.to_spec();

    // Verify spec contains all portfolio data
    assert_eq!(spec.id, "TEST_PORTFOLIO");
    assert_eq!(spec.name, Some("Test Portfolio".to_string()));
    assert_eq!(spec.base_ccy, Currency::USD);
    assert_eq!(spec.as_of, as_of);
    assert_eq!(spec.entities.len(), 2);
    assert_eq!(spec.positions.len(), 2);
    assert_eq!(spec.tags.get("strategy"), Some(&"fixed_income".to_string()));

    // Verify positions are in spec
    assert_eq!(spec.positions[0].position_id, "POS_001");
    assert_eq!(spec.positions[1].position_id, "POS_002");
}

#[test]
fn test_portfolio_spec_json_roundtrip() {
    let as_of = base_date();
    let end_date = as_of + Duration::days(30);

    let deposit = Deposit::builder()
        .id("DEP_1M".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(as_of)
        .end(end_date)
        .day_count(finstack_core::dates::DayCount::Act360)
        .discount_curve_id("USD".into())
        .build()
        .unwrap();

    let position = Position::new(
        "POS_001",
        "ENTITY_A",
        "DEP_1M",
        Arc::new(deposit),
        1.0,
        PositionUnit::Units,
    )
    .unwrap();

    let portfolio = PortfolioBuilder::new("TEST")
        .base_ccy(Currency::USD)
        .as_of(as_of)
        .entity(Entity::new("ENTITY_A"))
        .position(position)
        .build()
        .unwrap();

    // Convert to spec and serialize to JSON
    let spec = portfolio.to_spec();
    let json = serde_json::to_string(&spec).expect("Serialization should succeed");

    // Deserialize back
    let spec_roundtrip: finstack_portfolio::portfolio::PortfolioSpec =
        serde_json::from_str(&json).expect("Deserialization should succeed");

    // Verify roundtrip preserved data
    assert_eq!(spec_roundtrip.id, spec.id);
    assert_eq!(spec_roundtrip.base_ccy, spec.base_ccy);
    assert_eq!(spec_roundtrip.as_of, spec.as_of);
    assert_eq!(spec_roundtrip.positions.len(), spec.positions.len());

    // Note: Full reconstruction (from_spec) requires instrument_spec to be Some
    // This will work once we implement to_instrument_json() for all instrument types
}
