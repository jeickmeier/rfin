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

/// Apply a percentage change to a statement node's forecast values.
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
pub fn apply_forecast_assign(
    model: &mut FinancialModelSpec,
    node_id: &str,
    value: f64,
) -> Result<()> {
    with_node_values_mut(model, node_id, |val| {
        *val = AmountOrScalar::Scalar(value);
    })
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
        return set_scalar_rate(model, &binding.node_id, converted);
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

        let accrual_years = Tenor::from_years(curve.tenor(), effective_dc)
            .to_years_with_context(
                curve.base_date(),
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
        return set_scalar_rate(model, &binding.node_id, converted);
    }

    Err(Error::MarketDataNotFound {
        id: curve_id.to_string(),
    })
}

fn set_scalar_rate(model: &mut FinancialModelSpec, node_id: &str, rate: f64) -> Result<()> {
    let node = model
        .get_node_mut(node_id)
        .ok_or_else(|| Error::NodeNotFound {
            node_id: node_id.to_string(),
        })?;

    if let Some(values) = node.values.as_mut() {
        for val in values.values_mut() {
            *val = AmountOrScalar::Scalar(rate);
        }
    }

    Ok(())
}

fn convert_continuous_rate(
    continuous_rate: f64,
    comp: Compounding,
    year_fraction: f64,
) -> Result<f64> {
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

/// Re-evaluate the financial model to propagate changes (no-op placeholder).
///
/// # Arguments
/// - `_model`: Financial model that would be re-evaluated. Currently unused.
///
/// # Returns
/// Always returns `Ok(())`.
///
/// # Examples
/// ```rust
/// use finstack_scenarios::adapters::statements::reevaluate_model;
/// use finstack_statements::FinancialModelSpec;
///
/// let mut model = FinancialModelSpec::new("demo", vec![]);
/// assert!(reevaluate_model(&mut model).is_ok());
/// ```
pub fn reevaluate_model(_model: &mut FinancialModelSpec) -> Result<()> {
    // Evaluate the model to propagate any changes made by scenario operations.
    // Results are intentionally discarded here; callers can re-run evaluation
    // and consume results as needed after scenarios are applied.
    let mut evaluator = Evaluator::new();
    evaluator.evaluate(_model)?;
    Ok(())
}
