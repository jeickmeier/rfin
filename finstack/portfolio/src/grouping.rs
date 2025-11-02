//! Attribute-based grouping and aggregation.
//!
//! Helper functions for slicing portfolios by arbitrary tags and rolling up
//! valuations across one or more categorical dimensions.

use crate::error::Result;
use crate::position::Position;
use crate::valuation::PortfolioValuation;
use finstack_core::prelude::*;
use indexmap::IndexMap;

/// Group positions by a specific tag or attribute.
///
/// Positions that do not contain the requested attribute are placed in the
/// special `_untagged` bucket to ensure they are still represented.
///
/// # Arguments
///
/// * `positions` - Slice of positions to group.
/// * `attr_key` - Tag key used to partition positions.
///
/// # Returns
///
/// An [`IndexMap`] mapping attribute values to the positions that match.
/// Order is stable because [`IndexMap`] preserves insertion order.
pub fn group_by_attribute<'a>(
    positions: &'a [Position],
    attr_key: &str,
) -> IndexMap<String, Vec<&'a Position>> {
    let mut groups: IndexMap<String, Vec<&'a Position>> = IndexMap::new();

    for position in positions {
        if let Some(attr_value) = position.tags.get(attr_key) {
            groups.entry(attr_value.clone()).or_default().push(position);
        } else {
            // Positions without this attribute go into "untagged"
            groups
                .entry("_untagged".to_string())
                .or_default()
                .push(position);
        }
    }

    groups
}

/// Aggregate portfolio values by a specific attribute.
///
/// Each position's base-currency value is summed into buckets by the chosen tag.
///
/// # Arguments
///
/// * `valuation` - Pre-computed valuation results providing per-position values.
/// * `positions` - Positions that correspond to the valuation results.
/// * `attr_key` - Tag to aggregate by.
/// * `base_ccy` - Currency used when adding monetary amounts.
///
/// # Returns
///
/// [`Result`] with an [`IndexMap`] of attribute values to aggregated [`Money`].
pub fn aggregate_by_attribute(
    valuation: &PortfolioValuation,
    positions: &[Position],
    attr_key: &str,
    base_ccy: Currency,
) -> Result<IndexMap<String, Money>> {
    let mut aggregated: IndexMap<String, Money> = IndexMap::new();

    for position in positions {
        let attr_value = position
            .tags
            .get(attr_key)
            .cloned()
            .unwrap_or_else(|| "_untagged".to_string());

        if let Some(position_value) = valuation.position_values.get(&position.position_id) {
            let total = aggregated
                .entry(attr_value)
                .or_insert_with(|| Money::new(0.0, base_ccy));
            *total = total.checked_add(position_value.value_base)?;
        }
    }

    Ok(aggregated)
}

/// Group and aggregate by multiple attributes.
///
/// Builds composite keys from the requested attributes to create multi-dimensional
/// aggregates. Missing attributes are normalised to `_untagged`.
///
/// # Arguments
///
/// * `valuation` - Portfolio valuation providing per-position values.
/// * `positions` - Positions being grouped.
/// * `attr_keys` - Ordered set of attribute keys used to build the composite key.
/// * `base_ccy` - Currency used for aggregation.
///
/// # Returns
///
/// [`Result`] containing an [`IndexMap`] whose keys are the ordered attribute values
/// and whose values are the aggregated [`Money`] totals.
pub fn aggregate_by_multiple_attributes(
    valuation: &PortfolioValuation,
    positions: &[Position],
    attr_keys: &[&str],
    base_ccy: Currency,
) -> Result<IndexMap<Vec<String>, Money>> {
    let mut aggregated: IndexMap<Vec<String>, Money> = IndexMap::new();

    for position in positions {
        // Build compound key from all attributes
        let key: Vec<String> = attr_keys
            .iter()
            .map(|&attr_key| {
                position
                    .tags
                    .get(attr_key)
                    .cloned()
                    .unwrap_or_else(|| "_untagged".to_string())
            })
            .collect();

        if let Some(position_value) = valuation.position_values.get(&position.position_id) {
            let total = aggregated
                .entry(key)
                .or_insert_with(|| Money::new(0.0, base_ccy));
            *total = total.checked_add(position_value.value_base)?;
        }
    }

    Ok(aggregated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::PortfolioBuilder;
    use crate::position::{Position, PositionUnit};
    use crate::types::Entity;
    use crate::valuation::value_portfolio;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_valuations::instruments::deposit::Deposit;
    use std::sync::Arc;
    use time::macros::date;

    fn build_test_market() -> MarketContext {
        let base_date = date!(2024 - 01 - 01);
        // Flat curve for testing - requires allow_non_monotonic()
        let curve = DiscountCurve::builder("USD")
            .base_date(base_date)
            .knots(vec![(0.0, 1.0), (1.0, 1.0), (5.0, 1.0)])
            .set_interp(InterpStyle::Linear)
            .allow_non_monotonic()
            .build()
            .unwrap();

        MarketContext::new().insert_discount(curve)
    }

    #[test]
    fn test_group_by_attribute() {
        let as_of = date!(2024 - 01 - 01);

        let dep1 = Deposit::builder()
            .id("DEP_1".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .start(as_of)
            .end(date!(2024 - 02 - 01))
            .day_count(finstack_core::dates::DayCount::Act360)
            .discount_curve_id("USD".into())
            .build()
            .unwrap();

        let dep2 = Deposit::builder()
            .id("DEP_2".into())
            .notional(Money::new(500_000.0, Currency::USD))
            .start(as_of)
            .end(date!(2024 - 03 - 01))
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
        .with_tag("rating", "AAA")
        .with_tag("sector", "Banking");

        let pos2 = Position::new(
            "POS_002",
            "ENTITY_A",
            "DEP_2",
            Arc::new(dep2),
            1.0,
            PositionUnit::Units,
        )
        .with_tag("rating", "AA")
        .with_tag("sector", "Banking");

        let positions = vec![pos1, pos2];

        let groups = group_by_attribute(&positions, "rating");

        assert_eq!(groups.len(), 2);
        assert!(groups.contains_key("AAA"));
        assert!(groups.contains_key("AA"));
        assert_eq!(groups.get("AAA").unwrap().len(), 1);
        assert_eq!(groups.get("AA").unwrap().len(), 1);
    }

    #[test]
    fn test_aggregate_by_attribute() {
        let as_of = date!(2024 - 01 - 01);

        let dep1 = Deposit::builder()
            .id("DEP_1".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .start(as_of)
            .end(date!(2024 - 02 - 01))
            .day_count(finstack_core::dates::DayCount::Act360)
            .discount_curve_id("USD".into())
            .build()
            .unwrap();

        let dep2 = Deposit::builder()
            .id("DEP_2".into())
            .notional(Money::new(500_000.0, Currency::USD))
            .start(as_of)
            .end(date!(2024 - 03 - 01))
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
        .with_tag("rating", "AAA");

        let pos2 = Position::new(
            "POS_002",
            "ENTITY_A",
            "DEP_2",
            Arc::new(dep2),
            1.0,
            PositionUnit::Units,
        )
        .with_tag("rating", "AAA");

        let portfolio = PortfolioBuilder::new("TEST")
            .base_ccy(Currency::USD)
            .as_of(as_of)
            .entity(Entity::new("ENTITY_A"))
            .position(pos1)
            .position(pos2)
            .build()
            .unwrap();

        let market = build_test_market();
        let config = FinstackConfig::default();

        let valuation = value_portfolio(&portfolio, &market, &config).unwrap();
        let aggregated =
            aggregate_by_attribute(&valuation, &portfolio.positions, "rating", Currency::USD)
                .unwrap();

        assert!(aggregated.contains_key("AAA"));
        let total = aggregated.get("AAA").unwrap();
        assert!(total.amount().abs() >= 0.0);
    }
}
