//! WASM bindings for factor models.
//!
//! Provides single-factor, two-factor, and multi-factor models for
//! correlated behavior in credit portfolios.

use finstack_correlation::{
    FactorModel, FactorSpec, MultiFactorModel, SingleFactorModel, TwoFactorModel,
};
use wasm_bindgen::prelude::*;

use crate::core::error::{js_error, js_error_with_kind, ErrorKind};

// ---------------------------------------------------------------------------
// SingleFactorModel
// ---------------------------------------------------------------------------

/// Single-factor model (common market factor).
///
/// Models all correlation through a single systematic factor.
///
/// @example
/// ```javascript
/// const model = new SingleFactorModel(0.3, 0.5);
/// console.log(model.volatility); // 0.3
/// console.log(model.numFactors()); // 1
/// ```
#[wasm_bindgen(js_name = SingleFactorModel)]
pub struct JsSingleFactorModel {
    inner: SingleFactorModel,
}

impl JsSingleFactorModel {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: SingleFactorModel) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &SingleFactorModel {
        &self.inner
    }
}

#[wasm_bindgen(js_class = SingleFactorModel)]
impl JsSingleFactorModel {
    /// Create a single-factor model.
    ///
    /// @param volatility - Factor volatility (clamped to [0.01, 2.0]).
    /// @param meanReversion - Mean reversion speed (clamped to [0.0, 10.0]).
    #[wasm_bindgen(constructor)]
    pub fn new(volatility: f64, mean_reversion: f64) -> JsSingleFactorModel {
        JsSingleFactorModel {
            inner: SingleFactorModel::new(volatility, mean_reversion),
        }
    }

    /// Factor volatility.
    #[wasm_bindgen(getter)]
    pub fn volatility(&self) -> f64 {
        self.inner.volatility()
    }

    /// Mean reversion speed.
    #[wasm_bindgen(getter, js_name = meanReversion)]
    pub fn mean_reversion(&self) -> f64 {
        self.inner.mean_reversion()
    }

    /// Number of factors (always 1).
    #[wasm_bindgen(js_name = numFactors)]
    pub fn num_factors(&self) -> usize {
        self.inner.num_factors()
    }

    /// Factor correlation matrix (flattened row-major).
    #[wasm_bindgen(js_name = correlationMatrix)]
    pub fn correlation_matrix(&self) -> Vec<f64> {
        self.inner.correlation_matrix().to_vec()
    }

    /// Factor volatilities.
    #[wasm_bindgen(js_name = volatilities)]
    pub fn volatilities(&self) -> Vec<f64> {
        self.inner.volatilities().to_vec()
    }

    /// Factor names for reporting.
    #[wasm_bindgen(js_name = factorNames)]
    pub fn factor_names(&self) -> Vec<String> {
        self.inner
            .factor_names()
            .into_iter()
            .map(String::from)
            .collect()
    }

    /// Model name for diagnostics.
    #[wasm_bindgen(js_name = modelName)]
    pub fn model_name(&self) -> String {
        self.inner.model_name().to_string()
    }

    /// Diagonal factor contribution for a single z draw.
    #[wasm_bindgen(js_name = diagonalFactorContribution)]
    pub fn diagonal_factor_contribution(&self, factor_index: usize, z: f64) -> f64 {
        self.inner.diagonal_factor_contribution(factor_index, z)
    }

    /// Get string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "SingleFactorModel(vol={:.4}, mr={:.4})",
            self.inner.volatility(),
            self.inner.mean_reversion()
        )
    }
}

// ---------------------------------------------------------------------------
// TwoFactorModel
// ---------------------------------------------------------------------------

/// Two-factor model for prepayment and credit.
///
/// Captures the empirical negative correlation between prepayment and default.
///
/// @example
/// ```javascript
/// const model = new TwoFactorModel(0.2, 0.25, -0.3);
/// const rmbs = TwoFactorModel.rmbsStandard();
/// ```
#[wasm_bindgen(js_name = TwoFactorModel)]
pub struct JsTwoFactorModel {
    inner: TwoFactorModel,
}

impl JsTwoFactorModel {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: TwoFactorModel) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &TwoFactorModel {
        &self.inner
    }
}

#[wasm_bindgen(js_class = TwoFactorModel)]
impl JsTwoFactorModel {
    /// Create a two-factor model.
    ///
    /// @param prepayVol - Prepayment factor volatility (clamped to [0.01, 2.0]).
    /// @param creditVol - Credit factor volatility (clamped to [0.01, 2.0]).
    /// @param correlation - Correlation between factors (clamped to [-0.99, 0.99]).
    #[wasm_bindgen(constructor)]
    pub fn new(prepay_vol: f64, credit_vol: f64, correlation: f64) -> JsTwoFactorModel {
        JsTwoFactorModel {
            inner: TwoFactorModel::new(prepay_vol, credit_vol, correlation),
        }
    }

    /// Standard RMBS calibration (prepay=0.20, credit=0.25, corr=-0.30).
    #[wasm_bindgen(js_name = rmbsStandard)]
    pub fn rmbs_standard() -> JsTwoFactorModel {
        JsTwoFactorModel {
            inner: TwoFactorModel::rmbs_standard(),
        }
    }

    /// Standard CLO calibration (prepay=0.15, credit=0.30, corr=-0.20).
    #[wasm_bindgen(js_name = cloStandard)]
    pub fn clo_standard() -> JsTwoFactorModel {
        JsTwoFactorModel {
            inner: TwoFactorModel::clo_standard(),
        }
    }

    /// Prepayment factor volatility.
    #[wasm_bindgen(getter, js_name = prepayVol)]
    pub fn prepay_vol(&self) -> f64 {
        self.inner.prepay_vol()
    }

    /// Credit factor volatility.
    #[wasm_bindgen(getter, js_name = creditVol)]
    pub fn credit_vol(&self) -> f64 {
        self.inner.credit_vol()
    }

    /// Factor correlation.
    #[wasm_bindgen(getter)]
    pub fn correlation(&self) -> f64 {
        self.inner.correlation()
    }

    /// Cholesky L[1][0] coefficient.
    #[wasm_bindgen(getter, js_name = choleskyL10)]
    pub fn cholesky_l10(&self) -> f64 {
        self.inner.cholesky_l10()
    }

    /// Cholesky L[1][1] coefficient.
    #[wasm_bindgen(getter, js_name = choleskyL11)]
    pub fn cholesky_l11(&self) -> f64 {
        self.inner.cholesky_l11()
    }

    /// Number of factors (always 2).
    #[wasm_bindgen(js_name = numFactors)]
    pub fn num_factors(&self) -> usize {
        self.inner.num_factors()
    }

    /// Factor correlation matrix (flattened row-major).
    #[wasm_bindgen(js_name = correlationMatrix)]
    pub fn correlation_matrix(&self) -> Vec<f64> {
        self.inner.correlation_matrix().to_vec()
    }

    /// Factor volatilities.
    #[wasm_bindgen(js_name = volatilities)]
    pub fn volatilities(&self) -> Vec<f64> {
        self.inner.volatilities().to_vec()
    }

    /// Factor names for reporting.
    #[wasm_bindgen(js_name = factorNames)]
    pub fn factor_names(&self) -> Vec<String> {
        self.inner
            .factor_names()
            .into_iter()
            .map(String::from)
            .collect()
    }

    /// Model name for diagnostics.
    #[wasm_bindgen(js_name = modelName)]
    pub fn model_name(&self) -> String {
        self.inner.model_name().to_string()
    }

    /// Diagonal factor contribution for a single z draw.
    #[wasm_bindgen(js_name = diagonalFactorContribution)]
    pub fn diagonal_factor_contribution(&self, factor_index: usize, z: f64) -> f64 {
        self.inner.diagonal_factor_contribution(factor_index, z)
    }

    /// Get string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "TwoFactorModel(prepayVol={:.4}, creditVol={:.4}, corr={:.4})",
            self.inner.prepay_vol(),
            self.inner.credit_vol(),
            self.inner.correlation()
        )
    }
}

// ---------------------------------------------------------------------------
// MultiFactorModel
// ---------------------------------------------------------------------------

/// Multi-factor model with custom correlation structure.
///
/// Supports arbitrary number of factors with a validated correlation matrix.
/// Uses pivoted Cholesky decomposition for correlated factor generation.
///
/// @example
/// ```javascript
/// const model = MultiFactorModel.uncorrelated(3, [0.2, 0.3, 0.25]);
/// const correlated = model.generateCorrelatedFactors([0.5, -0.3, 0.1]);
/// ```
#[wasm_bindgen(js_name = MultiFactorModel)]
pub struct JsMultiFactorModel {
    inner: MultiFactorModel,
}

impl JsMultiFactorModel {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: MultiFactorModel) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &MultiFactorModel {
        &self.inner
    }
}

#[wasm_bindgen(js_class = MultiFactorModel)]
impl JsMultiFactorModel {
    /// Create a multi-factor model with validation.
    ///
    /// @param numFactors - Number of factors (must be >= 1).
    /// @param volatilities - Factor volatilities (one per factor).
    /// @param correlations - Correlation matrix (flattened row-major, n x n values).
    /// @throws ValidationError if the correlation matrix is invalid.
    #[wasm_bindgen(constructor)]
    pub fn new(
        num_factors: usize,
        volatilities: Vec<f64>,
        correlations: Vec<f64>,
    ) -> Result<JsMultiFactorModel, JsValue> {
        let inner =
            MultiFactorModel::new(num_factors, volatilities, correlations).map_err(|e| {
                js_error_with_kind(
                    ErrorKind::Validation,
                    format!("Invalid correlation matrix: {e}"),
                )
            })?;
        Ok(JsMultiFactorModel { inner })
    }

    /// Create an uncorrelated (identity) multi-factor model.
    ///
    /// @param numFactors - Number of factors.
    /// @param volatilities - Factor volatilities.
    #[wasm_bindgen(js_name = uncorrelated)]
    pub fn uncorrelated(num_factors: usize, volatilities: Vec<f64>) -> JsMultiFactorModel {
        JsMultiFactorModel {
            inner: MultiFactorModel::uncorrelated(num_factors, volatilities),
        }
    }

    /// Generate correlated factor values from independent standard normal draws.
    ///
    /// @param independentZ - Vector of n independent standard normal values.
    /// @returns Vector of n correlated factor values (scaled by volatilities).
    #[wasm_bindgen(js_name = generateCorrelatedFactors)]
    pub fn generate_correlated_factors(&self, independent_z: Vec<f64>) -> Vec<f64> {
        self.inner.generate_correlated_factors(&independent_z)
    }

    /// Number of factors.
    #[wasm_bindgen(js_name = numFactors)]
    pub fn num_factors(&self) -> usize {
        self.inner.num_factors()
    }

    /// Factor correlation matrix (flattened row-major).
    #[wasm_bindgen(js_name = correlationMatrix)]
    pub fn correlation_matrix(&self) -> Vec<f64> {
        self.inner.correlation_matrix().to_vec()
    }

    /// Factor volatilities.
    #[wasm_bindgen(js_name = volatilities)]
    pub fn volatilities(&self) -> Vec<f64> {
        self.inner.volatilities().to_vec()
    }

    /// Factor names for reporting.
    #[wasm_bindgen(js_name = factorNames)]
    pub fn factor_names(&self) -> Vec<String> {
        self.inner
            .factor_names()
            .into_iter()
            .map(String::from)
            .collect()
    }

    /// Model name for diagnostics.
    #[wasm_bindgen(js_name = modelName)]
    pub fn model_name(&self) -> String {
        self.inner.model_name().to_string()
    }

    /// Diagonal factor contribution for a single z draw.
    #[wasm_bindgen(js_name = diagonalFactorContribution)]
    pub fn diagonal_factor_contribution(&self, factor_index: usize, z: f64) -> f64 {
        self.inner.diagonal_factor_contribution(factor_index, z)
    }

    /// Cholesky factor matrix (flattened row-major).
    #[wasm_bindgen(js_name = choleskyFactorMatrix)]
    pub fn cholesky_factor_matrix(&self) -> Vec<f64> {
        self.inner.cholesky_factor().factor_matrix().to_vec()
    }

    /// Get string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("MultiFactorModel(numFactors={})", self.inner.num_factors())
    }
}

// ---------------------------------------------------------------------------
// FactorSpec
// ---------------------------------------------------------------------------

/// Factor model specification for configuration and serialization.
///
/// @example
/// ```javascript
/// const spec = FactorSpec.singleFactor(0.3, 0.5);
/// const json = spec.toJson();
/// const restored = FactorSpec.fromJson(json);
/// ```
#[wasm_bindgen(js_name = FactorSpec)]
pub struct JsFactorSpec {
    inner: FactorSpec,
}

impl JsFactorSpec {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: FactorSpec) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &FactorSpec {
        &self.inner
    }
}

#[wasm_bindgen(js_class = FactorSpec)]
impl JsFactorSpec {
    /// Create a single factor specification.
    ///
    /// @param volatility - Factor volatility (clamped to [0.01, 2.0]).
    /// @param meanReversion - Mean reversion speed (clamped to [0.0, 10.0]).
    #[wasm_bindgen(js_name = singleFactor)]
    pub fn single_factor(volatility: f64, mean_reversion: f64) -> JsFactorSpec {
        JsFactorSpec {
            inner: FactorSpec::single_factor(volatility, mean_reversion),
        }
    }

    /// Create a two-factor specification.
    ///
    /// @param prepayVol - Prepayment factor volatility.
    /// @param creditVol - Credit factor volatility.
    /// @param correlation - Correlation between factors.
    #[wasm_bindgen(js_name = twoFactor)]
    pub fn two_factor(prepay_vol: f64, credit_vol: f64, correlation: f64) -> JsFactorSpec {
        JsFactorSpec {
            inner: FactorSpec::two_factor(prepay_vol, credit_vol, correlation),
        }
    }

    /// Number of factors implied by this specification.
    #[wasm_bindgen(js_name = numFactors)]
    pub fn num_factors(&self) -> usize {
        self.inner.num_factors()
    }

    /// Serialize to JSON string.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner)
            .map_err(|e| js_error(format!("Serialization failed: {e}")))
    }

    /// Deserialize from JSON string.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json: &str) -> Result<JsFactorSpec, JsValue> {
        let inner: FactorSpec = serde_json::from_str(json)
            .map_err(|e| js_error(format!("Deserialization failed: {e}")))?;
        Ok(JsFactorSpec { inner })
    }

    /// Get string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("{:?}", self.inner)
    }
}
