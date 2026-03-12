//! Python bindings for the XVA (Valuation Adjustments) framework.
//!
//! Wraps CVA computation, exposure profiling, netting, and collateral
//! from `finstack_valuations::xva`.

use crate::core::currency::{extract_currency, PyCurrency};
use crate::core::dates::utils::py_to_date;
use crate::core::market_data::context::PyMarketContext;
use crate::core::market_data::term_structures::{PyDiscountCurve, PyHazardCurve};
use crate::errors::core_to_py;
use crate::valuations::instruments::extract_instrument;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;
use std::sync::Arc;

use finstack_valuations::xva;

// ---------------------------------------------------------------------------
// FundingConfig
// ---------------------------------------------------------------------------

/// Funding cost/benefit configuration for FVA calculations.
#[pyclass(
    module = "finstack.valuations.xva",
    name = "FundingConfig",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyFundingConfig {
    pub(crate) inner: xva::types::FundingConfig,
}

#[pymethods]
impl PyFundingConfig {
    #[new]
    #[pyo3(signature = (funding_spread_bps, funding_benefit_bps=None))]
    fn new(funding_spread_bps: f64, funding_benefit_bps: Option<f64>) -> Self {
        Self {
            inner: xva::types::FundingConfig {
                funding_spread_bps,
                funding_benefit_bps,
            },
        }
    }

    #[getter]
    fn funding_spread_bps(&self) -> f64 {
        self.inner.funding_spread_bps
    }

    #[getter]
    fn funding_benefit_bps(&self) -> Option<f64> {
        self.inner.funding_benefit_bps
    }

    #[getter]
    fn effective_benefit_bps(&self) -> f64 {
        self.inner.effective_benefit_bps()
    }

    fn __repr__(&self) -> String {
        format!(
            "FundingConfig(funding_spread_bps={}, funding_benefit_bps={:?})",
            self.inner.funding_spread_bps, self.inner.funding_benefit_bps
        )
    }
}

// ---------------------------------------------------------------------------
// XvaConfig
// ---------------------------------------------------------------------------

/// Configuration for XVA calculations.
///
/// Args:
///     time_grid: Time points (in years) for the exposure simulation.
///         Defaults to a quarterly grid out to 30 years.
///     recovery_rate: Assumed recovery rate upon default (0 to 1, default 0.40).
#[pyclass(
    module = "finstack.valuations.xva",
    name = "XvaConfig",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyXvaConfig {
    pub(crate) inner: xva::types::XvaConfig,
}

#[pymethods]
impl PyXvaConfig {
    #[new]
    #[pyo3(signature = (time_grid=None, recovery_rate=0.40, own_recovery_rate=None, funding=None))]
    fn new(
        time_grid: Option<Vec<f64>>,
        recovery_rate: f64,
        own_recovery_rate: Option<f64>,
        funding: Option<PyFundingConfig>,
    ) -> PyResult<Self> {
        let time_grid = time_grid.unwrap_or_else(|| (1..=120).map(|i| i as f64 * 0.25).collect());
        let inner = xva::types::XvaConfig {
            time_grid,
            recovery_rate,
            own_recovery_rate,
            funding: funding.map(|cfg| cfg.inner),
        };
        inner.validate().map_err(core_to_py)?;
        Ok(Self { inner })
    }

    #[getter]
    fn time_grid(&self) -> Vec<f64> {
        self.inner.time_grid.clone()
    }

    #[getter]
    fn recovery_rate(&self) -> f64 {
        self.inner.recovery_rate
    }

    #[getter]
    fn own_recovery_rate(&self) -> Option<f64> {
        self.inner.own_recovery_rate
    }

    #[getter]
    fn funding(&self) -> Option<PyFundingConfig> {
        self.inner
            .funding
            .as_ref()
            .map(|cfg| PyFundingConfig { inner: cfg.clone() })
    }

    fn __repr__(&self) -> String {
        format!(
            "XvaConfig(grid_points={}, recovery_rate={:.2}, own_recovery_rate={:?}, funding={})",
            self.inner.time_grid.len(),
            self.inner.recovery_rate,
            self.inner.own_recovery_rate,
            if self.inner.funding.is_some() {
                "yes"
            } else {
                "none"
            }
        )
    }
}

// ---------------------------------------------------------------------------
// CsaTerms
// ---------------------------------------------------------------------------

/// Credit Support Annex terms for collateralization.
///
/// Args:
///     threshold: Threshold below which no collateral is required.
///     mta: Minimum transfer amount.
///     mpor_days: Margin period of risk in calendar days.
///     independent_amount: Additional collateral posted regardless of MtM.
#[pyclass(
    module = "finstack.valuations.xva",
    name = "CsaTerms",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyCsaTerms {
    pub(crate) inner: xva::types::CsaTerms,
}

#[pymethods]
impl PyCsaTerms {
    #[new]
    #[pyo3(signature = (threshold, mta, mpor_days=10, independent_amount=0.0))]
    fn new(threshold: f64, mta: f64, mpor_days: u32, independent_amount: f64) -> Self {
        Self {
            inner: xva::types::CsaTerms {
                threshold,
                mta,
                mpor_days,
                independent_amount,
            },
        }
    }

    #[getter]
    fn threshold(&self) -> f64 {
        self.inner.threshold
    }

    #[getter]
    fn mta(&self) -> f64 {
        self.inner.mta
    }

    #[getter]
    fn mpor_days(&self) -> u32 {
        self.inner.mpor_days
    }

    #[getter]
    fn independent_amount(&self) -> f64 {
        self.inner.independent_amount
    }

    fn __repr__(&self) -> String {
        format!(
            "CsaTerms(threshold={}, mta={}, mpor_days={}, ia={})",
            self.inner.threshold,
            self.inner.mta,
            self.inner.mpor_days,
            self.inner.independent_amount
        )
    }
}

// ---------------------------------------------------------------------------
// NettingSet
// ---------------------------------------------------------------------------

/// Netting set specification under an ISDA master agreement.
///
/// Args:
///     id: Unique identifier for this netting set.
///     counterparty_id: Counterparty identifier (maps to hazard curve).
///     csa: Optional :class:`CsaTerms` governing collateral exchange.
#[pyclass(
    module = "finstack.valuations.xva",
    name = "NettingSet",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyNettingSet {
    pub(crate) inner: xva::types::NettingSet,
}

#[pymethods]
impl PyNettingSet {
    #[new]
    #[pyo3(signature = (id, counterparty_id, csa=None, reporting_currency=None))]
    fn new(
        id: String,
        counterparty_id: String,
        csa: Option<PyCsaTerms>,
        reporting_currency: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let reporting_currency = match reporting_currency {
            Some(value) => Some(extract_currency(value)?),
            None => None,
        };
        Ok(Self {
            inner: xva::types::NettingSet {
                id,
                counterparty_id,
                csa: csa.map(|c| c.inner),
                reporting_currency,
            },
        })
    }

    #[getter]
    fn id(&self) -> &str {
        &self.inner.id
    }

    #[getter]
    fn counterparty_id(&self) -> &str {
        &self.inner.counterparty_id
    }

    #[getter]
    fn reporting_currency(&self) -> Option<PyCurrency> {
        self.inner.reporting_currency.map(PyCurrency::new)
    }

    fn __repr__(&self) -> String {
        format!(
            "NettingSet(id='{}', counterparty='{}', csa={})",
            self.inner.id,
            self.inner.counterparty_id,
            if self.inner.csa.is_some() {
                "yes"
            } else {
                "none"
            }
        )
    }
}

// ---------------------------------------------------------------------------
// ExposureProfile
// ---------------------------------------------------------------------------

/// Exposure profile computed at each time grid point.
///
/// Attributes:
///     times: Time points in years from the valuation date.
///     mtm_values: Portfolio mark-to-market at each time point.
///     epe: Expected positive exposure at each time point.
///     ene: Expected negative exposure at each time point.
#[pyclass(
    module = "finstack.valuations.xva",
    name = "ExposureProfile",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyExposureProfile {
    pub(crate) inner: xva::types::ExposureProfile,
}

#[pymethods]
impl PyExposureProfile {
    #[getter]
    fn times(&self) -> Vec<f64> {
        self.inner.times.clone()
    }

    #[getter]
    fn mtm_values(&self) -> Vec<f64> {
        self.inner.mtm_values.clone()
    }

    #[getter]
    fn epe(&self) -> Vec<f64> {
        self.inner.epe.clone()
    }

    #[getter]
    fn ene(&self) -> Vec<f64> {
        self.inner.ene.clone()
    }

    /// Simulation quality diagnostics, if any failures occurred.
    ///
    /// Returns a dict with ``market_roll_failures``, ``valuation_failures``,
    /// and ``total_time_points``; or ``None`` when every grid point succeeded.
    #[getter]
    fn diagnostics(&self) -> Option<std::collections::HashMap<&'static str, usize>> {
        self.inner.diagnostics.as_ref().map(|d| {
            let mut m = std::collections::HashMap::with_capacity(3);
            m.insert("market_roll_failures", d.market_roll_failures);
            m.insert("valuation_failures", d.valuation_failures);
            m.insert("total_time_points", d.total_time_points);
            m
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "ExposureProfile(points={}, max_epe={:.2})",
            self.inner.times.len(),
            self.inner.epe.iter().copied().fold(0.0_f64, f64::max)
        )
    }
}

// ---------------------------------------------------------------------------
// XvaResult
// ---------------------------------------------------------------------------

/// Result of XVA calculations.
///
/// Attributes:
///     cva: Unilateral CVA (positive = cost to the desk).
///     epe_profile: Expected positive exposure profile as ``[(time, value)]``.
///     ene_profile: Expected negative exposure profile.
///     pfe_profile: Potential future exposure profile (97.5%).
///     max_pfe: Maximum PFE across the profile.
///     effective_epe_profile: Non-decreasing effective EPE per Basel III SA-CCR.
///     effective_epe: Time-weighted average of effective EPE.
#[pyclass(
    module = "finstack.valuations.xva",
    name = "XvaResult",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyXvaResult {
    inner: xva::types::XvaResult,
}

#[pymethods]
impl PyXvaResult {
    #[getter]
    fn cva(&self) -> f64 {
        self.inner.cva
    }

    #[getter]
    fn dva(&self) -> Option<f64> {
        self.inner.dva
    }

    #[getter]
    fn fva(&self) -> Option<f64> {
        self.inner.fva
    }

    #[getter]
    fn bilateral_cva(&self) -> Option<f64> {
        self.inner.bilateral_cva
    }

    #[getter]
    fn epe_profile(&self) -> Vec<(f64, f64)> {
        self.inner.epe_profile.clone()
    }

    #[getter]
    fn ene_profile(&self) -> Vec<(f64, f64)> {
        self.inner.ene_profile.clone()
    }

    #[getter]
    fn pfe_profile(&self) -> Vec<(f64, f64)> {
        self.inner.pfe_profile.clone()
    }

    #[getter]
    fn max_pfe(&self) -> f64 {
        self.inner.max_pfe
    }

    #[getter]
    fn effective_epe_profile(&self) -> Vec<(f64, f64)> {
        self.inner.effective_epe_profile.clone()
    }

    #[getter]
    fn effective_epe(&self) -> f64 {
        self.inner.effective_epe
    }

    fn __repr__(&self) -> String {
        format!(
            "XvaResult(cva={:.4}, max_pfe={:.2}, effective_epe={:.2})",
            self.inner.cva, self.inner.max_pfe, self.inner.effective_epe
        )
    }
}

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

/// Apply close-out netting to a set of instrument mark-to-market values.
///
/// Args:
///     instrument_values: Individual instrument MtM values.
///
/// Returns:
///     float: Net positive exposure ``max(sum(values), 0)``.
#[pyfunction]
fn apply_netting(instrument_values: Vec<f64>) -> f64 {
    xva::netting::apply_netting(&instrument_values)
}

/// Apply CSA collateral terms to reduce gross exposure.
///
/// Args:
///     gross_exposure: Portfolio exposure before collateral.
///     csa: :class:`CsaTerms` governing collateral exchange.
///
/// Returns:
///     float: Net exposure after collateral, always non-negative.
#[pyfunction]
fn apply_collateral(gross_exposure: f64, csa: &PyCsaTerms) -> f64 {
    xva::netting::apply_collateral(gross_exposure, &csa.inner)
}

/// Compute exposure profile for a portfolio of instruments.
///
/// For each time point in the configuration's time grid, instruments are
/// revalued, netting is applied, and CSA collateral reduction is applied.
///
/// Args:
///     instruments: List of instrument objects from ``finstack.valuations``.
///     market: :class:`~finstack.core.market_data.context.MarketContext` with curves.
///     as_of: Valuation date.
///     config: :class:`XvaConfig` controlling the exposure simulation.
///     netting_set: :class:`NettingSet` specifying counterparty and CSA.
///
/// Returns:
///     ExposureProfile: EPE, ENE, and MtM at each time point.
///
/// Raises:
///     FinstackError: If configuration is invalid or >50% of time points fail.
#[pyfunction]
fn compute_exposure_profile<'py>(
    instruments: &Bound<'py, PyList>,
    market: &PyMarketContext,
    as_of: &Bound<'py, PyAny>,
    config: &PyXvaConfig,
    netting_set: &PyNettingSet,
) -> PyResult<PyExposureProfile> {
    let as_of_date = py_to_date(as_of)?;
    let mut arcs: Vec<Arc<dyn finstack_valuations::instruments::Instrument>> = Vec::new();
    for item in instruments.iter() {
        let handle = extract_instrument(&item)?;
        arcs.push(handle.instrument);
    }
    let profile = xva::exposure::compute_exposure_profile(
        &arcs,
        &market.inner,
        as_of_date,
        &config.inner,
        &netting_set.inner,
    )
    .map_err(core_to_py)?;
    Ok(PyExposureProfile { inner: profile })
}

/// Compute unilateral CVA from an exposure profile.
///
/// Uses trapezoidal numerical integration for :math:`O(\\Delta t^2)` accuracy.
///
/// Args:
///     exposure_profile: :class:`ExposureProfile` from exposure simulation.
///     hazard_curve: Counterparty hazard curve for default probabilities.
///     discount_curve: Risk-free discount curve.
///     recovery_rate: Assumed recovery rate upon default (0 to 1).
///
/// Returns:
///     XvaResult: CVA value and exposure metrics.
///
/// Raises:
///     ValueError: If inputs are invalid.
///     FinstackError: On computation errors.
#[pyfunction]
fn compute_cva(
    exposure_profile: &PyExposureProfile,
    hazard_curve: &PyHazardCurve,
    discount_curve: &PyDiscountCurve,
    recovery_rate: f64,
) -> PyResult<PyXvaResult> {
    if !(0.0..=1.0).contains(&recovery_rate) {
        return Err(PyValueError::new_err(
            "recovery_rate must be between 0 and 1",
        ));
    }
    let result = xva::cva::compute_cva(
        &exposure_profile.inner,
        &hazard_curve.inner,
        &discount_curve.inner,
        recovery_rate,
    )
    .map_err(core_to_py)?;
    Ok(PyXvaResult { inner: result })
}

/// Compute debit valuation adjustment from the negative exposure profile.
#[pyfunction]
fn compute_dva(
    exposure_profile: &PyExposureProfile,
    own_hazard_curve: &PyHazardCurve,
    discount_curve: &PyDiscountCurve,
    own_recovery_rate: f64,
) -> PyResult<f64> {
    if !(0.0..=1.0).contains(&own_recovery_rate) {
        return Err(PyValueError::new_err(
            "own_recovery_rate must be between 0 and 1",
        ));
    }
    xva::cva::compute_dva(
        &exposure_profile.inner,
        &own_hazard_curve.inner,
        &discount_curve.inner,
        own_recovery_rate,
    )
    .map_err(core_to_py)
}

/// Compute funding valuation adjustment from the exposure profile.
#[pyfunction]
fn compute_fva(
    exposure_profile: &PyExposureProfile,
    discount_curve: &PyDiscountCurve,
    funding_spread_bps: f64,
    funding_benefit_bps: f64,
) -> PyResult<f64> {
    xva::cva::compute_fva(
        &exposure_profile.inner,
        &discount_curve.inner,
        funding_spread_bps,
        funding_benefit_bps,
    )
    .map_err(core_to_py)
}

/// Compute bilateral XVA including CVA, DVA, and optional FVA.
#[pyfunction]
fn compute_bilateral_xva(
    exposure_profile: &PyExposureProfile,
    counterparty_hazard_curve: &PyHazardCurve,
    own_hazard_curve: &PyHazardCurve,
    discount_curve: &PyDiscountCurve,
    counterparty_recovery_rate: f64,
    own_recovery_rate: f64,
    funding: Option<&PyFundingConfig>,
) -> PyResult<PyXvaResult> {
    let result = xva::cva::compute_bilateral_xva(
        &exposure_profile.inner,
        &counterparty_hazard_curve.inner,
        &own_hazard_curve.inner,
        &discount_curve.inner,
        counterparty_recovery_rate,
        own_recovery_rate,
        funding.map(|cfg| &cfg.inner),
    )
    .map_err(core_to_py)?;
    Ok(PyXvaResult { inner: result })
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "xva")?;
    module.setattr(
        "__doc__",
        "XVA (Valuation Adjustments) framework: CVA, exposure profiling, netting, and collateral.",
    )?;

    module.add_class::<PyFundingConfig>()?;
    module.add_class::<PyXvaConfig>()?;
    module.add_class::<PyCsaTerms>()?;
    module.add_class::<PyNettingSet>()?;
    module.add_class::<PyExposureProfile>()?;
    module.add_class::<PyXvaResult>()?;
    module.add_function(wrap_pyfunction!(apply_netting, &module)?)?;
    module.add_function(wrap_pyfunction!(apply_collateral, &module)?)?;
    module.add_function(wrap_pyfunction!(compute_exposure_profile, &module)?)?;
    module.add_function(wrap_pyfunction!(compute_cva, &module)?)?;
    module.add_function(wrap_pyfunction!(compute_dva, &module)?)?;
    module.add_function(wrap_pyfunction!(compute_fva, &module)?)?;
    module.add_function(wrap_pyfunction!(compute_bilateral_xva, &module)?)?;

    let exports = vec![
        "FundingConfig",
        "XvaConfig",
        "CsaTerms",
        "NettingSet",
        "ExposureProfile",
        "XvaResult",
        "apply_netting",
        "apply_collateral",
        "compute_exposure_profile",
        "compute_cva",
        "compute_dva",
        "compute_fva",
        "compute_bilateral_xva",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports)
}
