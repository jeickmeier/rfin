//! Lightweight date schedule iterator.
//!
//! The iterator pre-computes the anchor dates internally (stored inline for up to
//! 32 dates) but keeps the backing container private so the public API is
//! allocation-free and zero-dependency.

#![allow(missing_docs)]
#![allow(clippy::needless_lifetimes)]

use smallvec::SmallVec;
use time::{Date, Duration};

use super::{adjust, BusinessDayConvention, HolidayCalendar};
use crate::dates::holiday::rule::add_months;

/// Small helper alias when we need to pre-buffer (used only for `ShortFront`).
type Buffer = SmallVec<[Date; 32]>;

/// Coupon/payment frequency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Frequency {
    /// Calendar-month based frequency (e.g. 3 = quarterly).
    Months(u8), // 1..=12
    /// Day-based frequency (e.g. 14 = bi-weekly).
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
#[non_exhaustive]
pub enum StubKind {
    None,
    ShortFront,
    ShortBack,
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

/// Lazy schedule iterator. Internally uses either a streaming lazy generator
/// or a small pre-buffer for `ShortFront` stub schedules.
pub enum ScheduleIter {
    /// Streaming iterator (no heap allocation)
    Lazy(LazyIter),
    /// Fallback buffered iterator (only used for ShortFront stub)
    Buf { buf: Buffer, idx: usize },
}

impl Iterator for ScheduleIter {
    type Item = Date;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ScheduleIter::Lazy(it) => it.next(),
            ScheduleIter::Buf { buf, idx } => {
                if *idx < buf.len() {
                    let d = buf[*idx];
                    *idx += 1;
                    Some(d)
                } else {
                    None
                }
            }
        }
    }
}

impl Default for ScheduleIter {
    fn default() -> Self {
        ScheduleIter::Buf {
            buf: Buffer::new(),
            idx: 0,
        }
    }
}

/// Iterator adaptor that applies business day adjustments to date sequences.
pub struct AdjustIter<'a, I> {
    inner: I,
    conv: BusinessDayConvention,
    cal: &'a dyn HolidayCalendar,
}

impl<'a, I> AdjustIter<'a, I> {
    /// Wrap an iterator to apply business day adjustments.
    pub fn new(inner: I, conv: BusinessDayConvention, cal: &'a dyn HolidayCalendar) -> Self {
        Self { inner, conv, cal }
    }
}

impl<'a, I> Iterator for AdjustIter<'a, I>
where
    I: Iterator<Item = Date>,
{
    type Item = Date;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|date| adjust(date, self.conv, self.cal))
    }
}

/// Internal stateful lazy iterator for forward-building schedules.
pub struct LazyIter {
    next_date: Date,
    end: Date,
    step: Step,
    finished: bool,
}

impl Iterator for LazyIter {
    type Item = Date;
    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        // Current date in the schedule sequence
        let current = self.next_date;

        if current == self.end {
            // We've reached the end date - this is the last item
            self.finished = true;
        } else {
            // Calculate next date in sequence
            let mut next = self.step.add(current);

            // For StubKind::None or ShortBack: if next step would overshoot
            // the end date, clamp to end date (creating a short stub at back)
            if next > self.end {
                next = self.end;
            }

            self.next_date = next;
        }

        Some(current)
    }
}

/// Public entry-point creating a schedule iterator.
///
/// Example:
/// ```
/// use finstack_core::dates::{schedule, Frequency};
/// use time::{Date, Month};
/// let start = Date::from_calendar_date(2025, Month::January, 15).unwrap();
/// let end   = Date::from_calendar_date(2026, Month::January, 15).unwrap();
/// let dates: Vec<_> = schedule(start, end, Frequency::quarterly()).collect();
/// assert_eq!(dates[0], start);
/// ```
pub fn schedule(start: Date, end: Date, freq: Frequency) -> impl Iterator<Item = Date> {
    ScheduleBuilder::new(start, end).frequency(freq).build_raw()
}

/// Public builder for configuring schedule generation with
/// fluent API (frequency, stub rule, business-day adjustment).
///
/// # Examples
///
/// ## Basic monthly schedule
/// ```
/// use finstack_core::dates::{ScheduleBuilder, Frequency};
/// use time::{Date, Month};
///
/// let start = Date::from_calendar_date(2025, Month::January, 15).unwrap();
/// let end = Date::from_calendar_date(2025, Month::April, 15).unwrap();
///
/// let dates: Vec<_> = ScheduleBuilder::new(start, end)
///     .frequency(Frequency::monthly())
///     .build_raw()
///     .collect();
///     
/// assert_eq!(dates.len(), 4);
/// assert_eq!(dates[0], Date::from_calendar_date(2025, Month::January, 15).unwrap());
/// assert_eq!(dates[3], Date::from_calendar_date(2025, Month::April, 15).unwrap());
/// ```
///
/// ## Short-back stub example
/// ```
/// use finstack_core::dates::{ScheduleBuilder, Frequency, StubKind};
/// use time::{Date, Month};
///
/// // Period not evenly divisible by quarterly frequency
/// let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
/// let end = Date::from_calendar_date(2025, Month::November, 1).unwrap();  // 10 months
///
/// let dates: Vec<_> = ScheduleBuilder::new(start, end)
///     .frequency(Frequency::quarterly())  // 3-month periods
///     .stub_rule(StubKind::None)  // For now use None to see default behavior
///     .build_raw()
///     .collect();
///
/// // Debug: print actual dates
/// // for (i, d) in dates.iter().enumerate() {
///     // println!("{}: {}", i, d);
/// // }
///     
/// // With StubKind::None, we get all dates including partial period at end
/// assert_eq!(dates.len(), 5);  // Jan, Apr, Jul, Oct, Nov
/// assert_eq!(dates[0], Date::from_calendar_date(2025, Month::January, 1).unwrap());
/// assert_eq!(dates[1], Date::from_calendar_date(2025, Month::April, 1).unwrap());
/// assert_eq!(dates[2], Date::from_calendar_date(2025, Month::July, 1).unwrap());
/// assert_eq!(dates[3], Date::from_calendar_date(2025, Month::October, 1).unwrap());
/// assert_eq!(dates[4], Date::from_calendar_date(2025, Month::November, 1).unwrap());
/// ```
///
/// ## Short-front stub example
/// ```
/// use finstack_core::dates::{ScheduleBuilder, Frequency, StubKind};
/// use time::{Date, Month};
///
/// // Same period with short-front stub
/// let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
/// let end = Date::from_calendar_date(2025, Month::November, 1).unwrap();
///
/// let dates: Vec<_> = ScheduleBuilder::new(start, end)
///     .frequency(Frequency::quarterly())
///     .stub_rule(StubKind::ShortFront)
///     .build_raw()
///     .collect();
///     
/// // Short stub first: Jan-Feb (1 month), then regular quarters
/// assert_eq!(dates.len(), 5);  // Start, Feb, May, Aug, Nov
/// assert_eq!(dates[0], Date::from_calendar_date(2025, Month::January, 1).unwrap());
/// assert_eq!(dates[1], Date::from_calendar_date(2025, Month::February, 1).unwrap());
/// assert_eq!(dates[2], Date::from_calendar_date(2025, Month::May, 1).unwrap());
/// assert_eq!(dates[3], Date::from_calendar_date(2025, Month::August, 1).unwrap());
/// assert_eq!(dates[4], Date::from_calendar_date(2025, Month::November, 1).unwrap());
/// ```
///
/// ## Business day adjustment
/// ```
/// use finstack_core::dates::{ScheduleBuilder, Frequency, BusinessDayConvention, calendars::Target2};
/// use time::{Date, Month};
///
/// let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();  // Wed
/// let end = Date::from_calendar_date(2025, Month::July, 1).unwrap();
///
/// let cal = Target2::new();
/// let dates: Vec<_> = ScheduleBuilder::new(start, end)
///     .frequency(Frequency::quarterly())
///     .adjust_with(BusinessDayConvention::Following, &cal)
///     .build()
///     .collect();
///     
/// // Jan 1 is New Year's Day, adjusts to Jan 2
/// assert_eq!(dates[0], Date::from_calendar_date(2025, Month::January, 2).unwrap());
/// ```
#[derive(Clone, Copy)]
pub struct ScheduleBuilder<'a> {
    start: Date,
    end: Date,
    freq: Frequency,
    stub: StubKind,
    conv: Option<BusinessDayConvention>,
    cal: Option<&'a dyn HolidayCalendar>,
}

impl<'a> ScheduleBuilder<'a> {
    /// Create a new builder with mandatory `start` and `end` dates.
    /// Defaults: frequency = Monthly, stub = None, no adjustment.
    pub fn new(start: Date, end: Date) -> Self {
        Self {
            start,
            end,
            freq: Frequency::Months(1),
            stub: StubKind::None,
            conv: None,
            cal: None,
        }
    }

    /// Set coupon/payment frequency.
    #[must_use]
    pub fn frequency(mut self, freq: Frequency) -> Self {
        self.freq = freq;
        self
    }

    /// Set stub handling rule.
    #[must_use]
    #[allow(dead_code)]
    pub fn stub_rule(mut self, stub: StubKind) -> Self {
        self.stub = stub;
        self
    }

    /// Configure business-day adjustment using `conv` and `cal`.
    #[must_use]
    #[allow(dead_code)]
    pub fn adjust_with(
        mut self,
        conv: BusinessDayConvention,
        cal: &'a dyn HolidayCalendar,
    ) -> Self {
        self.conv = Some(conv);
        self.cal = Some(cal);
        self
    }

    /// Generate the schedule iterator.
    #[allow(dead_code)]
    pub fn build(self) -> Box<dyn Iterator<Item = Date> + 'a> {
        let builder = BuilderInternal {
            start: self.start,
            end: self.end,
            freq: self.freq,
            stub: self.stub,
        };

        let base_iter = builder.generate();

        // Wrap with business day adjustment if configured
        if let (Some(conv), Some(cal)) = (self.conv, self.cal) {
            Box::new(AdjustIter::new(base_iter, conv, cal))
        } else {
            Box::new(base_iter)
        }
    }

    /// Generate the schedule iterator without business day adjustment.
    pub fn build_raw(self) -> ScheduleIter {
        let builder = BuilderInternal {
            start: self.start,
            end: self.end,
            freq: self.freq,
            stub: self.stub,
        };

        builder.generate()
    }
}

// Rename original private Builder to avoid conflict and make internal.
struct BuilderInternal {
    start: Date,
    end: Date,
    freq: Frequency,
    stub: StubKind,
}

impl BuilderInternal {
    fn generate(self) -> ScheduleIter {
        debug_assert!(self.start <= self.end);

        let step = self.freq.to_step();

        if self.stub == StubKind::ShortFront {
            // ShortFront: Build schedule backwards from end date, then reverse.
            // This ensures any partial period (stub) appears at the start.
            let mut buf: Buffer = Buffer::new();
            let mut dt = self.end;

            loop {
                buf.push(dt);

                if dt == self.start {
                    break;
                }

                // Step backwards by frequency amount
                let prev = match step {
                    Step::Months(m) => add_months(dt, -m),
                    Step::Days(d) => dt - Duration::days(d as i64),
                };

                // Clamp to start date if we would undershoot
                let prev = if prev < self.start { self.start } else { prev };
                dt = prev;
            }

            // Reverse to get chronological order
            buf.as_mut_slice().reverse();
            return ScheduleIter::Buf { buf, idx: 0 };
        }

        // StubKind::None and ShortBack: Stream dates lazily from start to end.
        // Any partial period (stub) naturally appears at the end.
        let lazy = LazyIter {
            next_date: self.start,
            end: self.end,
            step,
            finished: false,
        };

        ScheduleIter::Lazy(lazy)
    }
}
