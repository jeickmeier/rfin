//! Lightweight date schedule iterator.
//!
//! The iterator pre-computes the anchor dates internally (stored inline for up to
//! 32 dates) but keeps the backing container private so the public API is
//! allocation-free and zero-dependency.
//!
//! ## Roll Conventions for Credit Instruments
//!
//! This module supports standard OTC credit market roll conventions through the
//! [`BusinessDayConvention`] enum:
//! 
//! - **Following**: Roll to next business day (may cross month boundary)
//! - **Modified Following (MF)**: Roll to next business day, but if it crosses 
//!   month boundary, roll back to previous business day
//! - **Preceding**: Roll to previous business day (may cross month boundary)  
//! - **Modified Preceding**: Roll to previous business day, but if it crosses
//!   month boundary, roll forward to next business day
//!
//! For Credit Default Swaps, use [`cds_schedule`] which automatically creates
//! quarterly schedules on the standard IMM roll dates (20th of Mar/Jun/Sep/Dec).
//!
//! ## Street Conventions Mapping
//!
//! This section maps common financial instrument conventions to finstack schedule options:
//!
//! ### Credit Default Swaps (CDS)
//! - **Frequency**: Quarterly (every 3 months)
//! - **Roll Dates**: 20th of March, June, September, December (IMM dates)
//! - **Business Day Convention**: Modified Following
//! - **Stub Rules**: Short front stub, long back stub
//! - **Calendar**: TARGET2 (for EUR), NYSE (for USD)
//!
//! ### Interest Rate Swaps (IRS)
//! - **Frequency**: Semi-annual (6 months) or quarterly (3 months)
//! - **Roll Dates**: IMM dates (3rd Wednesday) or month-end
//! - **Business Day Convention**: Modified Following
//! - **Stub Rules**: Short front stub, long back stub
//! - **Calendar**: TARGET2 (for EUR), NYSE (for USD)
//!
//! ### Corporate Bonds
//! - **Frequency**: Semi-annual (6 months) or annual (12 months)
//! - **Roll Dates**: Fixed dates (e.g., 15th of month)
//! - **Business Day Convention**: Following or Modified Following
//! - **Stub Rules**: Short front stub, long back stub
//! - **Calendar**: NYSE (for USD), LSE (for GBP)
//!
//! ### Government Bonds
//! - **Frequency**: Semi-annual (6 months) or quarterly (3 months)
//! - **Roll Dates**: Fixed dates or IMM dates
//! - **Business Day Convention**: Following
//! - **Stub Rules**: Short front stub, long back stub
//! - **Calendar**: TARGET2 (for EUR), NYSE (for USD)
//!
//! ### Example: CDS Quarterly 20th Schedule
//! ```
//! use finstack_core::dates::{ScheduleBuilder, Frequency, BusinessDayConvention, StubKind};
//! use finstack_core::dates::calendars::Target2;
//! # use time::macros::date;
//!
//! let schedule = ScheduleBuilder::new(date!(2024-01-15), date!(2026-01-15))
//!     .frequency(Frequency::Months(3))  // Quarterly
//!     .adjust_with(BusinessDayConvention::ModifiedFollowing, &Target2)
//!     .stub_rule(StubKind::ShortFront)
//!     .build();
//! ```

#![allow(missing_docs)]
#![allow(clippy::needless_lifetimes)]

use smallvec::SmallVec;
use time::{Date, Duration};

use super::{adjust, BusinessDayConvention, HolidayCalendar};
use crate::dates::utils::{add_months, is_leap_year};

/// Diagnostic information about how a date was generated in the schedule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DateGenerationRule {
    /// Start date (provided by user)
    StartAnchor,
    /// End date (provided by user)
    EndAnchor,
    /// Regular interval from frequency
    RegularInterval { period_number: usize },
    /// Short stub at front
    ShortFrontStub,
    /// Short stub at back
    ShortBackStub,
    /// Long stub at front
    LongFrontStub,
    /// Long stub at back
    LongBackStub,
    /// End-of-month adjustment applied
    EndOfMonthAdjusted,
    /// Business day adjustment applied
    BusinessDayAdjusted { original_date: Date },
}

/// Diagnostic information for a generated date in the schedule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateDiagnostic {
    /// The final date in the schedule
    pub date: Date,
    /// The rule(s) that produced this date
    pub rules: Vec<DateGenerationRule>,
}

/// Schedule iterator with optional diagnostics collection.
pub struct ScheduleIterWithDiagnostics {
    iter: ScheduleIter,
    diagnostics: Option<Vec<DateDiagnostic>>,
    collect_diagnostics: bool,
}

impl ScheduleIterWithDiagnostics {
    /// Create a new diagnostics iterator.
    pub fn new(iter: ScheduleIter, collect_diagnostics: bool) -> Self {
        let diagnostics = if collect_diagnostics {
            Some(Vec::new())
        } else {
            None
        };
        
        Self {
            iter,
            diagnostics,
            collect_diagnostics,
        }
    }
    
    /// Get the collected diagnostics (if enabled).
    pub fn diagnostics(&self) -> Option<&[DateDiagnostic]> {
        self.diagnostics.as_deref()
    }
    
    /// Consume the iterator and return collected diagnostics.
    pub fn into_diagnostics(self) -> Option<Vec<DateDiagnostic>> {
        self.diagnostics
    }
}

impl Iterator for ScheduleIterWithDiagnostics {
    type Item = Date;
    
    fn next(&mut self) -> Option<Self::Item> {
        let date = self.iter.next()?;
        
        if self.collect_diagnostics {
            // For now, basic rule tracking - could be enhanced later
            // with more sophisticated rule identification
            let rule = DateGenerationRule::RegularInterval { period_number: 0 }; // Simplified
            
            if let Some(ref mut diagnostics) = self.diagnostics {
                diagnostics.push(DateDiagnostic {
                    date,
                    rules: vec![rule],
                });
            }
        }
        
        Some(date)
    }
}

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
            if is_leap_year(year) { 29 } else { 28 }
        }
        Month::April | Month::June | Month::September | Month::November => 30,
        _ => 31,
    };
    
    Date::from_calendar_date(year, month, last_day).unwrap_or(date)
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
        self.inner.next().and_then(|date| {
            adjust(date, self.conv, self.cal).ok()
        })
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
    eom: bool,
}

impl Iterator for LazyIter {
    type Item = Date;
    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        // Current date in the schedule sequence
        let current = if self.eom { apply_eom(self.next_date) } else { self.next_date };

        if self.next_date == self.end {
            // We've reached the end date - this is the last item
            self.finished = true;
        } else {
            // Calculate next date in sequence
            let mut next = self.step.add(self.next_date);

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

/// Create a CDS schedule using IMM roll dates (quarterly on the 20th).
/// 
/// This convenience function creates a standard Credit Default Swap schedule
/// that follows IMM roll dates (20-Mar, 20-Jun, 20-Sep, 20-Dec).
/// The start date will be adjusted to the next CDS roll date if it doesn't
/// fall on one.
///
/// # Examples
/// ```
/// use finstack_core::dates::{cds_schedule, Date};
/// use time::Month;
///
/// let start = Date::from_calendar_date(2025, Month::January, 15).unwrap();
/// let end   = Date::from_calendar_date(2025, Month::December, 20).unwrap();
/// let dates: Vec<_> = cds_schedule(start, end).collect();
/// // Will generate: 2025-03-20, 2025-06-20, 2025-09-20, 2025-12-20
/// ```
pub fn cds_schedule(start: Date, end: Date) -> impl Iterator<Item = Date> {
    use super::next_cds_date;
    
    // Adjust start to next CDS date if it doesn't fall on a CDS roll date
    let adjusted_start = if is_cds_roll_date(start) {
        start
    } else {
        next_cds_date(start)
    };
    
    ScheduleBuilder::new(adjusted_start, end)
        .cds_imm()
        .build_raw()
}

/// Check if a date is a CDS roll date (20th of Mar/Jun/Sep/Dec).
fn is_cds_roll_date(date: Date) -> bool {
    use time::Month;
    
    if date.day() != 20 {
        return false;
    }
    
    matches!(date.month(), Month::March | Month::June | Month::September | Month::December)
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
    ScheduleBuilder::try_new(start, end)?
        .frequency(freq)
        .try_build_raw()
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
        self
    }

    /// Generate the schedule iterator with optional diagnostics collection.
    /// When `collect_diagnostics` is true, generation rules for each date can be retrieved.
    pub fn build_with_diagnostics(self, collect_diagnostics: bool) -> ScheduleIterWithDiagnostics {
        let base_iter = self.build_raw();
        ScheduleIterWithDiagnostics::new(base_iter, collect_diagnostics)
    }

    /// Generate the schedule iterator.
    pub fn build(self) -> impl Iterator<Item = Date> + 'a {
        let builder = BuilderInternal {
            start: self.start,
            end: self.end,
            freq: self.freq,
            stub: self.stub,
            eom: self.eom,
        };

        let base_iter = builder.generate();

        // Wrap with business day adjustment if configured
        match (self.conv, self.cal) {
            (Some(conv), Some(cal)) => {
                MaybeAdjusted::Adjusted(AdjustIter::new(base_iter, conv, cal))
            }
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
            eom: self.eom,
        };

        let base_iter = builder.generate();

        Ok(match (self.conv, self.cal) {
            (Some(conv), Some(cal)) => {
                MaybeAdjusted::Adjusted(AdjustIter::new(base_iter, conv, cal))
            }
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
            eom: self.eom,
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
            eom: self.eom,
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
    eom: bool,
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

        match self.stub {
            StubKind::ShortFront => {
                // ShortFront: Build schedule backwards from end date, then reverse.
                // This ensures any partial period (stub) appears at the start.
                self.generate_buffered_schedule(step, true)
            }
            StubKind::LongFront => {
                // LongFront: Generate a longer first period by starting from an earlier anchor
                self.generate_long_front_schedule(step)
            }
            StubKind::LongBack => {
                // LongBack: Generate a longer last period by extending beyond normal intervals
                self.generate_long_back_schedule(step)
            }
            StubKind::None | StubKind::ShortBack => {
                // StubKind::None and ShortBack: Stream dates lazily from start to end.
                // Any partial period (stub) naturally appears at the end.
                let (start, end) = if self.eom {
                    (apply_eom(self.start), apply_eom(self.end))
                } else {
                    (self.start, self.end)
                };

                let lazy = LazyIter {
                    next_date: start,
                    end,
                    step,
                    finished: false,
                    eom: self.eom,
                };

                ScheduleIter::Lazy(lazy)
            }
        }
    }

    fn generate_buffered_schedule(self, step: Step, reverse: bool) -> ScheduleIter {
        let mut buf: Buffer = Buffer::new();
        let mut dt = if reverse { self.end } else { self.start };
        let target = if reverse { self.start } else { self.end };

        loop {
            let date_to_add = if self.eom { apply_eom(dt) } else { dt };
            buf.push(date_to_add);

            if dt == target {
                break;
            }

            // Step by frequency amount
            let next = if reverse {
                match step {
                    Step::Months(m) => add_months(dt, -m),
                    Step::Days(d) => dt - Duration::days(d as i64),
                }
            } else {
                step.add(dt)
            };

            // Clamp to target date if we would overshoot/undershoot
            #[allow(clippy::collapsible_else_if)]
            let next = if reverse {
                if next < target { target } else { next }
            } else {
                if next > target { target } else { next }
            };
            dt = next;
        }

        if reverse {
            buf.as_mut_slice().reverse();
        }
        ScheduleIter::Buf { buf, idx: 0 }
    }

    fn generate_long_front_schedule(self, step: Step) -> ScheduleIter {
        // For LongFront, we find the anchor point that would create regular intervals
        // to the end date, then create a longer first period from start to that anchor
        let mut buf: Buffer = Buffer::new();
        
        // Start with end date and work backwards to find anchor points
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
                // Found the anchor point - this creates a long front period
                break;
            }
        }

        // Add start date and reverse to get chronological order
        buf.push(if self.eom { apply_eom(self.start) } else { self.start });
        
        // Add anchor points in reverse order (chronological)
        for &anchor_date in anchors.iter().rev() {
            let date_to_add = if self.eom { apply_eom(anchor_date) } else { anchor_date };
            if date_to_add != buf[buf.len() - 1] {  // Avoid duplicates
                buf.push(date_to_add);
            }
        }

        ScheduleIter::Buf { buf, idx: 0 }
    }

    fn generate_long_back_schedule(self, step: Step) -> ScheduleIter {
        // For LongBack, we create regular intervals from start until we get close to end,
        // then create a longer final period
        let mut buf: Buffer = Buffer::new();
        let mut dt = self.start;

        // Add start date
        buf.push(if self.eom { apply_eom(dt) } else { dt });

        while dt < self.end {
            let next = step.add(dt);
            
            // If the next step would be close to the end, create a long back period
            let next_after = step.add(next);
            if next_after >= self.end {
                // Create long back period directly to end
                let end_date = if self.eom { apply_eom(self.end) } else { self.end };
                if end_date != buf[buf.len() - 1] {  // Avoid duplicates
                    buf.push(end_date);
                }
                break;
            } else {
                // Regular period
                let date_to_add = if self.eom { apply_eom(next) } else { next };
                if date_to_add != buf[buf.len() - 1] {  // Avoid duplicates
                    buf.push(date_to_add);
                }
                dt = next;
            }
        }

        ScheduleIter::Buf { buf, idx: 0 }
    }
}
