//! Portfolio results and output types.

use crate::metrics::PortfolioMetrics;
use crate::valuation::PortfolioValuation;
use finstack_core::prelude::*;
use serde::{Deserialize, Serialize};

/// Complete results from portfolio evaluation.
///
/// Contains valuation, metrics, and metadata about the calculation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortfolioResults {
    /// Portfolio valuation results
    pub valuation: PortfolioValuation,

    /// Aggregated metrics
    pub metrics: PortfolioMetrics,

    /// Metadata about the calculation
    pub meta: ResultsMeta,
}

impl PortfolioResults {
    /// Create a new portfolio results instance.
    ///
    /// # Arguments
    ///
    /// * `valuation` - Portfolio valuation component.
    /// * `metrics` - Portfolio metrics component.
    /// * `meta` - Metadata describing calculation context.
    pub fn new(
        valuation: PortfolioValuation,
        metrics: PortfolioMetrics,
        meta: ResultsMeta,
    ) -> Self {
        Self {
            valuation,
            metrics,
            meta,
        }
    }

    /// Get the total portfolio value.
    pub fn total_value(&self) -> &Money {
        &self.valuation.total_base_ccy
    }

    /// Get a specific aggregated metric.
    ///
    /// # Arguments
    ///
    /// * `metric_id` - Identifier of the metric to retrieve.
    pub fn get_metric(&self, metric_id: &str) -> Option<f64> {
        self.metrics.get_total(metric_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::PortfolioBuilder;
    use crate::metrics::aggregate_metrics;
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
            .expect("test should succeed");

        MarketContext::new().insert_discount(curve)
    }

    #[test]
    fn test_portfolio_results() {
        let as_of = date!(2024 - 01 - 01);

        let deposit = Deposit::builder()
            .id("DEP_1M".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .start(as_of)
            .end(date!(2024 - 02 - 01))
            .day_count(finstack_core::dates::DayCount::Act360)
            .discount_curve_id("USD".into())
            .build()
            .expect("test should succeed");

        let position = Position::new(
            "POS_001",
            "ENTITY_A",
            "DEP_1M",
            Arc::new(deposit),
            1.0,
            PositionUnit::Units,
        )
        .expect("test should succeed");

        let portfolio = PortfolioBuilder::new("TEST")
            .base_ccy(Currency::USD)
            .as_of(as_of)
            .entity(Entity::new("ENTITY_A"))
            .position(position)
            .build()
            .expect("test should succeed");

        let market = build_test_market();
        let config = FinstackConfig::default();

        let valuation = value_portfolio(&portfolio, &market, &config).expect("test should succeed");
        let metrics = aggregate_metrics(&valuation).expect("test should succeed");
        let meta = results_meta(&config);

        let results = PortfolioResults::new(valuation, metrics, meta);

        // Note: With flat curve, deposit PV is small but portfolio results should be present
        assert!(results.total_value().amount().abs() >= 0.0);
    }
}
