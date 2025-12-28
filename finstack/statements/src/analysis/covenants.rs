//! Covenant forecasting bridge for statements.
//!
//! This module provides the integration between financial statement forecasts
//! and the covenant engine, allowing for future compliance checking.

use crate::evaluator::Results;
use crate::types::{FinancialModelSpec, ForecastMethod};
use finstack_core::dates::{Date, PeriodId};
use finstack_core::Result;
use finstack_valuations::covenants::engine::{CovenantEngine, CovenantSpec};
use finstack_valuations::covenants::forward::{
    forecast_breaches_generic, forecast_covenant_generic,
    CovenantForecast as ValuationCovenantForecast, FutureBreach, ModelTimeSeries,
};
use time::Month;

pub use finstack_valuations::covenants::forward::CovenantForecastConfig;
/// Forecast output envelope for covenant compliance projections.
pub type CovenantForecast = ValuationCovenantForecast;

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

        // Try the target date, fallback to Dec 31 which is always valid for any year
        Date::from_calendar_date(period.year, month, day)
            .or_else(|_| Date::from_calendar_date(period.year, Month::December, 31))
            .unwrap_or_else(|_| {
                // This fallback should never be reached since Dec 31 is always valid,
                // but we need a value for the type system. Use epoch as ultimate fallback.
                Date::from_calendar_date(1970, Month::January, 1).unwrap_or(time::Date::MIN)
            })
    }
}

/// Forecast a single covenant's future compliance using statement results.
pub fn forecast_covenant(
    covenant: &CovenantSpec,
    model: &FinancialModelSpec,
    base_case: &Results,
    periods: &[PeriodId],
    config: CovenantForecastConfig,
) -> Result<CovenantForecast> {
    let adapter = StatementsAdapter::new(base_case, Some(model));
    let mut cfg = config;
    if cfg.volatility.is_none() {
        if let Some(driver) = default_driver_node_id(covenant) {
            if let Some((sigma, seed)) = extract_sigma_and_seed(model, driver) {
                cfg.volatility = Some(sigma);
                if cfg.random_seed.is_none() {
                    cfg.random_seed = Some(seed);
                }
            }
        }
    }
    forecast_covenant_generic(covenant, &adapter, periods, cfg)
}

/// Forecast multiple covenants with shared statement inputs.
pub fn forecast_covenants(
    covenants: &[CovenantSpec],
    model: &FinancialModelSpec,
    base_case: &Results,
    periods: &[PeriodId],
    config: CovenantForecastConfig,
) -> Result<Vec<CovenantForecast>> {
    covenants
        .iter()
        .map(|c| forecast_covenant(c, model, base_case, periods, config.clone()))
        .collect()
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

fn default_driver_node_id(spec: &CovenantSpec) -> Option<&'static str> {
    use finstack_valuations::covenants::engine::CovenantType;
    match &spec.covenant.covenant_type {
        CovenantType::MaxDebtToEBITDA { .. } => Some("ebitda"),
        CovenantType::MinInterestCoverage { .. } => Some("ebit"),
        CovenantType::MinFixedChargeCoverage { .. } => Some("ebitda"),
        CovenantType::MaxTotalLeverage { .. } => Some("ebitda"),
        CovenantType::MaxSeniorLeverage { .. } => Some("ebitda"),
        CovenantType::MinAssetCoverage { .. }
        | CovenantType::Negative { .. }
        | CovenantType::Affirmative { .. }
        | CovenantType::Custom { .. }
        | CovenantType::Basket { .. } => None,
    }
}

fn extract_sigma_and_seed(model: &FinancialModelSpec, node_id: &str) -> Option<(f64, u64)> {
    let node = model.nodes.get(node_id)?;
    let spec = node.forecast.as_ref()?;
    match spec.method {
        ForecastMethod::Normal | ForecastMethod::LogNormal => {
            let sigma = spec.params.get("std_dev")?.as_f64()?;
            let seed = spec.params.get("seed")?.as_u64()?;
            Some((sigma, seed))
        }
        _ => None,
    }
}

#[cfg(feature = "dataframes")]
/// Convert a covenant forecast into a Polars DataFrame for downstream analysis.
///
/// # Panics
/// Panics if the DataFrame construction fails, which should never happen
/// with well-formed forecast data (all vectors have the same length).
#[allow(clippy::expect_used)] // DataFrame build from aligned vectors should never fail
pub fn to_polars(forecast: &CovenantForecast) -> polars::prelude::DataFrame {
    use polars::prelude::*;
    let dates = forecast
        .test_dates
        .iter()
        .map(|d| d.to_string())
        .collect::<Vec<_>>();
    df![
        "test_date" => dates,
        "projected_value" => forecast.projected_values.clone(),
        "threshold" => forecast.thresholds.clone(),
        "headroom" => forecast.headroom.clone(),
        "breach_prob" => forecast.breach_probability.clone()
    ]
    .expect("dataframe build")
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::evaluator::{Results, ResultsMeta};
    use finstack_core::dates::{Date, Tenor};
    use finstack_valuations::covenants::engine::CovenantType;
    use finstack_valuations::covenants::engine::{Covenant, CovenantEngine, CovenantSpec};
    use finstack_valuations::metrics::MetricId;
    use indexmap::IndexMap;
    use time::Month;

    #[test]
    fn test_forecast_breaches_concrete() {
        // 1. Setup Covenant Engine
        let mut engine = CovenantEngine::new();
        let covenant = Covenant::new(
            CovenantType::MaxDebtToEBITDA { threshold: 4.0 },
            Tenor::quarterly(),
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
        let breaches =
            forecast_breaches(&results, &engine, None, config).expect("Forecast should succeed");

        // 4. Verify
        assert_eq!(breaches.len(), 1);
        assert_eq!(breaches[0].covenant_id, "Debt/EBITDA <= 4.00x");
        assert_eq!(breaches[0].projected_value, 4.5);

        // Verify date approximation (Q2 2025 -> June 30)
        let expected_date = Date::from_calendar_date(2025, Month::June, 30)
            .expect("June 30, 2025 should be a valid date");
        assert_eq!(breaches[0].breach_date, expected_date);
    }
}
