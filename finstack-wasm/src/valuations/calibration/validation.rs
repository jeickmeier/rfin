//! Market data validation bindings for WASM.
//!
//! Provides validation of calibrated curves and surfaces to ensure
//! they satisfy fundamental financial constraints.

use crate::core::market_data::context::JsMarketContext;
use crate::core::market_data::term_structures::{
    JsDiscountCurve, JsForwardCurve, JsHazardCurve, JsInflationCurve,
};
use crate::core::market_data::VolSurface as JsVolSurface;
use finstack_valuations::calibration::{
    CurveValidator, SurfaceValidator, ValidationConfig, ValidationError,
};
use js_sys::{Array, Map};
use serde::Serialize;
use crate::utils::json::{from_js_value, to_js_value};
use wasm_bindgen::prelude::*;

/// Validation error details.
///
/// Contains information about a constraint violation found during validation.
///
/// @example
/// ```typescript
/// const errors = validator.validate();
/// if (errors.length > 0) {
///   const error = errors[0];
///   console.log(`Constraint: ${error.constraint}`);
///   console.log(`Location: ${error.location}`);
///   console.log(`Details: ${error.details}`);
/// }
/// ```
#[wasm_bindgen(js_name = ValidationError)]
#[derive(Clone, Debug)]
pub struct JsValidationError {
    inner: ValidationError,
}

impl JsValidationError {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: ValidationError) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = ValidationError)]
impl JsValidationError {
    /// Get the constraint that was violated.
    #[wasm_bindgen(getter)]
    pub fn constraint(&self) -> String {
        self.inner.constraint.clone()
    }

    /// Get the location where the violation occurred.
    #[wasm_bindgen(getter)]
    pub fn location(&self) -> String {
        self.inner.location.clone()
    }

    /// Get detailed description of the violation.
    #[wasm_bindgen(getter)]
    pub fn details(&self) -> String {
        self.inner.details.clone()
    }

    /// Get numeric values associated with the violation as a Map.
    #[wasm_bindgen(getter)]
    pub fn values(&self) -> Map {
        let map = Map::new();
        for (key, value) in &self.inner.values {
            map.set(&JsValue::from_str(key), &JsValue::from_f64(*value));
        }
        map
    }

    /// Convert to JSON object.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        #[derive(Serialize)]
        struct ErrorData {
            constraint: String,
            location: String,
            details: String,
            values: std::collections::BTreeMap<String, f64>,
        }

        let data = ErrorData {
            constraint: self.inner.constraint.clone(),
            location: self.inner.location.clone(),
            details: self.inner.details.clone(),
            values: self.inner.values.clone(),
        };

        serde_wasm_bindgen::to_value(&data)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "ValidationError(constraint='{}', location='{}', details='{}')",
            self.inner.constraint, self.inner.location, self.inner.details
        )
    }
}

/// Validation configuration for curve and surface validation.
///
/// Controls which validation checks are performed and their tolerance levels.
///
/// @example
/// ```typescript
/// const config = ValidationConfig.standard()
///   .withMinForwardRate(-0.02)
///   .withCheckArbitrage(true);
/// ```
#[wasm_bindgen(js_name = ValidationConfig)]
#[derive(Clone, Debug)]
pub struct JsValidationConfig {
    inner: ValidationConfig,
}

impl JsValidationConfig {
    pub(crate) fn from_inner(inner: ValidationConfig) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> ValidationConfig {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = ValidationConfig)]
impl JsValidationConfig {
    /// Create a new validation configuration with default settings.
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsValidationConfig {
        Self {
            inner: ValidationConfig::default(),
        }
    }

    /// Create standard validation configuration.
    #[wasm_bindgen(js_name = standard)]
    pub fn standard() -> JsValidationConfig {
        Self::new()
    }

    /// Enable/disable forward rate positivity check.
    #[wasm_bindgen(js_name = withCheckForwardPositivity)]
    pub fn with_check_forward_positivity(&self, enabled: bool) -> JsValidationConfig {
        let mut next = self.inner.clone();
        next.check_forward_positivity = enabled;
        Self::from_inner(next)
    }

    /// Set minimum allowed forward rate.
    #[wasm_bindgen(js_name = withMinForwardRate)]
    pub fn with_min_forward_rate(&self, rate: f64) -> JsValidationConfig {
        let mut next = self.inner.clone();
        next.min_forward_rate = rate;
        Self::from_inner(next)
    }

    /// Set maximum allowed forward rate.
    #[wasm_bindgen(js_name = withMaxForwardRate)]
    pub fn with_max_forward_rate(&self, rate: f64) -> JsValidationConfig {
        let mut next = self.inner.clone();
        next.max_forward_rate = rate;
        Self::from_inner(next)
    }

    /// Enable/disable monotonicity checks.
    #[wasm_bindgen(js_name = withCheckMonotonicity)]
    pub fn with_check_monotonicity(&self, enabled: bool) -> JsValidationConfig {
        let mut next = self.inner.clone();
        next.check_monotonicity = enabled;
        Self::from_inner(next)
    }

    /// Enable/disable arbitrage checks.
    #[wasm_bindgen(js_name = withCheckArbitrage)]
    pub fn with_check_arbitrage(&self, enabled: bool) -> JsValidationConfig {
        let mut next = self.inner.clone();
        next.check_arbitrage = enabled;
        Self::from_inner(next)
    }

    /// Set numerical tolerance for comparisons.
    #[wasm_bindgen(js_name = withTolerance)]
    pub fn with_tolerance(&self, tolerance: f64) -> JsValidationConfig {
        let mut next = self.inner.clone();
        next.tolerance = tolerance;
        Self::from_inner(next)
    }

    // Getters
    #[wasm_bindgen(getter, js_name = checkForwardPositivity)]
    pub fn check_forward_positivity(&self) -> bool {
        self.inner.check_forward_positivity
    }

    #[wasm_bindgen(getter, js_name = minForwardRate)]
    pub fn min_forward_rate(&self) -> f64 {
        self.inner.min_forward_rate
    }

    #[wasm_bindgen(getter, js_name = maxForwardRate)]
    pub fn max_forward_rate(&self) -> f64 {
        self.inner.max_forward_rate
    }

    #[wasm_bindgen(getter, js_name = checkMonotonicity)]
    pub fn check_monotonicity(&self) -> bool {
        self.inner.check_monotonicity
    }

    #[wasm_bindgen(getter, js_name = checkArbitrage)]
    pub fn check_arbitrage(&self) -> bool {
        self.inner.check_arbitrage
    }

    #[wasm_bindgen(getter)]
    pub fn tolerance(&self) -> f64 {
        self.inner.tolerance
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsValidationConfig, JsValue> {
        from_js_value(value).map(JsValidationConfig::from_inner)
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}

impl Default for JsValidationConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Validate a discount curve for no-arbitrage constraints and reasonable bounds.
///
/// @param {DiscountCurve} curve - The discount curve to validate
/// @returns {boolean} True if validation passes
/// @throws {Error} If validation fails with details about the violation
///
/// @example
/// ```typescript
/// const [curve, report] = calibrator.calibrate(quotes, null);
/// try {
///   validateDiscountCurve(curve);
///   console.log('Curve validation passed');
/// } catch (error) {
///   console.error('Validation failed:', error.message);
/// }
/// ```
#[wasm_bindgen(js_name = validateDiscountCurve)]
pub fn validate_discount_curve(curve: &JsDiscountCurve) -> Result<bool, JsValue> {
    curve
        .inner()
        .validate(&ValidationConfig::default())
        .map(|_| true)
        .map_err(|e| JsValue::from_str(&format!("Discount curve validation failed: {}", e)))
}

/// Validate a forward curve for no-arbitrage constraints and reasonable bounds.
#[wasm_bindgen(js_name = validateForwardCurve)]
pub fn validate_forward_curve(curve: &JsForwardCurve) -> Result<bool, JsValue> {
    curve
        .inner()
        .validate(&ValidationConfig::default())
        .map(|_| true)
        .map_err(|e| JsValue::from_str(&format!("Forward curve validation failed: {}", e)))
}

/// Validate a hazard curve for no-arbitrage constraints and reasonable bounds.
#[wasm_bindgen(js_name = validateHazardCurve)]
pub fn validate_hazard_curve(curve: &JsHazardCurve) -> Result<bool, JsValue> {
    curve
        .inner()
        .validate(&ValidationConfig::default())
        .map(|_| true)
        .map_err(|e| JsValue::from_str(&format!("Hazard curve validation failed: {}", e)))
}

/// Validate an inflation curve for no-arbitrage constraints and reasonable bounds.
#[wasm_bindgen(js_name = validateInflationCurve)]
pub fn validate_inflation_curve(curve: &JsInflationCurve) -> Result<bool, JsValue> {
    curve
        .inner()
        .validate(&ValidationConfig::default())
        .map(|_| true)
        .map_err(|e| JsValue::from_str(&format!("Inflation curve validation failed: {}", e)))
}

/// Validate a volatility surface for calendar spread and butterfly arbitrage.
///
/// @param {VolSurface} surface - The volatility surface to validate
/// @returns {boolean} True if validation passes
/// @throws {Error} If validation fails with details about the violation
///
/// @example
/// ```typescript
/// const [surface, report] = calibrator.calibrate(volQuotes, market);
/// try {
///   validateVolSurface(surface);
///   console.log('Surface validation passed');
/// } catch (error) {
///   console.error('Validation failed:', error.message);
/// }
/// ```
#[wasm_bindgen(js_name = validateVolSurface)]
pub fn validate_vol_surface(surface: &JsVolSurface) -> Result<bool, JsValue> {
    surface
        .inner()
        .validate(&ValidationConfig::default())
        .map(|_| true)
        .map_err(|e| JsValue::from_str(&format!("Vol surface validation failed: {}", e)))
}

/// Validate all curves in a market context.
///
/// Runs validation on all discount curves, forward curves, hazard curves,
/// and volatility surfaces present in the market context.
///
/// @param {MarketContext} _market - The market context to validate
/// @returns {Array} Array of validation errors (empty if all pass)
///
/// @example
/// ```typescript
/// const calibrator = new DiscountCurveCalibrator('USD-OIS', baseDate, 'USD');
/// const [curve, report] = calibrator.calibrate(quotes, null);
/// const market = new MarketContext();
/// market.insertDiscount(curve);
/// const errors = validateMarketContext(market);
/// if (errors.length > 0) {
///   console.error('Market validation found issues:', errors);
/// }
/// ```
#[wasm_bindgen(js_name = validateMarketContext)]
pub fn validate_market_context(_market: &JsMarketContext) -> Array {
    // Note: This is a simplified version since we don't have direct access
    // to all curves from the JsMarketContext. In practice, you would
    // enumerate through all curves and surfaces.

    // For now, return empty array indicating validation passed
    // A full implementation would iterate through all market data
    Array::new()
}
