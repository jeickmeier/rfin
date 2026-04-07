//! Python bindings for Agency MBS passthrough instruments.

use crate::core::common::args::CurrencyArg;
use crate::core::dates::daycount::PyDayCount;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::errors::PyContext;
use finstack_core::dates::DayCount;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::cashflow::builder::specs::PrepaymentModelSpec;
use finstack_valuations::instruments::fixed_income::mbs_passthrough::{
    AgencyMbsPassthrough, AgencyProgram, PoolType,
};
use finstack_valuations::instruments::Attributes;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRefMut};
use std::sync::Arc;

// =============================================================================
// Agency Program Enum
// =============================================================================

/// Agency program (FNMA, FHLMC, GNMA).
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "AgencyProgram",
    eq,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PyAgencyProgram {
    /// Federal National Mortgage Association (Fannie Mae).
    Fnma,
    /// Federal Home Loan Mortgage Corporation (Freddie Mac).
    Fhlmc,
    /// Government National Mortgage Association (Ginnie Mae).
    ///
    /// Legacy alias that follows GNMA II conventions in the Rust core.
    Gnma,
    /// Government National Mortgage Association (Ginnie Mae).
    /// GNMA I (single-family MBS, 14-day payment delay).
    GnmaI,
    /// GNMA II (single-family MBS, 45-day payment delay).
    GnmaII,
}

impl From<PyAgencyProgram> for AgencyProgram {
    fn from(py: PyAgencyProgram) -> Self {
        match py {
            PyAgencyProgram::Fnma => AgencyProgram::Fnma,
            PyAgencyProgram::Fhlmc => AgencyProgram::Fhlmc,
            PyAgencyProgram::Gnma => AgencyProgram::Gnma,
            PyAgencyProgram::GnmaI => AgencyProgram::GnmaI,
            PyAgencyProgram::GnmaII => AgencyProgram::GnmaII,
        }
    }
}

impl From<AgencyProgram> for PyAgencyProgram {
    fn from(rust: AgencyProgram) -> Self {
        match rust {
            AgencyProgram::Fnma => PyAgencyProgram::Fnma,
            AgencyProgram::Fhlmc => PyAgencyProgram::Fhlmc,
            AgencyProgram::Gnma => PyAgencyProgram::Gnma,
            AgencyProgram::GnmaI => PyAgencyProgram::GnmaI,
            AgencyProgram::GnmaII => PyAgencyProgram::GnmaII,
        }
    }
}

#[pymethods]
impl PyAgencyProgram {
    fn __repr__(&self) -> String {
        format!("AgencyProgram.{:?}", self)
    }
}

// =============================================================================
// Pool Type Enum
// =============================================================================

/// Pool type classification.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "PoolType",
    eq,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PyPoolType {
    /// Generic pool (TBA-eligible, standard assumptions).
    Generic,
    /// Specified pool with known loan-level characteristics.
    Specified,
}

impl From<PyPoolType> for PoolType {
    fn from(py: PyPoolType) -> Self {
        match py {
            PyPoolType::Generic => PoolType::Generic,
            PyPoolType::Specified => PoolType::Specified,
        }
    }
}

impl From<PoolType> for PyPoolType {
    fn from(rust: PoolType) -> Self {
        match rust {
            PoolType::Generic => PyPoolType::Generic,
            PoolType::Specified => PyPoolType::Specified,
        }
    }
}

#[pymethods]
impl PyPoolType {
    fn __repr__(&self) -> String {
        format!("PoolType.{:?}", self)
    }
}

// =============================================================================
// Agency MBS Passthrough
// =============================================================================

/// Agency mortgage-backed security passthrough.
///
/// Examples:
///     >>> mbs = AgencyMbsPassthrough.builder("FN-MA1234").pool_id("MA1234").agency(AgencyProgram.Fnma).original_face(1_000_000.0).current_face(950_000.0).currency("USD").wac(0.045).pass_through_rate(0.04).wam(348).issue_date(Date(2022, 1, 1)).maturity_date(Date(2052, 1, 1)).discount_curve_id("USD-OIS").build()
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "AgencyMbsPassthrough",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyAgencyMbsPassthrough {
    pub(crate) inner: Arc<AgencyMbsPassthrough>,
}

impl PyAgencyMbsPassthrough {
    pub(crate) fn new(inner: AgencyMbsPassthrough) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "AgencyMbsPassthroughBuilder",
    unsendable,
    skip_from_py_object
)]
pub struct PyAgencyMbsPassthroughBuilder {
    instrument_id: InstrumentId,
    pool_id: Option<String>,
    agency: Option<AgencyProgram>,
    original_face: Option<f64>,
    current_face: Option<f64>,
    currency: Option<finstack_core::currency::Currency>,
    wac: Option<f64>,
    pass_through_rate: Option<f64>,
    wam: Option<u32>,
    issue_date: Option<time::Date>,
    maturity_date: Option<time::Date>,
    discount_curve_id: Option<String>,
    current_factor: Option<f64>,
    servicing_fee_rate: f64,
    guarantee_fee_rate: f64,
    pool_type: PoolType,
    psa_speed: f64,
    day_count: DayCount,
}

impl PyAgencyMbsPassthroughBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            pool_id: None,
            agency: None,
            original_face: None,
            current_face: None,
            currency: None,
            wac: None,
            pass_through_rate: None,
            wam: None,
            issue_date: None,
            maturity_date: None,
            discount_curve_id: None,
            current_factor: None,
            servicing_fee_rate: 0.0025,
            guarantee_fee_rate: 0.0025,
            pool_type: PoolType::Generic,
            psa_speed: 1.0,
            day_count: DayCount::Thirty360,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.pool_id.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("pool_id() is required."));
        }
        if self.agency.is_none() {
            return Err(PyValueError::new_err("agency() is required."));
        }
        if self.original_face.is_none() {
            return Err(PyValueError::new_err("original_face() is required."));
        }
        if self.current_face.is_none() {
            return Err(PyValueError::new_err("current_face() is required."));
        }
        if self.currency.is_none() {
            return Err(PyValueError::new_err("currency() is required."));
        }
        if self.wac.is_none() {
            return Err(PyValueError::new_err("wac() is required."));
        }
        if self.pass_through_rate.is_none() {
            return Err(PyValueError::new_err("pass_through_rate() is required."));
        }
        if self.wam.is_none() {
            return Err(PyValueError::new_err("wam() is required."));
        }
        if self.issue_date.is_none() {
            return Err(PyValueError::new_err("issue_date() is required."));
        }
        if self.maturity_date.is_none() {
            return Err(PyValueError::new_err("maturity_date() is required."));
        }
        if self.discount_curve_id.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("discount_curve_id() is required."));
        }
        Ok(())
    }
}

#[pymethods]
impl PyAgencyMbsPassthroughBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    fn pool_id(mut slf: PyRefMut<'_, Self>, pool_id: String) -> PyRefMut<'_, Self> {
        slf.pool_id = Some(pool_id);
        slf
    }

    fn agency(mut slf: PyRefMut<'_, Self>, agency: PyAgencyProgram) -> PyRefMut<'_, Self> {
        slf.agency = Some(agency.into());
        slf
    }

    fn original_face(mut slf: PyRefMut<'_, Self>, original_face: f64) -> PyRefMut<'_, Self> {
        slf.original_face = Some(original_face);
        slf
    }

    fn current_face(mut slf: PyRefMut<'_, Self>, current_face: f64) -> PyRefMut<'_, Self> {
        slf.current_face = Some(current_face);
        slf
    }

    fn currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        currency: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        slf.currency = Some(ccy);
        Ok(slf)
    }

    fn wac(mut slf: PyRefMut<'_, Self>, wac: f64) -> PyRefMut<'_, Self> {
        slf.wac = Some(wac);
        slf
    }

    fn pass_through_rate(
        mut slf: PyRefMut<'_, Self>,
        pass_through_rate: f64,
    ) -> PyRefMut<'_, Self> {
        slf.pass_through_rate = Some(pass_through_rate);
        slf
    }

    fn wam(mut slf: PyRefMut<'_, Self>, wam: u32) -> PyRefMut<'_, Self> {
        slf.wam = Some(wam);
        slf
    }

    fn issue_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        issue_date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.issue_date = Some(py_to_date(&issue_date).context("issue_date")?);
        Ok(slf)
    }

    fn maturity_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        maturity_date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.maturity_date = Some(py_to_date(&maturity_date).context("maturity_date")?);
        Ok(slf)
    }

    fn discount_curve_id(
        mut slf: PyRefMut<'_, Self>,
        discount_curve_id: String,
    ) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(discount_curve_id);
        slf
    }

    #[pyo3(signature = (current_factor=None))]
    fn current_factor(
        mut slf: PyRefMut<'_, Self>,
        current_factor: Option<f64>,
    ) -> PyRefMut<'_, Self> {
        slf.current_factor = current_factor;
        slf
    }

    fn servicing_fee_rate(
        mut slf: PyRefMut<'_, Self>,
        servicing_fee_rate: f64,
    ) -> PyRefMut<'_, Self> {
        slf.servicing_fee_rate = servicing_fee_rate;
        slf
    }

    fn guarantee_fee_rate(
        mut slf: PyRefMut<'_, Self>,
        guarantee_fee_rate: f64,
    ) -> PyRefMut<'_, Self> {
        slf.guarantee_fee_rate = guarantee_fee_rate;
        slf
    }

    fn pool_type(mut slf: PyRefMut<'_, Self>, pool_type: PyPoolType) -> PyRefMut<'_, Self> {
        slf.pool_type = pool_type.into();
        slf
    }

    fn psa_speed(mut slf: PyRefMut<'_, Self>, psa_speed: f64) -> PyRefMut<'_, Self> {
        slf.psa_speed = psa_speed;
        slf
    }

    fn day_count(mut slf: PyRefMut<'_, Self>, day_count: PyDayCount) -> PyRefMut<'_, Self> {
        slf.day_count = day_count.inner;
        slf
    }

    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyAgencyMbsPassthrough> {
        slf.ensure_ready()?;

        let original_face = slf.original_face.ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("AgencyMbsPassthroughBuilder internal error: missing original_face after validation"))?;
        let current_face = slf.current_face.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "AgencyMbsPassthroughBuilder internal error: missing current_face after validation",
            )
        })?;
        let factor = slf.current_factor.unwrap_or(current_face / original_face);
        let prepay = PrepaymentModelSpec::psa(slf.psa_speed);

        let ccy = slf.currency.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "AgencyMbsPassthroughBuilder internal error: missing currency after validation",
            )
        })?;
        let mbs = AgencyMbsPassthrough::builder()
            .id(slf.instrument_id.clone())
            .pool_id(slf.pool_id.clone().ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("AgencyMbsPassthroughBuilder internal error: missing pool_id after validation"))?.into())
            .agency(slf.agency.ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("AgencyMbsPassthroughBuilder internal error: missing agency after validation"))?)
            .pool_type(slf.pool_type)
            .original_face(Money::new(original_face, ccy))
            .current_face(Money::new(current_face, ccy))
            .current_factor(factor)
            .wac(slf.wac.ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("AgencyMbsPassthroughBuilder internal error: missing wac after validation"))?)
            .pass_through_rate(slf.pass_through_rate.ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("AgencyMbsPassthroughBuilder internal error: missing pass_through_rate after validation"))?)
            .servicing_fee_rate(slf.servicing_fee_rate)
            .guarantee_fee_rate(slf.guarantee_fee_rate)
            .wam(slf.wam.ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("AgencyMbsPassthroughBuilder internal error: missing wam after validation"))?)
            .issue_date(slf.issue_date.ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("AgencyMbsPassthroughBuilder internal error: missing issue_date after validation"))?)
            .maturity(slf.maturity_date.ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("AgencyMbsPassthroughBuilder internal error: missing maturity_date after validation"))?)
            .prepayment_model(prepay)
            .discount_curve_id(CurveId::new(slf.discount_curve_id.as_deref().ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("AgencyMbsPassthroughBuilder internal error: missing discount_curve_id after validation"))?))
            .day_count(slf.day_count)
            .attributes(Attributes::new())
            .build()
            .map_err(|e| PyValueError::new_err(format!("{e}")))?;

        Ok(PyAgencyMbsPassthrough::new(mbs))
    }
}

#[pymethods]
impl PyAgencyMbsPassthrough {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyAgencyMbsPassthroughBuilder>> {
        let py = cls.py();
        let builder = PyAgencyMbsPassthroughBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    /// Create an example MBS for testing.
    #[classmethod]
    fn example(_cls: &Bound<'_, PyType>) -> PyResult<Self> {
        AgencyMbsPassthrough::example()
            .map(Self::new)
            .map_err(|e| PyValueError::new_err(format!("{e}")))
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Pool identifier.
    #[getter]
    fn pool_id(&self) -> &str {
        &self.inner.pool_id
    }

    /// Agency program.
    #[getter]
    fn agency(&self) -> PyAgencyProgram {
        self.inner.agency.into()
    }

    /// Pool type.
    #[getter]
    fn pool_type(&self) -> PyPoolType {
        self.inner.pool_type.into()
    }

    /// Original face value.
    #[getter]
    fn original_face(&self) -> f64 {
        self.inner.original_face.amount()
    }

    /// Current face value.
    #[getter]
    fn current_face(&self) -> f64 {
        self.inner.current_face.amount()
    }

    /// Current pool factor.
    #[getter]
    fn current_factor(&self) -> f64 {
        self.inner.current_factor
    }

    /// Weighted average coupon.
    #[getter]
    fn wac(&self) -> f64 {
        self.inner.wac
    }

    /// Pass-through rate.
    #[getter]
    fn pass_through_rate(&self) -> f64 {
        self.inner.pass_through_rate
    }

    /// Servicing fee rate.
    #[getter]
    fn servicing_fee_rate(&self) -> f64 {
        self.inner.servicing_fee_rate
    }

    /// Guarantee fee rate.
    #[getter]
    fn guarantee_fee_rate(&self) -> f64 {
        self.inner.guarantee_fee_rate
    }

    /// Weighted average maturity (months).
    #[getter]
    fn wam(&self) -> u32 {
        self.inner.wam
    }

    /// Issue date.
    #[getter]
    fn issue_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.issue_date)
    }

    /// Maturity date.
    #[getter]
    fn maturity_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.maturity)
    }

    /// Discount curve ID.
    #[getter]
    fn discount_curve_id(&self) -> &str {
        self.inner.discount_curve_id.as_str()
    }

    fn __repr__(&self) -> String {
        format!(
            "AgencyMbsPassthrough(id='{}', agency={:?}, current_face={:.2})",
            self.inner.id.as_str(),
            self.inner.agency,
            self.inner.current_face.amount()
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
    parent.add_class::<PyAgencyProgram>()?;
    parent.add_class::<PyPoolType>()?;
    parent.add_class::<PyAgencyMbsPassthrough>()?;
    parent.add_class::<PyAgencyMbsPassthroughBuilder>()?;

    Ok(vec![
        "AgencyProgram",
        "PoolType",
        "AgencyMbsPassthrough",
        "AgencyMbsPassthroughBuilder",
    ])
}
