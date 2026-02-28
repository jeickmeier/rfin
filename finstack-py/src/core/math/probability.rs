//! Python bindings for joint probability utilities for correlated events.
//!
//! Provides functions and classes for computing joint probabilities of correlated
//! Bernoulli random variables, useful for credit modeling and scenario generation.

use finstack_core::math::probability::{
    correlation_bounds as core_correlation_bounds, joint_probabilities as core_joint_probabilities,
    CorrelatedBernoulli as CoreCorrelatedBernoulli,
};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

#[pyfunction(name = "joint_probabilities", text_signature = "(p1, p2, correlation)")]
/// Compute joint probabilities for two correlated Bernoulli random variables.
///
/// Given marginal probabilities p1 and p2 with correlation ρ, returns
/// the four joint probabilities (p11, p10, p01, p00) where:
///
/// - p11 = P(X₁=1, X₂=1)
/// - p10 = P(X₁=1, X₂=0)
/// - p01 = P(X₁=0, X₂=1)
/// - p00 = P(X₁=0, X₂=0)
///
/// The correlation is automatically clamped to the feasible Fréchet-Hoeffding bounds
/// to ensure valid joint probabilities while exactly preserving the marginals.
///
/// Args:
///     p1 (float): Marginal probability P(X₁=1), clamped to [0, 1].
///     p2 (float): Marginal probability P(X₂=1), clamped to [0, 1].
///     correlation (float): Correlation between X₁ and X₂, clamped to feasible bounds.
///
/// Returns:
///     tuple[float, float, float, float]: Tuple (p11, p10, p01, p00) that sums to 1.0
///         and exactly preserves marginals.
///
/// Use Cases:
///     - **Credit modeling**: Correlated default probabilities between obligors
///     - **Scenario generation**: Joint events in stress testing
///     - **Tree-based pricing**: Constructing correlated binomial trees
///
/// Examples:
///     >>> from finstack.core.math.probability import joint_probabilities
///     >>> p11, p10, p01, p00 = joint_probabilities(0.6, 0.4, 0.3)
///     >>> round(p11 + p10 + p01 + p00, 10)  # Sums to 1.0
///     1.0
///     >>> round(p11 + p10, 10)  # Marginal p1 preserved
///     0.6
///     >>> round(p11 + p01, 10)  # Marginal p2 preserved
///     0.4
pub fn joint_probabilities_py(p1: f64, p2: f64, correlation: f64) -> (f64, f64, f64, f64) {
    core_joint_probabilities(p1, p2, correlation)
}

#[pyfunction(name = "correlation_bounds", text_signature = "(p1, p2)")]
/// Compute the achievable correlation bounds for given marginal probabilities.
///
/// The Fréchet-Hoeffding bounds constrain the feasible correlation range
/// for two Bernoulli random variables. These bounds ensure that all joint
/// probabilities remain in [0, 1].
///
/// Args:
///     p1 (float): Marginal probability P(X₁=1).
///     p2 (float): Marginal probability P(X₂=1).
///
/// Returns:
///     tuple[float, float]: Tuple (ρ_min, ρ_max) of achievable correlation bounds.
///
/// Use Cases:
///     - **Validation**: Checking if a target correlation is achievable
///     - **Credit modeling**: Understanding correlation constraints for default models
///     - **Calibration**: Setting correlation bounds in optimization
///
/// Examples:
///     >>> from finstack.core.math.probability import correlation_bounds
///     >>> rho_min, rho_max = correlation_bounds(0.5, 0.5)
///     >>> rho_min < 0 < rho_max  # Can achieve both positive and negative correlation
///     True
///     >>> round(rho_max, 10)  # Perfect correlation possible when p1=p2=0.5
///     1.0
///
///     Asymmetric case:
///
///     >>> rho_min, rho_max = correlation_bounds(0.1, 0.9)
///     >>> rho_max < 1.0  # Perfect correlation not achievable with different marginals
///     True
pub fn correlation_bounds_py(p1: f64, p2: f64) -> (f64, f64) {
    core_correlation_bounds(p1, p2)
}

#[pyclass(
    name = "CorrelatedBernoulli",
    module = "finstack.core.math.probability",
    from_py_object
)]
/// Correlated Bernoulli distribution for scenario generation.
///
/// Provides methods for working with correlated binary outcomes,
/// useful for tree-based pricing and analytical calculations in credit modeling.
///
/// The class precomputes joint probabilities at construction time for efficient
/// repeated sampling and analysis.
///
/// Attributes:
///     p1 (float): Marginal probability of first event.
///     p2 (float): Marginal probability of second event.
///     correlation (float): Correlation between events (clamped to feasible bounds).
///
/// Use Cases:
///     - **Correlated default modeling**: Simulating joint defaults
///     - **Tree-based option pricing**: Building correlated binomial trees
///     - **Scenario generation**: Creating correlated binary outcomes
///     - **Credit portfolio analysis**: Modeling dependent credit events
///
/// Examples:
///     >>> from finstack.core.math.probability import CorrelatedBernoulli
///     >>> dist = CorrelatedBernoulli(0.5, 0.5, 0.5)
///     >>> dist.p1
///     0.5
///     >>> dist.p2
///     0.5
///
///     Sample correlated outcomes:
///
///     >>> x1, x2 = dist.sample_from_uniform(0.1)  # Returns (0 or 1, 0 or 1)
///
///     Analyze conditional probabilities:
///
///     >>> dist.conditional_p2_given_x1  # P(X₂=1 | X₁=1)
#[derive(Clone, Debug)]
pub struct PyCorrelatedBernoulli {
    inner: CoreCorrelatedBernoulli,
}

#[pymethods]
impl PyCorrelatedBernoulli {
    #[new]
    #[pyo3(text_signature = "(p1, p2, correlation)")]
    /// Create a correlated Bernoulli distribution.
    ///
    /// The correlation is automatically clamped to the Fréchet-Hoeffding bounds
    /// for the given marginal probabilities to ensure valid joint probabilities.
    ///
    /// Args:
    ///     p1 (float): Marginal probability of first event, clamped to [0, 1].
    ///     p2 (float): Marginal probability of second event, clamped to [0, 1].
    ///     correlation (float): Correlation between events, clamped to feasible bounds.
    pub fn new(p1: f64, p2: f64, correlation: f64) -> Self {
        Self {
            inner: CoreCorrelatedBernoulli::new(p1, p2, correlation),
        }
    }

    /// Marginal probability P(X₁=1).
    #[getter]
    pub fn p1(&self) -> f64 {
        self.inner.p1()
    }

    /// Marginal probability P(X₂=1).
    #[getter]
    pub fn p2(&self) -> f64 {
        self.inner.p2()
    }

    /// Correlation between X₁ and X₂.
    #[getter]
    pub fn correlation(&self) -> f64 {
        self.inner.correlation()
    }

    /// Joint probability P(X₁=1, X₂=1).
    #[getter]
    pub fn joint_p11(&self) -> f64 {
        self.inner.joint_p11()
    }

    /// Joint probability P(X₁=1, X₂=0).
    #[getter]
    pub fn joint_p10(&self) -> f64 {
        self.inner.joint_p10()
    }

    /// Joint probability P(X₁=0, X₂=1).
    #[getter]
    pub fn joint_p01(&self) -> f64 {
        self.inner.joint_p01()
    }

    /// Joint probability P(X₁=0, X₂=0).
    #[getter]
    pub fn joint_p00(&self) -> f64 {
        self.inner.joint_p00()
    }

    /// Conditional probability P(X₂=1 | X₁=1).
    #[getter]
    pub fn conditional_p2_given_x1(&self) -> f64 {
        self.inner.conditional_p2_given_x1()
    }

    /// Conditional probability P(X₁=1 | X₂=1).
    #[getter]
    pub fn conditional_p1_given_x2(&self) -> f64 {
        self.inner.conditional_p1_given_x2()
    }

    #[pyo3(text_signature = "($self)")]
    /// Get all four joint probabilities as a tuple.
    ///
    /// Returns:
    ///     tuple[float, float, float, float]: (p11, p10, p01, p00)
    pub fn joint_probabilities(&self) -> (f64, f64, f64, f64) {
        self.inner.joint_probabilities()
    }

    #[pyo3(text_signature = "($self, u)")]
    /// Sample a pair of correlated binary outcomes given a uniform random value.
    ///
    /// Args:
    ///     u (float): Uniform random value in [0, 1].
    ///
    /// Returns:
    ///     tuple[int, int]: Pair (x1, x2) where each is 0 or 1.
    ///
    /// Examples:
    ///     >>> from finstack.core.math.probability import CorrelatedBernoulli
    ///     >>> dist = CorrelatedBernoulli(0.5, 0.5, 0.0)
    ///     >>> x1, x2 = dist.sample_from_uniform(0.99)  # High u -> likely (0, 0)
    ///     >>> (x1, x2)
    ///     (0, 0)
    pub fn sample_from_uniform(&self, u: f64) -> (u8, u8) {
        self.inner.sample_from_uniform(u)
    }

    pub fn __repr__(&self) -> String {
        format!(
            "CorrelatedBernoulli(p1={:.4}, p2={:.4}, correlation={:.4})",
            self.inner.p1(),
            self.inner.p2(),
            self.inner.correlation()
        )
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "probability")?;
    module.setattr(
        "__doc__",
        concat!(
            "Joint probability utilities for correlated events.\n\n",
            "Provides functions and classes for computing joint probabilities of correlated\n",
            "Bernoulli random variables, useful for credit modeling and scenario generation.\n\n",
            "Functions:\n",
            "- joint_probabilities: Compute joint probs for correlated Bernoulli variables\n",
            "- correlation_bounds: Get achievable correlation bounds for given marginals\n\n",
            "Classes:\n",
            "- CorrelatedBernoulli: Distribution for correlated binary outcomes"
        ),
    )?;

    module.add_function(wrap_pyfunction!(joint_probabilities_py, &module)?)?;
    module.add_function(wrap_pyfunction!(correlation_bounds_py, &module)?)?;
    module.add_class::<PyCorrelatedBernoulli>()?;

    let exports = [
        "joint_probabilities",
        "correlation_bounds",
        "CorrelatedBernoulli",
    ];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
