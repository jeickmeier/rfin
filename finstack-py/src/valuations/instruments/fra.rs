use crate::core::common::args::DayCountArg;
use crate::core::error::core_to_py;
use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use crate::valuations::common::{extract_curve_id, extract_instrument_id, PyInstrumentType};
use finstack_valuations::instruments::fra::ForwardRateAgreement;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
use std::fmt;

/// Forward Rate Agreement binding exposing standard FRA parameters.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "ForwardRateAgreement",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyForwardRateAgreement {
    pub(crate) inner: ForwardRateAgreement,
}

impl PyForwardRateAgreement {
    pub(crate) fn new(inner: ForwardRateAgreement) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyForwardRateAgreement {
    #[classmethod]
    #[pyo3(
        signature = (
            instrument_id,
            notional,
            fixed_rate,
            fixing_date,
            start_date,
            end_date,
            discount_curve,
            forward_curve,
            *,
            day_count=None,
            reset_lag=2,
            pay_fixed=true
        ),
        text_signature = "(cls, instrument_id, notional, fixed_rate, fixing_date, start_date, end_date, discount_curve, forward_curve, /, *, day_count='act_360', reset_lag=2, pay_fixed=True)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a standard FRA referencing discount and forward curves.
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        fixed_rate: f64,
        fixing_date: Bound<'_, PyAny>,
        start_date: Bound<'_, PyAny>,
        end_date: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        forward_curve: Bound<'_, PyAny>,
        day_count: Option<Bound<'_, PyAny>>,
        reset_lag: Option<i32>,
        pay_fixed: Option<bool>,
    ) -> PyResult<Self> {
        let id = extract_instrument_id(&instrument_id)?;
        let amt = extract_money(&notional)?;
        let fixing = py_to_date(&fixing_date)?;
        let start = py_to_date(&start_date)?;
        let end = py_to_date(&end_date)?;
        let disc = extract_curve_id(&discount_curve)?;
        let fwd = extract_curve_id(&forward_curve)?;
        let day_count_value = if let Some(dc_obj) = day_count {
            let DayCountArg(dc) = dc_obj.extract()?;
            dc
        } else {
            finstack_core::dates::DayCount::Act360
        };

        let mut builder = ForwardRateAgreement::builder();
        builder = builder.id(id);
        builder = builder.notional(amt);
        builder = builder.fixed_rate(fixed_rate);
        builder = builder.fixing_date(fixing);
        builder = builder.start_date(start);
        builder = builder.end_date(end);
        builder = builder.day_count(day_count_value);
        builder = builder.reset_lag(reset_lag.unwrap_or(2));
        builder = builder.disc_id(disc);
        builder = builder.forward_id(fwd);
        builder = builder.pay_fixed(pay_fixed.unwrap_or(true));

        let fra = builder.build().map_err(core_to_py)?;
        Ok(Self::new(fra))
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// FRA fixed rate as decimal.
    #[getter]
    fn fixed_rate(&self) -> f64 {
        self.inner.fixed_rate
    }

    /// Day-count convention used for accrual.
    #[getter]
    fn day_count(&self) -> crate::core::dates::PyDayCount {
        crate::core::dates::PyDayCount::new(self.inner.day_count)
    }

    /// Reset lag in business days.
    #[getter]
    fn reset_lag(&self) -> i32 {
        self.inner.reset_lag
    }

    /// Whether the FRA pays fixed / receives floating.
    #[getter]
    fn pay_fixed(&self) -> bool {
        self.inner.pay_fixed
    }

    /// Discount curve identifier.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.disc_id.as_str().to_string()
    }

    /// Forward curve identifier.
    #[getter]
    fn forward_curve(&self) -> String {
        self.inner.forward_id.as_str().to_string()
    }

    /// Fixing date for the reference rate.
    #[getter]
    fn fixing_date(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.fixing_date)
    }

    /// Start date of the accrual period.
    #[getter]
    fn start_date(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.start_date)
    }

    /// End date of the accrual period.
    #[getter]
    fn end_date(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.end_date)
    }

    /// Notional amount for the FRA.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Instrument type enum (``InstrumentType.FRA``).
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::FRA)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "ForwardRateAgreement(id='{}', fixed_rate={:.4})",
            self.inner.id, self.inner.fixed_rate
        ))
    }
}

impl fmt::Display for PyForwardRateAgreement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FRA({}, rate={:.4})",
            self.inner.id, self.inner.fixed_rate
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyForwardRateAgreement>()?;
    Ok(vec!["ForwardRateAgreement"])
}
