//! Python bindings for Monte Carlo Greek estimators.
//!
//! Exposes finite-difference deltas and gammas with both the conservative
//! independence-bound stderr and the tighter common-random-number (CRN)
//! paired stderr. The CRN variants compute true paired standard errors per
//! path and are 1–2 orders of magnitude tighter than the independence
//! bound for smooth payoffs — preferred for hedge-ratio sizing.
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

#[allow(clippy::too_many_arguments)]
fn build_engine(
    num_paths: usize,
    seed: u64,
    expiry: f64,
    num_steps: usize,
    use_parallel: bool,
) -> PyResult<McEngine> {
    let time_grid = TimeGrid::uniform(expiry, num_steps).map_err(core_to_py)?;
    Ok(McEngine::new(McEngineConfig {
        num_paths,
        seed,
        time_grid,
        target_ci_half_width: None,
        use_parallel,
        chunk_size: 1000,
        path_capture: finstack_monte_carlo::engine::PathCaptureConfig::default(),
        antithetic: false,
    }))
}

/// Finite-difference delta for a vanilla European option under GBM.
///
/// Reports the conservative independence-bound stderr by default (fast,
/// safe upper bound). For hedge-ratio sizing, prefer
/// [`finite_diff_delta_crn`] which returns the tighter paired CRN stderr.
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
    let engine = build_engine(num_paths, seed, expiry, num_steps, false)?;
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

/// Finite-difference delta with **paired CRN stderr** (preferred).
///
/// Reports the true paired standard error of `(V_up_i − V_down_i) / 2h`.
/// CRN makes this 1–2 orders of magnitude tighter than the independence
/// bound returned by [`fd_delta`] for smooth payoffs — use this for
/// risk-budget and hedge-ratio sizing.
///
/// Always runs serially (paired stderr requires deterministic per-path
/// pairing). Returns `(delta, stderr)`.
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
    let engine = build_engine(num_paths, seed, expiry, num_steps, false)?;
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
/// See [`fd_gamma_crn`] for a tighter paired CRN stderr suitable for
/// hedge-ratio sizing. Returns `(gamma, stderr)`.
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
    let engine = build_engine(num_paths, seed, expiry, num_steps, false)?;
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

/// Finite-difference gamma with **paired CRN stderr** (preferred).
///
/// Returns `(gamma, stderr)` where `stderr` is the per-path paired
/// standard error of `(V_up_i − 2 V_base_i + V_down_i) / h²`. Always
/// runs serially.
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
    let engine = build_engine(num_paths, seed, expiry, num_steps, false)?;
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
