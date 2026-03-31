//! Margin type bindings.
//!
//! This module exposes the core margin types from `finstack-margin` as thin
//! Python wrappers. No Python-side margin logic is implemented here.

use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::PyMoney;
use finstack_core::currency::Currency;
use finstack_core::types::CurveId;
use finstack_margin::calculators::im::simm::SimmVersion;
use finstack_margin::{
    ClearingStatus, CollateralAssetClass, CsaSpec, EligibleCollateralSchedule, ImMethodology,
    ImParameters, ImResult, InstrumentMarginResult, MarginCall, MarginCallTiming, MarginCallType,
    MarginTenor, OtcMarginSpec, RepoMarginSpec, RepoMarginType, SimmCalculator, SimmRiskClass,
    SimmSensitivities, VmCalculator, VmParameters, VmResult,
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
        s.parse().map_err(pyo3::exceptions::PyValueError::new_err)
    } else {
        Err(PyTypeError::new_err("Expected MarginTenor or string"))
    }
}

fn parse_im_methodology(v: &Bound<'_, PyAny>) -> PyResult<ImMethodology> {
    if let Ok(py) = v.extract::<PyRef<PyImMethodology>>() {
        Ok(py.inner)
    } else if let Ok(s) = v.extract::<String>() {
        s.parse().map_err(pyo3::exceptions::PyValueError::new_err)
    } else {
        Err(PyTypeError::new_err("Expected ImMethodology or string"))
    }
}

/// Margin call frequency (tenor).
#[pyclass(
    name = "MarginTenor",
    module = "finstack.valuations.margin",
    frozen,
    from_py_object
)]
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
#[pyclass(
    name = "ImMethodology",
    module = "finstack.valuations.margin",
    frozen,
    from_py_object
)]
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
#[pyclass(
    name = "MarginCallTiming",
    module = "finstack.valuations.margin",
    from_py_object
)]
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
    fn regulatory_standard() -> PyResult<Self> {
        Ok(Self::new(MarginCallTiming::regulatory_standard().map_err(
            |e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()),
        )?))
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
#[pyclass(
    name = "VmParameters",
    module = "finstack.valuations.margin",
    from_py_object
)]
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
        Ok(Self::new(
            VmParameters::regulatory_standard(parse_currency(currency)?)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?,
        ))
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
#[pyclass(
    name = "ImParameters",
    module = "finstack.valuations.margin",
    from_py_object
)]
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
        Ok(Self::new(
            ImParameters::simm_standard(parse_currency(currency)?)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?,
        ))
    }

    #[staticmethod]
    fn schedule_based(currency: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self::new(
            ImParameters::schedule_based(parse_currency(currency)?)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?,
        ))
    }

    #[staticmethod]
    fn cleared(currency: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self::new(
            ImParameters::cleared(parse_currency(currency)?)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?,
        ))
    }

    #[staticmethod]
    fn repo_haircut(currency: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self::new(
            ImParameters::repo_haircut(parse_currency(currency)?)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?,
        ))
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
    module = "finstack.valuations.margin",
    from_py_object
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
    fn cash_only() -> PyResult<Self> {
        Ok(Self::new(EligibleCollateralSchedule::cash_only().map_err(
            |e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()),
        )?))
    }

    #[staticmethod]
    fn bcbs_standard() -> PyResult<Self> {
        Ok(Self::new(
            EligibleCollateralSchedule::bcbs_standard()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?,
        ))
    }

    #[staticmethod]
    fn us_treasuries() -> PyResult<Self> {
        Ok(Self::new(
            EligibleCollateralSchedule::us_treasuries()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?,
        ))
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
#[pyclass(
    name = "CsaSpec",
    module = "finstack.valuations.margin",
    from_py_object
)]
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
        let collateral = match eligible_collateral {
            Some(s) => s.inner,
            None => EligibleCollateralSchedule::bcbs_standard()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?,
        };
        let timing = match call_timing {
            Some(t) => t.inner,
            None => MarginCallTiming::regulatory_standard()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?,
        };
        Ok(Self::new(CsaSpec {
            id,
            base_currency: parse_currency(base_currency)?,
            vm_params: vm_params.inner,
            im_params: im_params.map(|p| p.inner),
            eligible_collateral: collateral,
            call_timing: timing,
            collateral_curve_id: CurveId::new(&collateral_curve_id),
        }))
    }

    #[staticmethod]
    fn usd_regulatory() -> PyResult<Self> {
        Ok(Self::new(CsaSpec::usd_regulatory().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
        })?))
    }

    #[staticmethod]
    fn eur_regulatory() -> PyResult<Self> {
        Ok(Self::new(CsaSpec::eur_regulatory().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
        })?))
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

// ---------------------------------------------------------------------------
// MarginCallType
// ---------------------------------------------------------------------------

/// Type of margin call (IM delivery, VM delivery, VM return, top-up, substitution).
#[pyclass(
    name = "MarginCallType",
    module = "finstack.valuations.margin",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyMarginCallType {
    pub(crate) inner: MarginCallType,
}

impl PyMarginCallType {
    pub(crate) const fn new(inner: MarginCallType) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMarginCallType {
    #[classattr]
    const INITIAL_MARGIN: Self = Self::new(MarginCallType::InitialMargin);
    #[classattr]
    const VARIATION_MARGIN_DELIVERY: Self = Self::new(MarginCallType::VariationMarginDelivery);
    #[classattr]
    const VARIATION_MARGIN_RETURN: Self = Self::new(MarginCallType::VariationMarginReturn);
    #[classattr]
    const TOP_UP: Self = Self::new(MarginCallType::TopUp);
    #[classattr]
    const SUBSTITUTION: Self = Self::new(MarginCallType::Substitution);

    #[getter]
    fn name(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("MarginCallType.{}", self.name().to_ascii_uppercase())
    }

    fn __str__(&self) -> String {
        self.name()
    }
}

// ---------------------------------------------------------------------------
// CollateralAssetClass
// ---------------------------------------------------------------------------

/// BCBS-IOSCO collateral asset class.
#[pyclass(
    name = "CollateralAssetClass",
    module = "finstack.valuations.margin",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PyCollateralAssetClass {
    pub(crate) inner: CollateralAssetClass,
}

impl PyCollateralAssetClass {
    pub(crate) fn new(inner: CollateralAssetClass) -> Self {
        Self { inner }
    }
}

#[pymethods]
#[allow(non_snake_case)]
impl PyCollateralAssetClass {
    #[classattr]
    fn CASH() -> Self {
        Self::new(CollateralAssetClass::Cash)
    }
    #[classattr]
    fn GOVERNMENT_BONDS() -> Self {
        Self::new(CollateralAssetClass::GovernmentBonds)
    }
    #[classattr]
    fn AGENCY_BONDS() -> Self {
        Self::new(CollateralAssetClass::AgencyBonds)
    }
    #[classattr]
    fn COVERED_BONDS() -> Self {
        Self::new(CollateralAssetClass::CoveredBonds)
    }
    #[classattr]
    fn CORPORATE_BONDS() -> Self {
        Self::new(CollateralAssetClass::CorporateBonds)
    }
    #[classattr]
    fn EQUITY() -> Self {
        Self::new(CollateralAssetClass::Equity)
    }
    #[classattr]
    fn GOLD() -> Self {
        Self::new(CollateralAssetClass::Gold)
    }
    #[classattr]
    fn MUTUAL_FUNDS() -> Self {
        Self::new(CollateralAssetClass::MutualFunds)
    }

    #[staticmethod]
    fn custom(name: String) -> Self {
        Self::new(CollateralAssetClass::Custom(name))
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.as_str().to_string()
    }

    fn standard_haircut(&self) -> PyResult<f64> {
        self.inner
            .standard_haircut()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("CollateralAssetClass({})", self.inner.as_str())
    }

    fn __str__(&self) -> String {
        self.inner.as_str().to_string()
    }
}

// ---------------------------------------------------------------------------
// ClearingStatus
// ---------------------------------------------------------------------------

/// Clearing status: bilateral or cleared through a CCP.
#[pyclass(
    name = "ClearingStatus",
    module = "finstack.valuations.margin",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PyClearingStatus {
    pub(crate) inner: ClearingStatus,
}

impl PyClearingStatus {
    pub(crate) fn new(inner: ClearingStatus) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyClearingStatus {
    #[staticmethod]
    fn bilateral() -> Self {
        Self::new(ClearingStatus::Bilateral)
    }

    #[staticmethod]
    fn cleared(ccp: String) -> Self {
        Self::new(ClearingStatus::Cleared { ccp })
    }

    #[getter]
    fn is_bilateral(&self) -> bool {
        matches!(self.inner, ClearingStatus::Bilateral)
    }

    #[getter]
    fn is_cleared(&self) -> bool {
        matches!(self.inner, ClearingStatus::Cleared { .. })
    }

    #[getter]
    fn ccp(&self) -> Option<String> {
        match &self.inner {
            ClearingStatus::Cleared { ccp } => Some(ccp.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("ClearingStatus({})", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

// ---------------------------------------------------------------------------
// SimmRiskClass
// ---------------------------------------------------------------------------

/// ISDA SIMM risk class.
#[pyclass(
    name = "SimmRiskClass",
    module = "finstack.valuations.margin",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PySimmRiskClass {
    pub(crate) inner: SimmRiskClass,
}

impl PySimmRiskClass {
    pub(crate) const fn new(inner: SimmRiskClass) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySimmRiskClass {
    #[classattr]
    const INTEREST_RATE: Self = Self::new(SimmRiskClass::InterestRate);
    #[classattr]
    const CREDIT_QUALIFYING: Self = Self::new(SimmRiskClass::CreditQualifying);
    #[classattr]
    const CREDIT_NON_QUALIFYING: Self = Self::new(SimmRiskClass::CreditNonQualifying);
    #[classattr]
    const EQUITY: Self = Self::new(SimmRiskClass::Equity);
    #[classattr]
    const COMMODITY: Self = Self::new(SimmRiskClass::Commodity);
    #[classattr]
    const FX: Self = Self::new(SimmRiskClass::Fx);

    #[getter]
    fn name(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("SimmRiskClass.{}", self.name().to_ascii_uppercase())
    }

    fn __str__(&self) -> String {
        self.name()
    }
}

// ---------------------------------------------------------------------------
// RepoMarginType
// ---------------------------------------------------------------------------

/// Repo margin mechanism type.
#[pyclass(
    name = "RepoMarginType",
    module = "finstack.valuations.margin",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyRepoMarginType {
    pub(crate) inner: RepoMarginType,
}

impl PyRepoMarginType {
    pub(crate) const fn new(inner: RepoMarginType) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRepoMarginType {
    #[classattr]
    const NONE: Self = Self::new(RepoMarginType::None);
    #[classattr]
    const MARK_TO_MARKET: Self = Self::new(RepoMarginType::MarkToMarket);
    #[classattr]
    const NET_EXPOSURE: Self = Self::new(RepoMarginType::NetExposure);
    #[classattr]
    const TRIPARTY: Self = Self::new(RepoMarginType::Triparty);

    #[getter]
    fn name(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("RepoMarginType.{}", self.name().to_ascii_uppercase())
    }

    fn __str__(&self) -> String {
        self.name()
    }
}

// ---------------------------------------------------------------------------
// MarginCall
// ---------------------------------------------------------------------------

/// A margin call event with all relevant details.
#[pyclass(
    name = "MarginCall",
    module = "finstack.valuations.margin",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyMarginCall {
    pub(crate) inner: MarginCall,
}

impl PyMarginCall {
    pub(crate) fn new(inner: MarginCall) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMarginCall {
    #[getter]
    fn call_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.call_date)
    }

    #[getter]
    fn settlement_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.settlement_date)
    }

    #[getter]
    fn call_type(&self) -> PyMarginCallType {
        PyMarginCallType::new(self.inner.call_type)
    }

    #[getter]
    fn amount(&self) -> PyMoney {
        PyMoney::new(self.inner.amount)
    }

    #[getter]
    fn collateral_type(&self) -> Option<PyCollateralAssetClass> {
        self.inner
            .collateral_type
            .clone()
            .map(PyCollateralAssetClass::new)
    }

    #[getter]
    fn mtm_trigger(&self) -> PyMoney {
        PyMoney::new(self.inner.mtm_trigger)
    }

    #[getter]
    fn threshold(&self) -> PyMoney {
        PyMoney::new(self.inner.threshold)
    }

    #[getter]
    fn mta_applied(&self) -> PyMoney {
        PyMoney::new(self.inner.mta_applied)
    }

    fn is_delivery(&self) -> bool {
        self.inner.is_delivery()
    }

    fn is_return(&self) -> bool {
        self.inner.is_return()
    }

    fn days_to_settle(&self) -> i64 {
        self.inner.days_to_settle()
    }

    fn __repr__(&self) -> String {
        format!(
            "MarginCall(type={}, amount={}, call_date={})",
            self.inner.call_type, self.inner.amount, self.inner.call_date
        )
    }
}

// ---------------------------------------------------------------------------
// VmResult
// ---------------------------------------------------------------------------

/// Variation margin calculation result.
#[pyclass(
    name = "VmResult",
    module = "finstack.valuations.margin",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyVmResult {
    pub(crate) inner: VmResult,
}

impl PyVmResult {
    pub(crate) fn new(inner: VmResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyVmResult {
    #[getter]
    fn date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.date)
    }

    #[getter]
    fn gross_exposure(&self) -> PyMoney {
        PyMoney::new(self.inner.gross_exposure)
    }

    #[getter]
    fn net_exposure(&self) -> PyMoney {
        PyMoney::new(self.inner.net_exposure)
    }

    #[getter]
    fn delivery_amount(&self) -> PyMoney {
        PyMoney::new(self.inner.delivery_amount)
    }

    #[getter]
    fn return_amount(&self) -> PyMoney {
        PyMoney::new(self.inner.return_amount)
    }

    #[getter]
    fn settlement_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.settlement_date)
    }

    fn net_margin(&self) -> PyMoney {
        PyMoney::new(self.inner.net_margin())
    }

    fn requires_call(&self) -> bool {
        self.inner.requires_call()
    }

    fn __repr__(&self) -> String {
        format!(
            "VmResult(date={}, delivery={}, return={})",
            self.inner.date, self.inner.delivery_amount, self.inner.return_amount
        )
    }
}

// ---------------------------------------------------------------------------
// ImResult
// ---------------------------------------------------------------------------

/// Initial margin calculation result.
#[pyclass(
    name = "ImResult",
    module = "finstack.valuations.margin",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyImResult {
    pub(crate) inner: ImResult,
}

impl PyImResult {
    #[allow(dead_code)]
    pub(crate) fn new(inner: ImResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyImResult {
    #[getter]
    fn amount(&self) -> PyMoney {
        PyMoney::new(self.inner.amount)
    }

    #[getter]
    fn methodology(&self) -> PyImMethodology {
        PyImMethodology::new(self.inner.methodology)
    }

    #[getter]
    fn as_of(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.as_of)
    }

    #[getter]
    fn mpor_days(&self) -> u32 {
        self.inner.mpor_days
    }

    #[getter]
    fn breakdown(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let d = PyDict::new(py);
        for (key, &value) in &self.inner.breakdown {
            d.set_item(key, PyMoney::new(value).into_pyobject(py)?)?;
        }
        Ok(d.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "ImResult(amount={}, methodology={}, mpor_days={})",
            self.inner.amount, self.inner.methodology, self.inner.mpor_days
        )
    }
}

// ---------------------------------------------------------------------------
// InstrumentMarginResult
// ---------------------------------------------------------------------------

/// Per-instrument margin calculation result.
#[pyclass(
    name = "InstrumentMarginResult",
    module = "finstack.valuations.margin",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyInstrumentMarginResult {
    pub(crate) inner: InstrumentMarginResult,
}

impl PyInstrumentMarginResult {
    #[allow(dead_code)]
    pub(crate) fn new(inner: InstrumentMarginResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyInstrumentMarginResult {
    #[getter]
    fn instrument_id(&self) -> String {
        self.inner.instrument_id.clone()
    }

    #[getter]
    fn as_of(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.as_of)
    }

    #[getter]
    fn initial_margin(&self) -> PyMoney {
        PyMoney::new(self.inner.initial_margin)
    }

    #[getter]
    fn variation_margin(&self) -> PyMoney {
        PyMoney::new(self.inner.variation_margin)
    }

    #[getter]
    fn total_margin(&self) -> PyMoney {
        PyMoney::new(self.inner.total_margin)
    }

    #[getter]
    fn im_methodology(&self) -> PyImMethodology {
        PyImMethodology::new(self.inner.im_methodology)
    }

    #[getter]
    fn is_cleared(&self) -> bool {
        self.inner.is_cleared
    }

    fn __repr__(&self) -> String {
        format!(
            "InstrumentMarginResult(id={}, total={})",
            self.inner.instrument_id, self.inner.total_margin
        )
    }
}

// ---------------------------------------------------------------------------
// SimmSensitivities
// ---------------------------------------------------------------------------

/// SIMM sensitivity inputs organized by risk class.
#[pyclass(
    name = "SimmSensitivities",
    module = "finstack.valuations.margin",
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PySimmSensitivities {
    pub(crate) inner: SimmSensitivities,
}

impl PySimmSensitivities {
    pub(crate) fn new(inner: SimmSensitivities) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySimmSensitivities {
    #[new]
    fn ctor(base_currency: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self::new(SimmSensitivities::new(parse_currency(
            base_currency,
        )?)))
    }

    #[getter]
    fn base_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.base_currency)
    }

    fn add_ir_delta(
        &mut self,
        currency: &Bound<'_, PyAny>,
        tenor: String,
        delta: f64,
    ) -> PyResult<()> {
        self.inner
            .add_ir_delta(parse_currency(currency)?, tenor, delta);
        Ok(())
    }

    fn add_ir_vega(
        &mut self,
        currency: &Bound<'_, PyAny>,
        tenor: String,
        vega: f64,
    ) -> PyResult<()> {
        self.inner
            .add_ir_vega(parse_currency(currency)?, tenor, vega);
        Ok(())
    }

    fn add_credit_delta(&mut self, name: String, qualifying: bool, tenor: String, delta: f64) {
        self.inner.add_credit_delta(name, qualifying, tenor, delta);
    }

    fn add_equity_delta(&mut self, underlier: String, delta: f64) {
        self.inner.add_equity_delta(underlier, delta);
    }

    fn add_equity_vega(&mut self, underlier: String, vega: f64) {
        self.inner.add_equity_vega(underlier, vega);
    }

    fn add_fx_delta(&mut self, currency: &Bound<'_, PyAny>, delta: f64) -> PyResult<()> {
        self.inner.add_fx_delta(parse_currency(currency)?, delta);
        Ok(())
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn total_ir_delta(&self) -> f64 {
        self.inner.total_ir_delta()
    }

    fn total_credit_delta(&self) -> f64 {
        self.inner.total_credit_delta()
    }

    fn total_equity_delta(&self) -> f64 {
        self.inner.total_equity_delta()
    }

    fn merge(&mut self, other: &PySimmSensitivities) {
        self.inner.merge(&other.inner);
    }

    fn __repr__(&self) -> String {
        format!(
            "SimmSensitivities(base_currency={}, empty={})",
            self.inner.base_currency,
            self.inner.is_empty()
        )
    }
}

// ---------------------------------------------------------------------------
// OtcMarginSpec
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// RepoMarginSpec
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// VmCalculator
// ---------------------------------------------------------------------------

/// Variation margin calculator following ISDA CSA rules.
#[pyclass(
    name = "VmCalculator",
    module = "finstack.valuations.margin",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyVmCalculator {
    pub(crate) inner: VmCalculator,
}

impl PyVmCalculator {
    pub(crate) fn new(inner: VmCalculator) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyVmCalculator {
    #[new]
    fn ctor(csa: PyCsaSpec) -> Self {
        Self::new(VmCalculator::new(csa.inner))
    }

    /// Calculate variation margin for a given exposure and posted collateral.
    fn calculate(
        &self,
        exposure: PyMoney,
        posted_collateral: PyMoney,
        as_of: &Bound<'_, PyAny>,
    ) -> PyResult<PyVmResult> {
        let date = py_to_date(as_of)?;
        let result = self
            .inner
            .calculate(exposure.inner, posted_collateral.inner, date)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(PyVmResult::new(result))
    }

    /// Generate a series of margin calls from an exposure time series.
    fn generate_margin_calls(
        &self,
        exposures: &Bound<'_, PyList>,
        initial_collateral: PyMoney,
    ) -> PyResult<Vec<PyMarginCall>> {
        let mut rust_exposures = Vec::with_capacity(exposures.len());
        for item in exposures.iter() {
            let tuple = item.extract::<(Bound<'_, PyAny>, PyMoney)>()?;
            let date = py_to_date(&tuple.0)?;
            rust_exposures.push((date, tuple.1.inner));
        }
        let calls = self
            .inner
            .generate_margin_calls(&rust_exposures, initial_collateral.inner)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(calls.into_iter().map(PyMarginCall::new).collect())
    }

    /// Generate margin call dates based on frequency.
    fn margin_call_dates(
        &self,
        py: Python<'_>,
        start: &Bound<'_, PyAny>,
        end: &Bound<'_, PyAny>,
    ) -> PyResult<Vec<Py<PyAny>>> {
        let start_date = py_to_date(start)?;
        let end_date = py_to_date(end)?;
        let dates = self.inner.margin_call_dates(start_date, end_date);
        dates.into_iter().map(|d| date_to_py(py, d)).collect()
    }

    fn __repr__(&self) -> String {
        "VmCalculator(...)".to_string()
    }
}

// ---------------------------------------------------------------------------
// SimmCalculator
// ---------------------------------------------------------------------------

/// SIMM version identifier for calculator construction.
#[pyclass(
    name = "SimmVersion",
    module = "finstack.valuations.margin",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PySimmVersion {
    pub(crate) inner: SimmVersion,
}

impl PySimmVersion {
    pub(crate) const fn new(inner: SimmVersion) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySimmVersion {
    #[classattr]
    const V2_5: Self = Self::new(SimmVersion::V2_5);
    #[classattr]
    const V2_6: Self = Self::new(SimmVersion::V2_6);

    #[getter]
    fn name(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("SimmVersion({})", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

/// ISDA SIMM calculator for sensitivity-based IM.
#[pyclass(
    name = "SimmCalculator",
    module = "finstack.valuations.margin",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PySimmCalculator {
    pub(crate) inner: SimmCalculator,
}

impl PySimmCalculator {
    pub(crate) fn new(inner: SimmCalculator) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySimmCalculator {
    #[new]
    #[pyo3(signature = (version=None))]
    fn ctor(version: Option<PySimmVersion>) -> PyResult<Self> {
        let v = version.map_or(SimmVersion::default(), |pv| pv.inner);
        let calc = SimmCalculator::new(v)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self::new(calc))
    }

    /// Calculate IM from a complete set of SIMM sensitivities.
    ///
    /// Returns a tuple of (total_im_amount, breakdown_dict) where the
    /// breakdown maps risk class names to Money amounts.
    fn calculate_from_sensitivities<'py>(
        &self,
        py: Python<'py>,
        sensitivities: &PySimmSensitivities,
        currency: &Bound<'py, PyAny>,
    ) -> PyResult<(f64, Py<PyAny>)> {
        let ccy = parse_currency(currency)?;
        let (total, breakdown) = self
            .inner
            .calculate_from_sensitivities(&sensitivities.inner, ccy);
        let d = PyDict::new(py);
        for (key, value) in &breakdown {
            d.set_item(key, PyMoney::new(*value).into_pyobject(py)?)?;
        }
        Ok((total, d.into()))
    }

    fn __repr__(&self) -> String {
        "SimmCalculator(...)".to_string()
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
        "Margin and collateral management types and calculators (CSA specs, VM/IM parameters, collateral schedules, margin calls, SIMM, repo margin).",
    )?;

    module.add_class::<PyMarginTenor>()?;
    module.add_class::<PyImMethodology>()?;
    module.add_class::<PyMarginCallTiming>()?;
    module.add_class::<PyVmParameters>()?;
    module.add_class::<PyImParameters>()?;
    module.add_class::<PyEligibleCollateralSchedule>()?;
    module.add_class::<PyCsaSpec>()?;
    module.add_class::<PyMarginCallType>()?;
    module.add_class::<PyCollateralAssetClass>()?;
    module.add_class::<PyClearingStatus>()?;
    module.add_class::<PySimmRiskClass>()?;
    module.add_class::<PyRepoMarginType>()?;
    module.add_class::<PyMarginCall>()?;
    module.add_class::<PyVmResult>()?;
    module.add_class::<PyImResult>()?;
    module.add_class::<PyInstrumentMarginResult>()?;
    module.add_class::<PySimmSensitivities>()?;
    module.add_class::<PyOtcMarginSpec>()?;
    module.add_class::<PyRepoMarginSpec>()?;
    module.add_class::<PyVmCalculator>()?;
    module.add_class::<PySimmVersion>()?;
    module.add_class::<PySimmCalculator>()?;

    let exports = [
        "MarginTenor",
        "ImMethodology",
        "MarginCallTiming",
        "VmParameters",
        "ImParameters",
        "EligibleCollateralSchedule",
        "CsaSpec",
        "MarginCallType",
        "CollateralAssetClass",
        "ClearingStatus",
        "SimmRiskClass",
        "RepoMarginType",
        "MarginCall",
        "VmResult",
        "ImResult",
        "InstrumentMarginResult",
        "SimmSensitivities",
        "OtcMarginSpec",
        "RepoMarginSpec",
        "VmCalculator",
        "SimmVersion",
        "SimmCalculator",
    ];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    parent.setattr("margin", &module)?;
    Ok(exports.to_vec())
}
