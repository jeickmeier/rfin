use crate::core::currency::PyCurrency;
use crate::core::market_data::PyMarketContext;
use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use crate::errors::core_to_py;
use crate::valuations::cashflow::builder::PyScheduleParams;
use crate::valuations::common::PyInstrumentType;
use finstack_core::dates::DayCount;
use finstack_core::types::InstrumentId;
use finstack_valuations::instruments::common::parameters::legs::FinancingLegSpec;
use finstack_valuations::instruments::common::parameters::underlying::{
    EquityUnderlyingParams, IndexUnderlyingParams,
};
use finstack_valuations::instruments::common::traits::Attributes;
use finstack_valuations::instruments::trs::{
    EquityTotalReturnSwap, FIIndexTotalReturnSwap, TrsScheduleSpec, TrsSide,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
use std::fmt;

fn parse_curve_id(label: &Bound<'_, PyAny>, context: &str) -> PyResult<String> {
    if let Ok(value) = label.extract::<&str>() {
        Ok(value.to_string())
    } else {
        Err(PyValueError::new_err(format!(
            "Expected {context} identifier string",
        )))
    }
}

fn day_count_label(dc: DayCount) -> &'static str {
    match dc {
        DayCount::Act360 => "act_360",
        DayCount::Act365F => "act_365f",
        DayCount::Act365L => "act_365l",
        DayCount::Thirty360 => "thirty_360",
        DayCount::ThirtyE360 => "thirty_e_360",
        DayCount::ActAct => "act_act",
        DayCount::ActActIsma => "act_act_isma",
        DayCount::Bus252 => "bus_252",
        _ => "act_360",
    }
}

/// Total return swap side wrapper.
#[pyclass(module = "finstack.valuations.instruments", name = "TrsSide", frozen)]
#[derive(Clone, Copy, Debug)]
pub struct PyTrsSide {
    pub(crate) inner: TrsSide,
}

impl PyTrsSide {
    const fn new(inner: TrsSide) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyTrsSide {
    #[classattr]
    const RECEIVE_TOTAL_RETURN: Self = Self::new(TrsSide::ReceiveTotalReturn);
    #[classattr]
    const PAY_TOTAL_RETURN: Self = Self::new(TrsSide::PayTotalReturn);

    fn __repr__(&self) -> &'static str {
        match self.inner {
            TrsSide::ReceiveTotalReturn => "TrsSide.RECEIVE_TOTAL_RETURN",
            TrsSide::PayTotalReturn => "TrsSide.PAY_TOTAL_RETURN",
        }
    }

    fn __str__(&self) -> &'static str {
        self.__repr__()
    }
}

/// Financing leg specification wrapper.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "TrsFinancingLegSpec",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyFinancingLegSpec {
    pub(crate) inner: FinancingLegSpec,
}

#[pymethods]
impl PyFinancingLegSpec {
    #[classmethod]
    #[pyo3(text_signature = "(cls, discount_curve, forward_curve, day_count, *, spread_bp=0.0)")]
    fn new(
        _cls: &Bound<'_, PyType>,
        discount_curve: Bound<'_, PyAny>,
        forward_curve: Bound<'_, PyAny>,
        day_count: crate::core::dates::daycount::PyDayCount,
        spread_bp: Option<f64>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;
        let disc = parse_curve_id(&discount_curve, "discount curve").context("discount_curve")?;
        let fwd = parse_curve_id(&forward_curve, "forward curve").context("forward_curve")?;
        let spec = FinancingLegSpec::new(disc, fwd, spread_bp.unwrap_or(0.0), day_count.inner);
        Ok(Self { inner: spec })
    }

    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[getter]
    fn forward_curve(&self) -> String {
        self.inner.forward_curve_id.as_str().to_string()
    }

    #[getter]
    fn spread_bp(&self) -> f64 {
        self.inner.spread_bp
    }

    #[getter]
    fn day_count(&self) -> &'static str {
        day_count_label(self.inner.day_count)
    }
}

/// TRS schedule specification wrapper.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "TrsScheduleSpec",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyTrsScheduleSpec {
    pub(crate) inner: TrsScheduleSpec,
}

#[pymethods]
impl PyTrsScheduleSpec {
    #[classmethod]
    #[pyo3(text_signature = "(cls, start, end, schedule_params)")]
    fn new(
        _cls: &Bound<'_, PyType>,
        start: Bound<'_, PyAny>,
        end: Bound<'_, PyAny>,
        schedule_params: PyScheduleParams,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;
        let start_date = py_to_date(&start).context("start")?;
        let end_date = py_to_date(&end).context("end")?;
        if end_date <= start_date {
            return Err(PyValueError::new_err("Schedule end must be after start"));
        }
        let spec = TrsScheduleSpec::from_params(start_date, end_date, schedule_params.inner);
        Ok(Self { inner: spec })
    }

    #[getter]
    fn start(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.start)
    }

    #[getter]
    fn end(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.end)
    }
}

/// Equity underlying parameters wrapper.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "EquityUnderlying",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyEquityUnderlyingParams {
    pub(crate) inner: EquityUnderlyingParams,
}

#[pymethods]
impl PyEquityUnderlyingParams {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, ticker, spot_id, currency, *, div_yield_id=None, contract_size=None)"
    )]
    fn new(
        _cls: &Bound<'_, PyType>,
        ticker: &str,
        spot_id: &str,
        currency: &PyCurrency,
        div_yield_id: Option<&str>,
        contract_size: Option<f64>,
    ) -> Self {
        let mut params = EquityUnderlyingParams::new(ticker, spot_id, currency.inner);
        if let Some(div) = div_yield_id {
            params = params.with_dividend_yield(div);
        }
        if let Some(size) = contract_size {
            params = params.with_contract_size(size);
        }
        Self { inner: params }
    }

    #[getter]
    fn ticker(&self) -> &str {
        &self.inner.ticker
    }

    #[getter]
    fn spot_id(&self) -> &str {
        &self.inner.spot_id
    }

    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.currency)
    }
}

/// Fixed-income index underlying parameters wrapper.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "IndexUnderlying",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyIndexUnderlyingParams {
    pub(crate) inner: IndexUnderlyingParams,
}

#[pymethods]
impl PyIndexUnderlyingParams {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, index_id, base_currency, *, yield_id=None, duration_id=None, convexity_id=None, contract_size=None)"
    )]
    fn new(
        _cls: &Bound<'_, PyType>,
        index_id: &str,
        base_currency: &PyCurrency,
        yield_id: Option<&str>,
        duration_id: Option<&str>,
        convexity_id: Option<&str>,
        contract_size: Option<f64>,
    ) -> Self {
        let mut params = IndexUnderlyingParams::new(index_id, base_currency.inner);
        if let Some(y) = yield_id {
            params = params.with_yield(y);
        }
        if let Some(d) = duration_id {
            params = params.with_duration(d);
        }
        if let Some(c) = convexity_id {
            params = params.with_convexity(c);
        }
        if let Some(size) = contract_size {
            params = params.with_contract_size(size);
        }
        Self { inner: params }
    }

    #[getter]
    fn index_id(&self) -> &str {
        self.inner.index_id.as_str()
    }

    #[getter]
    fn base_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.base_currency)
    }
}

/// Equity TRS wrapper.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "EquityTotalReturnSwap",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyEquityTotalReturnSwap {
    pub(crate) inner: EquityTotalReturnSwap,
}

impl PyEquityTotalReturnSwap {
    pub(crate) fn new(inner: EquityTotalReturnSwap) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyEquityTotalReturnSwap {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, notional, underlying, financing, schedule, side, /, *, initial_level=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        underlying: &PyEquityUnderlyingParams,
        financing: &PyFinancingLegSpec,
        schedule: &PyTrsScheduleSpec,
        side: PyTrsSide,
        initial_level: Option<f64>,
    ) -> PyResult<Self> {
        let id = InstrumentId::new(instrument_id.extract::<&str>()?);
        let notional_money = extract_money(&notional)?;

        let inner = EquityTotalReturnSwap {
            id,
            notional: notional_money,
            underlying: underlying.inner.clone(),
            financing: financing.inner.clone(),
            schedule: schedule.inner.clone(),
            side: side.inner,
            initial_level,
            attributes: Attributes::new(),
        };

        Ok(Self::new(inner))
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::TRS)
    }

    #[getter]
    fn side(&self) -> &'static str {
        match self.inner.side {
            TrsSide::ReceiveTotalReturn => "receive_total_return",
            TrsSide::PayTotalReturn => "pay_total_return",
        }
    }

    fn npv(&self, market: &PyMarketContext, as_of: Bound<'_, PyAny>) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of)?;
        let value = self.inner.npv(&market.inner, date).map_err(core_to_py)?;
        Ok(PyMoney::new(value))
    }

    fn pv_total_return_leg(
        &self,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of)?;
        let value = self
            .inner
            .pv_total_return_leg(&market.inner, date)
            .map_err(core_to_py)?;
        Ok(PyMoney::new(value))
    }

    fn pv_financing_leg(
        &self,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of)?;
        let value = self
            .inner
            .pv_financing_leg(&market.inner, date)
            .map_err(core_to_py)?;
        Ok(PyMoney::new(value))
    }

    fn financing_annuity(
        &self,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        self.inner
            .financing_annuity(&market.inner, date)
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "EquityTotalReturnSwap(id='{}', notional={}, side='{}')",
            self.inner.id,
            self.inner.notional,
            self.side()
        ))
    }
}

impl fmt::Display for PyEquityTotalReturnSwap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EquityTotalReturnSwap({}, side={})",
            self.inner.id,
            self.side()
        )
    }
}

/// Fixed income index TRS wrapper.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "FiIndexTotalReturnSwap",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyFiIndexTotalReturnSwap {
    pub(crate) inner: FIIndexTotalReturnSwap,
}

impl PyFiIndexTotalReturnSwap {
    pub(crate) fn new(inner: FIIndexTotalReturnSwap) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFiIndexTotalReturnSwap {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, notional, underlying, financing, schedule, side, /, *, initial_level=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        underlying: &PyIndexUnderlyingParams,
        financing: &PyFinancingLegSpec,
        schedule: &PyTrsScheduleSpec,
        side: PyTrsSide,
        initial_level: Option<f64>,
    ) -> PyResult<Self> {
        let id = InstrumentId::new(instrument_id.extract::<&str>()?);
        let notional_money = extract_money(&notional)?;

        let inner = FIIndexTotalReturnSwap {
            id,
            notional: notional_money,
            underlying: underlying.inner.clone(),
            financing: financing.inner.clone(),
            schedule: schedule.inner.clone(),
            side: side.inner,
            initial_level,
            attributes: Attributes::new(),
        };

        Ok(Self::new(inner))
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::TRS)
    }

    #[getter]
    fn side(&self) -> &'static str {
        match self.inner.side {
            TrsSide::ReceiveTotalReturn => "receive_total_return",
            TrsSide::PayTotalReturn => "pay_total_return",
        }
    }

    fn npv(&self, market: &PyMarketContext, as_of: Bound<'_, PyAny>) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of)?;
        let value = self.inner.npv(&market.inner, date).map_err(core_to_py)?;
        Ok(PyMoney::new(value))
    }

    fn pv_total_return_leg(
        &self,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of)?;
        let value = self
            .inner
            .pv_total_return_leg(&market.inner, date)
            .map_err(core_to_py)?;
        Ok(PyMoney::new(value))
    }

    fn pv_financing_leg(
        &self,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of)?;
        let value = self
            .inner
            .pv_financing_leg(&market.inner, date)
            .map_err(core_to_py)?;
        Ok(PyMoney::new(value))
    }

    fn financing_annuity(
        &self,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        self.inner
            .financing_annuity(&market.inner, date)
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "FiIndexTotalReturnSwap(id='{}', notional={}, side='{}')",
            self.inner.id,
            self.inner.notional,
            self.side()
        ))
    }
}

impl fmt::Display for PyFiIndexTotalReturnSwap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FiIndexTotalReturnSwap({}, side={})",
            self.inner.id,
            self.side()
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyTrsSide>()?;
    module.add_class::<PyFinancingLegSpec>()?;
    module.add_class::<PyTrsScheduleSpec>()?;
    module.add_class::<PyEquityUnderlyingParams>()?;
    module.add_class::<PyIndexUnderlyingParams>()?;
    module.add_class::<PyEquityTotalReturnSwap>()?;
    module.add_class::<PyFiIndexTotalReturnSwap>()?;
    Ok(vec![
        "TrsSide",
        "TrsFinancingLegSpec",
        "TrsScheduleSpec",
        "EquityUnderlying",
        "IndexUnderlying",
        "EquityTotalReturnSwap",
        "FiIndexTotalReturnSwap",
    ])
}
