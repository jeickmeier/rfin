//! Credit Support Annex (CSA) specification WASM bindings.

use crate::core::currency::JsCurrency;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::margin::parameters::{JsImParameters, JsMarginCallTiming, JsVmParameters};
use finstack_core::types::CurveId;
use finstack_valuations::margin::CsaSpec;
use wasm_bindgen::prelude::*;

/// Credit Support Annex specification (ISDA standard).
///
/// The CSA governs the exchange of collateral between counterparties
/// for OTC derivatives. This specification captures all key commercial
/// terms needed for margin calculation and management.
///
/// @example
/// ```javascript
/// // Standard regulatory CSA for USD
/// const csa = CsaSpec.usdRegulatory();
///
/// // Standard regulatory CSA for EUR
/// const csaEur = CsaSpec.eurRegulatory();
///
/// // Check if CSA requires IM
/// if (csa.requiresIm()) {
///   console.log("IM threshold:", csa.imThreshold);
/// }
/// ```
#[wasm_bindgen(js_name = CsaSpec)]
#[derive(Clone)]
pub struct JsCsaSpec {
    inner: CsaSpec,
}

#[wasm_bindgen(js_class = CsaSpec)]
impl JsCsaSpec {
    /// Create a standard regulatory CSA for USD derivatives.
    ///
    /// Post-2016 regulatory compliant terms with:
    /// - Zero VM threshold
    /// - Daily margin exchange
    /// - SIMM for IM
    /// - Cash and government bonds as eligible collateral
    #[wasm_bindgen(js_name = usdRegulatory)]
    pub fn usd_regulatory() -> JsCsaSpec {
        JsCsaSpec {
            inner: CsaSpec::usd_regulatory(),
        }
    }

    /// Create a standard regulatory CSA for EUR derivatives.
    #[wasm_bindgen(js_name = eurRegulatory)]
    pub fn eur_regulatory() -> JsCsaSpec {
        JsCsaSpec {
            inner: CsaSpec::eur_regulatory(),
        }
    }

    /// Create a custom CSA specification.
    ///
    /// @param {string} id - CSA identifier
    /// @param {Currency} baseCurrency - Base currency for margin calculations
    /// @param {VmParameters} vmParams - Variation margin parameters
    /// @param {ImParameters|undefined} imParams - Initial margin parameters (optional)
    /// @param {string} collateralCurveId - Discount curve ID for collateral valuation
    #[wasm_bindgen(constructor)]
    pub fn new(
        id: &str,
        base_currency: &JsCurrency,
        vm_params: &JsVmParameters,
        im_params: Option<JsImParameters>,
        collateral_curve_id: &str,
    ) -> JsCsaSpec {
        use finstack_valuations::margin::{EligibleCollateralSchedule, MarginCallTiming};

        JsCsaSpec {
            inner: CsaSpec {
                id: id.to_string(),
                base_currency: base_currency.inner(),
                vm_params: vm_params.inner(),
                im_params: im_params.map(|p| p.inner()),
                eligible_collateral: EligibleCollateralSchedule::default(),
                call_timing: MarginCallTiming::default(),
                collateral_curve_id: CurveId::new(collateral_curve_id),
            },
        }
    }

    /// Get the CSA identifier.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id.clone()
    }

    /// Get the base currency.
    #[wasm_bindgen(getter, js_name = baseCurrency)]
    pub fn base_currency(&self) -> JsCurrency {
        JsCurrency::from_inner(self.inner.base_currency)
    }

    /// Get the VM parameters.
    #[wasm_bindgen(getter, js_name = vmParams)]
    pub fn vm_params(&self) -> JsVmParameters {
        JsVmParameters::from_inner(self.inner.vm_params.clone())
    }

    /// Get the IM parameters (if present).
    #[wasm_bindgen(getter, js_name = imParams)]
    pub fn im_params(&self) -> Option<JsImParameters> {
        self.inner.im_params.clone().map(JsImParameters::from_inner)
    }

    /// Get the margin call timing parameters.
    #[wasm_bindgen(getter, js_name = callTiming)]
    pub fn call_timing(&self) -> JsMarginCallTiming {
        JsMarginCallTiming::from_inner(self.inner.call_timing.clone())
    }

    /// Get the collateral discount curve ID.
    #[wasm_bindgen(getter, js_name = collateralCurveId)]
    pub fn collateral_curve_id(&self) -> String {
        self.inner.collateral_curve_id.as_str().to_string()
    }

    /// Check if this CSA requires initial margin.
    #[wasm_bindgen(js_name = requiresIm)]
    pub fn requires_im(&self) -> bool {
        self.inner.requires_im()
    }

    /// Get the VM threshold amount.
    #[wasm_bindgen(getter, js_name = vmThreshold)]
    pub fn vm_threshold(&self) -> JsMoney {
        JsMoney::from_inner(*self.inner.vm_threshold())
    }

    /// Get the IM threshold amount (if IM is required).
    #[wasm_bindgen(getter, js_name = imThreshold)]
    pub fn im_threshold(&self) -> Option<JsMoney> {
        self.inner.im_threshold().map(|m| JsMoney::from_inner(*m))
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsCsaSpec, JsValue> {
        from_js_value(value).map(|inner| JsCsaSpec { inner })
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}

impl JsCsaSpec {
    pub(crate) fn inner(&self) -> CsaSpec {
        self.inner.clone()
    }

    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: CsaSpec) -> Self {
        Self { inner }
    }
}
