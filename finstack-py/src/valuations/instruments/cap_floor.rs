use crate::core::common::args::DayCountArg;
// use crate::errors::core_to_py; // not used in this module currently
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::{extract_money, PyMoney};
use crate::valuations::common::{frequency_from_payments_per_year, PyInstrumentType};
use finstack_core::dates::DayCount;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::rates::cap_floor::InterestRateOption;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
use std::fmt;

fn extract_day_count(dc: Option<Bound<'_, PyAny>>) -> PyResult<DayCount> {
    if let Some(bound) = dc {
        let DayCountArg(inner) = bound.extract()?;
        Ok(inner)
    } else {
        Ok(DayCount::Act360)
    }
}

/// Interest rate cap/floor instruments using Black pricing.
///
/// Examples:
///     >>> cap = InterestRateOption.cap(
///     ...     "cap_1",
///     ...     Money("USD", 5_000_000),
///     ...     0.035,
///     ...     date(2024, 1, 1),
///     ...     date(2027, 1, 1),
///     ...     "usd_discount",
///     ...     "usd_libor_3m"
///     ... )
///     >>> cap.strike
///     0.035
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
            vol_surface,
            *,
            payments_per_year=4,
            day_count=None
        ),
        text_signature = "(cls, instrument_id, notional, strike, start_date, end_date, discount_curve, forward_curve, vol_surface, payments_per_year=4, day_count='act_360')"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a standard interest-rate cap.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     notional: Notional principal as :class:`finstack.core.money.Money`.
    ///     strike: Strike rate in decimal form.
    ///     start_date: Start of the accrual period.
    ///     end_date: End of the accrual period.
    ///     discount_curve: Discount curve identifier for valuation.
    ///     forward_curve: Forward curve identifier for rate projections.
    ///     vol_surface: Optional volatility surface identifier.
    ///     payments_per_year: Optional number of payments per year.
    ///     day_count: Optional day-count convention.
    ///
    /// Returns:
    ///     InterestRateOption: Configured interest rate cap instrument.
    ///
    /// Raises:
    ///     ValueError: If frequencies are invalid or inputs cannot be parsed.
    fn cap(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        strike: f64,
        start_date: Bound<'_, PyAny>,
        end_date: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        forward_curve: Bound<'_, PyAny>,
        vol_surface: Bound<'_, PyAny>,
        payments_per_year: Option<u32>,
        day_count: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;
        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let amt = extract_money(&notional).context("notional")?;
        let start = py_to_date(&start_date).context("start_date")?;
        let end = py_to_date(&end_date).context("end_date")?;
        let disc = CurveId::new(discount_curve.extract::<&str>().context("discount_curve")?);
        let fwd = CurveId::new(forward_curve.extract::<&str>().context("forward_curve")?);
        let freq =
            frequency_from_payments_per_year(payments_per_year).context("payments_per_year")?;
        let dc = extract_day_count(day_count).context("day_count")?;
        let vol_surface_id = vol_surface.extract::<&str>().context("vol_surface")?;
        let option = InterestRateOption::new_cap(
            id,
            amt,
            strike,
            start,
            end,
            freq,
            dc,
            disc,
            fwd,
            vol_surface_id,
        );
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
            vol_surface,
            *,
            payments_per_year=4,
            day_count=None
        ),
        text_signature = "(cls, instrument_id, notional, strike, start_date, end_date, discount_curve, forward_curve, vol_surface, payments_per_year=4, day_count='act_360')"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a standard interest-rate floor.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     notional: Notional principal as :class:`finstack.core.money.Money`.
    ///     strike: Strike rate in decimal form.
    ///     start_date: Start of the accrual period.
    ///     end_date: End of the accrual period.
    ///     discount_curve: Discount curve identifier for valuation.
    ///     forward_curve: Forward curve identifier for rate projections.
    ///     vol_surface: Optional volatility surface identifier.
    ///     payments_per_year: Optional number of payments per year.
    ///     day_count: Optional day-count convention.
    ///
    /// Returns:
    ///     InterestRateOption: Configured interest rate floor instrument.
    ///
    /// Raises:
    ///     ValueError: If frequencies are invalid or inputs cannot be parsed.
    fn floor(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        strike: f64,
        start_date: Bound<'_, PyAny>,
        end_date: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        forward_curve: Bound<'_, PyAny>,
        vol_surface: Bound<'_, PyAny>,
        payments_per_year: Option<u32>,
        day_count: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;
        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let amt = extract_money(&notional).context("notional")?;
        let start = py_to_date(&start_date).context("start_date")?;
        let end = py_to_date(&end_date).context("end_date")?;
        let disc = CurveId::new(discount_curve.extract::<&str>().context("discount_curve")?);
        let fwd = CurveId::new(forward_curve.extract::<&str>().context("forward_curve")?);
        let freq =
            frequency_from_payments_per_year(payments_per_year).context("payments_per_year")?;
        let dc = extract_day_count(day_count).context("day_count")?;
        let vol_surface_id = vol_surface.extract::<&str>().context("vol_surface")?;
        let option = InterestRateOption::new_floor(
            id,
            amt,
            strike,
            start,
            end,
            freq,
            dc,
            disc,
            fwd,
            vol_surface_id,
        );
        Ok(Self::new(option))
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier for the cap/floor.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Notional principal amount.
    ///
    /// Returns:
    ///     Money: Notional wrapped as :class:`finstack.core.money.Money`.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Strike rate in decimal form.
    ///
    /// Returns:
    ///     float: Strike rate of the instrument.
    #[getter]
    fn strike(&self) -> f64 {
        self.inner.strike_rate
    }

    /// Start date for accrual.
    ///
    /// Returns:
    ///     datetime.date: Start date converted to Python.
    #[getter]
    fn start_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.start_date)
    }

    /// End date for accrual.
    ///
    /// Returns:
    ///     datetime.date: End date converted to Python.
    #[getter]
    fn end_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.end_date)
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
    ///     str: Forward curve used for rate projections.
    #[getter]
    fn forward_curve(&self) -> String {
        self.inner.forward_id.as_str().to_string()
    }

    /// Volatility surface identifier.
    ///
    /// Returns:
    ///     str: Volatility surface used for option pricing.
    #[getter]
    fn vol_surface(&self) -> &str {
        self.inner.vol_surface_id.as_str()
    }

    /// Instrument type enumeration.
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value ``InstrumentType.CAP_FLOOR``.
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
