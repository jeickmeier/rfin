//! Margin calculators for WASM bindings.

use crate::core::dates::FsDate;
use crate::core::money::JsMoney;
use crate::valuations::margin::csa::JsCsaSpec;
use finstack_margin::{VmCalculator, VmResult};
use wasm_bindgen::prelude::*;

/// Variation margin calculation result.
///
/// @example
/// ```javascript
/// const csa = CsaSpec.usdRegulatory();
/// const calc = new VmCalculator(csa);
///
/// const exposure = new Money(5_000_000, Currency.USD());
/// const posted = new Money(3_000_000, Currency.USD());
/// const result = calc.calculate(exposure, posted, new FsDate(2025, 1, 15));
///
/// if (result.requiresCall()) {
///   console.log("Delivery amount:", result.deliveryAmount.amount);
/// }
/// ```
#[wasm_bindgen(js_name = VmResult)]
pub struct JsVmResult {
    inner: VmResult,
}

#[wasm_bindgen(js_class = VmResult)]
impl JsVmResult {
    /// Get the calculation date.
    #[wasm_bindgen(getter)]
    pub fn date(&self) -> FsDate {
        FsDate::from_core(self.inner.date)
    }

    /// Get the gross mark-to-market exposure.
    #[wasm_bindgen(getter, js_name = grossExposure)]
    pub fn gross_exposure(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.gross_exposure)
    }

    /// Get the net exposure after applying threshold and independent amount.
    #[wasm_bindgen(getter, js_name = netExposure)]
    pub fn net_exposure(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.net_exposure)
    }

    /// Get the delivery amount (positive = we need to post margin).
    #[wasm_bindgen(getter, js_name = deliveryAmount)]
    pub fn delivery_amount(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.delivery_amount)
    }

    /// Get the return amount (positive = we receive margin back).
    #[wasm_bindgen(getter, js_name = returnAmount)]
    pub fn return_amount(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.return_amount)
    }

    /// Get the settlement date for the margin transfer.
    #[wasm_bindgen(getter, js_name = settlementDate)]
    pub fn settlement_date(&self) -> FsDate {
        FsDate::from_core(self.inner.settlement_date)
    }

    /// Get the net margin amount (delivery - return).
    #[wasm_bindgen(js_name = netMargin)]
    pub fn net_margin(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.net_margin())
    }

    /// Check if a margin call is required.
    #[wasm_bindgen(js_name = requiresCall)]
    pub fn requires_call(&self) -> bool {
        self.inner.requires_call()
    }
}

impl JsVmResult {
    pub(crate) fn from_inner(inner: VmResult) -> Self {
        Self { inner }
    }
}

/// Variation margin calculator following ISDA CSA rules.
///
/// Calculates variation margin based on mark-to-market exposure,
/// applying threshold, MTA, independent amount, and rounding rules.
///
/// @example
/// ```javascript
/// const csa = CsaSpec.usdRegulatory();
/// const calc = new VmCalculator(csa);
///
/// const exposure = new Money(5_000_000, Currency.USD());
/// const posted = new Money(3_000_000, Currency.USD());
/// const asOf = new FsDate(2025, 1, 15);
///
/// const result = calc.calculate(exposure, posted, asOf);
/// console.log("Delivery required:", result.deliveryAmount.amount);
/// ```
#[wasm_bindgen(js_name = VmCalculator)]
pub struct JsVmCalculator {
    inner: VmCalculator,
}

#[wasm_bindgen(js_class = VmCalculator)]
impl JsVmCalculator {
    /// Create a new VM calculator with the given CSA specification.
    #[wasm_bindgen(constructor)]
    pub fn new(csa: &JsCsaSpec) -> JsVmCalculator {
        JsVmCalculator {
            inner: VmCalculator::new(csa.inner()),
        }
    }

    /// Calculate variation margin given current exposure and posted collateral.
    ///
    /// @param {Money} exposure - Current mark-to-market exposure (positive = counterparty owes us)
    /// @param {Money} postedCollateral - Value of currently posted collateral
    /// @param {FsDate} asOf - Calculation date
    /// @returns {VmResult} Calculation result with delivery and return amounts
    pub fn calculate(
        &self,
        exposure: &JsMoney,
        posted_collateral: &JsMoney,
        as_of: &FsDate,
    ) -> Result<JsVmResult, JsValue> {
        self.inner
            .calculate(exposure.inner(), posted_collateral.inner(), as_of.inner())
            .map(JsVmResult::from_inner)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Generate margin call dates based on frequency.
    ///
    /// @param {FsDate} start - Start date
    /// @param {FsDate} end - End date
    /// @returns {Array<FsDate>} Array of margin call dates
    #[wasm_bindgen(js_name = marginCallDates)]
    pub fn margin_call_dates(&self, start: &FsDate, end: &FsDate) -> Vec<FsDate> {
        self.inner
            .margin_call_dates(start.inner(), end.inner())
            .into_iter()
            .map(FsDate::from_core)
            .collect()
    }
}
