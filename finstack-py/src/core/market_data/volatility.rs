//! Volatility bindings: conventions, pricing models, and conversion utilities.
//!
//! This module provides access to:
//! - `VolatilityConvention`: enum for Normal, Lognormal, and ShiftedLognormal conventions
//! - Pricing functions: `bachelier_price`, `black_price`, `black_shifted_price`
//! - `convert_atm_volatility`: utility to convert ATM volatility between conventions
use finstack_core::math::volatility::{
    bachelier_price, black_price, black_shifted_price, convert_atm_volatility, VolatilityConvention,
};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};
use pyo3::{Bound, PyRef};

/// Volatility quoting convention.
///
/// Parameters
/// ----------
/// None
///     Instantiate via class methods or attributes.
///
/// Returns
/// -------
/// VolatilityConvention
///     Convention descriptor.
#[pyclass(
    name = "VolatilityConvention",
    module = "finstack.core.market_data.volatility",
    frozen
)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PyVolatilityConvention {
    pub(crate) inner: VolatilityConvention,
}

impl PyVolatilityConvention {
    pub(crate) fn new(inner: VolatilityConvention) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyVolatilityConvention {
    #[classattr]
    const NORMAL: Self = Self {
        inner: VolatilityConvention::Normal,
    };

    #[classattr]
    const LOGNORMAL: Self = Self {
        inner: VolatilityConvention::Lognormal,
    };

    #[classmethod]
    #[pyo3(text_signature = "(cls, shift)")]
    /// Create a Shifted Lognormal convention with the specified shift.
    fn shifted_lognormal(_cls: &Bound<'_, PyType>, shift: f64) -> Self {
        Self::new(VolatilityConvention::ShiftedLognormal { shift })
    }

    #[getter]
    /// String representation of the convention type.
    fn kind(&self) -> &'static str {
        match self.inner {
            VolatilityConvention::Normal => "normal",
            VolatilityConvention::Lognormal => "lognormal",
            VolatilityConvention::ShiftedLognormal { .. } => "shifted_lognormal",
        }
    }

    #[getter]
    /// Shift amount (only for Shifted Lognormal, else None).
    fn shift(&self) -> Option<f64> {
        match self.inner {
            VolatilityConvention::ShiftedLognormal { shift } => Some(shift),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self.inner {
            VolatilityConvention::Normal => "VolatilityConvention.NORMAL".to_string(),
            VolatilityConvention::Lognormal => "VolatilityConvention.LOGNORMAL".to_string(),
            VolatilityConvention::ShiftedLognormal { shift } => {
                format!("VolatilityConvention.shifted_lognormal(shift={})", shift)
            }
        }
    }
}

/// Compute the price of a call option under the Bachelier (Normal) model.
///
/// Assumes a unit annuity (PV01=1).
///
/// Parameters
/// ----------
/// forward : float
///     Forward rate.
/// strike : float
///     Strike rate.
/// sigma_n : float
///     Normal volatility (in absolute units, e.g. 0.01 for 100bps).
/// t : float
///     Time to expiry in years.
///
/// Returns
/// -------
/// float
///     Option price.
#[pyfunction(
    name = "bachelier_price",
    text_signature = "(forward, strike, sigma_n, t)"
)]
pub fn py_bachelier_price(forward: f64, strike: f64, sigma_n: f64, t: f64) -> f64 {
    bachelier_price(forward, strike, sigma_n, t)
}

/// Compute the price of a call option under the Black (Lognormal) model.
///
/// Assumes a unit annuity (PV01=1).
///
/// Parameters
/// ----------
/// forward : float
///     Forward rate.
/// strike : float
///     Strike rate.
/// sigma : float
///     Lognormal volatility (decimal, e.g. 0.20 for 20%).
/// t : float
///     Time to expiry in years.
///
/// Returns
/// -------
/// float
///     Option price.
#[pyfunction(name = "black_price", text_signature = "(forward, strike, sigma, t)")]
pub fn py_black_price(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    black_price(forward, strike, sigma, t)
}

/// Compute the price of a call option under the Shifted Black model.
///
/// Parameters
/// ----------
/// forward : float
///     Forward rate.
/// strike : float
///     Strike rate.
/// sigma : float
///     Lognormal volatility.
/// t : float
///     Time to expiry in years.
/// shift : float
///     Shift amount applied to forward and strike.
///
/// Returns
/// -------
/// float
///     Option price.
#[pyfunction(
    name = "black_shifted_price",
    text_signature = "(forward, strike, sigma, t, shift)"
)]
pub fn py_black_shifted_price(forward: f64, strike: f64, sigma: f64, t: f64, shift: f64) -> f64 {
    black_shifted_price(forward, strike, sigma, t, shift)
}

/// Convert ATM volatility between conventions by equating option prices.
///
/// This function performs ATM (at-the-money, strike = forward) volatility conversion.
/// For surface-aware or strike-specific conversions, use a volatility surface.
///
/// Parameters
/// ----------
/// vol : float
///     Input volatility (must be positive and finite).
/// from_convention : VolatilityConvention
///     Source convention.
/// to_convention : VolatilityConvention
///     Target convention.
/// forward_rate : float
///     Forward rate for the underlying.
/// time_to_expiry : float
///     Time to expiry in years (must be non-negative).
///
/// Returns
/// -------
/// float
///     Converted volatility in the target convention.
///
/// Raises
/// ------
/// ValueError
///     If vol is not positive/finite, time_to_expiry is negative,
///     or forward_rate is non-positive for lognormal conventions.
#[pyfunction(
    name = "convert_atm_volatility",
    signature = (vol, from_convention, to_convention, forward_rate, time_to_expiry),
    text_signature = "(vol, from_convention, to_convention, forward_rate, time_to_expiry)"
)]
pub fn py_convert_atm_volatility(
    vol: f64,
    from_convention: PyRef<PyVolatilityConvention>,
    to_convention: PyRef<PyVolatilityConvention>,
    forward_rate: f64,
    time_to_expiry: f64,
) -> PyResult<f64> {
    convert_atm_volatility(
        vol,
        from_convention.inner,
        to_convention.inner,
        forward_rate,
        time_to_expiry,
    )
    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

/// Convert volatility between conventions by equating option prices.
///
/// .. deprecated:: 0.2.0
///     Use :func:`convert_atm_volatility` instead, which provides explicit error handling.
///
/// Parameters
/// ----------
/// vol : float
///     Input volatility.
/// from_convention : VolatilityConvention
///     Source convention.
/// to_convention : VolatilityConvention
///     Target convention.
/// forward_rate : float
///     Forward rate for the underlying.
/// time_to_expiry : float
///     Time to expiry in years.
/// zero_threshold : float, optional
///     Threshold below which rates are considered zero (default 1e-8). **Ignored**.
///
/// Returns
/// -------
/// float
///     Converted volatility in the target convention. Returns input volatility on error.
#[pyfunction(
    name = "convert_volatility",
    signature = (vol, from_convention, to_convention, forward_rate, time_to_expiry, _zero_threshold=1e-8),
    text_signature = "(vol, from_convention, to_convention, forward_rate, time_to_expiry, zero_threshold=1e-8)"
)]
pub fn py_convert_volatility(
    vol: f64,
    from_convention: PyRef<PyVolatilityConvention>,
    to_convention: PyRef<PyVolatilityConvention>,
    forward_rate: f64,
    time_to_expiry: f64,
    _zero_threshold: f64,
) -> f64 {
    convert_atm_volatility(
        vol,
        from_convention.inner,
        to_convention.inner,
        forward_rate,
        time_to_expiry,
    )
    .unwrap_or(vol)
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "volatility")?;
    module.setattr(
        "__doc__",
        "Volatility conventions, pricing models, and conversion utilities.",
    )?;

    module.add_class::<PyVolatilityConvention>()?;
    module.add_function(wrap_pyfunction!(py_bachelier_price, &module)?)?;
    module.add_function(wrap_pyfunction!(py_black_price, &module)?)?;
    module.add_function(wrap_pyfunction!(py_black_shifted_price, &module)?)?;
    module.add_function(wrap_pyfunction!(py_convert_atm_volatility, &module)?)?;
    module.add_function(wrap_pyfunction!(py_convert_volatility, &module)?)?;

    let exports = [
        "VolatilityConvention",
        "bachelier_price",
        "black_price",
        "black_shifted_price",
        "convert_atm_volatility",
        "convert_volatility",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
