//! Margin parameter types for WASM bindings.

use crate::core::currency::JsCurrency;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::margin::enums::{JsImMethodology, JsMarginTenor};
use finstack_valuations::margin::{ImParameters, MarginCallTiming, VmParameters};
use wasm_bindgen::prelude::*;

/// Margin call timing parameters.
///
/// Specifies the operational timing for margin calls including
/// notification and dispute resolution windows.
///
/// @example
/// ```javascript
/// const timing = MarginCallTiming.regulatoryStandard();
/// console.log(timing.notificationDeadlineHours); // 13
/// ```
#[wasm_bindgen(js_name = MarginCallTiming)]
#[derive(Clone)]
pub struct JsMarginCallTiming {
    inner: MarginCallTiming,
}

#[wasm_bindgen(js_class = MarginCallTiming)]
impl JsMarginCallTiming {
    /// Create with default timing parameters.
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsMarginCallTiming {
        JsMarginCallTiming {
            inner: MarginCallTiming::default(),
        }
    }

    /// Standard timing for regulatory VM CSA.
    #[wasm_bindgen(js_name = regulatoryStandard)]
    pub fn regulatory_standard() -> Result<JsMarginCallTiming, JsValue> {
        Ok(JsMarginCallTiming {
            inner: MarginCallTiming::regulatory_standard()
                .map_err(|e| JsValue::from_str(&e.to_string()))?,
        })
    }

    /// Notification deadline (hours after valuation).
    #[wasm_bindgen(getter, js_name = notificationDeadlineHours)]
    pub fn notification_deadline_hours(&self) -> u8 {
        self.inner.notification_deadline_hours
    }

    /// Response deadline (hours after notification).
    #[wasm_bindgen(getter, js_name = responseDeadlineHours)]
    pub fn response_deadline_hours(&self) -> u8 {
        self.inner.response_deadline_hours
    }

    /// Dispute resolution window (business days).
    #[wasm_bindgen(getter, js_name = disputeResolutionDays)]
    pub fn dispute_resolution_days(&self) -> u8 {
        self.inner.dispute_resolution_days
    }

    /// Grace period for collateral delivery (business days).
    #[wasm_bindgen(getter, js_name = deliveryGraceDays)]
    pub fn delivery_grace_days(&self) -> u8 {
        self.inner.delivery_grace_days
    }
}

impl Default for JsMarginCallTiming {
    fn default() -> Self {
        Self::new()
    }
}

impl JsMarginCallTiming {
    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> MarginCallTiming {
        self.inner.clone()
    }

    pub(crate) fn from_inner(inner: MarginCallTiming) -> Self {
        Self { inner }
    }
}

/// Variation margin parameters.
///
/// These parameters govern the daily (or periodic) exchange of variation margin
/// under a CSA agreement. VM is exchanged to eliminate mark-to-market exposure.
///
/// @example
/// ```javascript
/// // Regulatory standard (zero threshold)
/// const vmParams = VmParameters.regulatoryStandard(Currency.USD());
///
/// // Custom threshold
/// const custom = VmParameters.withThreshold(
///   new Money(10_000_000, Currency.USD()),
///   new Money(500_000, Currency.USD())
/// );
/// ```
#[wasm_bindgen(js_name = VmParameters)]
#[derive(Clone)]
pub struct JsVmParameters {
    inner: VmParameters,
}

#[wasm_bindgen(js_class = VmParameters)]
impl JsVmParameters {
    /// Create VM parameters with zero threshold (regulatory standard).
    #[wasm_bindgen(js_name = regulatoryStandard)]
    pub fn regulatory_standard(currency: &JsCurrency) -> Result<JsVmParameters, JsValue> {
        Ok(JsVmParameters {
            inner: VmParameters::regulatory_standard(currency.inner())
                .map_err(|e| JsValue::from_str(&e.to_string()))?,
        })
    }

    /// Create VM parameters with a custom threshold.
    #[wasm_bindgen(js_name = withThreshold)]
    pub fn with_threshold(threshold: &JsMoney, mta: &JsMoney) -> JsVmParameters {
        JsVmParameters {
            inner: VmParameters::with_threshold(threshold.inner(), mta.inner()),
        }
    }

    /// Create custom VM parameters.
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        threshold: &JsMoney,
        mta: &JsMoney,
        rounding: &JsMoney,
        independent_amount: &JsMoney,
        frequency: &JsMarginTenor,
        settlement_lag: u32,
    ) -> JsVmParameters {
        JsVmParameters {
            inner: VmParameters {
                threshold: threshold.inner(),
                mta: mta.inner(),
                rounding: rounding.inner(),
                independent_amount: independent_amount.inner(),
                frequency: frequency.inner(),
                settlement_lag,
            },
        }
    }

    /// Get the threshold amount.
    #[wasm_bindgen(getter)]
    pub fn threshold(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.threshold)
    }

    /// Get the minimum transfer amount (MTA).
    #[wasm_bindgen(getter)]
    pub fn mta(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.mta)
    }

    /// Get the rounding increment.
    #[wasm_bindgen(getter)]
    pub fn rounding(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.rounding)
    }

    /// Get the independent amount.
    #[wasm_bindgen(getter, js_name = independentAmount)]
    pub fn independent_amount(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.independent_amount)
    }

    /// Get the margin call frequency.
    #[wasm_bindgen(getter)]
    pub fn frequency(&self) -> JsMarginTenor {
        JsMarginTenor::from_inner(self.inner.frequency)
    }

    /// Get the settlement lag in business days.
    #[wasm_bindgen(getter, js_name = settlementLag)]
    pub fn settlement_lag(&self) -> u32 {
        self.inner.settlement_lag
    }

    /// Calculate the credit support amount (margin call).
    ///
    /// @param {Money} exposure - Current mark-to-market exposure
    /// @param {Money} currentCollateral - Value of currently posted collateral
    /// @returns {Money} Net margin amount to be delivered (positive) or returned (negative)
    #[wasm_bindgen(js_name = calculateMarginCall)]
    pub fn calculate_margin_call(
        &self,
        exposure: &JsMoney,
        current_collateral: &JsMoney,
    ) -> Result<JsMoney, JsValue> {
        let result = self
            .inner
            .calculate_margin_call(exposure.inner(), current_collateral.inner())
            .map_err(|e| js_error(e.to_string()))?;
        Ok(JsMoney::from_inner(result))
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsVmParameters, JsValue> {
        from_js_value(value).map(|inner| JsVmParameters { inner })
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}

impl JsVmParameters {
    pub(crate) fn inner(&self) -> VmParameters {
        self.inner.clone()
    }

    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: VmParameters) -> Self {
        Self { inner }
    }
}

/// Initial margin parameters.
///
/// Initial margin is collateral posted to cover potential future exposure (PFE)
/// during the close-out period following a default.
///
/// @example
/// ```javascript
/// // SIMM methodology (standard for bilateral OTC)
/// const simm = ImParameters.simmStandard(Currency.USD());
///
/// // Schedule-based methodology
/// const schedule = ImParameters.scheduleBased(Currency.EUR());
///
/// // Cleared trades (CCP methodology)
/// const cleared = ImParameters.cleared(Currency.USD());
/// ```
#[wasm_bindgen(js_name = ImParameters)]
#[derive(Clone)]
pub struct JsImParameters {
    inner: ImParameters,
}

#[wasm_bindgen(js_class = ImParameters)]
impl JsImParameters {
    /// Create IM parameters using ISDA SIMM methodology.
    #[wasm_bindgen(js_name = simmStandard)]
    pub fn simm_standard(currency: &JsCurrency) -> Result<JsImParameters, JsValue> {
        Ok(JsImParameters {
            inner: ImParameters::simm_standard(currency.inner())
                .map_err(|e| JsValue::from_str(&e.to_string()))?,
        })
    }

    /// Create IM parameters using schedule-based methodology.
    #[wasm_bindgen(js_name = scheduleBased)]
    pub fn schedule_based(currency: &JsCurrency) -> Result<JsImParameters, JsValue> {
        Ok(JsImParameters {
            inner: ImParameters::schedule_based(currency.inner())
                .map_err(|e| JsValue::from_str(&e.to_string()))?,
        })
    }

    /// Create IM parameters for cleared trades (CCP methodology).
    #[wasm_bindgen(js_name = cleared)]
    pub fn cleared(currency: &JsCurrency) -> Result<JsImParameters, JsValue> {
        Ok(JsImParameters {
            inner: ImParameters::cleared(currency.inner())
                .map_err(|e| JsValue::from_str(&e.to_string()))?,
        })
    }

    /// Create IM parameters for repos using haircut methodology.
    #[wasm_bindgen(js_name = repoHaircut)]
    pub fn repo_haircut(currency: &JsCurrency) -> Result<JsImParameters, JsValue> {
        Ok(JsImParameters {
            inner: ImParameters::repo_haircut(currency.inner())
                .map_err(|e| JsValue::from_str(&e.to_string()))?,
        })
    }

    /// Get the IM methodology.
    #[wasm_bindgen(getter)]
    pub fn methodology(&self) -> JsImMethodology {
        JsImMethodology::from_inner(self.inner.methodology)
    }

    /// Get the margin period of risk in business days.
    #[wasm_bindgen(getter, js_name = mporDays)]
    pub fn mpor_days(&self) -> u32 {
        self.inner.mpor_days
    }

    /// Get the IM threshold.
    #[wasm_bindgen(getter)]
    pub fn threshold(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.threshold)
    }

    /// Get the minimum transfer amount.
    #[wasm_bindgen(getter)]
    pub fn mta(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.mta)
    }

    /// Check if IM must be held in a segregated account.
    #[wasm_bindgen(getter)]
    pub fn segregated(&self) -> bool {
        self.inner.segregated
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsImParameters, JsValue> {
        from_js_value(value).map(|inner| JsImParameters { inner })
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}

impl JsImParameters {
    pub(crate) fn inner(&self) -> ImParameters {
        self.inner.clone()
    }

    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: ImParameters) -> Self {
        Self { inner }
    }
}
