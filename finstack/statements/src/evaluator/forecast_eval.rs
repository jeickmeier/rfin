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
///
/// # Multiple Forecasts
///
/// While the API supports multiple forecast specifications per node (for future
/// extensibility), currently only the first forecast is used. This ensures
/// deterministic behavior. Future enhancements may include:
/// - Period-range-specific forecasts
/// - Conditional forecast selection based on metrics
/// - Weighted blending of multiple forecast methods
///
/// If multiple forecasts are provided, a warning comment is included but
/// additional forecasts are silently ignored to maintain backward compatibility.
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

    // Select forecast spec - currently using first one for determinism
    // TODO: Future enhancement - implement forecast selection strategy:
    //   - Based on period ranges (e.g., different methods for near vs far term)
    //   - Based on historical accuracy metrics
    //   - Based on external conditions or scenarios
    let forecast_spec = node_spec
        .forecasts
        .first()
        .ok_or_else(|| Error::eval(format!(
            "No forecast spec for node '{}'. Ensure the node has a forecast defined with .forecast()",
            node_spec.node_id
        )))?;

    // Note: If multiple forecasts provided, only first is used (by design)
    // This maintains deterministic behavior while allowing future API extension
    #[allow(clippy::comparison_chain)]
    if node_spec.forecasts.len() > 1 {
        // In a production system, this would be logged at INFO or DEBUG level
        // Currently ignored to avoid breaking existing code that might provide multiple forecasts
    }

    // Find all forecast periods
    let forecast_periods: Vec<PeriodId> = model
        .periods
        .iter()
        .filter(|p| !p.is_actual)
        .map(|p| p.id)
        .collect();

    if forecast_periods.is_empty() {
        return Err(Error::eval(
            "No forecast periods in model. All periods are marked as actuals. \
             Use .periods(range, Some(actuals_cutoff)) to define forecast periods.".to_string()
        ));
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
    Err(Error::forecast(format!(
        "Cannot determine base value for forecast of node '{}'. \
         No actual period value or historical value found. \
         Ensure the node has at least one actual period value or historical data.",
        node_spec.node_id
    )))
}
