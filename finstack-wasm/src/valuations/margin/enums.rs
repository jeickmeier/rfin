//! Margin-related enumerations for WASM bindings.

use finstack_margin::{ClearingStatus, ImMethodology, MarginTenor};
use wasm_bindgen::prelude::*;

/// Margin call frequency.
///
/// @example
/// ```javascript
/// const frequency = MarginTenor.Daily();
/// console.log(frequency.isDaily()); // true
/// ```
#[wasm_bindgen(js_name = MarginTenor)]
#[derive(Clone, Copy)]
pub struct JsMarginTenor {
    inner: MarginTenor,
}

#[wasm_bindgen(js_class = MarginTenor)]
impl JsMarginTenor {
    /// Daily margin calls (standard for OTC derivatives post-2016).
    #[wasm_bindgen(js_name = Daily)]
    pub fn daily() -> JsMarginTenor {
        JsMarginTenor {
            inner: MarginTenor::Daily,
        }
    }

    /// Weekly margin calls (pre-regulatory period).
    #[wasm_bindgen(js_name = Weekly)]
    pub fn weekly() -> JsMarginTenor {
        JsMarginTenor {
            inner: MarginTenor::Weekly,
        }
    }

    /// Monthly margin calls (pre-regulatory period).
    #[wasm_bindgen(js_name = Monthly)]
    pub fn monthly() -> JsMarginTenor {
        JsMarginTenor {
            inner: MarginTenor::Monthly,
        }
    }

    /// On-demand margin calls (used for repos).
    #[wasm_bindgen(js_name = OnDemand)]
    pub fn on_demand() -> JsMarginTenor {
        JsMarginTenor {
            inner: MarginTenor::OnDemand,
        }
    }

    /// Check if this is daily frequency.
    #[wasm_bindgen(js_name = isDaily)]
    pub fn is_daily(&self) -> bool {
        matches!(self.inner, MarginTenor::Daily)
    }

    /// Get string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        self.inner.to_string()
    }
}

impl JsMarginTenor {
    pub(crate) fn inner(&self) -> MarginTenor {
        self.inner
    }

    pub(crate) fn from_inner(inner: MarginTenor) -> Self {
        Self { inner }
    }
}

/// Initial margin calculation methodology.
///
/// @example
/// ```javascript
/// const methodology = ImMethodology.Simm();
/// console.log(methodology.isSimm()); // true
/// ```
#[wasm_bindgen(js_name = ImMethodology)]
#[derive(Clone, Copy)]
pub struct JsImMethodology {
    inner: ImMethodology,
}

#[wasm_bindgen(js_class = ImMethodology)]
impl JsImMethodology {
    /// ISDA Standard Initial Margin Model (SIMM).
    ///
    /// Sensitivities-based model for bilateral OTC derivatives.
    #[wasm_bindgen(js_name = Simm)]
    pub fn simm() -> JsImMethodology {
        JsImMethodology {
            inner: ImMethodology::Simm,
        }
    }

    /// BCBS-IOSCO regulatory schedule approach.
    ///
    /// Grid-based IM calculation using notional × rate.
    #[wasm_bindgen(js_name = Schedule)]
    pub fn schedule() -> JsImMethodology {
        JsImMethodology {
            inner: ImMethodology::Schedule,
        }
    }

    /// Haircut-based IM (standard for repos).
    #[wasm_bindgen(js_name = Haircut)]
    pub fn haircut() -> JsImMethodology {
        JsImMethodology {
            inner: ImMethodology::Haircut,
        }
    }

    /// Internal model approved by regulator.
    #[wasm_bindgen(js_name = InternalModel)]
    pub fn internal_model() -> JsImMethodology {
        JsImMethodology {
            inner: ImMethodology::InternalModel,
        }
    }

    /// Clearing house methodology (CCP-specific).
    #[wasm_bindgen(js_name = ClearingHouse)]
    pub fn clearing_house() -> JsImMethodology {
        JsImMethodology {
            inner: ImMethodology::ClearingHouse,
        }
    }

    /// Check if this is SIMM methodology.
    #[wasm_bindgen(js_name = isSimm)]
    pub fn is_simm(&self) -> bool {
        matches!(self.inner, ImMethodology::Simm)
    }

    /// Get string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        self.inner.to_string()
    }
}

impl JsImMethodology {
    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> ImMethodology {
        self.inner
    }

    pub(crate) fn from_inner(inner: ImMethodology) -> Self {
        Self { inner }
    }
}

/// Clearing status for OTC derivatives.
///
/// @example
/// ```javascript
/// const status = ClearingStatus.Bilateral();
/// const cleared = ClearingStatus.Cleared("LCH");
/// ```
#[wasm_bindgen(js_name = ClearingStatus)]
#[derive(Clone)]
pub struct JsClearingStatus {
    inner: ClearingStatus,
}

#[wasm_bindgen(js_class = ClearingStatus)]
impl JsClearingStatus {
    /// Bilateral (uncleared) trade governed by CSA.
    #[wasm_bindgen(js_name = Bilateral)]
    pub fn bilateral() -> JsClearingStatus {
        JsClearingStatus {
            inner: ClearingStatus::Bilateral,
        }
    }

    /// Trade cleared through a central counterparty (CCP).
    #[wasm_bindgen(js_name = Cleared)]
    pub fn cleared(ccp: &str) -> JsClearingStatus {
        JsClearingStatus {
            inner: ClearingStatus::Cleared {
                ccp: ccp.to_string(),
            },
        }
    }

    /// Check if this is bilateral (uncleared).
    #[wasm_bindgen(js_name = isBilateral)]
    pub fn is_bilateral(&self) -> bool {
        matches!(self.inner, ClearingStatus::Bilateral)
    }

    /// Check if this is cleared.
    #[wasm_bindgen(js_name = isCleared)]
    pub fn is_cleared(&self) -> bool {
        matches!(self.inner, ClearingStatus::Cleared { .. })
    }

    /// Get CCP name if cleared.
    #[wasm_bindgen(getter)]
    pub fn ccp(&self) -> Option<String> {
        match &self.inner {
            ClearingStatus::Cleared { ccp } => Some(ccp.clone()),
            _ => None,
        }
    }

    /// Get string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        self.inner.to_string()
    }
}

impl JsClearingStatus {
    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> ClearingStatus {
        self.inner.clone()
    }

    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: ClearingStatus) -> Self {
        Self { inner }
    }
}
