//! Schedules: frequencies, stub rules, and a fluent business-day-aware builder.
//!
//! This module exposes `Frequency`, `StubKind`, `ScheduleBuilder`, and `Schedule`.
//! Build schedules by chaining methods on `ScheduleBuilder`—set frequency, stub
//! handling, business-day convention+calendar, and end-of-month alignment—then
//! call `build()` to produce an immutable `Schedule` of dates.
use super::calendar::{PyBusinessDayConvention, PyCalendar};
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::errors::{core_to_py, PyContext};
use finstack_core::dates::{Frequency, ScheduleBuilder, ScheduleSpec, StubKind};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};
use pyo3::{Bound, PyRef};
use std::fmt;
use time::Date;

/// Represent coupon or payment frequency values.
///
/// Parameters
/// ----------
/// months : int, optional
///     Number of months between periods when using :py:meth:`Frequency.from_months`.
/// days : int, optional
///     Number of days between periods when using :py:meth:`Frequency.from_days`.
///
/// Returns
/// -------
/// Frequency
///     Frequency token usable for schedules and day-count contexts.
#[pyclass(name = "Frequency", module = "finstack.core.dates.schedule", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyFrequency {
    pub(crate) inner: Frequency,
}

impl PyFrequency {
    pub(crate) const fn new(inner: Frequency) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFrequency {
    #[classmethod]
    #[pyo3(text_signature = "(cls, months)")]
    /// Construct a frequency based on a number of calendar months.
    fn from_months(_cls: &Bound<'_, PyType>, months: u8) -> PyResult<Self> {
        if months == 0 || months > 12 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Months must be in the range 1..=12",
            ));
        }
        Ok(Self::new(Frequency::Months(months)))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, days)")]
    /// Construct a frequency using a number of days between events.
    fn from_days(_cls: &Bound<'_, PyType>, days: u16) -> PyResult<Self> {
        if days == 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Days must be greater than zero",
            ));
        }
        Ok(Self::new(Frequency::Days(days)))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, payments_per_year)")]
    /// Construct a frequency from a number of payments per year.
    ///
    /// This mirrors :rust:`finstack_core::dates::Frequency::from_payments_per_year`,
    /// accepting common values such as 1, 2, 3, 4, 6, or 12.
    fn from_payments_per_year(_cls: &Bound<'_, PyType>, payments_per_year: u32) -> PyResult<Self> {
        Frequency::from_payments_per_year(payments_per_year)
            .map(Self::new)
            .map_err(|msg| pyo3::exceptions::PyValueError::new_err(msg))
    }

    #[classattr]
    const ANNUAL: Self = Self {
        inner: Frequency::Months(12),
    };
    #[classattr]
    const SEMI_ANNUAL: Self = Self {
        inner: Frequency::Months(6),
    };
    #[classattr]
    const QUARTERLY: Self = Self {
        inner: Frequency::Months(3),
    };
    #[classattr]
    const MONTHLY: Self = Self {
        inner: Frequency::Months(1),
    };
    #[classattr]
    const BIWEEKLY: Self = Self {
        inner: Frequency::Days(14),
    };
    #[classattr]
    const WEEKLY: Self = Self {
        inner: Frequency::Days(7),
    };
    #[classattr]
    const DAILY: Self = Self {
        inner: Frequency::Days(1),
    };

    #[getter]
    /// Month-based interval represented by this frequency, if applicable.
    fn months(&self) -> Option<u8> {
        self.inner.months()
    }

    #[getter]
    /// Day-based interval represented by this frequency, if applicable.
    fn days(&self) -> Option<u16> {
        self.inner.days()
    }

    fn __repr__(&self) -> String {
        if let Some(m) = self.inner.months() {
            format!("Frequency.months({m})")
        } else if let Some(d) = self.inner.days() {
            format!("Frequency.days({d})")
        } else {
            "Frequency(?)".to_string()
        }
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl fmt::Display for PyFrequency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.__repr__())
    }
}

/// Encode how to handle irregular front/back stubs when building schedules.
///
/// Parameters
/// ----------
/// None
///     Access constants (e.g. :attr:`StubKind.SHORT_FRONT`).
///
/// Returns
/// -------
/// StubKind
///     Stub rule identifier.
#[pyclass(name = "StubKind", module = "finstack.core.dates.schedule", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyStubKind {
    pub(crate) inner: StubKind,
}

impl PyStubKind {
    fn label(&self) -> &'static str {
        match self.inner {
            StubKind::None => "none",
            StubKind::ShortFront => "short_front",
            StubKind::ShortBack => "short_back",
            StubKind::LongFront => "long_front",
            StubKind::LongBack => "long_back",
            _ => "custom",
        }
    }
}

#[pymethods]
impl PyStubKind {
    #[classattr]
    const NONE: Self = Self {
        inner: StubKind::None,
    };
    #[classattr]
    const SHORT_FRONT: Self = Self {
        inner: StubKind::ShortFront,
    };
    #[classattr]
    const SHORT_BACK: Self = Self {
        inner: StubKind::ShortBack,
    };
    #[classattr]
    const LONG_FRONT: Self = Self {
        inner: StubKind::LongFront,
    };
    #[classattr]
    const LONG_BACK: Self = Self {
        inner: StubKind::LongBack,
    };

    #[getter]
    /// Snake-case label describing the stub type.
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("StubKind('{}')", self.label())
    }

    fn __str__(&self) -> &'static str {
        self.label()
    }
}

impl fmt::Display for PyStubKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Fluent builder used to construct business-day aware schedules.
///
/// Parameters
/// ----------
/// start : datetime.date
///     Schedule start date when invoking :py:meth:`ScheduleBuilder.new`.
/// end : datetime.date
///     Schedule end date when invoking :py:meth:`ScheduleBuilder.new`.
///
/// Returns
/// -------
/// ScheduleBuilder
///     Builder supporting method chaining prior to :py:meth:`ScheduleBuilder.build`.
#[pyclass(
    name = "ScheduleBuilder",
    module = "finstack.core.dates.schedule",
    unsendable
)]
#[derive(Clone, Copy)]
pub struct PyScheduleBuilder {
    inner: ScheduleBuilder<'static>,
    start: Date,
    end: Date,
}

impl PyScheduleBuilder {
    fn new_with_builder(inner: ScheduleBuilder<'static>, start: Date, end: Date) -> Self {
        Self { inner, start, end }
    }
}

#[pymethods]
impl PyScheduleBuilder {
    #[classmethod]
    #[pyo3(text_signature = "(cls, start, end)")]
    /// Start constructing a schedule between two boundary dates.
    ///
    /// Parameters
    /// ----------
    /// start : datetime.date
    ///     Start date (inclusive) for the schedule.
    /// end : datetime.date
    ///     End date (exclusive or inclusive based on stub handling).
    ///
    /// Returns
    /// -------
    /// ScheduleBuilder
    ///     Builder ready for method chaining.
    ///
    /// Examples
    /// --------
    /// Build a quarterly schedule, following convention, USNY calendar, EOM aligned:
    ///
    /// >>> sb = ScheduleBuilder.new(date(2024, 1, 31), date(2025, 1, 31))
    /// >>> sb = sb.frequency(Frequency.QUARTERLY)
    /// >>> sb = sb.stub_rule(StubKind.NONE)
    /// >>> sb = sb.adjust_with(BusinessDayConvention.FOLLOWING, cal)
    /// >>> sb = sb.end_of_month(True)
    /// >>> schedule = sb.build()
    fn new(
        _cls: &Bound<'_, PyType>,
        start: Bound<'_, PyAny>,
        end: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let start_date = py_to_date(&start).context("start")?;
        let end_date = py_to_date(&end).context("end")?;
        let builder = ScheduleBuilder::try_new(start_date, end_date).map_err(core_to_py)?;
        Ok(Self::new_with_builder(builder, start_date, end_date))
    }

    #[pyo3(text_signature = "(self, frequency)")]
    /// Set the accrual/payment frequency for the schedule.
    ///
    /// Parameters
    /// ----------
    /// frequency : Frequency
    ///     Interval between successive dates (e.g. monthly, quarterly).
    ///
    /// Returns
    /// -------
    /// ScheduleBuilder
    ///     Updated builder for chaining.
    fn frequency(&self, frequency: &PyFrequency) -> Self {
        Self::new_with_builder(self.inner.frequency(frequency.inner), self.start, self.end)
    }

    #[pyo3(text_signature = "(self, stub)")]
    /// Specify how to handle front/back stubs.
    ///
    /// Parameters
    /// ----------
    /// stub : StubKind
    ///     Rule for irregular period alignment (short/long front/back).
    ///
    /// Returns
    /// -------
    /// ScheduleBuilder
    ///     Updated builder.
    fn stub_rule(&self, stub: &PyStubKind) -> Self {
        Self::new_with_builder(self.inner.stub_rule(stub.inner), self.start, self.end)
    }

    #[pyo3(text_signature = "(self, convention, calendar)")]
    /// Apply a business-day convention using the provided calendar.
    ///
    /// Parameters
    /// ----------
    /// convention : BusinessDayConvention
    ///     Adjustment rule (e.g. following, modified following).
    /// calendar : Calendar
    ///     Holiday calendar supplying business-day logic.
    ///
    /// Returns
    /// -------
    /// ScheduleBuilder
    ///     Updated builder.
    fn adjust_with(
        &self,
        convention: &PyBusinessDayConvention,
        calendar: PyRef<PyCalendar>,
    ) -> Self {
        Self::new_with_builder(
            self.inner.adjust_with(convention.inner, calendar.inner),
            self.start,
            self.end,
        )
    }

    #[pyo3(text_signature = "(self, enabled)")]
    /// Enable or disable end-of-month alignment.
    ///
    /// Parameters
    /// ----------
    /// enabled : bool
    ///     Whether to align generated dates to month-ends when appropriate.
    ///
    /// Returns
    /// -------
    /// ScheduleBuilder
    ///     Updated builder.
    fn end_of_month(&self, enabled: bool) -> Self {
        Self::new_with_builder(self.inner.end_of_month(enabled), self.start, self.end)
    }

    #[pyo3(text_signature = "(self)")]
    /// Align the schedule to CDS IMM dates (Mar/Jun/Sep/Dec third Wednesday).
    ///
    /// Returns
    /// -------
    /// ScheduleBuilder
    ///     Updated builder configured for CDS IMM alignment.
    fn cds_imm(&self) -> Self {
        Self::new_with_builder(self.inner.cds_imm(), self.start, self.end)
    }

    #[pyo3(text_signature = "(self)")]
    /// Build the schedule and return an immutable `Schedule` wrapper.
    ///
    /// Returns
    /// -------
    /// Schedule
    ///     Immutable schedule of generated dates.
    fn build(&self) -> PyResult<PySchedule> {
        self.inner
            .build()
            .map(|schedule| PySchedule::new(schedule.dates))
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        format!(
            "ScheduleBuilder(start={:?}, end={:?})",
            self.start, self.end
        )
    }
}

/// Immutable collection of dates produced by :class:`ScheduleBuilder`.
///
/// Parameters
/// ----------
/// None
///     Instances are returned by :py:meth:`ScheduleBuilder.build`.
///
/// Returns
/// -------
/// Schedule
///     Sequence-like container of schedule anchor dates.
#[pyclass(name = "Schedule", module = "finstack.core.dates.schedule", frozen)]
#[derive(Clone)]
pub struct PySchedule {
    dates: Vec<Date>,
}

impl PySchedule {
    pub(crate) fn new(dates: Vec<Date>) -> Self {
        Self { dates }
    }
}

#[pymethods]
impl PySchedule {
    #[getter]
    /// Dates contained in the schedule as `datetime.date` objects.
    fn dates(&self, py: Python<'_>) -> PyResult<Vec<PyObject>> {
        self.dates.iter().map(|d| date_to_py(py, *d)).collect()
    }

    #[getter]
    /// Number of scheduled dates.
    fn len(&self) -> usize {
        self.dates.len()
    }

    fn __len__(&self) -> usize {
        self.dates.len()
    }

    fn __repr__(&self) -> String {
        format!("Schedule({} dates)", self.dates.len())
    }
}

#[pyclass(name = "ScheduleSpec", module = "finstack.core.dates.schedule", frozen)]
#[derive(Clone)]
pub struct PyScheduleSpec {
    pub(crate) inner: ScheduleSpec,
}

#[pymethods]
impl PyScheduleSpec {
    #[new]
    #[pyo3(signature = (
        start,
        end,
        frequency,
        stub=None,
        business_day_convention=None,
        calendar_id=None,
        end_of_month=false,
        cds_imm_mode=false,
        graceful=false
    ))]
    fn new(
        start: Bound<'_, PyAny>,
        end: Bound<'_, PyAny>,
        frequency: &PyFrequency,
        stub: Option<&PyStubKind>,
        business_day_convention: Option<&PyBusinessDayConvention>,
        calendar_id: Option<String>,
        end_of_month: bool,
        cds_imm_mode: bool,
        graceful: bool,
    ) -> PyResult<Self> {
        let start_date = py_to_date(&start).context("start")?;
        let end_date = py_to_date(&end).context("end")?;
        Ok(Self {
            inner: ScheduleSpec {
                start: start_date,
                end: end_date,
                frequency: frequency.inner,
                stub: stub.map(|s| s.inner).unwrap_or(StubKind::None),
                business_day_convention: business_day_convention.map(|c| c.inner),
                calendar_id,
                end_of_month,
                cds_imm_mode,
                graceful,
            },
        })
    }

    #[getter]
    fn calendar_id(&self) -> Option<&str> {
        self.inner.calendar_id.as_deref()
    }

    #[getter]
    fn frequency(&self) -> PyFrequency {
        PyFrequency::new(self.inner.frequency)
    }

    #[getter]
    fn stub(&self) -> PyStubKind {
        PyStubKind {
            inner: self.inner.stub,
        }
    }

    fn build(&self) -> PyResult<PySchedule> {
        self.inner
            .build()
            .map(|schedule| PySchedule::new(schedule.dates))
            .map_err(core_to_py)
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| {
            PyValueError::new_err(format!("failed to serialize ScheduleSpec to JSON: {e}"))
        })
    }

    #[classmethod]
    fn from_json(_cls: &Bound<'_, PyType>, payload: &str) -> PyResult<Self> {
        serde_json::from_str(payload)
            .map(|inner| Self { inner })
            .map_err(|e| PyValueError::new_err(format!("failed to parse ScheduleSpec JSON: {e}")))
    }

    fn __repr__(&self) -> String {
        format!(
            "ScheduleSpec(start={:?}, end={:?}, frequency={:?})",
            self.inner.start, self.inner.end, self.inner.frequency
        )
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "schedule")?;
    module.setattr(
        "__doc__",
        "Schedule generation helpers mirroring finstack_core::dates::schedule_iter.",
    )?;
    module.add_class::<PyFrequency>()?;
    module.add_class::<PyStubKind>()?;
    module.add_class::<PyScheduleBuilder>()?;
    module.add_class::<PySchedule>()?;
    module.add_class::<PyScheduleSpec>()?;
    let exports = [
        "Frequency",
        "StubKind",
        "ScheduleBuilder",
        "Schedule",
        "ScheduleSpec",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
