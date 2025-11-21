//! Covenant forecasting bridge for statements.
//!
//! This module provides the integration between financial statement forecasts
//! and the covenant engine, allowing for future compliance checking.

use crate::evaluator::Results;
use crate::types::FinancialModelSpec;
use finstack_core::dates::{Date, PeriodId};
use finstack_core::Result;
use finstack_valuations::covenants::engine::CovenantEngine;
use finstack_valuations::covenants::forward::{
    forecast_breaches_generic, CovenantForecastConfig, FutureBreach, ModelTimeSeries,
};
use time::Month;

/// Adapter to use Statements Results as a ModelTimeSeries.
pub struct StatementsAdapter<'a> {
    model: Option<&'a FinancialModelSpec>,
    results: &'a Results,
}

impl<'a> StatementsAdapter<'a> {
    /// Create a new adapter from results and optional model spec.
    pub fn new(results: &'a Results, model: Option<&'a FinancialModelSpec>) -> Self {
        Self { model, results }
    }
}

impl<'a> ModelTimeSeries for StatementsAdapter<'a> {
    fn get_scalar(&self, node_id: &str, period: &PeriodId) -> Option<f64> {
        self.results.get(node_id, period)
    }

    fn period_end_date(&self, period: &PeriodId) -> Date {
        if let Some(model) = self.model {
            for p in &model.periods {
                if p.id == *period {
                    return p.end;
                }
            }
        }
        // Fallback if model not provided or period not found: end of quarter/year approximation
        // This should match the logic in valuations/src/covenants/forward.rs tests or be robust
        let month = match period.index {
            1 => Month::March,
            2 => Month::June,
            3 => Month::September,
            4 => Month::December,
            _ => Month::December, // Should not happen for valid quarters
        };
        
        // Simple end of month approximation (30th or 31st)
        // Using 28th to be safe for Feb if we ever supported monthly, but for quarters 30 is fine
        // except for Q1 which is March 31.
        // Let's use the last day of the month logic if we had it, but for now specific dates:
        let day = match month {
            Month::March | Month::December => 31,
            Month::June | Month::September => 30,
            _ => 30,
        };

        Date::from_calendar_date(period.year, month, day).unwrap_or(
            Date::from_calendar_date(period.year, Month::December, 31)
                .expect("December 31 should always be a valid date")
        )
    }
}

/// Forecast covenant breaches based on statement results.
///
/// # Arguments
///
/// * `results` - The forecast results (time-series of metrics)
/// * `covenants` - The covenant engine containing covenant specifications
/// * `model` - Optional financial model spec (for precise period dates)
/// * `config` - Forecasting configuration
///
/// # Returns
///
/// List of projected breaches.
pub fn forecast_breaches(
    results: &Results,
    covenants: &CovenantEngine,
    model: Option<&FinancialModelSpec>,
    config: CovenantForecastConfig,
) -> Result<Vec<FutureBreach>> {
    // Extract all periods from results
    let mut periods: Vec<PeriodId> = results
        .nodes
        .values()
        .flat_map(|map| map.keys().cloned())
        .collect();
    periods.sort();
    periods.dedup();

    let adapter = StatementsAdapter::new(results, model);
    forecast_breaches_generic(covenants, &adapter, &periods, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_valuations::covenants::engine::{CovenantEngine, CovenantSpec, Covenant};
    use finstack_valuations::covenants::engine::CovenantType;
    use finstack_valuations::metrics::MetricId;
    use finstack_core::dates::{Date, Frequency};
    use time::Month;
    use crate::evaluator::{Results, ResultsMeta};
    use indexmap::IndexMap;

    #[test]
    fn test_forecast_breaches_concrete() {
        // 1. Setup Covenant Engine
        let mut engine = CovenantEngine::new();
        let covenant = Covenant::new(
            CovenantType::MaxDebtToEBITDA { threshold: 4.0 },
            Frequency::quarterly(),
        );
        let spec = CovenantSpec {
            covenant,
            metric_id: Some(MetricId::custom("NetDebtEbitda")),
            custom_evaluator: None,
        };
        engine.add_spec(spec);

        // 2. Setup Results (Forecast)
        let p1 = PeriodId::quarter(2025, 1);
        let p2 = PeriodId::quarter(2025, 2);
        
        let mut nodes = IndexMap::new();
        let mut net_debt_ebitda = IndexMap::new();
        net_debt_ebitda.insert(p1, 3.0); // Pass
        net_debt_ebitda.insert(p2, 4.5); // Fail
        nodes.insert("NetDebtEbitda".to_string(), net_debt_ebitda);

        let results = Results {
            nodes,
            monetary_nodes: IndexMap::new(),
            node_value_types: IndexMap::new(),
            meta: ResultsMeta::default(),
        };

        // 3. Run Forecast
        let config = CovenantForecastConfig::default();
        let breaches = forecast_breaches(&results, &engine, None, config)
            .expect("Forecast should succeed");

        // 4. Verify
        assert_eq!(breaches.len(), 1);
        assert_eq!(breaches[0].covenant_id, "Debt/EBITDA ≤ 4.00x");
        assert_eq!(breaches[0].projected_value, 4.5);
        
        // Verify date approximation (Q2 2025 -> June 30)
        let expected_date = Date::from_calendar_date(2025, Month::June, 30)
            .expect("June 30, 2025 should be a valid date");
        assert_eq!(breaches[0].breach_date, expected_date);
    }
}
