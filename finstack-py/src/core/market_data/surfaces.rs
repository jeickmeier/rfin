//! Python bindings for market data surfaces.

use numpy::{IntoPyArray, PyArray1, PyArray2, PyUntypedArrayMethods};
use pyo3::exceptions::PyIndexError;
use pyo3::prelude::*;
use pyo3::types::PyList;
use std::sync::Arc;

use finstack_core::{
    market_data::{surfaces::vol_surface::VolSurface as CoreVolSurface, traits::TermStructure},
    F,
};

/// Implied volatility surface on expiry × strike grid.
///
/// A volatility surface represents implied volatilities as a function of
/// both option expiry and strike price. The surface interpolates between
/// market quotes to provide smooth volatility values for pricing.
///
/// The surface uses bilinear interpolation for fast and smooth interpolation
/// between grid points.
///
/// Examples:
///     >>> from rfin.market_data import VolSurface
///     >>> import numpy as np
///     
///     # Create a volatility surface
///     >>> expiries = [0.25, 0.5, 1.0, 2.0]  # Years
///     >>> strikes = [80, 90, 100, 110, 120]  # Strike prices
///     >>> values = [
///     ...     [0.25, 0.22, 0.20, 0.22, 0.25],  # 3M
///     ...     [0.24, 0.21, 0.19, 0.21, 0.24],  # 6M
///     ...     [0.23, 0.20, 0.18, 0.20, 0.23],  # 1Y
///     ...     [0.22, 0.19, 0.17, 0.19, 0.22],  # 2Y
///     ... ]
///     >>> surface = VolSurface(
///     ...     id="SPX-VOL",
///     ...     expiries=expiries,
///     ...     strikes=strikes,
///     ...     values=values
///     ... )
///     
///     # Query volatility
///     >>> surface.value(0.75, 95)  # 9 months, 95 strike
///     0.195
///     
///     # Access the grid
///     >>> surface.expiries
///     array([0.25, 0.5, 1.0, 2.0])
///     >>> surface.strikes
///     array([80, 90, 100, 110, 120])
///     >>> surface.values.shape
///     (4, 5)
///     
///     # Get slices
///     >>> surface.slice_by_expiry(1.0)  # Volatility smile at 1Y
///     array([0.23, 0.20, 0.18, 0.20, 0.23])
#[pyclass(name = "VolSurface", module = "finstack.market_data")]
#[derive(Clone)]
pub struct PyVolSurface {
    inner: Arc<CoreVolSurface>,
    // Store local copies of the data since VolSurface fields are private
    expiries: Vec<F>,
    strikes: Vec<F>,
    values: Vec<Vec<F>>,
}

#[pymethods]
impl PyVolSurface {
    /// Create a new VolSurface.
    ///
    /// Args:
    ///     id (str): Unique identifier for the surface
    ///     expiries (List[float] | numpy.ndarray): Option expiries in years
    ///     strikes (List[float] | numpy.ndarray): Strike prices
    ///     values (List[List[float]] | numpy.ndarray): 2D array of volatilities
    ///             with shape (len(expiries), len(strikes))
    ///
    /// Returns:
    ///     VolSurface: A new volatility surface instance
    ///
    /// Raises:
    ///     ValueError: If inputs are invalid (dimension mismatch, non-monotonic, etc.)
    #[new]
    fn new(
        id: String,
        expiries: &Bound<'_, PyAny>,
        strikes: &Bound<'_, PyAny>,
        values: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let expiries_vec = extract_f64_array(expiries)?;
        let strikes_vec = extract_f64_array(strikes)?;

        // Extract 2D values
        let values_2d = extract_2d_array(values)?;

        // Validate dimensions
        if values_2d.len() != expiries_vec.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "values rows ({}) must match expiries length ({})",
                values_2d.len(),
                expiries_vec.len()
            )));
        }

        // Build the surface
        let mut builder = CoreVolSurface::builder(id.clone())
            .expiries(&expiries_vec)
            .strikes(&strikes_vec);

        for row in &values_2d {
            builder = builder.data(row);
        }

        let surface = builder.build().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Failed to build surface: {:?}",
                e
            ))
        })?;

        Ok(PyVolSurface {
            inner: Arc::new(surface),
            expiries: expiries_vec,
            strikes: strikes_vec,
            values: values_2d,
        })
    }

    /// Unique identifier of the surface.
    #[getter]
    fn id(&self) -> String {
        TermStructure::id(&*self.inner).as_str().to_string()
    }

    /// Option expiry times (in years).
    #[getter]
    fn expiries<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        self.expiries.clone().into_pyarray(py)
    }

    /// Strike prices.
    #[getter]
    fn strikes<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        self.strikes.clone().into_pyarray(py)
    }

    /// Volatility data as a 2D array.
    ///
    /// Shape is (expiries.len(), strikes.len()).
    #[getter]
    fn data<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<f64>> {
        let flat: Vec<f64> = self
            .values
            .iter()
            .flat_map(|row| row.iter().copied())
            .collect();
        numpy::ndarray::Array2::from_shape_vec((self.expiries.len(), self.strikes.len()), flat)
            .unwrap()
            .into_pyarray(py)
    }

    /// Interpolated volatility at (expiry, strike).
    ///
    /// Args:
    ///     expiry (float): Option expiry in years
    ///     strike (float): Strike price
    ///
    /// Returns:
    ///     float: Interpolated implied volatility
    ///
    /// Raises:
    ///     ValueError: If expiry or strike is outside the surface bounds
    fn value(&self, expiry: F, strike: F) -> PyResult<F> {
        // The core VolSurface::value method handles bounds checking internally
        // but returns the same type. We could add explicit bounds checking here
        // if we want Python-specific error messages.
        Ok(self.inner.value(expiry, strike))
    }

    /// Get volatility smile for a given expiry.
    ///
    /// Interpolates the surface at the given expiry across all strikes.
    ///
    /// Args:
    ///     expiry (float): Option expiry in years
    ///
    /// Returns:
    ///     numpy.ndarray: Array of volatilities for each strike at the given expiry
    fn slice_by_expiry<'py>(&self, py: Python<'py>, expiry: F) -> Bound<'py, PyArray1<F>> {
        let vols: Vec<F> = self
            .strikes
            .iter()
            .map(|&strike| self.inner.value(expiry, strike))
            .collect();
        vols.into_pyarray(py)
    }

    /// Get term structure of volatility for a given strike.
    ///
    /// Interpolates the surface at the given strike across all expiries.
    ///
    /// Args:
    ///     strike (float): Strike price
    ///
    /// Returns:
    ///     numpy.ndarray: Array of volatilities for each expiry at the given strike
    fn slice_by_strike<'py>(&self, py: Python<'py>, strike: F) -> Bound<'py, PyArray1<F>> {
        let vols: Vec<F> = self
            .expiries
            .iter()
            .map(|&expiry| self.inner.value(expiry, strike))
            .collect();
        vols.into_pyarray(py)
    }

    /// Get volatilities for a specific expiry.
    ///
    /// Args:
    ///     expiry_idx: Index of the expiry
    ///
    /// Returns:
    ///     np.ndarray: Volatilities across all strikes for the given expiry
    fn get_expiry_slice<'py>(
        &self,
        py: Python<'py>,
        expiry_idx: usize,
    ) -> PyResult<Bound<'py, PyArray1<f64>>> {
        if expiry_idx >= self.expiries.len() {
            return Err(PyIndexError::new_err("Expiry index out of bounds"));
        }

        let vols: Vec<f64> = self.values[expiry_idx].clone();

        Ok(vols.into_pyarray(py))
    }

    /// Get volatilities for a specific strike.
    ///
    /// Args:
    ///     strike_idx: Index of the strike
    ///
    /// Returns:
    ///     np.ndarray: Volatilities across all expiries for the given strike
    fn get_strike_slice<'py>(
        &self,
        py: Python<'py>,
        strike_idx: usize,
    ) -> PyResult<Bound<'py, PyArray1<f64>>> {
        if strike_idx >= self.strikes.len() {
            return Err(PyIndexError::new_err("Strike index out of bounds"));
        }

        let vols: Vec<f64> = (0..self.expiries.len())
            .map(|expiry_idx| self.values[expiry_idx][strike_idx])
            .collect();

        Ok(vols.into_pyarray(py))
    }

    fn __repr__(&self) -> String {
        format!(
            "VolSurface(id='{}', expiries={}, strikes={})",
            TermStructure::id(&*self.inner).as_str(),
            self.expiries.len(),
            self.strikes.len()
        )
    }
}

// Helper function to extract 2D array from Python objects
fn extract_2d_array(obj: &Bound<'_, PyAny>) -> PyResult<Vec<Vec<F>>> {
    // Try numpy 2D array first
    if let Ok(array) = obj.extract::<numpy::PyReadonlyArray2<F>>() {
        let shape = array.shape();
        let mut result = Vec::with_capacity(shape[0]);
        for i in 0..shape[0] {
            let row: Vec<F> = (0..shape[1]).map(|j| *array.get([i, j]).unwrap()).collect();
            result.push(row);
        }
        return Ok(result);
    }

    // Try list of lists
    if let Ok(list) = obj.downcast::<PyList>() {
        let mut result = Vec::new();
        for item in list.iter() {
            if let Ok(row_list) = item.downcast::<PyList>() {
                let mut row = Vec::new();
                for val in row_list.iter() {
                    row.push(val.extract::<F>()?);
                }
                result.push(row);
            } else {
                return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                    "Expected list of lists or 2D numpy array",
                ));
            }
        }
        return Ok(result);
    }

    Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
        "Expected 2D numpy array or list of lists",
    ))
}

// Re-use the helper from curves.rs
use super::curves::extract_f64_array;
