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
//! - **IMM mode**: Standard IMM quarterly schedules (third Wednesday of Mar/Jun/Sep/Dec)
//! - **CDS IMM mode**: Credit default swap quarterly schedules (20th of Mar/Jun/Sep/Dec)
//! - **Deterministic**: Same inputs always produce identical outputs
//! - **Deduplication**: Automatically removes duplicate dates from EOM/adjustment
//!
//! # Quick Example
//!
//! Basic monthly schedule:
//! ```rust
//! use finstack_core::dates::{ScheduleBuilder, Tenor};
//! use time::{Date, Month};
//!
//! let start = Date::from_calendar_date(2025, Month::January, 15)?;
//! let end = Date::from_calendar_date(2025, Month::April, 15)?;
//!
//! let sched = ScheduleBuilder::new(start, end)
//!     .frequency(Tenor::monthly())
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
//! Standard IMM schedule (quarterly on third Wednesday):
//! ```rust
//! use finstack_core::dates::ScheduleBuilder;
//! use time::{Date, Month};
//!
//! let start = Date::from_calendar_date(2025, Month::January, 15)?;
//! let end = Date::from_calendar_date(2025, Month::December, 31)?;
//!
//! let sched = ScheduleBuilder::new(start, end)
//!     .imm()  // Auto-adjusts start to next IMM date (third Wednesday)
//!     .build()?;
//!
//! let dates: Vec<_> = sched.into_iter().collect();
//! // Mar-19, Jun-18, Sep-17, Dec-17 (2025 third Wednesdays)
//! assert_eq!(dates.len(), 4);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! With business day adjustment:
//! ```rust
//! use finstack_core::dates::{ScheduleBuilder, Tenor, BusinessDayConvention};
//! use finstack_core::dates::CalendarRegistry;
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
//!     .frequency(Tenor::monthly())
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

use super::{adjust, next_cds_date, next_imm, BusinessDayConvention, HolidayCalendar};
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
/// Using predefined tenor constructors:
/// ```rust
/// use finstack_core::dates::Tenor;
///
/// let quarterly = Tenor::quarterly();
/// assert_eq!(quarterly.months(), Some(3));
///
/// let weekly = Tenor::weekly();
/// assert_eq!(weekly.days(), Some(7));
/// ```
///
/// Creating from payments per year:
/// ```rust
/// use finstack_core::dates::Tenor;
///
/// // 4 payments per year = quarterly
/// let freq = Tenor::from_payments_per_year(4)?;
/// assert_eq!(freq, Tenor::quarterly());
///
/// // 2 payments per year = semi-annual
/// let freq = Tenor::from_payments_per_year(2)?;
/// assert_eq!(freq, Tenor::semi_annual());
/// # Ok::<(), finstack_core::Error>(())
/// ```
///
/// # See Also
///
/// See Also
///
/// - [`ScheduleBuilder::frequency`] to use with schedule builder
/// - [`Tenor`] for the underlying time interval type
pub use crate::dates::Tenor;

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
/// use finstack_core::dates::{ScheduleBuilder, Tenor, StubKind};
/// use time::{Date, Month};
///
/// let start = Date::from_calendar_date(2025, Month::January, 10)?;
/// let end = Date::from_calendar_date(2025, Month::December, 15)?;
///
/// // Short stub at front
/// let sched = ScheduleBuilder::new(start, end)
///     .frequency(Tenor::quarterly())
///     .stub_rule(StubKind::ShortFront)
///     .build()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # See Also
///
/// - [`ScheduleBuilder::stub_rule`] to configure stub behavior
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum StubKind {
    /// No special stub handling.
    #[default]
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

/// Warning generated during schedule construction.
///
/// Warnings indicate non-fatal issues that occurred during schedule generation.
/// Unlike errors, these allow the schedule to be created but signal that
/// something unexpected happened that callers should be aware of.
///
/// # Use Cases
///
/// - **Graceful fallback**: When `graceful_fallback(true)` is set and an error
///   would normally occur, the builder returns an empty schedule with a warning
///   describing the original error.
///
/// # Examples
///
/// ```rust
/// use finstack_core::dates::{ScheduleBuilder, Tenor, ScheduleWarning};
/// use time::{Date, Month};
///
/// let start = Date::from_calendar_date(2025, Month::December, 31)?;
/// let end = Date::from_calendar_date(2025, Month::January, 1)?; // Invalid: end before start
///
/// let schedule = ScheduleBuilder::new(start, end)
///     .frequency(Tenor::monthly())
///     .graceful_fallback(true)
///     .build()?;
///
/// assert!(schedule.dates.is_empty());
/// assert!(schedule.has_warnings());
/// assert!(schedule.warnings.iter().any(|w| matches!(w, ScheduleWarning::GracefulFallback { .. })));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum ScheduleWarning {
    /// Schedule generation failed but graceful fallback returned an empty schedule.
    ///
    /// This warning captures the original error message that would have been
    /// returned if graceful fallback mode was not enabled. Callers should
    /// inspect this to understand why the schedule is empty.
    GracefulFallback {
        /// Human-readable description of the error that was suppressed.
        error_message: String,
    },

    /// A calendar ID was provided, but resolution was skipped because
    /// `allow_missing_calendar(true)` was enabled.
    MissingCalendarId {
        /// The calendar identifier that could not be resolved.
        calendar_id: String,
    },
}

impl std::fmt::Display for ScheduleWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GracefulFallback { error_message } => {
                write!(f, "graceful fallback triggered: {error_message}")
            }
            Self::MissingCalendarId { calendar_id } => {
                write!(
                    f,
                    "calendar id '{calendar_id}' not found; adjustment skipped"
                )
            }
        }
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
/// # Warnings
///
/// When using [`ScheduleBuilder::graceful_fallback(true)`](ScheduleBuilder::graceful_fallback),
/// the schedule may contain warnings that describe issues encountered during
/// generation. Always check [`has_warnings()`](Schedule::has_warnings) when
/// using graceful fallback mode to detect potential pricing issues.
///
/// # Examples
///
/// ```rust
/// use finstack_core::dates::{ScheduleBuilder, Tenor};
/// use time::{Date, Month};
///
/// let start = Date::from_calendar_date(2025, Month::January, 15)?;
/// let end = Date::from_calendar_date(2025, Month::March, 15)?;
///
/// let schedule = ScheduleBuilder::new(start, end)
///     .frequency(Tenor::monthly())
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
/// - [`ScheduleWarning`] for warning types
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Schedule {
    /// The generated sequence of dates, monotonically increasing.
    pub dates: Vec<Date>,
    /// Warnings generated during schedule construction.
    ///
    /// Non-empty when graceful fallback mode suppressed an error or when
    /// other non-fatal issues occurred during generation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<ScheduleWarning>,
}

impl Schedule {
    /// Returns `true` if any warnings were generated during schedule construction.
    ///
    /// When using graceful fallback mode, this should be checked to ensure
    /// the schedule was generated successfully. An empty schedule with warnings
    /// indicates a generation error was suppressed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::dates::{ScheduleBuilder, Tenor};
    /// use time::{Date, Month};
    ///
    /// let start = Date::from_calendar_date(2025, Month::December, 31)?;
    /// let end = Date::from_calendar_date(2025, Month::January, 1)?; // Invalid
    ///
    /// let schedule = ScheduleBuilder::new(start, end)
    ///     .frequency(Tenor::monthly())
    ///     .graceful_fallback(true)
    ///     .build()?;
    ///
    /// if schedule.has_warnings() {
    ///     // Handle degraded schedule - PV may be zero
    ///     for warning in &schedule.warnings {
    ///         eprintln!("Schedule warning: {warning}");
    ///     }
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Returns `true` if schedule generation used graceful fallback.
    ///
    /// This is a convenience method equivalent to checking for the presence
    /// of [`ScheduleWarning::GracefulFallback`] in the warnings.
    #[must_use]
    pub fn used_graceful_fallback(&self) -> bool {
        self.warnings
            .iter()
            .any(|w| matches!(w, ScheduleWarning::GracefulFallback { .. }))
    }
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
    crate::dates::imm::is_cds_date(date)
}

/// Check if a date is a standard IMM date (third Wednesday of Mar/Jun/Sep/Dec).
fn is_imm_roll_date(date: Date) -> bool {
    crate::dates::imm::is_imm_date(date)
}

/// Generate IMM dates (third Wednesday of Mar/Jun/Sep/Dec) within the given range.
///
/// Unlike regular schedule generation which adds fixed intervals, this function
/// computes the actual third Wednesday of each quarterly month to handle the
/// variable day-of-month correctly.
fn generate_imm_dates(start: Date, end: Date) -> Vec<Date> {
    let mut dates = Vec::new();

    // Find the first IMM date on or after start
    let first_imm = if is_imm_roll_date(start) {
        start
    } else {
        next_imm(start)
    };

    // If first IMM is already past end, return empty
    if first_imm > end {
        return dates;
    }

    dates.push(first_imm);

    // Keep adding IMM dates until we exceed end
    let mut current = first_imm;
    loop {
        let next = next_imm(current);
        if next > end {
            break;
        }
        dates.push(next);
        current = next;
    }

    dates
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
/// - **IMM mode**: Standard IMM quarterly schedule (third Wednesday of Mar/Jun/Sep/Dec)
/// - **CDS IMM mode**: CDS quarterly schedule (20th of Mar/Jun/Sep/Dec)
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
/// use finstack_core::dates::{ScheduleBuilder, Tenor};
/// use time::{Date, Month};
///
/// let start = Date::from_calendar_date(2025, Month::March, 20)?;
/// let end = Date::from_calendar_date(2025, Month::December, 20)?;
///
/// let schedule = ScheduleBuilder::new(start, end)
///     .frequency(Tenor::quarterly())
///     .build()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// With business day adjustment:
/// ```rust
/// use finstack_core::dates::{ScheduleBuilder, Tenor, BusinessDayConvention};
/// use finstack_core::dates::CalendarRegistry;
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
///     .frequency(Tenor::monthly())
///     .adjust_with(BusinessDayConvention::ModifiedFollowing, nyse)
///     .build()?;
/// # Ok(())
/// # }
/// ```
///
/// CDS IMM schedule (credit default swaps):
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
/// Standard IMM schedule (futures):
/// ```rust
/// use finstack_core::dates::ScheduleBuilder;
/// use time::{Date, Month};
///
/// let start = Date::from_calendar_date(2025, Month::January, 15)?;
/// let end = Date::from_calendar_date(2025, Month::December, 31)?;
///
/// let schedule = ScheduleBuilder::new(start, end)
///     .imm()  // Quarterly on third Wednesday of Mar/Jun/Sep/Dec
///     .build()?;
/// // Generates: Mar-19, Jun-18, Sep-17, Dec-17 (2025 third Wednesdays)
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// End-of-month convention:
/// ```rust
/// use finstack_core::dates::{ScheduleBuilder, Tenor};
/// use time::{Date, Month};
///
/// let start = Date::from_calendar_date(2025, Month::January, 31)?;
/// let end = Date::from_calendar_date(2025, Month::June, 30)?;
///
/// let schedule = ScheduleBuilder::new(start, end)
///     .frequency(Tenor::monthly())
///     .end_of_month(true)  // Snap to month-end
///     .build()?;
///
/// // Generates: Jan-31, Feb-28, Mar-31, Apr-30, May-31, Jun-30
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # See Also
///
/// - [`Tenor`] for payment frequency options
/// - [`StubKind`] for stub period handling
/// - [`BusinessDayConvention`] for adjustment rules
///
/// [`BusinessDayConvention`]: super::BusinessDayConvention
#[derive(Clone)]
pub struct ScheduleBuilder<'a> {
    start: Date,
    end: Date,
    freq: Tenor,
    stub: StubKind,
    conv: Option<BusinessDayConvention>,
    cal: Option<&'a dyn HolidayCalendar>,
    /// Pending calendar ID from `adjust_with_id` - resolved at build time.
    pending_calendar_id: Option<String>,
    eom: bool,
    /// Standard IMM mode (third Wednesday of Mar/Jun/Sep/Dec) for futures.
    imm_mode: bool,
    /// CDS IMM mode (20th of Mar/Jun/Sep/Dec) for credit default swaps.
    cds_imm_mode: bool,
    graceful: bool,
    allow_missing_calendar: bool,
}

impl<'a> ScheduleBuilder<'a> {
    /// Create a new builder with mandatory `start` and `end` dates.
    /// Defaults: frequency = Monthly, stub = None, no adjustment, no EOM.
    ///
    /// # Notes
    /// Inputs should satisfy `start` <= `end`. If not, [`build`](Self::build) returns
    /// `Err(InputError::InvalidDateRange)`.
    pub fn new(start: Date, end: Date) -> Self {
        Self {
            start,
            end,
            freq: Tenor::monthly(),
            stub: StubKind::None,
            conv: None,
            cal: None,
            pending_calendar_id: None,
            eom: false,
            imm_mode: false,
            cds_imm_mode: false,
            graceful: false,
            allow_missing_calendar: false,
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
    pub fn frequency(mut self, freq: Tenor) -> Self {
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
    /// standard CDS roll dates.
    #[must_use]
    pub fn cds_imm(mut self) -> Self {
        self.freq = Tenor::three_months();
        self.stub = StubKind::ShortBack;
        self.cds_imm_mode = true;
        self
    }

    /// Create a standard IMM schedule (quarterly on third Wednesday: Mar, Jun, Sep, Dec).
    ///
    /// This is used for interest rate futures (Eurodollar, SOFR), currency futures,
    /// and equity index futures that follow CME IMM roll conventions.
    ///
    /// Unlike [`cds_imm()`](Self::cds_imm) which uses the 20th of quarterly months,
    /// standard IMM dates fall on the third Wednesday.
    ///
    /// # Example
    /// ```rust
    /// use finstack_core::dates::ScheduleBuilder;
    /// use time::{Date, Month};
    ///
    /// let start = Date::from_calendar_date(2025, Month::January, 15)?;
    /// let end = Date::from_calendar_date(2025, Month::December, 31)?;
    ///
    /// let schedule = ScheduleBuilder::new(start, end)
    ///     .imm()  // Quarterly on third Wednesday
    ///     .build()?;
    ///
    /// // Generates: Mar-19, Jun-18, Sep-17, Dec-17 (2025 third Wednesdays)
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn imm(mut self) -> Self {
        self.freq = Tenor::three_months();
        self.stub = StubKind::ShortBack;
        self.imm_mode = true;
        self
    }

    /// Enable graceful fallback mode.
    ///
    /// When enabled, [`build()`](Self::build) returns an empty schedule with a
    /// [`ScheduleWarning::GracefulFallback`] warning on errors instead of propagating
    /// them. This is useful for instrument pricing where you want to avoid panics
    /// but need to detect degraded schedules.
    ///
    /// # Warning Detection
    ///
    /// **Always check [`Schedule::has_warnings()`]** when using graceful fallback mode.
    /// An empty schedule without warning detection can silently cause:
    /// - PV = 0 due to missing cashflows
    /// - Incorrect accruals from missing periods
    /// - Silent pricing failures
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::dates::{ScheduleBuilder, Tenor, ScheduleWarning};
    /// use time::{Date, Month};
    ///
    /// let start = Date::from_calendar_date(2025, Month::December, 31).expect("Valid date");
    /// let end = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date"); // Invalid: end before start
    ///
    /// // Without graceful mode: returns error
    /// let result = ScheduleBuilder::new(start, end)
    ///     .frequency(Tenor::monthly())
    ///     .build();
    /// assert!(result.is_err());
    ///
    /// // With graceful mode: returns empty schedule WITH warning
    /// let schedule = ScheduleBuilder::new(start, end)
    ///     .frequency(Tenor::monthly())
    ///     .graceful_fallback(true)
    ///     .build()
    ///     .expect("Schedule builder should succeed");
    /// assert_eq!(schedule.dates.len(), 0);
    ///
    /// // CRITICAL: Always check for warnings
    /// assert!(schedule.has_warnings(), "Should have warning about suppressed error");
    /// assert!(schedule.used_graceful_fallback());
    ///
    /// // Inspect the original error
    /// for warning in &schedule.warnings {
    ///     println!("Schedule generation warning: {warning}");
    /// }
    /// ```
    #[must_use]
    pub fn graceful_fallback(mut self, enabled: bool) -> Self {
        self.graceful = enabled;
        self
    }

    /// Allow missing calendar IDs without error.
    ///
    /// By default, [`adjust_with_id`](Self::adjust_with_id) returns an error at build time
    /// if the calendar ID is not found. This method enables lenient behavior where unknown
    /// calendars are silently ignored and the schedule is generated without adjustment.
    ///
    /// # Warning
    ///
    /// Enabling this option is **dangerous** for production use. A wrong holiday calendar
    /// is a first-order pricing error for accrual periods and payment dates. Only enable
    /// this for testing or when you explicitly want to tolerate missing calendars.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::dates::{ScheduleBuilder, Tenor, BusinessDayConvention};
    /// use time::{Date, Month};
    ///
    /// let start = Date::from_calendar_date(2025, Month::January, 15).expect("Valid date");
    /// let end = Date::from_calendar_date(2025, Month::December, 15).expect("Valid date");
    ///
    /// // Without allow_missing_calendar: error on unknown calendar
    /// let result = ScheduleBuilder::new(start, end)
    ///     .frequency(Tenor::monthly())
    ///     .adjust_with_id(BusinessDayConvention::Following, "unknown_calendar")
    ///     .build();
    /// assert!(result.is_err());
    ///
    /// // With allow_missing_calendar: proceeds without adjustment and records a warning
    /// let schedule = ScheduleBuilder::new(start, end)
    ///     .frequency(Tenor::monthly())
    ///     .allow_missing_calendar(true)
    ///     .adjust_with_id(BusinessDayConvention::Following, "unknown_calendar")
    ///     .build()
    ///     .expect("Schedule builder should succeed");
    /// assert!(schedule.dates.len() > 0);
    /// ```
    #[must_use]
    pub fn allow_missing_calendar(mut self, enabled: bool) -> Self {
        self.allow_missing_calendar = enabled;
        self
    }

    /// Configure business-day adjustment using calendar ID string lookup.
    ///
    /// This is a convenience method that combines calendar lookup with adjustment
    /// configuration. The calendar lookup is performed at build time.
    ///
    /// # Errors
    ///
    /// By default, returns an error at [`build()`](Self::build) time if the calendar ID
    /// is not found. Use [`allow_missing_calendar(true)`](Self::allow_missing_calendar)
    /// to opt-in to lenient behavior where unknown calendars are silently ignored.
    ///
    /// # Arguments
    ///
    /// * `conv` - Business day convention (Following, Modified Following, etc.)
    /// * `calendar_id` - Calendar identifier string (e.g., "nyse", "target2", "gblo")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::dates::{ScheduleBuilder, Tenor, BusinessDayConvention};
    /// use time::{Date, Month};
    ///
    /// let start = Date::from_calendar_date(2025, Month::January, 15).expect("Valid date");
    /// let end = Date::from_calendar_date(2025, Month::December, 15).expect("Valid date");
    ///
    /// let schedule = ScheduleBuilder::new(start, end)
    ///     .frequency(Tenor::monthly())
    ///     .adjust_with_id(BusinessDayConvention::Following, "nyse")
    ///     .build()
    ///     .expect("Schedule builder should succeed");
    /// # assert!(schedule.dates.len() > 0);
    /// ```
    #[must_use]
    pub fn adjust_with_id(mut self, conv: BusinessDayConvention, calendar_id: &str) -> Self {
        self.conv = Some(conv);
        self.pending_calendar_id = Some(calendar_id.to_string());
        self
    }

    /// Build a concrete schedule (adjusted if configured).
    ///
    /// When graceful fallback mode is enabled via [`graceful_fallback(true)`](Self::graceful_fallback),
    /// this method returns an empty schedule with a [`ScheduleWarning::GracefulFallback`]
    /// warning instead of propagating errors. Always check [`Schedule::has_warnings()`]
    /// when using graceful mode to detect potential pricing issues.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Start date is after end date (and graceful mode is disabled)
    /// - Calendar lookup fails (and neither graceful nor `allow_missing_calendar` is enabled)
    pub fn build(self) -> crate::Result<Schedule> {
        let graceful = self.graceful;
        let result = self.build_impl();

        match result {
            Ok(schedule) => Ok(schedule),
            Err(e) if graceful => {
                // Capture the error as a warning instead of propagating
                Ok(Schedule {
                    dates: Vec::new(),
                    warnings: vec![ScheduleWarning::GracefulFallback {
                        error_message: e.to_string(),
                    }],
                })
            }
            Err(e) => Err(e),
        }
    }

    /// Internal implementation of schedule building.
    fn build_impl(self) -> crate::Result<Schedule> {
        use super::calendar::calendar_by_id;

        if self.start > self.end {
            return Err(crate::error::InputError::InvalidDateRange.into());
        }

        let mut warnings: Vec<ScheduleWarning> = Vec::new();

        // Resolve pending calendar ID if present, otherwise use directly provided calendar
        let resolved_cal: Option<&dyn HolidayCalendar> =
            if let Some(ref calendar_id) = self.pending_calendar_id {
                match calendar_by_id(calendar_id) {
                    Some(cal) => Some(cal),
                    None => {
                        if self.allow_missing_calendar {
                            // Silently skip adjustment
                            warnings.push(ScheduleWarning::MissingCalendarId {
                                calendar_id: calendar_id.clone(),
                            });
                            None
                        } else {
                            // Strict mode: error on missing calendar
                            return Err(crate::error::Error::calendar_not_found_with_suggestions(
                                calendar_id.clone(),
                                super::available_calendars(),
                            ));
                        }
                    }
                }
            } else {
                self.cal
            };

        // Generate dates based on mode
        let mut dates = if self.imm_mode {
            // Standard IMM: generate dates using next_imm to get proper third Wednesdays
            generate_imm_dates(self.start, self.end)
        } else if self.cds_imm_mode {
            // CDS IMM: 20th of quarterly months
            let adj_start = if is_cds_roll_date(self.start) {
                self.start
            } else {
                next_cds_date(self.start)
            };

            let builder = BuilderInternal {
                start: adj_start,
                end: self.end,
                freq: self.freq,
                stub: self.stub,
                eom: self.eom,
            };
            builder.generate()?
        } else {
            let builder = BuilderInternal {
                start: self.start,
                end: self.end,
                freq: self.freq,
                stub: self.stub,
                eom: self.eom,
            };
            builder.generate()?
        };

        // Enforce monotonicity and remove duplicates produced by EOM/stub handling
        enforce_monotonic_and_dedup(&mut dates);

        // Apply business day adjustment if configured
        if let (Some(conv), Some(cal)) = (self.conv, resolved_cal) {
            for d in &mut dates {
                *d = adjust(*d, conv, cal)?;
            }

            // Adjustment can create duplicates (e.g., both anchors adjust to same business day)
            // and, in edge cases, non-monotonicities. Enforce again.
            enforce_monotonic_and_dedup(&mut dates);
        }

        Ok(Schedule { dates, warnings })
    }
}

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
    pub frequency: Tenor,
    /// Stub convention (short/long front/back).
    pub stub: StubKind,
    /// Business day convention for adjusting dates.
    pub business_day_convention: Option<BusinessDayConvention>,
    /// Optional calendar identifier for holiday adjustments.
    pub calendar_id: Option<String>,
    /// If true, always roll to end of month when applicable.
    pub end_of_month: bool,
    /// If true, use standard IMM date logic (third Wednesday of quarterly months).
    #[serde(default)]
    pub imm_mode: bool,
    /// If true, use CDS IMM date logic (20th of quarterly months).
    pub cds_imm_mode: bool,
    /// If true, allow graceful handling of edge cases.
    pub graceful: bool,
    /// If true, silently ignore missing calendar IDs instead of returning an error.
    /// Default: false (strict mode - errors on unknown calendar IDs).
    #[serde(default)]
    pub allow_missing_calendar: bool,
}

impl ScheduleSpec {
    /// Reconstruct a [`Schedule`] using the persisted configuration.
    pub fn build(&self) -> crate::Result<Schedule> {
        let mut builder = ScheduleBuilder::new(self.start, self.end)
            .frequency(self.frequency)
            .stub_rule(self.stub)
            .end_of_month(self.end_of_month)
            .graceful_fallback(self.graceful)
            .allow_missing_calendar(self.allow_missing_calendar);

        if let (Some(conv), Some(id)) = (self.business_day_convention, self.calendar_id.as_deref())
        {
            builder = builder.adjust_with_id(conv, id);
        }

        if self.imm_mode {
            builder = builder.imm();
        } else if self.cds_imm_mode {
            builder = builder.cds_imm();
        }

        builder.build()
    }
}

// Internal generator for schedule construction
#[derive(Clone, Copy)]
struct BuilderInternal {
    start: Date,
    end: Date,
    freq: Tenor,
    stub: StubKind,
    eom: bool,
}

impl BuilderInternal {
    fn generate(self) -> crate::Result<Vec<Date>> {
        match self.stub {
            StubKind::ShortFront => self.gen_short_front(),
            StubKind::LongFront => self.gen_long_front(),
            StubKind::LongBack => self.gen_long_back(),
            StubKind::None => self.gen_regular(),
            StubKind::ShortBack => self.gen_short_back(),
        }
    }

    fn add_tenor(self, date: Date, count: i32) -> crate::Result<Date> {
        let tenor = self.freq;
        // Tenor doesn't support negative count directly in its struct, but we can handle it here
        // or use `add_to_date` with a multiplier if we want to add multiple periods.
        // For simple single-step additions scan:
        if count == 1 {
            tenor.add_to_date(date, None, BusinessDayConvention::Unadjusted)
        } else if count == -1 {
            // Need to reverse the tenor operation
            Ok(match tenor.unit {
                crate::dates::TenorUnit::Months => date.add_months(-(tenor.count as i32)),
                crate::dates::TenorUnit::Years => date.add_months(-(tenor.count as i32) * 12),
                crate::dates::TenorUnit::Weeks => date - Duration::weeks(tenor.count as i64),
                crate::dates::TenorUnit::Days => date - Duration::days(tenor.count as i64),
            })
        } else {
            // Should not happen in current logic but safe fallback
            Ok(date)
        }
    }

    fn gen_regular(self) -> crate::Result<Vec<Date>> {
        let mut buf: Buffer = Buffer::new();
        let (mut dt, end) = (
            maybe_eom(self.eom, self.start),
            maybe_eom(self.eom, self.end),
        );
        buf.push(dt);
        while dt < end {
            let mut next = self.add_tenor(dt, 1)?;
            if next > end {
                next = end;
            }
            dt = maybe_eom(self.eom, next);
            push_if_new(&mut buf, dt);
        }
        Ok(buf.into_vec())
    }

    fn gen_short_back(self) -> crate::Result<Vec<Date>> {
        // Short back stub is naturally produced by forward generation that truncates the final step.
        self.gen_regular()
    }

    fn gen_short_front(self) -> crate::Result<Vec<Date>> {
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
            let prev = self.add_tenor(dt, -1)?;
            dt = if prev < target { target } else { prev };
        }
        buf.as_mut_slice().reverse();
        Ok(buf.into_vec())
    }

    fn gen_long_front(self) -> crate::Result<Vec<Date>> {
        let mut buf: Buffer = Buffer::new();
        let mut anchors = Vec::new();
        let mut dt = self.end;
        anchors.push(dt);
        while dt > self.start {
            let prev = self.add_tenor(dt, -1)?;
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
        Ok(buf.into_vec())
    }

    fn gen_long_back(self) -> crate::Result<Vec<Date>> {
        let mut buf: Buffer = Buffer::new();
        let mut dt = self.start;
        buf.push(maybe_eom(self.eom, dt));
        while dt < self.end {
            let next = self.add_tenor(dt, 1)?;
            let next_after = self.add_tenor(next, 1)?;
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
        Ok(buf.into_vec())
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
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use time::Month;

    fn d(y: i32, m: u8, day: u8) -> Date {
        Date::from_calendar_date(y, Month::try_from(m).expect("Valid month (1-12)"), day)
            .expect("Valid test date")
    }

    #[test]
    fn test_graceful_fallback_returns_empty_with_warning_on_invalid_range() {
        // Invalid: end before start
        let start = d(2025, 12, 31);
        let end = d(2025, 1, 1);

        // Without graceful mode: should error
        let result = ScheduleBuilder::new(start, end)
            .frequency(Tenor::monthly())
            .build();
        assert!(result.is_err());

        // With graceful mode: should return empty schedule WITH warning
        let schedule = ScheduleBuilder::new(start, end)
            .frequency(Tenor::monthly())
            .graceful_fallback(true)
            .build()
            .expect("Schedule builder should succeed with graceful_fallback");
        assert_eq!(schedule.dates.len(), 0);

        // CRITICAL: Verify warning is present so callers know something went wrong
        assert!(
            schedule.has_warnings(),
            "Graceful fallback should set warning flag"
        );
        assert!(
            schedule.used_graceful_fallback(),
            "Should indicate graceful fallback was used"
        );
        assert_eq!(schedule.warnings.len(), 1);

        // Warning should contain the original error message
        let warning = &schedule.warnings[0];
        match warning {
            ScheduleWarning::GracefulFallback { error_message } => {
                assert!(
                    error_message.contains("date") || error_message.contains("range"),
                    "Warning should describe the invalid date range: {error_message}"
                );
            }
            ScheduleWarning::MissingCalendarId { .. } => {
                panic!("Expected GracefulFallback warning, got MissingCalendarId")
            }
        }
    }

    #[test]
    fn test_graceful_fallback_warning_is_displayable() {
        let warning = ScheduleWarning::GracefulFallback {
            error_message: "Invalid date range".to_string(),
        };
        let display = format!("{warning}");
        assert!(display.contains("graceful fallback"));
        assert!(display.contains("Invalid date range"));
    }

    #[test]
    fn test_valid_schedule_has_no_warnings() {
        let start = d(2025, 1, 15);
        let end = d(2025, 4, 15);

        let schedule = ScheduleBuilder::new(start, end)
            .frequency(Tenor::monthly())
            .build()
            .expect("Valid schedule should succeed");

        assert!(
            !schedule.has_warnings(),
            "Valid schedule should have no warnings"
        );
        assert!(
            !schedule.used_graceful_fallback(),
            "Valid schedule should not indicate fallback"
        );
        assert!(schedule.warnings.is_empty());
    }

    #[test]
    fn test_adjust_with_id_valid_calendar() {
        let start = d(2025, 1, 15);
        let end = d(2025, 3, 15);

        // Use a known calendar (target2)
        let schedule = ScheduleBuilder::new(start, end)
            .frequency(Tenor::monthly())
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

        // Invalid calendar with strict mode (default) should error
        let result = ScheduleBuilder::new(start, end)
            .frequency(Tenor::monthly())
            .adjust_with_id(BusinessDayConvention::Following, "INVALID_CALENDAR")
            .build();

        // Should fail with CalendarNotFound error
        assert!(result.is_err());
        let err_msg = format!("{}", result.expect_err("Expected CalendarNotFound error"));
        assert!(err_msg.contains("Calendar not found"));
        assert!(err_msg.contains("INVALID_CALENDAR"));
    }

    #[test]
    fn test_adjust_with_id_invalid_calendar_with_suggestions() {
        let start = d(2025, 1, 15);
        let end = d(2025, 3, 15);

        // Calendar ID with typo should suggest similar calendars
        let result = ScheduleBuilder::new(start, end)
            .frequency(Tenor::monthly())
            .adjust_with_id(BusinessDayConvention::Following, "targt2") // typo in target2
            .build();

        // Should fail with CalendarNotFound error and include suggestion
        assert!(result.is_err());
        let err_msg = format!("{}", result.expect_err("Expected CalendarNotFound error"));
        assert!(err_msg.contains("Calendar not found"));
        assert!(err_msg.contains("Did you mean"));
        assert!(err_msg.contains("target2"));
    }

    #[test]
    fn test_adjust_with_id_invalid_calendar_allow_missing() {
        let start = d(2025, 1, 15);
        let end = d(2025, 3, 15);

        // Invalid calendar with allow_missing_calendar enabled
        // Should succeed and return schedule without adjustment
        let schedule = ScheduleBuilder::new(start, end)
            .frequency(Tenor::monthly())
            .allow_missing_calendar(true)
            .adjust_with_id(BusinessDayConvention::Following, "INVALID_CALENDAR")
            .build()
            .expect("Schedule builder should succeed with allow_missing_calendar");

        // Should generate schedule (unadjusted)
        assert!(schedule.dates.len() >= 2);

        // allow_missing_calendar does NOT use graceful_fallback, but it does record a warning
        // so callers can detect that adjustment was skipped.
        assert!(
            schedule.has_warnings(),
            "should record MissingCalendarId warning"
        );
        assert!(!schedule.used_graceful_fallback());
        assert!(matches!(
            schedule.warnings.as_slice(),
            [ScheduleWarning::MissingCalendarId { calendar_id }] if calendar_id == "INVALID_CALENDAR"
        ));
    }

    #[test]
    fn test_adjust_with_id_invalid_calendar_graceful_mode() {
        let start = d(2025, 1, 15);
        let end = d(2025, 3, 15);

        // Invalid calendar with graceful mode (returns empty schedule with warning)
        let schedule = ScheduleBuilder::new(start, end)
            .frequency(Tenor::monthly())
            .adjust_with_id(BusinessDayConvention::Following, "INVALID_CALENDAR")
            .graceful_fallback(true)
            .build()
            .expect("Schedule builder should succeed with graceful_fallback");

        // Should return empty schedule due to graceful fallback on error
        assert_eq!(schedule.dates.len(), 0);

        // CRITICAL: Warning must be present so callers know the calendar lookup failed
        assert!(
            schedule.has_warnings(),
            "Invalid calendar with graceful fallback should produce warning"
        );
        assert!(schedule.used_graceful_fallback());

        // Warning should mention the calendar error
        match &schedule.warnings[0] {
            ScheduleWarning::GracefulFallback { error_message } => {
                assert!(
                    error_message.contains("Calendar not found")
                        || error_message.contains("INVALID_CALENDAR"),
                    "Warning should describe calendar error: {error_message}"
                );
            }
            ScheduleWarning::MissingCalendarId { .. } => {
                panic!("Expected GracefulFallback warning, got MissingCalendarId")
            }
        }
    }

    #[test]
    fn test_graceful_mode_with_valid_inputs_has_no_warnings() {
        let start = d(2025, 1, 15);
        let end = d(2025, 4, 15);

        // Valid inputs with graceful mode should work normally without warnings
        let schedule = ScheduleBuilder::new(start, end)
            .frequency(Tenor::monthly())
            .graceful_fallback(true)
            .build()
            .expect("Schedule builder should succeed with valid test data");

        assert_eq!(schedule.dates.len(), 4);
        assert_eq!(schedule.dates[0], start);
        assert_eq!(schedule.dates[3], end);

        // Valid inputs should NOT produce warnings even with graceful mode enabled
        assert!(
            !schedule.has_warnings(),
            "Valid schedule with graceful mode should have no warnings"
        );
        assert!(!schedule.used_graceful_fallback());
    }

    #[test]
    fn test_adjust_with_id_combined_with_other_options() {
        let start = d(2025, 1, 31);
        let end = d(2025, 4, 30);

        // Combine adjust_with_id with end_of_month
        let schedule = ScheduleBuilder::new(start, end)
            .frequency(Tenor::monthly())
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
            .frequency(Tenor::monthly())
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
            .frequency(Tenor::monthly())
            .stub_rule(StubKind::LongBack)
            .build()
            .expect("Schedule builder should succeed with LongBack");

        assert_eq!(
            schedule.dates,
            vec![
                d(2025, 1, 15),
                d(2025, 2, 15),
                d(2025, 3, 15),
                d(2025, 4, 15),
                end
            ]
        );
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod serde_tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_tenor_serde_roundtrip() {
        use serde_json;

        // Test different Tenor variants
        let tenors = vec![
            Tenor::annual(),
            Tenor::semi_annual(),
            Tenor::quarterly(),
            Tenor::monthly(),
            Tenor::biweekly(),
            Tenor::weekly(),
            Tenor::daily(),
        ];

        for tenor in tenors {
            let json =
                serde_json::to_string(&tenor).expect("JSON serialization should succeed in test");
            let deserialized: Tenor =
                serde_json::from_str(&json).expect("JSON deserialization should succeed in test");
            // Tenor doesn't strictly derive PartialEq but it should. If not, compare fields.
            // Wait, Tenor doesn't derive PartialEq? Let's check or assume it does or use custom assertion.
            // Just checked tenor.rs, it might not derive PartialEq.
            // Assuming it does for now, or I'll fix it if it errors.
            // Actually I should verify if Tenor derives PartialEq. It typically does.
            assert_eq!(tenor.count, deserialized.count);
            assert_eq!(tenor.unit, deserialized.unit);
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
            .frequency(Tenor::monthly())
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
