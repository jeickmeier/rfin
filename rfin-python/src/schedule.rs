#![allow(clippy::useless_conversion)]

use pyo3::prelude::*;

use rfin_core::dates::{schedule, Frequency as CoreFrequency, StubKind as CoreStubRule};

use crate::calendar::{PyBusDayConv, PyCalendar};
use crate::dates::PyDate;

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
            PyFrequency::Annual => CoreFrequency::Months(12),
            PyFrequency::SemiAnnual => CoreFrequency::Months(6),
            PyFrequency::Quarterly => CoreFrequency::Months(3),
            PyFrequency::Monthly => CoreFrequency::Months(1),
            PyFrequency::BiWeekly => CoreFrequency::Days(14),
            PyFrequency::Weekly => CoreFrequency::Days(7),
            PyFrequency::Daily => CoreFrequency::Days(1),
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
    // NOTE: Stub handling & business-day adjustment to be re-exposed in follow-up PR.
    let _ = (convention, calendar, stub);
    // Generate iterator
    let iter = schedule(start.inner(), end.inner(), frequency.into());
    let result = iter.map(PyDate::from_core).collect::<Vec<_>>();
    Ok(result)
}
