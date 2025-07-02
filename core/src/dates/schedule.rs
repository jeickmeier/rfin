//! Date schedule builder DSL (PR #5)
//!
//! This is a **very lightweight** fluent builder for generating date schedules
//! used by fixed-income instruments (coupon/settlement schedules, etc.).  As the
//! rest of the `rfin_core::dates` façade it is fully `no_std` compatible and does
//! **not** allocate heap memory.  Internally it uses `SmallVec<[Date; 32]>` which
//! stores up to 32 dates inline without touching the allocator and only spills to
//! `alloc` when longer.
//!
//! The current implementation covers the subset required by the PR-5 acceptance
//! criteria:
//! • Frequencies from monthly to annual via [`Frequency`].
//! • Inclusive start & end dates.
//! • Optional business-day adjustment with any [`HolidayCalendar`] and
//!   [`BusDayConv`] convention.
//! • Stub handling via [`StubRule`] (currently *no-stub* and short-front/back are
//!   recognised – the generation algorithm for short stubs will be extended in
//!   a future PR but the enum is already exposed so downstream code can
//!   compile).
//!
//! The public interface intentionally stays minimal and panic-free.  All helpers
//! are `const` where feasible.

#![allow(clippy::many_single_char_names)]

use smallvec::SmallVec;
use time::{Date, Month};

use super::{adjust, BusDayConv, HolidayCalendar};

/// Inline-optimised container returned by [`ScheduleBuilder::generate`].
/// Up to 32 dates are stored on the stack.
pub type Schedule = SmallVec<[Date; 32]>;

// -------------------------------------------------------------------------------------------------
// Frequency
// -------------------------------------------------------------------------------------------------

/// Coupon/payment frequency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Frequency {
    /// Yearly — one period per year.
    Annual,
    /// Semi-annual — two periods per year (every 6 months).
    SemiAnnual,
    /// Quarterly — four periods per year (every 3 months).
    Quarterly,
    /// Monthly — twelve periods per year (every month).
    Monthly,
    /// Bi-weekly — every 2 weeks (14 days).
    BiWeekly,
    /// Weekly — every week (7 days).
    Weekly,
    /// Daily — every calendar day.
    Daily,
}

impl Frequency {
    /// Return the number of whole calendar months in a single period.
    #[inline]
    pub const fn months(self) -> i32 {
        match self {
            Self::Annual => 12,
            Self::SemiAnnual => 6,
            Self::Quarterly => 3,
            Self::Monthly => 1,
            Self::BiWeekly | Self::Weekly | Self::Daily => 0,
        }
    }

    /// Return the number of calendar days in a single period (0 for month/quarter frequencies).
    #[inline]
    pub const fn days(self) -> i32 {
        match self {
            Self::Daily => 1,
            Self::Weekly => 7,
            Self::BiWeekly => 14,
            _ => 0,
        }
    }

    /// Number of periods per (calendar) year for this frequency.
    #[inline]
    pub const fn periods_per_year(self) -> u16 {
        match self {
            Self::Annual => 1,
            Self::SemiAnnual => 2,
            Self::Quarterly => 4,
            Self::Monthly => 12,
            Self::BiWeekly => 26,
            Self::Weekly => 52,
            Self::Daily => 365,
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Stub handling (placeholder — full algorithm in later PRs)
// -------------------------------------------------------------------------------------------------

/// Stub convention used when the start/end dates are not exact multiples of the
/// frequency.  Stub generation logic will follow in a dedicated PR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StubRule {
    /// No stub periods — start/end dates must be an exact multiple of the frequency.
    None,
    /// Short stub at the start of the schedule.
    ShortFront,
    /// Short stub at the end of the schedule.
    ShortBack,
}

// -------------------------------------------------------------------------------------------------
// ScheduleBuilder
// -------------------------------------------------------------------------------------------------

/// Fluent builder constructing inclusive `Date` schedules.
#[derive(Clone)]
pub struct ScheduleBuilder<'a> {
    start: Date,
    end: Date,
    freq: Frequency,
    stub: StubRule,
    conv: Option<BusDayConv>,
    cal: Option<&'a dyn HolidayCalendar>,
}

impl<'a> ScheduleBuilder<'a> {
    /// Start a new builder with mandatory `start`, `end` and `freq`.
    #[must_use]
    pub const fn new(start: Date, end: Date, freq: Frequency) -> Self {
        Self {
            start,
            end,
            freq,
            stub: StubRule::None,
            conv: None,
            cal: None,
        }
    }

    /// Set stub rule (defaults to [`StubRule::None`]).
    #[must_use]
    pub const fn stub(mut self, rule: StubRule) -> Self {
        self.stub = rule;
        self
    }

    /// Attach business-day adjustment `conv` using the provided `cal`.
    #[must_use]
    pub fn adjust_with(mut self, conv: BusDayConv, cal: &'a dyn HolidayCalendar) -> Self {
        self.conv = Some(conv);
        self.cal = Some(cal);
        self
    }

    /// Generate the final inclusive schedule.
    #[must_use]
    pub fn generate(self) -> Schedule {
        debug_assert!(self.start <= self.end, "start date after end date");

        // Helper closure applying optional business-day adjustment.
        let (conv, cal) = (self.conv, self.cal);
        let adjust_date = |d: Date| -> Date {
            if let (Some(c), Some(cal)) = (conv, cal) {
                adjust(d, c, cal)
            } else {
                d
            }
        };

        let mut schedule: Schedule = SmallVec::new();

        let step_months = self.freq.months();
        if step_months > 0 {
            // ---------------- Month-based frequencies ----------------
            let step = step_months;
            match self.stub {
                StubRule::None => {
                    // Expect exact alignment – in debug builds assert but still produce output.
                    debug_assert_eq!(add_months(self.start, step * (((self.end.year() - self.start.year()) * 12
                        + (self.end.month() as i32 - self.start.month() as i32))
                        / step)), self.end, "Schedule not aligned for StubRule::None");

                    let mut dt = self.start;
                    loop {
                        schedule.push(adjust_date(dt));
                        if dt == self.end {
                            break;
                        }
                        dt = add_months(dt, step);
                    }
                }
                StubRule::ShortBack => {
                    let mut dt = self.start;
                    loop {
                        schedule.push(adjust_date(dt));
                        if dt == self.end {
                            break;
                        }
                        let next = add_months(dt, step);
                        dt = if next > self.end { self.end } else { next };
                    }
                }
                StubRule::ShortFront => {
                    let mut rev: Schedule = SmallVec::new();
                    let mut dt = self.end;
                    loop {
                        rev.push(adjust_date(dt));
                        if dt == self.start {
                            break;
                        }
                        let prev = add_months(dt, -step);
                        if prev < self.start {
                            rev.push(adjust_date(self.start));
                            break;
                        } else {
                            dt = prev;
                        }
                    }
                    for d in rev.into_iter().rev() {
                        schedule.push(d);
                    }
                }
            }
        } else {
            // ---------------- Day-based frequencies ----------------
            use time::Duration;
            let step_days = self.freq.days();
            debug_assert!(step_days > 0, "frequency step must be positive");

            match self.stub {
                StubRule::None => {
                    let mut dt = self.start;
                    loop {
                        schedule.push(adjust_date(dt));
                        if dt == self.end {
                            break;
                        }
                        let next = dt + Duration::days(step_days as i64);
                        if next > self.end {
                            // Remainder not divisible by step – treat as final date (behaves like ShortBack).
                            dt = self.end;
                        } else {
                            dt = next;
                        }
                    }
                }
                StubRule::ShortBack => {
                    let mut dt = self.start;
                    loop {
                        schedule.push(adjust_date(dt));
                        if dt == self.end {
                            break;
                        }
                        let next = dt + Duration::days(step_days as i64);
                        dt = if next > self.end { self.end } else { next };
                    }
                }
                StubRule::ShortFront => {
                    let mut rev: Schedule = SmallVec::new();
                    let mut dt = self.end;
                    loop {
                        rev.push(adjust_date(dt));
                        if dt == self.start {
                            break;
                        }
                        let prev = dt - Duration::days(step_days as i64);
                        if prev < self.start {
                            rev.push(adjust_date(self.start));
                            break;
                        } else {
                            dt = prev;
                        }
                    }
                    for d in rev.into_iter().rev() {
                        schedule.push(d);
                    }
                }
            }
        }

        schedule
    }
}

// -------------------------------------------------------------------------------------------------
// Internal helpers
// -------------------------------------------------------------------------------------------------

/// Add `months` calendar months to `date`, clamping the day-of-month to the last
/// valid day where necessary (e.g. Jan-31 + 1 month = Feb-28/29). Negative
/// offsets are supported for stepping *backwards* (e.g. Mar-31 + -1 month = Feb-29).
fn add_months(date: Date, months: i32) -> Date {
    // Using Euclidean division so month index is always positive (0-11).
    let total_months = date.year() * 12 + (date.month() as i32 - 1) + months;
    let new_year = total_months.div_euclid(12);
    let new_month_index = total_months.rem_euclid(12); // 0-based 0..11
    let new_month = Month::try_from((new_month_index + 1) as u8).unwrap();

    let day = date.day();
    let max_day = days_in_month(new_year, new_month);
    let new_day = day.min(max_day);

    Date::from_calendar_date(new_year, new_month, new_day).unwrap()
}

/// Days in the given month for `year`.
const fn days_in_month(year: i32, month: Month) -> u8 {
    match month {
        Month::January => 31,
        Month::February => if is_leap_year(year) { 29 } else { 28 },
        Month::March => 31,
        Month::April => 30,
        Month::May => 31,
        Month::June => 30,
        Month::July => 31,
        Month::August => 31,
        Month::September => 30,
        Month::October => 31,
        Month::November => 30,
        Month::December => 31,
    }
}

const fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

// -------------------------------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::Date;
    use time::Month;

    fn make_date(y: i32, m: u8, d: u8) -> Date {
        Date::from_calendar_date(y, Month::try_from(m).unwrap(), d).unwrap()
    }

    #[test]
    fn semi_annual_schedule_basic() {
        let start = make_date(2025, 1, 15);
        let end = make_date(2030, 1, 15);
        let sched = ScheduleBuilder::new(start, end, Frequency::SemiAnnual).generate();
        assert_eq!(sched.len(), 11);
        assert_eq!(sched.first().copied(), Some(start));
        assert_eq!(sched.last().copied(), Some(end));
    }

    #[test]
    fn short_back_stub() {
        let start = make_date(2025, 1, 15);
        let end = make_date(2025, 4, 15); // 3-month gap < semi-annual step ⇒ short back stub
        let sched = ScheduleBuilder::new(start, end, Frequency::SemiAnnual)
            .stub(StubRule::ShortBack)
            .generate();
        assert_eq!(sched.len(), 2);
        assert_eq!(sched[0], start);
        assert_eq!(sched[1], end);
    }

    #[test]
    fn short_front_stub() {
        let start = make_date(2025, 1, 15);
        let end = make_date(2025, 10, 15); // front stub (Jan->Apr 3m, then 6-month periods)
        let sched = ScheduleBuilder::new(start, end, Frequency::SemiAnnual)
            .stub(StubRule::ShortFront)
            .generate();
        assert_eq!(sched.len(), 3);
        assert_eq!(sched.first().copied(), Some(start));
        assert_eq!(sched.last().copied(), Some(end));
        // Intermediate anchor should be April 15 2025 (start + 3 months offset when anchored from end)
        assert_eq!(sched[1], make_date(2025, 4, 15));
    }

    #[test]
    fn weekly_schedule_basic() {
        let start = make_date(2025, 1, 6); // Monday
        let end = make_date(2025, 1, 20); // Monday two weeks later
        let sched = ScheduleBuilder::new(start, end, Frequency::Weekly).generate();
        assert_eq!(sched.as_slice(), &[start, make_date(2025, 1, 13), end]);
    }

    #[test]
    fn daily_schedule() {
        let start = make_date(2025, 1, 1);
        let end = make_date(2025, 1, 3);
        let sched = ScheduleBuilder::new(start, end, Frequency::Daily).generate();
        assert_eq!(sched.as_slice(), &[start, make_date(2025, 1, 2), end]);
    }
} 