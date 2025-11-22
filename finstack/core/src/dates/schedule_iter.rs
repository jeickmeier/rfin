//! Date schedule construction for cashflows, coupons, and payment dates.
//!
//! Provides a fluent builder API for constructing deterministic date schedules
//! with support for frequency-based generation, stub periods, end-of-month
//! conventions, and business day adjustments.
//!
//! # Features
//!
//! - **Frequency-based**: Monthly, quarterly, annual, or custom day intervals
//! - **Stub handling**: Short/long stubs at front or back of schedule
//! - **Business day adjustment**: Modified Following, Following, Preceding
//! - **End-of-month**: Snap to month-end for month-based frequencies
//! - **CDS IMM mode**: Standard credit default swap quarterly schedules
//! - **Deterministic**: Same inputs always produce identical outputs
//! - **Deduplication**: Automatically removes duplicate dates from EOM/adjustment
//!
//! # Quick Example
//!
//! Basic monthly schedule:
//! ```rust
//! use finstack_core::dates::{ScheduleBuilder, Frequency};
//! use time::{Date, Month};
//!
//! let start = Date::from_calendar_date(2025, Month::January, 15)?;
//! let end = Date::from_calendar_date(2025, Month::April, 15)?;
//!
//! let sched = ScheduleBuilder::new(start, end)
//!     .frequency(Frequency::monthly())
//!     .build()?;
//!
//! let dates: Vec<_> = sched.into_iter().collect();
//! assert_eq!(dates.len(), 4); // Jan-15, Feb-15, Mar-15, Apr-15
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! CDS IMM schedule (quarterly on 20-Mar/Jun/Sep/Dec):
//! ```rust
//! use finstack_core::dates::ScheduleBuilder;
//! use time::{Date, Month};
//!
//! let start = Date::from_calendar_date(2025, Month::January, 15)?;
//! let end = Date::from_calendar_date(2025, Month::December, 20)?;
//!
//! let sched = ScheduleBuilder::new(start, end)
//!     .cds_imm()  // Auto-adjusts start to next CDS roll date
//!     .build()?;
//!
//! let dates: Vec<_> = sched.into_iter().collect();
//! // Mar-20, Jun-20, Sep-20, Dec-20 (2025)
//! assert_eq!(dates.len(), 4);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! With business day adjustment:
//! ```rust
//! use finstack_core::dates::{ScheduleBuilder, Frequency, BusinessDayConvention};
//! use finstack_core::dates::calendar::registry::CalendarRegistry;
//! use time::{Date, Month};
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//!
//! let start = Date::from_calendar_date(2025, Month::June, 15)?;
//! let end = Date::from_calendar_date(2025, Month::December, 15)?;
//! let nyse = CalendarRegistry::global()
//!     .resolve_str("nyse")
//!     .ok_or("NYSE calendar not found")?;
//!
//! let sched = ScheduleBuilder::new(start, end)
//!     .frequency(Frequency::monthly())
//!     .adjust_with(BusinessDayConvention::ModifiedFollowing, nyse)
//!     .build()?;
//!
//! // Dates are adjusted to business days according to NYSE calendar
//! # Ok(())
//! # }
//! ```
//!
//! # Stub Conventions
//!
//! When start/end dates don't align exactly with the frequency:
//!
//! - **`StubKind::None`**: No special handling (default)
//! - **`StubKind::ShortFront`**: Short period at start, regular thereafter
//! - **`StubKind::ShortBack`**: Regular periods, short period at end
//! - **`StubKind::LongFront`**: Long period at start, regular thereafter
//! - **`StubKind::LongBack`**: Regular periods, long period at end
//!
//! # See Also
//!
//! - [`ScheduleBuilder`] for the main builder API
//! - [`Frequency`] for payment frequency options
//! - [`StubKind`] for stub period handling
//! - [`BusinessDayConvention`] for date adjustment rules
//!
//! [`BusinessDayConvention`]: super::BusinessDayConvention

#![allow(clippy::needless_lifetimes)]

use smallvec::SmallVec;
use time::{Date, Duration};

use super::{adjust, next_cds_date, BusinessDayConvention, HolidayCalendar};
use crate::dates::date_extensions::DateExt;

/// Small helper alias when we need to pre-buffer (used only for `ShortFront`).
type Buffer = SmallVec<[Date; 32]>;

/// Payment or coupon frequency for schedule generation.
///
/// Specifies how often payments occur in a financial instrument schedule.
/// Supports both calendar-month-based frequencies (e.g., quarterly, monthly)
/// and day-based frequencies (e.g., weekly, biweekly).
///
/// # Variants
///
/// - **`Months(n)`**: Period advances by `n` calendar months (1-12)
/// - **`Days(n)`**: Period advances by `n` calendar days (1+)
///
/// # Examples
///
/// Using predefined frequency constructors:
/// ```rust
/// use finstack_core::dates::Frequency;
///
/// let quarterly = Frequency::quarterly();
/// assert_eq!(quarterly.months(), Some(3));
///
/// let weekly = Frequency::weekly();
/// assert_eq!(weekly.days(), Some(7));
/// ```
///
/// Creating from payments per year:
/// ```rust
/// use finstack_core::dates::Frequency;
///
/// // 4 payments per year = quarterly
/// let freq = Frequency::from_payments_per_year(4)?;
/// assert_eq!(freq, Frequency::quarterly());
///
/// // 2 payments per year = semi-annual
/// let freq = Frequency::from_payments_per_year(2)?;
/// assert_eq!(freq, Frequency::semi_annual());
/// # Ok::<(), String>(())
/// ```
///
/// # See Also
///
/// - [`ScheduleBuilder::frequency`] to use with schedule builder
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum Frequency {
    /// Calendar-month based frequency (e.g., 3 = quarterly).
    ///
    /// Valid range: 1-12 months.
    Months(u8), // 1..=12

    /// Day-based frequency (e.g., 14 = biweekly, 7 = weekly).
    ///
    /// Valid range: 1+ days.
    Days(u16), // >0
}

impl Frequency {
    /// Returns the number of months if this frequency is month-based.
    ///
    /// Returns `None` if the frequency is day-based.
    #[inline]
    pub const fn months(self) -> Option<u8> {
        match self {
            Self::Months(m) => Some(m),
            _ => None,
        }
    }

    /// Returns the number of days if this frequency is day-based.
    ///
    /// Returns `None` if the frequency is month-based.
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

    /// Returns a frequency of 12 months (annual).
    pub const fn annual() -> Self {
        Self::Months(12)
    }

    /// Returns a frequency of 6 months (semi-annual).
    pub const fn semi_annual() -> Self {
        Self::Months(6)
    }

    /// Every two months.
    pub const fn bimonthly() -> Self {
        Self::Months(2)
    }

    /// Returns a frequency of 3 months (quarterly).
    pub const fn quarterly() -> Self {
        Self::Months(3)
    }

    /// Returns a frequency of 1 month (monthly).
    pub const fn monthly() -> Self {
        Self::Months(1)
    }

    /// Returns a frequency of 14 days (biweekly).
    pub const fn biweekly() -> Self {
        Self::Days(14)
    }

    /// Returns a frequency of 7 days (weekly).
    pub const fn weekly() -> Self {
        Self::Days(7)
    }

    /// Returns a frequency of 1 day (daily).
    pub const fn daily() -> Self {
        Self::Days(1)
    }

    /// Create a Frequency from payments per year.
    ///
    /// Returns an error if payments_per_year is 0 or does not divide 12 evenly.
    ///
    /// # Examples
    /// ```
    /// use finstack_core::dates::Frequency;
    ///
    /// // Valid frequencies
    /// assert_eq!(Frequency::from_payments_per_year(4).expect("Frequency creation should succeed"), Frequency::quarterly());
    /// assert_eq!(Frequency::from_payments_per_year(2).expect("Frequency creation should succeed"), Frequency::semi_annual());
    /// assert_eq!(Frequency::from_payments_per_year(12).expect("Frequency creation should succeed"), Frequency::monthly());
    ///
    /// // Error handling for invalid inputs
    /// assert!(Frequency::from_payments_per_year(0).is_err());
    /// assert!(Frequency::from_payments_per_year(5).is_err()); // Doesn't divide 12
    ///
    /// // Proper error handling in production code
    /// let freq = Frequency::from_payments_per_year(4)
    ///     .map_err(|e| format!("Invalid payment frequency: {}", e))?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn from_payments_per_year(payments: u32) -> std::result::Result<Self, String> {
        if payments == 0 {
            return Err("payments_per_year must be positive".to_string());
        }
        if 12 % payments != 0 {
            return Err(format!(
                "payments_per_year must divide 12 evenly (e.g., 1, 2, 3, 4, 6, 12), got {}",
                payments
            ));
        }
        let months = (12 / payments) as u8;
        Ok(Self::Months(months))
    }
}

/// Stub period handling when start/end dates don't align with payment frequency.
///
/// Controls how schedules are generated when the start and end dates don't
/// divide evenly by the payment frequency, resulting in an irregular period
/// (stub) at the beginning or end of the schedule.
///
/// # Variants
///
/// - **`None`**: No special stub handling (default). Generates regular periods
///   from start to end, with the final period potentially irregular.
/// - **`ShortFront`**: Short stub period at the start. Schedule is built
///   backward from the end date, creating a short first period.
/// - **`ShortBack`**: Short stub period at the end. Schedule is built forward
///   from the start date, creating a short final period.
/// - **`LongFront`**: Long stub period at the start. Combines the first two
///   periods into a single longer period.
/// - **`LongBack`**: Long stub period at the end. Combines the last two periods
///   into a single longer period.
///
/// # Financial Context
///
/// Stub conventions are important for:
/// - Interest accrual calculations (short/long first coupons)
/// - Cash flow present value computations
/// - Matching market conventions for specific instruments
///
/// # Examples
///
/// ```rust
/// use finstack_core::dates::{ScheduleBuilder, Frequency, StubKind};
/// use time::{Date, Month};
///
/// let start = Date::from_calendar_date(2025, Month::January, 10)?;
/// let end = Date::from_calendar_date(2025, Month::December, 15)?;
///
/// // Short stub at front
/// let sched = ScheduleBuilder::new(start, end)
///     .frequency(Frequency::quarterly())
///     .stub_rule(StubKind::ShortFront)
///     .build()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # See Also
///
/// - [`ScheduleBuilder::stub_rule`] to configure stub behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum StubKind {
    /// No special stub handling.
    None,
    /// Short stub period at the beginning of the schedule.
    ShortFront,
    /// Short stub period at the end of the schedule (final step truncated to maturity).
    ShortBack,
    /// Long stub period at the beginning of the schedule.
    LongFront,
    /// Long stub period at the end of the schedule (merges final two periods).
    LongBack,
}

impl std::fmt::Display for StubKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StubKind::None => write!(f, "none"),
            StubKind::ShortFront => write!(f, "short_front"),
            StubKind::ShortBack => write!(f, "short_back"),
            StubKind::LongFront => write!(f, "long_front"),
            StubKind::LongBack => write!(f, "long_back"),
        }
    }
}

impl std::str::FromStr for StubKind {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let normalized = s.to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "none" => Ok(StubKind::None),
            "short_front" => Ok(StubKind::ShortFront),
            "short_back" => Ok(StubKind::ShortBack),
            "long_front" => Ok(StubKind::LongFront),
            "long_back" => Ok(StubKind::LongBack),
            other => Err(format!("Unknown stub kind: {}", other)),
        }
    }
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
            Step::Months(m) => date.add_months(m),
            Step::Days(d) => date + Duration::days(d as i64),
        }
    }
}

/// Apply End-of-Month (EOM) convention to a date.
/// Returns the last day of the month for the given date.
fn apply_eom(date: Date) -> Date {
    date.end_of_month()
}

#[inline]
fn maybe_eom(eom: bool, d: Date) -> Date {
    if eom {
        apply_eom(d)
    } else {
        d
    }
}

#[inline]
fn push_if_new(buf: &mut Buffer, d: Date) {
    if buf.last().copied() != Some(d) {
        buf.push(d)
    }
}

/// Concrete schedule containing generated payment/coupon dates.
///
/// Represents the output of schedule generation: a sequence of dates
/// for cashflows, coupon payments, or other periodic events. Dates are
/// guaranteed to be monotonically increasing with no duplicates.
///
/// # Invariants
///
/// - Dates are strictly increasing (no duplicates)
/// - Empty schedules are allowed (zero-length Vec)
/// - All dates are valid `time::Date` values
///
/// # Examples
///
/// ```rust
/// use finstack_core::dates::{ScheduleBuilder, Frequency};
/// use time::{Date, Month};
///
/// let start = Date::from_calendar_date(2025, Month::January, 15)?;
/// let end = Date::from_calendar_date(2025, Month::March, 15)?;
///
/// let schedule = ScheduleBuilder::new(start, end)
///     .frequency(Frequency::monthly())
///     .build()?;
///
/// // Iterate over dates
/// for date in schedule.into_iter() {
///     println!("Payment date: {}", date);
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # See Also
///
/// - [`ScheduleBuilder`] for constructing schedules
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Schedule {
    /// The generated sequence of dates, monotonically increasing.
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

/// Fluent builder for constructing date schedules with full configurability.
///
/// Provides a type-safe, fluent API for generating payment/coupon schedules
/// with support for frequency, stub periods, business day adjustments, and
/// end-of-month conventions.
///
/// # Configuration Options
///
/// - **Frequency**: Monthly, quarterly, annual, or day-based intervals
/// - **Stub handling**: Short/long stubs at front or back
/// - **Business day adjustment**: Following, Modified Following, Preceding
/// - **End-of-month**: Snap to last day of month for month-based frequencies
/// - **CDS IMM mode**: Standard CDS quarterly schedule (auto-adjusts start)
///
/// # Construction Flow
///
/// 1. Create builder with `new(start, end)`
/// 2. Configure options via fluent methods
/// 3. Call `build()` to generate the [`Schedule`]
///
/// # Examples
///
/// Basic quarterly schedule:
/// ```rust
/// use finstack_core::dates::{ScheduleBuilder, Frequency};
/// use time::{Date, Month};
///
/// let start = Date::from_calendar_date(2025, Month::March, 20)?;
/// let end = Date::from_calendar_date(2025, Month::December, 20)?;
///
/// let schedule = ScheduleBuilder::new(start, end)
///     .frequency(Frequency::quarterly())
///     .build()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// With business day adjustment:
/// ```rust
/// use finstack_core::dates::{ScheduleBuilder, Frequency, BusinessDayConvention};
/// use finstack_core::dates::calendar::registry::CalendarRegistry;
/// use time::{Date, Month};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
///
/// let start = Date::from_calendar_date(2025, Month::January, 15)?;
/// let end = Date::from_calendar_date(2025, Month::December, 15)?;
/// let nyse = CalendarRegistry::global()
///     .resolve_str("nyse")
///     .ok_or("NYSE calendar not found")?;
///
/// let schedule = ScheduleBuilder::new(start, end)
///     .frequency(Frequency::monthly())
///     .adjust_with(BusinessDayConvention::ModifiedFollowing, nyse)
///     .build()?;
/// # Ok(())
/// # }
/// ```
///
/// CDS IMM schedule:
/// ```rust
/// use finstack_core::dates::ScheduleBuilder;
/// use time::{Date, Month};
///
/// let start = Date::from_calendar_date(2025, Month::January, 15)?;
/// let end = Date::from_calendar_date(2026, Month::December, 20)?;
///
/// let schedule = ScheduleBuilder::new(start, end)
///     .cds_imm()  // Quarterly on 20-Mar/Jun/Sep/Dec
///     .build()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// End-of-month convention:
/// ```rust
/// use finstack_core::dates::{ScheduleBuilder, Frequency};
/// use time::{Date, Month};
///
/// let start = Date::from_calendar_date(2025, Month::January, 31)?;
/// let end = Date::from_calendar_date(2025, Month::June, 30)?;
///
/// let schedule = ScheduleBuilder::new(start, end)
///     .frequency(Frequency::monthly())
///     .end_of_month(true)  // Snap to month-end
///     .build()?;
///
/// // Generates: Jan-31, Feb-28, Mar-31, Apr-30, May-31, Jun-30
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # See Also
///
/// - [`Frequency`] for payment frequency options
/// - [`StubKind`] for stub period handling
/// - [`BusinessDayConvention`] for adjustment rules
///
/// [`BusinessDayConvention`]: super::BusinessDayConvention
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
    graceful: bool,
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
            graceful: false,
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

    /// Enable graceful fallback mode.
    ///
    /// When enabled, [`build()`](Self::build) returns an empty schedule on errors
    /// instead of propagating them. This is useful for instrument pricing where
    /// you want to avoid panics but can handle empty schedules gracefully.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::dates::{ScheduleBuilder, Frequency};
    /// use time::{Date, Month};
    ///
    /// let start = Date::from_calendar_date(2025, Month::December, 31).expect("Valid date");
    /// let end = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date"); // Invalid: end before start
    ///
    /// // Without graceful mode: returns error
    /// let result = ScheduleBuilder::new(start, end)
    ///     .frequency(Frequency::monthly())
    ///     .build();
    /// assert!(result.is_err());
    ///
    /// // With graceful mode: returns empty schedule
    /// let schedule = ScheduleBuilder::new(start, end)
    ///     .frequency(Frequency::monthly())
    ///     .graceful_fallback(true)
    ///     .build()
    ///     .expect("Schedule builder should succeed");
    /// assert_eq!(schedule.dates.len(), 0);
    /// ```
    #[must_use]
    pub fn graceful_fallback(mut self, enabled: bool) -> Self {
        self.graceful = enabled;
        self
    }

    /// Configure business-day adjustment using calendar ID string lookup.
    ///
    /// This is a convenience method that combines calendar lookup with adjustment
    /// configuration. If the calendar is not found:
    /// - In strict mode (graceful=false): schedule generation will proceed without adjustment
    /// - In graceful mode (graceful=true): schedule generation will proceed without adjustment
    ///
    /// # Arguments
    ///
    /// * `conv` - Business day convention (Following, Modified Following, etc.)
    /// * `calendar_id` - Calendar identifier string (e.g., "nyse", "target2", "gblo")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::dates::{ScheduleBuilder, Frequency, BusinessDayConvention};
    /// use time::{Date, Month};
    ///
    /// let start = Date::from_calendar_date(2025, Month::January, 15).expect("Valid date");
    /// let end = Date::from_calendar_date(2025, Month::December, 15).expect("Valid date");
    ///
    /// let schedule = ScheduleBuilder::new(start, end)
    ///     .frequency(Frequency::monthly())
    ///     .adjust_with_id(BusinessDayConvention::Following, "nyse")
    ///     .build()
    ///     .expect("Schedule builder should succeed");
    /// # assert!(schedule.dates.len() > 0);
    /// ```
    #[must_use]
    pub fn adjust_with_id(mut self, conv: BusinessDayConvention, calendar_id: &str) -> Self {
        use super::calendar::calendar_by_id;

        if let Some(cal) = calendar_by_id(calendar_id) {
            self.conv = Some(conv);
            self.cal = Some(cal);
        }
        // If calendar not found, silently skip adjustment
        // The schedule will be generated without business day adjustment
        self
    }

    /// Build a concrete schedule (adjusted if configured).
    ///
    /// When graceful fallback mode is enabled via [`graceful_fallback(true)`](Self::graceful_fallback),
    /// this method returns an empty schedule on errors instead of propagating them.
    pub fn build(self) -> crate::Result<Schedule> {
        let result = self.build_impl();

        if self.graceful && result.is_err() {
            return Ok(Schedule { dates: Vec::new() });
        }

        result
    }

    /// Internal implementation of schedule building.
    fn build_impl(self) -> crate::Result<Schedule> {
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

#[cfg(feature = "serde")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
/// Serializable specification for building a schedule.
///
/// This struct captures all parameters needed to generate a schedule of dates
/// for cashflows, coupons, or other periodic events. It can be deserialized
/// from configuration files and converted to a runtime [`ScheduleBuilder`].
pub struct ScheduleSpec {
    /// Start date of the schedule.
    pub start: Date,
    /// End date (maturity) of the schedule.
    pub end: Date,
    /// Payment frequency (e.g., quarterly, monthly).
    pub frequency: Frequency,
    /// Stub convention (short/long front/back).
    pub stub: StubKind,
    /// Business day convention for adjusting dates.
    pub business_day_convention: Option<BusinessDayConvention>,
    /// Optional calendar identifier for holiday adjustments.
    pub calendar_id: Option<String>,
    /// If true, always roll to end of month when applicable.
    pub end_of_month: bool,
    /// If true, use CDS IMM date logic.
    pub cds_imm_mode: bool,
    /// If true, allow graceful handling of edge cases.
    pub graceful: bool,
}

#[cfg(feature = "serde")]
impl ScheduleSpec {
    /// Reconstruct a [`Schedule`] using the persisted configuration.
    pub fn build(&self) -> crate::Result<Schedule> {
        let mut builder = ScheduleBuilder::new(self.start, self.end)
            .frequency(self.frequency)
            .stub_rule(self.stub)
            .end_of_month(self.end_of_month)
            .graceful_fallback(self.graceful);

        if let (Some(conv), Some(id)) = (self.business_day_convention, self.calendar_id.as_deref())
        {
            builder = builder.adjust_with_id(conv, id);
        }

        if self.cds_imm_mode {
            builder = builder.cds_imm();
        }

        builder.build()
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
            StubKind::None => self.gen_regular(step),
            StubKind::ShortBack => self.gen_short_back(step),
        }
    }

    fn gen_regular(self, step: Step) -> Vec<Date> {
        let mut buf: Buffer = Buffer::new();
        let (mut dt, end) = (
            maybe_eom(self.eom, self.start),
            maybe_eom(self.eom, self.end),
        );
        buf.push(dt);
        while dt < end {
            let mut next = step.add(dt);
            if next > end {
                next = end;
            }
            dt = maybe_eom(self.eom, next);
            push_if_new(&mut buf, dt);
        }
        buf.into_vec()
    }

    fn gen_short_back(self, step: Step) -> Vec<Date> {
        // Short back stub is naturally produced by forward generation that truncates the final step.
        self.gen_regular(step)
    }

    fn gen_short_front(self, step: Step) -> Vec<Date> {
        // Build backwards from end, then reverse
        let mut buf: Buffer = Buffer::new();
        let mut dt = self.end;
        let target = self.start;
        loop {
            let date_to_add = maybe_eom(self.eom, dt);
            push_if_new(&mut buf, date_to_add);
            if dt == target {
                break;
            }
            let prev = match step {
                Step::Months(m) => dt.add_months(-m),
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
                Step::Months(m) => dt.add_months(-m),
                Step::Days(d) => dt - Duration::days(d as i64),
            };
            if prev >= self.start {
                dt = prev;
                anchors.push(dt);
            } else {
                break;
            }
        }
        buf.push(maybe_eom(self.eom, self.start));
        for &a in anchors.iter().rev() {
            let d = maybe_eom(self.eom, a);
            push_if_new(&mut buf, d);
        }
        buf.into_vec()
    }

    fn gen_long_back(self, step: Step) -> Vec<Date> {
        let mut buf: Buffer = Buffer::new();
        let mut dt = self.start;
        buf.push(maybe_eom(self.eom, dt));
        while dt < self.end {
            let next = step.add(dt);
            let next_after = step.add(next);
            if next_after >= self.end {
                let end_date = maybe_eom(self.eom, self.end);
                push_if_new(&mut buf, end_date);
                break;
            } else {
                let d = maybe_eom(self.eom, next);
                push_if_new(&mut buf, d);
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
    // In-place deduplication and monotonic filtering
    let mut write = 0;
    for read in 1..dates.len() {
        if dates[read] > dates[write] {
            write += 1;
            // Avoid self-assignment if indices match
            if read != write {
                dates[write] = dates[read];
            }
        }
    }
    dates.truncate(write + 1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    fn d(y: i32, m: u8, day: u8) -> Date {
        Date::from_calendar_date(y, Month::try_from(m).expect("Valid month (1-12)"), day)
            .expect("Valid test date")
    }

    #[test]
    fn test_graceful_fallback_returns_empty_on_invalid_range() {
        // Invalid: end before start
        let start = d(2025, 12, 31);
        let end = d(2025, 1, 1);

        // Without graceful mode: should error
        let result = ScheduleBuilder::new(start, end)
            .frequency(Frequency::monthly())
            .build();
        assert!(result.is_err());

        // With graceful mode: should return empty schedule
        let schedule = ScheduleBuilder::new(start, end)
            .frequency(Frequency::monthly())
            .graceful_fallback(true)
            .build()
            .expect("Schedule builder should succeed with valid test data");
        assert_eq!(schedule.dates.len(), 0);
    }

    #[test]
    fn test_adjust_with_id_valid_calendar() {
        let start = d(2025, 1, 15);
        let end = d(2025, 3, 15);

        // Use a known calendar (target2)
        let schedule = ScheduleBuilder::new(start, end)
            .frequency(Frequency::monthly())
            .adjust_with_id(BusinessDayConvention::Following, "target2")
            .build()
            .expect("Schedule builder should succeed with valid test data");

        // Should have generated a schedule
        assert!(schedule.dates.len() >= 2);
    }

    #[test]
    fn test_adjust_with_id_invalid_calendar_strict_mode() {
        let start = d(2025, 1, 15);
        let end = d(2025, 3, 15);

        // Invalid calendar with strict mode (graceful=false)
        // Should succeed but without adjustment since calendar not found
        let schedule = ScheduleBuilder::new(start, end)
            .frequency(Frequency::monthly())
            .adjust_with_id(BusinessDayConvention::Following, "INVALID_CALENDAR")
            .build()
            .expect("Schedule builder should succeed with valid test data");

        // Should still generate schedule (unadjusted)
        assert!(schedule.dates.len() >= 2);
    }

    #[test]
    fn test_adjust_with_id_invalid_calendar_graceful_mode() {
        let start = d(2025, 1, 15);
        let end = d(2025, 3, 15);

        // Invalid calendar with graceful mode
        // Should succeed and return schedule without adjustment
        let schedule = ScheduleBuilder::new(start, end)
            .frequency(Frequency::monthly())
            .adjust_with_id(BusinessDayConvention::Following, "INVALID_CALENDAR")
            .graceful_fallback(true)
            .build()
            .expect("Schedule builder should succeed with valid test data");

        // Should generate schedule (unadjusted)
        assert!(schedule.dates.len() >= 2);
    }

    #[test]
    fn test_graceful_mode_with_valid_inputs() {
        let start = d(2025, 1, 15);
        let end = d(2025, 4, 15);

        // Valid inputs with graceful mode should work normally
        let schedule = ScheduleBuilder::new(start, end)
            .frequency(Frequency::monthly())
            .graceful_fallback(true)
            .build()
            .expect("Schedule builder should succeed with valid test data");

        assert_eq!(schedule.dates.len(), 4);
        assert_eq!(schedule.dates[0], start);
        assert_eq!(schedule.dates[3], end);
    }

    #[test]
    fn test_adjust_with_id_combined_with_other_options() {
        let start = d(2025, 1, 31);
        let end = d(2025, 4, 30);

        // Combine adjust_with_id with end_of_month
        let schedule = ScheduleBuilder::new(start, end)
            .frequency(Frequency::monthly())
            .end_of_month(true)
            .adjust_with_id(BusinessDayConvention::Following, "target2")
            .build()
            .expect("Schedule builder should succeed with valid test data");

        // Should generate a valid schedule
        assert!(schedule.dates.len() >= 2);
    }

    #[test]
    fn stub_short_back_truncates_last_period() {
        let start = d(2025, 1, 15);
        let end = d(2025, 5, 20);

        let schedule = ScheduleBuilder::new(start, end)
            .frequency(Frequency::monthly())
            .stub_rule(StubKind::ShortBack)
            .build()
            .expect("Schedule builder should succeed with ShortBack");

        assert_eq!(
            schedule.dates,
            vec![
                d(2025, 1, 15),
                d(2025, 2, 15),
                d(2025, 3, 15),
                d(2025, 4, 15),
                d(2025, 5, 15),
                end
            ]
        );
    }

    #[test]
    fn stub_long_back_merges_final_two_periods() {
        let start = d(2025, 1, 15);
        let end = d(2025, 5, 20);

        let schedule = ScheduleBuilder::new(start, end)
            .frequency(Frequency::monthly())
            .stub_rule(StubKind::LongBack)
            .build()
            .expect("Schedule builder should succeed with LongBack");

        assert_eq!(
            schedule.dates,
            vec![d(2025, 1, 15), d(2025, 2, 15), d(2025, 3, 15), d(2025, 4, 15), end]
        );
    }
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
            let json =
                serde_json::to_string(&freq).expect("JSON serialization should succeed in test");
            let deserialized: Frequency =
                serde_json::from_str(&json).expect("JSON deserialization should succeed in test");
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
            let json =
                serde_json::to_string(&stub).expect("JSON serialization should succeed in test");
            let deserialized: StubKind =
                serde_json::from_str(&json).expect("JSON deserialization should succeed in test");
            assert_eq!(stub, deserialized);
        }
    }

    #[test]
    fn test_schedule_serde_roundtrip() {
        use serde_json;

        // Create a schedule
        let start = Date::from_calendar_date(2025, Month::January, 15).expect("Valid test date");
        let end = Date::from_calendar_date(2025, Month::April, 15).expect("Valid test date");
        let sched = ScheduleBuilder::new(start, end)
            .frequency(Frequency::monthly())
            .build()
            .expect("Schedule builder should succeed with valid test data");

        let json =
            serde_json::to_string(&sched).expect("JSON serialization should succeed in test");
        let deserialized: Schedule =
            serde_json::from_str(&json).expect("JSON deserialization should succeed in test");

        assert_eq!(sched.dates.len(), deserialized.dates.len());
        for (original, deserialized) in sched.dates.iter().zip(deserialized.dates.iter()) {
            assert_eq!(original, deserialized);
        }
    }
}
