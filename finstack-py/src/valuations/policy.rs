//! Python bindings for policy implementations.

use pyo3::prelude::*;

use finstack_valuations::policy::policies::{
    DSCRSweepPolicy, GridMarginPolicy, IndexFallbackPolicy,
};
use std::collections::HashMap;

use crate::core::money::PyMoney;

/// Python wrapper for grid margin policy.
#[pyclass(name = "GridMarginPolicy")]
#[derive(Clone)]
pub struct PyGridMarginPolicy {
    inner: GridMarginPolicy,
}

impl Default for PyGridMarginPolicy {
    fn default() -> Self {
        Self {
            inner: GridMarginPolicy::new(),
        }
    }
}

#[pymethods]
impl PyGridMarginPolicy {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate credit spread based on rating, sector, and maturity.
    pub fn calculate_spread(&self, rating: &str, sector: Option<&str>, maturity_years: f64) -> f64 {
        self.inner.calculate_spread(rating, sector, maturity_years)
    }

    /// Set base spread for a rating.
    pub fn set_base_spread(&mut self, rating: String, spread_bps: f64) {
        self.inner.base_spreads.insert(rating, spread_bps);
    }

    /// Set sector adjustment.
    pub fn set_sector_adjustment(&mut self, sector: String, adjustment_bps: f64) {
        self.inner.sector_adjustments.insert(sector, adjustment_bps);
    }

    /// Add maturity bucket.
    pub fn add_maturity_bucket(&mut self, years: f64, adjustment_bps: f64) {
        self.inner.maturity_buckets.push((years, adjustment_bps));
        self.inner
            .maturity_buckets
            .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    }

    /// Set floor spread.
    pub fn set_floor_spread(&mut self, spread_bps: f64) {
        self.inner.floor_spread = spread_bps;
    }

    /// Set cap spread.
    pub fn set_cap_spread(&mut self, spread_bps: f64) {
        self.inner.cap_spread = spread_bps;
    }

    fn __str__(&self) -> String {
        format!(
            "GridMarginPolicy(ratings={}, sectors={})",
            self.inner.base_spreads.len(),
            self.inner.sector_adjustments.len()
        )
    }

    fn __repr__(&self) -> String {
        self.__str__()
    }
}

/// Python wrapper for index fallback policy.
#[pyclass(name = "IndexFallbackPolicy")]
#[derive(Clone)]
pub struct PyIndexFallbackPolicy {
    inner: IndexFallbackPolicy,
}

impl Default for PyIndexFallbackPolicy {
    fn default() -> Self {
        Self {
            inner: IndexFallbackPolicy::new(),
        }
    }
}

#[pymethods]
impl PyIndexFallbackPolicy {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get rate for an index with fallback logic.
    pub fn get_rate(&self, index: &str, available_rates: HashMap<String, f64>) -> PyResult<f64> {
        self.inner
            .get_rate(index, &available_rates)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Set fallback chain for an index.
    pub fn set_fallback_chain(&mut self, index: String, fallbacks: Vec<String>) {
        self.inner.fallback_chain.insert(index, fallbacks);
    }

    /// Set static fallback rate.
    pub fn set_static_rate(&mut self, index: String, rate: f64) {
        self.inner.static_rates.insert(index, rate);
    }

    /// Set fallback spread adjustment.
    pub fn set_fallback_spread(&mut self, index: String, spread_bps: f64) {
        self.inner.fallback_spreads.insert(index, spread_bps);
    }

    fn __str__(&self) -> String {
        format!(
            "IndexFallbackPolicy(chains={}, static_rates={})",
            self.inner.fallback_chain.len(),
            self.inner.static_rates.len()
        )
    }

    fn __repr__(&self) -> String {
        self.__str__()
    }
}

/// Python wrapper for DSCR sweep policy.
#[pyclass(name = "DSCRSweepPolicy")]
#[derive(Clone)]
pub struct PyDSCRSweepPolicy {
    inner: DSCRSweepPolicy,
}

impl Default for PyDSCRSweepPolicy {
    fn default() -> Self {
        Self {
            inner: DSCRSweepPolicy::new(),
        }
    }
}

#[pymethods]
impl PyDSCRSweepPolicy {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate sweep percentage based on DSCR.
    pub fn calculate_sweep(&self, dscr: f64) -> f64 {
        self.inner.calculate_sweep(dscr)
    }

    /// Calculate sweep amount based on DSCR and available cash.
    pub fn calculate_sweep_amount(
        &self,
        dscr: f64,
        excess_cash: PyMoney,
        minimum_cash: PyMoney,
    ) -> PyMoney {
        let amount =
            self.inner
                .calculate_sweep_amount(dscr, excess_cash.inner(), minimum_cash.inner());
        PyMoney::from_inner(amount)
    }

    /// Add sweep tier.
    pub fn add_sweep_tier(&mut self, dscr_threshold: f64, sweep_percentage: f64) {
        self.inner
            .sweep_schedule
            .push((dscr_threshold, sweep_percentage));
        self.inner
            .sweep_schedule
            .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    }

    /// Set minimum DSCR.
    pub fn set_min_dscr(&mut self, min_dscr: f64) {
        self.inner.min_dscr = min_dscr;
    }

    /// Set maximum sweep percentage.
    pub fn set_max_sweep(&mut self, max_sweep: f64) {
        self.inner.max_sweep = max_sweep;
    }

    fn __str__(&self) -> String {
        format!(
            "DSCRSweepPolicy(tiers={}, min_dscr={:.2}, max_sweep={:.1}%)",
            self.inner.sweep_schedule.len(),
            self.inner.min_dscr,
            self.inner.max_sweep * 100.0
        )
    }

    fn __repr__(&self) -> String {
        self.__str__()
    }
}

/// Python function to create a default grid margin policy.
#[pyfunction]
#[pyo3(name = "create_default_grid_margin_policy")]
pub fn py_create_default_grid_margin_policy() -> PyGridMarginPolicy {
    PyGridMarginPolicy::new()
}

/// Python function to create a default index fallback policy.
#[pyfunction]
#[pyo3(name = "create_default_index_fallback_policy")]
pub fn py_create_default_index_fallback_policy() -> PyIndexFallbackPolicy {
    PyIndexFallbackPolicy::new()
}

/// Python function to create a default DSCR sweep policy.
#[pyfunction]
#[pyo3(name = "create_default_dscr_sweep_policy")]
pub fn py_create_default_dscr_sweep_policy() -> PyDSCRSweepPolicy {
    PyDSCRSweepPolicy::new()
}

/// Register the policy module with Python.
pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "policy")?;

    m.add_class::<PyGridMarginPolicy>()?;
    m.add_class::<PyIndexFallbackPolicy>()?;
    m.add_class::<PyDSCRSweepPolicy>()?;

    m.add_function(pyo3::wrap_pyfunction!(
        py_create_default_grid_margin_policy,
        &m
    )?)?;
    m.add_function(pyo3::wrap_pyfunction!(
        py_create_default_index_fallback_policy,
        &m
    )?)?;
    m.add_function(pyo3::wrap_pyfunction!(
        py_create_default_dscr_sweep_policy,
        &m
    )?)?;

    parent.add_submodule(&m)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("finstack.policy", &m)?;

    Ok(())
}
