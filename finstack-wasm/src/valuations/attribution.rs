//! WASM bindings for P&L attribution.

use finstack_valuations::attribution::{
    AttributionFactor, AttributionMeta, AttributionMethod, ModelParamsAttribution,
    RatesCurvesAttribution,
};
use wasm_bindgen::prelude::*;

/// WASM wrapper for AttributionMethod.
#[wasm_bindgen(js_name = AttributionMethod)]
#[derive(Clone)]
pub struct WasmAttributionMethod {
    #[wasm_bindgen(skip)]
    pub inner: AttributionMethod,
}

#[wasm_bindgen(js_class = AttributionMethod)]
impl WasmAttributionMethod {
    /// Create parallel attribution method.
    ///
    /// Independent factor isolation (may not sum due to cross-effects).
    #[wasm_bindgen(constructor)]
    pub fn parallel() -> Self {
        Self {
            inner: AttributionMethod::Parallel,
        }
    }

    /// Create waterfall attribution method with custom factor order.
    ///
    /// Sequential application (guarantees sum = total, order matters).
    ///
    /// # Arguments
    ///
    /// * `factors` - Array of factor names: "carry", "rates_curves", "credit_curves",
    ///   "inflation_curves", "correlations", "fx", "volatility", "model_parameters", "market_scalars"
    #[wasm_bindgen(js_name = waterfall)]
    pub fn waterfall(factors: JsValue) -> Result<WasmAttributionMethod, JsValue> {
        let factor_names: Vec<String> = serde_wasm_bindgen::from_value(factors)
            .map_err(|e| JsValue::from_str(&format!("Invalid factors array: {}", e)))?;

        let parsed_factors: Result<Vec<AttributionFactor>, String> = factor_names
            .into_iter()
            .map(|s| match s.to_lowercase().as_str() {
                "carry" => Ok(AttributionFactor::Carry),
                "rates_curves" => Ok(AttributionFactor::RatesCurves),
                "credit_curves" => Ok(AttributionFactor::CreditCurves),
                "inflation_curves" => Ok(AttributionFactor::InflationCurves),
                "correlations" => Ok(AttributionFactor::Correlations),
                "fx" => Ok(AttributionFactor::Fx),
                "volatility" => Ok(AttributionFactor::Volatility),
                "model_parameters" => Ok(AttributionFactor::ModelParameters),
                "market_scalars" => Ok(AttributionFactor::MarketScalars),
                _ => Err(format!("Unknown attribution factor: {}", s)),
            })
            .collect();

        let factors = parsed_factors.map_err(|e| JsValue::from_str(&e))?;

        Ok(Self {
            inner: AttributionMethod::Waterfall(factors),
        })
    }

    /// Create metrics-based attribution method.
    ///
    /// Fast linear approximation using existing metrics.
    #[wasm_bindgen(js_name = metricsBased)]
    pub fn metrics_based() -> Self {
        Self {
            inner: AttributionMethod::MetricsBased,
        }
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("{}", self.inner)
    }
}

/// WASM wrapper for AttributionMeta.
#[wasm_bindgen(js_name = AttributionMeta)]
pub struct WasmAttributionMeta {
    #[wasm_bindgen(skip)]
    pub inner: AttributionMeta,
}

#[wasm_bindgen(js_class = AttributionMeta)]
impl WasmAttributionMeta {
    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.instrument_id.clone()
    }

    #[wasm_bindgen(getter, js_name = numRepricings)]
    pub fn num_repricings(&self) -> usize {
        self.inner.num_repricings
    }

    #[wasm_bindgen(getter, js_name = residualPct)]
    pub fn residual_pct(&self) -> f64 {
        self.inner.residual_pct
    }

    #[wasm_bindgen(getter, js_name = toleranceAbs)]
    pub fn tolerance_abs(&self) -> f64 {
        self.inner.tolerance_abs
    }

    #[wasm_bindgen(getter, js_name = tolerancePct)]
    pub fn tolerance_pct(&self) -> f64 {
        self.inner.tolerance_pct
    }

    /// Get method as string ("Parallel", "Waterfall", or "MetricsBased")
    #[wasm_bindgen(getter)]
    pub fn method(&self) -> String {
        format!("{}", self.inner.method)
    }

    /// Get T₀ date as ISO string
    #[wasm_bindgen(getter)]
    pub fn t0(&self) -> String {
        self.inner.t0.to_string()
    }

    /// Get T₁ date as ISO string
    #[wasm_bindgen(getter)]
    pub fn t1(&self) -> String {
        self.inner.t1.to_string()
    }
}

/// WASM wrapper for RatesCurvesAttribution.
#[wasm_bindgen(js_name = RatesCurvesAttribution)]
pub struct WasmRatesCurvesAttribution {
    #[wasm_bindgen(skip)]
    pub inner: RatesCurvesAttribution,
}

#[wasm_bindgen(js_class = RatesCurvesAttribution)]
impl WasmRatesCurvesAttribution {
    #[wasm_bindgen(getter, js_name = discountTotal)]
    pub fn discount_total(&self) -> f64 {
        self.inner.discount_total.amount()
    }

    #[wasm_bindgen(getter, js_name = forwardTotal)]
    pub fn forward_total(&self) -> f64 {
        self.inner.forward_total.amount()
    }

    /// Get curve breakdown as JSON object
    #[wasm_bindgen(js_name = byCurveToJson)]
    pub fn by_curve_to_json(&self) -> Result<String, JsValue> {
        let map: finstack_core::HashMap<String, f64> = self
            .inner
            .by_curve
            .iter()
            .map(|(k, v)| (k.to_string(), v.amount()))
            .collect();

        serde_json::to_string(&map)
            .map_err(|e| JsValue::from_str(&format!("Serialization failed: {}", e)))
    }
}

/// WASM wrapper for ModelParamsAttribution.
#[wasm_bindgen(js_name = ModelParamsAttribution)]
pub struct WasmModelParamsAttribution {
    #[wasm_bindgen(skip)]
    pub inner: ModelParamsAttribution,
}

#[wasm_bindgen(js_class = ModelParamsAttribution)]
impl WasmModelParamsAttribution {
    #[wasm_bindgen(getter)]
    pub fn prepayment(&self) -> Option<f64> {
        self.inner.prepayment.map(|m| m.amount())
    }

    #[wasm_bindgen(getter, js_name = defaultRate)]
    pub fn default_rate(&self) -> Option<f64> {
        self.inner.default_rate.map(|m| m.amount())
    }

    #[wasm_bindgen(getter, js_name = recoveryRate)]
    pub fn recovery_rate(&self) -> Option<f64> {
        self.inner.recovery_rate.map(|m| m.amount())
    }

    #[wasm_bindgen(getter, js_name = conversionRatio)]
    pub fn conversion_ratio(&self) -> Option<f64> {
        self.inner.conversion_ratio.map(|m| m.amount())
    }
}

// Attribution functions (attributePnl, attributePortfolioPnl) are exported
// via JsPnlAttribution / JsPortfolioAttribution in portfolio/attribution.rs.
