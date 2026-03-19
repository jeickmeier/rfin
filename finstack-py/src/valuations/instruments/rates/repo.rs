use crate::core::common::args::{BusinessDayConventionArg, DayCountArg};
use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::PyMoney;
use crate::errors::{core_to_py, PyContext};
use crate::valuations::common::{to_optional_string, PyInstrumentType};
use finstack_core::dates::BusinessDayConvention;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::rates::repo::{
    CollateralSpec, CollateralType, Repo, RepoType,
};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRef, PyRefMut};
use std::fmt;
use std::sync::Arc;

fn parse_repo_type(label: Option<&str>) -> PyResult<RepoType> {
    match label {
        None => Ok(RepoType::Term),
        Some(s) => s.parse().map_err(|e: String| PyValueError::new_err(e)),
    }
}

fn parse_collateral_type(
    value: Option<&Bound<'_, PyAny>>,
    special_security_id: Option<&str>,
    special_rate_adjust_bp: Option<f64>,
) -> PyResult<CollateralType> {
    let Some(value) = value else {
        return Ok(CollateralType::General);
    };

    if value.is_none() {
        return Ok(CollateralType::General);
    }

    if let Ok(typed) = value.extract::<PyRef<'_, PyCollateralType>>() {
        return Ok(typed.inner.clone());
    }

    if let Ok(label) = value.extract::<&str>() {
        let normalized = crate::core::common::labels::normalize_label(label);
        return match normalized.as_str() {
            "general" | "gc" => Ok(CollateralType::General),
            "special" => Ok(CollateralType::Special {
                security_id: special_security_id.map(str::to_string).ok_or_else(|| {
                    PyValueError::new_err("special_security_id required for special collateral")
                })?,
                rate_adjustment_bp: special_rate_adjust_bp,
            }),
            other => Err(PyValueError::new_err(format!(
                "Unknown collateral type: '{}'. Valid: general, special",
                other
            ))),
        };
    }

    Err(PyTypeError::new_err(
        "collateral_type expects CollateralType, str, or None",
    ))
}

// ============================================================================
// RepoType wrapper
// ============================================================================

/// Repo type (Term, Open, or Overnight).
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "RepoType",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyRepoType {
    pub(crate) inner: RepoType,
}

impl PyRepoType {
    pub(crate) const fn new(inner: RepoType) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRepoType {
    #[classattr]
    const TERM: Self = Self::new(RepoType::Term);
    #[classattr]
    const OPEN: Self = Self::new(RepoType::Open);
    #[classattr]
    const OVERNIGHT: Self = Self::new(RepoType::Overnight);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        name.parse()
            .map(Self::new)
            .map_err(|e: String| PyValueError::new_err(e))
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("RepoType('{}')", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

impl From<PyRepoType> for RepoType {
    fn from(value: PyRepoType) -> Self {
        value.inner
    }
}

// ============================================================================
// RepoCollateral wrapper
// ============================================================================

/// Collateral classification (general or special) for repo collateral.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CollateralType",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyCollateralType {
    pub(crate) inner: CollateralType,
}

impl PyCollateralType {
    pub(crate) fn new(inner: CollateralType) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCollateralType {
    #[classattr]
    const GENERAL: Self = Self {
        inner: CollateralType::General,
    };

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        let normalized = crate::core::common::labels::normalize_label(name);
        match normalized.as_str() {
            "general" | "gc" => Ok(Self::new(CollateralType::General)),
            "special" => Err(PyValueError::new_err(
                "CollateralType::special() requires a security_id",
            )),
            other => Err(PyValueError::new_err(format!(
                "Unknown collateral type: '{}'. Valid: general, special",
                other
            ))),
        }
    }

    #[classmethod]
    #[pyo3(signature = (security_id, rate_adjustment_bp = None))]
    fn special(
        _cls: &Bound<'_, PyType>,
        security_id: &str,
        rate_adjustment_bp: Option<f64>,
    ) -> Self {
        Self::new(CollateralType::Special {
            security_id: security_id.to_string(),
            rate_adjustment_bp,
        })
    }

    #[getter]
    fn name(&self) -> &'static str {
        match &self.inner {
            CollateralType::General => "general",
            CollateralType::Special { .. } => "special",
            _ => "unknown",
        }
    }

    #[getter]
    fn security_id(&self) -> Option<String> {
        match &self.inner {
            CollateralType::General => None,
            CollateralType::Special { security_id, .. } => Some(security_id.clone()),
            _ => None,
        }
    }

    #[getter]
    fn rate_adjustment_bp(&self) -> Option<f64> {
        match &self.inner {
            CollateralType::General => None,
            CollateralType::Special {
                rate_adjustment_bp, ..
            } => *rate_adjustment_bp,
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            CollateralType::General => "CollateralType('general')".to_string(),
            CollateralType::Special {
                security_id,
                rate_adjustment_bp,
            } => format!(
                "CollateralType.special(security_id='{}', rate_adjustment_bp={:?})",
                security_id, rate_adjustment_bp
            ),
            _ => "CollateralType('unknown')".to_string(),
        }
    }

    fn __str__(&self) -> &'static str {
        self.name()
    }
}

/// Collateral specification helper mirroring `CollateralSpec`.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "RepoCollateral",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyRepoCollateral {
    pub(crate) inner: CollateralSpec,
}

#[pymethods]
impl PyRepoCollateral {
    #[new]
    #[pyo3(
        signature = (
            instrument_id,
            quantity,
            market_value_id,
            *,
            collateral_type = None,
            special_security_id = None,
            special_rate_adjust_bp = None
        ),
        text_signature = "(instrument_id, quantity, market_value_id, *, collateral_type=None, special_security_id=None, special_rate_adjust_bp=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn new(
        instrument_id: &str,
        quantity: f64,
        market_value_id: &str,
        collateral_type: Option<Bound<'_, PyAny>>,
        special_security_id: Option<&str>,
        special_rate_adjust_bp: Option<f64>,
    ) -> PyResult<Self> {
        let ctype = parse_collateral_type(
            collateral_type.as_ref(),
            special_security_id,
            special_rate_adjust_bp,
        )?;
        let spec = CollateralSpec {
            collateral_type: ctype,
            instrument_id: instrument_id.to_string(),
            quantity,
            market_value_id: market_value_id.to_string(),
        };
        Ok(Self { inner: spec })
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        &self.inner.instrument_id
    }

    #[getter]
    fn quantity(&self) -> f64 {
        self.inner.quantity
    }

    #[getter]
    fn market_value_id(&self) -> &str {
        &self.inner.market_value_id
    }

    #[getter]
    fn collateral_type(&self) -> PyCollateralType {
        PyCollateralType::new(self.inner.collateral_type.clone())
    }
}

/// Repo wrapper exposing a convenience constructor.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "Repo",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyRepo {
    pub(crate) inner: Arc<Repo>,
}

impl PyRepo {
    pub(crate) fn new(inner: Repo) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "RepoBuilder",
    unsendable
)]
pub struct PyRepoBuilder {
    instrument_id: InstrumentId,
    pending_cash_amount: Option<f64>,
    pending_currency: Option<finstack_core::currency::Currency>,
    collateral: Option<CollateralSpec>,
    repo_rate: Option<f64>,
    start_date: Option<time::Date>,
    maturity: Option<time::Date>,
    discount_curve_id: Option<CurveId>,
    repo_type: RepoType,
    haircut: f64,
    day_count: finstack_core::dates::DayCount,
    business_day_convention: BusinessDayConvention,
    calendar: Option<String>,
    triparty: bool,
}

impl PyRepoBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            pending_cash_amount: None,
            pending_currency: None,
            collateral: None,
            repo_rate: None,
            start_date: None,
            maturity: None,
            discount_curve_id: None,
            repo_type: RepoType::Term,
            haircut: 0.02,
            day_count: finstack_core::dates::DayCount::Act360,
            business_day_convention: BusinessDayConvention::Following,
            calendar: None,
            triparty: false,
        }
    }

    fn cash_money(&self) -> Option<Money> {
        match (self.pending_cash_amount, self.pending_currency) {
            (Some(amount), Some(currency)) => Some(Money::new(amount, currency)),
            _ => None,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.cash_money().is_none() {
            return Err(PyValueError::new_err(
                "Both cash_amount() and currency() must be provided before build().",
            ));
        }
        if self.collateral.is_none() {
            return Err(PyValueError::new_err("collateral() is required."));
        }
        if self.repo_rate.is_none() {
            return Err(PyValueError::new_err("repo_rate() is required."));
        }
        if self.start_date.is_none() {
            return Err(PyValueError::new_err("start_date() is required."));
        }
        if self.maturity.is_none() {
            return Err(PyValueError::new_err("maturity() is required."));
        }
        if self.discount_curve_id.is_none() {
            return Err(PyValueError::new_err("discount_curve() is required."));
        }
        Ok(())
    }

    fn parse_currency(value: &Bound<'_, PyAny>) -> PyResult<finstack_core::currency::Currency> {
        if let Ok(py_ccy) = value.extract::<PyRef<PyCurrency>>() {
            Ok(py_ccy.inner)
        } else if let Ok(code) = value.extract::<&str>() {
            code.parse::<finstack_core::currency::Currency>()
                .map_err(|_| PyValueError::new_err("Invalid currency code"))
        } else {
            Err(PyTypeError::new_err("currency() expects str or Currency"))
        }
    }
}

#[pymethods]
impl PyRepoBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    #[pyo3(text_signature = "($self, amount)")]
    fn cash(mut slf: PyRefMut<'_, Self>, amount: f64) -> PyResult<PyRefMut<'_, Self>> {
        if amount <= 0.0 {
            return Err(PyValueError::new_err("cash amount must be positive"));
        }
        slf.pending_cash_amount = Some(amount);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, currency)")]
    fn currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        currency: &Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.pending_currency = Some(Self::parse_currency(currency)?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, money)")]
    fn cash_amount<'py>(
        mut slf: PyRefMut<'py, Self>,
        money: PyRef<'py, PyMoney>,
    ) -> PyRefMut<'py, Self> {
        slf.pending_cash_amount = Some(money.inner.amount());
        slf.pending_currency = Some(money.inner.currency());
        slf
    }

    #[pyo3(text_signature = "($self, collateral)")]
    fn collateral(mut slf: PyRefMut<'_, Self>, collateral: PyRepoCollateral) -> PyRefMut<'_, Self> {
        slf.collateral = Some(collateral.inner);
        slf
    }

    #[pyo3(text_signature = "($self, repo_rate)")]
    fn repo_rate(mut slf: PyRefMut<'_, Self>, repo_rate: f64) -> PyResult<PyRefMut<'_, Self>> {
        if repo_rate < 0.0 {
            return Err(PyValueError::new_err("repo_rate must be non-negative"));
        }
        slf.repo_rate = Some(repo_rate);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, start_date)")]
    fn start_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        start_date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.start_date = Some(py_to_date(&start_date).context("start_date")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, maturity)")]
    fn maturity<'py>(
        mut slf: PyRefMut<'py, Self>,
        maturity: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.maturity = Some(py_to_date(&maturity).context("maturity")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn discount_curve(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, repo_type)")]
    fn repo_type<'py>(
        mut slf: PyRefMut<'py, Self>,
        repo_type: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        if let Ok(typed) = repo_type.extract::<PyRef<PyRepoType>>() {
            slf.repo_type = typed.inner;
        } else if let Ok(name) = repo_type.extract::<String>() {
            slf.repo_type = parse_repo_type(Some(name.as_str()))?;
        } else if repo_type.is_none() {
            slf.repo_type = RepoType::Term;
        } else {
            return Err(PyTypeError::new_err("repo_type() expects RepoType or str"));
        }
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, haircut)")]
    fn haircut(mut slf: PyRefMut<'_, Self>, haircut: f64) -> PyRefMut<'_, Self> {
        slf.haircut = haircut;
        slf
    }

    #[pyo3(text_signature = "($self, day_count)")]
    fn day_count<'py>(
        mut slf: PyRefMut<'py, Self>,
        day_count: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let DayCountArg(value) = day_count.extract().context("day_count")?;
        slf.day_count = value;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, business_day_convention)")]
    fn business_day_convention<'py>(
        mut slf: PyRefMut<'py, Self>,
        business_day_convention: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let BusinessDayConventionArg(value) = business_day_convention
            .extract()
            .context("business_day_convention")?;
        slf.business_day_convention = value;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, calendar=None)", signature = (calendar=None))]
    fn calendar(mut slf: PyRefMut<'_, Self>, calendar: Option<String>) -> PyRefMut<'_, Self> {
        slf.calendar = calendar;
        slf
    }

    #[pyo3(text_signature = "($self, triparty)")]
    fn triparty(mut slf: PyRefMut<'_, Self>, triparty: bool) -> PyRefMut<'_, Self> {
        slf.triparty = triparty;
        slf
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyRepo> {
        slf.ensure_ready()?;

        let cash = slf.cash_money().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "RepoBuilder internal error: missing cash amount after validation",
            )
        })?;
        let collateral = slf.collateral.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "RepoBuilder internal error: missing collateral after validation",
            )
        })?;
        let repo_rate = slf.repo_rate.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "RepoBuilder internal error: missing repo_rate after validation",
            )
        })?;
        let start = slf.start_date.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "RepoBuilder internal error: missing start_date after validation",
            )
        })?;
        let maturity = slf.maturity.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "RepoBuilder internal error: missing maturity after validation",
            )
        })?;
        let discount = slf.discount_curve_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "RepoBuilder internal error: missing discount curve after validation",
            )
        })?;

        Repo::builder()
            .id(slf.instrument_id.clone())
            .cash_amount(cash)
            .collateral(collateral)
            .repo_rate(rust_decimal::Decimal::try_from(repo_rate).map_err(|_| {
                PyValueError::new_err(format!("Cannot convert {} to decimal", repo_rate))
            })?)
            .start_date(start)
            .maturity(maturity)
            .haircut(slf.haircut)
            .repo_type(slf.repo_type)
            .triparty(slf.triparty)
            .day_count(slf.day_count)
            .bdc(slf.business_day_convention)
            .calendar_id_opt(to_optional_string(slf.calendar.as_deref()).map(Into::into))
            .discount_curve_id(discount)
            .build()
            .map(PyRepo::new)
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        "RepoBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyRepo {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(cls: &Bound<'py, PyType>, instrument_id: &str) -> PyResult<Py<PyRepoBuilder>> {
        let py = cls.py();
        let builder = PyRepoBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    /// Create an overnight repo.
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    fn overnight(_cls: &Bound<'_, PyType>, instrument_id: &str) -> PyResult<Py<PyRepoBuilder>> {
        let py = _cls.py();
        let mut builder = PyRepoBuilder::new_with_id(InstrumentId::new(instrument_id));
        builder.repo_type = RepoType::Overnight;
        Py::new(py, builder)
    }

    /// Create a term repo builder.
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    fn term(_cls: &Bound<'_, PyType>, instrument_id: &str) -> PyResult<Py<PyRepoBuilder>> {
        let py = _cls.py();
        let mut builder = PyRepoBuilder::new_with_id(InstrumentId::new(instrument_id));
        builder.repo_type = RepoType::Term;
        Py::new(py, builder)
    }

    /// Create an open repo builder.
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    fn open(_cls: &Bound<'_, PyType>, instrument_id: &str) -> PyResult<Py<PyRepoBuilder>> {
        let py = _cls.py();
        let mut builder = PyRepoBuilder::new_with_id(InstrumentId::new(instrument_id));
        builder.repo_type = RepoType::Open;
        Py::new(py, builder)
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn cash_amount(&self) -> PyMoney {
        PyMoney::new(self.inner.cash_amount)
    }

    #[getter]
    fn repo_rate(&self) -> f64 {
        rust_decimal::prelude::ToPrimitive::to_f64(&self.inner.repo_rate).unwrap_or_default()
    }

    #[getter]
    fn start_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.start_date)
    }

    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.maturity)
    }

    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[getter]
    fn effective_rate(&self) -> f64 {
        self.inner.effective_rate()
    }

    #[getter]
    fn interest_amount(&self) -> PyResult<PyMoney> {
        self.inner
            .interest_amount()
            .map(PyMoney::new)
            .map_err(crate::errors::core_to_py)
    }

    #[getter]
    fn total_repayment(&self) -> PyResult<PyMoney> {
        self.inner
            .total_repayment()
            .map(PyMoney::new)
            .map_err(crate::errors::core_to_py)
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::Repo)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "Repo(id='{}', rate={:.4})",
            self.inner.id, self.inner.repo_rate
        ))
    }
}

impl fmt::Display for PyRepo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Repo({}, rate={:.4})",
            self.inner.id, self.inner.repo_rate
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyRepoType>()?;
    module.add_class::<PyCollateralType>()?;
    module.add_class::<PyRepoCollateral>()?;
    module.add_class::<PyRepo>()?;
    module.add_class::<PyRepoBuilder>()?;
    Ok(vec![
        "RepoType",
        "CollateralType",
        "RepoCollateral",
        "Repo",
        "RepoBuilder",
    ])
}
