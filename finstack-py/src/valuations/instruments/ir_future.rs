use crate::core::common::args::DayCountArg;
use crate::core::error::core_to_py;
use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::py_to_date;
use crate::valuations::common::{extract_curve_id, extract_instrument_id, PyInstrumentType};
use finstack_core::dates::DayCount;
use finstack_valuations::instruments::ir_future::{
    FutureContractSpecs, InterestRateFuture, Position,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
use std::fmt;

fn parse_position(label: Option<&str>) -> PyResult<Position> {
    match label {
        None => Ok(Position::Long),
        Some(s) => s
            .parse()
            .map_err(|e: String| PyValueError::new_err(e)),
    }
}

/// Interest rate future wrapper exposing a convenience constructor.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "InterestRateFuture",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyInterestRateFuture {
    pub(crate) inner: InterestRateFuture,
}

impl PyInterestRateFuture {
    pub(crate) fn new(inner: InterestRateFuture) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyInterestRateFuture {
    #[classmethod]
    #[pyo3(
        signature = (
            instrument_id,
            notional,
            quoted_price,
            expiry,
            fixing_date,
            period_start,
            period_end,
            discount_curve,
            forward_curve,
            *,
            position=None,
            day_count=None,
            face_value=1_000_000.0,
            tick_size=0.0025,
            tick_value=None,
            delivery_months=3,
            convexity_adjustment=None
        ),
        text_signature = "(cls, instrument_id, notional, quoted_price, expiry, fixing_date, period_start, period_end, discount_curve, forward_curve, /, *, position='long', day_count='act_360', face_value=1_000_000.0, tick_size=0.0025, tick_value=None, delivery_months=3, convexity_adjustment=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        quoted_price: f64,
        expiry: Bound<'_, PyAny>,
        fixing_date: Bound<'_, PyAny>,
        period_start: Bound<'_, PyAny>,
        period_end: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        forward_curve: Bound<'_, PyAny>,
        position: Option<&str>,
        day_count: Option<Bound<'_, PyAny>>,
        face_value: Option<f64>,
        tick_size: Option<f64>,
        tick_value: Option<f64>,
        delivery_months: Option<u8>,
        convexity_adjustment: Option<f64>,
    ) -> PyResult<Self> {
        let id = extract_instrument_id(&instrument_id)?;
        let notional_money = extract_money(&notional)?;
        let expiry_date = py_to_date(&expiry)?;
        let fixing = py_to_date(&fixing_date)?;
        let start = py_to_date(&period_start)?;
        let end = py_to_date(&period_end)?;
        let disc_id = extract_curve_id(&discount_curve)?;
        let fwd_id = extract_curve_id(&forward_curve)?;
        let day_count_value = if let Some(obj) = day_count {
            let DayCountArg(value) = obj.extract()?;
            value
        } else {
            DayCount::Act360
        };
        let position_value = parse_position(position)?;

        let mut specs = FutureContractSpecs::default();
        if let Some(face) = face_value {
            specs.face_value = face;
        }
        if let Some(tick) = tick_size {
            specs.tick_size = tick;
        }
        if let Some(value) = tick_value {
            specs.tick_value = value;
        }
        if let Some(months) = delivery_months {
            specs.delivery_months = months;
        }
        specs.convexity_adjustment = convexity_adjustment;

        let mut builder = InterestRateFuture::builder();
        builder = builder.id(id);
        builder = builder.notional(notional_money);
        builder = builder.quoted_price(quoted_price);
        builder = builder.expiry_date(expiry_date);
        builder = builder.fixing_date(fixing);
        builder = builder.period_start(start);
        builder = builder.period_end(end);
        builder = builder.disc_id(disc_id);
        builder = builder.forward_id(fwd_id);
        builder = builder.day_count(day_count_value);
        builder = builder.position(position_value);
        builder = builder.contract_specs(specs);
        builder = builder.attributes(Default::default());

        let future = builder.build().map_err(core_to_py)?;
        Ok(Self::new(future))
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
    Ok(vec!["InterestRateFuture"])
}
