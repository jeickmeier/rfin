//! Market surfaces: two-dimensional grids such as implied volatility.
//!
//! Exposes a Python-friendly `VolSurface` with expiry and strike axes. Values
//! are interpolated across the supplied grid; helpers return either raw values,
//! checked values (error on out-of-bounds), or clamped values. Use this when
//! pricing options or any model requiring a volatility surface.
use crate::errors::core_to_py;
use finstack_core::market_data::surfaces::VolSurface;
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
        let surface = Python::attach(|py| {
            py.detach(|| {
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
    /// Interpolated volatility at `(expiry, strike)` with flat extrapolation.
    ///
    /// This method clamps coordinates to grid bounds, providing safe evaluation
    /// that never raises. Use `value_checked` for explicit error handling or
    /// `value_unchecked` when bounds are guaranteed.
    fn value(&self, expiry: f64, strike: f64) -> f64 {
        self.inner.value_clamped(expiry, strike)
    }

    #[pyo3(text_signature = "(self, expiry, strike)")]
    /// Volatility at `(expiry, strike)`, raising an error if out of bounds.
    ///
    /// Use this method when you need explicit error handling for out-of-bounds
    /// coordinates rather than flat extrapolation.
    fn value_checked(&self, expiry: f64, strike: f64) -> PyResult<f64> {
        self.inner.value_checked(expiry, strike).map_err(core_to_py)
    }

    #[pyo3(text_signature = "(self, expiry, strike)")]
    /// Volatility at `(expiry, strike)`, clamping to the surface domain.
    ///
    /// Alias for `value()` - both use flat extrapolation (clamping to edge values).
    fn value_clamped(&self, expiry: f64, strike: f64) -> f64 {
        self.inner.value_clamped(expiry, strike)
    }

    #[pyo3(text_signature = "(self, expiry, strike)")]
    /// Volatility at `(expiry, strike)` without bounds checking.
    ///
    /// Panics if `expiry` or `strike` is outside the grid bounds.
    /// Use only when bounds are guaranteed by the caller.
    fn value_unchecked(&self, expiry: f64, strike: f64) -> f64 {
        self.inner.value_unchecked(expiry, strike)
    }

    #[pyo3(text_signature = "(self, expiry, strike, bump_pct)")]
    /// Return a new surface with a single point bumped.
    ///
    /// Parameters
    /// ----------
    /// expiry : float
    ///     Expiry time in years.
    /// strike : float
    ///     Strike value.
    /// bump_pct : float
    ///     Relative bump percentage (e.g., 0.01 for 1%).
    ///
    /// Returns
    /// -------
    /// VolSurface
    ///     New surface with the specified point bumped.
    fn bump_point(&self, expiry: f64, strike: f64, bump_pct: f64) -> PyResult<Self> {
        let new_surface = self
            .inner
            .bump_point(expiry, strike, bump_pct)
            .map_err(core_to_py)?;
        Ok(Self::new_arc(Arc::new(new_surface)))
    }

    #[pyo3(text_signature = "(self, scale)")]
    /// Return a new surface with all volatilities scaled by a factor.
    ///
    /// Parameters
    /// ----------
    /// scale : float
    ///     Scaling factor (e.g., 1.1 for 10% increase).
    ///
    /// Returns
    /// -------
    /// VolSurface
    ///     New surface with scaled volatilities.
    fn scaled(&self, scale: f64) -> Self {
        let new_surface = self.inner.scaled(scale);
        Self::new_arc(Arc::new(new_surface))
    }

    #[pyo3(signature = (pct, expiries_filter=None, strikes_filter=None))]
    #[pyo3(text_signature = "(self, pct, expiries_filter=None, strikes_filter=None)")]
    /// Apply a bucket bump to volatilities matching the filters.
    ///
    /// Parameters
    /// ----------
    /// pct : float
    ///     Percentage bump to apply (e.g. 1.0 for 1% bump).
    /// expiries_filter : list[float], optional
    ///     List of expiries to bump. If None, all expiries are bumped.
    /// strikes_filter : list[float], optional
    ///     List of strikes to bump. If None, all strikes are bumped.
    ///
    /// Returns
    /// -------
    /// VolSurface
    ///     New surface with applied bumps.
    fn apply_bucket_bump(
        &self,
        pct: f64,
        expiries_filter: Option<Vec<f64>>,
        strikes_filter: Option<Vec<f64>>,
    ) -> PyResult<Self> {
        let new_surface = self
            .inner
            .apply_bucket_bump(expiries_filter.as_deref(), strikes_filter.as_deref(), pct)
            .ok_or_else(|| PyValueError::new_err("Failed to apply bucket bump"))?;
        Ok(Self::new_arc(Arc::new(new_surface)))
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
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
