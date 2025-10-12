//! Statement shock and rate binding adapters.

use crate::error::{Error, Result};
use finstack_core::market_data::MarketContext;
use finstack_statements::{AmountOrScalar, FinancialModelSpec};

/// Apply percent change to a statement node's forecast values.
///
/// Modifies all explicit values in the node by the given percentage.
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

/// Assign explicit value to a statement node's forecasts.
///
/// Sets all explicit values in the node to the given scalar value.
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

/// Update a statement rate node from a market curve.
///
/// Retrieves the curve's current rate and sets it on all explicit node values.
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

/// Re-evaluate the financial model to propagate changes.
///
/// Note: This is a no-op in the current implementation as FinancialModelSpec
/// is a wire type. Full evaluation would require using the Evaluator with the
/// modified spec.
pub fn reevaluate_model(_model: &mut FinancialModelSpec) -> Result<()> {
    // FinancialModelSpec needs to be evaluated via Evaluator
    // For scenarios, the caller should re-evaluate after applying shocks
    Ok(())
}
