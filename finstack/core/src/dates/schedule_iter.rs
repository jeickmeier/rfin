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
use crate::dates::utils::add_months;

/// Small helper alias when we need to pre-buffer (used only for `ShortFront`).
type Buffer = SmallVec<[Date; 32]>;

/// Coupon/payment frequency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    #[deprecated(note = "Use bimonthly() instead")]
    pub const fn bi_monthly() -> Self {
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
    #[deprecated(note = "Use biweekly() instead")]
    pub const fn bi_weekly() -> Self {
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

/// Concrete return type used by `ScheduleBuilder::build` to avoid boxing.
pub enum MaybeAdjusted<'a> {
    Raw(ScheduleIter),
    Adjusted(AdjustIter<'a, ScheduleIter>),
}

impl<'a> Iterator for MaybeAdjusted<'a> {
    type Item = Date;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            MaybeAdjusted::Raw(it) => it.next(),
            MaybeAdjusted::Adjusted(it) => it.next(),
        }
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
/// Returns an iterator of anchor dates including both the start and end dates
/// (i.e., the sequence is inclusive of both anchors).
///
/// Panics if `start` > `end`.
///
/// Note: Inputs must satisfy `start` <= `end`.
///
/// Examples
/// ```
/// use finstack_core::dates::{Date, Frequency, schedule};
/// use time::Month;
///
/// let start = Date::from_calendar_date(2025, Month::January, 15).unwrap();
/// let end   = Date::from_calendar_date(2025, Month::April,   15).unwrap();
/// let dates: Vec<_> = schedule(start, end, Frequency::monthly()).collect();
/// assert_eq!(dates, vec![
///   start,
///   Date::from_calendar_date(2025, Month::February, 15).unwrap(),
///   Date::from_calendar_date(2025, Month::March,    15).unwrap(),
///   end,
/// ]);
/// ```
pub fn schedule(start: Date, end: Date, freq: Frequency) -> impl Iterator<Item = Date> {
    ScheduleBuilder::new(start, end).frequency(freq).build_raw()
}

/// Fallible variant of [`schedule`]: validates that `start` <= `end` and returns
/// an error rather than panicking when inputs are invalid.
///
/// Returns an iterator of anchor dates including both the start and end dates
/// (i.e., the sequence is inclusive of both anchors).
///
/// Errors with [`Error::Input(InputError::InvalidDateRange)`](crate::error::InputError::InvalidDateRange)
/// if `start` > `end`.
pub fn try_schedule(start: Date, end: Date, freq: Frequency) -> crate::Result<ScheduleIter> {
    ScheduleBuilder::try_new(start, end)?.frequency(freq).try_build_raw()
}

/// Public builder for configuring schedule generation with
/// fluent API (frequency, stub rule, business-day adjustment).
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
}

impl<'a> ScheduleBuilder<'a> {
    /// Create a new builder with mandatory `start` and `end` dates.
    /// Defaults: frequency = Monthly, stub = None, no adjustment.
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

    /// Generate the schedule iterator.
    pub fn build(self) -> impl Iterator<Item = Date> + 'a {
        let builder = BuilderInternal {
            start: self.start,
            end: self.end,
            freq: self.freq,
            stub: self.stub,
        };

        let base_iter = builder.generate();

        // Wrap with business day adjustment if configured
        match (self.conv, self.cal) {
            (Some(conv), Some(cal)) => MaybeAdjusted::Adjusted(AdjustIter::new(base_iter, conv, cal)),
            _ => MaybeAdjusted::Raw(base_iter),
        }
    }

    /// Fallible variant of [`build`]: returns an error when `start` > `end`.
    pub fn try_build(self) -> crate::Result<MaybeAdjusted<'a>> {
        if self.start > self.end {
            return Err(crate::error::InputError::InvalidDateRange.into());
        }

        let builder = BuilderInternal {
            start: self.start,
            end: self.end,
            freq: self.freq,
            stub: self.stub,
        };

        let base_iter = builder.generate();

        Ok(match (self.conv, self.cal) {
            (Some(conv), Some(cal)) => MaybeAdjusted::Adjusted(AdjustIter::new(base_iter, conv, cal)),
            _ => MaybeAdjusted::Raw(base_iter),
        })
    }

    /// Generate the schedule iterator without business day adjustment.
    ///
    /// This path is zero-allocation for `StubKind::None` and `StubKind::ShortBack`
    /// (streaming lazy iterator). For `StubKind::ShortFront`, a small fixed-size
    /// stack buffer is used internally.
    pub fn build_raw(self) -> ScheduleIter {
        let builder = BuilderInternal {
            start: self.start,
            end: self.end,
            freq: self.freq,
            stub: self.stub,
        };

        builder.generate()
    }

    /// Fallible variant of [`build_raw`]: returns an error when `start` > `end`.
    pub fn try_build_raw(self) -> crate::Result<ScheduleIter> {
        if self.start > self.end {
            return Err(crate::error::InputError::InvalidDateRange.into());
        }

        let builder = BuilderInternal {
            start: self.start,
            end: self.end,
            freq: self.freq,
            stub: self.stub,
        };

        Ok(builder.generate())
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
        if self.start > self.end {
            panic!(
                "ScheduleBuilder: start date must be <= end date (start={:?}, end={:?})",
                self.start, self.end
            );
        }

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
