//! Python bindings for credit migration infrastructure.
//!
//! Exposes rating scales, transition matrices, generator matrices,
//! matrix exponentiation (projection), and CTMC simulation from
//! `finstack_core::credit::migration` under `finstack.core.credit`.

use finstack_core::credit::migration::error::MigrationError;
use finstack_core::credit::migration::{
    projection, GeneratorMatrix, MigrationSimulator, RatingPath, RatingScale, TransitionMatrix,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};
use pyo3::Bound;
use rand::SeedableRng;
use rand_pcg::Pcg64;

// ---------------------------------------------------------------------------
// Error mapping
// ---------------------------------------------------------------------------

/// Map a `MigrationError` to a Python `ValueError`.
fn migration_to_py(err: MigrationError) -> PyErr {
    PyValueError::new_err(err.to_string())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Flatten an `n x n` matrix into a row-major `Vec<f64>`.
///
/// Shared helper for migration matrix wrappers that expose `as_list()` to
/// Python. Accepts any indexer via closure so that we don't need a direct
/// dependency on `nalgebra` from the binding crate.
fn flatten_square_matrix_row_major<F>(n: usize, at: F) -> Vec<f64>
where
    F: Fn(usize, usize) -> f64,
{
    let mut out = Vec::with_capacity(n * n);
    for i in 0..n {
        for j in 0..n {
            out.push(at(i, j));
        }
    }
    out
}

// ===================================================================
// PyRatingScale
// ===================================================================

/// An ordered set of rating states (e.g. S&P/Fitch scale).
///
/// Defines the row/column layout of transition and generator matrices.
/// States are identified by string labels; the last label is typically the
/// absorbing default state.
///
/// Examples
/// --------
/// >>> from finstack.core.credit import RatingScale
/// >>> scale = RatingScale.standard()
/// >>> scale.n_states
/// 10
/// >>> scale.index_of("BBB")
/// 3
#[pyclass(
    name = "RatingScale",
    module = "finstack.core.credit",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyRatingScale {
    pub(crate) inner: RatingScale,
}

#[pymethods]
impl PyRatingScale {
    /// Standard 10-state S&P/Fitch scale: AAA, AA, A, BBB, BB, B, CCC, CC, C, D.
    #[classmethod]
    #[pyo3(text_signature = "()")]
    fn standard(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: RatingScale::standard(),
        }
    }

    /// 11-state scale with NR (not rated): AAA .. C, NR, D.
    #[classmethod]
    #[pyo3(text_signature = "()")]
    fn standard_with_nr(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: RatingScale::standard_with_nr(),
        }
    }

    /// 22-state notched scale: AAA, AA+, AA, AA-, .. CC, C, D.
    #[classmethod]
    #[pyo3(text_signature = "()")]
    fn notched(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: RatingScale::notched(),
        }
    }

    /// Custom scale where the last label is the default (absorbing) state.
    ///
    /// Parameters
    /// ----------
    /// labels : list[str]
    ///     At least 2 unique state labels; the last is the default state.
    #[classmethod]
    #[pyo3(text_signature = "(labels)")]
    fn custom(_cls: &Bound<'_, PyType>, labels: Vec<String>) -> PyResult<Self> {
        let inner = RatingScale::custom(labels).map_err(migration_to_py)?;
        Ok(Self { inner })
    }

    /// Custom scale with an explicit default (absorbing) state label.
    ///
    /// Parameters
    /// ----------
    /// labels : list[str]
    ///     At least 2 unique state labels.
    /// default_label : str
    ///     Label of the absorbing default state (must be in ``labels``).
    #[classmethod]
    #[pyo3(text_signature = "(labels, default_label)")]
    fn custom_with_default(
        _cls: &Bound<'_, PyType>,
        labels: Vec<String>,
        default_label: String,
    ) -> PyResult<Self> {
        let inner =
            RatingScale::custom_with_default(labels, default_label).map_err(migration_to_py)?;
        Ok(Self { inner })
    }

    /// Number of states in the scale.
    #[getter]
    fn n_states(&self) -> usize {
        self.inner.n_states()
    }

    /// All state labels in order.
    #[getter]
    fn labels(&self) -> Vec<String> {
        self.inner.labels().to_vec()
    }

    /// Index of the absorbing default state, or ``None``.
    #[getter]
    fn default_state(&self) -> Option<usize> {
        self.inner.default_state()
    }

    /// Return the index of a label, or ``None`` if not found.
    ///
    /// Parameters
    /// ----------
    /// label : str
    ///     State label to look up.
    #[pyo3(text_signature = "($self, label)")]
    fn index_of(&self, label: &str) -> Option<usize> {
        self.inner.index_of(label)
    }

    /// Return the label for a given state index, or ``None`` if out of range.
    ///
    /// Parameters
    /// ----------
    /// index : int
    ///     Zero-based state index.
    #[pyo3(text_signature = "($self, index)")]
    fn label_of(&self, index: usize) -> Option<String> {
        self.inner.label_of(index).map(str::to_owned)
    }

    fn __repr__(&self) -> String {
        format!(
            "RatingScale(n_states={}, labels={:?})",
            self.inner.n_states(),
            self.inner.labels(),
        )
    }

    fn __str__(&self) -> String {
        format!("RatingScale[{}]", self.inner.labels().join(", "),)
    }

    fn __len__(&self) -> usize {
        self.inner.n_states()
    }
}

// ===================================================================
// PyTransitionMatrix
// ===================================================================

/// Row-stochastic N x N transition matrix over a fixed time horizon.
///
/// Entry ``(i, j)`` is the probability of migrating from state ``i`` to
/// state ``j`` during the matrix's horizon.
///
/// Parameters
/// ----------
/// scale : RatingScale
///     The rating scale defining row/column states.
/// data : list[float]
///     Row-major probabilities (length ``n * n``).
/// horizon : float
///     Time horizon in years (must be positive).
///
/// Examples
/// --------
/// >>> from finstack.core.credit import RatingScale, TransitionMatrix
/// >>> scale = RatingScale.custom(["A", "D"])
/// >>> tm = TransitionMatrix(scale, [0.99, 0.01, 0.0, 1.0], 1.0)
/// >>> tm.probability("A", "D")
/// 0.01
#[pyclass(
    name = "TransitionMatrix",
    module = "finstack.core.credit",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyTransitionMatrix {
    pub(crate) inner: TransitionMatrix,
}

#[pymethods]
impl PyTransitionMatrix {
    #[new]
    #[pyo3(text_signature = "(scale, data, horizon)")]
    fn new(scale: &PyRatingScale, data: Vec<f64>, horizon: f64) -> PyResult<Self> {
        let inner =
            TransitionMatrix::new(scale.inner.clone(), &data, horizon).map_err(migration_to_py)?;
        Ok(Self { inner })
    }

    /// Time horizon in years.
    #[getter]
    fn horizon(&self) -> f64 {
        self.inner.horizon()
    }

    /// Number of states.
    #[getter]
    fn n_states(&self) -> usize {
        self.inner.n_states()
    }

    /// The rating scale.
    #[getter]
    fn scale(&self) -> PyRatingScale {
        PyRatingScale {
            inner: self.inner.scale().clone(),
        }
    }

    /// Transition probability from one state to another (by label).
    ///
    /// Parameters
    /// ----------
    /// from_state : str
    ///     Source state label.
    /// to_state : str
    ///     Target state label.
    ///
    /// Returns
    /// -------
    /// float
    ///     Transition probability P(from -> to).
    #[pyo3(text_signature = "($self, from_state, to_state)")]
    fn probability(&self, from_state: &str, to_state: &str) -> PyResult<f64> {
        self.inner
            .probability(from_state, to_state)
            .map_err(migration_to_py)
    }

    /// Row of transition probabilities from a given state.
    ///
    /// Parameters
    /// ----------
    /// from_state : str
    ///     Source state label.
    ///
    /// Returns
    /// -------
    /// list[float]
    ///     Probability vector for transitions from ``from_state``.
    #[pyo3(text_signature = "($self, from_state)")]
    fn row(&self, from_state: &str) -> PyResult<Vec<f64>> {
        self.inner.row(from_state).map_err(migration_to_py)
    }

    /// Compose two transition matrices: P(s+t) = P(s) * P(t).
    ///
    /// Both matrices must share the same rating scale.
    ///
    /// Parameters
    /// ----------
    /// other : TransitionMatrix
    ///     The other transition matrix to compose with.
    ///
    /// Returns
    /// -------
    /// TransitionMatrix
    ///     The composed transition matrix with horizon ``self.horizon + other.horizon``.
    #[pyo3(text_signature = "($self, other)")]
    fn compose(&self, other: &PyTransitionMatrix) -> PyResult<PyTransitionMatrix> {
        let inner = self.inner.compose(&other.inner).map_err(migration_to_py)?;
        Ok(PyTransitionMatrix { inner })
    }

    /// Default probability from each state (column of the default state).
    ///
    /// Returns ``None`` if no default state is defined on the scale.
    ///
    /// Returns
    /// -------
    /// list[float] | None
    ///     Probability of reaching the default state from each row.
    #[pyo3(text_signature = "($self)")]
    fn default_probabilities(&self) -> Option<Vec<f64>> {
        self.inner.default_probabilities()
    }

    /// Flat row-major matrix data.
    ///
    /// Returns
    /// -------
    /// list[float]
    ///     All ``n * n`` entries in row-major order.
    #[pyo3(text_signature = "($self)")]
    fn as_list(&self) -> Vec<f64> {
        let m = self.inner.as_matrix();
        flatten_square_matrix_row_major(m.nrows(), |i, j| m[(i, j)])
    }

    fn __repr__(&self) -> String {
        format!(
            "TransitionMatrix(n_states={}, horizon={})",
            self.inner.n_states(),
            self.inner.horizon(),
        )
    }
}

// ===================================================================
// PyGeneratorMatrix
// ===================================================================

/// Continuous-time generator (intensity) matrix Q for a CTMC.
///
/// Off-diagonal entry ``q_ij`` (i != j) is the instantaneous transition
/// rate from state i to state j.  Diagonal ``q_ii = -sum_{j!=i} q_ij``
/// so rows sum to zero.
///
/// Parameters
/// ----------
/// scale : RatingScale
///     The rating scale defining states.
/// data : list[float]
///     Row-major entries (length ``n * n``).
///
/// Examples
/// --------
/// >>> from finstack.core.credit import RatingScale, GeneratorMatrix
/// >>> scale = RatingScale.custom(["A", "D"])
/// >>> gen = GeneratorMatrix(scale, [-0.05, 0.05, 0.0, 0.0])
#[pyclass(
    name = "GeneratorMatrix",
    module = "finstack.core.credit",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyGeneratorMatrix {
    pub(crate) inner: GeneratorMatrix,
}

#[pymethods]
impl PyGeneratorMatrix {
    #[new]
    #[pyo3(text_signature = "(scale, data)")]
    fn new(scale: &PyRatingScale, data: Vec<f64>) -> PyResult<Self> {
        let inner = GeneratorMatrix::new(scale.inner.clone(), &data).map_err(migration_to_py)?;
        Ok(Self { inner })
    }

    /// Extract a generator from an annual transition matrix via matrix logarithm.
    ///
    /// Uses real Schur decomposition + Kreinin-Sidenius post-processing.
    /// Default round-trip tolerance is 1e-2.
    ///
    /// Parameters
    /// ----------
    /// p : TransitionMatrix
    ///     Annual transition matrix to extract the generator from.
    ///
    /// Returns
    /// -------
    /// GeneratorMatrix
    ///     The extracted continuous-time generator.
    #[classmethod]
    #[pyo3(text_signature = "(p)")]
    fn from_transition_matrix(_cls: &Bound<'_, PyType>, p: &PyTransitionMatrix) -> PyResult<Self> {
        let inner = GeneratorMatrix::from_transition_matrix(&p.inner).map_err(migration_to_py)?;
        Ok(Self { inner })
    }

    /// Extract a generator with a custom round-trip tolerance.
    ///
    /// Parameters
    /// ----------
    /// p : TransitionMatrix
    ///     Annual transition matrix.
    /// tol : float
    ///     Round-trip tolerance for ``||exp(Q) - P||_inf``.
    ///
    /// Returns
    /// -------
    /// GeneratorMatrix
    ///     The extracted continuous-time generator.
    #[classmethod]
    #[pyo3(text_signature = "(p, tol)")]
    fn from_transition_matrix_with_tol(
        _cls: &Bound<'_, PyType>,
        p: &PyTransitionMatrix,
        tol: f64,
    ) -> PyResult<Self> {
        let inner = GeneratorMatrix::from_transition_matrix_with_tol(&p.inner, tol)
            .map_err(migration_to_py)?;
        Ok(Self { inner })
    }

    /// Number of states.
    #[getter]
    fn n_states(&self) -> usize {
        self.inner.n_states()
    }

    /// The rating scale.
    #[getter]
    fn scale(&self) -> PyRatingScale {
        PyRatingScale {
            inner: self.inner.scale().clone(),
        }
    }

    /// Transition intensity q_ij looked up by state labels.
    ///
    /// Parameters
    /// ----------
    /// from_state : str
    ///     Source state label.
    /// to_state : str
    ///     Target state label.
    ///
    /// Returns
    /// -------
    /// float
    ///     Intensity rate q(from -> to).
    #[pyo3(text_signature = "($self, from_state, to_state)")]
    fn intensity(&self, from_state: &str, to_state: &str) -> PyResult<f64> {
        self.inner
            .intensity(from_state, to_state)
            .map_err(migration_to_py)
    }

    /// Total exit rate from a state: ``-q_ii``.
    ///
    /// Parameters
    /// ----------
    /// state : str
    ///     State label.
    ///
    /// Returns
    /// -------
    /// float
    ///     Exit rate (non-negative).
    #[pyo3(text_signature = "($self, state)")]
    fn exit_rate(&self, state: &str) -> PyResult<f64> {
        self.inner.exit_rate(state).map_err(migration_to_py)
    }

    /// Flat row-major matrix data.
    ///
    /// Returns
    /// -------
    /// list[float]
    ///     All ``n * n`` entries in row-major order.
    #[pyo3(text_signature = "($self)")]
    fn as_list(&self) -> Vec<f64> {
        let m = self.inner.as_matrix();
        flatten_square_matrix_row_major(m.nrows(), |i, j| m[(i, j)])
    }

    fn __repr__(&self) -> String {
        format!("GeneratorMatrix(n_states={})", self.inner.n_states(),)
    }
}

// ===================================================================
// PyRatingPath
// ===================================================================

/// A simulated rating trajectory from a CTMC migration simulation.
///
/// The path is piecewise-constant and right-continuous: at any time ``t``,
/// the state is the most recent transition at or before ``t``.
#[pyclass(
    name = "RatingPath",
    module = "finstack.core.credit",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyRatingPath {
    pub(crate) inner: RatingPath,
}

#[pymethods]
impl PyRatingPath {
    /// The simulation horizon.
    #[getter]
    fn horizon(&self) -> f64 {
        self.inner.horizon()
    }

    /// Number of transitions (excluding the initial state at t=0).
    #[getter]
    fn n_transitions(&self) -> usize {
        self.inner.n_transitions()
    }

    /// The rating scale associated with this path.
    #[getter]
    fn scale(&self) -> PyRatingScale {
        PyRatingScale {
            inner: self.inner.scale().clone(),
        }
    }

    /// State index at time ``t`` (piecewise-constant, right-continuous).
    ///
    /// Parameters
    /// ----------
    /// t : float
    ///     Time point in years.
    ///
    /// Returns
    /// -------
    /// int
    ///     State index at time ``t``.
    #[pyo3(text_signature = "($self, t)")]
    fn state_at(&self, t: f64) -> usize {
        self.inner.state_at(t)
    }

    /// State label at time ``t``.
    ///
    /// Parameters
    /// ----------
    /// t : float
    ///     Time point in years.
    ///
    /// Returns
    /// -------
    /// str
    ///     State label at time ``t``.
    #[pyo3(text_signature = "($self, t)")]
    fn label_at(&self, t: f64) -> String {
        self.inner.label_at(t).to_owned()
    }

    /// Whether the obligor defaulted during the simulation horizon.
    ///
    /// Returns
    /// -------
    /// bool
    #[pyo3(text_signature = "($self)")]
    fn defaulted(&self) -> bool {
        self.inner.defaulted()
    }

    /// Time of default, or ``None`` if no default occurred.
    ///
    /// Returns
    /// -------
    /// float | None
    #[pyo3(text_signature = "($self)")]
    fn default_time(&self) -> Option<f64> {
        self.inner.default_time()
    }

    /// All transition events as ``(time, state_index)`` pairs.
    ///
    /// The first element is always ``(0.0, initial_state)``.
    ///
    /// Returns
    /// -------
    /// list[tuple[float, int]]
    #[pyo3(text_signature = "($self)")]
    fn transitions(&self) -> Vec<(f64, usize)> {
        self.inner.transitions().to_vec()
    }

    fn __repr__(&self) -> String {
        format!(
            "RatingPath(horizon={}, n_transitions={})",
            self.inner.horizon(),
            self.inner.n_transitions(),
        )
    }
}

// ===================================================================
// PyMigrationSimulator
// ===================================================================

/// Simulator for generating rating paths from a generator matrix
/// using the Gillespie (competing exponentials) algorithm.
///
/// Parameters
/// ----------
/// generator : GeneratorMatrix
///     The continuous-time generator matrix.
/// horizon : float
///     Simulation horizon in years (must be positive).
///
/// Examples
/// --------
/// >>> from finstack.core.credit import RatingScale, GeneratorMatrix, MigrationSimulator
/// >>> scale = RatingScale.custom(["A", "D"])
/// >>> gen = GeneratorMatrix(scale, [-0.1, 0.1, 0.0, 0.0])
/// >>> sim = MigrationSimulator(gen, 5.0)
/// >>> paths = sim.simulate(0, 1000)
/// >>> len(paths)
/// 1000
#[pyclass(
    name = "MigrationSimulator",
    module = "finstack.core.credit",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyMigrationSimulator {
    pub(crate) inner: MigrationSimulator,
}

#[pymethods]
impl PyMigrationSimulator {
    #[new]
    #[pyo3(text_signature = "(generator, horizon)")]
    fn new(generator: &PyGeneratorMatrix, horizon: f64) -> PyResult<Self> {
        let inner =
            MigrationSimulator::new(generator.inner.clone(), horizon).map_err(migration_to_py)?;
        Ok(Self { inner })
    }

    /// The simulation horizon.
    #[getter]
    fn horizon(&self) -> f64 {
        self.inner.horizon()
    }

    /// Simulate independent rating paths from an initial state.
    ///
    /// Parameters
    /// ----------
    /// initial_state : int
    ///     Starting state index.
    /// n_paths : int
    ///     Number of paths to generate.
    /// seed : int | None
    ///     RNG seed (default 42 if not provided).
    ///
    /// Returns
    /// -------
    /// list[RatingPath]
    ///     Simulated rating trajectories.
    #[pyo3(text_signature = "($self, initial_state, n_paths, seed=None)")]
    fn simulate(
        &self,
        initial_state: usize,
        n_paths: usize,
        seed: Option<u64>,
    ) -> Vec<PyRatingPath> {
        let mut rng =
            Pcg64::seed_from_u64(seed.unwrap_or(finstack_valuations::constants::DEFAULT_SEED));
        self.inner
            .simulate(initial_state, n_paths, &mut rng)
            .into_iter()
            .map(|p| PyRatingPath { inner: p })
            .collect()
    }

    /// Estimate the transition matrix from batch simulation.
    ///
    /// Runs ``n_paths_per_state`` paths from every state and records the
    /// terminal state to build the empirical transition matrix.
    ///
    /// Parameters
    /// ----------
    /// n_paths_per_state : int
    ///     Number of paths per starting state.
    /// seed : int | None
    ///     RNG seed (default 42 if not provided).
    ///
    /// Returns
    /// -------
    /// TransitionMatrix
    ///     Empirical transition matrix estimated from simulation.
    #[pyo3(text_signature = "($self, n_paths_per_state, seed=None)")]
    fn empirical_matrix(&self, n_paths_per_state: usize, seed: Option<u64>) -> PyTransitionMatrix {
        let mut rng =
            Pcg64::seed_from_u64(seed.unwrap_or(finstack_valuations::constants::DEFAULT_SEED));
        let inner = self.inner.empirical_matrix(n_paths_per_state, &mut rng);
        PyTransitionMatrix { inner }
    }

    fn __repr__(&self) -> String {
        format!("MigrationSimulator(horizon={})", self.inner.horizon(),)
    }
}

// ===================================================================
// Free functions
// ===================================================================

/// Compute P(t) = exp(Q * t) using Pade scaling-and-squaring.
///
/// Parameters
/// ----------
/// generator : GeneratorMatrix
///     Continuous-time generator matrix Q.
/// t : float
///     Time horizon in years (must be positive).
///
/// Returns
/// -------
/// TransitionMatrix
///     The projected transition matrix P(t).
#[pyfunction(name = "project")]
#[pyo3(text_signature = "(generator, t)")]
fn project_py(generator: &PyGeneratorMatrix, t: f64) -> PyResult<PyTransitionMatrix> {
    let inner = projection::project(&generator.inner, t).map_err(migration_to_py)?;
    Ok(PyTransitionMatrix { inner })
}

/// Compute P(t) = exp(Q * t) using the [13/13] Pade method (explicit selection).
///
/// Equivalent to ``project()``; provided for API completeness.
///
/// Parameters
/// ----------
/// generator : GeneratorMatrix
///     Continuous-time generator matrix Q.
/// t : float
///     Time horizon in years (must be positive).
///
/// Returns
/// -------
/// TransitionMatrix
///     The projected transition matrix P(t).
#[pyfunction(name = "project_pade")]
#[pyo3(text_signature = "(generator, t)")]
fn project_pade_py(generator: &PyGeneratorMatrix, t: f64) -> PyResult<PyTransitionMatrix> {
    let inner = projection::project_pade(&generator.inner, t).map_err(migration_to_py)?;
    Ok(PyTransitionMatrix { inner })
}

// ===================================================================
// Module registration
// ===================================================================

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "credit")?;
    module.setattr(
        "__doc__",
        "Credit migration infrastructure: rating scales, transition matrices, \
         generator matrices, matrix exponentiation, and CTMC simulation.",
    )?;

    // Classes
    module.add_class::<PyRatingScale>()?;
    module.add_class::<PyTransitionMatrix>()?;
    module.add_class::<PyGeneratorMatrix>()?;
    module.add_class::<PyRatingPath>()?;
    module.add_class::<PyMigrationSimulator>()?;

    // Free functions
    module.add_function(wrap_pyfunction!(project_py, &module)?)?;
    module.add_function(wrap_pyfunction!(project_pade_py, &module)?)?;

    let exports = PyList::new(
        py,
        [
            "RatingScale",
            "TransitionMatrix",
            "GeneratorMatrix",
            "RatingPath",
            "MigrationSimulator",
            "project",
            "project_pade",
        ],
    )?;
    module.setattr("__all__", exports)?;

    parent.add_submodule(&module)?;
    Ok(())
}
