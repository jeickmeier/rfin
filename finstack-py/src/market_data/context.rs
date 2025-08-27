//! Python bindings for MarketContext (lightweight aggregator for pricing input).

use pyo3::prelude::*;

use super::curve_set::PyCurveSet;
use super::fx::PyFxMatrix;

#[pyclass(name = "MarketContext", module = "finstack.market_data")]
pub struct PyMarketContext {
    curves: Py<PyCurveSet>,
    fx: Option<Py<PyFxMatrix>>,
}

#[pymethods]
impl PyMarketContext {
    /// Create an empty MarketContext with an empty CurveSet and no FX.
    #[new]
    pub fn new() -> Self {
        let curves_obj = Python::with_gil(|py| Py::new(py, PyCurveSet::empty()).unwrap());
        Self { curves: curves_obj, fx: None }
    }

    /// Get a handle to the CurveSet. This shares storage with the context.
    #[getter]
    pub fn curves<'py>(&self, py: Python<'py>) -> Py<PyCurveSet> { self.curves.clone_ref(py) }

    /// Replace the entire CurveSet.
    pub fn set_curves(&mut self, curves: Py<PyCurveSet>) { self.curves = curves; }

    /// Set FX using an existing FxMatrix.
    pub fn set_fx_matrix(&mut self, matrix: Py<PyFxMatrix>) { self.fx = Some(matrix); }

    /// Remove any FX matrix.
    pub fn clear_fx(&mut self) { self.fx = None; }

    /// Whether FX is configured.
    #[getter]
    pub fn has_fx(&self) -> bool { self.fx.is_some() }

    /// Get the FX matrix if configured.
    pub fn fx_matrix(&self) -> Option<Py<PyFxMatrix>> {
        Python::with_gil(|py| self.fx.as_ref().map(|m| m.clone_ref(py)))
    }
}

// No helpers needed; users can construct FxMatrix in Python and pass it by value.


