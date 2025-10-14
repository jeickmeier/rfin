mod common;

use crate::common::*;
use finstack_core::prelude::*;
use finstack_portfolio::grouping::{
    aggregate_by_attribute, aggregate_by_multiple_attributes, group_by_attribute,
};
use finstack_portfolio::types::Entity;
use finstack_portfolio::{PortfolioBuilder, Position, PositionUnit};
use finstack_valuations::instruments::deposit::Deposit;
use std::sync::Arc;

#[test]
fn grouping_and_multi_attribute_aggregation() {
    let as_of = base_date();

    let dep1 = Deposit::builder()
        .id("D1".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(as_of)
        .end(as_of)
        .day_count(finstack_core::dates::DayCount::Act360)
        .disc_id("USD".into())
        .build()
        .unwrap();
    let dep2 = Deposit::builder()
        .id("D2".into())
        .notional(Money::new(500_000.0, Currency::USD))
        .start(as_of)
        .end(as_of)
        .day_count(finstack_core::dates::DayCount::Act360)
        .disc_id("USD".into())
        .build()
        .unwrap();

    let p1 = Position::new("P1", "E", "D1", Arc::new(dep1), 1.0, PositionUnit::Units)
        .with_tag("rating", "AAA")
        .with_tag("sector", "Banking");
    let p2 = Position::new("P2", "E", "D2", Arc::new(dep2), 1.0, PositionUnit::Units)
        .with_tag("rating", "AA");

    let portfolio = PortfolioBuilder::new("P")
        .base_ccy(Currency::USD)
        .as_of(as_of)
        .entity(Entity::new("E"))
        .positions(vec![p1, p2])
        .build()
        .unwrap();

    let market = market_with_usd();
    let config = FinstackConfig::default();
    let valuation = finstack_portfolio::value_portfolio(&portfolio, &market, &config).unwrap();

    let groups = group_by_attribute(&portfolio.positions, "rating");
    assert!(groups.contains_key("AAA") && groups.contains_key("AA"));
    assert!(!groups.contains_key("_untagged"));

    let agg =
        aggregate_by_attribute(&valuation, &portfolio.positions, "rating", Currency::USD).unwrap();
    assert!(agg.contains_key("AAA") && agg.contains_key("AA"));

    let agg2 = aggregate_by_multiple_attributes(
        &valuation,
        &portfolio.positions,
        &["rating", "sector"],
        Currency::USD,
    )
    .unwrap();
    assert!(!agg2.is_empty());
}

#[test]
fn dataframe_exports_have_expected_columns() {
    let as_of = base_date();

    let dep = Deposit::builder()
        .id("D".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(as_of)
        .end(as_of)
        .day_count(finstack_core::dates::DayCount::Act360)
        .disc_id("USD".into())
        .build()
        .unwrap();

    let p = Position::new("P", "E", "D", Arc::new(dep), 1.0, PositionUnit::Units);

    let portfolio = PortfolioBuilder::new("P")
        .base_ccy(Currency::USD)
        .as_of(as_of)
        .entity(Entity::new("E"))
        .position(p)
        .build()
        .unwrap();

    let market = market_with_usd();
    let config = FinstackConfig::default();
    let valuation = finstack_portfolio::value_portfolio(&portfolio, &market, &config).unwrap();
    let metrics = finstack_portfolio::aggregate_metrics(&valuation).unwrap();

    let df_pos = finstack_portfolio::dataframe::positions_to_dataframe(&valuation).unwrap();
    let df_ent = finstack_portfolio::dataframe::entities_to_dataframe(&valuation).unwrap();
    let df_m = finstack_portfolio::dataframe::metrics_to_dataframe(&metrics).unwrap();
    let df_ma = finstack_portfolio::dataframe::aggregated_metrics_to_dataframe(&metrics).unwrap();

    let pos_cols: Vec<&str> = df_pos
        .get_column_names()
        .iter()
        .map(|s| s.as_str())
        .collect();
    assert!(pos_cols.contains(&"position_id") && pos_cols.contains(&"value_base"));
    let ent_cols: Vec<&str> = df_ent
        .get_column_names()
        .iter()
        .map(|s| s.as_str())
        .collect();
    assert!(ent_cols.contains(&"entity_id") && ent_cols.contains(&"total_value"));
    let m_cols: Vec<&str> = df_m.get_column_names().iter().map(|s| s.as_str()).collect();
    assert!(
        m_cols.contains(&"metric_id")
            && m_cols.contains(&"position_id")
            && m_cols.contains(&"value")
    );
    let ma_cols: Vec<&str> = df_ma
        .get_column_names()
        .iter()
        .map(|s| s.as_str())
        .collect();
    assert!(ma_cols.contains(&"metric_id") && ma_cols.contains(&"total"));
}
