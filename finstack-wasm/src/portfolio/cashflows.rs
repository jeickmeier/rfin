//! Portfolio cashflow aggregation bindings for WASM.

use crate::core::currency::JsCurrency;
use crate::core::market_data::context::JsMarketContext;
use crate::core::money::JsMoney;
use crate::portfolio::positions::JsPortfolio;
use finstack_portfolio::cashflows::{
    aggregate_cashflows, cashflows_to_base_by_period, collapse_cashflows_to_base_by_date,
    PortfolioCashflowBuckets, PortfolioCashflows,
};
use js_sys::{Array, Object};
use wasm_bindgen::prelude::*;

/// Aggregated portfolio cashflows by date and currency.
#[wasm_bindgen]
pub struct JsPortfolioCashflows {
    inner: PortfolioCashflows,
}

#[wasm_bindgen]
impl JsPortfolioCashflows {
    /// Get cashflows by date and currency as a nested object.
    ///
    /// Returns `{ [date: string]: { [ccy: string]: Money } }`.
    #[wasm_bindgen(getter, js_name = byDate)]
    pub fn by_date(&self) -> Result<JsValue, JsValue> {
        let outer = Object::new();
        for (date, per_ccy) in &self.inner.by_date {
            let ccy_map = Object::new();
            for (ccy, money) in per_ccy {
                js_sys::Reflect::set(
                    &ccy_map,
                    &JsValue::from_str(&ccy.to_string()),
                    &JsValue::from(JsMoney::from_inner(*money)),
                )?;
            }
            js_sys::Reflect::set(&outer, &JsValue::from_str(&date.to_string()), &ccy_map)?;
        }
        Ok(JsValue::from(outer))
    }

    /// Get per-position cashflows as `{ [positionId]: [ [date, Money], ... ] }`.
    #[wasm_bindgen(getter, js_name = byPosition)]
    pub fn by_position(&self) -> Result<JsValue, JsValue> {
        let dict = Object::new();
        for (pos_id, flows) in &self.inner.by_position {
            let array = Array::new();
            for (date, money) in flows {
                let pair = Array::new();
                pair.push(&JsValue::from_str(&date.to_string()));
                pair.push(&JsValue::from(JsMoney::from_inner(*money)));
                array.push(&pair);
            }
            js_sys::Reflect::set(&dict, &JsValue::from_str(pos_id.as_str()), &array)?;
        }
        Ok(JsValue::from(dict))
    }
}

impl JsPortfolioCashflows {
    pub(crate) fn from_inner(inner: PortfolioCashflows) -> Self {
        Self { inner }
    }
}

/// Portfolio cashflows bucketed by reporting period.
#[wasm_bindgen]
pub struct JsPortfolioCashflowBuckets {
    inner: PortfolioCashflowBuckets,
}

#[wasm_bindgen]
impl JsPortfolioCashflowBuckets {
    /// Get bucketed totals as `{ [periodId]: Money }`.
    #[wasm_bindgen(getter, js_name = byPeriod)]
    pub fn by_period(&self) -> Result<JsValue, JsValue> {
        let dict = Object::new();
        for (period_id, money) in &self.inner.by_period {
            js_sys::Reflect::set(
                &dict,
                &JsValue::from_str(&period_id.to_string()),
                &JsValue::from(JsMoney::from_inner(*money)),
            )?;
        }
        Ok(JsValue::from(dict))
    }
}

impl JsPortfolioCashflowBuckets {
    pub(crate) fn from_inner(inner: PortfolioCashflowBuckets) -> Self {
        Self { inner }
    }
}

/// Aggregate portfolio cashflows by date and currency.
#[wasm_bindgen(js_name = aggregateCashflows)]
pub fn js_aggregate_cashflows(
    portfolio: &JsPortfolio,
    market_context: &JsMarketContext,
) -> Result<JsPortfolioCashflows, JsValue> {
    aggregate_cashflows(&portfolio.inner, market_context.inner())
        .map(JsPortfolioCashflows::from_inner)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Collapse a multi-currency cashflow ladder into base currency by date.
#[wasm_bindgen(js_name = collapseCashflowsToBaseByDate)]
pub fn js_collapse_cashflows_to_base_by_date(
    ladder: &JsPortfolioCashflows,
    market_context: &JsMarketContext,
    base_ccy: JsCurrency,
) -> Result<JsValue, JsValue> {
    let collapsed =
        collapse_cashflows_to_base_by_date(&ladder.inner, market_context.inner(), base_ccy.inner())
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let dict = Object::new();
    for (date, money) in collapsed {
        js_sys::Reflect::set(
            &dict,
            &JsValue::from_str(&date.to_string()),
            &JsValue::from(JsMoney::from_inner(money)),
        )?;
    }

    Ok(JsValue::from(dict))
}

/// Bucket base-currency cashflows by reporting period.
#[wasm_bindgen(js_name = cashflowsToBaseByPeriod)]
pub fn js_cashflows_to_base_by_period(
    ladder: &JsPortfolioCashflows,
    market_context: &JsMarketContext,
    base_ccy: JsCurrency,
    periods: &Array,
) -> Result<JsPortfolioCashflowBuckets, JsValue> {
    let mut rust_periods = Vec::with_capacity(periods.length() as usize);
    for value in periods.iter() {
        let period: finstack_core::dates::Period = serde_wasm_bindgen::from_value(value)
            .map_err(|e| JsValue::from_str(&format!("Expected Period objects: {}", e)))?;
        rust_periods.push(period);
    }

    cashflows_to_base_by_period(
        &ladder.inner,
        market_context.inner(),
        base_ccy.inner(),
        &rust_periods,
    )
    .map(JsPortfolioCashflowBuckets::from_inner)
    .map_err(|e| JsValue::from_str(&e.to_string()))
}
