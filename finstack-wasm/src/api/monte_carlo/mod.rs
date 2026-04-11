//! WASM bindings for the `finstack-monte-carlo` crate.
//!
//! Provides convenience functions for pricing European options under GBM
//! dynamics via Monte Carlo simulation. Results are returned as serialized
//! JSON objects.

use std::str::FromStr;

use crate::utils::to_js_err;
use finstack_core::currency::Currency;
use finstack_monte_carlo::payoff::vanilla::{EuropeanCall, EuropeanPut};
use finstack_monte_carlo::pricer::european::{EuropeanPricer, EuropeanPricerConfig};
use finstack_monte_carlo::process::gbm::GbmProcess;
use finstack_monte_carlo::results::MoneyEstimate;
use wasm_bindgen::prelude::*;

/// Serializable result shape returned to JavaScript.
#[derive(serde::Serialize)]
struct McResultJs {
    /// Discounted mean present value.
    mean: f64,
    /// Currency code of the estimate.
    currency: String,
    /// Standard error of the mean.
    stderr: f64,
    /// Sample standard deviation (if available).
    std_dev: Option<f64>,
    /// Lower 95% confidence bound.
    ci_lower: f64,
    /// Upper 95% confidence bound.
    ci_upper: f64,
    /// Number of paths simulated.
    num_paths: usize,
}

impl McResultJs {
    /// Convert a [`MoneyEstimate`] into the JS-friendly shape.
    fn from_estimate(est: &MoneyEstimate) -> Self {
        Self {
            mean: est.mean.amount(),
            currency: est.mean.currency().to_string(),
            stderr: est.stderr,
            std_dev: est.std_dev,
            ci_lower: est.ci_95.0.amount(),
            ci_upper: est.ci_95.1.amount(),
            num_paths: est.num_paths,
        }
    }
}

/// Price a European call option via Monte Carlo under GBM dynamics.
///
/// Returns a JSON object with `mean`, `currency`, `stderr`, `std_dev`,
/// `ci_lower`, `ci_upper`, and `num_paths`.
#[allow(clippy::too_many_arguments)]
#[wasm_bindgen(js_name = priceEuropeanCall)]
pub fn price_european_call(
    spot: f64,
    strike: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    expiry: f64,
    num_paths: usize,
    seed: u64,
    num_steps: Option<usize>,
    currency: Option<String>,
) -> Result<JsValue, JsValue> {
    let ccy = resolve_currency(currency.as_deref())?;
    let steps = num_steps.unwrap_or(252);
    let payoff = EuropeanCall::new(strike, 1.0, steps);
    let df = (-rate * expiry).exp();
    let est = run_pricer(
        spot, rate, div_yield, vol, expiry, num_paths, seed, steps, ccy, &payoff, df,
    )?;
    let result = McResultJs::from_estimate(&est);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

/// Price a European put option via Monte Carlo under GBM dynamics.
///
/// Returns a JSON object with `mean`, `currency`, `stderr`, `std_dev`,
/// `ci_lower`, `ci_upper`, and `num_paths`.
#[allow(clippy::too_many_arguments)]
#[wasm_bindgen(js_name = priceEuropeanPut)]
pub fn price_european_put(
    spot: f64,
    strike: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    expiry: f64,
    num_paths: usize,
    seed: u64,
    num_steps: Option<usize>,
    currency: Option<String>,
) -> Result<JsValue, JsValue> {
    let ccy = resolve_currency(currency.as_deref())?;
    let steps = num_steps.unwrap_or(252);
    let payoff = EuropeanPut::new(strike, 1.0, steps);
    let df = (-rate * expiry).exp();
    let est = run_pricer(
        spot, rate, div_yield, vol, expiry, num_paths, seed, steps, ccy, &payoff, df,
    )?;
    let result = McResultJs::from_estimate(&est);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

// ---------------------------------------------------------------------------
// Analytical
// ---------------------------------------------------------------------------

/// Black-Scholes call price.
#[wasm_bindgen(js_name = blackScholesCall)]
pub fn black_scholes_call(
    spot: f64,
    strike: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    expiry: f64,
) -> f64 {
    finstack_monte_carlo::variance_reduction::control_variate::black_scholes_call(
        spot, strike, rate, div_yield, vol, expiry,
    )
}

/// Black-Scholes put price.
#[wasm_bindgen(js_name = blackScholesPut)]
pub fn black_scholes_put(
    spot: f64,
    strike: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    expiry: f64,
) -> f64 {
    finstack_monte_carlo::variance_reduction::control_variate::black_scholes_put(
        spot, strike, rate, div_yield, vol, expiry,
    )
}

// ---------------------------------------------------------------------------
// Path-dependent pricing
// ---------------------------------------------------------------------------

/// Price an Asian call via Monte Carlo under GBM dynamics.
#[allow(clippy::too_many_arguments)]
#[wasm_bindgen(js_name = priceAsianCall)]
pub fn price_asian_call(
    spot: f64,
    strike: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    expiry: f64,
    num_paths: usize,
    seed: u64,
    num_steps: Option<usize>,
    currency: Option<String>,
) -> Result<JsValue, JsValue> {
    use finstack_monte_carlo::payoff::asian::{AsianCall, AveragingMethod};
    use finstack_monte_carlo::pricer::path_dependent::{
        PathDependentPricer, PathDependentPricerConfig,
    };

    let ccy = resolve_currency(currency.as_deref())?;
    let steps = num_steps.unwrap_or(252);
    let fixing_steps: Vec<usize> = (1..=steps).collect();
    let payoff = AsianCall::new(strike, 1.0, AveragingMethod::Arithmetic, fixing_steps);
    let df = (-rate * expiry).exp();
    let config = PathDependentPricerConfig::new(num_paths).with_seed(seed);
    let pricer = PathDependentPricer::new(config);
    let process = GbmProcess::with_params(rate, div_yield, vol);
    let est = pricer
        .price(&process, spot, expiry, steps, &payoff, ccy, df)
        .map_err(to_js_err)?;
    let result = McResultJs::from_estimate(&est);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

/// Price an Asian put via Monte Carlo under GBM dynamics.
#[allow(clippy::too_many_arguments)]
#[wasm_bindgen(js_name = priceAsianPut)]
pub fn price_asian_put(
    spot: f64,
    strike: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    expiry: f64,
    num_paths: usize,
    seed: u64,
    num_steps: Option<usize>,
    currency: Option<String>,
) -> Result<JsValue, JsValue> {
    use finstack_monte_carlo::payoff::asian::{AsianPut, AveragingMethod};
    use finstack_monte_carlo::pricer::path_dependent::{
        PathDependentPricer, PathDependentPricerConfig,
    };

    let ccy = resolve_currency(currency.as_deref())?;
    let steps = num_steps.unwrap_or(252);
    let fixing_steps: Vec<usize> = (1..=steps).collect();
    let payoff = AsianPut::new(strike, 1.0, AveragingMethod::Arithmetic, fixing_steps);
    let df = (-rate * expiry).exp();
    let config = PathDependentPricerConfig::new(num_paths).with_seed(seed);
    let pricer = PathDependentPricer::new(config);
    let process = GbmProcess::with_params(rate, div_yield, vol);
    let est = pricer
        .price(&process, spot, expiry, steps, &payoff, ccy, df)
        .map_err(to_js_err)?;
    let result = McResultJs::from_estimate(&est);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

/// Price an American put via LSMC under GBM dynamics.
#[allow(clippy::too_many_arguments)]
#[wasm_bindgen(js_name = priceAmericanPut)]
pub fn price_american_put(
    spot: f64,
    strike: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    expiry: f64,
    num_paths: usize,
    seed: u64,
    num_steps: Option<usize>,
    currency: Option<String>,
) -> Result<JsValue, JsValue> {
    use finstack_monte_carlo::pricer::basis::LaguerreBasis;
    use finstack_monte_carlo::pricer::lsmc::{AmericanPut, LsmcConfig, LsmcPricer};

    let ccy = resolve_currency(currency.as_deref())?;
    let steps = num_steps.unwrap_or(50);
    let exercise_dates: Vec<usize> = (1..=steps).collect();
    let exercise = AmericanPut::new(strike).map_err(to_js_err)?;
    let config = LsmcConfig::new(num_paths, exercise_dates).with_seed(seed);
    let pricer = LsmcPricer::new(config);
    let process = GbmProcess::with_params(rate, div_yield, vol);
    let df = (-rate * expiry).exp();
    let est = pricer
        .price(
            &process,
            spot,
            expiry,
            steps,
            &exercise,
            &LaguerreBasis::new(3, strike),
            ccy,
            df,
        )
        .map_err(to_js_err)?;
    let result = McResultJs::from_estimate(&est);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Resolve an optional currency string, defaulting to USD.
fn resolve_currency(code: Option<&str>) -> Result<Currency, JsValue> {
    let s = code.unwrap_or("USD");
    Currency::from_str(s).map_err(to_js_err)
}

/// Shared European pricer runner.
#[allow(clippy::too_many_arguments)]
fn run_pricer(
    spot: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    expiry: f64,
    num_paths: usize,
    seed: u64,
    num_steps: usize,
    currency: Currency,
    payoff: &impl finstack_monte_carlo::traits::Payoff,
    discount_factor: f64,
) -> Result<MoneyEstimate, JsValue> {
    let config = EuropeanPricerConfig::new(num_paths)
        .with_seed(seed)
        .with_parallel(false);
    let pricer = EuropeanPricer::new(config);
    let process = GbmProcess::with_params(rate, div_yield, vol);

    pricer
        .price(
            &process,
            spot,
            expiry,
            num_steps,
            payoff,
            currency,
            discount_factor,
        )
        .map_err(to_js_err)
}
