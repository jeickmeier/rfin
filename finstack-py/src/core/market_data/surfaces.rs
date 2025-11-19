//! Market surfaces: two-dimensional grids such as implied volatility.
//!
//! Exposes a Python-friendly `VolSurface` with expiry and strike axes. Values
//! are interpolated across the supplied grid; helpers return either raw values,
//! checked values (error on out-of-bounds), or clamped values. Use this when
//! pricing options or any model requiring a volatility surface.
use crate::errors::core_to_py;
use finstack_core::market_data::surfaces::vol_surface::VolSurface;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;
use std::sync::Arc;

/// Two-dimensional implied volatility surface.
///
/// Parameters
/// ----------
/// id : str
///     Identifier for the surface.
/// expiries : list[float]
///     Expiry axis expressed in year fractions.
/// strikes : list[float]
///     Strike axis values.
/// grid : list[list[float]]
///     Volatility grid matching ``expiries`` by ``strikes``.
///
/// Returns
/// -------
/// VolSurface
///     Surface wrapper with interpolation helpers.
#[pyclass(module = "finstack.core.market_data.surfaces", name = "VolSurface")]
#[derive(Clone)]
pub struct PyVolSurface {
    pub(crate) inner: Arc<VolSurface>,
}

impl PyVolSurface {
    pub(crate) fn new_arc(inner: Arc<VolSurface>) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyVolSurface {
    #[new]
    #[pyo3(text_signature = "(id, expiries, strikes, grid)")]
    /// Build a bilinear volatility surface from expiry and strike axes plus a 2D volatility grid.
    fn ctor(
        id: &str,
        expiries: Vec<f64>,
        strikes: Vec<f64>,
        grid: Vec<Vec<f64>>,
    ) -> PyResult<Self> {
        if expiries.is_empty() || strikes.is_empty() {
            return Err(PyValueError::new_err(
                "expiries and strikes must contain at least one value",
            ));
        }
        if grid.len() != expiries.len() {
            return Err(PyValueError::new_err(format!(
                "grid row count {} must match expiries length {}",
                grid.len(),
                expiries.len()
            )));
        }
        for (row_idx, row) in grid.iter().enumerate() {
            if row.len() != strikes.len() {
                return Err(PyValueError::new_err(format!(
                    "grid row {row_idx} length {} must match strikes length {}",
                    row.len(),
                    strikes.len()
                )));
            }
        }
        let surface = Python::with_gil(|py| {
            py.allow_threads(|| {
                let mut builder = VolSurface::builder(id)
                    .expiries(&expiries)
                    .strikes(&strikes);
                for row in &grid {
                    builder = builder.row(row);
                }
                builder.build().map_err(core_to_py)
            })
        })?;
        Ok(Self::new_arc(Arc::new(surface)))
    }

    #[getter]
    /// Surface identifier string.
    fn id(&self) -> String {
        self.inner.id().to_string()
    }

    #[getter]
    /// Expiry axis values supplied during construction.
    fn expiries(&self) -> Vec<f64> {
        self.inner.expiries().to_vec()
    }

    #[getter]
    /// Strike axis values supplied during construction.
    fn strikes(&self) -> Vec<f64> {
        self.inner.strikes().to_vec()
    }

    #[getter]
    /// Grid dimensions `(n_expiries, n_strikes)`.
    fn grid_shape(&self) -> (usize, usize) {
        self.inner.grid_shape()
    }

    #[pyo3(text_signature = "(self, expiry, strike)")]
    /// Interpolated volatility at `(expiry, strike)`.
    fn value(&self, expiry: f64, strike: f64) -> f64 {
        self.inner.value(expiry, strike)
    }

    #[pyo3(text_signature = "(self, expiry, strike)")]
    /// Volatility at `(expiry, strike)`, returning an error if out of bounds.
    fn value_checked(&self, expiry: f64, strike: f64) -> PyResult<f64> {
        self.inner.value_checked(expiry, strike).map_err(core_to_py)
    }

    #[pyo3(text_signature = "(self, expiry, strike)")]
    /// Volatility at `(expiry, strike)`, clamping to the surface domain.
    fn value_clamped(&self, expiry: f64, strike: f64) -> f64 {
        self.inner.value_clamped(expiry, strike)
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "surfaces")?;
    module.setattr(
        "__doc__",
        "Two-dimensional market surfaces (e.g. implied volatility grids).",
    )?;
    module.add_class::<PyVolSurface>()?;
    let exports = ["VolSurface"];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
