//! Volatility bindings: conventions, pricing models, and conversion utilities.
//!
//! This module provides access to:
//! - `VolatilityConvention`: enum for Normal, Lognormal, and ShiftedLognormal conventions
//! - Pricing functions: `bachelier_price`, `black_price`, `black_shifted_price`
//! - `convert_atm_volatility`: utility to convert ATM volatility between conventions
use finstack_core::math::volatility::{
    bachelier_call, bachelier_delta_call, bachelier_delta_put, bachelier_gamma, bachelier_put,
    bachelier_vega, black_call, black_delta_call, black_delta_put, black_gamma, black_put,
    black_shifted_call, black_shifted_put, black_shifted_vega, black_vega,
    brenner_subrahmanyam_approx, convert_atm_volatility, implied_vol_bachelier, implied_vol_black,
    implied_vol_initial_guess, manaster_koehler_approx, VolatilityConvention,
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
    frozen,
    from_py_object
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
    bachelier_call(forward, strike, sigma_n, t)
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
    black_call(forward, strike, sigma, t)
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
    black_shifted_call(forward, strike, sigma, t, shift)
}

// =============================================================================
// Black-76 (Lognormal) Model — Individual Greeks
// =============================================================================

/// Compute the price of a call option under the Black-76 (Lognormal) model.
///
/// Assumes a unit annuity (PV01=1).
///
/// Parameters
/// ----------
/// forward : float
///     Forward rate (must be positive).
/// strike : float
///     Strike rate (must be positive).
/// sigma : float
///     Lognormal volatility (e.g. 0.20 for 20%).
/// t : float
///     Time to expiry in years.
///
/// Returns
/// -------
/// float
///     Call option price per unit annuity.
#[pyfunction(name = "black_call", signature = (forward, strike, sigma, t))]
pub fn py_black_call(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    black_call(forward, strike, sigma, t)
}

/// Compute the price of a put option under the Black-76 (Lognormal) model.
///
/// Assumes a unit annuity (PV01=1).
///
/// Parameters
/// ----------
/// forward : float
///     Forward rate (must be positive).
/// strike : float
///     Strike rate (must be positive).
/// sigma : float
///     Lognormal volatility (e.g. 0.20 for 20%).
/// t : float
///     Time to expiry in years.
///
/// Returns
/// -------
/// float
///     Put option price per unit annuity.
#[pyfunction(name = "black_put", signature = (forward, strike, sigma, t))]
pub fn py_black_put(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    black_put(forward, strike, sigma, t)
}

/// Compute Black-76 vega: sensitivity of option price to lognormal volatility.
///
/// Same for both calls and puts.
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
///
/// Returns
/// -------
/// float
///     Vega per unit change in vol (per unit annuity).
#[pyfunction(name = "black_vega", signature = (forward, strike, sigma, t))]
pub fn py_black_vega(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    black_vega(forward, strike, sigma, t)
}

/// Compute Black-76 call delta: sensitivity of call price to forward rate.
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
///
/// Returns
/// -------
/// float
///     Call delta (per unit annuity).
#[pyfunction(name = "black_delta_call", signature = (forward, strike, sigma, t))]
pub fn py_black_delta_call(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    black_delta_call(forward, strike, sigma, t)
}

/// Compute Black-76 put delta: sensitivity of put price to forward rate.
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
///
/// Returns
/// -------
/// float
///     Put delta (per unit annuity).
#[pyfunction(name = "black_delta_put", signature = (forward, strike, sigma, t))]
pub fn py_black_delta_put(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    black_delta_put(forward, strike, sigma, t)
}

/// Compute Black-76 gamma: second derivative of option price w.r.t. forward.
///
/// Same for both calls and puts.
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
///
/// Returns
/// -------
/// float
///     Gamma (per unit annuity).
#[pyfunction(name = "black_gamma", signature = (forward, strike, sigma, t))]
pub fn py_black_gamma(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    black_gamma(forward, strike, sigma, t)
}

// =============================================================================
// Bachelier (Normal) Model — Individual Greeks
// =============================================================================

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
///     Normal volatility (in rate terms, e.g. 0.005 = 50bp).
/// t : float
///     Time to expiry in years.
///
/// Returns
/// -------
/// float
///     Call option price per unit annuity.
#[pyfunction(name = "bachelier_call", signature = (forward, strike, sigma_n, t))]
pub fn py_bachelier_call(forward: f64, strike: f64, sigma_n: f64, t: f64) -> f64 {
    bachelier_call(forward, strike, sigma_n, t)
}

/// Compute the price of a put option under the Bachelier (Normal) model.
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
///     Normal volatility (in rate terms, e.g. 0.005 = 50bp).
/// t : float
///     Time to expiry in years.
///
/// Returns
/// -------
/// float
///     Put option price per unit annuity.
#[pyfunction(name = "bachelier_put", signature = (forward, strike, sigma_n, t))]
pub fn py_bachelier_put(forward: f64, strike: f64, sigma_n: f64, t: f64) -> f64 {
    bachelier_put(forward, strike, sigma_n, t)
}

/// Compute Bachelier vega: sensitivity of option price to normal volatility.
///
/// Same for both calls and puts.
///
/// Parameters
/// ----------
/// forward : float
///     Forward rate.
/// strike : float
///     Strike rate.
/// sigma_n : float
///     Normal volatility.
/// t : float
///     Time to expiry in years.
///
/// Returns
/// -------
/// float
///     Vega per unit change in normal vol (per unit annuity).
#[pyfunction(name = "bachelier_vega", signature = (forward, strike, sigma_n, t))]
pub fn py_bachelier_vega(forward: f64, strike: f64, sigma_n: f64, t: f64) -> f64 {
    bachelier_vega(forward, strike, sigma_n, t)
}

/// Compute Bachelier call delta: sensitivity of call price to forward rate.
///
/// Parameters
/// ----------
/// forward : float
///     Forward rate.
/// strike : float
///     Strike rate.
/// sigma_n : float
///     Normal volatility.
/// t : float
///     Time to expiry in years.
///
/// Returns
/// -------
/// float
///     Call delta (per unit annuity).
#[pyfunction(name = "bachelier_delta_call", signature = (forward, strike, sigma_n, t))]
pub fn py_bachelier_delta_call(forward: f64, strike: f64, sigma_n: f64, t: f64) -> f64 {
    bachelier_delta_call(forward, strike, sigma_n, t)
}

/// Compute Bachelier put delta: sensitivity of put price to forward rate.
///
/// Parameters
/// ----------
/// forward : float
///     Forward rate.
/// strike : float
///     Strike rate.
/// sigma_n : float
///     Normal volatility.
/// t : float
///     Time to expiry in years.
///
/// Returns
/// -------
/// float
///     Put delta (per unit annuity).
#[pyfunction(name = "bachelier_delta_put", signature = (forward, strike, sigma_n, t))]
pub fn py_bachelier_delta_put(forward: f64, strike: f64, sigma_n: f64, t: f64) -> f64 {
    bachelier_delta_put(forward, strike, sigma_n, t)
}

/// Compute Bachelier gamma: second derivative of option price w.r.t. forward.
///
/// Same for both calls and puts.
///
/// Parameters
/// ----------
/// forward : float
///     Forward rate.
/// strike : float
///     Strike rate.
/// sigma_n : float
///     Normal volatility.
/// t : float
///     Time to expiry in years.
///
/// Returns
/// -------
/// float
///     Gamma (per unit annuity).
#[pyfunction(name = "bachelier_gamma", signature = (forward, strike, sigma_n, t))]
pub fn py_bachelier_gamma(forward: f64, strike: f64, sigma_n: f64, t: f64) -> f64 {
    bachelier_gamma(forward, strike, sigma_n, t)
}

// =============================================================================
// Shifted Black Model — Individual Functions
// =============================================================================

/// Compute the price of a call option under the Shifted Black model.
///
/// Handles negative rates by shifting both forward and strike.
///
/// Parameters
/// ----------
/// forward : float
///     Forward rate (can be negative).
/// strike : float
///     Strike rate (can be negative).
/// sigma : float
///     Lognormal volatility.
/// t : float
///     Time to expiry in years.
/// shift : float
///     Shift amount (e.g. 0.03 = 3% shift).
///
/// Returns
/// -------
/// float
///     Call option price per unit annuity.
#[pyfunction(name = "black_shifted_call", signature = (forward, strike, sigma, t, shift))]
pub fn py_black_shifted_call(forward: f64, strike: f64, sigma: f64, t: f64, shift: f64) -> f64 {
    black_shifted_call(forward, strike, sigma, t, shift)
}

/// Compute the price of a put option under the Shifted Black model.
///
/// Parameters
/// ----------
/// forward : float
///     Forward rate (can be negative).
/// strike : float
///     Strike rate (can be negative).
/// sigma : float
///     Lognormal volatility.
/// t : float
///     Time to expiry in years.
/// shift : float
///     Shift amount (e.g. 0.03 = 3% shift).
///
/// Returns
/// -------
/// float
///     Put option price per unit annuity.
#[pyfunction(name = "black_shifted_put", signature = (forward, strike, sigma, t, shift))]
pub fn py_black_shifted_put(forward: f64, strike: f64, sigma: f64, t: f64, shift: f64) -> f64 {
    black_shifted_put(forward, strike, sigma, t, shift)
}

/// Compute Shifted Black vega with unit annuity.
///
/// Parameters
/// ----------
/// forward : float
///     Forward rate (can be negative).
/// strike : float
///     Strike rate (can be negative).
/// sigma : float
///     Lognormal volatility.
/// t : float
///     Time to expiry in years.
/// shift : float
///     Shift amount (e.g. 0.03 = 3% shift).
///
/// Returns
/// -------
/// float
///     Vega per unit change in vol (per unit annuity).
#[pyfunction(name = "black_shifted_vega", signature = (forward, strike, sigma, t, shift))]
pub fn py_black_shifted_vega(forward: f64, strike: f64, sigma: f64, t: f64, shift: f64) -> f64 {
    black_shifted_vega(forward, strike, sigma, t, shift)
}

// =============================================================================
// Implied Volatility Solvers
// =============================================================================

/// Extract Black-76 (lognormal) implied volatility from an option price.
///
/// Given a market option price, finds the unique lognormal volatility that
/// reproduces the price under the Black-76 model.
///
/// Parameters
/// ----------
/// price : float
///     Market option price per unit annuity (non-negative).
/// forward : float
///     Forward rate or price (must be positive and finite).
/// strike : float
///     Strike rate or price (must be positive and finite).
/// t : float
///     Time to expiry in years (must be positive and finite).
/// is_call : bool
///     True for a call option, False for a put option.
///
/// Returns
/// -------
/// float
///     The implied lognormal volatility.
///
/// Raises
/// ------
/// ValueError
///     If inputs are invalid or the solver fails to converge.
#[pyfunction(name = "implied_vol_black", signature = (price, forward, strike, t, is_call))]
pub fn py_implied_vol_black(
    price: f64,
    forward: f64,
    strike: f64,
    t: f64,
    is_call: bool,
) -> PyResult<f64> {
    implied_vol_black(price, forward, strike, t, is_call)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

/// Extract Bachelier (normal) implied volatility from an option price.
///
/// Given a market option price, finds the unique normal volatility that
/// reproduces the price under the Bachelier model.
///
/// Parameters
/// ----------
/// price : float
///     Market option price per unit annuity (non-negative).
/// forward : float
///     Forward rate (any finite value; negative rates supported).
/// strike : float
///     Strike rate (any finite value).
/// t : float
///     Time to expiry in years (must be positive and finite).
/// is_call : bool
///     True for a call option, False for a put option.
///
/// Returns
/// -------
/// float
///     The implied normal volatility.
///
/// Raises
/// ------
/// ValueError
///     If inputs are invalid or the solver fails to converge.
#[pyfunction(name = "implied_vol_bachelier", signature = (price, forward, strike, t, is_call))]
pub fn py_implied_vol_bachelier(
    price: f64,
    forward: f64,
    strike: f64,
    t: f64,
    is_call: bool,
) -> PyResult<f64> {
    implied_vol_bachelier(price, forward, strike, t, is_call)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

/// Brenner-Subrahmanyam ATM approximation for Black implied volatility.
#[pyfunction(
    name = "brenner_subrahmanyam_approx",
    text_signature = "(forward, strike, option_price, t)"
)]
pub fn py_brenner_subrahmanyam_approx(forward: f64, strike: f64, option_price: f64, t: f64) -> f64 {
    brenner_subrahmanyam_approx(forward, strike, option_price, t)
}

/// Manaster-Koehler approximation for Black implied volatility.
#[pyfunction(
    name = "manaster_koehler_approx",
    text_signature = "(forward, strike, t)"
)]
pub fn py_manaster_koehler_approx(forward: f64, strike: f64, t: f64) -> f64 {
    manaster_koehler_approx(forward, strike, t)
}

/// Combined initial guess for implied volatility solvers.
#[pyfunction(
    name = "implied_vol_initial_guess",
    text_signature = "(forward, strike, option_price, t)"
)]
pub fn py_implied_vol_initial_guess(forward: f64, strike: f64, option_price: f64, t: f64) -> f64 {
    implied_vol_initial_guess(forward, strike, option_price, t)
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

    // Legacy convenience wrappers (call-only)
    module.add_function(wrap_pyfunction!(py_bachelier_price, &module)?)?;
    module.add_function(wrap_pyfunction!(py_black_price, &module)?)?;
    module.add_function(wrap_pyfunction!(py_black_shifted_price, &module)?)?;

    // Black-76 pricing and Greeks
    module.add_function(wrap_pyfunction!(py_black_call, &module)?)?;
    module.add_function(wrap_pyfunction!(py_black_put, &module)?)?;
    module.add_function(wrap_pyfunction!(py_black_vega, &module)?)?;
    module.add_function(wrap_pyfunction!(py_black_delta_call, &module)?)?;
    module.add_function(wrap_pyfunction!(py_black_delta_put, &module)?)?;
    module.add_function(wrap_pyfunction!(py_black_gamma, &module)?)?;

    // Bachelier pricing and Greeks
    module.add_function(wrap_pyfunction!(py_bachelier_call, &module)?)?;
    module.add_function(wrap_pyfunction!(py_bachelier_put, &module)?)?;
    module.add_function(wrap_pyfunction!(py_bachelier_vega, &module)?)?;
    module.add_function(wrap_pyfunction!(py_bachelier_delta_call, &module)?)?;
    module.add_function(wrap_pyfunction!(py_bachelier_delta_put, &module)?)?;
    module.add_function(wrap_pyfunction!(py_bachelier_gamma, &module)?)?;

    // Shifted Black pricing and Greeks
    module.add_function(wrap_pyfunction!(py_black_shifted_call, &module)?)?;
    module.add_function(wrap_pyfunction!(py_black_shifted_put, &module)?)?;
    module.add_function(wrap_pyfunction!(py_black_shifted_vega, &module)?)?;

    // Implied volatility solvers
    module.add_function(wrap_pyfunction!(py_implied_vol_black, &module)?)?;
    module.add_function(wrap_pyfunction!(py_implied_vol_bachelier, &module)?)?;
    module.add_function(wrap_pyfunction!(py_brenner_subrahmanyam_approx, &module)?)?;
    module.add_function(wrap_pyfunction!(py_manaster_koehler_approx, &module)?)?;
    module.add_function(wrap_pyfunction!(py_implied_vol_initial_guess, &module)?)?;

    // Volatility convention conversion
    module.add_function(wrap_pyfunction!(py_convert_atm_volatility, &module)?)?;
    module.add_function(wrap_pyfunction!(py_convert_volatility, &module)?)?;

    let exports = [
        "VolatilityConvention",
        // Legacy convenience wrappers
        "bachelier_price",
        "black_price",
        "black_shifted_price",
        // Black-76
        "black_call",
        "black_put",
        "black_vega",
        "black_delta_call",
        "black_delta_put",
        "black_gamma",
        // Bachelier
        "bachelier_call",
        "bachelier_put",
        "bachelier_vega",
        "bachelier_delta_call",
        "bachelier_delta_put",
        "bachelier_gamma",
        // Shifted Black
        "black_shifted_call",
        "black_shifted_put",
        "black_shifted_vega",
        // Implied vol solvers
        "implied_vol_black",
        "implied_vol_bachelier",
        "brenner_subrahmanyam_approx",
        "manaster_koehler_approx",
        "implied_vol_initial_guess",
        // Conversion
        "convert_atm_volatility",
        "convert_volatility",
    ];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
