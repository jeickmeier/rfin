//! WASM bindings for copula models.
//!
//! Provides Gaussian, Student-t, Multi-Factor, and Random Factor Loading
//! copulas for credit portfolio default correlation modeling.

use finstack_correlation::{
    Copula, CopulaSpec, GaussianCopula, MultiFactorCopula, RandomFactorLoadingCopula,
    StudentTCopula,
};
use wasm_bindgen::prelude::*;

use crate::core::error::{js_error, js_error_with_kind, ErrorKind};

// ---------------------------------------------------------------------------
// GaussianCopula
// ---------------------------------------------------------------------------

/// One-factor Gaussian copula (market standard).
///
/// The industry-standard model for credit index tranche pricing.
/// Zero tail dependence; use with base correlation to capture the smile.
///
/// @example
/// ```javascript
/// const copula = new GaussianCopula();
/// const condProb = copula.conditionalDefaultProb(-1.5, [0.3], 0.3);
/// ```
#[wasm_bindgen(js_name = GaussianCopula)]
pub struct JsGaussianCopula {
    inner: GaussianCopula,
}

impl JsGaussianCopula {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: GaussianCopula) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &GaussianCopula {
        &self.inner
    }
}

#[wasm_bindgen(js_class = GaussianCopula)]
impl JsGaussianCopula {
    /// Create a Gaussian copula with optional quadrature order.
    ///
    /// @param quadratureOrder - Number of quadrature points (default 20).
    #[wasm_bindgen(constructor)]
    pub fn new(quadrature_order: Option<u8>) -> JsGaussianCopula {
        let inner = match quadrature_order {
            Some(order) => GaussianCopula::with_quadrature_order(order),
            None => GaussianCopula::new(),
        };
        JsGaussianCopula { inner }
    }

    /// Conditional default probability P(default | Z).
    ///
    /// @param defaultThreshold - Phi^{-1}(PD) threshold.
    /// @param factorRealization - Systematic factor value(s). Length must be 1.
    /// @param correlation - Asset correlation parameter.
    /// @returns Conditional default probability in [0, 1].
    #[wasm_bindgen(js_name = conditionalDefaultProb)]
    pub fn conditional_default_prob(
        &self,
        default_threshold: f64,
        factor_realization: Vec<f64>,
        correlation: f64,
    ) -> f64 {
        self.inner
            .conditional_default_prob(default_threshold, &factor_realization, correlation)
    }

    /// Number of systematic factors (always 1).
    #[wasm_bindgen(js_name = numFactors)]
    pub fn num_factors(&self) -> usize {
        self.inner.num_factors()
    }

    /// Model name for diagnostics.
    #[wasm_bindgen(js_name = modelName)]
    pub fn model_name(&self) -> String {
        self.inner.model_name().to_string()
    }

    /// Lower-tail dependence (always 0 for Gaussian).
    #[wasm_bindgen(js_name = tailDependence)]
    pub fn tail_dependence(&self, correlation: f64) -> f64 {
        self.inner.tail_dependence(correlation)
    }

    /// Get string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("GaussianCopula(numFactors={})", self.inner.num_factors())
    }
}

// ---------------------------------------------------------------------------
// StudentTCopula
// ---------------------------------------------------------------------------

/// Student-t copula with configurable degrees of freedom.
///
/// Captures tail dependence -- joint extreme defaults cluster more than
/// Gaussian predicts. Lower df = more tail dependence.
///
/// @example
/// ```javascript
/// const copula = new StudentTCopula(5.0);
/// console.log(copula.degreesOfFreedom); // 5.0
/// ```
#[wasm_bindgen(js_name = StudentTCopula)]
pub struct JsStudentTCopula {
    inner: StudentTCopula,
}

impl JsStudentTCopula {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: StudentTCopula) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &StudentTCopula {
        &self.inner
    }
}

#[wasm_bindgen(js_class = StudentTCopula)]
impl JsStudentTCopula {
    /// Create a Student-t copula.
    ///
    /// @param degreesOfFreedom - Must be > 2 for finite variance. Typical: 4-10.
    /// @param quadratureOrder - Number of quadrature points (default 20).
    #[wasm_bindgen(constructor)]
    pub fn new(
        degrees_of_freedom: f64,
        quadrature_order: Option<u8>,
    ) -> Result<JsStudentTCopula, JsValue> {
        if degrees_of_freedom <= 2.0 {
            return Err(js_error_with_kind(
                ErrorKind::Validation,
                "Student-t degrees_of_freedom must be > 2 for finite variance",
            ));
        }
        let inner = match quadrature_order {
            Some(order) => StudentTCopula::with_quadrature_order(degrees_of_freedom, order),
            None => StudentTCopula::new(degrees_of_freedom),
        };
        Ok(JsStudentTCopula { inner })
    }

    /// Degrees of freedom.
    #[wasm_bindgen(getter, js_name = degreesOfFreedom)]
    pub fn degrees_of_freedom(&self) -> f64 {
        self.inner.df()
    }

    /// Conditional default probability P(default | M).
    #[wasm_bindgen(js_name = conditionalDefaultProb)]
    pub fn conditional_default_prob(
        &self,
        default_threshold: f64,
        factor_realization: Vec<f64>,
        correlation: f64,
    ) -> f64 {
        self.inner
            .conditional_default_prob(default_threshold, &factor_realization, correlation)
    }

    /// Number of systematic factors (always 1).
    #[wasm_bindgen(js_name = numFactors)]
    pub fn num_factors(&self) -> usize {
        self.inner.num_factors()
    }

    /// Model name for diagnostics.
    #[wasm_bindgen(js_name = modelName)]
    pub fn model_name(&self) -> String {
        self.inner.model_name().to_string()
    }

    /// Lower-tail dependence coefficient lambda_L.
    #[wasm_bindgen(js_name = tailDependence)]
    pub fn tail_dependence(&self, correlation: f64) -> f64 {
        self.inner.tail_dependence(correlation)
    }

    /// Get string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "StudentTCopula(df={:.1}, numFactors={})",
            self.inner.df(),
            self.inner.num_factors()
        )
    }
}

// ---------------------------------------------------------------------------
// MultiFactorCopula
// ---------------------------------------------------------------------------

/// Multi-factor Gaussian copula with sector structure.
///
/// Uses a global factor plus sector-specific factors to model
/// intra-sector vs. inter-sector correlation differences.
///
/// @example
/// ```javascript
/// const copula = new MultiFactorCopula(2);
/// console.log(copula.interSectorCorrelation);
/// ```
#[wasm_bindgen(js_name = MultiFactorCopula)]
pub struct JsMultiFactorCopula {
    inner: MultiFactorCopula,
}

impl JsMultiFactorCopula {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: MultiFactorCopula) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &MultiFactorCopula {
        &self.inner
    }
}

#[wasm_bindgen(js_class = MultiFactorCopula)]
impl JsMultiFactorCopula {
    /// Create a multi-factor copula.
    ///
    /// @param numFactors - Number of factors (1 or 2; capped at 2).
    /// @param globalLoading - Loading on global factor (default 0.4).
    /// @param sectorLoading - Loading on sector factor (default 0.3).
    /// @param sectorFraction - Fraction of total correlation from sector factor (default 0.4).
    #[wasm_bindgen(constructor)]
    pub fn new(
        num_factors: usize,
        global_loading: Option<f64>,
        sector_loading: Option<f64>,
        sector_fraction: Option<f64>,
    ) -> JsMultiFactorCopula {
        let inner = match (global_loading, sector_loading, sector_fraction) {
            (Some(gl), Some(sl), Some(sf)) => {
                MultiFactorCopula::with_loadings_and_sector_fraction(num_factors, gl, sl, sf)
            }
            (Some(gl), Some(sl), None) => MultiFactorCopula::with_loadings(num_factors, gl, sl),
            _ => MultiFactorCopula::new(num_factors),
        };
        JsMultiFactorCopula { inner }
    }

    /// Inter-sector correlation (beta_G^2).
    #[wasm_bindgen(getter, js_name = interSectorCorrelation)]
    pub fn inter_sector_correlation(&self) -> f64 {
        self.inner.inter_sector_correlation()
    }

    /// Intra-sector correlation (beta_G^2 + beta_S^2).
    #[wasm_bindgen(getter, js_name = intraSectorCorrelation)]
    pub fn intra_sector_correlation(&self) -> f64 {
        self.inner.intra_sector_correlation()
    }

    /// Decompose total correlation into [globalLoading, sectorLoading].
    ///
    /// @returns Float64Array of [globalLoading, sectorLoading].
    #[wasm_bindgen(js_name = decomposeCorrelation)]
    pub fn decompose_correlation(
        &self,
        total_correlation: f64,
        sector_fraction: f64,
    ) -> Vec<f64> {
        let (gl, sl) = self
            .inner
            .decompose_correlation(total_correlation, sector_fraction);
        vec![gl, sl]
    }

    /// Conditional default probability P(default | Z_G, Z_S).
    #[wasm_bindgen(js_name = conditionalDefaultProb)]
    pub fn conditional_default_prob(
        &self,
        default_threshold: f64,
        factor_realization: Vec<f64>,
        correlation: f64,
    ) -> f64 {
        self.inner
            .conditional_default_prob(default_threshold, &factor_realization, correlation)
    }

    /// Number of systematic factors.
    #[wasm_bindgen(js_name = numFactors)]
    pub fn num_factors(&self) -> usize {
        self.inner.num_factors()
    }

    /// Model name for diagnostics.
    #[wasm_bindgen(js_name = modelName)]
    pub fn model_name(&self) -> String {
        self.inner.model_name().to_string()
    }

    /// Lower-tail dependence (always 0 for multi-factor Gaussian).
    #[wasm_bindgen(js_name = tailDependence)]
    pub fn tail_dependence(&self, correlation: f64) -> f64 {
        self.inner.tail_dependence(correlation)
    }

    /// Get string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "MultiFactorCopula(numFactors={}, inter={:.4}, intra={:.4})",
            self.inner.num_factors(),
            self.inner.inter_sector_correlation(),
            self.inner.intra_sector_correlation()
        )
    }
}

// ---------------------------------------------------------------------------
// RandomFactorLoadingCopula
// ---------------------------------------------------------------------------

/// Random Factor Loading copula with stochastic correlation.
///
/// Models correlation itself as random, capturing increased correlation
/// during market stress. Important for senior tranche pricing.
///
/// @example
/// ```javascript
/// const copula = new RandomFactorLoadingCopula(0.15);
/// console.log(copula.loadingVolatility); // 0.15
/// ```
#[wasm_bindgen(js_name = RandomFactorLoadingCopula)]
pub struct JsRandomFactorLoadingCopula {
    inner: RandomFactorLoadingCopula,
}

impl JsRandomFactorLoadingCopula {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: RandomFactorLoadingCopula) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &RandomFactorLoadingCopula {
        &self.inner
    }
}

#[wasm_bindgen(js_class = RandomFactorLoadingCopula)]
impl JsRandomFactorLoadingCopula {
    /// Create an RFL copula.
    ///
    /// @param loadingVolatility - Volatility of the factor loading (clamped to [0, 0.5]). Typical: 0.05-0.20.
    /// @param quadratureOrder - Number of quadrature points (default 20).
    #[wasm_bindgen(constructor)]
    pub fn new(
        loading_volatility: f64,
        quadrature_order: Option<u8>,
    ) -> JsRandomFactorLoadingCopula {
        let inner = match quadrature_order {
            Some(order) => {
                RandomFactorLoadingCopula::with_quadrature_order(loading_volatility, order)
            }
            None => RandomFactorLoadingCopula::new(loading_volatility),
        };
        JsRandomFactorLoadingCopula { inner }
    }

    /// Loading volatility.
    #[wasm_bindgen(getter, js_name = loadingVolatility)]
    pub fn loading_volatility(&self) -> f64 {
        self.inner.loading_volatility()
    }

    /// Conditional default probability P(default | Z, eta).
    #[wasm_bindgen(js_name = conditionalDefaultProb)]
    pub fn conditional_default_prob(
        &self,
        default_threshold: f64,
        factor_realization: Vec<f64>,
        correlation: f64,
    ) -> f64 {
        self.inner
            .conditional_default_prob(default_threshold, &factor_realization, correlation)
    }

    /// Number of systematic factors (2: market + loading shock).
    #[wasm_bindgen(js_name = numFactors)]
    pub fn num_factors(&self) -> usize {
        self.inner.num_factors()
    }

    /// Model name for diagnostics.
    #[wasm_bindgen(js_name = modelName)]
    pub fn model_name(&self) -> String {
        self.inner.model_name().to_string()
    }

    /// Stress-dependence gauge (monotone proxy, not strict lambda_L).
    #[wasm_bindgen(js_name = tailDependence)]
    pub fn tail_dependence(&self, correlation: f64) -> f64 {
        self.inner.tail_dependence(correlation)
    }

    /// Get string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "RandomFactorLoadingCopula(loadingVol={:.4}, numFactors={})",
            self.inner.loading_volatility(),
            self.inner.num_factors()
        )
    }
}

// ---------------------------------------------------------------------------
// CopulaSpec
// ---------------------------------------------------------------------------

/// Copula model specification for configuration and serialization.
///
/// Allows copula selection and deferred construction.
///
/// @example
/// ```javascript
/// const spec = CopulaSpec.gaussian();
/// console.log(spec.isGaussian()); // true
/// const json = spec.toJson();
/// const restored = CopulaSpec.fromJson(json);
/// ```
#[wasm_bindgen(js_name = CopulaSpec)]
pub struct JsCopulaSpec {
    inner: CopulaSpec,
}

impl JsCopulaSpec {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: CopulaSpec) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &CopulaSpec {
        &self.inner
    }
}

#[wasm_bindgen(js_class = CopulaSpec)]
impl JsCopulaSpec {
    /// Create a Gaussian copula specification.
    #[wasm_bindgen(js_name = gaussian)]
    pub fn gaussian() -> JsCopulaSpec {
        JsCopulaSpec {
            inner: CopulaSpec::gaussian(),
        }
    }

    /// Create a Student-t copula specification.
    ///
    /// @param degreesOfFreedom - Must be > 2 for finite variance.
    #[wasm_bindgen(js_name = studentT)]
    pub fn student_t(degrees_of_freedom: f64) -> Result<JsCopulaSpec, JsValue> {
        if degrees_of_freedom <= 2.0 {
            return Err(js_error_with_kind(
                ErrorKind::Validation,
                "Student-t degrees_of_freedom must be > 2 for finite variance",
            ));
        }
        Ok(JsCopulaSpec {
            inner: CopulaSpec::student_t(degrees_of_freedom),
        })
    }

    /// Create a Random Factor Loading specification.
    ///
    /// @param loadingVolatility - Volatility of factor loading (clamped to [0, 0.5]).
    #[wasm_bindgen(js_name = randomFactorLoading)]
    pub fn random_factor_loading(loading_volatility: f64) -> JsCopulaSpec {
        JsCopulaSpec {
            inner: CopulaSpec::random_factor_loading(loading_volatility),
        }
    }

    /// Create a multi-factor copula specification.
    ///
    /// @param numFactors - Number of systematic factors.
    #[wasm_bindgen(js_name = multiFactor)]
    pub fn multi_factor(num_factors: usize) -> JsCopulaSpec {
        JsCopulaSpec {
            inner: CopulaSpec::multi_factor(num_factors),
        }
    }

    /// Whether this is a Gaussian copula specification.
    #[wasm_bindgen(js_name = isGaussian)]
    pub fn is_gaussian(&self) -> bool {
        self.inner.is_gaussian()
    }

    /// Whether this is a Student-t copula specification.
    #[wasm_bindgen(js_name = isStudentT)]
    pub fn is_student_t(&self) -> bool {
        self.inner.is_student_t()
    }

    /// Whether this is a Random Factor Loading specification.
    #[wasm_bindgen(js_name = isRfl)]
    pub fn is_rfl(&self) -> bool {
        self.inner.is_rfl()
    }

    /// Whether this is a multi-factor specification.
    #[wasm_bindgen(js_name = isMultiFactor)]
    pub fn is_multi_factor(&self) -> bool {
        self.inner.is_multi_factor()
    }

    /// Serialize to JSON string.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner)
            .map_err(|e| js_error(format!("Serialization failed: {e}")))
    }

    /// Deserialize from JSON string.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json: &str) -> Result<JsCopulaSpec, JsValue> {
        let inner: CopulaSpec = serde_json::from_str(json)
            .map_err(|e| js_error(format!("Deserialization failed: {e}")))?;
        Ok(JsCopulaSpec { inner })
    }

    /// Get string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("{:?}", self.inner)
    }
}
