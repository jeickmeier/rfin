//! Margin type bindings.
//!
//! This module exposes the core margin types from `finstack_valuations::margin` as thin
//! Python wrappers. No Python-side margin logic is implemented here.

use crate::core::currency::PyCurrency;
use crate::core::money::PyMoney;
use finstack_core::currency::Currency;
use finstack_core::types::CurveId;
use finstack_valuations::margin::{
    CsaSpec, EligibleCollateralSchedule, ImMethodology, ImParameters, MarginCallTiming,
    MarginTenor, VmParameters,
};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};
use pyo3::Bound;

fn parse_currency(ccy: &Bound<'_, PyAny>) -> PyResult<Currency> {
    if let Ok(py_ccy) = ccy.extract::<PyRef<PyCurrency>>() {
        Ok(py_ccy.inner)
    } else if let Ok(s) = ccy.extract::<String>() {
        s.parse().map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid currency: {}", e))
        })
    } else {
        Err(PyTypeError::new_err("Expected Currency or string"))
    }
}

fn parse_margin_tenor(v: &Bound<'_, PyAny>) -> PyResult<MarginTenor> {
    if let Ok(py) = v.extract::<PyRef<PyMarginTenor>>() {
        Ok(py.inner)
    } else if let Ok(s) = v.extract::<String>() {
        s.parse()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e))
    } else {
        Err(PyTypeError::new_err("Expected MarginTenor or string"))
    }
}

fn parse_im_methodology(v: &Bound<'_, PyAny>) -> PyResult<ImMethodology> {
    if let Ok(py) = v.extract::<PyRef<PyImMethodology>>() {
        Ok(py.inner)
    } else if let Ok(s) = v.extract::<String>() {
        s.parse()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e))
    } else {
        Err(PyTypeError::new_err("Expected ImMethodology or string"))
    }
}

/// Margin call frequency (tenor).
#[pyclass(name = "MarginTenor", module = "finstack.valuations.margin", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyMarginTenor {
    pub(crate) inner: MarginTenor,
}

impl PyMarginTenor {
    pub(crate) const fn new(inner: MarginTenor) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMarginTenor {
    #[classattr]
    const DAILY: Self = Self::new(MarginTenor::Daily);
    #[classattr]
    const WEEKLY: Self = Self::new(MarginTenor::Weekly);
    #[classattr]
    const MONTHLY: Self = Self::new(MarginTenor::Monthly);
    #[classattr]
    const ON_DEMAND: Self = Self::new(MarginTenor::OnDemand);

    #[getter]
    fn name(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("MarginTenor.{}", self.name().to_ascii_uppercase())
    }

    fn __str__(&self) -> String {
        self.name()
    }
}

/// Initial margin methodology.
#[pyclass(name = "ImMethodology", module = "finstack.valuations.margin", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyImMethodology {
    pub(crate) inner: ImMethodology,
}

impl PyImMethodology {
    pub(crate) const fn new(inner: ImMethodology) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyImMethodology {
    #[classattr]
    const HAIRCUT: Self = Self::new(ImMethodology::Haircut);
    #[classattr]
    const SIMM: Self = Self::new(ImMethodology::Simm);
    #[classattr]
    const SCHEDULE: Self = Self::new(ImMethodology::Schedule);
    #[classattr]
    const INTERNAL_MODEL: Self = Self::new(ImMethodology::InternalModel);
    #[classattr]
    const CLEARING_HOUSE: Self = Self::new(ImMethodology::ClearingHouse);

    #[getter]
    fn name(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("ImMethodology.{}", self.name().to_ascii_uppercase())
    }

    fn __str__(&self) -> String {
        self.name()
    }
}

/// Operational timing parameters for margin calls.
#[pyclass(name = "MarginCallTiming", module = "finstack.valuations.margin")]
#[derive(Clone, Debug)]
pub struct PyMarginCallTiming {
    pub(crate) inner: MarginCallTiming,
}

impl PyMarginCallTiming {
    pub(crate) fn new(inner: MarginCallTiming) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMarginCallTiming {
    #[new]
    #[pyo3(
        signature = (*, notification_deadline_hours=None, response_deadline_hours=None, dispute_resolution_days=None, delivery_grace_days=None),
        text_signature = "(*, notification_deadline_hours=None, response_deadline_hours=None, dispute_resolution_days=None, delivery_grace_days=None)"
    )]
    fn ctor(
        notification_deadline_hours: Option<u8>,
        response_deadline_hours: Option<u8>,
        dispute_resolution_days: Option<u8>,
        delivery_grace_days: Option<u8>,
    ) -> Self {
        let mut timing = MarginCallTiming::default();
        if let Some(v) = notification_deadline_hours {
            timing.notification_deadline_hours = v;
        }
        if let Some(v) = response_deadline_hours {
            timing.response_deadline_hours = v;
        }
        if let Some(v) = dispute_resolution_days {
            timing.dispute_resolution_days = v;
        }
        if let Some(v) = delivery_grace_days {
            timing.delivery_grace_days = v;
        }
        Self::new(timing)
    }

    #[staticmethod]
    fn regulatory_standard() -> Self {
        Self::new(MarginCallTiming::regulatory_standard())
    }

    #[getter]
    fn notification_deadline_hours(&self) -> u8 {
        self.inner.notification_deadline_hours
    }

    #[getter]
    fn response_deadline_hours(&self) -> u8 {
        self.inner.response_deadline_hours
    }

    #[getter]
    fn dispute_resolution_days(&self) -> u8 {
        self.inner.dispute_resolution_days
    }

    #[getter]
    fn delivery_grace_days(&self) -> u8 {
        self.inner.delivery_grace_days
    }

    fn __repr__(&self) -> String {
        format!(
            "MarginCallTiming(notification_deadline_hours={}, response_deadline_hours={}, dispute_resolution_days={}, delivery_grace_days={})",
            self.inner.notification_deadline_hours,
            self.inner.response_deadline_hours,
            self.inner.dispute_resolution_days,
            self.inner.delivery_grace_days
        )
    }
}

/// Variation margin parameters (threshold, MTA, frequency, etc.).
#[pyclass(name = "VmParameters", module = "finstack.valuations.margin")]
#[derive(Clone, Debug)]
pub struct PyVmParameters {
    pub(crate) inner: VmParameters,
}

impl PyVmParameters {
    pub(crate) fn new(inner: VmParameters) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyVmParameters {
    #[new]
    #[pyo3(
        signature = (threshold, mta, *, rounding=None, independent_amount=None, frequency=None, settlement_lag=None),
        text_signature = "(threshold, mta, *, rounding=None, independent_amount=None, frequency=None, settlement_lag=None)"
    )]
    fn ctor(
        threshold: PyMoney,
        mta: PyMoney,
        rounding: Option<PyMoney>,
        independent_amount: Option<PyMoney>,
        frequency: Option<&Bound<'_, PyAny>>,
        settlement_lag: Option<u32>,
    ) -> PyResult<Self> {
        let freq = if let Some(v) = frequency {
            parse_margin_tenor(v)?
        } else {
            MarginTenor::Daily
        };
        Ok(Self::new(VmParameters {
            threshold: threshold.inner,
            mta: mta.inner,
            rounding: rounding.map(|m| m.inner).unwrap_or_else(|| {
                let ccy = threshold.inner.currency();
                finstack_core::money::Money::new(10_000.0, ccy)
            }),
            independent_amount: independent_amount.map(|m| m.inner).unwrap_or_else(|| {
                finstack_core::money::Money::new(0.0, threshold.inner.currency())
            }),
            frequency: freq,
            settlement_lag: settlement_lag.unwrap_or(1),
        }))
    }

    #[staticmethod]
    fn regulatory_standard(currency: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self::new(VmParameters::regulatory_standard(
            parse_currency(currency)?,
        )))
    }

    #[staticmethod]
    fn with_threshold(threshold: PyMoney, mta: PyMoney) -> Self {
        Self::new(VmParameters::with_threshold(threshold.inner, mta.inner))
    }

    #[getter]
    fn threshold(&self) -> PyMoney {
        PyMoney::new(self.inner.threshold)
    }

    #[getter]
    fn mta(&self) -> PyMoney {
        PyMoney::new(self.inner.mta)
    }

    #[getter]
    fn rounding(&self) -> PyMoney {
        PyMoney::new(self.inner.rounding)
    }

    #[getter]
    fn independent_amount(&self) -> PyMoney {
        PyMoney::new(self.inner.independent_amount)
    }

    #[getter]
    fn frequency(&self) -> PyMarginTenor {
        PyMarginTenor::new(self.inner.frequency)
    }

    #[getter]
    fn settlement_lag(&self) -> u32 {
        self.inner.settlement_lag
    }

    fn __repr__(&self) -> String {
        format!(
            "VmParameters(threshold={}, mta={}, frequency={}, settlement_lag={})",
            self.inner.threshold, self.inner.mta, self.inner.frequency, self.inner.settlement_lag
        )
    }
}

/// Initial margin parameters (methodology, MPOR, threshold, etc.).
#[pyclass(name = "ImParameters", module = "finstack.valuations.margin")]
#[derive(Clone, Debug)]
pub struct PyImParameters {
    pub(crate) inner: ImParameters,
}

impl PyImParameters {
    pub(crate) fn new(inner: ImParameters) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyImParameters {
    #[new]
    #[pyo3(
        signature = (methodology, mpor_days, threshold, mta, *, segregated=true),
        text_signature = "(methodology, mpor_days, threshold, mta, *, segregated=True)"
    )]
    fn ctor(
        methodology: &Bound<'_, PyAny>,
        mpor_days: u32,
        threshold: PyMoney,
        mta: PyMoney,
        segregated: bool,
    ) -> PyResult<Self> {
        Ok(Self::new(ImParameters {
            methodology: parse_im_methodology(methodology)?,
            mpor_days,
            threshold: threshold.inner,
            mta: mta.inner,
            segregated,
        }))
    }

    #[staticmethod]
    fn simm_standard(currency: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self::new(ImParameters::simm_standard(parse_currency(
            currency,
        )?)))
    }

    #[staticmethod]
    fn schedule_based(currency: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self::new(ImParameters::schedule_based(parse_currency(
            currency,
        )?)))
    }

    #[staticmethod]
    fn cleared(currency: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self::new(ImParameters::cleared(parse_currency(currency)?)))
    }

    #[staticmethod]
    fn repo_haircut(currency: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self::new(ImParameters::repo_haircut(parse_currency(
            currency,
        )?)))
    }

    #[getter]
    fn methodology(&self) -> PyImMethodology {
        PyImMethodology::new(self.inner.methodology)
    }

    #[getter]
    fn mpor_days(&self) -> u32 {
        self.inner.mpor_days
    }

    #[getter]
    fn threshold(&self) -> PyMoney {
        PyMoney::new(self.inner.threshold)
    }

    #[getter]
    fn mta(&self) -> PyMoney {
        PyMoney::new(self.inner.mta)
    }

    #[getter]
    fn segregated(&self) -> bool {
        self.inner.segregated
    }

    fn __repr__(&self) -> String {
        format!(
            "ImParameters(methodology={}, mpor_days={}, segregated={})",
            self.inner.methodology, self.inner.mpor_days, self.inner.segregated
        )
    }
}

/// Eligible collateral schedule.
#[pyclass(
    name = "EligibleCollateralSchedule",
    module = "finstack.valuations.margin"
)]
#[derive(Clone, Debug)]
pub struct PyEligibleCollateralSchedule {
    pub(crate) inner: EligibleCollateralSchedule,
}

impl PyEligibleCollateralSchedule {
    pub(crate) fn new(inner: EligibleCollateralSchedule) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyEligibleCollateralSchedule {
    #[new]
    fn ctor() -> Self {
        Self::new(EligibleCollateralSchedule::default())
    }

    #[staticmethod]
    fn cash_only() -> Self {
        Self::new(EligibleCollateralSchedule::cash_only())
    }

    #[staticmethod]
    fn bcbs_standard() -> Self {
        Self::new(EligibleCollateralSchedule::bcbs_standard())
    }

    #[staticmethod]
    fn us_treasuries() -> Self {
        Self::new(EligibleCollateralSchedule::us_treasuries())
    }

    #[getter]
    fn default_haircut(&self) -> Option<f64> {
        self.inner.default_haircut
    }

    #[getter]
    fn rehypothecation_allowed(&self) -> bool {
        self.inner.rehypothecation_allowed
    }

    #[getter]
    fn eligible(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let list = PyList::empty(py);
        for entry in &self.inner.eligible {
            let d = PyDict::new(py);
            d.set_item("asset_class", entry.asset_class.as_str())?;
            d.set_item("min_rating", entry.min_rating.clone())?;
            if let Some(mc) = &entry.maturity_constraints {
                let mc_d = PyDict::new(py);
                mc_d.set_item("min_remaining_years", mc.min_remaining_years)?;
                mc_d.set_item("max_remaining_years", mc.max_remaining_years)?;
                d.set_item("maturity_constraints", mc_d)?;
            } else {
                d.set_item("maturity_constraints", py.None())?;
            }
            d.set_item("haircut", entry.haircut)?;
            d.set_item("fx_haircut_addon", entry.fx_haircut_addon)?;
            d.set_item("concentration_limit", entry.concentration_limit)?;
            list.append(d)?;
        }
        Ok(list.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "EligibleCollateralSchedule(eligible={}, rehypothecation_allowed={})",
            self.inner.eligible.len(),
            self.inner.rehypothecation_allowed
        )
    }
}

/// Credit Support Annex (CSA) specification.
#[pyclass(name = "CsaSpec", module = "finstack.valuations.margin")]
#[derive(Clone, Debug)]
pub struct PyCsaSpec {
    pub(crate) inner: CsaSpec,
}

impl PyCsaSpec {
    pub(crate) fn new(inner: CsaSpec) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCsaSpec {
    #[new]
    #[pyo3(
        signature = (id, base_currency, vm_params, *, im_params=None, eligible_collateral=None, call_timing=None, collateral_curve_id),
        text_signature = "(id, base_currency, vm_params, *, im_params=None, eligible_collateral=None, call_timing=None, collateral_curve_id)"
    )]
    fn ctor(
        id: String,
        base_currency: &Bound<'_, PyAny>,
        vm_params: PyVmParameters,
        im_params: Option<PyImParameters>,
        eligible_collateral: Option<PyEligibleCollateralSchedule>,
        call_timing: Option<PyMarginCallTiming>,
        collateral_curve_id: String,
    ) -> PyResult<Self> {
        Ok(Self::new(CsaSpec {
            id,
            base_currency: parse_currency(base_currency)?,
            vm_params: vm_params.inner,
            im_params: im_params.map(|p| p.inner),
            eligible_collateral: eligible_collateral
                .map(|s| s.inner)
                .unwrap_or_else(EligibleCollateralSchedule::bcbs_standard),
            call_timing: call_timing
                .map(|t| t.inner)
                .unwrap_or_else(MarginCallTiming::regulatory_standard),
            collateral_curve_id: CurveId::new(&collateral_curve_id),
        }))
    }

    #[staticmethod]
    fn usd_regulatory() -> Self {
        Self::new(CsaSpec::usd_regulatory())
    }

    #[staticmethod]
    fn eur_regulatory() -> Self {
        Self::new(CsaSpec::eur_regulatory())
    }

    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    fn base_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.base_currency)
    }

    #[getter]
    fn vm_params(&self) -> PyVmParameters {
        PyVmParameters::new(self.inner.vm_params.clone())
    }

    #[getter]
    fn im_params(&self) -> Option<PyImParameters> {
        self.inner.im_params.clone().map(PyImParameters::new)
    }

    #[getter]
    fn eligible_collateral(&self) -> PyEligibleCollateralSchedule {
        PyEligibleCollateralSchedule::new(self.inner.eligible_collateral.clone())
    }

    #[getter]
    fn call_timing(&self) -> PyMarginCallTiming {
        PyMarginCallTiming::new(self.inner.call_timing.clone())
    }

    #[getter]
    fn collateral_curve_id(&self) -> String {
        self.inner.collateral_curve_id.as_str().to_string()
    }

    fn requires_im(&self) -> bool {
        self.inner.requires_im()
    }

    fn vm_threshold(&self) -> PyMoney {
        PyMoney::new(*self.inner.vm_threshold())
    }

    fn im_threshold(&self) -> Option<PyMoney> {
        self.inner.im_threshold().copied().map(PyMoney::new)
    }

    fn __repr__(&self) -> String {
        format!(
            "CsaSpec(id={}, base_currency={}, requires_im={})",
            self.inner.id,
            self.inner.base_currency,
            self.inner.requires_im()
        )
    }
}

/// Register margin type exports.
pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "margin")?;
    module.setattr(
        "__doc__",
        "Margin and collateral management types (CSA specs, VM/IM parameters, collateral schedules).",
    )?;

    module.add_class::<PyMarginTenor>()?;
    module.add_class::<PyImMethodology>()?;
    module.add_class::<PyMarginCallTiming>()?;
    module.add_class::<PyVmParameters>()?;
    module.add_class::<PyImParameters>()?;
    module.add_class::<PyEligibleCollateralSchedule>()?;
    module.add_class::<PyCsaSpec>()?;

    let exports = [
        "MarginTenor",
        "ImMethodology",
        "MarginCallTiming",
        "VmParameters",
        "ImParameters",
        "EligibleCollateralSchedule",
        "CsaSpec",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    parent.setattr("margin", &module)?;
    Ok(exports.to_vec())
}
