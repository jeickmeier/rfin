//! WASM bindings for P&L attribution.

use crate::core::error::{core_to_js, js_error};
use finstack_valuations::attribution::{
    AttributionConfig, AttributionEnvelope, AttributionFactor, AttributionMeta, AttributionMethod,
    AttributionSpec, JsonEnvelope as _, ModelParamsAttribution, RatesCurvesAttribution,
    TaylorAttributionConfig,
};
use wasm_bindgen::prelude::*;

// =============================================================================
// AttributionMethod
// =============================================================================

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
            .map_err(|e| js_error(format!("Invalid factors array: {}", e)))?;

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

        let factors = parsed_factors.map_err(js_error)?;

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

    /// Create Taylor-expansion attribution method.
    ///
    /// Sensitivity-based decomposition via bump-and-reprice.
    #[wasm_bindgen(js_name = taylor)]
    pub fn taylor(config: Option<JsTaylorAttributionConfig>) -> Self {
        let taylor_cfg = config.map(|c| c.inner).unwrap_or_default();
        Self {
            inner: AttributionMethod::Taylor(taylor_cfg),
        }
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("{}", self.inner)
    }
}

// =============================================================================
// AttributionMeta
// =============================================================================

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

// =============================================================================
// RatesCurvesAttribution
// =============================================================================

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
            .map_err(|e| js_error(format!("Serialization failed: {}", e)))
    }
}

// =============================================================================
// ModelParamsAttribution
// =============================================================================

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

// =============================================================================
// TaylorAttributionConfig
// =============================================================================

/// Configuration for Taylor-based P&L attribution.
///
/// Controls whether second-order (gamma) terms are included and sets bump
/// sizes for sensitivity computation.
#[wasm_bindgen(js_name = TaylorAttributionConfig)]
#[derive(Clone)]
pub struct JsTaylorAttributionConfig {
    pub(crate) inner: TaylorAttributionConfig,
}

#[wasm_bindgen(js_class = TaylorAttributionConfig)]
impl JsTaylorAttributionConfig {
    /// Create a Taylor attribution config with default bump sizes.
    #[wasm_bindgen(constructor)]
    pub fn new(include_gamma: bool) -> Self {
        Self {
            inner: TaylorAttributionConfig {
                include_gamma,
                ..TaylorAttributionConfig::default()
            },
        }
    }

    /// Create with custom bump sizes (in basis points for rates/credit, absolute for vol).
    #[wasm_bindgen(js_name = withBumps)]
    pub fn with_bumps(
        include_gamma: bool,
        rate_bump_bp: f64,
        credit_bump_bp: f64,
        vol_bump: f64,
    ) -> Self {
        Self {
            inner: TaylorAttributionConfig {
                include_gamma,
                rate_bump_bp,
                credit_bump_bp,
                vol_bump,
            },
        }
    }

    /// Whether second-order gamma terms are included.
    #[wasm_bindgen(getter, js_name = includeGamma)]
    pub fn include_gamma(&self) -> bool {
        self.inner.include_gamma
    }

    /// Rate bump size in basis points.
    #[wasm_bindgen(getter, js_name = rateBumpBp)]
    pub fn rate_bump_bp(&self) -> f64 {
        self.inner.rate_bump_bp
    }

    /// Credit spread bump size in basis points.
    #[wasm_bindgen(getter, js_name = creditBumpBp)]
    pub fn credit_bump_bp(&self) -> f64 {
        self.inner.credit_bump_bp
    }

    /// Volatility bump size (absolute vol points).
    #[wasm_bindgen(getter, js_name = volBump)]
    pub fn vol_bump(&self) -> f64 {
        self.inner.vol_bump
    }
}

// =============================================================================
// AttributionConfig
// =============================================================================

/// Optional configuration for attribution runs.
///
/// Allows overriding default tolerances and metrics for attribution calculations.
#[wasm_bindgen(js_name = AttributionConfig)]
#[derive(Clone)]
pub struct JsAttributionConfig {
    pub(crate) inner: AttributionConfig,
}

impl Default for JsAttributionConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = AttributionConfig)]
impl JsAttributionConfig {
    /// Create a new attribution config with default values.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: AttributionConfig {
                tolerance_abs: None,
                tolerance_pct: None,
                metrics: None,
                strict_validation: None,
                rounding_scale: None,
                rate_bump_bp: None,
            },
        }
    }

    /// Set absolute tolerance for residual validation.
    #[wasm_bindgen(js_name = withToleranceAbs)]
    pub fn with_tolerance_abs(mut self, tol: f64) -> Self {
        self.inner.tolerance_abs = Some(tol);
        self
    }

    /// Set percentage tolerance for residual validation.
    #[wasm_bindgen(js_name = withTolerancePct)]
    pub fn with_tolerance_pct(mut self, tol: f64) -> Self {
        self.inner.tolerance_pct = Some(tol);
        self
    }

    /// Set strict validation mode.
    #[wasm_bindgen(js_name = withStrictValidation)]
    pub fn with_strict_validation(mut self, strict: bool) -> Self {
        self.inner.strict_validation = Some(strict);
        self
    }

    /// Set rounding scale (number of decimal places).
    #[wasm_bindgen(js_name = withRoundingScale)]
    pub fn with_rounding_scale(mut self, scale: u32) -> Self {
        self.inner.rounding_scale = Some(scale);
        self
    }

    /// Set rate bump size in basis points for sensitivities.
    #[wasm_bindgen(js_name = withRateBumpBp)]
    pub fn with_rate_bump_bp(mut self, bp: f64) -> Self {
        self.inner.rate_bump_bp = Some(bp);
        self
    }
}

// =============================================================================
// AttributionSpec
// =============================================================================

/// Attribution specification for a single P&L attribution run.
///
/// Contains all data needed to perform attribution as a JSON-serializable spec.
/// Use `fromJson` to parse a complete attribution spec, then `execute` to run it.
#[wasm_bindgen(js_name = AttributionSpec)]
pub struct JsAttributionSpec {
    inner: AttributionSpec,
}

#[wasm_bindgen(js_class = AttributionSpec)]
impl JsAttributionSpec {
    /// Parse an attribution spec from a JSON string.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json: &str) -> Result<JsAttributionSpec, JsValue> {
        let spec: AttributionSpec =
            serde_json::from_str(json).map_err(|e| js_error(format!("Invalid JSON: {}", e)))?;
        Ok(JsAttributionSpec { inner: spec })
    }

    /// Serialize the spec to a JSON string.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|e| js_error(format!("Serialization failed: {}", e)))
    }

    /// Execute the attribution and return the result as a JsValue.
    pub fn execute(&self) -> Result<JsValue, JsValue> {
        let result = self.inner.execute().map_err(core_to_js)?;
        serde_wasm_bindgen::to_value(&result)
            .map_err(|e| js_error(format!("Result serialization failed: {}", e)))
    }
}

// =============================================================================
// AttributionEnvelope
// =============================================================================

/// Top-level envelope for attribution specifications with schema versioning.
///
/// Mirrors the calibration/instrument envelope pattern with strict field validation.
#[wasm_bindgen(js_name = AttributionEnvelope)]
pub struct JsAttributionEnvelope {
    inner: AttributionEnvelope,
}

#[wasm_bindgen(js_class = AttributionEnvelope)]
impl JsAttributionEnvelope {
    /// Parse an attribution envelope from a JSON string.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json: &str) -> Result<JsAttributionEnvelope, JsValue> {
        let envelope = AttributionEnvelope::from_json(json).map_err(core_to_js)?;
        Ok(JsAttributionEnvelope { inner: envelope })
    }

    /// Serialize the envelope to a JSON string.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        self.inner.to_json().map_err(core_to_js)
    }

    /// Execute the attribution and return the result envelope as a JsValue.
    pub fn execute(&self) -> Result<JsValue, JsValue> {
        let result = self.inner.execute().map_err(core_to_js)?;
        serde_wasm_bindgen::to_value(&result)
            .map_err(|e| js_error(format!("Result serialization failed: {}", e)))
    }

    /// Get the schema version string.
    #[wasm_bindgen(getter)]
    pub fn schema(&self) -> String {
        self.inner.schema.clone()
    }
}

// =============================================================================
// Top-level attribution functions
// =============================================================================

/// Run a complete attribution from a JSON specification string.
///
/// Parses the JSON as an `AttributionEnvelope`, executes the attribution,
/// and returns the result as a JavaScript object.
#[wasm_bindgen(js_name = attributePnlFromJson)]
pub fn attribute_pnl_from_json(json: &str) -> Result<JsValue, JsValue> {
    let envelope = AttributionEnvelope::from_json(json).map_err(core_to_js)?;
    let result = envelope.execute().map_err(core_to_js)?;
    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| js_error(format!("Result serialization failed: {}", e)))
}
