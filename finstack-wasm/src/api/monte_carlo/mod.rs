//! WASM bindings for the `finstack-monte-carlo` crate.
//!
//! Provides convenience functions for pricing European options under GBM
//! dynamics via Monte Carlo simulation. Results are returned as serialized
//! JSON objects.

use std::str::FromStr;

use crate::utils::to_js_err;
use finstack_core::currency::Currency;
use finstack_monte_carlo::pricer::european::{EuropeanPricer, EuropeanPricerConfig};
use finstack_monte_carlo::process::gbm::GbmProcess;
use finstack_monte_carlo::results::MoneyEstimate;
use wasm_bindgen::prelude::*;

/// Serializable result shape returned to JavaScript.
///
/// Field layout mirrors the accessors on the Python `MonteCarloResult`
/// binding so both hosts see the same vocabulary.
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
    /// Number of paths skipped due to non-finite payoffs.
    num_skipped: usize,
    /// Median of captured discounted path values (if captured).
    median: Option<f64>,
    /// 25th percentile of captured discounted path values (if captured).
    percentile_25: Option<f64>,
    /// 75th percentile of captured discounted path values (if captured).
    percentile_75: Option<f64>,
    /// Minimum of captured discounted path values (if captured).
    min: Option<f64>,
    /// Maximum of captured discounted path values (if captured).
    max: Option<f64>,
    /// Relative standard error (`stderr / |mean|`); `f64::INFINITY` near zero.
    relative_stderr: f64,
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
            num_skipped: est.num_skipped,
            median: est.median,
            percentile_25: est.percentile_25,
            percentile_75: est.percentile_75,
            min: est.min,
            max: est.max,
            relative_stderr: est.relative_stderr(),
        }
    }
}

/// Price a European call option via Monte Carlo under GBM dynamics.
///
/// Returns a JSON object with `mean`, `currency`, `stderr`, `std_dev`,
/// `ci_lower`, `ci_upper`, `num_paths`, `num_skipped`, `median`,
/// `percentile_25`, `percentile_75`, `min`, `max`, and `relative_stderr`.
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
    let est = build_pricer(num_paths, seed)
        .price_gbm_call(spot, strike, rate, div_yield, vol, expiry, steps, ccy)
        .map_err(to_js_err)?;
    let result = McResultJs::from_estimate(&est);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

/// Price a European put option via Monte Carlo under GBM dynamics.
///
/// Returns a JSON object with `mean`, `currency`, `stderr`, `std_dev`,
/// `ci_lower`, `ci_upper`, `num_paths`, `num_skipped`, `median`,
/// `percentile_25`, `percentile_75`, `min`, `max`, and `relative_stderr`.
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
    let est = build_pricer(num_paths, seed)
        .price_gbm_put(spot, strike, rate, div_yield, vol, expiry, steps, ccy)
        .map_err(to_js_err)?;
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
        spot, strike, expiry, rate, div_yield, vol,
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
        spot, strike, expiry, rate, div_yield, vol,
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
    let process = GbmProcess::with_params(rate, div_yield, vol).map_err(to_js_err)?;
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
    let process = GbmProcess::with_params(rate, div_yield, vol).map_err(to_js_err)?;
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
    let config = LsmcConfig::new(num_paths, exercise_dates, steps)
        .map_err(to_js_err)?
        .with_seed(seed);
    let pricer = LsmcPricer::new(config);
    let process = GbmProcess::with_params(rate, div_yield, vol).map_err(to_js_err)?;
    let est = pricer
        .price(
            &process,
            spot,
            expiry,
            steps,
            &exercise,
            &LaguerreBasis::new(3, strike),
            ccy,
            rate,
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

/// Shared European pricer builder.
fn build_pricer(num_paths: usize, seed: u64) -> EuropeanPricer {
    EuropeanPricer::new(
        EuropeanPricerConfig::new(num_paths)
            .with_seed(seed)
            .with_parallel(false),
    )
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::money::Money;

    #[test]
    fn black_scholes_call_atm_reasonable() {
        let price = black_scholes_call(100.0, 100.0, 0.05, 0.0, 0.2, 1.0);
        assert!(price > 5.0 && price < 15.0, "ATM call price={price}");
    }

    #[test]
    fn black_scholes_put_atm_positive() {
        let price = black_scholes_put(100.0, 100.0, 0.05, 0.0, 0.2, 1.0);
        assert!(price > 0.0);
    }

    #[test]
    fn resolve_currency_defaults_and_parses_eur() {
        let Ok(usd) = resolve_currency(None) else {
            panic!("resolve_currency(None) should default to USD");
        };
        assert_eq!(usd, Currency::USD);

        let Ok(eur) = resolve_currency(Some("EUR")) else {
            panic!("resolve_currency EUR should succeed");
        };
        assert_eq!(eur, Currency::EUR);
    }

    #[test]
    fn mc_result_js_from_estimate_maps_fields() {
        let est = MoneyEstimate {
            mean: Money::new(10.0, Currency::USD),
            stderr: 0.25,
            ci_95: (
                Money::new(9.0, Currency::USD),
                Money::new(11.0, Currency::USD),
            ),
            num_paths: 1000,
            std_dev: Some(5.0),
            median: None,
            percentile_25: None,
            percentile_75: None,
            min: None,
            max: None,
            num_skipped: 0,
        };
        let js = McResultJs::from_estimate(&est);
        assert!((js.mean - 10.0).abs() < 1e-12);
        assert_eq!(js.currency, "USD");
        assert!((js.stderr - 0.25).abs() < 1e-12);
        assert_eq!(js.std_dev, Some(5.0));
        assert!((js.ci_lower - 9.0).abs() < 1e-12);
        assert!((js.ci_upper - 11.0).abs() < 1e-12);
        assert_eq!(js.num_paths, 1000);
    }

    #[test]
    fn build_pricer_european_call_positive_mean() {
        let Ok(est) = build_pricer(1000, 42).price_gbm_call(
            100.0,
            100.0,
            0.05,
            0.0,
            0.2,
            1.0,
            252,
            Currency::USD,
        ) else {
            panic!("price_gbm_call should succeed");
        };
        assert!(est.mean.amount() > 0.0);
    }
}
