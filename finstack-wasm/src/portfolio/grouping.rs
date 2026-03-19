//! Attribute-based grouping and aggregation for WASM.

use crate::core::money::JsMoney;
use crate::portfolio::positions::JsPortfolio;
use crate::portfolio::types::JsPosition;
use crate::portfolio::valuation::JsPortfolioValuation;
use js_sys::{Array, Object};
use wasm_bindgen::prelude::*;

/// Group portfolio positions by an attribute.
///
/// Returns a JavaScript object mapping attribute values to lists of positions.
/// The attribute key must exist in position tags for positions to be included.
///
/// # Arguments
///
/// * `portfolio` - Portfolio to group
/// * `attribute_key` - Tag key to group by (e.g., "sector", "rating")
///
/// # Returns
///
/// JavaScript object mapping attribute values to position arrays
///
/// # Throws
///
/// Error if grouping fails
///
/// # Examples
///
/// ```javascript
/// const bySector = groupByAttribute(portfolio, "sector");
/// console.log(bySector["Technology"]);  // Array of positions
/// ```
#[wasm_bindgen(js_name = groupByAttribute)]
pub fn js_group_by_attribute(
    portfolio: &JsPortfolio,
    attribute_key: &str,
) -> Result<JsValue, JsValue> {
    let groups = finstack_portfolio::group_by_attribute(portfolio.inner.positions(), attribute_key);

    let obj = Object::new();
    for (attr_value, positions) in groups {
        let arr = Array::new();
        for position in positions {
            let js_position = JsPosition::from_inner(position.clone());
            arr.push(&JsValue::from(js_position));
        }
        js_sys::Reflect::set(&obj, &JsValue::from_str(&attr_value), &JsValue::from(arr))?;
    }

    Ok(JsValue::from(obj))
}

/// Aggregate portfolio valuation by an attribute.
///
/// Sums position values within each attribute group. Only positions with the
/// specified attribute key in their tags are included. Values are converted
/// to the portfolio base currency before aggregation.
///
/// # Arguments
///
/// * `valuation` - Portfolio valuation results
/// * `portfolio` - Portfolio containing positions
/// * `attribute_key` - Tag key to group by (e.g., "sector", "rating")
///
/// # Returns
///
/// JavaScript object mapping attribute values to aggregated Money amounts
///
/// # Throws
///
/// Error if aggregation fails
///
/// # Examples
///
/// ```javascript
/// const bySector = aggregateByAttribute(valuation, portfolio, "sector");
/// console.log(bySector["Technology"]);  // Money value
/// ```
#[wasm_bindgen(js_name = aggregateByAttribute)]
pub fn js_aggregate_by_attribute(
    valuation: &JsPortfolioValuation,
    portfolio: &JsPortfolio,
    attribute_key: &str,
) -> Result<JsValue, JsValue> {
    let aggregated = finstack_portfolio::aggregate_by_attribute(
        &valuation.inner,
        portfolio.inner.positions(),
        attribute_key,
        portfolio.inner.base_ccy,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let obj = Object::new();
    for (attr_value, money) in aggregated {
        let js_money = JsMoney::from_inner(money);
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str(&attr_value),
            &JsValue::from(js_money),
        )?;
    }

    Ok(JsValue::from(obj))
}
