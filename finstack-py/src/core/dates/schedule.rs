//! Python bindings for schedule generation

use finstack_core::dates::{Frequency, ScheduleBuilder, StubKind};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use super::calendar::{PyBusDayConv, PyCalendar};
use super::date::PyDate;

/// Payment frequency enumeration for financial instruments.
///
/// Defines how often payments are made on financial instruments like
/// bonds, swaps, and loans. The frequency determines the interval between
/// successive payment dates.
///
/// Examples:
///     >>> from rfin.dates import Frequency, Date, generate_schedule
///     >>> start = Date(2023, 1, 1)
///     >>> end = Date(2024, 1, 1)
///     
///     # Semi-annual payments (every 6 months)
///     >>> schedule = generate_schedule(start, end, Frequency.SemiAnnual)
///     >>> len(schedule)
///     3  # Jan 1, Jul 1, Jan 1
///     
///     # Quarterly payments (every 3 months)
///     >>> schedule = generate_schedule(start, end, Frequency.Quarterly)
///     >>> len(schedule)
///     5  # Jan 1, Apr 1, Jul 1, Oct 1, Jan 1
///     
///     # Monthly payments
///     >>> schedule = generate_schedule(start, end, Frequency.Monthly)
///     >>> len(schedule)
///     13  # Monthly from Jan 1 to Jan 1 next year
#[pyclass(name = "Frequency", module = "finstack.dates", eq)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyFrequency {
    /// Annual payments (once per year)
    Annual,
    /// Semi-annual payments (twice per year, every 6 months)
    SemiAnnual,
    /// Quarterly payments (four times per year, every 3 months)
    Quarterly,
    /// Monthly payments (twelve times per year)
    Monthly,
    /// Biweekly payments (every 2 weeks)
    BiWeekly,
    /// Weekly payments (every week)
    Weekly,
    /// Daily payments (every day)
    Daily,
}

impl From<PyFrequency> for Frequency {
    fn from(f: PyFrequency) -> Self {
        match f {
            PyFrequency::Annual => Frequency::Months(12),
            PyFrequency::SemiAnnual => Frequency::Months(6),
            PyFrequency::Quarterly => Frequency::Months(3),
            PyFrequency::Monthly => Frequency::Months(1),
            PyFrequency::BiWeekly => Frequency::Days(14),
            PyFrequency::Weekly => Frequency::Days(7),
            PyFrequency::Daily => Frequency::Days(1),
        }
    }
}

impl PyFrequency {
    /// Create PyFrequency from core Frequency type
    pub fn from_inner(freq: Frequency) -> Self {
        match freq {
            Frequency::Months(12) => PyFrequency::Annual,
            Frequency::Months(6) => PyFrequency::SemiAnnual,
            Frequency::Months(3) => PyFrequency::Quarterly,
            Frequency::Months(1) => PyFrequency::Monthly,
            Frequency::Days(14) => PyFrequency::BiWeekly,
            Frequency::Days(7) => PyFrequency::Weekly,
            Frequency::Days(1) => PyFrequency::Daily,
            _ => PyFrequency::Monthly, // Default fallback
        }
    }
}

impl PyFrequency {
    /// Return the underlying core Frequency value.
    pub fn inner(&self) -> Frequency {
        (*self).into()
    }
}

/// Stub period rule for handling irregular periods in schedules.
///
/// When generating payment schedules, the total period may not divide evenly
/// into regular intervals. Stub rules control how these irregular periods
/// are handled.
///
/// Examples:
///     >>> from rfin.dates import Date, Frequency, StubRule, generate_schedule
///     >>> start = Date(2023, 1, 15)  # Mid-month start
///     >>> end = Date(2023, 7, 1)    # Different day end
///     
///     # Without stub handling (current implementation)
///     >>> schedule = generate_schedule(start, end, Frequency.Monthly)
///     >>> # Creates regular monthly intervals from start to end
///     
///     # Future: StubRule.ShortFront would create a short period at the beginning
///     # Future: StubRule.ShortBack would create a short period at the end
#[pyclass(name = "StubRule", module = "finstack.dates", eq)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyStubRule {
    /// No stub periods - use regular intervals only
    None,
    /// Short stub at the beginning of the schedule
    ShortFront,
    /// Short stub at the end of the schedule
    ShortBack,
}

impl From<PyStubRule> for StubKind {
    fn from(s: PyStubRule) -> Self {
        match s {
            PyStubRule::None => StubKind::None,
            PyStubRule::ShortFront => StubKind::ShortFront,
            PyStubRule::ShortBack => StubKind::ShortBack,
        }
    }
}

impl PyStubRule {
    /// Get the inner StubKind value
    pub fn inner(&self) -> StubKind {
        (*self).into()
    }
}

/// Generate a date schedule for financial instruments.
///
/// Creates a sequence of dates at regular intervals between start and end dates.
/// This is commonly used for generating payment schedules for bonds, swaps,
/// and other financial instruments.
///
/// Args:
///     start (Date): The start date of the schedule (inclusive).
///     end (Date): The end date of the schedule (inclusive).
///     frequency (Frequency): The payment frequency (e.g., Frequency.SemiAnnual).
///     convention (Optional[BusDayConvention]): Business day convention for date adjustment.
///                                           Currently not implemented - reserved for future use.
///     calendar (Optional[Calendar]): Holiday calendar for business day adjustment.
///                                   Currently not implemented - reserved for future use.
///     stub (Optional[StubRule]): Rule for handling irregular periods.
///                               Currently not implemented - reserved for future use.
///
/// Returns:
///     List[Date]: A list of dates representing the payment schedule.
///                The list includes both the start and end dates.
///
/// Examples:
///     >>> from rfin.dates import Date, Frequency, generate_schedule
///     
///     # Generate semi-annual schedule
///     >>> start = Date(2023, 1, 1)
///     >>> end = Date(2024, 1, 1)
///     >>> schedule = generate_schedule(start, end, Frequency.SemiAnnual)
///     >>> schedule
///     [Date('2023-01-01'), Date('2023-07-01'), Date('2024-01-01')]
///     
///     # Generate quarterly schedule
///     >>> schedule = generate_schedule(start, end, Frequency.Quarterly)
///     >>> schedule
///     [Date('2023-01-01'), Date('2023-04-01'), Date('2023-07-01'),
///      Date('2023-10-01'), Date('2024-01-01')]
///     
///     # Generate monthly schedule for shorter period
///     >>> start = Date(2023, 1, 15)
///     >>> end = Date(2023, 4, 15)
///     >>> schedule = generate_schedule(start, end, Frequency.Monthly)
///     >>> schedule
///     [Date('2023-01-15'), Date('2023-02-15'), Date('2023-03-15'), Date('2023-04-15')]
///     
///     # Weekly schedule
///     >>> start = Date(2023, 1, 1)
///     >>> end = Date(2023, 1, 29)
///     >>> schedule = generate_schedule(start, end, Frequency.Weekly)
///     >>> len(schedule)
///     5  # 4 weeks + end date
///
/// Note:
///     The `convention`, `calendar`, and `stub` parameters are reserved for future
///     implementation and are currently ignored.
#[pyfunction(name = "generate_schedule", signature = (start, end, frequency, convention = None, calendar = None, stub = None))]
pub fn py_generate_schedule(
    start: &PyDate,
    end: &PyDate,
    frequency: PyFrequency,
    convention: Option<PyBusDayConv>,
    calendar: Option<&PyCalendar>,
    stub: Option<PyStubRule>,
) -> PyResult<Vec<PyDate>> {
    // Validate input range to raise a friendly Python error instead of panicking.
    if start.inner() > end.inner() {
        return Err(PyValueError::new_err(
            "Invalid date range: start must be before or equal to end",
        ));
    }
    // NOTE: Business-day adjustment and explicit calendar support remain reserved
    // for a follow-up PR. We now route through the core ScheduleBuilder and honor
    // the optional stub rule when provided.
    let mut builder = ScheduleBuilder::new(start.inner(), end.inner()).frequency(frequency.into());
    if let Some(s) = stub {
        builder = builder.stub_rule(s.into());
    }

    // Ignore convention/calendar for now to preserve existing API behavior
    let _ = (convention, calendar);

    let schedule = builder
        .build()
        .map_err(|e| PyValueError::new_err(format!("{}", e)))?;
    Ok(schedule
        .into_iter()
        .map(PyDate::from_core)
        .collect::<Vec<_>>())
}
