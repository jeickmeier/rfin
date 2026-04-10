use crate::core::currency::PyCurrency;
use finstack_margin::{MarginTenor, OtcMarginSpec, RepoMarginSpec, RepoMarginType};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;

use super::classification::{PyClearingStatus, PyRepoMarginType};
use super::csa::{PyCsaSpec, PyImMethodology, PyMarginTenor};
use super::helpers::{parse_currency, parse_margin_tenor};

/// OTC derivative margin specification (ISDA CSA compliant).
#[pyclass(
    name = "OtcMarginSpec",
    module = "finstack.valuations.margin",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyOtcMarginSpec {
    pub(crate) inner: OtcMarginSpec,
}

impl PyOtcMarginSpec {
    pub(crate) fn new(inner: OtcMarginSpec) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyOtcMarginSpec {
    #[staticmethod]
    fn bilateral_simm(csa: PyCsaSpec) -> Self {
        Self::new(OtcMarginSpec::bilateral_simm(csa.inner))
    }

    #[staticmethod]
    fn bilateral_schedule(csa: PyCsaSpec) -> Self {
        Self::new(OtcMarginSpec::bilateral_schedule(csa.inner))
    }

    #[staticmethod]
    fn cleared_spec(ccp: String, currency: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self::new(
            OtcMarginSpec::cleared(ccp, parse_currency(currency)?)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?,
        ))
    }

    #[staticmethod]
    fn usd_bilateral() -> PyResult<Self> {
        Ok(Self::new(OtcMarginSpec::usd_bilateral().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
        })?))
    }

    #[staticmethod]
    fn eur_bilateral() -> PyResult<Self> {
        Ok(Self::new(OtcMarginSpec::eur_bilateral().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
        })?))
    }

    #[staticmethod]
    fn lch_swapclear(currency: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self::new(
            OtcMarginSpec::lch_swapclear(parse_currency(currency)?)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?,
        ))
    }

    #[staticmethod]
    fn cme_cleared(currency: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self::new(
            OtcMarginSpec::cme_cleared(parse_currency(currency)?)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?,
        ))
    }

    #[staticmethod]
    fn ice_clear_credit() -> PyResult<Self> {
        Ok(Self::new(OtcMarginSpec::ice_clear_credit().map_err(
            |e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()),
        )?))
    }

    #[getter]
    fn csa(&self) -> PyCsaSpec {
        PyCsaSpec::new(self.inner.csa.clone())
    }

    #[getter]
    fn clearing_status(&self) -> PyClearingStatus {
        PyClearingStatus::new(self.inner.clearing_status.clone())
    }

    #[getter]
    fn im_methodology(&self) -> PyImMethodology {
        PyImMethodology::new(self.inner.im_methodology)
    }

    #[getter]
    fn vm_frequency(&self) -> PyMarginTenor {
        PyMarginTenor::new(self.inner.vm_frequency)
    }

    #[getter]
    fn settlement_lag(&self) -> u32 {
        self.inner.settlement_lag
    }

    fn is_cleared(&self) -> bool {
        self.inner.is_cleared()
    }

    fn is_bilateral(&self) -> bool {
        self.inner.is_bilateral()
    }

    fn ccp(&self) -> Option<String> {
        self.inner.ccp().map(|s| s.to_string())
    }

    fn base_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.base_currency())
    }

    fn __repr__(&self) -> String {
        format!(
            "OtcMarginSpec(clearing={}, im={}, vm_freq={})",
            self.inner.clearing_status, self.inner.im_methodology, self.inner.vm_frequency
        )
    }
}

/// GMRA 2011 compliant repo margin specification.
#[pyclass(
    name = "RepoMarginSpec",
    module = "finstack.valuations.margin",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyRepoMarginSpec {
    pub(crate) inner: RepoMarginSpec,
}

impl PyRepoMarginSpec {
    pub(crate) fn new(inner: RepoMarginSpec) -> Self {
        Self { inner }
    }
}

fn parse_repo_margin_type(v: &Bound<'_, PyAny>) -> PyResult<RepoMarginType> {
    if let Ok(py) = v.extract::<PyRef<PyRepoMarginType>>() {
        Ok(py.inner)
    } else if let Ok(s) = v.extract::<String>() {
        s.parse().map_err(pyo3::exceptions::PyValueError::new_err)
    } else {
        Err(PyTypeError::new_err("Expected RepoMarginType or string"))
    }
}

#[pymethods]
impl PyRepoMarginSpec {
    #[new]
    #[pyo3(
        signature = (margin_type, margin_ratio, margin_call_threshold, *, call_frequency=None, settlement_lag=1, pays_margin_interest=false, margin_interest_rate=None, substitution_allowed=false),
        text_signature = "(margin_type, margin_ratio, margin_call_threshold, *, call_frequency=None, settlement_lag=1, pays_margin_interest=False, margin_interest_rate=None, substitution_allowed=False)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn ctor(
        margin_type: &Bound<'_, PyAny>,
        margin_ratio: f64,
        margin_call_threshold: f64,
        call_frequency: Option<&Bound<'_, PyAny>>,
        settlement_lag: u32,
        pays_margin_interest: bool,
        margin_interest_rate: Option<f64>,
        substitution_allowed: bool,
    ) -> PyResult<Self> {
        let freq = if let Some(v) = call_frequency {
            parse_margin_tenor(v)?
        } else {
            MarginTenor::Daily
        };
        Ok(Self::new(RepoMarginSpec {
            margin_type: parse_repo_margin_type(margin_type)?,
            margin_ratio,
            margin_call_threshold,
            call_frequency: freq,
            settlement_lag,
            pays_margin_interest,
            margin_interest_rate,
            substitution_allowed,
            eligible_substitutes: None,
        }))
    }

    #[staticmethod]
    fn none() -> Self {
        Self::new(RepoMarginSpec::none())
    }

    #[staticmethod]
    fn mark_to_market(margin_ratio: f64, threshold: f64) -> PyResult<Self> {
        Ok(Self::new(
            RepoMarginSpec::mark_to_market(margin_ratio, threshold)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?,
        ))
    }

    #[staticmethod]
    fn triparty(margin_ratio: f64) -> PyResult<Self> {
        Ok(Self::new(RepoMarginSpec::triparty(margin_ratio).map_err(
            |e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()),
        )?))
    }

    #[getter]
    fn margin_type(&self) -> PyRepoMarginType {
        PyRepoMarginType::new(self.inner.margin_type)
    }

    #[getter]
    fn margin_ratio(&self) -> f64 {
        self.inner.margin_ratio
    }

    #[getter]
    fn margin_call_threshold(&self) -> f64 {
        self.inner.margin_call_threshold
    }

    #[getter]
    fn call_frequency(&self) -> PyMarginTenor {
        PyMarginTenor::new(self.inner.call_frequency)
    }

    #[getter]
    fn settlement_lag(&self) -> u32 {
        self.inner.settlement_lag
    }

    #[getter]
    fn pays_margin_interest(&self) -> bool {
        self.inner.pays_margin_interest
    }

    #[getter]
    fn margin_interest_rate(&self) -> Option<f64> {
        self.inner.margin_interest_rate
    }

    #[getter]
    fn substitution_allowed(&self) -> bool {
        self.inner.substitution_allowed
    }

    fn has_margining(&self) -> bool {
        self.inner.has_margining()
    }

    fn required_collateral(&self, cash_amount: f64) -> f64 {
        self.inner.required_collateral(cash_amount)
    }

    fn call_trigger_value(&self, cash_amount: f64) -> f64 {
        self.inner.call_trigger_value(cash_amount)
    }

    fn requires_margin_call(&self, cash_amount: f64, current_collateral: f64) -> bool {
        self.inner
            .requires_margin_call(cash_amount, current_collateral)
    }

    fn margin_deficit(&self, cash_amount: f64, current_collateral: f64) -> f64 {
        self.inner.margin_deficit(cash_amount, current_collateral)
    }

    fn excess_collateral(&self, cash_amount: f64, current_collateral: f64) -> f64 {
        self.inner
            .excess_collateral(cash_amount, current_collateral)
    }

    fn __repr__(&self) -> String {
        format!(
            "RepoMarginSpec(type={}, ratio={}, threshold={})",
            self.inner.margin_type, self.inner.margin_ratio, self.inner.margin_call_threshold
        )
    }
}
