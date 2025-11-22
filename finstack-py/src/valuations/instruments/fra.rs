use crate::core::common::args::DayCountArg;
use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use crate::errors::core_to_py;
use crate::valuations::common::PyInstrumentType;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::fra::ForwardRateAgreement;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
use std::fmt;

/// Forward Rate Agreement binding exposing standard FRA parameters.
///
/// Examples:
///     >>> fra = ForwardRateAgreement.create(
///     ...     "fra_3x6",
///     ...     Money("USD", 5_000_000),
///     ...     0.035,
///     ...     date(2024, 3, 15),
///     ...     date(2024, 6, 17),
///     ...     date(2024, 9, 16),
///     ...     "usd_discount",
///     ...     "usd_libor_3m"
///     ... )
///     >>> fra.fixed_rate
///     0.035
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
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     notional: Notional principal as :class:`finstack.core.money.Money`.
    ///     fixed_rate: Fixed rate quoted in decimal form.
    ///     fixing_date: Date when the underlying floating rate is observed.
    ///     start_date: Start date of the accrual period.
    ///     end_date: End date of the accrual period.
    ///     discount_curve: Discount curve identifier for valuation.
    ///     forward_curve: Forward curve identifier for projecting floating rates.
    ///     day_count: Optional day-count convention.
    ///     reset_lag: Optional reset lag in business days.
    ///     pay_fixed: Optional flag indicating pay-fixed/receive-floating orientation.
    ///
    /// Returns:
    ///     ForwardRateAgreement: Configured FRA instrument.
    ///
    /// Raises:
    ///     ValueError: If identifiers or dates cannot be parsed.
    ///     RuntimeError: When the underlying builder detects invalid input.
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
        use crate::errors::PyContext;
        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let amt = extract_money(&notional).context("notional")?;
        let fixing = py_to_date(&fixing_date).context("fixing_date")?;
        let start = py_to_date(&start_date).context("start_date")?;
        let end = py_to_date(&end_date).context("end_date")?;
        let disc = CurveId::new(discount_curve.extract::<&str>().context("discount_curve")?);
        let fwd = CurveId::new(forward_curve.extract::<&str>().context("forward_curve")?);
        let day_count_value = if let Some(dc_obj) = day_count {
            let DayCountArg(dc) = dc_obj.extract().context("day_count")?;
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
        builder = builder.discount_curve_id(disc);
        builder = builder.forward_id(fwd);
        builder = builder.pay_fixed(pay_fixed.unwrap_or(true));

        let fra = builder.build().map_err(core_to_py)?;
        Ok(Self::new(fra))
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier assigned to the instrument.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// FRA fixed rate as decimal.
    ///
    /// Returns:
    ///     float: Fixed rate paid or received on the FRA.
    #[getter]
    fn fixed_rate(&self) -> f64 {
        self.inner.fixed_rate
    }

    /// Day-count convention used for accrual.
    ///
    /// Returns:
    ///     DayCount: Day-count convention wrapper.
    #[getter]
    fn day_count(&self) -> crate::core::dates::PyDayCount {
        crate::core::dates::PyDayCount::new(self.inner.day_count)
    }

    /// Reset lag in business days.
    ///
    /// Returns:
    ///     int: Number of business days prior to start when the rate fixes.
    #[getter]
    fn reset_lag(&self) -> i32 {
        self.inner.reset_lag
    }

    /// Whether the FRA pays fixed / receives floating.
    ///
    /// Returns:
    ///     bool: ``True`` when the FRA position pays fixed.
    #[getter]
    fn pay_fixed(&self) -> bool {
        self.inner.pay_fixed
    }

    /// Discount curve identifier.
    ///
    /// Returns:
    ///     str: Discount curve used for valuation.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    /// Forward curve identifier.
    ///
    /// Returns:
    ///     str: Forward curve used for projecting floating rates.
    #[getter]
    fn forward_curve(&self) -> String {
        self.inner.forward_id.as_str().to_string()
    }

    /// Fixing date for the reference rate.
    ///
    /// Returns:
    ///     datetime.date: Date on which the floating rate is observed.
    #[getter]
    fn fixing_date(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.fixing_date)
    }

    /// Start date of the accrual period.
    ///
    /// Returns:
    ///     datetime.date: Accrual start date converted to Python.
    #[getter]
    fn start_date(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.start_date)
    }

    /// End date of the accrual period.
    ///
    /// Returns:
    ///     datetime.date: Accrual end date converted to Python.
    #[getter]
    fn end_date(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.end_date)
    }

    /// Notional amount for the FRA.
    ///
    /// Returns:
    ///     Money: Notional wrapped as :class:`finstack.core.money.Money`.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Instrument type enum (``InstrumentType.FRA``).
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value ``InstrumentType.FRA``.
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
