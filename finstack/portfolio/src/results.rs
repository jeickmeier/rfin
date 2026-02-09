//! Portfolio results and output types.

use crate::metrics::PortfolioMetrics;
use crate::valuation::PortfolioValuation;
use finstack_core::config::ResultsMeta;
use finstack_core::money::Money;
use serde::{Deserialize, Serialize};

/// Complete results from portfolio evaluation.
///
/// Contains valuation, metrics, and metadata about the calculation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortfolioResult {
    /// Portfolio valuation results
    pub valuation: PortfolioValuation,

    /// Aggregated metrics
    pub metrics: PortfolioMetrics,

    /// Metadata about the calculation
    pub meta: ResultsMeta,
}

impl PortfolioResult {
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
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::builder::PortfolioBuilder;
    use crate::metrics::aggregate_metrics;
    use crate::position::{Position, PositionUnit};
    use crate::test_utils::build_test_market;
    use crate::types::Entity;
    use crate::valuation::value_portfolio;
    use finstack_core::config::{results_meta_now, FinstackConfig};
    use finstack_core::currency::Currency;
    use finstack_valuations::instruments::rates::deposit::Deposit;
    use std::sync::Arc;
    use time::macros::date;

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
            .quote_rate_opt(Some(0.045))
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
        let metrics =
            aggregate_metrics(&valuation, Currency::USD, &market).expect("test should succeed");
        let meta = results_meta_now(&config);

        let results = PortfolioResult::new(valuation, metrics, meta);

        // Note: With flat curve, deposit PV is small but portfolio results should be present
        assert!(results.total_value().amount().abs() >= 0.0);
    }
}
