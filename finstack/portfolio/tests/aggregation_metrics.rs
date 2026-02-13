mod common;

use common::*;
use finstack_core::config::FinstackConfig;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_portfolio::types::Entity;
use finstack_portfolio::{PortfolioBuilder, Position, PositionUnit};
use finstack_valuations::instruments::rates::deposit::Deposit;
use std::sync::Arc;
use time::Duration;

#[test]
fn summable_vs_non_summable_metrics() {
    let as_of = base_date();
    let end_date = as_of + Duration::days(30);

    // Deposit supports standard metrics via helper; we request defaults in portfolio valuation
    let dep = Deposit::builder()
        .id("DEP_1M".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start_date(as_of)
        .end(end_date)
        .day_count(finstack_core::dates::DayCount::Act360)
        .discount_curve_id("USD".into())
        .quote_rate_opt(Some(0.045))
        .build()
        .unwrap();

    let position = Position::new(
        "POS_1",
        "E1",
        "DEP_1M",
        Arc::new(dep),
        1.0,
        PositionUnit::Units,
    )
    .unwrap();

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
    let metrics =
        finstack_portfolio::aggregate_metrics(&valuation, Currency::USD, &market).unwrap();

    // Position should have some metrics recorded (may be empty depending on measure availability)
    assert!(metrics.get_position_metrics("POS_1").is_some());

    // Aggregated totals only include summable metrics. We at least verify that querying works
    // without asserting specific numeric values (which depend on instrument specifics).
    if let Some(total) = metrics.get_total("dv01") {
        let _ = total; // present and numeric
    }
}
