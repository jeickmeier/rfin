use super::calendar::PyCalendar;
use super::schedule::PyFrequency;
use crate::core::common::labels::normalize_label;
use crate::core::error::core_to_py;
use crate::core::utils::py_to_date;
use finstack_core::dates::{DayCount, DayCountCtx, Frequency, Thirty360Convention};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};
use pyo3::{Bound, PyRef};
use std::fmt;

/// Wrap finstack day-count conventions for year-fraction calculations.
///
/// Parameters
/// ----------
/// None
///     Instantiate via class attributes (e.g. :attr:`DayCount.ACT_365F`) or :py:meth:`DayCount.from_name`.
///
/// Returns
/// -------
/// DayCount
///     Enum-like value describing the convention to apply.
#[pyclass(name = "DayCount", module = "finstack.core.dates.daycount", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyDayCount {
    pub(crate) inner: DayCount,
}

impl PyDayCount {
    pub(crate) const fn new(inner: DayCount) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
        match self.inner {
            DayCount::Act360 => "act_360",
            DayCount::Act365F => "act_365f",
            DayCount::Act365L => "act_365l",
            DayCount::Thirty360 => "thirty_360",
            DayCount::ThirtyE360 => "thirty_e_360",
            DayCount::ActAct => "act_act",
            DayCount::ActActIsma => "act_act_isma",
            DayCount::Bus252 => "bus_252",
            _ => "custom",
        }
    }
}

#[pymethods]
impl PyDayCount {
    #[classattr]
    const ACT_360: Self = Self {
        inner: DayCount::Act360,
    };
    #[classattr]
    const ACT_365F: Self = Self {
        inner: DayCount::Act365F,
    };
    #[classattr]
    const ACT_365L: Self = Self {
        inner: DayCount::Act365L,
    };
    #[classattr]
    const THIRTY_360: Self = Self {
        inner: DayCount::Thirty360,
    };
    #[classattr]
    const THIRTY_E_360: Self = Self {
        inner: DayCount::ThirtyE360,
    };
    #[classattr]
    const ACT_ACT: Self = Self {
        inner: DayCount::ActAct,
    };
    #[classattr]
    const ACT_ACT_ISMA: Self = Self {
        inner: DayCount::ActActIsma,
    };
    #[classattr]
    const BUS_252: Self = Self {
        inner: DayCount::Bus252,
    };

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    /// Parse a day-count convention from a common alias (e.g. ``"act/365f"``).
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        let normalized = normalize_label(name);
        match normalized.as_str() {
            "act/360" | "act_360" | "actual/360" => Ok(Self::new(DayCount::Act360)),
            "act/365f" | "act_365f" | "actual/365f" => Ok(Self::new(DayCount::Act365F)),
            "act/365l" | "act_365l" | "actual/365l" | "act/365afb" => {
                Ok(Self::new(DayCount::Act365L))
            }
            "30/360" | "30_360" | "thirty/360" | "30u/360" => Ok(Self::new(DayCount::Thirty360)),
            "30e/360" | "30e_360" | "30/360e" => Ok(Self::new(DayCount::ThirtyE360)),
            "act/act" | "act_act" | "actual/actual" | "act/act isda" => {
                Ok(Self::new(DayCount::ActAct))
            }
            "act/act isma" | "act_act_isma" | "icma" => Ok(Self::new(DayCount::ActActIsma)),
            "bus/252" | "bus_252" | "business/252" => Ok(Self::new(DayCount::Bus252)),
            other => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Unknown day-count convention: {other}"
            ))),
        }
    }

    /// Snake-case identifier of the convention.
    #[getter]
    fn name(&self) -> &'static str {
        self.label()
    }

    #[pyo3(text_signature = "(self, start, end, ctx=None)")]
    /// Compute the year fraction between two dates (optionally using context).
    fn year_fraction(
        &self,
        start: Bound<'_, PyAny>,
        end: Bound<'_, PyAny>,
        ctx: Option<PyRef<PyDayCountContext>>,
    ) -> PyResult<f64> {
        let start_date = py_to_date(&start)?;
        let end_date = py_to_date(&end)?;
        let ctx_inner = if let Some(ctx_ref) = ctx {
            DayCountCtx {
                calendar: ctx_ref.calendar.as_ref().map(|cal| cal.inner),
                frequency: ctx_ref.frequency,
                bus_basis: None,
            }
        } else {
            DayCountCtx::default()
        };
        self.inner
            .year_fraction(start_date, end_date, ctx_inner)
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        format!("DayCount('{}')", self.label())
    }

    fn __str__(&self) -> &'static str {
        self.label()
    }
}

impl fmt::Display for PyDayCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Carry optional calendar and frequency hints for day-count evaluation.
///
/// Parameters
/// ----------
/// calendar : Calendar, optional
///     Calendar applied when conventions rely on business days.
/// frequency : Frequency, optional
///     Coupon frequency used by conventions such as Act/Act ISMA.
///
/// Returns
/// -------
/// DayCountContext
///     Context object passed to :py:meth:`DayCount.year_fraction`.
#[pyclass(
    name = "DayCountContext",
    module = "finstack.core.dates.daycount",
    unsendable
)]
#[derive(Clone, Default)]
pub struct PyDayCountContext {
    calendar: Option<PyCalendar>,
    frequency: Option<Frequency>,
}

#[pymethods]
impl PyDayCountContext {
    #[new]
    #[pyo3(signature = (calendar=None, frequency=None))]
    /// Create a context with optional calendar/frequency hints.
    fn ctor(calendar: Option<PyRef<PyCalendar>>, frequency: Option<PyRef<PyFrequency>>) -> Self {
        Self {
            calendar: calendar.map(|c| c.clone()),
            frequency: frequency.map(|f| f.inner),
        }
    }

    #[getter]
    /// Calendar used for business-day adjustments (if any).
    fn calendar(&self) -> Option<PyCalendar> {
        self.calendar.clone()
    }

    #[setter]
    /// Update the calendar component (``None`` to clear).
    fn set_calendar(&mut self, calendar: Option<PyRef<PyCalendar>>) {
        self.calendar = calendar.map(|c| c.clone());
    }

    #[getter]
    /// Frequency hint used by certain day-count conventions.
    fn frequency(&self) -> Option<PyFrequency> {
        self.frequency.map(PyFrequency::new)
    }

    #[setter]
    /// Update the frequency component (``None`` to clear).
    fn set_frequency(&mut self, frequency: Option<PyRef<PyFrequency>>) {
        self.frequency = frequency.map(|f| f.inner);
    }

    fn __repr__(&self) -> String {
        let cal = self
            .calendar
            .as_ref()
            .map(|c| c.code.as_ref().to_string())
            .unwrap_or_else(|| "None".to_string());
        let freq = self
            .frequency
            .map(|f| {
                if let Some(m) = f.months() {
                    format!("months({m})")
                } else if let Some(d) = f.days() {
                    format!("days({d})")
                } else {
                    "unknown".to_string()
                }
            })
            .unwrap_or_else(|| "None".to_string());
        format!("DayCountContext(calendar={cal}, frequency={freq})")
    }
}

/// Identify specific 30/360 day-count variants.
///
/// Parameters
/// ----------
/// None
///     Access constants :attr:`Thirty360Convention.US` or :attr:`Thirty360Convention.EUROPEAN`.
///
/// Returns
/// -------
/// Thirty360Convention
///     Convention token combined with :class:`DayCount` selections.
#[pyclass(
    name = "Thirty360Convention",
    module = "finstack.core.dates.daycount",
    frozen
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyThirty360Convention {
    pub(crate) inner: Thirty360Convention,
}

impl PyThirty360Convention {
    fn label(&self) -> &'static str {
        match self.inner {
            Thirty360Convention::Us => "us",
            Thirty360Convention::European => "european",
        }
    }
}

#[pymethods]
impl PyThirty360Convention {
    #[classattr]
    const US: Self = Self {
        inner: Thirty360Convention::Us,
    };
    #[classattr]
    const EUROPEAN: Self = Self {
        inner: Thirty360Convention::European,
    };

    #[getter]
    /// Identifier describing which 30/360 convention is used.
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("Thirty360Convention('{}')", self.label())
    }

    fn __str__(&self) -> &'static str {
        self.label()
    }
}

impl fmt::Display for PyThirty360Convention {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "daycount")?;
    module.setattr(
        "__doc__",
        "Day-count conventions and helpers mirroring finstack_core::dates::daycount.",
    )?;
    module.add_class::<PyDayCount>()?;
    module.add_class::<PyDayCountContext>()?;
    module.add_class::<PyThirty360Convention>()?;
    let exports = ["DayCount", "DayCountContext", "Thirty360Convention"];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
