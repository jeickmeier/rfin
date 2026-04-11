//! Payoff type bindings.
//!
//! These types hold parameters for Python construction and getter access.
//! The actual Rust payoff objects are constructed on-demand at pricing time.

#![allow(dead_code)]

use pyo3::prelude::*;

// ---------------------------------------------------------------------------
// Vanilla
// ---------------------------------------------------------------------------

/// European call option payoff.
#[pyclass(name = "EuropeanCall", module = "finstack.monte_carlo", frozen)]
pub struct PyEuropeanCall {
    pub(super) strike: f64,
    pub(super) notional: f64,
    pub(super) maturity_step: usize,
}

#[pymethods]
impl PyEuropeanCall {
    #[new]
    #[pyo3(signature = (strike, notional=1.0, maturity_step=252))]
    fn new(strike: f64, notional: f64, maturity_step: usize) -> Self {
        Self {
            strike,
            notional,
            maturity_step,
        }
    }
    #[getter]
    fn strike(&self) -> f64 {
        self.strike
    }
    #[getter]
    fn notional(&self) -> f64 {
        self.notional
    }
    #[getter]
    fn maturity_step(&self) -> usize {
        self.maturity_step
    }
    fn __repr__(&self) -> String {
        format!(
            "EuropeanCall(K={}, N={}, step={})",
            self.strike, self.notional, self.maturity_step
        )
    }
}

/// European put option payoff.
#[pyclass(name = "EuropeanPut", module = "finstack.monte_carlo", frozen)]
pub struct PyEuropeanPut {
    pub(super) strike: f64,
    pub(super) notional: f64,
    pub(super) maturity_step: usize,
}

#[pymethods]
impl PyEuropeanPut {
    #[new]
    #[pyo3(signature = (strike, notional=1.0, maturity_step=252))]
    fn new(strike: f64, notional: f64, maturity_step: usize) -> Self {
        Self {
            strike,
            notional,
            maturity_step,
        }
    }
    #[getter]
    fn strike(&self) -> f64 {
        self.strike
    }
    #[getter]
    fn notional(&self) -> f64 {
        self.notional
    }
    #[getter]
    fn maturity_step(&self) -> usize {
        self.maturity_step
    }
    fn __repr__(&self) -> String {
        format!(
            "EuropeanPut(K={}, N={}, step={})",
            self.strike, self.notional, self.maturity_step
        )
    }
}

// ---------------------------------------------------------------------------
// Digital
// ---------------------------------------------------------------------------

/// Digital (binary) call payoff — pays notional if S > strike at maturity.
#[pyclass(name = "DigitalCall", module = "finstack.monte_carlo", frozen)]
pub struct PyDigitalCall {
    pub(super) strike: f64,
    pub(super) notional: f64,
    pub(super) maturity_step: usize,
}

#[pymethods]
impl PyDigitalCall {
    #[new]
    #[pyo3(signature = (strike, notional=1.0, maturity_step=252))]
    fn new(strike: f64, notional: f64, maturity_step: usize) -> Self {
        Self {
            strike,
            notional,
            maturity_step,
        }
    }
    #[getter]
    fn strike(&self) -> f64 {
        self.strike
    }
    fn __repr__(&self) -> String {
        format!(
            "DigitalCall(K={}, step={})",
            self.strike, self.maturity_step
        )
    }
}

/// Digital (binary) put payoff — pays notional if S < strike at maturity.
#[pyclass(name = "DigitalPut", module = "finstack.monte_carlo", frozen)]
pub struct PyDigitalPut {
    pub(super) strike: f64,
    pub(super) notional: f64,
    pub(super) maturity_step: usize,
}

#[pymethods]
impl PyDigitalPut {
    #[new]
    #[pyo3(signature = (strike, notional=1.0, maturity_step=252))]
    fn new(strike: f64, notional: f64, maturity_step: usize) -> Self {
        Self {
            strike,
            notional,
            maturity_step,
        }
    }
    #[getter]
    fn strike(&self) -> f64 {
        self.strike
    }
    fn __repr__(&self) -> String {
        format!("DigitalPut(K={}, step={})", self.strike, self.maturity_step)
    }
}

// ---------------------------------------------------------------------------
// Forward
// ---------------------------------------------------------------------------

/// Long forward payoff (S - K at maturity).
#[pyclass(name = "ForwardLong", module = "finstack.monte_carlo", frozen)]
pub struct PyForwardLong {
    pub(super) strike: f64,
    pub(super) notional: f64,
    pub(super) maturity_step: usize,
}

#[pymethods]
impl PyForwardLong {
    #[new]
    #[pyo3(signature = (strike, notional=1.0, maturity_step=252))]
    fn new(strike: f64, notional: f64, maturity_step: usize) -> Self {
        Self {
            strike,
            notional,
            maturity_step,
        }
    }
    #[getter]
    fn strike(&self) -> f64 {
        self.strike
    }
    fn __repr__(&self) -> String {
        format!(
            "ForwardLong(K={}, step={})",
            self.strike, self.maturity_step
        )
    }
}

/// Short forward payoff (K - S at maturity).
#[pyclass(name = "ForwardShort", module = "finstack.monte_carlo", frozen)]
pub struct PyForwardShort {
    pub(super) strike: f64,
    pub(super) notional: f64,
    pub(super) maturity_step: usize,
}

#[pymethods]
impl PyForwardShort {
    #[new]
    #[pyo3(signature = (strike, notional=1.0, maturity_step=252))]
    fn new(strike: f64, notional: f64, maturity_step: usize) -> Self {
        Self {
            strike,
            notional,
            maturity_step,
        }
    }
    #[getter]
    fn strike(&self) -> f64 {
        self.strike
    }
    fn __repr__(&self) -> String {
        format!(
            "ForwardShort(K={}, step={})",
            self.strike, self.maturity_step
        )
    }
}

// ---------------------------------------------------------------------------
// Asian
// ---------------------------------------------------------------------------

/// Asian (arithmetic average) call payoff.
#[pyclass(name = "AsianCall", module = "finstack.monte_carlo", frozen)]
pub struct PyAsianCall {
    pub(super) strike: f64,
    pub(super) notional: f64,
    pub(super) maturity_step: usize,
}

#[pymethods]
impl PyAsianCall {
    #[new]
    #[pyo3(signature = (strike, notional=1.0, maturity_step=252))]
    fn new(strike: f64, notional: f64, maturity_step: usize) -> Self {
        Self {
            strike,
            notional,
            maturity_step,
        }
    }
    #[getter]
    fn strike(&self) -> f64 {
        self.strike
    }
    fn __repr__(&self) -> String {
        format!("AsianCall(K={}, step={})", self.strike, self.maturity_step)
    }
}

/// Asian (arithmetic average) put payoff.
#[pyclass(name = "AsianPut", module = "finstack.monte_carlo", frozen)]
pub struct PyAsianPut {
    pub(super) strike: f64,
    pub(super) notional: f64,
    pub(super) maturity_step: usize,
}

#[pymethods]
impl PyAsianPut {
    #[new]
    #[pyo3(signature = (strike, notional=1.0, maturity_step=252))]
    fn new(strike: f64, notional: f64, maturity_step: usize) -> Self {
        Self {
            strike,
            notional,
            maturity_step,
        }
    }
    #[getter]
    fn strike(&self) -> f64 {
        self.strike
    }
    fn __repr__(&self) -> String {
        format!("AsianPut(K={}, step={})", self.strike, self.maturity_step)
    }
}

// ---------------------------------------------------------------------------
// Barrier
// ---------------------------------------------------------------------------

/// Barrier option payoff.
#[pyclass(name = "BarrierOption", module = "finstack.monte_carlo", frozen)]
pub struct PyBarrierOption {
    pub(super) strike: f64,
    pub(super) barrier: f64,
    pub(super) notional: f64,
    pub(super) is_call: bool,
    pub(super) is_up: bool,
    pub(super) is_knock_out: bool,
    pub(super) maturity_step: usize,
}

#[pymethods]
impl PyBarrierOption {
    /// Create a barrier option payoff.
    #[new]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (strike, barrier, is_call=true, is_up=true, is_knock_out=true, notional=1.0, maturity_step=252))]
    fn new(
        strike: f64,
        barrier: f64,
        is_call: bool,
        is_up: bool,
        is_knock_out: bool,
        notional: f64,
        maturity_step: usize,
    ) -> Self {
        Self {
            strike,
            barrier,
            notional,
            is_call,
            is_up,
            is_knock_out,
            maturity_step,
        }
    }

    #[getter]
    fn strike(&self) -> f64 {
        self.strike
    }
    #[getter]
    fn barrier(&self) -> f64 {
        self.barrier
    }

    fn __repr__(&self) -> String {
        let dir = if self.is_up { "Up" } else { "Down" };
        let kind = if self.is_knock_out { "Out" } else { "In" };
        let opt = if self.is_call { "Call" } else { "Put" };
        format!(
            "BarrierOption({dir}-and-{kind} {opt}, K={}, B={})",
            self.strike, self.barrier
        )
    }
}

// ---------------------------------------------------------------------------
// Basket
// ---------------------------------------------------------------------------

/// Basket call payoff (average of basket - strike).
#[pyclass(name = "BasketCall", module = "finstack.monte_carlo", frozen)]
pub struct PyBasketCall {
    pub(super) strike: f64,
    pub(super) weights: Vec<f64>,
    pub(super) notional: f64,
    pub(super) maturity_step: usize,
}

#[pymethods]
impl PyBasketCall {
    #[new]
    #[pyo3(signature = (strike, weights, notional=1.0, maturity_step=252))]
    fn new(strike: f64, weights: Vec<f64>, notional: f64, maturity_step: usize) -> Self {
        Self {
            strike,
            weights,
            notional,
            maturity_step,
        }
    }
    #[getter]
    fn strike(&self) -> f64 {
        self.strike
    }
    fn __repr__(&self) -> String {
        format!(
            "BasketCall(K={}, assets={})",
            self.strike,
            self.weights.len()
        )
    }
}

/// Basket put payoff (strike - average of basket).
#[pyclass(name = "BasketPut", module = "finstack.monte_carlo", frozen)]
pub struct PyBasketPut {
    pub(super) strike: f64,
    pub(super) weights: Vec<f64>,
    pub(super) notional: f64,
    pub(super) maturity_step: usize,
}

#[pymethods]
impl PyBasketPut {
    #[new]
    #[pyo3(signature = (strike, weights, notional=1.0, maturity_step=252))]
    fn new(strike: f64, weights: Vec<f64>, notional: f64, maturity_step: usize) -> Self {
        Self {
            strike,
            weights,
            notional,
            maturity_step,
        }
    }
    #[getter]
    fn strike(&self) -> f64 {
        self.strike
    }
    fn __repr__(&self) -> String {
        format!(
            "BasketPut(K={}, assets={})",
            self.strike,
            self.weights.len()
        )
    }
}

// ---------------------------------------------------------------------------
// American (LSMC)
// ---------------------------------------------------------------------------

/// American put option payoff (for use with LSMC pricer).
#[pyclass(name = "AmericanPut", module = "finstack.monte_carlo", frozen)]
pub struct PyAmericanPut {
    pub(super) strike: f64,
}

#[pymethods]
impl PyAmericanPut {
    #[new]
    fn new(strike: f64) -> Self {
        Self { strike }
    }
    #[getter]
    fn strike(&self) -> f64 {
        self.strike
    }
    fn __repr__(&self) -> String {
        format!("AmericanPut(K={})", self.strike)
    }
}

/// American call option payoff (for use with LSMC pricer).
#[pyclass(name = "AmericanCall", module = "finstack.monte_carlo", frozen)]
pub struct PyAmericanCall {
    pub(super) strike: f64,
}

#[pymethods]
impl PyAmericanCall {
    #[new]
    fn new(strike: f64) -> Self {
        Self { strike }
    }
    #[getter]
    fn strike(&self) -> f64 {
        self.strike
    }
    fn __repr__(&self) -> String {
        format!("AmericanCall(K={})", self.strike)
    }
}

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEuropeanCall>()?;
    m.add_class::<PyEuropeanPut>()?;
    m.add_class::<PyDigitalCall>()?;
    m.add_class::<PyDigitalPut>()?;
    m.add_class::<PyForwardLong>()?;
    m.add_class::<PyForwardShort>()?;
    m.add_class::<PyAsianCall>()?;
    m.add_class::<PyAsianPut>()?;
    m.add_class::<PyBarrierOption>()?;
    m.add_class::<PyBasketCall>()?;
    m.add_class::<PyBasketPut>()?;
    m.add_class::<PyAmericanPut>()?;
    m.add_class::<PyAmericanCall>()?;
    Ok(())
}
