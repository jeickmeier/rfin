#[cfg(feature = "scenarios")]
mod common;

#[cfg(feature = "scenarios")]
use crate::common::*;
#[cfg(feature = "scenarios")]
use finstack_core::prelude::*;
#[cfg(feature = "scenarios")]
use finstack_portfolio::types::Entity;
#[cfg(feature = "scenarios")]
use finstack_portfolio::{PortfolioBuilder, Position, PositionUnit};
#[cfg(feature = "scenarios")]
use finstack_scenarios::spec::{CurveKind, OperationSpec, ScenarioSpec};
#[cfg(feature = "scenarios")]
use finstack_valuations::instruments::deposit::Deposit;
#[cfg(feature = "scenarios")]
use std::sync::Arc;
#[cfg(feature = "scenarios")]
use time::Duration;

#[cfg(feature = "scenarios")]
#[test]
fn apply_and_revalue_succeeds() {
    let as_of = base_date();
    let end_date = as_of + Duration::days(30);

    let dep = Deposit::builder()
        .id("D".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(as_of)
        .end(end_date)
        .day_count(finstack_core::dates::DayCount::Act360)
        .discount_curve_id("USD".into())
        .build()
        .unwrap();

    let pos = Position::new("P", "E", "D", Arc::new(dep), 1.0, PositionUnit::Units);
    let portfolio = PortfolioBuilder::new("PF")
        .base_ccy(Currency::USD)
        .as_of(as_of)
        .entity(Entity::new("E"))
        .position(pos)
        .build()
        .unwrap();

    let market = market_with_usd();
    let config = FinstackConfig::default();

    let scenario = ScenarioSpec {
        id: "s".to_string(),
        name: Some("s".to_string()),
        description: None,
        operations: vec![OperationSpec::CurveParallelBp {
            curve_kind: CurveKind::Discount,
            curve_id: "USD".to_string(),
            bp: 10.0,
        }],
        priority: 0,
    };

    let (_valuation, report) =
        finstack_portfolio::apply_and_revalue(&portfolio, &scenario, &market, &config).unwrap();
    assert!(report.operations_applied > 0);
}
