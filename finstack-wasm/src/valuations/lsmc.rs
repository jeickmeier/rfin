//! WASM bindings for LSMC (Longstaff-Schwartz Monte Carlo) pricer.
//!
//! Provides American/Bermudan option pricing via least-squares regression
//! on simulated GBM paths.

use crate::core::error::{core_to_js, js_error};
use finstack_core::currency::Currency;
use finstack_monte_carlo::prelude::{
    AmericanCall, AmericanPut, LaguerreBasis, LsmcConfig, LsmcPricer, PolynomialBasis,
};
use finstack_monte_carlo::process::gbm::GbmProcess;
use finstack_monte_carlo::results::MoneyEstimate;
use std::str::FromStr;
use wasm_bindgen::prelude::*;

// =============================================================================
// Exercise Payoffs
// =============================================================================

/// American put option exercise payoff.
#[wasm_bindgen(js_name = LsmcAmericanPut)]
#[derive(Clone)]
pub struct JsAmericanPut {
    inner: AmericanPut,
}

#[wasm_bindgen(js_class = LsmcAmericanPut)]
impl JsAmericanPut {
    /// Create an American put payoff with the given strike.
    #[wasm_bindgen(constructor)]
    pub fn new(strike: f64) -> Result<JsAmericanPut, JsValue> {
        AmericanPut::new(strike)
            .map(|inner| JsAmericanPut { inner })
            .map_err(js_error)
    }

    /// Strike price.
    #[wasm_bindgen(getter)]
    pub fn strike(&self) -> f64 {
        self.inner.strike
    }
}

/// American call option exercise payoff.
#[wasm_bindgen(js_name = LsmcAmericanCall)]
#[derive(Clone)]
pub struct JsAmericanCall {
    inner: AmericanCall,
}

#[wasm_bindgen(js_class = LsmcAmericanCall)]
impl JsAmericanCall {
    /// Create an American call payoff with the given strike.
    #[wasm_bindgen(constructor)]
    pub fn new(strike: f64) -> Result<JsAmericanCall, JsValue> {
        AmericanCall::new(strike)
            .map(|inner| JsAmericanCall { inner })
            .map_err(js_error)
    }

    /// Strike price.
    #[wasm_bindgen(getter)]
    pub fn strike(&self) -> f64 {
        self.inner.strike
    }
}

// =============================================================================
// Basis Functions
// =============================================================================

/// Polynomial basis functions for LSMC regression.
#[wasm_bindgen(js_name = LsmcPolynomialBasis)]
#[derive(Clone)]
pub struct JsPolynomialBasis {
    degree: usize,
}

#[wasm_bindgen(js_class = LsmcPolynomialBasis)]
impl JsPolynomialBasis {
    /// Create polynomial basis of given degree ({1, x, x², ..., x^degree}).
    #[wasm_bindgen(constructor)]
    pub fn new(degree: usize) -> Result<JsPolynomialBasis, JsValue> {
        PolynomialBasis::try_new(degree)
            .map(|_| JsPolynomialBasis { degree })
            .map_err(js_error)
    }

    /// Polynomial degree.
    #[wasm_bindgen(getter)]
    pub fn degree(&self) -> usize {
        self.degree
    }
}

/// Laguerre polynomial basis functions for LSMC regression.
#[wasm_bindgen(js_name = LsmcLaguerreBasis)]
#[derive(Clone)]
pub struct JsLaguerreBasis {
    degree: usize,
    strike: f64,
}

#[wasm_bindgen(js_class = LsmcLaguerreBasis)]
impl JsLaguerreBasis {
    /// Create Laguerre basis of given degree, normalized by strike.
    #[wasm_bindgen(constructor)]
    pub fn new(degree: usize, strike: f64) -> Result<JsLaguerreBasis, JsValue> {
        LaguerreBasis::try_new(degree, strike)
            .map(|_| JsLaguerreBasis { degree, strike })
            .map_err(js_error)
    }

    /// Polynomial degree.
    #[wasm_bindgen(getter)]
    pub fn degree(&self) -> usize {
        self.degree
    }

    /// Normalization strike.
    #[wasm_bindgen(getter)]
    pub fn strike(&self) -> f64 {
        self.strike
    }
}

// =============================================================================
// LsmcConfig
// =============================================================================

/// Configuration for LSMC (Longstaff-Schwartz Monte Carlo) pricer.
#[wasm_bindgen(js_name = LsmcConfig)]
#[derive(Clone)]
pub struct JsLsmcConfig {
    inner: LsmcConfig,
}

#[wasm_bindgen(js_class = LsmcConfig)]
impl JsLsmcConfig {
    /// Create LSMC config with specified paths, exercise dates, and seed.
    #[wasm_bindgen(constructor)]
    pub fn new(
        num_paths: usize,
        exercise_dates: Vec<usize>,
        seed: Option<u64>,
    ) -> Result<JsLsmcConfig, JsValue> {
        LsmcConfig::try_new(num_paths, exercise_dates)
            .map(|config| JsLsmcConfig {
                inner: config.with_seed(seed.unwrap_or(42)),
            })
            .map_err(js_error)
    }

    /// Number of Monte Carlo paths.
    #[wasm_bindgen(getter, js_name = numPaths)]
    pub fn num_paths(&self) -> usize {
        self.inner.num_paths
    }

    /// Random seed.
    #[wasm_bindgen(getter)]
    pub fn seed(&self) -> u64 {
        self.inner.seed
    }
}

// =============================================================================
// LsmcResult
// =============================================================================

/// LSMC result containing price estimate and statistics.
#[wasm_bindgen(js_name = LsmcResult)]
pub struct JsLsmcResult {
    inner: MoneyEstimate,
}

#[wasm_bindgen(js_class = LsmcResult)]
impl JsLsmcResult {
    /// Point estimate of the option price (as amount in the specified currency).
    #[wasm_bindgen(getter)]
    pub fn mean(&self) -> f64 {
        self.inner.mean.amount()
    }

    /// Currency of the result.
    #[wasm_bindgen(getter)]
    pub fn currency(&self) -> String {
        self.inner.mean.currency().to_string()
    }

    /// Standard error of the estimate.
    #[wasm_bindgen(getter)]
    pub fn stderr(&self) -> f64 {
        self.inner.stderr
    }

    /// Number of paths used.
    #[wasm_bindgen(getter, js_name = numPaths)]
    pub fn num_paths(&self) -> usize {
        self.inner.num_paths
    }

    /// 95% confidence interval lower bound.
    #[wasm_bindgen(getter, js_name = ci95Lower)]
    pub fn ci95_lower(&self) -> f64 {
        self.inner.ci_95.0.amount()
    }

    /// 95% confidence interval upper bound.
    #[wasm_bindgen(getter, js_name = ci95Upper)]
    pub fn ci95_upper(&self) -> f64 {
        self.inner.ci_95.1.amount()
    }

    /// Relative standard error (stderr / mean).
    #[wasm_bindgen(js_name = relativeStderr)]
    pub fn relative_stderr(&self) -> f64 {
        self.inner.relative_stderr()
    }
}

// =============================================================================
// LsmcPricer
// =============================================================================

/// LSMC (Longstaff-Schwartz Monte Carlo) pricer for American/Bermudan options.
#[wasm_bindgen(js_name = LsmcPricer)]
pub struct JsLsmcPricer {
    config: LsmcConfig,
}

#[wasm_bindgen(js_class = LsmcPricer)]
impl JsLsmcPricer {
    /// Create a new LSMC pricer with the given config.
    #[wasm_bindgen(constructor)]
    pub fn new(config: &JsLsmcConfig) -> JsLsmcPricer {
        JsLsmcPricer {
            config: config.inner.clone(),
        }
    }

    /// Price an American put using polynomial basis.
    #[wasm_bindgen(js_name = pricePutPolynomial)]
    #[allow(clippy::too_many_arguments)]
    pub fn price_put_polynomial(
        &self,
        initial_spot: f64,
        r: f64,
        q: f64,
        sigma: f64,
        time_to_maturity: f64,
        num_steps: usize,
        strike: f64,
        degree: usize,
        currency: &str,
    ) -> Result<JsLsmcResult, JsValue> {
        let ccy = Currency::from_str(currency)
            .map_err(|e| js_error(format!("Invalid currency: {}", e)))?;
        let process = GbmProcess::with_params(r, q, sigma);
        let pricer = LsmcPricer::new(self.config.clone());
        let exercise = AmericanPut { strike };
        let basis = PolynomialBasis::new(degree);
        pricer
            .price(
                &process,
                initial_spot,
                time_to_maturity,
                num_steps,
                &exercise,
                &basis,
                ccy,
                r,
            )
            .map(|inner| JsLsmcResult { inner })
            .map_err(core_to_js)
    }

    /// Price an American call using polynomial basis.
    #[wasm_bindgen(js_name = priceCallPolynomial)]
    #[allow(clippy::too_many_arguments)]
    pub fn price_call_polynomial(
        &self,
        initial_spot: f64,
        r: f64,
        q: f64,
        sigma: f64,
        time_to_maturity: f64,
        num_steps: usize,
        strike: f64,
        degree: usize,
        currency: &str,
    ) -> Result<JsLsmcResult, JsValue> {
        let ccy = Currency::from_str(currency)
            .map_err(|e| js_error(format!("Invalid currency: {}", e)))?;
        let process = GbmProcess::with_params(r, q, sigma);
        let pricer = LsmcPricer::new(self.config.clone());
        let exercise = AmericanCall { strike };
        let basis = PolynomialBasis::new(degree);
        pricer
            .price(
                &process,
                initial_spot,
                time_to_maturity,
                num_steps,
                &exercise,
                &basis,
                ccy,
                r,
            )
            .map(|inner| JsLsmcResult { inner })
            .map_err(core_to_js)
    }

    /// Price an American put using Laguerre basis.
    #[wasm_bindgen(js_name = pricePutLaguerre)]
    #[allow(clippy::too_many_arguments)]
    pub fn price_put_laguerre(
        &self,
        initial_spot: f64,
        r: f64,
        q: f64,
        sigma: f64,
        time_to_maturity: f64,
        num_steps: usize,
        strike: f64,
        degree: usize,
        currency: &str,
    ) -> Result<JsLsmcResult, JsValue> {
        let ccy = Currency::from_str(currency)
            .map_err(|e| js_error(format!("Invalid currency: {}", e)))?;
        let process = GbmProcess::with_params(r, q, sigma);
        let pricer = LsmcPricer::new(self.config.clone());
        let exercise = AmericanPut { strike };
        let basis = LaguerreBasis::new(degree, strike);
        pricer
            .price(
                &process,
                initial_spot,
                time_to_maturity,
                num_steps,
                &exercise,
                &basis,
                ccy,
                r,
            )
            .map(|inner| JsLsmcResult { inner })
            .map_err(core_to_js)
    }

    /// Price an American call using Laguerre basis.
    #[wasm_bindgen(js_name = priceCallLaguerre)]
    #[allow(clippy::too_many_arguments)]
    pub fn price_call_laguerre(
        &self,
        initial_spot: f64,
        r: f64,
        q: f64,
        sigma: f64,
        time_to_maturity: f64,
        num_steps: usize,
        strike: f64,
        degree: usize,
        currency: &str,
    ) -> Result<JsLsmcResult, JsValue> {
        let ccy = Currency::from_str(currency)
            .map_err(|e| js_error(format!("Invalid currency: {}", e)))?;
        let process = GbmProcess::with_params(r, q, sigma);
        let pricer = LsmcPricer::new(self.config.clone());
        let exercise = AmericanCall { strike };
        let basis = LaguerreBasis::new(degree, strike);
        pricer
            .price(
                &process,
                initial_spot,
                time_to_maturity,
                num_steps,
                &exercise,
                &basis,
                ccy,
                r,
            )
            .map(|inner| JsLsmcResult { inner })
            .map_err(core_to_js)
    }
}
