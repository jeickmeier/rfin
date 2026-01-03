//! Python bindings for Agency MBS instruments.
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
use std::sync::Arc;

// =============================================================================
// Agency Program Enum
// =============================================================================

/// Agency program (FNMA, FHLMC, GNMA).
#[pyclass(module = "finstack.valuations.instruments", name = "AgencyProgram", eq)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PyAgencyProgram {
    /// Federal National Mortgage Association (Fannie Mae).
    Fnma,
    /// Federal Home Loan Mortgage Corporation (Freddie Mac).
    Fhlmc,
    /// Government National Mortgage Association (Ginnie Mae).
    Gnma,
}

impl From<PyAgencyProgram> for AgencyProgram {
    fn from(py: PyAgencyProgram) -> Self {
        match py {
            PyAgencyProgram::Fnma => AgencyProgram::Fnma,
            PyAgencyProgram::Fhlmc => AgencyProgram::Fhlmc,
            PyAgencyProgram::Gnma => AgencyProgram::Gnma,
        }
    }
}

impl From<AgencyProgram> for PyAgencyProgram {
    fn from(rust: AgencyProgram) -> Self {
        match rust {
            AgencyProgram::Fnma => PyAgencyProgram::Fnma,
            AgencyProgram::Fhlmc => PyAgencyProgram::Fhlmc,
            AgencyProgram::Gnma => PyAgencyProgram::Gnma,
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
#[pyclass(module = "finstack.valuations.instruments", name = "PoolType", eq)]
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
#[pyclass(module = "finstack.valuations.instruments", name = "TbaTerm", eq)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    eq
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
///     >>> mbs = AgencyMbsPassthrough.create(
///     ...     "FN-MA1234",
///     ...     pool_id="MA1234",
///     ...     agency=AgencyProgram.Fnma,
///     ...     original_face=1_000_000.0,
///     ...     current_face=950_000.0,
///     ...     currency="USD",
///     ...     wac=0.045,
///     ...     pass_through_rate=0.04,
///     ...     wam=348,
///     ...     issue_date=Date(2022, 1, 1),
///     ...     maturity_date=Date(2052, 1, 1),
///     ...     discount_curve_id="USD-OIS"
///     ... )
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "AgencyMbsPassthrough",
    frozen
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

#[pymethods]
impl PyAgencyMbsPassthrough {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, *, pool_id, agency, original_face, current_face, currency, wac, pass_through_rate, wam, issue_date, maturity_date, discount_curve_id, current_factor=None, servicing_fee_rate=0.0025, guarantee_fee_rate=0.0025, pool_type=None, psa_speed=1.0, day_count=None)"
    )]
    #[pyo3(
        signature = (
            instrument_id,
            *,
            pool_id,
            agency,
            original_face,
            current_face,
            currency,
            wac,
            pass_through_rate,
            wam,
            issue_date,
            maturity_date,
            discount_curve_id,
            current_factor = None,
            servicing_fee_rate = 0.0025,
            guarantee_fee_rate = 0.0025,
            pool_type = None,
            psa_speed = 1.0,
            day_count = None
        )
    )]
    /// Create an agency MBS passthrough security.
    ///
    /// Args:
    ///     instrument_id: Unique identifier for this instrument.
    ///     pool_id: Pool identifier (CUSIP or internal ID).
    ///     agency: Agency program (Fnma, Fhlmc, Gnma).
    ///     original_face: Original face value.
    ///     current_face: Current face value.
    ///     currency: Currency (e.g., "USD").
    ///     wac: Weighted average coupon of underlying loans.
    ///     pass_through_rate: Net coupon paid to investors.
    ///     wam: Weighted average maturity in months.
    ///     issue_date: Pool issue date.
    ///     maturity_date: Pool maturity date.
    ///     discount_curve_id: Discount curve ID.
    ///     current_factor: Current pool factor (defaults to current_face/original_face).
    ///     servicing_fee_rate: Servicing fee rate (default 25 bps).
    ///     guarantee_fee_rate: Guarantee fee rate (default 25 bps).
    ///     pool_type: Pool type classification.
    ///     psa_speed: PSA prepayment speed (default 1.0 = 100% PSA).
    ///     day_count: Day count convention (default Thirty360).
    ///
    /// Returns:
    ///     AgencyMbsPassthrough: Configured MBS passthrough instrument.
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        pool_id: &str,
        agency: PyAgencyProgram,
        original_face: f64,
        current_face: f64,
        currency: Bound<'_, PyAny>,
        wac: f64,
        pass_through_rate: f64,
        wam: u32,
        issue_date: Bound<'_, PyAny>,
        maturity_date: Bound<'_, PyAny>,
        discount_curve_id: &str,
        current_factor: Option<f64>,
        servicing_fee_rate: f64,
        guarantee_fee_rate: f64,
        pool_type: Option<PyPoolType>,
        psa_speed: f64,
        day_count: Option<PyDayCount>,
    ) -> PyResult<Self> {
        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        let issue = py_to_date(&issue_date).context("issue_date")?;
        let maturity = py_to_date(&maturity_date).context("maturity_date")?;

        let factor = current_factor.unwrap_or(current_face / original_face);
        let pool = pool_type.map(PoolType::from).unwrap_or(PoolType::Generic);
        let dc = day_count.map(|d| d.inner).unwrap_or(DayCount::Thirty360);
        let prepay = PrepaymentModelSpec::psa(psa_speed);

        let mbs = AgencyMbsPassthrough::builder()
            .id(id)
            .pool_id(pool_id.to_string())
            .agency(agency.into())
            .pool_type(pool)
            .original_face(Money::new(original_face, ccy))
            .current_face(Money::new(current_face, ccy))
            .current_factor(factor)
            .wac(wac)
            .pass_through_rate(pass_through_rate)
            .servicing_fee_rate(servicing_fee_rate)
            .guarantee_fee_rate(guarantee_fee_rate)
            .wam(wam)
            .issue_date(issue)
            .maturity_date(maturity)
            .prepayment_model(prepay)
            .discount_curve_id(CurveId::new(discount_curve_id))
            .day_count(dc)
            .attributes(Attributes::new())
            .build()
            .map_err(|e| PyValueError::new_err(format!("{}", e)))?;

        Ok(Self::new(mbs))
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
        date_to_py(py, self.inner.maturity_date)
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
///     >>> tba = AgencyTba.create(
///     ...     "FN30-4.0-202403",
///     ...     agency=AgencyProgram.Fnma,
///     ...     coupon=0.04,
///     ...     term=TbaTerm.ThirtyYear,
///     ...     settlement_year=2024,
///     ...     settlement_month=3,
///     ...     notional=10_000_000.0,
///     ...     currency="USD",
///     ...     trade_price=98.5,
///     ...     discount_curve_id="USD-OIS"
///     ... )
#[pyclass(module = "finstack.valuations.instruments", name = "AgencyTba", frozen)]
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

#[pymethods]
impl PyAgencyTba {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, *, agency, coupon, term, settlement_year, settlement_month, notional, currency, trade_price, discount_curve_id, trade_date=None)"
    )]
    #[pyo3(
        signature = (
            instrument_id,
            *,
            agency,
            coupon,
            term,
            settlement_year,
            settlement_month,
            notional,
            currency,
            trade_price,
            discount_curve_id,
            trade_date = None
        )
    )]
    /// Create a TBA forward contract.
    ///
    /// Args:
    ///     instrument_id: Unique identifier for this instrument.
    ///     agency: Agency program (Fnma, Fhlmc, Gnma).
    ///     coupon: Pass-through coupon rate (e.g., 0.04 for 4%).
    ///     term: Original loan term (ThirtyYear, FifteenYear, etc.).
    ///     settlement_year: Settlement year.
    ///     settlement_month: Settlement month (1-12).
    ///     notional: Trade notional (par amount).
    ///     currency: Currency (e.g., "USD").
    ///     trade_price: Trade price (percentage of par).
    ///     discount_curve_id: Discount curve ID.
    ///     trade_date: Optional trade date.
    ///
    /// Returns:
    ///     AgencyTba: Configured TBA forward contract.
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        agency: PyAgencyProgram,
        coupon: f64,
        term: PyTbaTerm,
        settlement_year: i32,
        settlement_month: u8,
        notional: f64,
        currency: Bound<'_, PyAny>,
        trade_price: f64,
        discount_curve_id: &str,
        trade_date: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        let trade = trade_date.map(|d| py_to_date(&d)).transpose()?;

        let mut builder = AgencyTba::builder()
            .id(id)
            .agency(agency.into())
            .coupon(coupon)
            .term(term.into())
            .settlement_year(settlement_year)
            .settlement_month(settlement_month)
            .notional(Money::new(notional, ccy))
            .trade_price(trade_price)
            .discount_curve_id(CurveId::new(discount_curve_id))
            .attributes(Attributes::new());

        if let Some(td) = trade {
            builder = builder.trade_date_opt(Some(td));
        }

        let tba = builder
            .build()
            .map_err(|e| PyValueError::new_err(format!("{}", e)))?;

        Ok(Self::new(tba))
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
///     >>> roll = DollarRoll.create(
///     ...     "FN30-4.0-ROLL-0324-0424",
///     ...     agency=AgencyProgram.Fnma,
///     ...     coupon=0.04,
///     ...     term=TbaTerm.ThirtyYear,
///     ...     notional=10_000_000.0,
///     ...     currency="USD",
///     ...     front_settlement_year=2024,
///     ...     front_settlement_month=3,
///     ...     back_settlement_year=2024,
///     ...     back_settlement_month=4,
///     ...     front_price=98.5,
///     ...     back_price=98.0,
///     ...     discount_curve_id="USD-OIS"
///     ... )
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "DollarRoll",
    frozen
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

#[pymethods]
impl PyDollarRoll {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, *, agency, coupon, term, notional, currency, front_settlement_year, front_settlement_month, back_settlement_year, back_settlement_month, front_price, back_price, discount_curve_id, trade_date=None)"
    )]
    #[pyo3(
        signature = (
            instrument_id,
            *,
            agency,
            coupon,
            term,
            notional,
            currency,
            front_settlement_year,
            front_settlement_month,
            back_settlement_year,
            back_settlement_month,
            front_price,
            back_price,
            discount_curve_id,
            trade_date = None
        )
    )]
    /// Create a dollar roll position.
    ///
    /// Args:
    ///     instrument_id: Unique identifier for this instrument.
    ///     agency: Agency program (Fnma, Fhlmc, Gnma).
    ///     coupon: Pass-through coupon rate.
    ///     term: Original loan term.
    ///     notional: Trade notional (par amount).
    ///     currency: Currency.
    ///     front_settlement_year: Front-month settlement year.
    ///     front_settlement_month: Front-month settlement month (1-12).
    ///     back_settlement_year: Back-month settlement year.
    ///     back_settlement_month: Back-month settlement month (1-12).
    ///     front_price: Front-month price (sell price).
    ///     back_price: Back-month price (buy price).
    ///     discount_curve_id: Discount curve ID.
    ///     trade_date: Optional trade date.
    ///
    /// Returns:
    ///     DollarRoll: Configured dollar roll instrument.
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        agency: PyAgencyProgram,
        coupon: f64,
        term: PyTbaTerm,
        notional: f64,
        currency: Bound<'_, PyAny>,
        front_settlement_year: i32,
        front_settlement_month: u8,
        back_settlement_year: i32,
        back_settlement_month: u8,
        front_price: f64,
        back_price: f64,
        discount_curve_id: &str,
        trade_date: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        let trade = trade_date.map(|d| py_to_date(&d)).transpose()?;

        let mut builder = DollarRoll::builder()
            .id(id)
            .agency(agency.into())
            .coupon(coupon)
            .term(term.into())
            .notional(Money::new(notional, ccy))
            .front_settlement_year(front_settlement_year)
            .front_settlement_month(front_settlement_month)
            .back_settlement_year(back_settlement_year)
            .back_settlement_month(back_settlement_month)
            .front_price(front_price)
            .back_price(back_price)
            .discount_curve_id(CurveId::new(discount_curve_id))
            .attributes(Attributes::new());

        if let Some(td) = trade {
            builder = builder.trade_date_opt(Some(td));
        }

        let roll = builder
            .build()
            .map_err(|e| PyValueError::new_err(format!("{}", e)))?;

        Ok(Self::new(roll))
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
#[pyclass(module = "finstack.valuations.instruments", name = "PacCollar", frozen)]
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
    frozen
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
    frozen
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
///     >>> cmo = AgencyCmo.create(
///     ...     "FNR-2024-1-A",
///     ...     deal_name="FNR 2024-1",
///     ...     agency=AgencyProgram.Fnma,
///     ...     issue_date=Date(2024, 1, 1),
///     ...     waterfall=waterfall,
///     ...     reference_tranche_id="A",
///     ...     discount_curve_id="USD-OIS"
///     ... )
#[pyclass(module = "finstack.valuations.instruments", name = "AgencyCmo", frozen)]
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

#[pymethods]
impl PyAgencyCmo {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, *, deal_name, agency, issue_date, waterfall, reference_tranche_id, discount_curve_id, collateral_wac=None, collateral_wam=None)"
    )]
    #[pyo3(
        signature = (
            instrument_id,
            *,
            deal_name,
            agency,
            issue_date,
            waterfall,
            reference_tranche_id,
            discount_curve_id,
            collateral_wac = None,
            collateral_wam = None
        )
    )]
    /// Create an agency CMO instrument.
    ///
    /// Args:
    ///     instrument_id: Unique identifier for this instrument.
    ///     deal_name: Deal/series name.
    ///     agency: Agency program.
    ///     issue_date: Deal issue date.
    ///     waterfall: Waterfall structure with tranches.
    ///     reference_tranche_id: ID of the tranche being valued.
    ///     discount_curve_id: Discount curve ID.
    ///     collateral_wac: Optional collateral weighted average coupon.
    ///     collateral_wam: Optional collateral weighted average maturity (months).
    ///
    /// Returns:
    ///     AgencyCmo: Configured CMO instrument.
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        deal_name: &str,
        agency: PyAgencyProgram,
        issue_date: Bound<'_, PyAny>,
        waterfall: &PyCmoWaterfall,
        reference_tranche_id: &str,
        discount_curve_id: &str,
        collateral_wac: Option<f64>,
        collateral_wam: Option<u32>,
    ) -> PyResult<Self> {
        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let issue = py_to_date(&issue_date).context("issue_date")?;

        let mut builder = AgencyCmo::builder()
            .id(id)
            .deal_name(deal_name.to_string())
            .agency(agency.into())
            .issue_date(issue)
            .waterfall(waterfall.inner.clone())
            .reference_tranche_id(reference_tranche_id.to_string())
            .discount_curve_id(CurveId::new(discount_curve_id))
            .attributes(Attributes::new());

        if let Some(wac) = collateral_wac {
            builder = builder.collateral_wac_opt(Some(wac));
        }
        if let Some(wam) = collateral_wam {
            builder = builder.collateral_wam_opt(Some(wam));
        }

        let cmo = builder
            .build()
            .map_err(|e| PyValueError::new_err(format!("{}", e)))?;

        Ok(Self::new(cmo))
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
    parent.add_class::<PyAgencyTba>()?;
    parent.add_class::<PyDollarRoll>()?;
    parent.add_class::<PyPacCollar>()?;
    parent.add_class::<PyCmoTranche>()?;
    parent.add_class::<PyCmoWaterfall>()?;
    parent.add_class::<PyAgencyCmo>()?;

    Ok(vec![
        "AgencyProgram",
        "PoolType",
        "TbaTerm",
        "CmoTrancheType",
        "AgencyMbsPassthrough",
        "AgencyTba",
        "DollarRoll",
        "PacCollar",
        "CmoTranche",
        "CmoWaterfall",
        "AgencyCmo",
    ])
}
