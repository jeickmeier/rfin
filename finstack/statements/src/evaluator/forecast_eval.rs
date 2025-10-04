//! Forecast evaluation logic.

use crate::error::{Error, Result};
use crate::evaluator::context::EvaluationContext;
use crate::forecast;
use crate::types::{FinancialModelSpec, NodeSpec};
use finstack_core::dates::PeriodId;
use indexmap::IndexMap;

/// Evaluate a forecast for a specific period.
///
/// This function determines the base value and applies the forecast method
/// to generate values for all forecast periods. Results are cached.
pub(crate) fn evaluate_forecast(
    node_spec: &NodeSpec,
    model: &FinancialModelSpec,
    period_id: &PeriodId,
    context: &EvaluationContext,
    forecast_cache: &mut IndexMap<String, IndexMap<PeriodId, f64>>,
) -> Result<f64> {
    // Check cache first
    if let Some(cached) = forecast_cache.get(&node_spec.node_id) {
        if let Some(value) = cached.get(period_id) {
            return Ok(*value);
        }
    }

    // Compute forecast for all periods and cache
    let forecast_spec = node_spec
        .forecasts
        .first()
        .ok_or_else(|| Error::eval(format!("No forecast spec for node '{}'", node_spec.node_id)))?;

    // Find all forecast periods
    let forecast_periods: Vec<PeriodId> = model
        .periods
        .iter()
        .filter(|p| !p.is_actual)
        .map(|p| p.id)
        .collect();

    if forecast_periods.is_empty() {
        return Err(Error::eval("No forecast periods in model"));
    }

    // Determine base value (last actual or last historical)
    let base_value = determine_base_value(node_spec, period_id, model, context)?;

    // Apply forecast method
    let forecast_results = forecast::apply_forecast(forecast_spec, base_value, &forecast_periods)?;

    // Cache results
    forecast_cache.insert(node_spec.node_id.clone(), forecast_results.clone());

    // Return value for requested period
    forecast_results.get(period_id).copied().ok_or_else(|| {
        Error::eval(format!(
            "Forecast did not produce value for period {:?}",
            period_id
        ))
    })
}

/// Determine the base value for forecasting.
///
/// Logic:
/// 1. If there's a value in the last actual period, use it
/// 2. If there's a historical value (from context), use the most recent
/// 3. Otherwise error
fn determine_base_value(
    node_spec: &NodeSpec,
    _current_period_id: &PeriodId,
    model: &FinancialModelSpec,
    context: &EvaluationContext,
) -> Result<f64> {
    // Try to get last actual period value
    let last_actual_period = model.periods.iter().filter(|p| p.is_actual).last();

    if let Some(last_actual) = last_actual_period {
        // Check node's explicit values
        if let Some(values) = &node_spec.values {
            if let Some(val) = values.get(&last_actual.id) {
                return Ok(val.value());
            }
        }

        // Check historical context
        if let Some(val) = context.get_historical_value(&node_spec.node_id, &last_actual.id) {
            return Ok(val);
        }
    }

    // Try to find any historical value
    for historical_period in context.historical_results.keys().rev() {
        if let Some(val) = context.get_historical_value(&node_spec.node_id, historical_period) {
            return Ok(val);
        }
    }

    // No base value found
    Err(Error::eval(format!(
        "Cannot determine base value for forecast of node '{}'. \
         No actual period value or historical value found.",
        node_spec.node_id
    )))
}
