#![allow(clippy::useless_conversion)]

use pyo3::prelude::*;

use rfin_core::dates::{ScheduleBuilder, Frequency as CoreFrequency, StubRule as CoreStubRule};

use crate::dates::PyDate;
use crate::calendar::{PyBusDayConv, PyCalendar};

/// Coupon/payment frequency enumeration.
#[pyclass(name = "Frequency", module = "rfin.dates", eq)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyFrequency {
    Annual,
    SemiAnnual,
    Quarterly,
    Monthly,
    BiWeekly,
    Weekly,
    Daily,
}

impl From<PyFrequency> for CoreFrequency {
    fn from(f: PyFrequency) -> Self {
        match f {
            PyFrequency::Annual => CoreFrequency::Annual,
            PyFrequency::SemiAnnual => CoreFrequency::SemiAnnual,
            PyFrequency::Quarterly => CoreFrequency::Quarterly,
            PyFrequency::Monthly => CoreFrequency::Monthly,
            PyFrequency::BiWeekly => CoreFrequency::BiWeekly,
            PyFrequency::Weekly => CoreFrequency::Weekly,
            PyFrequency::Daily => CoreFrequency::Daily,
        }
    }
}

/// Stub rule enumeration controlling how irregular periods are handled.
#[pyclass(name = "StubRule", module = "rfin.dates", eq)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyStubRule {
    None,
    ShortFront,
    ShortBack,
}

impl From<PyStubRule> for CoreStubRule {
    fn from(s: PyStubRule) -> Self {
        match s {
            PyStubRule::None => CoreStubRule::None,
            PyStubRule::ShortFront => CoreStubRule::ShortFront,
            PyStubRule::ShortBack => CoreStubRule::ShortBack,
        }
    }
}

/// Generate an inclusive date schedule between `start` and `end`.
///
/// Args:
///     start (Date): start date (inclusive)
///     end (Date): end date (inclusive)
///     frequency (Frequency): coupon frequency
///     convention (Optional[BusDayConvention]): business-day convention (default None → unadjusted)
///     calendar (Optional[Calendar]): holiday calendar used for adjustment
///     stub (Optional[StubRule]): stub rule controlling how irregular periods are handled
///
/// Returns:
///     List[Date]: generated schedule (Python list of Date objects)
#[pyfunction(name = "generate_schedule", signature = (start, end, frequency, convention = None, calendar = None, stub = None))]
pub fn py_generate_schedule(
    start: &PyDate,
    end: &PyDate,
    frequency: PyFrequency,
    convention: Option<PyBusDayConv>,
    calendar: Option<&PyCalendar>,
    stub: Option<PyStubRule>,
) -> PyResult<Vec<PyDate>> {
    let mut builder = ScheduleBuilder::new(start.inner(), end.inner(), frequency.into());

    if let Some(s) = stub {
        builder = builder.stub(s.into());
    }

    if let (Some(conv), Some(cal)) = (convention, calendar) {
        builder = builder.adjust_with(conv.into(), cal.hcal());
    }

    let sched = builder.generate();
    let result = sched
        .into_iter()
        .map(PyDate::from_core)
        .collect::<Vec<_>>();
    Ok(result)
} 