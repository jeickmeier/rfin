//! Covenant forward-projection bridge between statements and valuations.
//!
//! This module wires the generic forecasting in `finstack_valuations` to the
//! statements model/results without creating a crate cycle.

use finstack_core::dates::{Date, PeriodId};
use finstack_valuations::covenants::engine::CovenantSpec;
use finstack_valuations::covenants::forward::{
    forecast_covenant_generic, CovenantForecast as ValuationCovenantForecast, ModelTimeSeries,
};

pub use finstack_valuations::covenants::forward::CovenantForecastConfig;

pub type CovenantForecast = ValuationCovenantForecast;

struct StatementsAdapter<'a> {
    model: &'a finstack_statements::types::FinancialModelSpec,
    results: &'a finstack_statements::evaluator::Results,
}

impl<'a> StatementsAdapter<'a> {
    fn new(
        model: &'a finstack_statements::types::FinancialModelSpec,
        results: &'a finstack_statements::evaluator::Results,
    ) -> Self {
        Self { model, results }
    }
}

impl<'a> ModelTimeSeries for StatementsAdapter<'a> {
    fn get_scalar(&self, node_id: &str, period: &PeriodId) -> Option<f64> {
        self.results.get(node_id, period)
    }

    fn period_end_date(&self, period: &PeriodId) -> Date {
        // Prefer explicit model periods (accurate bounds)
        for p in &self.model.periods {
            if p.id == *period {
                return p.end;
            }
        }
        // Fallback: end-of-year if period not found (should not happen with consistent inputs)
        finstack_core::dates::Date::from_calendar_date(period.year, time::Month::December, 31)
            .unwrap()
    }
}

pub fn forecast_covenant(
    covenant: &CovenantSpec,
    model: &finstack_statements::types::FinancialModelSpec,
    base_case: &finstack_statements::evaluator::Results,
    periods: &[PeriodId],
    config: CovenantForecastConfig,
) -> finstack_core::Result<CovenantForecast> {
    let adapter = StatementsAdapter::new(model, base_case);
    let mut cfg = config.clone();
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

pub fn forecast_covenants(
    covenants: &[CovenantSpec],
    model: &finstack_statements::types::FinancialModelSpec,
    base_case: &finstack_statements::evaluator::Results,
    periods: &[PeriodId],
    config: CovenantForecastConfig,
) -> finstack_core::Result<Vec<CovenantForecast>> {
    let _adapter = StatementsAdapter::new(model, base_case);
    covenants
        .iter()
        .map(|c| forecast_covenant(c, model, base_case, periods, config.clone()))
        .collect()
}

fn default_driver_node_id(spec: &CovenantSpec) -> Option<&'static str> {
    use finstack_valuations::covenants::engine::CovenantType;
    match &spec.covenant.covenant_type {
        CovenantType::MaxDebtToEBITDA { .. } => Some("ebitda"),
        CovenantType::MinInterestCoverage { .. } => Some("ebit"),
        CovenantType::MinFixedChargeCoverage { .. } => Some("ebitda"),
        CovenantType::MaxTotalLeverage { .. } => Some("ebitda"),
        CovenantType::MaxSeniorLeverage { .. } => Some("ebitda"),
        CovenantType::MinAssetCoverage { .. } => None,
        CovenantType::Negative { .. } | CovenantType::Affirmative { .. } => None,
        CovenantType::Custom { .. } => None,
    }
}

fn extract_sigma_and_seed(
    model: &finstack_statements::types::FinancialModelSpec,
    node_id: &str,
) -> Option<(f64, u64)> {
    use finstack_statements::types::ForecastMethod;
    if let Some(node) = model.nodes.get(node_id) {
        if let Some(spec) = &node.forecast {
            match spec.method {
                ForecastMethod::Normal | ForecastMethod::LogNormal => {
                    let sigma = spec.params.get("std_dev")?.as_f64()?;
                    let seed = spec.params.get("seed")?.as_u64()?;
                    return Some((sigma, seed));
                }
                _ => {}
            }
        }
    }
    None
}

#[cfg(feature = "dataframes")]
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


