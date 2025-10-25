//! Statement shock and rate binding adapters.
//!
//! Functions in this file back statement-related `OperationSpec` variants by
//! manipulating `FinancialModelSpec` values directly or synchronising them with
//! market curve data.

use crate::error::{Error, Result};
use finstack_core::market_data::MarketContext;
use finstack_statements::{AmountOrScalar, FinancialModelSpec};
use finstack_statements::evaluator::Evaluator;

/// Apply a percentage change to a statement node's forecast values.
///
/// # Arguments
/// - `model`: Financial model containing the node to update.
/// - `node_id`: Identifier of the statement node.
/// - `pct`: Percentage change to apply (positive increases the forecast).
///
/// # Returns
/// [`Result`](crate::error::Result) with `Ok(())` when the update succeeds.
///
/// # Errors
/// - [`Error::NodeNotFound`](crate::error::Error::NodeNotFound) if the node
///   identifier cannot be resolved.
///
/// # Examples
/// ```rust,no_run
/// use finstack_scenarios::adapters::statements::apply_forecast_percent;
/// use finstack_statements::FinancialModelSpec;
///
/// # fn main() -> finstack_scenarios::Result<()> {
/// let mut model = FinancialModelSpec::new("demo", vec![]);
/// // ... populate node ...
/// apply_forecast_percent(&mut model, "Revenue", -5.0)?;
/// # Ok(())
/// # }
/// ```
pub fn apply_forecast_percent(
    model: &mut FinancialModelSpec,
    node_id: &str,
    pct: f64,
) -> Result<()> {
    let node = model
        .get_node_mut(node_id)
        .ok_or_else(|| Error::NodeNotFound {
            node_id: node_id.to_string(),
        })?;

    // Apply multiplicative factor to all explicit period values
    let factor = 1.0 + (pct / 100.0);

    if let Some(values) = node.values.as_mut() {
        for val in values.values_mut() {
            match val {
                AmountOrScalar::Scalar(s) => *s *= factor,
                AmountOrScalar::Amount(money) => {
                    *money =
                        finstack_core::money::Money::new(money.amount() * factor, money.currency());
                }
            }
        }
    }

    Ok(())
}

/// Assign a uniform scalar value to all explicit forecasts in a node.
///
/// # Arguments
/// - `model`: Financial model containing the node to update.
/// - `node_id`: Identifier of the statement node.
/// - `value`: Scalar value to assign to each explicit forecast entry.
///
/// # Returns
/// [`Result`](crate::error::Result) communicating success or failure.
///
/// # Errors
/// - [`Error::NodeNotFound`](crate::error::Error::NodeNotFound) if the node
///   identifier cannot be resolved.
///
/// # Examples
/// ```rust,no_run
/// use finstack_scenarios::adapters::statements::apply_forecast_assign;
/// use finstack_statements::FinancialModelSpec;
///
/// # fn main() -> finstack_scenarios::Result<()> {
/// let mut model = FinancialModelSpec::new("demo", vec![]);
/// // ... populate node ...
/// apply_forecast_assign(&mut model, "Capex", 1_000.0)?;
/// # Ok(())
/// # }
/// ```
pub fn apply_forecast_assign(
    model: &mut FinancialModelSpec,
    node_id: &str,
    value: f64,
) -> Result<()> {
    let node = model
        .get_node_mut(node_id)
        .ok_or_else(|| Error::NodeNotFound {
            node_id: node_id.to_string(),
        })?;

    // Assign value to all period values (as scalar)
    if let Some(values) = node.values.as_mut() {
        for val in values.values_mut() {
            *val = AmountOrScalar::Scalar(value);
        }
    }

    Ok(())
}

/// Update a statement rate node with a representative rate from a market curve.
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
/// use finstack_scenarios::adapters::statements::update_rate_from_curve;
/// use finstack_core::market_data::MarketContext;
/// use finstack_statements::FinancialModelSpec;
///
/// # fn main() -> finstack_scenarios::Result<()> {
/// let mut model = FinancialModelSpec::new("demo", vec![]);
/// let market = MarketContext::new();
/// update_rate_from_curve(&mut model, "FloatingRate", &market, "USD_SOFR")?;
/// # Ok(())
/// # }
/// ```
pub fn update_rate_from_curve(
    model: &mut FinancialModelSpec,
    node_id: &str,
    market: &MarketContext,
    curve_id: &str,
) -> Result<()> {
    // Try to get the curve and extract a representative rate
    let rate = if let Ok(curve) = market.get_discount_ref(curve_id) {
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
