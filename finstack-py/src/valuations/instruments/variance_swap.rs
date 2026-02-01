use crate::core::common::labels::normalize_label;
use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::market_data::PyMarketContext;
use crate::core::money::PyMoney;
use crate::errors::{core_to_py, PyContext};
use crate::valuations::common::PyInstrumentType;
use finstack_core::dates::Tenor;
use finstack_core::math::stats::RealizedVarMethod;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::equity::variance_swap::{PayReceive, VarianceSwap};
use finstack_valuations::instruments::Attributes;
use finstack_valuations::prelude::Instrument;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyModule, PyType};
use pyo3::{Bound, FromPyObject, Py, PyRef, PyRefMut};
use std::fmt;
use std::sync::Arc;

fn method_label(method: RealizedVarMethod) -> &'static str {
    match method {
        RealizedVarMethod::CloseToClose => "close_to_close",
        RealizedVarMethod::Parkinson => "parkinson",
        RealizedVarMethod::GarmanKlass => "garman_klass",
        RealizedVarMethod::RogersSatchell => "rogers_satchell",
        RealizedVarMethod::YangZhang => "yang_zhang",
    }
}

/// Pay/receive wrapper for variance swap payoffs.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "VarianceDirection",
    frozen
)]
#[derive(Clone, Copy, Debug)]
pub struct PyPayReceive {
    pub(crate) inner: PayReceive,
}

impl PyPayReceive {
    const fn new(inner: PayReceive) -> Self {
        Self { inner }
    }
}

#[derive(Clone, Copy, Debug)]
struct PayReceiveArg(PyPayReceive);

impl<'py> FromPyObject<'py> for PayReceiveArg {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(existing) = obj.extract::<PyRef<'py, PyPayReceive>>() {
            return Ok(PayReceiveArg(*existing));
        }

        if let Ok(label) = obj.extract::<&str>() {
            let normalized = normalize_label(label);
            let direction = match normalized.as_str() {
                "pay" | "payer" | "short" => PayReceive::Pay,
                "receive" | "receiver" | "long" => PayReceive::Receive,
                other => {
                    return Err(PyValueError::new_err(format!(
                        "Unknown variance direction: {other}"
                    )))
                }
            };
            return Ok(PayReceiveArg(PyPayReceive::new(direction)));
        }

        Err(PyTypeError::new_err(
            "Expected VarianceDirection or string identifier",
        ))
    }
}

#[pymethods]
impl PyPayReceive {
    #[classattr]
    const PAY: Self = Self::new(PayReceive::Pay);
    #[classattr]
    const RECEIVE: Self = Self::new(PayReceive::Receive);

    fn __repr__(&self) -> &'static str {
        match self.inner {
            PayReceive::Pay => "VarianceDirection.PAY",
            PayReceive::Receive => "VarianceDirection.RECEIVE",
        }
    }

    fn __str__(&self) -> &'static str {
        self.__repr__()
    }
}

/// Realized variance calculation method wrapper.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "RealizedVarianceMethod",
    frozen
)]
#[derive(Clone, Copy, Debug)]
pub struct PyRealizedVarMethod {
    pub(crate) inner: RealizedVarMethod,
}

impl PyRealizedVarMethod {
    const fn new(inner: RealizedVarMethod) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRealizedVarMethod {
    #[classattr]
    const CLOSE_TO_CLOSE: Self = Self::new(RealizedVarMethod::CloseToClose);
    #[classattr]
    const PARKINSON: Self = Self::new(RealizedVarMethod::Parkinson);
    #[classattr]
    const GARMAN_KLASS: Self = Self::new(RealizedVarMethod::GarmanKlass);
    #[classattr]
    const ROGERS_SATCHELL: Self = Self::new(RealizedVarMethod::RogersSatchell);
    #[classattr]
    const YANG_ZHANG: Self = Self::new(RealizedVarMethod::YangZhang);

    fn __repr__(&self) -> &'static str {
        match self.inner {
            RealizedVarMethod::CloseToClose => "RealizedVarianceMethod.CLOSE_TO_CLOSE",
            RealizedVarMethod::Parkinson => "RealizedVarianceMethod.PARKINSON",
            RealizedVarMethod::GarmanKlass => "RealizedVarianceMethod.GARMAN_KLASS",
            RealizedVarMethod::RogersSatchell => "RealizedVarianceMethod.ROGERS_SATCHELL",
            RealizedVarMethod::YangZhang => "RealizedVarianceMethod.YANG_ZHANG",
        }
    }

    fn __str__(&self) -> &'static str {
        self.__repr__()
    }
}

/// Variance swap wrapper.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "VarianceSwap",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyVarianceSwap {
    pub(crate) inner: Arc<VarianceSwap>,
}

impl PyVarianceSwap {
    pub(crate) fn new(inner: VarianceSwap) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "VarianceSwapBuilder",
    unsendable
)]
pub struct PyVarianceSwapBuilder {
    instrument_id: InstrumentId,
    underlying_id: Option<String>,
    pending_notional_amount: Option<f64>,
    pending_currency: Option<finstack_core::currency::Currency>,
    strike_variance: Option<f64>,
    start_date: Option<time::Date>,
    maturity: Option<time::Date>,
    discount_curve_id: Option<CurveId>,
    observation_frequency: Option<Tenor>,
    realized_method: RealizedVarMethod,
    side: PayReceive,
    day_count: finstack_core::dates::DayCount,
}

impl PyVarianceSwapBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            underlying_id: None,
            pending_notional_amount: None,
            pending_currency: None,
            strike_variance: None,
            start_date: None,
            maturity: None,
            discount_curve_id: None,
            observation_frequency: None,
            realized_method: RealizedVarMethod::CloseToClose,
            side: PayReceive::Receive,
            day_count: finstack_core::dates::DayCount::Act365F,
        }
    }

    fn notional_money(&self) -> Option<Money> {
        match (self.pending_notional_amount, self.pending_currency) {
            (Some(amount), Some(currency)) => Some(Money::new(amount, currency)),
            _ => None,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.underlying_id.as_deref().unwrap_or("").is_empty() {
            return Err(PyValueError::new_err("underlying_id() is required."));
        }
        if self.notional_money().is_none() {
            return Err(PyValueError::new_err(
                "Both notional() and currency() must be provided before build().",
            ));
        }
        if self.strike_variance.is_none() {
            return Err(PyValueError::new_err("strike_variance() is required."));
        }
        if self.start_date.is_none() {
            return Err(PyValueError::new_err("start_date() is required."));
        }
        if self.maturity.is_none() {
            return Err(PyValueError::new_err("maturity() is required."));
        }
        if self.discount_curve_id.is_none() {
            return Err(PyValueError::new_err("disc_id() is required."));
        }
        if self.observation_frequency.is_none() {
            return Err(PyValueError::new_err(
                "observation_frequency() is required.",
            ));
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
impl PyVarianceSwapBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    #[pyo3(text_signature = "($self, underlying_id)")]
    fn underlying_id(mut slf: PyRefMut<'_, Self>, underlying_id: String) -> PyRefMut<'_, Self> {
        slf.underlying_id = Some(underlying_id);
        slf
    }

    #[pyo3(text_signature = "($self, amount)")]
    fn notional(mut slf: PyRefMut<'_, Self>, amount: f64) -> PyResult<PyRefMut<'_, Self>> {
        if amount <= 0.0 {
            return Err(PyValueError::new_err("notional must be positive"));
        }
        slf.pending_notional_amount = Some(amount);
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
    fn money<'py>(mut slf: PyRefMut<'py, Self>, money: PyRef<'py, PyMoney>) -> PyRefMut<'py, Self> {
        slf.pending_notional_amount = Some(money.inner.amount());
        slf.pending_currency = Some(money.inner.currency());
        slf
    }

    #[pyo3(text_signature = "($self, strike_variance)")]
    fn strike_variance(
        mut slf: PyRefMut<'_, Self>,
        strike_variance: f64,
    ) -> PyResult<PyRefMut<'_, Self>> {
        if strike_variance < 0.0 {
            return Err(PyValueError::new_err(
                "Strike variance must be non-negative",
            ));
        }
        slf.strike_variance = Some(strike_variance);
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
    fn disc_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, observation_frequency)")]
    fn observation_frequency(
        mut slf: PyRefMut<'_, Self>,
        observation_frequency: crate::core::dates::schedule::PyFrequency,
    ) -> PyRefMut<'_, Self> {
        slf.observation_frequency = Some(observation_frequency.inner);
        slf
    }

    #[pyo3(text_signature = "($self, realized_method)")]
    fn realized_method(
        mut slf: PyRefMut<'_, Self>,
        realized_method: Option<PyRealizedVarMethod>,
    ) -> PyRefMut<'_, Self> {
        if let Some(m) = realized_method {
            slf.realized_method = m.inner;
        }
        slf
    }

    #[pyo3(text_signature = "($self, side)")]
    fn side(mut slf: PyRefMut<'_, Self>, side: Option<PayReceiveArg>) -> PyRefMut<'_, Self> {
        if let Some(s) = side {
            slf.side = s.0.inner;
        }
        slf
    }

    #[pyo3(text_signature = "($self, day_count)")]
    fn day_count(
        mut slf: PyRefMut<'_, Self>,
        day_count: crate::core::dates::daycount::PyDayCount,
    ) -> PyRefMut<'_, Self> {
        slf.day_count = day_count.inner;
        slf
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyVarianceSwap> {
        slf.ensure_ready()?;
        let underlying_id = slf.underlying_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "VarianceSwapBuilder internal error: missing underlying_id after validation",
            )
        })?;
        let notional = slf.notional_money().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "VarianceSwapBuilder internal error: missing notional after validation",
            )
        })?;
        let strike_variance = slf.strike_variance.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "VarianceSwapBuilder internal error: missing strike_variance after validation",
            )
        })?;
        let start_date = slf.start_date.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "VarianceSwapBuilder internal error: missing start_date after validation",
            )
        })?;
        let maturity = slf.maturity.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "VarianceSwapBuilder internal error: missing maturity after validation",
            )
        })?;
        if maturity <= start_date {
            return Err(PyValueError::new_err(
                "Maturity must be after observation start",
            ));
        }
        let discount_curve_id = slf.discount_curve_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "VarianceSwapBuilder internal error: missing discount curve after validation",
            )
        })?;
        let observation_freq = slf.observation_frequency.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "VarianceSwapBuilder internal error: missing observation_frequency after validation",
            )
        })?;

        let swap = VarianceSwap {
            id: slf.instrument_id.clone(),
            underlying_id,
            notional,
            strike_variance,
            start_date,
            maturity,
            observation_freq,
            realized_var_method: slf.realized_method,
            side: slf.side,
            discount_curve_id,
            day_count: slf.day_count,
            attributes: Attributes::new(),
        };

        Ok(PyVarianceSwap::new(swap))
    }

    fn __repr__(&self) -> String {
        "VarianceSwapBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyVarianceSwap {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyVarianceSwapBuilder>> {
        let py = cls.py();
        let builder = PyVarianceSwapBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::VarianceSwap)
    }

    #[getter]
    fn strike_variance(&self) -> f64 {
        self.inner.strike_variance
    }

    #[getter]
    fn observation_frequency(&self) -> &'static str {
        match self.inner.observation_freq {
            freq if freq == Tenor::daily() => "daily",
            freq if freq == Tenor::weekly() => "weekly",
            freq if freq == Tenor::monthly() => "monthly",
            freq if freq == Tenor::quarterly() => "quarterly",
            _ => "quarterly",
        }
    }

    #[getter]
    fn realized_method(&self) -> &'static str {
        method_label(self.inner.realized_var_method)
    }

    #[getter]
    fn side(&self) -> &'static str {
        match self.inner.side {
            PayReceive::Pay => "pay",
            PayReceive::Receive => "receive",
        }
    }

    fn value(&self, market: &PyMarketContext, as_of: Bound<'_, PyAny>) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of)?;
        let value = self.inner.value(&market.inner, date).map_err(core_to_py)?;
        Ok(PyMoney::new(value))
    }

    fn payoff(&self, realized_variance: f64) -> PyMoney {
        PyMoney::new(self.inner.payoff(realized_variance))
    }

    fn observation_dates(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dates = self.inner.observation_dates();
        let py_dates: PyResult<Vec<Py<PyAny>>> =
            dates.into_iter().map(|d| date_to_py(py, d)).collect();
        Ok(PyList::new(py, py_dates?)?.into())
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "VarianceSwap(id='{}', underlying='{}', strike_var={}, side='{}')",
            self.inner.id,
            self.inner.underlying_id,
            self.inner.strike_variance,
            self.side()
        ))
    }
}

impl fmt::Display for PyVarianceSwap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VarianceSwap({}, strike_var={}, side={})",
            self.inner.id,
            self.inner.strike_variance,
            self.side()
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyPayReceive>()?;
    module.add_class::<PyRealizedVarMethod>()?;
    module.add_class::<PyVarianceSwap>()?;
    module.add_class::<PyVarianceSwapBuilder>()?;
    Ok(vec![
        "VarianceDirection",
        "RealizedVarianceMethod",
        "VarianceSwap",
        "VarianceSwapBuilder",
    ])
}
