//! WASM bindings for the `finstack-correlation` crate.
//!
//! Exposes copula models, recovery models, and joint probability utilities
//! to JavaScript/TypeScript via `wasm-bindgen`.

use crate::utils::to_js_err;
use finstack_correlation::{self as corr, Copula, CopulaSpec, RecoveryModel};
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// CopulaSpec
// ---------------------------------------------------------------------------

/// Copula model specification for configuration and deferred construction.
#[wasm_bindgen(js_name = CopulaSpec)]
pub struct WasmCopulaSpec {
    #[wasm_bindgen(skip)]
    inner: CopulaSpec,
}

#[wasm_bindgen(js_class = CopulaSpec)]
impl WasmCopulaSpec {
    /// One-factor Gaussian copula (market standard).
    #[wasm_bindgen(js_name = gaussian)]
    pub fn gaussian() -> Self {
        Self {
            inner: CopulaSpec::gaussian(),
        }
    }

    /// Student-t copula with specified degrees of freedom (must be > 2).
    #[wasm_bindgen(js_name = studentT)]
    pub fn student_t(df: f64) -> Result<WasmCopulaSpec, JsValue> {
        if df <= 2.0 {
            return Err(to_js_err("Student-t degrees of freedom must be > 2"));
        }
        Ok(Self {
            inner: CopulaSpec::student_t(df),
        })
    }

    /// Random Factor Loading copula with stochastic correlation.
    #[wasm_bindgen(js_name = randomFactorLoading)]
    pub fn random_factor_loading(loading_vol: f64) -> Self {
        Self {
            inner: CopulaSpec::random_factor_loading(loading_vol),
        }
    }

    /// Multi-factor Gaussian copula with sector structure.
    #[wasm_bindgen(js_name = multiFactor)]
    pub fn multi_factor(num_factors: usize) -> Self {
        Self {
            inner: CopulaSpec::multi_factor(num_factors),
        }
    }

    /// Build a concrete copula from this specification.
    #[wasm_bindgen(js_name = build)]
    pub fn build(&self) -> WasmCopula {
        WasmCopula {
            inner: self.inner.build(),
        }
    }

    /// True if this is a Gaussian spec.
    #[wasm_bindgen(getter, js_name = isGaussian)]
    pub fn is_gaussian(&self) -> bool {
        self.inner.is_gaussian()
    }

    /// True if this is a Student-t spec.
    #[wasm_bindgen(getter, js_name = isStudentT)]
    pub fn is_student_t(&self) -> bool {
        self.inner.is_student_t()
    }
}

// ---------------------------------------------------------------------------
// Copula (trait object wrapper)
// ---------------------------------------------------------------------------

/// Concrete copula model for portfolio default correlation.
#[wasm_bindgen(js_name = Copula)]
pub struct WasmCopula {
    #[wasm_bindgen(skip)]
    inner: Box<dyn Copula + Send + Sync>,
}

#[wasm_bindgen(js_class = Copula)]
impl WasmCopula {
    /// Conditional default probability given factor realization(s).
    #[wasm_bindgen(js_name = conditionalDefaultProb)]
    pub fn conditional_default_prob(
        &self,
        default_threshold: f64,
        factor_realization: &[f64],
        correlation: f64,
    ) -> f64 {
        self.inner
            .conditional_default_prob(default_threshold, factor_realization, correlation)
    }

    /// Number of systematic factors in the model.
    #[wasm_bindgen(getter, js_name = numFactors)]
    pub fn num_factors(&self) -> usize {
        self.inner.num_factors()
    }

    /// Model name for diagnostics.
    #[wasm_bindgen(getter, js_name = modelName)]
    pub fn model_name(&self) -> String {
        self.inner.model_name().to_string()
    }

    /// Lower-tail dependence coefficient at the given correlation.
    #[wasm_bindgen(js_name = tailDependence)]
    pub fn tail_dependence(&self, correlation: f64) -> f64 {
        self.inner.tail_dependence(correlation)
    }
}

// ---------------------------------------------------------------------------
// RecoverySpec
// ---------------------------------------------------------------------------

/// Recovery model specification for configuration and deferred construction.
#[wasm_bindgen(js_name = RecoverySpec)]
pub struct WasmRecoverySpec {
    #[wasm_bindgen(skip)]
    inner: corr::RecoverySpec,
}

#[wasm_bindgen(js_class = RecoverySpec)]
impl WasmRecoverySpec {
    /// Constant recovery rate.
    #[wasm_bindgen(js_name = constant)]
    pub fn constant(rate: f64) -> Self {
        Self {
            inner: corr::RecoverySpec::constant(rate),
        }
    }

    /// Market-correlated (Andersen-Sidenius) stochastic recovery.
    #[wasm_bindgen(js_name = marketCorrelated)]
    pub fn market_correlated(mean: f64, vol: f64, correlation: f64) -> Self {
        Self {
            inner: corr::RecoverySpec::market_correlated(mean, vol, correlation),
        }
    }

    /// Expected (unconditional) recovery rate.
    #[wasm_bindgen(getter, js_name = expectedRecovery)]
    pub fn expected_recovery(&self) -> f64 {
        self.inner.expected_recovery()
    }

    /// Build a concrete recovery model from this specification.
    #[wasm_bindgen(js_name = build)]
    pub fn build(&self) -> WasmRecoveryModel {
        WasmRecoveryModel {
            inner: self.inner.build(),
        }
    }
}

// ---------------------------------------------------------------------------
// RecoveryModel (trait object wrapper)
// ---------------------------------------------------------------------------

/// Concrete recovery model for credit portfolio pricing.
#[wasm_bindgen(js_name = RecoveryModel)]
pub struct WasmRecoveryModel {
    #[wasm_bindgen(skip)]
    inner: Box<dyn RecoveryModel + Send + Sync>,
}

#[wasm_bindgen(js_class = RecoveryModel)]
impl WasmRecoveryModel {
    /// Expected (unconditional) recovery rate.
    #[wasm_bindgen(getter, js_name = expectedRecovery)]
    pub fn expected_recovery(&self) -> f64 {
        self.inner.expected_recovery()
    }

    /// Recovery conditional on the systematic market factor.
    #[wasm_bindgen(js_name = conditionalRecovery)]
    pub fn conditional_recovery(&self, market_factor: f64) -> f64 {
        self.inner.conditional_recovery(market_factor)
    }

    /// Loss given default (1 − recovery).
    #[wasm_bindgen(getter, js_name = lgd)]
    pub fn lgd(&self) -> f64 {
        self.inner.lgd()
    }

    /// Whether recovery varies with the market factor.
    #[wasm_bindgen(getter, js_name = isStochastic)]
    pub fn is_stochastic(&self) -> bool {
        self.inner.is_stochastic()
    }

    /// Model name for diagnostics.
    #[wasm_bindgen(getter, js_name = modelName)]
    pub fn model_name(&self) -> String {
        self.inner.model_name().to_string()
    }
}

// ---------------------------------------------------------------------------
// Module-level functions
// ---------------------------------------------------------------------------

/// Fréchet-Hoeffding correlation bounds for two Bernoulli marginals.
///
/// Returns `[rho_min, rho_max]`.
#[wasm_bindgen(js_name = correlationBounds)]
pub fn correlation_bounds(p1: f64, p2: f64) -> Vec<f64> {
    let (lo, hi) = corr::correlation_bounds(p1, p2);
    vec![lo, hi]
}

/// Joint probabilities for two correlated Bernoulli variables.
///
/// Returns `[p11, p10, p01, p00]`.
#[wasm_bindgen(js_name = jointProbabilities)]
pub fn joint_probabilities(p1: f64, p2: f64, correlation: f64) -> Vec<f64> {
    let (p11, p10, p01, p00) = corr::joint_probabilities(p1, p2, correlation);
    vec![p11, p10, p01, p00]
}
