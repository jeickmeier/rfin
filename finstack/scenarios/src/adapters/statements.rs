//! Statement shock and rate binding adapters.
//!
//! Functions in this file back statement-related `OperationSpec` variants by
//! manipulating `FinancialModelSpec` values directly or synchronising them with
//! market curve data.

use crate::adapters::traits::{ScenarioAdapter, ScenarioEffect};
use crate::engine::ExecutionContext;
use crate::error::{Error, Result};
use crate::spec::{Compounding, OperationSpec, RateBindingSpec};
use crate::utils::tenor_years_from_binding;
use finstack_core::dates::rate_conversions::{
    continuous_to_periodic, continuous_to_simple, simple_to_continuous,
};
use finstack_core::dates::{BusinessDayConvention, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_statements::evaluator::Evaluator;
use finstack_statements::{AmountOrScalar, FinancialModelSpec};

/// Adapter for statement operations.
pub struct StatementAdapter;

impl ScenarioAdapter for StatementAdapter {
    fn try_generate_effects(
        &self,
        op: &OperationSpec,
        _ctx: &ExecutionContext,
    ) -> Result<Option<Vec<ScenarioEffect>>> {
        match op {
            OperationSpec::StmtForecastPercent { node_id, pct } => {
                Ok(Some(vec![ScenarioEffect::StmtForecastPercent {
                    node_id: node_id.clone(),
                    pct: *pct,
                }]))
            }
            OperationSpec::StmtForecastAssign { node_id, value } => {
                Ok(Some(vec![ScenarioEffect::StmtForecastAssign {
                    node_id: node_id.clone(),
                    value: *value,
                }]))
            }
            OperationSpec::RateBinding { binding } => Ok(Some(vec![ScenarioEffect::RateBinding {
                binding: binding.clone(),
            }])),
            _ => Ok(None),
        }
    }
}

fn with_node_values_mut<F>(model: &mut FinancialModelSpec, node_id: &str, mut f: F) -> Result<()>
where
    F: FnMut(&mut AmountOrScalar),
{
    let node = model
        .get_node_mut(node_id)
        .ok_or_else(|| Error::NodeNotFound {
            node_id: node_id.to_string(),
        })?;

    if let Some(values) = node.values.as_mut() {
        for val in values.values_mut() {
            f(val);
        }
    }

    Ok(())
}

/// Apply a percentage change to a statement node's explicit forecast values.
///
/// `pct` is interpreted in percentage points (`5.0 = +5%`). Scalar values and
/// monetary amounts are both scaled multiplicatively.
///
/// # Arguments
///
/// - `model`: Statement model containing the target node.
/// - `node_id`: Identifier of the statement node to shock.
/// - `pct`: Percentage-point shock to apply.
///
/// # Errors
///
/// Returns [`Error::NodeNotFound`] if `node_id` is not present in `model`.
pub fn apply_forecast_percent(
    model: &mut FinancialModelSpec,
    node_id: &str,
    pct: f64,
) -> Result<()> {
    // Apply multiplicative factor to all explicit period values
    let factor = 1.0 + (pct / 100.0);

    with_node_values_mut(model, node_id, |val| match val {
        AmountOrScalar::Scalar(s) => *s *= factor,
        AmountOrScalar::Amount(money) => {
            *money = finstack_core::money::Money::new(money.amount() * factor, money.currency());
        }
    })
}

/// Assign a uniform scalar value to all explicit forecasts in a node.
///
/// # Arguments
///
/// - `model`: Statement model containing the target node.
/// - `node_id`: Identifier of the statement node to overwrite.
/// - `value`: Scalar value to assign to every explicit forecast period.
///
/// # Errors
///
/// Returns [`Error::NodeNotFound`] if `node_id` is not present in `model`.
pub fn apply_forecast_assign(
    model: &mut FinancialModelSpec,
    node_id: &str,
    value: f64,
) -> Result<()> {
    apply_forecast_assign_filtered(model, node_id, value, None)
}

/// Assign a scalar value to explicit forecasts in a node, optionally filtering periods.
///
/// # Arguments
///
/// - `model`: Statement model containing the target node.
/// - `node_id`: Identifier of the statement node to overwrite.
/// - `value`: Scalar value to assign to the selected periods.
/// - `period_filter`: Optional inclusive `(start, end)` date window used to
///   select forecast periods by their statement-period boundaries.
///
/// # Errors
///
/// Returns [`Error::NodeNotFound`] if `node_id` is not present in `model`.
pub fn apply_forecast_assign_filtered(
    model: &mut FinancialModelSpec,
    node_id: &str,
    value: f64,
    period_filter: Option<(finstack_core::dates::Date, finstack_core::dates::Date)>,
) -> Result<()> {
    let allowed_period_ids = period_filter.as_ref().map(|(start, end)| {
        model
            .periods
            .iter()
            .filter(|period| period.start >= *start && period.end <= *end)
            .map(|period| period.id)
            .collect::<std::collections::HashSet<_>>()
    });

    let node = model
        .get_node_mut(node_id)
        .ok_or_else(|| Error::NodeNotFound {
            node_id: node_id.to_string(),
        })?;

    if let Some(values) = node.values.as_mut() {
        for (period_id, val) in values.iter_mut() {
            if allowed_period_ids
                .as_ref()
                .is_none_or(|allowed| allowed.contains(period_id))
            {
                *val = AmountOrScalar::Scalar(value);
            }
        }
    }

    Ok(())
}

/// Update a statement rate node using a full [`RateBindingSpec`].
///
/// Behaviour:
/// - Extracts a rate at `binding.tenor` using the curve's day count and base date
///   (or the binding's day-count override).
/// - Converts from the curve's native quoting (continuous zeros for discount curves,
///   simple forwards for forward curves) into the requested [`Compounding`].
/// - Emits clear validation errors when the tenor is outside the curve range or
///   when compounding/day-count combinations are incompatible.
///
/// The assigned statement value is always a decimal annualized rate such as
/// `0.0525` for 5.25%.
///
/// # Arguments
///
/// - `binding`: Rate-binding specification defining the node, curve, tenor, and
///   output compounding.
/// - `model`: Statement model whose target node will be updated.
/// - `market`: Market context that supplies the referenced discount or forward curve.
///
/// # Errors
///
/// Returns:
/// - [`Error::MarketDataNotFound`] if `binding.curve_id` resolves to neither a
///   discount curve nor a forward curve.
/// - [`Error::InvalidTenor`] if `binding.tenor` cannot be parsed.
/// - [`Error::Validation`] if the requested tenor is outside the curve range or
///   if rate-conversion inputs are inconsistent.
/// - [`Error::NodeNotFound`] if the target statement node is missing.
/// - [`Error::Internal`] if calendar-aware year-fraction conversion fails.
///
/// # References
///
/// - Day-count and business-day conventions: `docs/REFERENCES.md#isda-2006-definitions`
/// - Term-structure and rate-conversion context: `docs/REFERENCES.md#hull-options-futures`
/// - Multi-curve term-structure conventions: `docs/REFERENCES.md#andersen-piterbarg-interest-rate-modeling`
pub fn update_rate_from_binding(
    binding: &RateBindingSpec,
    model: &mut FinancialModelSpec,
    market: &MarketContext,
) -> Result<()> {
    let curve_id = &binding.curve_id;

    if let Ok(curve) = market.get_discount(curve_id) {
        let (tenor_years, _) = tenor_years_from_binding(
            binding,
            curve.base_date(),
            curve.day_count(),
            None,
            BusinessDayConvention::ModifiedFollowing,
        )?;

        if let Some(&max_t) = curve.knots().last() {
            if tenor_years > max_t + 1e-8 {
                return Err(Error::Validation(format!(
                    "Tenor {} ({:.4}y) is out of range for discount curve {} (max {:.4}y)",
                    binding.tenor, tenor_years, curve_id, max_t
                )));
            }
        }

        let zero = curve.zero(tenor_years);
        let converted = convert_continuous_rate(zero, binding.compounding, tenor_years)?;
        return apply_forecast_assign(model, binding.node_id.as_str(), converted);
    }

    if let Ok(curve) = market.get_forward(curve_id) {
        let (start_years, effective_dc) = tenor_years_from_binding(
            binding,
            curve.base_date(),
            curve.day_count(),
            None,
            BusinessDayConvention::ModifiedFollowing,
        )?;

        if let Some(&max_t) = curve.knots().last() {
            if start_years > max_t + 1e-8 {
                return Err(Error::Validation(format!(
                    "Tenor {} ({:.4}y) is out of range for forward curve {} (max {:.4}y)",
                    binding.tenor, start_years, curve_id, max_t
                )));
            }
        }

        // Compute forward start date from binding tenor so the accrual period
        // is anchored at the forward start, not the curve base date.
        // E.g., a 3M accrual starting 1Y from now may span different days
        // than one starting today (Feb→May vs Jan→Apr).
        let forward_start = Tenor::parse(&binding.tenor)
            .map_err(|e| Error::InvalidTenor(e.to_string()))?
            .add_to_date(
                curve.base_date(),
                None,
                BusinessDayConvention::ModifiedFollowing,
            )
            .map_err(|e| Error::Internal(e.to_string()))?;

        let accrual_years = Tenor::from_years(curve.tenor(), effective_dc)
            .to_years_with_context(
                forward_start,
                None,
                BusinessDayConvention::ModifiedFollowing,
                effective_dc,
            )
            .map_err(|e| Error::Internal(e.to_string()))?;

        let forward_simple = curve.rate(start_years);
        let forward_continuous = simple_to_continuous(forward_simple, accrual_years)
            .map_err(|e| Error::Validation(e.to_string()))?;
        let converted =
            convert_continuous_rate(forward_continuous, binding.compounding, accrual_years)?;
        return apply_forecast_assign(model, binding.node_id.as_str(), converted);
    }

    Err(Error::MarketDataNotFound {
        id: curve_id.to_string(),
    })
}

fn convert_continuous_rate(
    continuous_rate: f64,
    comp: Compounding,
    year_fraction: f64,
) -> Result<f64> {
    // Validate year fraction to prevent division by zero or invalid calculations
    // in simple rate conversion: simple_rate = (exp(r * t) - 1) / t
    if year_fraction <= 0.0 {
        return Err(Error::Validation(format!(
            "Year fraction must be positive for rate conversion, got {}",
            year_fraction
        )));
    }

    let converted = match comp {
        Compounding::Continuous => continuous_rate,
        Compounding::Simple => continuous_to_simple(continuous_rate, year_fraction)
            .map_err(|e| Error::Validation(e.to_string()))?,
        Compounding::Annual => continuous_to_periodic(continuous_rate, 1)
            .map_err(|e| Error::Validation(e.to_string()))?,
        Compounding::SemiAnnual => continuous_to_periodic(continuous_rate, 2)
            .map_err(|e| Error::Validation(e.to_string()))?,
        Compounding::Quarterly => continuous_to_periodic(continuous_rate, 4)
            .map_err(|e| Error::Validation(e.to_string()))?,
        Compounding::Monthly => continuous_to_periodic(continuous_rate, 12)
            .map_err(|e| Error::Validation(e.to_string()))?,
    };

    Ok(converted)
}

/// Re-evaluate the financial model to propagate scenario changes.
///
/// Runs the evaluator and returns any warnings (division by zero, NaN
/// propagation, etc.) that were encountered during evaluation.
///
/// # Arguments
///
/// - `model`: Statement model to evaluate after scenario edits.
///
/// # Returns
///
/// A vector of warning strings emitted by the evaluator. An empty vector means
/// evaluation completed without warnings.
///
/// # Errors
///
/// Propagates any error returned by [`Evaluator::evaluate`].
///
/// # Examples
/// ```rust
/// use finstack_scenarios::adapters::statements::reevaluate_model;
/// use finstack_statements::FinancialModelSpec;
///
/// let mut model = FinancialModelSpec::new("demo", vec![]);
/// let warnings = reevaluate_model(&mut model).unwrap();
/// assert!(warnings.is_empty());
/// ```
pub fn reevaluate_model(model: &mut FinancialModelSpec) -> Result<Vec<String>> {
    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(model)?;
    let warnings: Vec<String> = results
        .meta
        .warnings
        .iter()
        .map(|w| format!("{:?}", w))
        .collect();
    Ok(warnings)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::dates::build_periods;
    use finstack_statements::{NodeSpec, NodeType};
    use indexmap::IndexMap;

    #[test]
    fn test_apply_forecast_assign_filtered_updates_only_selected_periods() {
        let period_plan = build_periods("2025Q1..Q4", None).expect("periods should build");
        let periods = period_plan.periods;
        let mut model = FinancialModelSpec::new("test", periods.clone());

        let mut values = IndexMap::new();
        for (i, period) in periods.iter().enumerate() {
            values.insert(period.id, AmountOrScalar::Scalar(100.0 * (i as f64 + 1.0)));
        }

        model.add_node(NodeSpec::new("Revenue", NodeType::Value).with_values(values));

        apply_forecast_assign_filtered(
            &mut model,
            "Revenue",
            500.0,
            Some((periods[1].start, periods[1].end)),
        )
        .expect("filtered assign should succeed");

        let shocked_values: Vec<f64> = model
            .get_node("Revenue")
            .expect("node should exist")
            .values
            .as_ref()
            .expect("values should exist")
            .values()
            .map(|v| match v {
                AmountOrScalar::Scalar(s) => *s,
                AmountOrScalar::Amount(_) => 0.0,
            })
            .collect();

        assert_eq!(shocked_values, vec![100.0, 500.0, 300.0, 400.0]);
    }
}
