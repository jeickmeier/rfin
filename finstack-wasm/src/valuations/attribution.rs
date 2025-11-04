//! WASM bindings for P&L attribution.

use finstack_valuations::attribution::{AttributionMethod, PnlAttribution};
use wasm_bindgen::prelude::*;

/// WASM wrapper for AttributionMethod.
#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmAttributionMethod {
    #[wasm_bindgen(skip)]
    pub inner: AttributionMethod,
}

#[wasm_bindgen]
impl WasmAttributionMethod {
    #[wasm_bindgen(constructor)]
    pub fn parallel() -> Self {
        Self {
            inner: AttributionMethod::Parallel,
        }
    }

    #[wasm_bindgen(js_name = metricsBasedmethod)]
    pub fn metrics_based() -> Self {
        Self {
            inner: AttributionMethod::MetricsBased,
        }
    }
}

/// WASM wrapper for PnlAttribution.
#[wasm_bindgen]
pub struct WasmPnlAttribution {
    #[wasm_bindgen(skip)]
    pub inner: PnlAttribution,
}

#[wasm_bindgen]
impl WasmPnlAttribution {
    #[wasm_bindgen(getter, js_name = totalPnl)]
    pub fn total_pnl(&self) -> f64 {
        self.inner.total_pnl.amount()
    }

    #[wasm_bindgen(getter)]
    pub fn carry(&self) -> f64 {
        self.inner.carry.amount()
    }

    #[wasm_bindgen(getter, js_name = ratesCurvesPnl)]
    pub fn rates_curves_pnl(&self) -> f64 {
        self.inner.rates_curves_pnl.amount()
    }

    #[wasm_bindgen(getter, js_name = creditCurvesPnl)]
    pub fn credit_curves_pnl(&self) -> f64 {
        self.inner.credit_curves_pnl.amount()
    }

    #[wasm_bindgen(getter, js_name = inflationCurvesPnl)]
    pub fn inflation_curves_pnl(&self) -> f64 {
        self.inner.inflation_curves_pnl.amount()
    }

    #[wasm_bindgen(getter, js_name = correlationsPnl)]
    pub fn correlations_pnl(&self) -> f64 {
        self.inner.correlations_pnl.amount()
    }

    #[wasm_bindgen(getter, js_name = fxPnl)]
    pub fn fx_pnl(&self) -> f64 {
        self.inner.fx_pnl.amount()
    }

    #[wasm_bindgen(getter, js_name = volPnl)]
    pub fn vol_pnl(&self) -> f64 {
        self.inner.vol_pnl.amount()
    }

    #[wasm_bindgen(getter, js_name = modelParamsPnl)]
    pub fn model_params_pnl(&self) -> f64 {
        self.inner.model_params_pnl.amount()
    }

    #[wasm_bindgen(getter, js_name = marketScalarsPnl)]
    pub fn market_scalars_pnl(&self) -> f64 {
        self.inner.market_scalars_pnl.amount()
    }

    #[wasm_bindgen(getter)]
    pub fn residual(&self) -> f64 {
        self.inner.residual.amount()
    }

    #[wasm_bindgen(js_name = toCsv)]
    pub fn to_csv(&self) -> String {
        self.inner.to_csv()
    }

    #[wasm_bindgen]
    pub fn explain(&self) -> String {
        self.inner.explain()
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        #[cfg(feature = "serde")]
        {
            self.inner
                .to_json()
                .map_err(|e| JsValue::from_str(&format!("JSON serialization failed: {}", e)))
        }
        #[cfg(not(feature = "serde"))]
        {
            Err(JsValue::from_str(
                "JSON serialization requires 'serde' feature",
            ))
        }
    }
}

// Note: Full attribution function requires instrument bindings which are
// instrument-type specific. This module provides the core types; actual
// attribution functions will be added as instrument bindings are completed.

