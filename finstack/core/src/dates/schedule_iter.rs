//! Date schedule construction.
//!
//! A single builder constructs a concrete `Schedule` in one step. Special modes
//! (e.g., CDS IMM) are expressed as builder modifiers.
//!
//! Examples
//! --------
//! Plain monthly schedule:
//! ```
//! use finstack_core::dates::{ScheduleBuilder, Frequency, Date};
//! use time::Month;
//!
//! let start = Date::from_calendar_date(2025, Month::January, 15).unwrap();
//! let end = Date::from_calendar_date(2025, Month::April, 15).unwrap();
//! let sched = ScheduleBuilder::new(start, end)
//!     .frequency(Frequency::monthly())
//!     .build()
//!     .unwrap();
//! let dates: Vec<_> = sched.into_iter().collect();
//! assert_eq!(dates.len(), 4);
//! ```
//!
//! CDS IMM schedule (quarterly on 20-Mar/Jun/Sep/Dec), start auto-adjusts to next
//! roll if needed:
//! ```
//! use finstack_core::dates::{ScheduleBuilder, Date};
//! use time::Month;
//!
//! let start = Date::from_calendar_date(2025, Month::January, 15).unwrap();
//! let end = Date::from_calendar_date(2025, Month::December, 20).unwrap();
//! let sched = ScheduleBuilder::new(start, end)
//!     .cds_imm()
//!     .build()
//!     .unwrap();
//! let dates: Vec<_> = sched.into_iter().collect();
//! assert_eq!(dates.len(), 4);
//! ```

#![allow(missing_docs)]
#![allow(clippy::needless_lifetimes)]

use smallvec::SmallVec;
use time::{Date, Duration};

use super::{adjust, next_cds_date, BusinessDayConvention, HolidayCalendar};
use crate::dates::utils::{add_months, is_leap_year};

/// Small helper alias when we need to pre-buffer (used only for `ShortFront`).
type Buffer = SmallVec<[Date; 32]>;

/// Coupon/payment frequency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum Frequency {
    /// Calendar-month based frequency (e.g. 3 = quarterly).
    Months(u8), // 1..=12
    /// Day-based frequency (e.g. 14 = biweekly).
    Days(u16), // >0
}

impl Frequency {
    #[inline]
    pub const fn months(self) -> Option<u8> {
        match self {
            Self::Months(m) => Some(m),
            _ => None,
        }
    }

    #[inline]
    pub const fn days(self) -> Option<u16> {
        match self {
            Self::Days(d) => Some(d),
            _ => None,
        }
    }

    pub(crate) fn to_step(self) -> Step {
        match self {
            Frequency::Months(m) => Step::Months(m as i32),
            Frequency::Days(d) => Step::Days(d as i32),
        }
    }

    // Convenience constructors for common frequencies
    pub const fn annual() -> Self {
        Self::Months(12)
    }
    pub const fn semi_annual() -> Self {
        Self::Months(6)
    }
    /// Every two months.
    pub const fn bimonthly() -> Self {
        Self::Months(2)
    }

    pub const fn quarterly() -> Self {
        Self::Months(3)
    }
    pub const fn monthly() -> Self {
        Self::Months(1)
    }
    pub const fn biweekly() -> Self {
        Self::Days(14)
    }

    pub const fn weekly() -> Self {
        Self::Days(7)
    }
    pub const fn daily() -> Self {
        Self::Days(1)
    }
}

/// Stub convention used when the start/end dates are not exact multiples of
/// the frequency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum StubKind {
    None,
    ShortFront,
    ShortBack,
    LongFront,
    LongBack,
}

/// Internal step abstraction allowing frequency-agnostic date arithmetic.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Step {
    /// Add *n* calendar months (positive or negative).
    Months(i32),
    /// Add *n* calendar days  (positive or negative).
    Days(i32),
}

impl Step {
    /// Return a new `Date` advanced by this step relative to `date`.
    fn add(self, date: Date) -> Date {
        match self {
            Step::Months(m) => add_months(date, m),
            Step::Days(d) => date + Duration::days(d as i64),
        }
    }
}

/// Apply End-of-Month (EOM) convention to a date.
/// Returns the last day of the month for the given date.
fn apply_eom(date: Date) -> Date {
    use time::Month;

    let year = date.year();
    let month = date.month();

    // Get the last day of this month
    let last_day = match month {
        Month::February => {
            // Check if it's a leap year
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        Month::April | Month::June | Month::September | Month::November => 30,
        _ => 31,
    };

    Date::from_calendar_date(year, month, last_day).unwrap_or(date)
}

/// Concrete schedule containing generated anchor dates.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Schedule {
    pub dates: Vec<Date>,
}

impl IntoIterator for Schedule {
    type Item = Date;
    type IntoIter = std::vec::IntoIter<Date>;
    fn into_iter(self) -> Self::IntoIter {
        self.dates.into_iter()
    }
}

/// Check if a date is a CDS roll date (20th of Mar/Jun/Sep/Dec).
fn is_cds_roll_date(date: Date) -> bool {
    use time::Month;

    if date.day() != 20 {
        return false;
    }

    matches!(
        date.month(),
        Month::March | Month::June | Month::September | Month::December
    )
}

/// Public builder for configuring schedule generation with
/// fluent API (frequency, stub rule, business-day adjustment, EOM convention).
///
/// See unit tests and `examples/` for usage patterns (stubs, adjustments, frequencies).
#[derive(Clone, Copy)]
pub struct ScheduleBuilder<'a> {
    start: Date,
    end: Date,
    freq: Frequency,
    stub: StubKind,
    conv: Option<BusinessDayConvention>,
    cal: Option<&'a dyn HolidayCalendar>,
    eom: bool,
    cds_imm_mode: bool,
}

impl<'a> ScheduleBuilder<'a> {
    /// Create a new builder with mandatory `start` and `end` dates.
    /// Defaults: frequency = Monthly, stub = None, no adjustment, no EOM.
    ///
    /// # Panics
    /// Panics if `start` > `end` when building the schedule.
    ///
    /// # Notes
    /// Inputs must satisfy `start` <= `end`.
    pub fn new(start: Date, end: Date) -> Self {
        Self {
            start,
            end,
            freq: Frequency::Months(1),
            stub: StubKind::None,
            conv: None,
            cal: None,
            eom: false,
            cds_imm_mode: false,
        }
    }

    /// Fallible constructor that validates `start` <= `end`.
    /// Returns an error rather than panicking when inputs are invalid.
    pub fn try_new(start: Date, end: Date) -> crate::Result<Self> {
        if start > end {
            return Err(crate::error::InputError::InvalidDateRange.into());
        }
        Ok(Self::new(start, end))
    }

    /// Set coupon/payment frequency.
    #[must_use]
    pub fn frequency(mut self, freq: Frequency) -> Self {
        self.freq = freq;
        self
    }

    /// Set stub handling rule.
    #[must_use]
    pub fn stub_rule(mut self, stub: StubKind) -> Self {
        self.stub = stub;
        self
    }

    /// Configure business-day adjustment using `conv` and `cal`.
    #[must_use]
    pub fn adjust_with(
        mut self,
        conv: BusinessDayConvention,
        cal: &'a dyn HolidayCalendar,
    ) -> Self {
        self.conv = Some(conv);
        self.cal = Some(cal);
        self
    }

    /// Enable End-of-Month (EOM) convention.
    /// When enabled, dates will be adjusted to the last day of each month.
    #[must_use]
    pub fn end_of_month(mut self, eom: bool) -> Self {
        self.eom = eom;
        self
    }

    /// Create a CDS IMM schedule (quarterly on the 20th: 20-Mar, 20-Jun, 20-Sep, 20-Dec).
    /// This is a convenience method for credit default swap schedules that follow
    /// standard IMM roll dates.
    #[must_use]
    pub fn cds_imm(mut self) -> Self {
        self.freq = Frequency::Months(3);
        self.stub = StubKind::ShortBack;
        self.cds_imm_mode = true;
        self
    }
    /// Build a concrete schedule (adjusted if configured).
    pub fn build(self) -> crate::Result<Schedule> {
        if self.start > self.end {
            return Err(crate::error::InputError::InvalidDateRange.into());
        }

        // Apply CDS IMM start adjustment if requested
        let (start, end) = if self.cds_imm_mode {
            let adj_start = if is_cds_roll_date(self.start) {
                self.start
            } else {
                next_cds_date(self.start)
            };
            (adj_start, self.end)
        } else {
            (self.start, self.end)
        };

        let builder = BuilderInternal {
            start,
            end,
            freq: self.freq,
            stub: self.stub,
            eom: self.eom,
        };

        let mut dates = builder.generate();

        // Enforce monotonicity and remove duplicates produced by EOM/stub handling
        enforce_monotonic_and_dedup(&mut dates);

        // Apply business day adjustment if configured
        if let (Some(conv), Some(cal)) = (self.conv, self.cal) {
            for d in &mut dates {
                *d = adjust(*d, conv, cal)?;
            }

            // Adjustment can create duplicates (e.g., both anchors adjust to same business day)
            // and, in edge cases, non-monotonicities. Enforce again.
            enforce_monotonic_and_dedup(&mut dates);
        }

        Ok(Schedule { dates })
    }
}

// Internal generator for schedule construction
struct BuilderInternal {
    start: Date,
    end: Date,
    freq: Frequency,
    stub: StubKind,
    eom: bool,
}

impl BuilderInternal {
    fn generate(self) -> Vec<Date> {
        let step = self.freq.to_step();
        match self.stub {
            StubKind::ShortFront => self.gen_short_front(step),
            StubKind::LongFront => self.gen_long_front(step),
            StubKind::LongBack => self.gen_long_back(step),
            StubKind::None | StubKind::ShortBack => self.gen_regular(step),
        }
    }

    fn gen_regular(self, step: Step) -> Vec<Date> {
        let mut buf: Buffer = Buffer::new();
        let (mut dt, end) = if self.eom {
            (apply_eom(self.start), apply_eom(self.end))
        } else {
            (self.start, self.end)
        };
        buf.push(dt);
        while dt < end {
            let mut next = step.add(dt);
            if next > end {
                next = end;
            }
            dt = if self.eom { apply_eom(next) } else { next };
            if dt != *buf.last().unwrap() {
                buf.push(dt);
            }
        }
        buf.into_vec()
    }

    fn gen_short_front(self, step: Step) -> Vec<Date> {
        // Build backwards from end, then reverse
        let mut buf: Buffer = Buffer::new();
        let mut dt = self.end;
        let target = self.start;
        loop {
            let date_to_add = if self.eom { apply_eom(dt) } else { dt };
            if buf.last().copied() != Some(date_to_add) {
                buf.push(date_to_add);
            }
            if dt == target {
                break;
            }
            let prev = match step {
                Step::Months(m) => add_months(dt, -m),
                Step::Days(d) => dt - Duration::days(d as i64),
            };
            dt = if prev < target { target } else { prev };
        }
        buf.as_mut_slice().reverse();
        buf.into_vec()
    }

    fn gen_long_front(self, step: Step) -> Vec<Date> {
        let mut buf: Buffer = Buffer::new();
        let mut anchors = Vec::new();
        let mut dt = self.end;
        anchors.push(dt);
        while dt > self.start {
            let prev = match step {
                Step::Months(m) => add_months(dt, -m),
                Step::Days(d) => dt - Duration::days(d as i64),
            };
            if prev >= self.start {
                dt = prev;
                anchors.push(dt);
            } else {
                break;
            }
        }
        buf.push(if self.eom {
            apply_eom(self.start)
        } else {
            self.start
        });
        for &a in anchors.iter().rev() {
            let d = if self.eom { apply_eom(a) } else { a };
            if d != *buf.last().unwrap() {
                buf.push(d);
            }
        }
        buf.into_vec()
    }

    fn gen_long_back(self, step: Step) -> Vec<Date> {
        let mut buf: Buffer = Buffer::new();
        let mut dt = self.start;
        buf.push(if self.eom { apply_eom(dt) } else { dt });
        while dt < self.end {
            let next = step.add(dt);
            let next_after = step.add(next);
            if next_after >= self.end {
                let end_date = if self.eom {
                    apply_eom(self.end)
                } else {
                    self.end
                };
                if end_date != *buf.last().unwrap() {
                    buf.push(end_date);
                }
                break;
            } else {
                let d = if self.eom { apply_eom(next) } else { next };
                if d != *buf.last().unwrap() {
                    buf.push(d);
                }
                dt = next;
            }
        }
        buf.into_vec()
    }
}

/// Enforce strictly increasing, duplicate-free dates while preserving original order.
/// Drops any consecutive duplicates and any dates that would not increase.
fn enforce_monotonic_and_dedup(dates: &mut Vec<Date>) {
    if dates.is_empty() {
        return;
    }
    let mut out: Vec<Date> = Vec::with_capacity(dates.len());
    let mut last = dates[0];
    out.push(last);
    for &d in dates.iter().skip(1) {
        if d > last {
            out.push(d);
            last = d;
        }
        // Else: skip duplicates and non-increasing values
    }
    *dates = out;
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_frequency_serde_roundtrip() {
        use serde_json;

        // Test different Frequency variants
        let frequencies = vec![
            Frequency::annual(),
            Frequency::semi_annual(),
            Frequency::quarterly(),
            Frequency::monthly(),
            Frequency::biweekly(),
            Frequency::weekly(),
            Frequency::daily(),
        ];

        for freq in frequencies {
            let json = serde_json::to_string(&freq).unwrap();
            let deserialized: Frequency = serde_json::from_str(&json).unwrap();
            assert_eq!(freq, deserialized);
        }
    }

    #[test]
    fn test_stub_kind_serde_roundtrip() {
        use serde_json;

        // Test all StubKind variants
        let stub_kinds = vec![
            StubKind::None,
            StubKind::ShortFront,
            StubKind::ShortBack,
            StubKind::LongFront,
            StubKind::LongBack,
        ];

        for stub in stub_kinds {
            let json = serde_json::to_string(&stub).unwrap();
            let deserialized: StubKind = serde_json::from_str(&json).unwrap();
            assert_eq!(stub, deserialized);
        }
    }

    #[test]
    fn test_schedule_serde_roundtrip() {
        use serde_json;

        // Create a schedule
        let start = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let end = Date::from_calendar_date(2025, Month::April, 15).unwrap();
        let sched = ScheduleBuilder::new(start, end)
            .frequency(Frequency::monthly())
            .build()
            .unwrap();

        let json = serde_json::to_string(&sched).unwrap();
        let deserialized: Schedule = serde_json::from_str(&json).unwrap();

        assert_eq!(sched.dates.len(), deserialized.dates.len());
        for (original, deserialized) in sched.dates.iter().zip(deserialized.dates.iter()) {
            assert_eq!(original, deserialized);
        }
    }
}
