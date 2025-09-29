use crate::core::common::args::DayCountArg;
// use crate::core::error::core_to_py; // not used in this module currently
use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use crate::valuations::common::{extract_curve_id, extract_instrument_id, PyInstrumentType};
use finstack_core::dates::{DayCount, Frequency};
use finstack_valuations::instruments::cap_floor::InterestRateOption;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
use std::fmt;

fn frequency_from_payments(payments_per_year: Option<u32>) -> PyResult<Frequency> {
    let payments = payments_per_year.unwrap_or(4);
    if payments == 0 || 12 % payments != 0 {
        return Err(PyValueError::new_err(
            "payments_per_year must divide 12 (e.g. 1,2,3,4,6,12)",
        ));
    }
    let months = (12 / payments) as u8;
    Ok(Frequency::Months(months))
}

fn leak_vol_id(vol: Option<&str>) -> &'static str {
    if let Some(value) = vol {
        Box::leak(value.to_owned().into_boxed_str())
    } else {
        "IR-CAP-VOL"
    }
}

fn extract_day_count(dc: Option<Bound<'_, PyAny>>) -> PyResult<DayCount> {
    if let Some(bound) = dc {
        let DayCountArg(inner) = bound.extract()?;
        Ok(inner)
    } else {
        Ok(DayCount::Act360)
    }
}

/// Interest rate cap/floor instruments using Black pricing.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "InterestRateOption",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyInterestRateOption {
    pub(crate) inner: InterestRateOption,
}

impl PyInterestRateOption {
    pub(crate) fn new(inner: InterestRateOption) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyInterestRateOption {
    #[classmethod]
    #[pyo3(
        signature = (
            instrument_id,
            notional,
            strike,
            start_date,
            end_date,
            discount_curve,
            forward_curve,
            *,
            vol_surface=None,
            payments_per_year=4,
            day_count=None
        ),
        text_signature = "(cls, instrument_id, notional, strike, start_date, end_date, discount_curve, forward_curve, vol_surface=None, payments_per_year=4, day_count='act_360')"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a standard interest-rate cap.
    fn cap(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        strike: f64,
        start_date: Bound<'_, PyAny>,
        end_date: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        forward_curve: Bound<'_, PyAny>,
        vol_surface: Option<&str>,
        payments_per_year: Option<u32>,
        day_count: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let id = extract_instrument_id(&instrument_id)?;
        let amt = extract_money(&notional)?;
        let start = py_to_date(&start_date)?;
        let end = py_to_date(&end_date)?;
        let disc = extract_curve_id(&discount_curve)?;
        let fwd = extract_curve_id(&forward_curve)?;
        let freq = frequency_from_payments(payments_per_year)?;
        let dc = extract_day_count(day_count)?;
        let vol_id = leak_vol_id(vol_surface);

        let option =
            InterestRateOption::new_cap(id, amt, strike, start, end, freq, dc, disc, fwd, vol_id);
        Ok(Self::new(option))
    }

    #[classmethod]
    #[pyo3(
        signature = (
            instrument_id,
            notional,
            strike,
            start_date,
            end_date,
            discount_curve,
            forward_curve,
            *,
            vol_surface=None,
            payments_per_year=4,
            day_count=None
        ),
        text_signature = "(cls, instrument_id, notional, strike, start_date, end_date, discount_curve, forward_curve, vol_surface=None, payments_per_year=4, day_count='act_360')"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a standard interest-rate floor.
    fn floor(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        strike: f64,
        start_date: Bound<'_, PyAny>,
        end_date: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        forward_curve: Bound<'_, PyAny>,
        vol_surface: Option<&str>,
        payments_per_year: Option<u32>,
        day_count: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let id = extract_instrument_id(&instrument_id)?;
        let amt = extract_money(&notional)?;
        let start = py_to_date(&start_date)?;
        let end = py_to_date(&end_date)?;
        let disc = extract_curve_id(&discount_curve)?;
        let fwd = extract_curve_id(&forward_curve)?;
        let freq = frequency_from_payments(payments_per_year)?;
        let dc = extract_day_count(day_count)?;
        let vol_id = leak_vol_id(vol_surface);

        let option =
            InterestRateOption::new_floor(id, amt, strike, start, end, freq, dc, disc, fwd, vol_id);
        Ok(Self::new(option))
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
    fn strike(&self) -> f64 {
        self.inner.strike_rate
    }

    #[getter]
    fn start_date(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.start_date)
    }

    #[getter]
    fn end_date(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.end_date)
    }

    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.disc_id.as_str().to_string()
    }

    #[getter]
    fn forward_curve(&self) -> String {
        self.inner.forward_id.as_str().to_string()
    }

    #[getter]
    fn vol_surface(&self) -> &'static str {
        self.inner.vol_id
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::CapFloor)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "InterestRateOption(id='{}', strike={:.4})",
            self.inner.id, self.inner.strike_rate
        ))
    }
}

impl fmt::Display for PyInterestRateOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "InterestRateOption({}, strike={:.4})",
            self.inner.id, self.inner.strike_rate
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyInterestRateOption>()?;
    Ok(vec!["InterestRateOption"])
}
