//! Python bindings for variance-reduction utilities.
//!
//! Exposes the antithetic-variates configuration and Black-Scholes analytical
//! formulas used as control variates.

use pyo3::prelude::*;

/// Configuration for antithetic-variates variance reduction.
///
/// Antithetic pairing generates complementary paths ``(Z, -Z)`` and averages
/// their payoffs, reducing variance through negative correlation.
///
/// Pass ``antithetic=True`` to ``price_european()`` to enable this at the
/// engine level. This configuration object is provided for transparency.
///
/// Args:
///     num_pairs: Number of path pairs to simulate.
///     discount_factor: PV multiplier applied to each payoff.
///     currency: ISO currency code for payoff amounts.
#[pyclass(
    module = "finstack.valuations.common.monte_carlo",
    name = "AntitheticConfig",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyAntitheticConfig {
    /// Number of path pairs.
    pub(crate) num_pairs: usize,
    /// PV discount factor.
    pub(crate) discount_factor: f64,
    /// ISO currency code.
    pub(crate) currency: String,
}

#[pymethods]
impl PyAntitheticConfig {
    #[new]
    #[pyo3(signature = (num_pairs, discount_factor=1.0, currency="USD"))]
    fn new(num_pairs: usize, discount_factor: f64, currency: &str) -> Self {
        Self {
            num_pairs,
            discount_factor,
            currency: currency.to_string(),
        }
    }

    /// Number of path pairs.
    #[getter]
    fn num_pairs(&self) -> usize {
        self.num_pairs
    }

    /// Discount factor.
    #[getter]
    fn discount_factor(&self) -> f64 {
        self.discount_factor
    }

    /// ISO currency code.
    #[getter]
    fn currency(&self) -> &str {
        &self.currency
    }

    fn __repr__(&self) -> String {
        format!(
            "AntitheticConfig(num_pairs={}, discount_factor={}, currency='{}')",
            self.num_pairs, self.discount_factor, self.currency
        )
    }
}

/// Black-Scholes analytical price for a European call option.
///
/// Useful as a benchmark to compare against Monte Carlo estimates or as a
/// control variate for variance reduction.
///
/// Args:
///     spot: Current spot price.
///     strike: Strike price.
///     time_to_maturity: Time to expiry in years.
///     rate: Risk-free rate (decimal).
///     dividend_yield: Dividend yield (decimal).
///     volatility: Annualised volatility (decimal).
///
/// Returns:
///     Call option price.
#[pyfunction]
#[pyo3(signature = (spot, strike, time_to_maturity, rate, dividend_yield, volatility))]
pub(crate) fn black_scholes_call(
    spot: f64,
    strike: f64,
    time_to_maturity: f64,
    rate: f64,
    dividend_yield: f64,
    volatility: f64,
) -> f64 {
    finstack_monte_carlo::variance_reduction::control_variate::black_scholes_call(
        spot,
        strike,
        time_to_maturity,
        rate,
        dividend_yield,
        volatility,
    )
}

/// Black-Scholes analytical price for a European put option.
///
/// Args:
///     spot: Current spot price.
///     strike: Strike price.
///     time_to_maturity: Time to expiry in years.
///     rate: Risk-free rate (decimal).
///     dividend_yield: Dividend yield (decimal).
///     volatility: Annualised volatility (decimal).
///
/// Returns:
///     Put option price.
#[pyfunction]
#[pyo3(signature = (spot, strike, time_to_maturity, rate, dividend_yield, volatility))]
pub(crate) fn black_scholes_put(
    spot: f64,
    strike: f64,
    time_to_maturity: f64,
    rate: f64,
    dividend_yield: f64,
    volatility: f64,
) -> f64 {
    finstack_monte_carlo::variance_reduction::control_variate::black_scholes_put(
        spot,
        strike,
        time_to_maturity,
        rate,
        dividend_yield,
        volatility,
    )
}
