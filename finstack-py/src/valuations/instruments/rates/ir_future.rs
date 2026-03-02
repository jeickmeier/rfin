use crate::core::common::args::DayCountArg;
use crate::core::currency::PyCurrency;
use crate::core::dates::utils::py_to_date;
use crate::core::money::PyMoney;
use crate::errors::{core_to_py, PyContext};
use crate::valuations::common::PyInstrumentType;
use finstack_core::dates::DayCount;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::rates::ir_future::{
    FutureContractSpecs, InterestRateFuture, Position,
};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRef, PyRefMut};
use std::fmt;
use std::sync::Arc;

fn parse_position(label: Option<&str>) -> PyResult<Position> {
    match label {
        None => Ok(Position::Long),
        Some(s) => s.parse().map_err(|e: String| PyValueError::new_err(e)),
    }
}

/// Interest rate future wrapper exposing a convenience constructor.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "InterestRateFuture",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyInterestRateFuture {
    pub(crate) inner: Arc<InterestRateFuture>,
}

impl PyInterestRateFuture {
    pub(crate) fn new(inner: InterestRateFuture) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "InterestRateFutureBuilder",
    unsendable
)]
pub struct PyInterestRateFutureBuilder {
    instrument_id: InstrumentId,
    pending_notional_amount: Option<f64>,
    pending_currency: Option<finstack_core::currency::Currency>,
    quoted_price: Option<f64>,
    expiry: Option<time::Date>,
    fixing_date: Option<time::Date>,
    period_start: Option<time::Date>,
    period_end: Option<time::Date>,
    discount_curve_id: Option<CurveId>,
    forward_curve_id: Option<CurveId>,
    position: Position,
    day_count: DayCount,
    face_value: f64,
    tick_size: f64,
    tick_value: Option<f64>,
    delivery_months: u8,
    convexity_adjustment: Option<f64>,
    vol_surface: Option<String>,
}

impl PyInterestRateFutureBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            pending_notional_amount: None,
            pending_currency: None,
            quoted_price: None,
            expiry: None,
            fixing_date: None,
            period_start: None,
            period_end: None,
            discount_curve_id: None,
            forward_curve_id: None,
            position: Position::Long,
            day_count: DayCount::Act360,
            face_value: 1_000_000.0,
            tick_size: 0.0025,
            tick_value: None,
            delivery_months: 3,
            convexity_adjustment: None,
            vol_surface: None,
        }
    }

    fn notional_money(&self) -> Option<Money> {
        match (self.pending_notional_amount, self.pending_currency) {
            (Some(amount), Some(currency)) => Some(Money::new(amount, currency)),
            _ => None,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.notional_money().is_none() {
            return Err(PyValueError::new_err(
                "Both notional() and currency() must be provided before build().",
            ));
        }
        if self.quoted_price.is_none() {
            return Err(PyValueError::new_err("quoted_price() is required."));
        }
        if self.expiry.is_none() {
            return Err(PyValueError::new_err("expiry() is required."));
        }
        if self.discount_curve_id.is_none() {
            return Err(PyValueError::new_err("discount_curve() is required."));
        }
        if self.forward_curve_id.is_none() {
            return Err(PyValueError::new_err("forward_curve() is required."));
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
impl PyInterestRateFutureBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
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

    #[pyo3(text_signature = "($self, quoted_price)")]
    fn quoted_price(mut slf: PyRefMut<'_, Self>, quoted_price: f64) -> PyRefMut<'_, Self> {
        slf.quoted_price = Some(quoted_price);
        slf
    }

    #[pyo3(text_signature = "($self, expiry)")]
    fn expiry<'py>(
        mut slf: PyRefMut<'py, Self>,
        expiry: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.expiry = Some(py_to_date(&expiry).context("expiry")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, fixing_date)")]
    fn fixing_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        fixing_date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.fixing_date = Some(py_to_date(&fixing_date).context("fixing_date")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, period_start)")]
    fn period_start<'py>(
        mut slf: PyRefMut<'py, Self>,
        period_start: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.period_start = Some(py_to_date(&period_start).context("period_start")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, period_end)")]
    fn period_end<'py>(
        mut slf: PyRefMut<'py, Self>,
        period_end: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.period_end = Some(py_to_date(&period_end).context("period_end")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn discount_curve(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    /// Deprecated: use `discount_curve()` instead.
    #[pyo3(name = "disc_id", text_signature = "($self, curve_id)")]
    fn disc_id_deprecated(slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        Self::discount_curve(slf, curve_id)
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn forward_curve(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.forward_curve_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    /// Deprecated: use `forward_curve()` instead.
    #[pyo3(name = "fwd_id", text_signature = "($self, curve_id)")]
    fn fwd_id_deprecated(slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        Self::forward_curve(slf, curve_id)
    }

    #[pyo3(text_signature = "($self, position)")]
    fn position<'py>(
        mut slf: PyRefMut<'py, Self>,
        position: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        if let Ok(name) = position.extract::<String>() {
            slf.position = parse_position(Some(name.as_str()))?;
        } else if position.is_none() {
            slf.position = Position::Long;
        } else {
            // Try extracting as the equity FuturePosition type for convenience
            let repr = position.str()?.to_string();
            match repr.to_lowercase().as_str() {
                "long" => slf.position = Position::Long,
                "short" => slf.position = Position::Short,
                _ => {
                    return Err(PyTypeError::new_err(
                        "position() expects 'long', 'short', or FuturePosition",
                    ))
                }
            }
        }
        Ok(slf)
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

    #[pyo3(text_signature = "($self, face_value)")]
    fn face_value(mut slf: PyRefMut<'_, Self>, face_value: f64) -> PyRefMut<'_, Self> {
        slf.face_value = face_value;
        slf
    }

    #[pyo3(text_signature = "($self, tick_size)")]
    fn tick_size(mut slf: PyRefMut<'_, Self>, tick_size: f64) -> PyRefMut<'_, Self> {
        slf.tick_size = tick_size;
        slf
    }

    #[pyo3(text_signature = "($self, tick_value=None)", signature = (tick_value=None))]
    fn tick_value(mut slf: PyRefMut<'_, Self>, tick_value: Option<f64>) -> PyRefMut<'_, Self> {
        slf.tick_value = tick_value;
        slf
    }

    #[pyo3(text_signature = "($self, delivery_months)")]
    fn delivery_months(mut slf: PyRefMut<'_, Self>, delivery_months: u8) -> PyRefMut<'_, Self> {
        slf.delivery_months = delivery_months;
        slf
    }

    #[pyo3(
        text_signature = "($self, convexity_adjustment=None)",
        signature = (convexity_adjustment=None)
    )]
    fn convexity_adjustment(
        mut slf: PyRefMut<'_, Self>,
        convexity_adjustment: Option<f64>,
    ) -> PyRefMut<'_, Self> {
        slf.convexity_adjustment = convexity_adjustment;
        slf
    }

    #[pyo3(text_signature = "($self, vol_surface=None)", signature = (vol_surface=None))]
    fn vol_surface(mut slf: PyRefMut<'_, Self>, vol_surface: Option<String>) -> PyRefMut<'_, Self> {
        slf.vol_surface = vol_surface;
        slf
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyInterestRateFuture> {
        slf.ensure_ready()?;
        let notional = slf.notional_money().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "InterestRateFutureBuilder internal error: missing notional after validation",
            )
        })?;
        let quoted_price = slf.quoted_price.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "InterestRateFutureBuilder internal error: missing quoted_price after validation",
            )
        })?;
        let expiry = slf.expiry.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "InterestRateFutureBuilder internal error: missing expiry after validation",
            )
        })?;
        let discount_curve_id = slf.discount_curve_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "InterestRateFutureBuilder internal error: missing discount curve after validation",
            )
        })?;
        let forward_curve_id = slf.forward_curve_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "InterestRateFutureBuilder internal error: missing forward curve after validation",
            )
        })?;

        let mut specs = FutureContractSpecs {
            face_value: slf.face_value,
            tick_size: slf.tick_size,
            delivery_months: slf.delivery_months,
            convexity_adjustment: slf.convexity_adjustment,
            ..Default::default()
        };
        if let Some(v) = slf.tick_value {
            specs.tick_value = v;
        }

        InterestRateFuture::builder()
            .id(slf.instrument_id.clone())
            .notional(notional)
            .quoted_price(quoted_price)
            .expiry(expiry)
            .fixing_date_opt(slf.fixing_date)
            .period_start_opt(slf.period_start)
            .period_end_opt(slf.period_end)
            .discount_curve_id(discount_curve_id)
            .forward_curve_id(forward_curve_id)
            .day_count(slf.day_count)
            .position(slf.position)
            .contract_specs(specs)
            .vol_surface_id_opt(
                slf.vol_surface
                    .as_deref()
                    .map(finstack_core::types::CurveId::new),
            )
            .attributes(Default::default())
            .build()
            .map(PyInterestRateFuture::new)
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        "InterestRateFutureBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyInterestRateFuture {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyInterestRateFutureBuilder>> {
        let py = cls.py();
        let builder = PyInterestRateFutureBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
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
    fn quoted_price(&self) -> f64 {
        self.inner.quoted_price
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::InterestRateFuture)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "InterestRateFuture(id='{}', price={:.2})",
            self.inner.id, self.inner.quoted_price
        ))
    }
}

impl fmt::Display for PyInterestRateFuture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "InterestRateFuture({}, price={:.2})",
            self.inner.id, self.inner.quoted_price
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyInterestRateFuture>()?;
    module.add_class::<PyInterestRateFutureBuilder>()?;
    Ok(vec!["InterestRateFuture", "InterestRateFutureBuilder"])
}
