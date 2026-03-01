#![allow(clippy::unwrap_used)]

//! Python bindings for Agency MBS instruments.
//!
//! Rust sources: consolidates 4 Rust modules into one Python module for convenience.
//!
//! This module provides Python bindings for:
//! - `AgencyMbsPassthrough` - Agency mortgage-backed security passthrough
//! - `AgencyTba` - To-Be-Announced forward contract
//! - `DollarRoll` - Dollar roll between TBA months
//! - `AgencyCmo` - Collateralized mortgage obligation

use crate::core::common::args::CurrencyArg;
use crate::core::dates::daycount::PyDayCount;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::errors::PyContext;
use finstack_core::dates::DayCount;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::cashflow::builder::specs::PrepaymentModelSpec;
use finstack_valuations::instruments::fixed_income::cmo::{
    AgencyCmo, CmoTranche, CmoTrancheType, CmoWaterfall, PacCollar,
};
use finstack_valuations::instruments::fixed_income::dollar_roll::DollarRoll;
use finstack_valuations::instruments::fixed_income::mbs_passthrough::{
    AgencyMbsPassthrough, AgencyProgram, PoolType,
};
use finstack_valuations::instruments::fixed_income::tba::{AgencyTba, TbaTerm};
use finstack_valuations::instruments::Attributes;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};
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
    /// Deprecated: use GnmaI or GnmaII for specific program type.
    Gnma,
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
// TBA Term Enum
// =============================================================================

/// TBA original loan term.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "TbaTerm",
    eq,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum PyTbaTerm {
    /// 15-year original term.
    FifteenYear,
    /// 20-year original term.
    TwentyYear,
    /// 30-year original term.
    ThirtyYear,
}

impl From<PyTbaTerm> for TbaTerm {
    fn from(py: PyTbaTerm) -> Self {
        match py {
            PyTbaTerm::FifteenYear => TbaTerm::FifteenYear,
            PyTbaTerm::TwentyYear => TbaTerm::TwentyYear,
            PyTbaTerm::ThirtyYear => TbaTerm::ThirtyYear,
        }
    }
}

impl From<TbaTerm> for PyTbaTerm {
    fn from(rust: TbaTerm) -> Self {
        match rust {
            TbaTerm::FifteenYear => PyTbaTerm::FifteenYear,
            TbaTerm::TwentyYear => PyTbaTerm::TwentyYear,
            TbaTerm::ThirtyYear => PyTbaTerm::ThirtyYear,
            _ => unreachable!("unknown TbaTerm variant"),
        }
    }
}

#[pymethods]
impl PyTbaTerm {
    fn __repr__(&self) -> String {
        format!("TbaTerm.{:?}", self)
    }
}

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
    unsendable
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

        let original_face = slf.original_face.unwrap();
        let current_face = slf.current_face.unwrap();
        let factor = slf.current_factor.unwrap_or(current_face / original_face);
        let prepay = PrepaymentModelSpec::psa(slf.psa_speed);

        let ccy = slf.currency.unwrap();
        let mbs = AgencyMbsPassthrough::builder()
            .id(slf.instrument_id.clone())
            .pool_id(slf.pool_id.clone().unwrap().into())
            .agency(slf.agency.unwrap())
            .pool_type(slf.pool_type)
            .original_face(Money::new(original_face, ccy))
            .current_face(Money::new(current_face, ccy))
            .current_factor(factor)
            .wac(slf.wac.unwrap())
            .pass_through_rate(slf.pass_through_rate.unwrap())
            .servicing_fee_rate(slf.servicing_fee_rate)
            .guarantee_fee_rate(slf.guarantee_fee_rate)
            .wam(slf.wam.unwrap())
            .issue_date(slf.issue_date.unwrap())
            .maturity(slf.maturity_date.unwrap())
            .prepayment_model(prepay)
            .discount_curve_id(CurveId::new(slf.discount_curve_id.as_deref().unwrap()))
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
    fn example(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(AgencyMbsPassthrough::example())
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
// Agency TBA
// =============================================================================

/// Agency To-Be-Announced (TBA) forward contract.
///
/// Examples:
///     >>> tba = AgencyTba.builder("FN30-4.0-202403").agency(AgencyProgram.Fnma).coupon(0.04).term(TbaTerm.ThirtyYear).settlement_year(2024).settlement_month(3).notional(10_000_000.0).currency("USD").trade_price(98.5).discount_curve_id("USD-OIS").build()
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "AgencyTba",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyAgencyTba {
    pub(crate) inner: Arc<AgencyTba>,
}

impl PyAgencyTba {
    pub(crate) fn new(inner: AgencyTba) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "AgencyTbaBuilder",
    unsendable
)]
pub struct PyAgencyTbaBuilder {
    instrument_id: InstrumentId,
    agency: Option<AgencyProgram>,
    coupon: Option<f64>,
    term: Option<TbaTerm>,
    settlement_year: Option<i32>,
    settlement_month: Option<u8>,
    notional: Option<f64>,
    currency: Option<finstack_core::currency::Currency>,
    trade_price: Option<f64>,
    discount_curve_id: Option<String>,
    trade_date: Option<time::Date>,
}

impl PyAgencyTbaBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            agency: None,
            coupon: None,
            term: None,
            settlement_year: None,
            settlement_month: None,
            notional: None,
            currency: None,
            trade_price: None,
            discount_curve_id: None,
            trade_date: None,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.agency.is_none() {
            return Err(PyValueError::new_err("agency() is required."));
        }
        if self.coupon.is_none() {
            return Err(PyValueError::new_err("coupon() is required."));
        }
        if self.term.is_none() {
            return Err(PyValueError::new_err("term() is required."));
        }
        if self.settlement_year.is_none() {
            return Err(PyValueError::new_err("settlement_year() is required."));
        }
        if self.settlement_month.is_none() {
            return Err(PyValueError::new_err("settlement_month() is required."));
        }
        if self.notional.is_none() {
            return Err(PyValueError::new_err("notional() is required."));
        }
        if self.currency.is_none() {
            return Err(PyValueError::new_err("currency() is required."));
        }
        if self.trade_price.is_none() {
            return Err(PyValueError::new_err("trade_price() is required."));
        }
        if self.discount_curve_id.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("discount_curve_id() is required."));
        }
        Ok(())
    }
}

#[pymethods]
impl PyAgencyTbaBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    fn agency(mut slf: PyRefMut<'_, Self>, agency: PyAgencyProgram) -> PyRefMut<'_, Self> {
        slf.agency = Some(agency.into());
        slf
    }

    fn coupon(mut slf: PyRefMut<'_, Self>, coupon: f64) -> PyRefMut<'_, Self> {
        slf.coupon = Some(coupon);
        slf
    }

    fn term(mut slf: PyRefMut<'_, Self>, term: PyTbaTerm) -> PyRefMut<'_, Self> {
        slf.term = Some(term.into());
        slf
    }

    fn settlement_year(mut slf: PyRefMut<'_, Self>, settlement_year: i32) -> PyRefMut<'_, Self> {
        slf.settlement_year = Some(settlement_year);
        slf
    }

    fn settlement_month(mut slf: PyRefMut<'_, Self>, settlement_month: u8) -> PyRefMut<'_, Self> {
        slf.settlement_month = Some(settlement_month);
        slf
    }

    fn notional(mut slf: PyRefMut<'_, Self>, notional: f64) -> PyRefMut<'_, Self> {
        slf.notional = Some(notional);
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

    fn trade_price(mut slf: PyRefMut<'_, Self>, trade_price: f64) -> PyRefMut<'_, Self> {
        slf.trade_price = Some(trade_price);
        slf
    }

    fn discount_curve_id(
        mut slf: PyRefMut<'_, Self>,
        discount_curve_id: String,
    ) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(discount_curve_id);
        slf
    }

    #[pyo3(signature = (trade_date=None))]
    fn trade_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        trade_date: Option<Bound<'py, PyAny>>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.trade_date = trade_date.map(|d| py_to_date(&d)).transpose()?;
        Ok(slf)
    }

    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyAgencyTba> {
        slf.ensure_ready()?;
        let ccy = slf.currency.unwrap();

        let mut builder = AgencyTba::builder()
            .id(slf.instrument_id.clone())
            .agency(slf.agency.unwrap())
            .coupon(slf.coupon.unwrap())
            .term(slf.term.unwrap())
            .settlement_year(slf.settlement_year.unwrap())
            .settlement_month(slf.settlement_month.unwrap())
            .notional(Money::new(slf.notional.unwrap(), ccy))
            .trade_price(slf.trade_price.unwrap())
            .discount_curve_id(CurveId::new(slf.discount_curve_id.as_deref().unwrap()))
            .attributes(Attributes::new());

        if let Some(td) = slf.trade_date {
            builder = builder.trade_date_opt(Some(td));
        }

        let tba = builder
            .build()
            .map_err(|e| PyValueError::new_err(format!("{e}")))?;
        Ok(PyAgencyTba::new(tba))
    }
}

#[pymethods]
impl PyAgencyTba {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyAgencyTbaBuilder>> {
        let py = cls.py();
        let builder = PyAgencyTbaBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    /// Create an example TBA for testing.
    #[classmethod]
    fn example(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(AgencyTba::example())
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Agency program.
    #[getter]
    fn agency(&self) -> PyAgencyProgram {
        self.inner.agency.into()
    }

    /// Coupon rate.
    #[getter]
    fn coupon(&self) -> f64 {
        self.inner.coupon
    }

    /// Term.
    #[getter]
    fn term(&self) -> PyTbaTerm {
        self.inner.term.into()
    }

    /// Settlement year.
    #[getter]
    fn settlement_year(&self) -> i32 {
        self.inner.settlement_year
    }

    /// Settlement month.
    #[getter]
    fn settlement_month(&self) -> u8 {
        self.inner.settlement_month
    }

    /// Notional amount.
    #[getter]
    fn notional(&self) -> f64 {
        self.inner.notional.amount()
    }

    /// Trade price.
    #[getter]
    fn trade_price(&self) -> f64 {
        self.inner.trade_price
    }

    /// Discount curve ID.
    #[getter]
    fn discount_curve_id(&self) -> &str {
        self.inner.discount_curve_id.as_str()
    }

    fn __repr__(&self) -> String {
        format!(
            "AgencyTba(id='{}', coupon={:.2}%, settlement={}/{:02})",
            self.inner.id.as_str(),
            self.inner.coupon * 100.0,
            self.inner.settlement_year,
            self.inner.settlement_month
        )
    }
}

// =============================================================================
// Dollar Roll
// =============================================================================

/// Dollar roll between TBA settlement months.
///
/// A dollar roll involves selling TBA for near-month settlement
/// and buying TBA for far-month settlement.
///
/// Examples:
///     >>> roll = DollarRoll.builder("FN30-4.0-ROLL-0324-0424").agency(AgencyProgram.Fnma).coupon(0.04).term(TbaTerm.ThirtyYear).notional(10_000_000.0).currency("USD").front_settlement_year(2024).front_settlement_month(3).back_settlement_year(2024).back_settlement_month(4).front_price(98.5).back_price(98.0).discount_curve_id("USD-OIS").build()
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "DollarRoll",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDollarRoll {
    pub(crate) inner: Arc<DollarRoll>,
}

impl PyDollarRoll {
    pub(crate) fn new(inner: DollarRoll) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "DollarRollBuilder",
    unsendable
)]
pub struct PyDollarRollBuilder {
    instrument_id: InstrumentId,
    agency: Option<AgencyProgram>,
    coupon: Option<f64>,
    term: Option<TbaTerm>,
    notional: Option<f64>,
    currency: Option<finstack_core::currency::Currency>,
    front_settlement_year: Option<i32>,
    front_settlement_month: Option<u8>,
    back_settlement_year: Option<i32>,
    back_settlement_month: Option<u8>,
    front_price: Option<f64>,
    back_price: Option<f64>,
    discount_curve_id: Option<String>,
    trade_date: Option<time::Date>,
}

impl PyDollarRollBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            agency: None,
            coupon: None,
            term: None,
            notional: None,
            currency: None,
            front_settlement_year: None,
            front_settlement_month: None,
            back_settlement_year: None,
            back_settlement_month: None,
            front_price: None,
            back_price: None,
            discount_curve_id: None,
            trade_date: None,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.agency.is_none() {
            return Err(PyValueError::new_err("agency() is required."));
        }
        if self.coupon.is_none() {
            return Err(PyValueError::new_err("coupon() is required."));
        }
        if self.term.is_none() {
            return Err(PyValueError::new_err("term() is required."));
        }
        if self.notional.is_none() {
            return Err(PyValueError::new_err("notional() is required."));
        }
        if self.currency.is_none() {
            return Err(PyValueError::new_err("currency() is required."));
        }
        if self.front_settlement_year.is_none() {
            return Err(PyValueError::new_err(
                "front_settlement_year() is required.",
            ));
        }
        if self.front_settlement_month.is_none() {
            return Err(PyValueError::new_err(
                "front_settlement_month() is required.",
            ));
        }
        if self.back_settlement_year.is_none() {
            return Err(PyValueError::new_err("back_settlement_year() is required."));
        }
        if self.back_settlement_month.is_none() {
            return Err(PyValueError::new_err(
                "back_settlement_month() is required.",
            ));
        }
        if self.front_price.is_none() {
            return Err(PyValueError::new_err("front_price() is required."));
        }
        if self.back_price.is_none() {
            return Err(PyValueError::new_err("back_price() is required."));
        }
        if self.discount_curve_id.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("discount_curve_id() is required."));
        }
        Ok(())
    }
}

#[pymethods]
impl PyDollarRollBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    fn agency(mut slf: PyRefMut<'_, Self>, agency: PyAgencyProgram) -> PyRefMut<'_, Self> {
        slf.agency = Some(agency.into());
        slf
    }

    fn coupon(mut slf: PyRefMut<'_, Self>, coupon: f64) -> PyRefMut<'_, Self> {
        slf.coupon = Some(coupon);
        slf
    }

    fn term(mut slf: PyRefMut<'_, Self>, term: PyTbaTerm) -> PyRefMut<'_, Self> {
        slf.term = Some(term.into());
        slf
    }

    fn notional(mut slf: PyRefMut<'_, Self>, notional: f64) -> PyRefMut<'_, Self> {
        slf.notional = Some(notional);
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

    fn front_settlement_year(
        mut slf: PyRefMut<'_, Self>,
        front_settlement_year: i32,
    ) -> PyRefMut<'_, Self> {
        slf.front_settlement_year = Some(front_settlement_year);
        slf
    }

    fn front_settlement_month(
        mut slf: PyRefMut<'_, Self>,
        front_settlement_month: u8,
    ) -> PyRefMut<'_, Self> {
        slf.front_settlement_month = Some(front_settlement_month);
        slf
    }

    fn back_settlement_year(
        mut slf: PyRefMut<'_, Self>,
        back_settlement_year: i32,
    ) -> PyRefMut<'_, Self> {
        slf.back_settlement_year = Some(back_settlement_year);
        slf
    }

    fn back_settlement_month(
        mut slf: PyRefMut<'_, Self>,
        back_settlement_month: u8,
    ) -> PyRefMut<'_, Self> {
        slf.back_settlement_month = Some(back_settlement_month);
        slf
    }

    fn front_price(mut slf: PyRefMut<'_, Self>, front_price: f64) -> PyRefMut<'_, Self> {
        slf.front_price = Some(front_price);
        slf
    }

    fn back_price(mut slf: PyRefMut<'_, Self>, back_price: f64) -> PyRefMut<'_, Self> {
        slf.back_price = Some(back_price);
        slf
    }

    fn discount_curve_id(
        mut slf: PyRefMut<'_, Self>,
        discount_curve_id: String,
    ) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(discount_curve_id);
        slf
    }

    #[pyo3(signature = (trade_date=None))]
    fn trade_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        trade_date: Option<Bound<'py, PyAny>>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.trade_date = trade_date.map(|d| py_to_date(&d)).transpose()?;
        Ok(slf)
    }

    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyDollarRoll> {
        slf.ensure_ready()?;
        let ccy = slf.currency.unwrap();

        let mut builder = DollarRoll::builder()
            .id(slf.instrument_id.clone())
            .agency(slf.agency.unwrap())
            .coupon(slf.coupon.unwrap())
            .term(slf.term.unwrap())
            .notional(Money::new(slf.notional.unwrap(), ccy))
            .front_settlement_year(slf.front_settlement_year.unwrap())
            .front_settlement_month(slf.front_settlement_month.unwrap())
            .back_settlement_year(slf.back_settlement_year.unwrap())
            .back_settlement_month(slf.back_settlement_month.unwrap())
            .front_price(slf.front_price.unwrap())
            .back_price(slf.back_price.unwrap())
            .discount_curve_id(CurveId::new(slf.discount_curve_id.as_deref().unwrap()))
            .attributes(Attributes::new());

        if let Some(td) = slf.trade_date {
            builder = builder.trade_date_opt(Some(td));
        }

        let roll = builder
            .build()
            .map_err(|e| PyValueError::new_err(format!("{e}")))?;
        Ok(PyDollarRoll::new(roll))
    }
}

#[pymethods]
impl PyDollarRoll {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyDollarRollBuilder>> {
        let py = cls.py();
        let builder = PyDollarRollBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    /// Create an example dollar roll for testing.
    #[classmethod]
    fn example(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(DollarRoll::example())
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Agency program.
    #[getter]
    fn agency(&self) -> PyAgencyProgram {
        self.inner.agency.into()
    }

    /// Coupon rate.
    #[getter]
    fn coupon(&self) -> f64 {
        self.inner.coupon
    }

    /// Term.
    #[getter]
    fn term(&self) -> PyTbaTerm {
        self.inner.term.into()
    }

    /// Notional amount.
    #[getter]
    fn notional(&self) -> f64 {
        self.inner.notional.amount()
    }

    /// Front-month settlement year.
    #[getter]
    fn front_settlement_year(&self) -> i32 {
        self.inner.front_settlement_year
    }

    /// Front-month settlement month.
    #[getter]
    fn front_settlement_month(&self) -> u8 {
        self.inner.front_settlement_month
    }

    /// Back-month settlement year.
    #[getter]
    fn back_settlement_year(&self) -> i32 {
        self.inner.back_settlement_year
    }

    /// Back-month settlement month.
    #[getter]
    fn back_settlement_month(&self) -> u8 {
        self.inner.back_settlement_month
    }

    /// Front-month price (sell price).
    #[getter]
    fn front_price(&self) -> f64 {
        self.inner.front_price
    }

    /// Back-month price (buy price).
    #[getter]
    fn back_price(&self) -> f64 {
        self.inner.back_price
    }

    /// Get the drop (price difference between front and back month).
    fn drop_value(&self) -> f64 {
        self.inner.as_ref().drop()
    }

    /// Get the drop in 32nds.
    fn drop_32nds(&self) -> f64 {
        self.inner.drop_32nds()
    }

    /// Discount curve ID.
    #[getter]
    fn discount_curve_id(&self) -> &str {
        self.inner.discount_curve_id.as_str()
    }

    fn __repr__(&self) -> String {
        format!(
            "DollarRoll(id='{}', drop={:.3})",
            self.inner.id.as_str(),
            self.inner.as_ref().drop()
        )
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
    unsendable
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
            .deal_name(slf.deal_name.clone().unwrap().into())
            .agency(slf.agency.unwrap())
            .issue_date(slf.issue_date.unwrap())
            .waterfall(slf.waterfall.clone().unwrap())
            .reference_tranche_id(slf.reference_tranche_id.clone().unwrap())
            .discount_curve_id(CurveId::new(slf.discount_curve_id.as_deref().unwrap()))
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
    fn example(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(AgencyCmo::example())
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
    // Add classes to the parent module
    parent.add_class::<PyAgencyProgram>()?;
    parent.add_class::<PyPoolType>()?;
    parent.add_class::<PyTbaTerm>()?;
    parent.add_class::<PyCmoTrancheType>()?;
    parent.add_class::<PyAgencyMbsPassthrough>()?;
    parent.add_class::<PyAgencyMbsPassthroughBuilder>()?;
    parent.add_class::<PyAgencyTba>()?;
    parent.add_class::<PyAgencyTbaBuilder>()?;
    parent.add_class::<PyDollarRoll>()?;
    parent.add_class::<PyDollarRollBuilder>()?;
    parent.add_class::<PyPacCollar>()?;
    parent.add_class::<PyCmoTranche>()?;
    parent.add_class::<PyCmoWaterfall>()?;
    parent.add_class::<PyAgencyCmo>()?;
    parent.add_class::<PyAgencyCmoBuilder>()?;

    Ok(vec![
        "AgencyProgram",
        "PoolType",
        "TbaTerm",
        "CmoTrancheType",
        "AgencyMbsPassthrough",
        "AgencyMbsPassthroughBuilder",
        "AgencyTba",
        "AgencyTbaBuilder",
        "DollarRoll",
        "DollarRollBuilder",
        "PacCollar",
        "CmoTranche",
        "CmoWaterfall",
        "AgencyCmo",
        "AgencyCmoBuilder",
    ])
}
