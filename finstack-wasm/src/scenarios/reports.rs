//! Report type bindings for scenarios.

use crate::core::dates::FsDate;
use finstack_scenarios::adapters::RollForwardReport;
use finstack_scenarios::engine::ApplicationReport;
use js_sys::Array;
use wasm_bindgen::prelude::*;

/// Report describing what happened during scenario application.
#[wasm_bindgen]
pub struct JsApplicationReport {
    pub(crate) inner: ApplicationReport,
}

#[wasm_bindgen]
impl JsApplicationReport {
    /// Number of operations successfully applied.
    #[wasm_bindgen(getter, js_name = operationsApplied)]
    pub fn operations_applied(&self) -> usize {
        self.inner.operations_applied
    }

    /// Warnings generated during application (non-fatal).
    ///
    /// # Returns
    /// Array of warning strings
    #[wasm_bindgen(getter)]
    pub fn warnings(&self) -> Array {
        self.inner
            .warnings
            .iter()
            .map(|s| JsValue::from_str(s))
            .collect()
    }

    /// Rounding context stamp (for determinism tracking).
    #[wasm_bindgen(getter, js_name = roundingContext)]
    pub fn rounding_context(&self) -> Option<String> {
        self.inner.rounding_context.clone()
    }
}

impl From<ApplicationReport> for JsApplicationReport {
    fn from(inner: ApplicationReport) -> Self {
        Self { inner }
    }
}

/// Report from time roll-forward operation.
#[wasm_bindgen]
pub struct JsRollForwardReport {
    pub(crate) inner: RollForwardReport,
}

#[wasm_bindgen]
impl JsRollForwardReport {
    /// Original as-of date.
    #[wasm_bindgen(getter, js_name = oldDate)]
    pub fn old_date(&self) -> FsDate {
        FsDate::from_core(self.inner.old_date)
    }

    /// New as-of date after roll.
    #[wasm_bindgen(getter, js_name = newDate)]
    pub fn new_date(&self) -> FsDate {
        FsDate::from_core(self.inner.new_date)
    }

    /// Number of days rolled forward.
    #[wasm_bindgen(getter)]
    pub fn days(&self) -> f64 {
        self.inner.days as f64
    }

    /// Per-instrument carry accrual.
    ///
    /// # Returns
    /// Array of [instrument_id, Array<[currency_code, amount]>] pairs
    #[wasm_bindgen(getter, js_name = instrumentCarry)]
    pub fn instrument_carry(&self) -> Array {
        let result = Array::new();
        for (id, per_ccy) in &self.inner.instrument_carry {
            let outer = Array::new();
            outer.push(&JsValue::from_str(id));

            let inner = Array::new();
            for (ccy, money) in per_ccy {
                let pair = Array::new();
                pair.push(&JsValue::from_str(&ccy.to_string()));
                pair.push(&JsValue::from_f64(money.amount()));
                inner.push(&pair.into());
            }

            outer.push(&inner.into());
            result.push(&outer.into());
        }
        result
    }

    /// Total P&L from carry.
    #[wasm_bindgen(getter, js_name = totalCarry)]
    pub fn total_carry(&self) -> Array {
        let result = Array::new();
        for (ccy, money) in &self.inner.total_carry {
            let pair = Array::new();
            pair.push(&JsValue::from_str(&ccy.to_string()));
            pair.push(&JsValue::from_f64(money.amount()));
            result.push(&pair.into());
        }
        result
    }

}

impl From<RollForwardReport> for JsRollForwardReport {
    fn from(inner: RollForwardReport) -> Self {
        Self { inner }
    }
}
