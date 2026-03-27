//! Precedence resolution: Value > Forecast > Formula.

use crate::error::{Error, Result};
use crate::types::{NodeSpec, NodeType};
use finstack_core::dates::PeriodId;

/// Resolve the value for a node in a specific period using precedence rules.
///
/// Precedence: Value > Forecast > Formula
///
/// - If explicit value exists for this period → use Value
/// - Else if forecast is applicable → use Forecast
/// - Else if formula exists → use Formula
/// - Else → error (node cannot be resolved)
///
/// # Arguments
/// * `node_spec` - Node metadata that includes values, forecast config, and formula text
/// * `period_id` - Period being evaluated
/// * `is_actual_period` - Flag indicating whether the period is classified as actuals (forecasts are skipped)
pub fn resolve_node_value(
    node_spec: &NodeSpec,
    period_id: &PeriodId,
    is_actual_period: bool,
) -> Result<NodeValueSource> {
    // 1. Check for explicit value (highest precedence)
    if let Some(values) = &node_spec.values {
        if let Some(amount_or_scalar) = values.get(period_id) {
            return Ok(NodeValueSource::Value(amount_or_scalar.value()));
        }
    }

    // 2. Check for forecast (only in forecast periods, not actuals)
    if !is_actual_period && node_spec.forecast.is_some() {
        return Ok(NodeValueSource::Forecast);
    }

    // 3. Check for formula (lowest precedence, always available as fallback)
    if let Some(formula) = &node_spec.formula_text {
        return Ok(NodeValueSource::Formula(formula.clone()));
    }

    // 4. No resolution method available
    match node_spec.node_type {
        NodeType::Value => Err(Error::eval(format!(
            "Value node '{}' has no value for period {}",
            node_spec.node_id, period_id
        ))),
        NodeType::Calculated => Err(Error::eval(format!(
            "Calculated node '{}' has no formula",
            node_spec.node_id
        ))),
        NodeType::Mixed => Err(Error::eval(format!(
            "Mixed node '{}' has no value, forecast, or formula for period {}",
            node_spec.node_id, period_id
        ))),
    }
}

/// Source of a node's value for a period.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeValueSource {
    /// Explicit value
    Value(f64),

    /// Forecast (to be evaluated in Phase 4)
    Forecast,

    /// Formula to evaluate
    Formula(String),
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::types::AmountOrScalar;
    use indexmap::IndexMap;

    #[test]
    fn test_value_precedence() {
        let mut values = IndexMap::new();
        values.insert(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100.0));

        let node = NodeSpec::new("revenue", NodeType::Mixed)
            .with_values(values)
            .with_formula("revenue * 1.05");

        let source = resolve_node_value(&node, &PeriodId::quarter(2025, 1), true)
            .expect("test should succeed");

        // Should use explicit value, not formula
        assert_eq!(source, NodeValueSource::Value(100.0));
    }

    #[test]
    fn test_formula_fallback() {
        let node = NodeSpec::new("cogs", NodeType::Calculated).with_formula("revenue * 0.6");

        let source = resolve_node_value(&node, &PeriodId::quarter(2025, 1), true)
            .expect("test should succeed");

        // Should use formula
        assert_eq!(source, NodeValueSource::Formula("revenue * 0.6".into()));
    }

    #[test]
    fn test_value_node_missing_value_error() {
        let node = NodeSpec::new("revenue", NodeType::Value);

        let result = resolve_node_value(&node, &PeriodId::quarter(2025, 1), true);

        // Should error because no value provided
        assert!(result.is_err());
    }

    #[test]
    fn test_forecast_in_forecast_period() {
        use crate::types::{ForecastMethod, ForecastSpec};

        let node = NodeSpec::new("revenue", NodeType::Mixed)
            .with_formula("lag(revenue, 1)")
            .with_forecast(ForecastSpec {
                method: ForecastMethod::GrowthPct,
                params: IndexMap::new(),
            });

        // In forecast period, should prefer forecast over formula
        let source = resolve_node_value(&node, &PeriodId::quarter(2025, 3), false)
            .expect("test should succeed");
        assert_eq!(source, NodeValueSource::Forecast);
    }

    #[test]
    fn test_formula_in_actual_period() {
        use crate::types::{ForecastMethod, ForecastSpec};

        let node = NodeSpec::new("revenue", NodeType::Mixed)
            .with_formula("lag(revenue, 1)")
            .with_forecast(ForecastSpec {
                method: ForecastMethod::GrowthPct,
                params: IndexMap::new(),
            });

        // In actual period, should use formula (not forecast)
        let source = resolve_node_value(&node, &PeriodId::quarter(2025, 1), true)
            .expect("test should succeed");
        assert!(matches!(source, NodeValueSource::Formula(_)));
    }
}
