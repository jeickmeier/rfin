//! WASM bindings for recovery models.
//!
//! Provides constant and market-correlated stochastic recovery rate models
//! for credit portfolio pricing.

use finstack_correlation::{ConstantRecovery, CorrelatedRecovery, RecoveryModel, RecoverySpec};
use wasm_bindgen::prelude::*;

use crate::core::error::js_error;

// ---------------------------------------------------------------------------
// ConstantRecovery
// ---------------------------------------------------------------------------

/// Constant recovery rate model.
///
/// Recovery is fixed regardless of market conditions. ISDA standard is 40%.
///
/// @example
/// ```javascript
/// const recovery = ConstantRecovery.isdaStandard();
/// console.log(recovery.rate); // 0.4
/// console.log(recovery.lgd()); // 0.6
/// ```
#[wasm_bindgen(js_name = ConstantRecovery)]
pub struct JsConstantRecovery {
    inner: ConstantRecovery,
}

#[wasm_bindgen(js_class = ConstantRecovery)]
impl JsConstantRecovery {
    /// Create a constant recovery model.
    ///
    /// @param rate - Recovery rate (clamped to [0, 1]).
    #[wasm_bindgen(constructor)]
    pub fn new(rate: f64) -> JsConstantRecovery {
        JsConstantRecovery {
            inner: ConstantRecovery::new(rate),
        }
    }

    /// ISDA standard recovery rate (40%).
    #[wasm_bindgen(js_name = isdaStandard)]
    pub fn isda_standard() -> JsConstantRecovery {
        JsConstantRecovery {
            inner: ConstantRecovery::isda_standard(),
        }
    }

    /// Senior secured recovery rate (55%).
    #[wasm_bindgen(js_name = seniorSecured)]
    pub fn senior_secured() -> JsConstantRecovery {
        JsConstantRecovery {
            inner: ConstantRecovery::senior_secured(),
        }
    }

    /// Subordinated debt recovery rate (25%).
    #[wasm_bindgen(js_name = subordinated)]
    pub fn subordinated() -> JsConstantRecovery {
        JsConstantRecovery {
            inner: ConstantRecovery::subordinated(),
        }
    }

    /// Recovery rate.
    #[wasm_bindgen(getter)]
    pub fn rate(&self) -> f64 {
        self.inner.rate()
    }

    /// Expected (unconditional) recovery rate.
    #[wasm_bindgen(js_name = expectedRecovery)]
    pub fn expected_recovery(&self) -> f64 {
        self.inner.expected_recovery()
    }

    /// Recovery rate conditional on market factor (constant for this model).
    #[wasm_bindgen(js_name = conditionalRecovery)]
    pub fn conditional_recovery(&self, market_factor: f64) -> f64 {
        self.inner.conditional_recovery(market_factor)
    }

    /// Loss given default = 1 - recovery.
    pub fn lgd(&self) -> f64 {
        self.inner.lgd()
    }

    /// Conditional LGD given market factor.
    #[wasm_bindgen(js_name = conditionalLgd)]
    pub fn conditional_lgd(&self, market_factor: f64) -> f64 {
        self.inner.conditional_lgd(market_factor)
    }

    /// Recovery-rate volatility (0 for constant models).
    #[wasm_bindgen(js_name = recoveryVolatility)]
    pub fn recovery_volatility(&self) -> f64 {
        self.inner.recovery_volatility()
    }

    /// Whether this model is stochastic (always false for constant).
    #[wasm_bindgen(js_name = isStochastic)]
    pub fn is_stochastic(&self) -> bool {
        self.inner.is_stochastic()
    }

    /// Model name for diagnostics.
    #[wasm_bindgen(js_name = modelName)]
    pub fn model_name(&self) -> String {
        self.inner.model_name().to_string()
    }

    /// Get string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("ConstantRecovery(rate={:.4})", self.inner.rate())
    }
}

// ---------------------------------------------------------------------------
// CorrelatedRecovery
// ---------------------------------------------------------------------------

/// Market-correlated stochastic recovery model (Andersen-Sidenius).
///
/// Recovery varies with the systematic market factor, capturing the
/// empirical negative correlation between defaults and recovery.
///
/// @example
/// ```javascript
/// const recovery = CorrelatedRecovery.marketStandard();
/// console.log(recovery.mean); // 0.4
/// console.log(recovery.isStochastic()); // true
/// ```
#[wasm_bindgen(js_name = CorrelatedRecovery)]
pub struct JsCorrelatedRecovery {
    inner: CorrelatedRecovery,
}

#[wasm_bindgen(js_class = CorrelatedRecovery)]
impl JsCorrelatedRecovery {
    /// Create a correlated recovery model.
    ///
    /// @param meanRecovery - Mean recovery rate (clamped to [0.05, 0.95]).
    /// @param recoveryVolatility - Recovery volatility (clamped to [0.0, 0.50]).
    /// @param factorCorrelation - Correlation with market factor (clamped to [-1.0, 1.0]).
    #[wasm_bindgen(constructor)]
    pub fn new(
        mean_recovery: f64,
        recovery_volatility: f64,
        factor_correlation: f64,
    ) -> JsCorrelatedRecovery {
        JsCorrelatedRecovery {
            inner: CorrelatedRecovery::new(mean_recovery, recovery_volatility, factor_correlation),
        }
    }

    /// Create with custom recovery bounds.
    ///
    /// @param meanRecovery - Mean recovery rate.
    /// @param recoveryVolatility - Recovery volatility.
    /// @param factorCorrelation - Correlation with market factor.
    /// @param minRecovery - Recovery floor (clamped to [0.0, 0.5]).
    /// @param maxRecovery - Recovery ceiling (clamped to [0.5, 1.0]).
    #[wasm_bindgen(js_name = withBounds)]
    pub fn with_bounds(
        mean_recovery: f64,
        recovery_volatility: f64,
        factor_correlation: f64,
        min_recovery: f64,
        max_recovery: f64,
    ) -> JsCorrelatedRecovery {
        JsCorrelatedRecovery {
            inner: CorrelatedRecovery::with_bounds(
                mean_recovery,
                recovery_volatility,
                factor_correlation,
                min_recovery,
                max_recovery,
            ),
        }
    }

    /// Market-standard calibration (mean=40%, vol=25%, corr=-40%).
    #[wasm_bindgen(js_name = marketStandard)]
    pub fn market_standard() -> JsCorrelatedRecovery {
        JsCorrelatedRecovery {
            inner: CorrelatedRecovery::market_standard(),
        }
    }

    /// Conservative calibration (mean=40%, vol=30%, corr=-50%).
    #[wasm_bindgen(js_name = conservative)]
    pub fn conservative() -> JsCorrelatedRecovery {
        JsCorrelatedRecovery {
            inner: CorrelatedRecovery::conservative(),
        }
    }

    /// Mean recovery rate.
    #[wasm_bindgen(getter)]
    pub fn mean(&self) -> f64 {
        self.inner.mean()
    }

    /// Recovery volatility.
    #[wasm_bindgen(getter)]
    pub fn volatility(&self) -> f64 {
        self.inner.volatility()
    }

    /// Factor correlation.
    #[wasm_bindgen(getter)]
    pub fn correlation(&self) -> f64 {
        self.inner.correlation()
    }

    /// Expected (unconditional) recovery rate.
    #[wasm_bindgen(js_name = expectedRecovery)]
    pub fn expected_recovery(&self) -> f64 {
        self.inner.expected_recovery()
    }

    /// Recovery rate conditional on market factor.
    #[wasm_bindgen(js_name = conditionalRecovery)]
    pub fn conditional_recovery(&self, market_factor: f64) -> f64 {
        self.inner.conditional_recovery(market_factor)
    }

    /// Loss given default = 1 - recovery.
    pub fn lgd(&self) -> f64 {
        self.inner.lgd()
    }

    /// Conditional LGD given market factor.
    #[wasm_bindgen(js_name = conditionalLgd)]
    pub fn conditional_lgd(&self, market_factor: f64) -> f64 {
        self.inner.conditional_lgd(market_factor)
    }

    /// Recovery-rate volatility scale.
    #[wasm_bindgen(js_name = recoveryVolatility)]
    pub fn recovery_volatility(&self) -> f64 {
        self.inner.recovery_volatility()
    }

    /// Whether this model is stochastic.
    #[wasm_bindgen(js_name = isStochastic)]
    pub fn is_stochastic(&self) -> bool {
        self.inner.is_stochastic()
    }

    /// Model name for diagnostics.
    #[wasm_bindgen(js_name = modelName)]
    pub fn model_name(&self) -> String {
        self.inner.model_name().to_string()
    }

    /// Get string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "CorrelatedRecovery(mean={:.4}, vol={:.4}, corr={:.4})",
            self.inner.mean(),
            self.inner.volatility(),
            self.inner.correlation()
        )
    }
}

// ---------------------------------------------------------------------------
// RecoverySpec
// ---------------------------------------------------------------------------

/// Recovery model specification for configuration and serialization.
///
/// @example
/// ```javascript
/// const spec = RecoverySpec.constant(0.4);
/// const json = spec.toJson();
/// const restored = RecoverySpec.fromJson(json);
/// ```
#[wasm_bindgen(js_name = RecoverySpec)]
pub struct JsRecoverySpec {
    inner: RecoverySpec,
}

#[wasm_bindgen(js_class = RecoverySpec)]
impl JsRecoverySpec {
    /// Create a constant recovery specification.
    ///
    /// @param rate - Recovery rate (clamped to [0.0, 1.0]).
    #[wasm_bindgen(js_name = constant)]
    pub fn constant(rate: f64) -> JsRecoverySpec {
        JsRecoverySpec {
            inner: RecoverySpec::constant(rate),
        }
    }

    /// Create a market-correlated recovery specification.
    ///
    /// @param meanRecovery - Mean recovery rate.
    /// @param recoveryVolatility - Recovery volatility.
    /// @param factorCorrelation - Correlation with market factor.
    #[wasm_bindgen(js_name = marketCorrelated)]
    pub fn market_correlated(
        mean_recovery: f64,
        recovery_volatility: f64,
        factor_correlation: f64,
    ) -> JsRecoverySpec {
        JsRecoverySpec {
            inner: RecoverySpec::market_correlated(
                mean_recovery,
                recovery_volatility,
                factor_correlation,
            ),
        }
    }

    /// Market-standard stochastic recovery (mean=40%, vol=25%, corr=-40%).
    #[wasm_bindgen(js_name = marketStandardStochastic)]
    pub fn market_standard_stochastic() -> JsRecoverySpec {
        JsRecoverySpec {
            inner: RecoverySpec::market_standard_stochastic(),
        }
    }

    /// Expected recovery rate from specification.
    #[wasm_bindgen(js_name = expectedRecovery)]
    pub fn expected_recovery(&self) -> f64 {
        self.inner.expected_recovery()
    }

    /// Serialize to JSON string.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner)
            .map_err(|e| js_error(format!("Serialization failed: {e}")))
    }

    /// Deserialize from JSON string.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json: &str) -> Result<JsRecoverySpec, JsValue> {
        let inner: RecoverySpec = serde_json::from_str(json)
            .map_err(|e| js_error(format!("Deserialization failed: {e}")))?;
        Ok(JsRecoverySpec { inner })
    }

    /// Get string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("{:?}", self.inner)
    }
}
