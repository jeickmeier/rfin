//! Fourier pricing method bindings (COS and Lewis).
//!
//! Exposes model-agnostic Fourier pricers for European options under the
//! Black-Scholes, Variance Gamma, and Merton jump-diffusion models.  Each
//! binding wraps a characteristic function (from `finstack-core`) together
//! with either the Fang-Oosterlee (2008) COS method or the Lewis (2001)
//! single-integral method (both from `finstack-valuations`).

use crate::errors::display_to_py;
use finstack_core::math::characteristic_function::{
    BlackScholesCf, MertonJumpCf, VarianceGammaCf,
};
use finstack_valuations::pricer::fourier::{CosConfig, CosPricer, LewisConfig, LewisPricer};
use pyo3::prelude::*;

// ---------------------------------------------------------------------------
// Black-Scholes
// ---------------------------------------------------------------------------

/// Price a European option under the Black-Scholes model using the COS method.
///
/// Parameters
/// ----------
/// spot : float
///     Current spot price of the underlying.
/// strike : float
///     Option strike.
/// rate : float
///     Continuously-compounded risk-free rate (annualized).
/// dividend : float
///     Continuous dividend yield (annualized).
/// vol : float
///     Annualized volatility (sigma).
/// maturity : float
///     Time to maturity in years.
/// is_call : bool
///     ``True`` for a call, ``False`` for a put.
/// n_terms : int, optional
///     Number of cosine terms in the expansion (default 128).
///
/// Returns
/// -------
/// float
///     Present-value option price in the underlying's currency units.
#[pyfunction]
#[pyo3(signature = (spot, strike, rate, dividend, vol, maturity, is_call, n_terms=128))]
#[allow(clippy::too_many_arguments)]
fn bs_cos_price(
    spot: f64,
    strike: f64,
    rate: f64,
    dividend: f64,
    vol: f64,
    maturity: f64,
    is_call: bool,
    n_terms: usize,
) -> PyResult<f64> {
    let cf = BlackScholesCf {
        r: rate,
        q: dividend,
        sigma: vol,
    };
    let config = CosConfig {
        num_terms: n_terms,
        ..CosConfig::default()
    };
    let pricer = CosPricer::new(&cf, config);
    if is_call {
        pricer.price_call(spot, strike, rate, maturity).map_err(display_to_py)
    } else {
        pricer.price_put(spot, strike, rate, maturity).map_err(display_to_py)
    }
}

/// Price a European option under the Black-Scholes model using the Lewis (2001) method.
///
/// Parameters
/// ----------
/// spot : float
///     Current spot price of the underlying.
/// strike : float
///     Option strike.
/// rate : float
///     Continuously-compounded risk-free rate (annualized).
/// dividend : float
///     Continuous dividend yield (annualized).
/// vol : float
///     Annualized volatility (sigma).
/// maturity : float
///     Time to maturity in years.
/// is_call : bool
///     ``True`` for a call, ``False`` for a put.
///
/// Returns
/// -------
/// float
///     Present-value option price.
#[pyfunction]
#[pyo3(signature = (spot, strike, rate, dividend, vol, maturity, is_call))]
fn bs_lewis_price(
    spot: f64,
    strike: f64,
    rate: f64,
    dividend: f64,
    vol: f64,
    maturity: f64,
    is_call: bool,
) -> PyResult<f64> {
    let cf = BlackScholesCf {
        r: rate,
        q: dividend,
        sigma: vol,
    };
    let pricer = LewisPricer::new(&cf, LewisConfig::default());
    if is_call {
        pricer
            .price_call(spot, strike, rate, dividend, maturity)
            .map_err(display_to_py)
    } else {
        pricer
            .price_put(spot, strike, rate, dividend, maturity)
            .map_err(display_to_py)
    }
}

// ---------------------------------------------------------------------------
// Variance Gamma
// ---------------------------------------------------------------------------

/// Price a European option under the Variance Gamma model using the COS method.
///
/// Parameters
/// ----------
/// spot : float
///     Current spot price.
/// strike : float
///     Option strike.
/// rate : float
///     Continuously-compounded risk-free rate.
/// dividend : float
///     Continuous dividend yield.
/// sigma : float
///     Volatility of the subordinated Brownian motion.
/// theta : float
///     Drift of the subordinated Brownian motion (negative values skew left).
/// nu : float
///     Variance rate of the Gamma subordinator (nu > 0).
/// maturity : float
///     Time to maturity in years.
/// is_call : bool
///     ``True`` for a call, ``False`` for a put.
/// n_terms : int, optional
///     Number of cosine terms (default 128; heavier tails may need 256+).
///
/// Returns
/// -------
/// float
///     Present-value option price.
#[pyfunction]
#[pyo3(signature = (spot, strike, rate, dividend, sigma, theta, nu, maturity, is_call, n_terms=128))]
#[allow(clippy::too_many_arguments)]
fn vg_cos_price(
    spot: f64,
    strike: f64,
    rate: f64,
    dividend: f64,
    sigma: f64,
    theta: f64,
    nu: f64,
    maturity: f64,
    is_call: bool,
    n_terms: usize,
) -> PyResult<f64> {
    let cf = VarianceGammaCf {
        r: rate,
        q: dividend,
        sigma,
        nu,
        theta,
    };
    let config = CosConfig {
        num_terms: n_terms,
        ..CosConfig::default()
    };
    let pricer = CosPricer::new(&cf, config);
    if is_call {
        pricer.price_call(spot, strike, rate, maturity).map_err(display_to_py)
    } else {
        pricer.price_put(spot, strike, rate, maturity).map_err(display_to_py)
    }
}

// ---------------------------------------------------------------------------
// Merton jump-diffusion
// ---------------------------------------------------------------------------

/// Price a European option under Merton (1976) jump-diffusion using the COS method.
///
/// Parameters
/// ----------
/// spot : float
///     Current spot price.
/// strike : float
///     Option strike.
/// rate : float
///     Continuously-compounded risk-free rate.
/// dividend : float
///     Continuous dividend yield.
/// sigma : float
///     Diffusion volatility.
/// mu_jump : float
///     Mean of log-jump size (negative for left-skewed crash risk).
/// sigma_jump : float
///     Standard deviation of log-jump size.
/// lambda : float
///     Jump intensity (expected number of jumps per year).
/// maturity : float
///     Time to maturity in years.
/// is_call : bool
///     ``True`` for a call, ``False`` for a put.
/// n_terms : int, optional
///     Number of cosine terms (default 128).
///
/// Returns
/// -------
/// float
///     Present-value option price.
#[pyfunction]
#[pyo3(signature = (spot, strike, rate, dividend, sigma, mu_jump, sigma_jump, lambda, maturity, is_call, n_terms=128))]
#[allow(clippy::too_many_arguments)]
fn merton_jump_cos_price(
    spot: f64,
    strike: f64,
    rate: f64,
    dividend: f64,
    sigma: f64,
    mu_jump: f64,
    sigma_jump: f64,
    lambda: f64,
    maturity: f64,
    is_call: bool,
    n_terms: usize,
) -> PyResult<f64> {
    let cf = MertonJumpCf {
        r: rate,
        q: dividend,
        sigma,
        lambda,
        mu_j: mu_jump,
        sigma_j: sigma_jump,
    };
    let config = CosConfig {
        num_terms: n_terms,
        ..CosConfig::default()
    };
    let pricer = CosPricer::new(&cf, config);
    if is_call {
        pricer.price_call(spot, strike, rate, maturity).map_err(display_to_py)
    } else {
        pricer.price_put(spot, strike, rate, maturity).map_err(display_to_py)
    }
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

/// Register Fourier pricing functions on the valuations submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(bs_cos_price, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(bs_lewis_price, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(vg_cos_price, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(merton_jump_cos_price, m)?)?;
    Ok(())
}
