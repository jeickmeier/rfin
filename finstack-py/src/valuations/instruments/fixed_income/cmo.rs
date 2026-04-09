//! Python bindings for Agency CMO instruments.

use crate::core::common::args::CurrencyArg;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::errors::PyContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::fixed_income::cmo::{
    AgencyCmo, CmoTranche, CmoTrancheType, CmoWaterfall, PacCollar,
};
use finstack_valuations::instruments::fixed_income::mbs_passthrough::AgencyProgram;
use finstack_valuations::instruments::Attributes;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRefMut};
use std::sync::Arc;

use super::mbs_passthrough::PyAgencyProgram;

// =============================================================================
// CMO Tranche Type Enum
// =============================================================================

/// CMO tranche type.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CmoTrancheType",
    eq,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PyCmoTrancheType {
    /// Sequential pay tranche.
    Sequential,
    /// Planned Amortization Class.
    Pac,
    /// Support/companion tranche.
    Support,
    /// Interest-only strip.
    InterestOnly,
    /// Principal-only strip.
    PrincipalOnly,
}

impl From<PyCmoTrancheType> for CmoTrancheType {
    fn from(py: PyCmoTrancheType) -> Self {
        match py {
            PyCmoTrancheType::Sequential => CmoTrancheType::Sequential,
            PyCmoTrancheType::Pac => CmoTrancheType::Pac,
            PyCmoTrancheType::Support => CmoTrancheType::Support,
            PyCmoTrancheType::InterestOnly => CmoTrancheType::InterestOnly,
            PyCmoTrancheType::PrincipalOnly => CmoTrancheType::PrincipalOnly,
        }
    }
}

impl From<CmoTrancheType> for PyCmoTrancheType {
    fn from(rust: CmoTrancheType) -> Self {
        match rust {
            CmoTrancheType::Sequential => PyCmoTrancheType::Sequential,
            CmoTrancheType::Pac => PyCmoTrancheType::Pac,
            CmoTrancheType::Support => PyCmoTrancheType::Support,
            CmoTrancheType::InterestOnly => PyCmoTrancheType::InterestOnly,
            CmoTrancheType::PrincipalOnly => PyCmoTrancheType::PrincipalOnly,
        }
    }
}

#[pymethods]
impl PyCmoTrancheType {
    fn __repr__(&self) -> String {
        format!("CmoTrancheType.{:?}", self)
    }
}

// =============================================================================
// PAC Collar
// =============================================================================

/// PAC collar boundaries.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "PacCollar",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyPacCollar {
    pub(crate) inner: PacCollar,
}

impl PyPacCollar {
    pub(crate) fn new(inner: PacCollar) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPacCollar {
    /// Create a PAC collar.
    #[new]
    #[pyo3(signature = (lower_psa, upper_psa))]
    fn new_py(lower_psa: f64, upper_psa: f64) -> Self {
        Self::new(PacCollar::new(lower_psa, upper_psa))
    }

    /// Create a standard 100-300 PSA collar.
    #[classmethod]
    fn standard(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(PacCollar::standard())
    }

    /// Lower PSA bound.
    #[getter]
    fn lower_psa(&self) -> f64 {
        self.inner.lower_psa
    }

    /// Upper PSA bound.
    #[getter]
    fn upper_psa(&self) -> f64 {
        self.inner.upper_psa
    }

    fn __repr__(&self) -> String {
        format!(
            "PacCollar(lower={:.0}%, upper={:.0}%)",
            self.inner.lower_psa * 100.0,
            self.inner.upper_psa * 100.0
        )
    }
}

// =============================================================================
// CMO Tranche
// =============================================================================

/// CMO tranche definition.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CmoTranche",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCmoTranche {
    pub(crate) inner: CmoTranche,
}

impl PyCmoTranche {
    pub(crate) fn new(inner: CmoTranche) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCmoTranche {
    /// Create a sequential tranche.
    #[classmethod]
    #[pyo3(signature = (tranche_id, face, currency, coupon, priority))]
    fn sequential(
        _cls: &Bound<'_, PyType>,
        tranche_id: &str,
        face: f64,
        currency: Bound<'_, PyAny>,
        coupon: f64,
        priority: u32,
    ) -> PyResult<Self> {
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        Ok(Self::new(CmoTranche::sequential(
            tranche_id,
            Money::new(face, ccy),
            coupon,
            priority,
        )))
    }

    /// Create a PAC tranche.
    #[classmethod]
    #[pyo3(signature = (tranche_id, face, currency, coupon, priority, collar))]
    fn pac(
        _cls: &Bound<'_, PyType>,
        tranche_id: &str,
        face: f64,
        currency: Bound<'_, PyAny>,
        coupon: f64,
        priority: u32,
        collar: &PyPacCollar,
    ) -> PyResult<Self> {
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        Ok(Self::new(CmoTranche::pac(
            tranche_id,
            Money::new(face, ccy),
            coupon,
            priority,
            collar.inner.clone(),
        )))
    }

    /// Create a support tranche.
    #[classmethod]
    #[pyo3(signature = (tranche_id, face, currency, coupon, priority))]
    fn support(
        _cls: &Bound<'_, PyType>,
        tranche_id: &str,
        face: f64,
        currency: Bound<'_, PyAny>,
        coupon: f64,
        priority: u32,
    ) -> PyResult<Self> {
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        Ok(Self::new(CmoTranche::support(
            tranche_id,
            Money::new(face, ccy),
            coupon,
            priority,
        )))
    }

    /// Create an IO strip.
    #[classmethod]
    #[pyo3(signature = (tranche_id, notional, currency, coupon))]
    fn io_strip(
        _cls: &Bound<'_, PyType>,
        tranche_id: &str,
        notional: f64,
        currency: Bound<'_, PyAny>,
        coupon: f64,
    ) -> PyResult<Self> {
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        Ok(Self::new(CmoTranche::io_strip(
            tranche_id,
            Money::new(notional, ccy),
            coupon,
        )))
    }

    /// Create a PO strip.
    #[classmethod]
    #[pyo3(signature = (tranche_id, face, currency))]
    fn po_strip(
        _cls: &Bound<'_, PyType>,
        tranche_id: &str,
        face: f64,
        currency: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        Ok(Self::new(CmoTranche::po_strip(
            tranche_id,
            Money::new(face, ccy),
        )))
    }

    /// Tranche identifier.
    #[getter]
    fn id(&self) -> &str {
        &self.inner.id
    }

    /// Tranche type.
    #[getter]
    fn tranche_type(&self) -> PyCmoTrancheType {
        self.inner.tranche_type.into()
    }

    /// Original face.
    #[getter]
    fn original_face(&self) -> f64 {
        self.inner.original_face.amount()
    }

    /// Current face.
    #[getter]
    fn current_face(&self) -> f64 {
        self.inner.current_face.amount()
    }

    /// Coupon rate.
    #[getter]
    fn coupon(&self) -> f64 {
        self.inner.coupon
    }

    /// Payment priority.
    #[getter]
    fn priority(&self) -> u32 {
        self.inner.priority
    }

    fn __repr__(&self) -> String {
        format!(
            "CmoTranche(id='{}', type={:?}, face={:.2})",
            self.inner.id,
            self.inner.tranche_type,
            self.inner.original_face.amount()
        )
    }
}

// =============================================================================
// CMO Waterfall
// =============================================================================

/// CMO waterfall structure.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CmoWaterfall",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCmoWaterfall {
    pub(crate) inner: CmoWaterfall,
}

impl PyCmoWaterfall {
    pub(crate) fn new(inner: CmoWaterfall) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCmoWaterfall {
    /// Create a new waterfall from tranches.
    #[new]
    #[pyo3(signature = (tranches))]
    fn new_py(tranches: Vec<PyCmoTranche>) -> Self {
        let rust_tranches: Vec<CmoTranche> = tranches.into_iter().map(|t| t.inner).collect();
        Self::new(CmoWaterfall::new(rust_tranches))
    }

    /// Get all tranches in the waterfall.
    #[getter]
    fn tranches(&self) -> Vec<PyCmoTranche> {
        self.inner
            .tranches
            .iter()
            .cloned()
            .map(PyCmoTranche::new)
            .collect()
    }

    /// Get tranche by ID.
    fn get_tranche(&self, tranche_id: &str) -> Option<PyCmoTranche> {
        self.inner
            .get_tranche(tranche_id)
            .cloned()
            .map(PyCmoTranche::new)
    }

    /// Total current face.
    fn total_current_face(&self) -> f64 {
        self.inner.total_current_face().amount()
    }

    fn __repr__(&self) -> String {
        format!(
            "CmoWaterfall(tranches={}, total_face={:.2})",
            self.inner.tranches.len(),
            self.inner.total_current_face().amount()
        )
    }
}

// =============================================================================
// Agency CMO
// =============================================================================

/// Agency Collateralized Mortgage Obligation.
///
/// Examples:
///     >>> tranches = [
///     ...     CmoTranche.sequential("A", 40_000_000.0, "USD", 0.04, 1),
///     ...     CmoTranche.sequential("B", 30_000_000.0, "USD", 0.045, 2),
///     ... ]
///     >>> waterfall = CmoWaterfall(tranches)
///     >>> cmo = AgencyCmo.builder("FNR-2024-1-A").deal_name("FNR 2024-1").agency(AgencyProgram.Fnma).issue_date(Date(2024, 1, 1)).waterfall(waterfall).reference_tranche_id("A").discount_curve_id("USD-OIS").build()
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "AgencyCmo",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyAgencyCmo {
    pub(crate) inner: Arc<AgencyCmo>,
}

impl PyAgencyCmo {
    pub(crate) fn new(inner: AgencyCmo) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "AgencyCmoBuilder",
    skip_from_py_object
)]
pub struct PyAgencyCmoBuilder {
    instrument_id: InstrumentId,
    deal_name: Option<String>,
    agency: Option<AgencyProgram>,
    issue_date: Option<time::Date>,
    waterfall: Option<CmoWaterfall>,
    reference_tranche_id: Option<String>,
    discount_curve_id: Option<String>,
    collateral_wac: Option<f64>,
    collateral_wam: Option<u32>,
}

impl PyAgencyCmoBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            deal_name: None,
            agency: None,
            issue_date: None,
            waterfall: None,
            reference_tranche_id: None,
            discount_curve_id: None,
            collateral_wac: None,
            collateral_wam: None,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.deal_name.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("deal_name() is required."));
        }
        if self.agency.is_none() {
            return Err(PyValueError::new_err("agency() is required."));
        }
        if self.issue_date.is_none() {
            return Err(PyValueError::new_err("issue_date() is required."));
        }
        if self.waterfall.is_none() {
            return Err(PyValueError::new_err("waterfall() is required."));
        }
        if self
            .reference_tranche_id
            .as_deref()
            .unwrap_or("")
            .is_empty()
        {
            return Err(PyValueError::new_err("reference_tranche_id() is required."));
        }
        if self.discount_curve_id.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("discount_curve_id() is required."));
        }
        Ok(())
    }
}

#[pymethods]
impl PyAgencyCmoBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    fn deal_name(mut slf: PyRefMut<'_, Self>, deal_name: String) -> PyRefMut<'_, Self> {
        slf.deal_name = Some(deal_name);
        slf
    }

    fn agency(mut slf: PyRefMut<'_, Self>, agency: PyAgencyProgram) -> PyRefMut<'_, Self> {
        slf.agency = Some(agency.into());
        slf
    }

    fn issue_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        issue_date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.issue_date = Some(py_to_date(&issue_date).context("issue_date")?);
        Ok(slf)
    }

    fn waterfall<'py>(
        mut slf: PyRefMut<'py, Self>,
        waterfall: &PyCmoWaterfall,
    ) -> PyRefMut<'py, Self> {
        slf.waterfall = Some(waterfall.inner.clone());
        slf
    }

    fn reference_tranche_id(
        mut slf: PyRefMut<'_, Self>,
        reference_tranche_id: String,
    ) -> PyRefMut<'_, Self> {
        slf.reference_tranche_id = Some(reference_tranche_id);
        slf
    }

    fn discount_curve_id(
        mut slf: PyRefMut<'_, Self>,
        discount_curve_id: String,
    ) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(discount_curve_id);
        slf
    }

    #[pyo3(signature = (collateral_wac=None))]
    fn collateral_wac(
        mut slf: PyRefMut<'_, Self>,
        collateral_wac: Option<f64>,
    ) -> PyRefMut<'_, Self> {
        slf.collateral_wac = collateral_wac;
        slf
    }

    #[pyo3(signature = (collateral_wam=None))]
    fn collateral_wam(
        mut slf: PyRefMut<'_, Self>,
        collateral_wam: Option<u32>,
    ) -> PyRefMut<'_, Self> {
        slf.collateral_wam = collateral_wam;
        slf
    }

    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyAgencyCmo> {
        slf.ensure_ready()?;

        let mut builder = AgencyCmo::builder()
            .id(slf.instrument_id.clone())
            .deal_name(slf.deal_name.clone().ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("AgencyCmoBuilder internal error: missing deal_name after validation"))?.into())
            .agency(slf.agency.ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("AgencyCmoBuilder internal error: missing agency after validation"))?)
            .issue_date(slf.issue_date.ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("AgencyCmoBuilder internal error: missing issue_date after validation"))?)
            .waterfall(slf.waterfall.clone().ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("AgencyCmoBuilder internal error: missing waterfall after validation"))?)
            .reference_tranche_id(slf.reference_tranche_id.clone().ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("AgencyCmoBuilder internal error: missing reference_tranche_id after validation"))?)
            .discount_curve_id(CurveId::new(slf.discount_curve_id.as_deref().ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("AgencyCmoBuilder internal error: missing discount_curve_id after validation"))?))
            .attributes(Attributes::new());

        if let Some(wac) = slf.collateral_wac {
            builder = builder.collateral_wac_opt(Some(wac));
        }
        if let Some(wam) = slf.collateral_wam {
            builder = builder.collateral_wam_opt(Some(wam));
        }

        let cmo = builder
            .build()
            .map_err(|e| PyValueError::new_err(format!("{e}")))?;
        Ok(PyAgencyCmo::new(cmo))
    }
}

#[pymethods]
impl PyAgencyCmo {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyAgencyCmoBuilder>> {
        let py = cls.py();
        let builder = PyAgencyCmoBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    /// Create an example CMO for testing.
    #[classmethod]
    fn example(_cls: &Bound<'_, PyType>) -> PyResult<Self> {
        AgencyCmo::example()
            .map(Self::new)
            .map_err(|e| PyValueError::new_err(format!("{e}")))
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Deal name.
    #[getter]
    fn deal_name(&self) -> &str {
        &self.inner.deal_name
    }

    /// Agency program.
    #[getter]
    fn agency(&self) -> PyAgencyProgram {
        self.inner.agency.into()
    }

    /// Issue date.
    #[getter]
    fn issue_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.issue_date)
    }

    /// Reference tranche ID.
    #[getter]
    fn reference_tranche_id(&self) -> &str {
        &self.inner.reference_tranche_id
    }

    /// Waterfall structure.
    #[getter]
    fn waterfall(&self) -> PyCmoWaterfall {
        PyCmoWaterfall::new(self.inner.waterfall.clone())
    }

    /// Discount curve ID.
    #[getter]
    fn discount_curve_id(&self) -> &str {
        self.inner.discount_curve_id.as_str()
    }

    fn __repr__(&self) -> String {
        format!(
            "AgencyCmo(id='{}', deal='{}', tranche='{}')",
            self.inner.id.as_str(),
            self.inner.deal_name,
            self.inner.reference_tranche_id
        )
    }
}

// =============================================================================
// Module Registration
// =============================================================================

pub(crate) fn register(
    _py: Python<'_>,
    parent: &Bound<'_, PyModule>,
) -> PyResult<Vec<&'static str>> {
    parent.add_class::<PyCmoTrancheType>()?;
    parent.add_class::<PyPacCollar>()?;
    parent.add_class::<PyCmoTranche>()?;
    parent.add_class::<PyCmoWaterfall>()?;
    parent.add_class::<PyAgencyCmo>()?;
    parent.add_class::<PyAgencyCmoBuilder>()?;

    Ok(vec![
        "CmoTrancheType",
        "PacCollar",
        "CmoTranche",
        "CmoWaterfall",
        "AgencyCmo",
        "AgencyCmoBuilder",
    ])
}
