//! Python bindings for Monte Carlo Greek estimators.
//!
//! Exposes finite-difference deltas and gammas with independent-path and
//! common-random-number standard errors.
//!
//! All functions release the GIL during the underlying Monte Carlo runs.

use super::engine::resolve_currency;
use crate::errors::core_to_py;
use finstack_monte_carlo::discretization::exact::ExactGbm;
use finstack_monte_carlo::engine::{McEngine, McEngineConfig};
use finstack_monte_carlo::greeks::finite_diff::{
    finite_diff_delta, finite_diff_delta_crn, finite_diff_gamma, finite_diff_gamma_crn,
};
use finstack_monte_carlo::payoff::vanilla::{EuropeanCall, EuropeanPut};
use finstack_monte_carlo::process::gbm::GbmProcess;
use finstack_monte_carlo::rng::philox::PhiloxRng;
use finstack_monte_carlo::time_grid::TimeGrid;
use pyo3::prelude::*;

#[derive(Debug, Clone, Copy)]
enum OptionType {
    Call,
    Put,
}

fn parse_option(name: &str) -> PyResult<OptionType> {
    match name.to_ascii_lowercase().as_str() {
        "call" | "c" => Ok(OptionType::Call),
        "put" | "p" => Ok(OptionType::Put),
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "unknown option_type '{other}'; expected 'call' or 'put'"
        ))),
    }
}

fn build_engine(num_paths: usize, seed: u64, expiry: f64, num_steps: usize) -> PyResult<McEngine> {
    let time_grid = TimeGrid::uniform(expiry, num_steps).map_err(core_to_py)?;
    Ok(McEngine::new(McEngineConfig {
        num_paths,
        seed,
        time_grid,
        target_ci_half_width: None,
        use_parallel: false,
        chunk_size: 1000,
        path_capture: finstack_monte_carlo::engine::PathCaptureConfig::default(),
        antithetic: false,
    }))
}

/// Finite-difference delta for a vanilla European option under GBM.
///
/// Reports the conservative independence-bound stderr. Use [`fd_delta_crn`]
/// for paired common-random-number stderr.
///
/// Returns `(delta, stderr)`.
#[pyfunction]
#[allow(clippy::too_many_arguments)]
#[pyo3(signature = (
    spot, strike, rate, div_yield, vol, expiry,
    num_paths=10_000, seed=42, num_steps=50,
    bump_size=0.01, option_type="call", currency=None,
))]
fn fd_delta(
    py: Python<'_>,
    spot: f64,
    strike: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    expiry: f64,
    num_paths: usize,
    seed: u64,
    num_steps: usize,
    bump_size: f64,
    option_type: &str,
    currency: Option<&Bound<'_, PyAny>>,
) -> PyResult<(f64, f64)> {
    let ccy = resolve_currency(currency)?;
    let kind = parse_option(option_type)?;
    let engine = build_engine(num_paths, seed, expiry, num_steps)?;
    let rng = PhiloxRng::new(seed);
    let gbm = GbmProcess::with_params(rate, div_yield, vol).map_err(core_to_py)?;
    let disc = ExactGbm::new();
    let df = (-rate * expiry).exp();
    py.detach(|| match kind {
        OptionType::Call => {
            let payoff = EuropeanCall::new(strike, expiry, num_steps);
            finite_diff_delta(
                &engine, &rng, &gbm, &disc, spot, &payoff, ccy, df, bump_size,
            )
        }
        OptionType::Put => {
            let payoff = EuropeanPut::new(strike, expiry, num_steps);
            finite_diff_delta(
                &engine, &rng, &gbm, &disc, spot, &payoff, ccy, df, bump_size,
            )
        }
    })
    .map_err(core_to_py)
}

/// Finite-difference delta with paired common-random-number stderr.
///
/// Returns `(delta, stderr)`.
#[pyfunction]
#[allow(clippy::too_many_arguments)]
#[pyo3(signature = (
    spot, strike, rate, div_yield, vol, expiry,
    num_paths=10_000, seed=42, num_steps=50,
    bump_size=0.01, option_type="call", currency=None,
))]
fn fd_delta_crn(
    py: Python<'_>,
    spot: f64,
    strike: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    expiry: f64,
    num_paths: usize,
    seed: u64,
    num_steps: usize,
    bump_size: f64,
    option_type: &str,
    currency: Option<&Bound<'_, PyAny>>,
) -> PyResult<(f64, f64)> {
    let ccy = resolve_currency(currency)?;
    let kind = parse_option(option_type)?;
    let engine = build_engine(num_paths, seed, expiry, num_steps)?;
    let rng = PhiloxRng::new(seed);
    let gbm = GbmProcess::with_params(rate, div_yield, vol).map_err(core_to_py)?;
    let disc = ExactGbm::new();
    let df = (-rate * expiry).exp();
    py.detach(|| match kind {
        OptionType::Call => {
            let payoff = EuropeanCall::new(strike, expiry, num_steps);
            finite_diff_delta_crn(
                &engine, &rng, &gbm, &disc, spot, &payoff, ccy, df, bump_size,
            )
        }
        OptionType::Put => {
            let payoff = EuropeanPut::new(strike, expiry, num_steps);
            finite_diff_delta_crn(
                &engine, &rng, &gbm, &disc, spot, &payoff, ccy, df, bump_size,
            )
        }
    })
    .map_err(core_to_py)
}

/// Finite-difference gamma (independence-bound stderr).
///
/// Returns `(gamma, stderr)`.
#[pyfunction]
#[allow(clippy::too_many_arguments)]
#[pyo3(signature = (
    spot, strike, rate, div_yield, vol, expiry,
    num_paths=10_000, seed=42, num_steps=50,
    bump_size=0.01, option_type="call", currency=None,
))]
fn fd_gamma(
    py: Python<'_>,
    spot: f64,
    strike: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    expiry: f64,
    num_paths: usize,
    seed: u64,
    num_steps: usize,
    bump_size: f64,
    option_type: &str,
    currency: Option<&Bound<'_, PyAny>>,
) -> PyResult<(f64, f64)> {
    let ccy = resolve_currency(currency)?;
    let kind = parse_option(option_type)?;
    let engine = build_engine(num_paths, seed, expiry, num_steps)?;
    let rng = PhiloxRng::new(seed);
    let gbm = GbmProcess::with_params(rate, div_yield, vol).map_err(core_to_py)?;
    let disc = ExactGbm::new();
    let df = (-rate * expiry).exp();
    py.detach(|| match kind {
        OptionType::Call => {
            let payoff = EuropeanCall::new(strike, expiry, num_steps);
            finite_diff_gamma(
                &engine, &rng, &gbm, &disc, spot, &payoff, ccy, df, bump_size,
            )
        }
        OptionType::Put => {
            let payoff = EuropeanPut::new(strike, expiry, num_steps);
            finite_diff_gamma(
                &engine, &rng, &gbm, &disc, spot, &payoff, ccy, df, bump_size,
            )
        }
    })
    .map_err(core_to_py)
}

/// Finite-difference gamma with paired common-random-number stderr.
///
/// Returns `(gamma, stderr)`.
#[pyfunction]
#[allow(clippy::too_many_arguments)]
#[pyo3(signature = (
    spot, strike, rate, div_yield, vol, expiry,
    num_paths=10_000, seed=42, num_steps=50,
    bump_size=0.01, option_type="call", currency=None,
))]
fn fd_gamma_crn(
    py: Python<'_>,
    spot: f64,
    strike: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    expiry: f64,
    num_paths: usize,
    seed: u64,
    num_steps: usize,
    bump_size: f64,
    option_type: &str,
    currency: Option<&Bound<'_, PyAny>>,
) -> PyResult<(f64, f64)> {
    let ccy = resolve_currency(currency)?;
    let kind = parse_option(option_type)?;
    let engine = build_engine(num_paths, seed, expiry, num_steps)?;
    let rng = PhiloxRng::new(seed);
    let gbm = GbmProcess::with_params(rate, div_yield, vol).map_err(core_to_py)?;
    let disc = ExactGbm::new();
    let df = (-rate * expiry).exp();
    py.detach(|| match kind {
        OptionType::Call => {
            let payoff = EuropeanCall::new(strike, expiry, num_steps);
            finite_diff_gamma_crn(
                &engine, &rng, &gbm, &disc, spot, &payoff, ccy, df, bump_size,
            )
        }
        OptionType::Put => {
            let payoff = EuropeanPut::new(strike, expiry, num_steps);
            finite_diff_gamma_crn(
                &engine, &rng, &gbm, &disc, spot, &payoff, ccy, df, bump_size,
            )
        }
    })
    .map_err(core_to_py)
}

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(fd_delta, m)?)?;
    m.add_function(wrap_pyfunction!(fd_delta_crn, m)?)?;
    m.add_function(wrap_pyfunction!(fd_gamma, m)?)?;
    m.add_function(wrap_pyfunction!(fd_gamma_crn, m)?)?;
    Ok(())
}
