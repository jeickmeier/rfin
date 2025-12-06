//! Statement shock and rate binding adapters.
//!
//! Functions in this file back statement-related `OperationSpec` variants by
//! manipulating `FinancialModelSpec` values directly or synchronising them with
//! market curve data.

use crate::adapters::traits::{ScenarioAdapter, ScenarioEffect};
use crate::engine::ExecutionContext;
use crate::error::{Error, Result};
use crate::spec::OperationSpec;
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

    with_node_values_mut(model, node_id, |val| {
        match val {
            AmountOrScalar::Scalar(s) => *s *= factor,
            AmountOrScalar::Amount(money) => {
                *money = finstack_core::money::Money::new(money.amount() * factor, money.currency());
            }
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

/// Update a statement rate node with a representative 1-year rate from a market curve.
///
/// This function uses a heuristic to extract a scalar rate from market curves:
/// - **Discount curves**: Continuously compounded 1Y zero rate, computed as `-ln(DF(1Y))`.
///   This provides the annualized rate implied by the 1-year discount factor.
/// - **Forward curves**: First tenor forward rate from the curve's forward array.
///   This represents the forward rate for the curve's native tenor at the earliest period.
///
/// # Assumptions
/// - Statement nodes are expected to accept rates in these units (continuously compounded
///   for discount-derived rates, simple forward rates for forecast curves).
/// - Day count conventions and compounding frequencies are not parameterized; the function
///   assumes the statement model will interpret rates consistently.
/// - For more sophisticated rate extraction (e.g., matching node tenor, different day count,
///   or compounding), extend this function or use custom bindings.
///
/// # Arguments
/// - `model`: Financial model whose node will be updated.
/// - `node_id`: Identifier of the statement node.
/// - `market`: Source of market data curves.
/// - `curve_id`: Identifier of the curve providing the rate.
///
/// # Returns
/// [`Result`](crate::error::Result) with the unit type on success.
///
/// # Errors
/// - [`Error::MarketDataNotFound`](crate::error::Error::MarketDataNotFound) if
///   the specified curve cannot be retrieved.
/// - [`Error::NodeNotFound`](crate::error::Error::NodeNotFound) if the node is
///   absent.
///
/// # Examples
/// ```rust,no_run
/// use finstack_scenarios::adapters::statements::update_1y_rate_from_curve;
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_statements::FinancialModelSpec;
///
/// # fn main() -> finstack_scenarios::Result<()> {
/// let mut model = FinancialModelSpec::new("demo", vec![]);
/// let market = MarketContext::new();
/// update_1y_rate_from_curve(&mut model, "FloatingRate", &market, "USD_SOFR")?;
/// # Ok(())
/// # }
/// ```
pub fn update_1y_rate_from_curve(
    model: &mut FinancialModelSpec,
    node_id: &str,
    market: &MarketContext,
    curve_id: &str,
) -> Result<()> {
    // Try to get the curve and extract a representative rate
    let rate = if let Ok(curve) = market.get_discount_ref(curve_id) {
        // Validation: Check if curve covers at least 1 year to avoid unsafe extrapolation
        if let Some(&max_t) = curve.knots().last() {
            if max_t < 1.0 {
                return Err(Error::Validation(format!(
                    "Curve {} maturity ({:.2}Y) is too short for 1Y rate binding",
                    curve_id, max_t
                )));
            }
        }
        // Extract 1Y rate from discount curve
        let df_1y = curve.df(1.0);
        -df_1y.ln()
    } else if let Ok(curve) = market.get_forward_ref(curve_id) {
        // Extract forward rate at 1Y (average of available forwards)
        let forwards = curve.forwards();
        if forwards.is_empty() {
            return Err(Error::MarketDataNotFound {
                id: curve_id.to_string(),
            });
        }
        forwards[0] // Use first forward rate as representative
    } else {
        return Err(Error::MarketDataNotFound {
            id: curve_id.to_string(),
        });
    };

    // Set the rate on all explicit values
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
