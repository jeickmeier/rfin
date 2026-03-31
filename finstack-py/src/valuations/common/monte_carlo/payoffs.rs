//! Python bindings for vanilla Monte Carlo payoffs.
//!
//! Each wrapper is a lightweight data class whose fields are read by
//! [`super::engine::price_european`] to construct the corresponding Rust payoff
//! on the fly.

use pyo3::prelude::*;

/// European call payoff: ``max(S_T - K, 0) * notional``.
///
/// Args:
///     strike: Strike price.
///     notional: Linear payoff scaling (default 1.0).
#[pyclass(
    module = "finstack.valuations.common.monte_carlo",
    name = "EuropeanCall",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyEuropeanCall {
    /// Strike price.
    pub(crate) strike: f64,
    /// Notional amount.
    pub(crate) notional: f64,
}

#[pymethods]
impl PyEuropeanCall {
    #[new]
    #[pyo3(signature = (strike, notional=1.0))]
    fn new(strike: f64, notional: f64) -> Self {
        Self { strike, notional }
    }

    /// Strike price.
    #[getter]
    fn strike(&self) -> f64 {
        self.strike
    }

    /// Notional amount.
    #[getter]
    fn notional(&self) -> f64 {
        self.notional
    }

    fn __repr__(&self) -> String {
        format!(
            "EuropeanCall(strike={}, notional={})",
            self.strike, self.notional
        )
    }
}

/// European put payoff: ``max(K - S_T, 0) * notional``.
///
/// Args:
///     strike: Strike price.
///     notional: Linear payoff scaling (default 1.0).
#[pyclass(
    module = "finstack.valuations.common.monte_carlo",
    name = "EuropeanPut",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyEuropeanPut {
    /// Strike price.
    pub(crate) strike: f64,
    /// Notional amount.
    pub(crate) notional: f64,
}

#[pymethods]
impl PyEuropeanPut {
    #[new]
    #[pyo3(signature = (strike, notional=1.0))]
    fn new(strike: f64, notional: f64) -> Self {
        Self { strike, notional }
    }

    /// Strike price.
    #[getter]
    fn strike(&self) -> f64 {
        self.strike
    }

    /// Notional amount.
    #[getter]
    fn notional(&self) -> f64 {
        self.notional
    }

    fn __repr__(&self) -> String {
        format!(
            "EuropeanPut(strike={}, notional={})",
            self.strike, self.notional
        )
    }
}

/// Digital (binary) option payoff.
///
/// - **Call**: pays ``payout`` if ``S_T > strike``.
/// - **Put**: pays ``payout`` if ``S_T < strike``.
///
/// Args:
///     strike: Strike price.
///     payout: Fixed payout amount.
///     is_call: ``True`` for a digital call, ``False`` for a digital put.
#[pyclass(
    module = "finstack.valuations.common.monte_carlo",
    name = "Digital",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyDigital {
    /// Strike price.
    pub(crate) strike: f64,
    /// Payout amount.
    pub(crate) payout: f64,
    /// Call (True) or put (False).
    pub(crate) is_call: bool,
}

#[pymethods]
impl PyDigital {
    #[new]
    #[pyo3(signature = (strike, payout, is_call=true))]
    fn new(strike: f64, payout: f64, is_call: bool) -> Self {
        Self {
            strike,
            payout,
            is_call,
        }
    }

    /// Strike price.
    #[getter]
    fn strike(&self) -> f64 {
        self.strike
    }

    /// Payout amount.
    #[getter]
    fn payout(&self) -> f64 {
        self.payout
    }

    /// Whether this is a call (True) or put (False).
    #[getter]
    fn is_call(&self) -> bool {
        self.is_call
    }

    fn __repr__(&self) -> String {
        let kind = if self.is_call { "call" } else { "put" };
        format!(
            "Digital(strike={}, payout={}, type={})",
            self.strike, self.payout, kind
        )
    }
}

/// Forward contract payoff.
///
/// - **Long**: pays ``(S_T - F) * notional``.
/// - **Short**: pays ``(F - S_T) * notional``.
///
/// Unlike an option, the payoff can be negative.
///
/// Args:
///     forward_price: Agreed forward price.
///     notional: Linear payoff scaling (default 1.0).
///     is_long: ``True`` for a long position, ``False`` for short.
#[pyclass(
    module = "finstack.valuations.common.monte_carlo",
    name = "Forward",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyForward {
    /// Forward price.
    pub(crate) forward_price: f64,
    /// Notional amount.
    pub(crate) notional: f64,
    /// Long (True) or short (False).
    pub(crate) is_long: bool,
}

#[pymethods]
impl PyForward {
    #[new]
    #[pyo3(signature = (forward_price, notional=1.0, is_long=true))]
    fn new(forward_price: f64, notional: f64, is_long: bool) -> Self {
        Self {
            forward_price,
            notional,
            is_long,
        }
    }

    /// Forward price.
    #[getter]
    fn forward_price(&self) -> f64 {
        self.forward_price
    }

    /// Notional amount.
    #[getter]
    fn notional(&self) -> f64 {
        self.notional
    }

    /// Whether this is a long (True) or short (False) position.
    #[getter]
    fn is_long(&self) -> bool {
        self.is_long
    }

    fn __repr__(&self) -> String {
        let side = if self.is_long { "long" } else { "short" };
        format!(
            "Forward(forward_price={}, notional={}, side={})",
            self.forward_price, self.notional, side
        )
    }
}
