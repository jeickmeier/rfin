use crate::core::common::labels::normalize_label;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::market_data::PyMarketContext;
use crate::core::money::{extract_money, PyMoney};
use crate::errors::core_to_py;
use crate::valuations::common::PyInstrumentType;
use finstack_core::dates::Frequency;
use finstack_core::math::stats::RealizedVarMethod;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::common::traits::Attributes;
use finstack_valuations::instruments::variance_swap::{PayReceive, VarianceSwap};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyModule, PyType};
use pyo3::{Bound, FromPyObject, PyRef};
use std::fmt;

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
    pub(crate) inner: VarianceSwap,
}

impl PyVarianceSwap {
    pub(crate) fn new(inner: VarianceSwap) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyVarianceSwap {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, underlying_id, notional, strike_variance, start_date, maturity, discount_curve, observation_frequency, *, realized_method=None, side=None, day_count=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        underlying_id: &str,
        notional: Bound<'_, PyAny>,
        strike_variance: f64,
        start_date: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        observation_frequency: crate::core::dates::schedule::PyFrequency,
        realized_method: Option<PyRealizedVarMethod>,
        side: Option<PayReceiveArg>,
        day_count: Option<crate::core::dates::daycount::PyDayCount>,
    ) -> PyResult<Self> {
        if strike_variance < 0.0 {
            return Err(PyValueError::new_err(
                "Strike variance must be non-negative",
            ));
        }
        use crate::errors::PyContext;
        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let notional_money = extract_money(&notional).context("notional")?;
        let start = py_to_date(&start_date).context("start_date")?;
        let maturity_date = py_to_date(&maturity).context("maturity")?;
        if maturity_date <= start {
            return Err(PyValueError::new_err(
                "Maturity must be after observation start",
            ));
        }
        let discount_curve_id =
            CurveId::new(discount_curve.extract::<&str>().context("discount_curve")?);
        let method = realized_method
            .map(|m| m.inner)
            .unwrap_or(RealizedVarMethod::CloseToClose);
        let direction = side.map(|s| s.0.inner).unwrap_or(PayReceive::Receive);
        let day_count = day_count
            .map(|dc| dc.inner)
            .unwrap_or(finstack_core::dates::DayCount::Act365F);

        let swap = VarianceSwap {
            id,
            underlying_id: underlying_id.to_string(),
            notional: notional_money,
            strike_variance,
            start_date: start,
            maturity: maturity_date,
            observation_freq: observation_frequency.inner,
            realized_var_method: method,
            side: direction,
            discount_curve_id,
            day_count,
            attributes: Attributes::new(),
        };

        Ok(Self::new(swap))
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
            freq if freq == Frequency::daily() => "daily",
            freq if freq == Frequency::weekly() => "weekly",
            freq if freq == Frequency::monthly() => "monthly",
            freq if freq == Frequency::quarterly() => "quarterly",
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

    fn npv(&self, market: &PyMarketContext, as_of: Bound<'_, PyAny>) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of)?;
        let value = self.inner.npv(&market.inner, date).map_err(core_to_py)?;
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
    Ok(vec![
        "VarianceDirection",
        "RealizedVarianceMethod",
        "VarianceSwap",
    ])
}
