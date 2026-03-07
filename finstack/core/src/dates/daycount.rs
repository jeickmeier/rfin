//! Day-count convention algorithms for fixed income and derivative accrual calculations.
//!
//! This module implements industry-standard day count conventions as defined by
//! ISDA (International Swaps and Derivatives Association) and ICMA (International
//! Capital Market Association). All implementations are panic-free and avoid heap
//! allocation.
//!
//! # Date Interval Convention
//!
//! **All day-count calculations use start-inclusive, end-exclusive intervals `[start, end)`.**
//!
//! This means:
//! - The start date **is** counted in the accrual period
//! - The end date **is not** counted in the accrual period
//! - A period from Jan 1 to Jan 2 contains 1 day (Jan 1 only)
//! - A period from Jan 1 to Jan 1 contains 0 days
//!
//! This convention is consistent with how payment dates work in financial instruments:
//! the accrual period ends the day before the payment date, and you don't accrue
//! interest on the payment date itself.
//!
//! # Industry Standards
//!
//! Day count conventions define how interest accrues between two dates. Different
//! markets and instruments use different conventions:
//!
//! ## ISDA Standard Conventions
//!
//! - **Actual/360** (Act/360): Money market standard for USD, EUR short-term rates
//! - **Actual/365 Fixed** (Act/365F): GBP money markets and some bond markets
//! - **30/360** (30U/360): US corporate and municipal bonds
//! - **30E/360** (30E/360): Eurobonds and international bonds
//! - **Actual/Actual (ISDA)**: US Treasury bonds, many swap contracts
//!
//! ## ICMA/ISMA Standard Conventions
//!
//! - **Actual/Actual (ICMA)**: International bonds with regular coupon schedules
//!
//! # Supported Conventions
//!
//! - [`DayCount::Act360`] - Actual/360
//! - [`DayCount::Act365F`] - Actual/365 Fixed
//! - [`DayCount::Act365L`] - Actual/365 Leap (AFB)
//! - [`DayCount::Thirty360`] - 30/360 US (Bond Basis)
//! - [`DayCount::ThirtyE360`] - 30E/360 (Eurobond Basis)
//! - [`DayCount::ActAct`] - Actual/Actual (ISDA)
//! - [`DayCount::ActActIsma`] - Actual/Actual (ICMA) regular-period helper
//! - [`DayCount::Bus252`] - Business/252 (Brazilian and some equity markets)
//!
//! # Examples
//! ```
//! use finstack_core::dates::{Date, DayCount, DayCountCtx};
//! use time::Month;
//!
//! let start = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
//! let end   = Date::from_calendar_date(2026, Month::January, 1).expect("Valid date");
//!
//! let yf = DayCount::ActAct
//!     .year_fraction(start, end, DayCountCtx::default())
//!     .expect("Year fraction calculation should succeed");
//! assert!((yf - 1.0).abs() < 1e-9);
//! ```
//!
//! # Bus/252 Convention
//!
//! The Bus/252 convention counts business days between dates and divides by 252 (typical trading days per year).
//! This requires a holiday calendar to determine business days. Provide the calendar via `DayCountCtx`.
//!
//! ```
//! use finstack_core::dates::{Date, DayCount, DayCountCtx};
//! use finstack_core::dates::calendar::TARGET2;
//! use time::Month;
//!
//! let start = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
//! let end   = Date::from_calendar_date(2025, Month::January, 31).expect("Valid date");
//! let calendar = TARGET2;
//!
//! // Calculate year fraction with a calendar in context
//! let yf = DayCount::Bus252
//!     .year_fraction(start, end, DayCountCtx { calendar: Some(&calendar), frequency: None, bus_basis: None })
//!     .expect("Year fraction calculation should succeed");
//! ```
//!
//! # ACT/ACT ISMA vs ISDA
//!
//! Both conventions use actual days in numerator and actual days in denominator, but differ in how
//! the denominator is calculated:
//!
//! - **ACT/ACT (ISDA)**: Uses the actual number of days in the year containing the period
//! - **ACT/ACT (ISMA)**: Uses the actual number of days in the coupon period containing the date
//!
//! ```
//! use finstack_core::dates::{Date, DayCount, Tenor, DayCountCtx};
//! use time::Month;
//!
//! // Example: 6-month period in a leap year
//! let start = Date::from_calendar_date(2024, Month::January, 1).expect("Valid date"); // Leap year
//! let end   = Date::from_calendar_date(2024, Month::July, 1).expect("Valid date");
//!
//! // ACT/ACT (ISDA): 181 days / 366 days (leap year) = 0.4945355191256831
//! let yf_isda = DayCount::ActAct.year_fraction(start, end, DayCountCtx::default()).expect("Year fraction calculation should succeed");
//!
//! // ACT/ACT (ISMA): frequency-only helper for regular coupon periods
//! // Returns year fractions: a full 6-month regular period = 0.5 years
//! let freq = Tenor::semi_annual(); // Semi-annual
//! let yf_isma = DayCount::ActActIsma
//!     .year_fraction(start, end, DayCountCtx { calendar: None, frequency: Some(freq), bus_basis: None })
//!     .expect("Year fraction calculation should succeed");
//! // yf_isma ≈ 0.5 (one full semi-annual period in years)
//! ```

#![allow(clippy::many_single_char_names)]

use crate::dates::date_extensions::DateExt;
#[cfg(test)]
use core::cmp::Ordering;
use smallvec::SmallVec;
use time::{Date, Duration, Month};

use crate::dates::date_extensions::BusinessDayIter;
use crate::dates::tenor::TenorUnit;
use crate::dates::{BusinessDayConvention, CalendarRegistry, HolidayCalendar, Tenor};
use crate::error::InputError;

/// Optional context for day-count year-fraction calculations.
///
/// Certain conventions require additional information:
/// - `Bus/252` requires a holiday `calendar`.
/// - `Act/Act (ISMA)` requires the coupon `frequency`.
#[derive(Clone, Copy, Default)]
pub struct DayCountCtx<'a> {
    /// Holiday calendar for business day conventions
    pub calendar: Option<&'a dyn HolidayCalendar>,
    /// Payment frequency (required for ACT/ACT ISMA)
    pub frequency: Option<Tenor>,
    /// Business day convention (required for Bus/252)
    pub bus_basis: Option<u16>,
}

impl<'a> std::fmt::Debug for DayCountCtx<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DayCountCtx")
            .field("calendar", &self.calendar.map(|_| "HolidayCalendar"))
            .field("frequency", &self.frequency)
            .field("bus_basis", &self.bus_basis)
            .finish()
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
/// Serializable snapshot of [`DayCountCtx`] state for persistence and interchange.
///
/// This struct captures the optional context parameters (calendar, frequency, business-day basis)
/// needed to reconstruct a [`DayCountCtx`] at runtime using a [`CalendarRegistry`].
pub struct DayCountCtxState {
    /// Optional calendar code (e.g. "target2").
    pub calendar_id: Option<String>,
    /// Optional coupon frequency for Act/Act ISMA.
    pub frequency: Option<Tenor>,
    /// Optional custom business-day divisor (defaults to 252 when `None`).
    pub bus_basis: Option<u16>,
}

impl DayCountCtxState {
    /// Build a runtime [`DayCountCtx`] using the provided calendar registry.
    pub fn to_ctx<'a>(&self, registry: &'a CalendarRegistry<'a>) -> DayCountCtx<'a> {
        let calendar = self
            .calendar_id
            .as_deref()
            .and_then(|code| registry.resolve_str(code));
        DayCountCtx {
            calendar,
            frequency: self.frequency,
            bus_basis: self.bus_basis,
        }
    }
}

impl<'a> From<DayCountCtx<'a>> for DayCountCtxState {
    fn from(value: DayCountCtx<'a>) -> Self {
        let calendar_id = value
            .calendar
            .and_then(|cal| cal.metadata().map(|meta| meta.id.to_string()));
        Self {
            calendar_id,
            frequency: value.frequency,
            bus_basis: value.bus_basis,
        }
    }
}

/// Supported day-count conventions with industry-standard definitions.
///
/// Each variant implements a specific day count convention as defined by
/// ISDA, ICMA, or local market conventions. The conventions determine how
/// interest accrues between payment dates.
///
/// # Standards References
///
/// Implementations follow:
/// - **ISDA**: 2006 ISDA Definitions, Section 4.16
/// - **ICMA**: ICMA Rule Book, Rule 251
/// - **ISO**: ISO 20022 Day Count Fraction Codes
///
/// # Examples
///
/// ```rust
/// use finstack_core::dates::{Date, DayCount, DayCountCtx};
/// use time::Month;
///
/// let start = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
/// let end = Date::from_calendar_date(2025, Month::July, 1).expect("Valid date");
///
/// // Actual/360 - money market convention
/// let yf_360 = DayCount::Act360.year_fraction(start, end, DayCountCtx::default()).expect("Year fraction calculation should succeed");
///
/// // 30/360 - bond convention
/// let yf_30360 = DayCount::Thirty360.year_fraction(start, end, DayCountCtx::default()).expect("Year fraction calculation should succeed");
///
/// assert!(yf_360 > yf_30360); // Act/360 has larger denominator
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub enum DayCount {
    /// Actual/360 day count convention.
    ///
    /// Year fraction = (actual days between dates) / 360
    ///
    /// # Standards Reference
    ///
    /// - **ISDA**: 2006 ISDA Definitions, Section 4.16(d)
    /// - **ISO 20022**: Day Count Fraction Code "Actual/360" (A004)
    /// - **Also known as**: Act/360, A/360, French
    ///
    /// # Usage
    ///
    /// Standard for:
    /// - USD money market deposits
    /// - EUR money market instruments
    /// - Short-term rate derivatives (SOFR, €STR)
    /// - FX swaps and forwards
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::dates::{Date, DayCount, DayCountCtx};
    /// use time::Month;
    ///
    /// let start = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
    /// let end = Date::from_calendar_date(2025, Month::April, 1).expect("Valid date"); // 90 days
    ///
    /// let yf = DayCount::Act360.year_fraction(start, end, DayCountCtx::default()).expect("Year fraction calculation should succeed");
    /// assert_eq!(yf, 90.0 / 360.0);
    /// ```
    #[serde(alias = "act360")]
    Act360,

    /// Actual/365 Fixed day count convention.
    ///
    /// Year fraction = (actual days between dates) / 365
    ///
    /// # Standards Reference
    ///
    /// - **ISDA**: 2006 ISDA Definitions, Section 4.16(e)
    /// - **ISO 20022**: Day Count Fraction Code "Actual/365 Fixed" (A005)
    /// - **Also known as**: Act/365F, A/365F, English
    ///
    /// # Usage
    ///
    /// Standard for:
    /// - GBP money markets (SONIA)
    /// - Cable (GBP/USD) FX transactions
    /// - Some Commonwealth bond markets
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::dates::{Date, DayCount, DayCountCtx};
    /// use time::Month;
    ///
    /// let start = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
    /// let end = Date::from_calendar_date(2026, Month::January, 1).expect("Valid date");
    ///
    /// let yf = DayCount::Act365F.year_fraction(start, end, DayCountCtx::default()).expect("Year fraction calculation should succeed");
    /// assert!((yf - 1.0).abs() < 1e-9); // 365 days / 365 = 1.0
    /// ```
    #[serde(alias = "act_365f", alias = "act365f", alias = "act_365_fixed")]
    Act365F,

    /// Actual/365 Leap day count convention (Actual/365L or AFB).
    ///
    /// Year fraction = (actual days) / (366 if Feb 29 in period else 365)
    ///
    /// # Standards Reference
    ///
    /// - **AFB**: Association Française des Banques (French Bankers Association)
    /// - **ISO 20022**: Day Count Fraction Code "Actual/365L" (A008)
    /// - **Also known as**: Act/365L, AFB, ISMA-Year
    ///
    /// # Usage
    ///
    /// Used in:
    /// - French government bonds (OATs)
    /// - Some European bond markets
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::dates::{Date, DayCount, DayCountCtx};
    /// use time::Month;
    ///
    /// // Period containing Feb 29, 2024 (leap year)
    /// let start = Date::from_calendar_date(2024, Month::February, 1).expect("Valid date");
    /// let end = Date::from_calendar_date(2024, Month::March, 1).expect("Valid date");
    ///
    /// let yf = DayCount::Act365L.year_fraction(start, end, DayCountCtx::default()).expect("Year fraction calculation should succeed");
    /// // 29 days / 366 (leap year denominator)
    /// assert_eq!(yf, 29.0 / 366.0);
    /// ```
    #[serde(alias = "act365l", alias = "act_365l")]
    Act365L,

    /// 30/360 US (Bond Basis) day count convention.
    ///
    /// Assumes 30 days per month and 360 days per year with US market adjustments.
    ///
    /// # Standards Reference
    ///
    /// - **ISDA**: 2006 ISDA Definitions, Section 4.16(f) - "30/360"
    /// - **ISO 20022**: Day Count Fraction Code "30/360" (A001)
    /// - **Also known as**: 30U/360, 30/360 US, Bond Basis
    ///
    /// # Formula
    ///
    /// ```text
    /// Days = 360(Y₂ - Y₁) + 30(M₂ - M₁) + (D₂' - D₁')
    ///
    /// where:
    ///   D₁' = min(D₁, 30)
    ///   D₂' = min(D₂, 30) if D₁' = 30, else D₂
    /// ```
    ///
    /// # Usage
    ///
    /// Standard for:
    /// - US corporate bonds
    /// - US municipal bonds
    /// - US agency debt
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::dates::{Date, DayCount, DayCountCtx};
    /// use time::Month;
    ///
    /// let start = Date::from_calendar_date(2025, Month::January, 31).expect("Valid date");
    /// let end = Date::from_calendar_date(2025, Month::February, 28).expect("Valid date");
    ///
    /// let yf = DayCount::Thirty360.year_fraction(start, end, DayCountCtx::default()).expect("Year fraction calculation should succeed");
    /// // Treats Jan 31 as day 30, Feb 28 as day 28: 28 days / 360
    /// assert_eq!(yf, 28.0 / 360.0);
    /// ```
    #[serde(alias = "thirty360")]
    Thirty360,

    /// 30E/360 (Eurobond Basis) day count convention.
    ///
    /// Assumes 30 days per month and 360 days per year with European adjustments.
    ///
    /// # Standards Reference
    ///
    /// - **ISDA**: 2006 ISDA Definitions, Section 4.16(g) - "30E/360"
    /// - **ISO 20022**: Day Count Fraction Code "30E/360" (A002)
    /// - **Also known as**: 30/360 ISDA, 30/360 European, Eurobond Basis
    ///
    /// # Formula
    ///
    /// ```text
    /// Days = 360(Y₂ - Y₁) + 30(M₂ - M₁) + (D₂' - D₁')
    ///
    /// where:
    ///   D₁' = min(D₁, 30)
    ///   D₂' = min(D₂, 30)
    /// ```
    ///
    /// # Usage
    ///
    /// Standard for:
    /// - Eurobonds
    /// - International bonds
    /// - Some interest rate swaps
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::dates::{Date, DayCount, DayCountCtx};
    /// use time::Month;
    ///
    /// let start = Date::from_calendar_date(2025, Month::January, 31).expect("Valid date");
    /// let end = Date::from_calendar_date(2025, Month::March, 31).expect("Valid date");
    ///
    /// let yf = DayCount::ThirtyE360.year_fraction(start, end, DayCountCtx::default()).expect("Year fraction calculation should succeed");
    /// // Treats both 31st as day 30: 60 days / 360
    /// assert_eq!(yf, 60.0 / 360.0);
    /// ```
    #[serde(alias = "thirty_e360")]
    ThirtyE360,

    /// Actual/Actual (ISDA) day count convention.
    ///
    /// Uses actual days in numerator and actual days in the containing year(s)
    /// as denominator, splitting across year boundaries.
    ///
    /// # Standards Reference
    ///
    /// - **ISDA**: 2006 ISDA Definitions, Section 4.16(b) - "Actual/Actual (ISDA)"
    /// - **ISO 20022**: Day Count Fraction Code "Actual/Actual ISDA" (A006)
    /// - **Also known as**: Act/Act (ISDA), Actual/Actual, Act/Act
    ///
    /// # Algorithm
    ///
    /// For a period spanning multiple calendar years:
    /// 1. Split period at year boundaries
    /// 2. For each year segment: (days in segment) / (days in that year)
    /// 3. Sum the year fractions
    ///
    /// # Usage
    ///
    /// Standard for:
    /// - US Treasury bonds
    /// - Interest rate swaps (USD, EUR fixed legs)
    /// - Government bonds in many markets
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::dates::{Date, DayCount, DayCountCtx};
    /// use time::Month;
    ///
    /// // Period spanning year boundary (leap year 2024)
    /// let start = Date::from_calendar_date(2024, Month::July, 1).expect("Valid date");
    /// let end = Date::from_calendar_date(2025, Month::July, 1).expect("Valid date");
    ///
    /// let yf = DayCount::ActAct.year_fraction(start, end, DayCountCtx::default()).expect("Year fraction calculation should succeed");
    /// // 184/366 (Jul-Dec 2024 in leap year) + 365/365 (all of 2025)
    /// assert!((yf - 1.0).abs() < 0.01);
    /// ```
    ///
    /// # References
    ///
    /// - ISDA (2006). "2006 ISDA Definitions." Section 4.16(b).
    #[serde(alias = "act_act")]
    ActAct,

    /// Actual/Actual (ICMA) day count convention.
    ///
    /// Uses actual days in numerator and actual days in the coupon period
    /// as denominator, requiring knowledge of payment frequency.
    ///
    /// # Standards Reference
    ///
    /// - **ICMA**: ICMA Rule Book, Rule 251 - "Actual/Actual (ICMA)"
    /// - **ISO 20022**: Day Count Fraction Code "Actual/Actual ICMA" (A007)
    /// - **Also known as**: Act/Act (ICMA), Act/Act (ISMA), ISMA-99
    ///
    /// # Algorithm
    ///
    /// 1. Determine quasi-coupon periods based on payment frequency
    /// 2. For each period: (actual days) / (actual days in coupon period)
    /// 3. Sum fractions across periods
    ///
    /// # Usage
    ///
    /// Standard for:
    /// - International bonds with regular coupons
    /// - Eurobonds with semi-annual or annual payments
    /// - ICMA-governed securities
    ///
    /// # Requirements
    ///
    /// Requires `frequency` in [`DayCountCtx`] to determine regular coupon periods.
    /// For irregular first/last coupons, use
    /// [`act_act_isma_year_fraction_with_reference_period`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::dates::{Date, DayCount, DayCountCtx, Tenor};
    /// use time::Month;
    ///
    /// let start = Date::from_calendar_date(2025, Month::January, 15).expect("Valid date");
    /// let end = Date::from_calendar_date(2025, Month::July, 15).expect("Valid date");
    /// let freq = Tenor::semi_annual(); // Semi-annual
    ///
    /// let yf = DayCount::ActActIsma.year_fraction(
    ///     start,
    ///     end,
    ///     DayCountCtx { frequency: Some(freq), ..Default::default() }
    /// ).expect("Year fraction calculation should succeed");
    ///
    /// // Full semi-annual period = 0.5 year fraction (6 months / 12 months)
    /// assert!((yf - 0.5).abs() < 1e-6);
    /// ```
    ///
    /// # References
    ///
    /// - ICMA (2010). "ICMA Rule Book." Rule 251.
    /// - ISMA (1999). "Recommendations for Accrued Interest Calculations."
    #[serde(alias = "act_act_isma")]
    ActActIsma,

    /// Business/252 day count convention.
    ///
    /// Year fraction = (business days between dates) / 252
    ///
    /// # Market Convention
    ///
    /// - **Brazil**: Standard for BRL-denominated instruments (ANBIMA)
    /// - **Also used**: Some equity derivatives and variance swaps
    /// - **Basis**: 252 represents typical trading days per year
    ///
    /// # Requirements
    ///
    /// Requires `calendar` in [`DayCountCtx`] to determine business days.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::dates::{Date, DayCount, DayCountCtx};
    /// use finstack_core::dates::calendar::NYSE;
    /// use time::Month;
    ///
    /// let start = Date::from_calendar_date(2025, Month::January, 6).expect("Valid date"); // Monday
    /// let end = Date::from_calendar_date(2025, Month::January, 13).expect("Valid date"); // Next Monday
    ///
    /// let yf = DayCount::Bus252.year_fraction(
    ///     start,
    ///     end,
    ///     DayCountCtx { calendar: Some(&NYSE), ..Default::default() }
    /// ).expect("Year fraction calculation should succeed");
    ///
    /// // 5 business days / 252
    /// assert!((yf * 252.0 - 5.0).abs() < 0.1);
    /// ```
    #[serde(alias = "bus252")]
    Bus252,
}

impl DayCount {
    /// Return the day count between `start` (inclusive) and `end` (exclusive).
    ///
    /// The output follows the specific convention rules and is **always ≥ 0**.
    ///
    /// # Note
    /// For `Bus/252`, this returns an error (requires calendar context via [`DayCountCtx`]).
    #[cfg(test)]
    #[doc(hidden)]
    pub(crate) fn days(self, start: Date, end: Date) -> crate::Result<i32> {
        match start.cmp(&end) {
            Ordering::Greater => Err(InputError::InvalidDateRange.into()),
            Ordering::Equal => Ok(0),
            Ordering::Less => match self {
                DayCount::Act360
                | DayCount::Act365F
                | DayCount::Act365L
                | DayCount::ActAct
                | DayCount::ActActIsma => {
                    let total_days = (end - start).whole_days();
                    Ok(total_days as i32)
                }
                DayCount::Thirty360 => Ok(days_30_360(start, end, Thirty360Convention::Us)),
                DayCount::ThirtyE360 => Ok(days_30_360(start, end, Thirty360Convention::European)),
                DayCount::Bus252 => Err(InputError::Invalid.into()),
            },
        }
    }

    /// Compute the year fraction between `start` and `end` per this convention.
    ///
    /// Provide any required context via [`DayCountCtx`]:
    /// - `Bus/252` requires a holiday calendar
    /// - `Act/Act (ISMA)` requires a coupon frequency
    ///
    /// # Arguments
    ///
    /// * `start` - Start date (inclusive)
    /// * `end` - End date (exclusive)
    /// * `ctx` - Optional context providing calendar or frequency as needed
    ///
    /// # Returns
    ///
    /// - `Ok(0.0)` if `start == end`
    /// - `Ok(year_fraction)` for the calculated year fraction (always ≥ 0)
    ///
    /// # Errors
    ///
    /// Returns an error when:
    /// - [`InputError::InvalidDateRange`](crate::error::InputError::InvalidDateRange):
    ///   `start > end` (inverted date range)
    /// - [`InputError::MissingCalendarForBus252`](crate::error::InputError::MissingCalendarForBus252):
    ///   Using `Bus252` without a calendar in `ctx`
    /// - [`InputError::InvalidBusBasis`](crate::error::InputError::InvalidBusBasis):
    ///   Using `Bus252` with a zero basis
    /// - [`InputError::MissingFrequencyForActActIsma`](crate::error::InputError::MissingFrequencyForActActIsma):
    ///   Using `ActActIsma` without a frequency in `ctx`
    /// - [`InputError::ActActIsmaUnsupportedFrequency`](crate::error::InputError::ActActIsmaUnsupportedFrequency):
    ///   Using `ActActIsma` with a Day or Week frequency
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::dates::{Date, DayCount, DayCountCtx};
    /// use time::Month;
    ///
    /// let start = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
    /// let end = Date::from_calendar_date(2025, Month::July, 1).expect("Valid date");
    ///
    /// let yf = DayCount::Act360.year_fraction(start, end, DayCountCtx::default())?;
    /// assert!(yf > 0.0);
    /// # Ok::<(), finstack_core::Error>(())
    /// ```
    pub fn year_fraction(self, start: Date, end: Date, ctx: DayCountCtx<'_>) -> crate::Result<f64> {
        // Early returns for edge cases - flattens nesting
        if start > end {
            return Err(InputError::InvalidDateRange.into());
        }
        if start == end {
            return Ok(0.0);
        }

        // Dispatch to convention-specific calculations
        self.year_fraction_impl(start, end, ctx)
    }

    /// Internal implementation dispatching to convention-specific calculations.
    ///
    /// Precondition: `start < end` (validated by `year_fraction`).
    fn year_fraction_impl(
        self,
        start: Date,
        end: Date,
        ctx: DayCountCtx<'_>,
    ) -> crate::Result<f64> {
        let days = (end - start).whole_days() as f64;

        match self {
            DayCount::Act360 => Ok(days / 360.0),
            DayCount::Act365F => Ok(days / 365.0),
            DayCount::Act365L => Ok(year_fraction_act_365l(start, end)),
            DayCount::Thirty360 => {
                Ok(days_30_360(start, end, Thirty360Convention::Us) as f64 / 360.0)
            }
            DayCount::ThirtyE360 => {
                Ok(days_30_360(start, end, Thirty360Convention::European) as f64 / 360.0)
            }
            DayCount::ActAct => year_fraction_act_act_isda(start, end),
            DayCount::ActActIsma => year_fraction_act_act_isma_with_ctx(start, end, ctx),
            DayCount::Bus252 => year_fraction_bus252(start, end, ctx),
        }
    }

    /// Calculate signed year fraction between two dates.
    ///
    /// Returns positive if `end > start`, negative if `end < start`, and zero if equal.
    /// This is useful for cashflow discounting where time can be negative relative to a base date.
    ///
    /// # Arguments
    ///
    /// * `start` - Reference date
    /// * `end` - Target date
    /// * `ctx` - Optional context providing calendar or frequency as needed
    ///
    /// # Returns
    ///
    /// - `Ok(0.0)` if `start == end`
    /// - `Ok(positive)` if `end > start`
    /// - `Ok(negative)` if `end < start`
    ///
    /// # Errors
    ///
    /// Same errors as [`year_fraction`](Self::year_fraction), but never returns
    /// `InvalidDateRange` since inverted dates produce negative fractions.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::dates::{Date, DayCount, DayCountCtx};
    /// use time::Month;
    ///
    /// let base = Date::from_calendar_date(2025, Month::July, 1).expect("Valid date");
    /// let past = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
    /// let future = Date::from_calendar_date(2026, Month::January, 1).expect("Valid date");
    ///
    /// let yf_past = DayCount::Act365F.signed_year_fraction(base, past, DayCountCtx::default())?;
    /// let yf_future = DayCount::Act365F.signed_year_fraction(base, future, DayCountCtx::default())?;
    ///
    /// assert!(yf_past < 0.0);  // Past is negative
    /// assert!(yf_future > 0.0); // Future is positive
    /// # Ok::<(), finstack_core::Error>(())
    /// ```
    pub fn signed_year_fraction(
        self,
        start: Date,
        end: Date,
        ctx: DayCountCtx<'_>,
    ) -> crate::Result<f64> {
        if start == end {
            Ok(0.0)
        } else if end > start {
            self.year_fraction(start, end, ctx)
        } else {
            Ok(-self.year_fraction(end, start, ctx)?)
        }
    }
}

/// Calculate ACT/ACT (ICMA/ISMA) year fraction using explicit reference coupon boundaries.
///
/// This helper is intended for irregular first/last coupons where the regular
/// coupon period cannot be inferred from `start`, `end`, and `frequency` alone.
/// The `reference_start`/`reference_end` pair must describe one regular coupon
/// period from the underlying schedule.
pub fn act_act_isma_year_fraction_with_reference_period(
    start: Date,
    end: Date,
    reference_start: Date,
    reference_end: Date,
) -> crate::Result<f64> {
    if start > end {
        return Err(InputError::InvalidDateRange.into());
    }
    if start == end {
        return Ok(0.0);
    }
    if reference_start >= reference_end {
        return Err(InputError::InvalidDateRange.into());
    }

    let period_months = reference_start.months_until(reference_end);
    if period_months == 0 {
        return Err(InputError::Invalid.into());
    }
    let coupon_length_years = period_months as f64 / 12.0;

    fn recurse(
        start: Date,
        end: Date,
        reference_start: Date,
        reference_end: Date,
        period_months: u32,
        coupon_length_years: f64,
    ) -> crate::Result<f64> {
        if start == end {
            return Ok(0.0);
        }
        if reference_start >= reference_end {
            return Err(InputError::InvalidDateRange.into());
        }

        if start >= reference_start && end <= reference_end {
            let accrual_days = (end - start).whole_days() as f64;
            let reference_days = (reference_end - reference_start).whole_days() as f64;
            if reference_days <= 0.0 {
                return Err(InputError::Invalid.into());
            }
            return Ok((accrual_days / reference_days) * coupon_length_years);
        }

        let period_months_i32 = i32::try_from(period_months).map_err(|_| InputError::Invalid)?;

        if end <= reference_start {
            let previous_start = reference_start.add_months(-period_months_i32);
            return recurse(
                start,
                end,
                previous_start,
                reference_start,
                period_months,
                coupon_length_years,
            );
        }

        if start >= reference_end {
            let next_end = reference_end.add_months(period_months_i32);
            return recurse(
                start,
                end,
                reference_end,
                next_end,
                period_months,
                coupon_length_years,
            );
        }

        if start < reference_start {
            let previous_start = reference_start.add_months(-period_months_i32);
            return Ok(recurse(
                start,
                reference_start,
                previous_start,
                reference_start,
                period_months,
                coupon_length_years,
            )? + recurse(
                reference_start,
                end,
                reference_start,
                reference_end,
                period_months,
                coupon_length_years,
            )?);
        }

        if end > reference_end {
            let next_end = reference_end.add_months(period_months_i32);
            return Ok(recurse(
                start,
                reference_end,
                reference_start,
                reference_end,
                period_months,
                coupon_length_years,
            )? + recurse(
                reference_end,
                end,
                reference_end,
                next_end,
                period_months,
                coupon_length_years,
            )?);
        }

        Err(InputError::Invalid.into())
    }

    recurse(
        start,
        end,
        reference_start,
        reference_end,
        period_months,
        coupon_length_years,
    )
}

// -------------------------------------------------------------------------------------------------
// 30/360 generalized helper
// -------------------------------------------------------------------------------------------------
/// 30/360 day-count variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Thirty360Convention {
    /// 30U/360 (US Bond Basis).
    Us,
    /// 30E/360 (European).
    European,
}

/// Compute day count between `start` (inclusive) and `end` (exclusive) under a 30/360 convention.
///
/// Precondition: `start <= end`. If violated, the returned value will be negative.
/// This helper is panic-free and allocation-free.
#[inline]
pub(crate) fn days_30_360(start: Date, end: Date, convention: Thirty360Convention) -> i32 {
    let (y1, m1, d1) = (start.year(), start.month() as i32, start.day() as i32);
    let (y2, m2, d2) = (end.year(), end.month() as i32, end.day() as i32);

    let (d1_adj, d2_adj) = match convention {
        Thirty360Convention::Us => {
            // ISDA 2006 §4.16(f) - 30/360 US:
            // - If D1 is 31 or last day of February, change D1 to 30
            // - If D2 is 31 and D1 was adjusted to 30, change D2 to 30
            // - If D2 is last day of Feb AND D1 was last day of Feb, change D2 to 30
            let d1_adj = if d1 == 31 || is_last_day_of_february(start) {
                30
            } else {
                d1
            };
            let d2_adj = if (d2 == 31 && d1_adj == 30)
                || (is_last_day_of_february(end) && is_last_day_of_february(start))
            {
                30
            } else {
                d2
            };
            (d1_adj, d2_adj)
        }
        Thirty360Convention::European => {
            // ISDA 2006 §4.16(g) - 30E/360:
            // - If D1 is 31, change D1 to 30
            // - If D2 is 31, change D2 to 30
            // Note: NO February EOM rule for European convention
            let d1_adj = if d1 == 31 { 30 } else { d1 };
            let d2_adj = if d2 == 31 { 30 } else { d2 };
            (d1_adj, d2_adj)
        }
    };

    (y2 - y1) * 360 + (m2 - m1) * 30 + (d2_adj - d1_adj)
}

/// Check if date is the last day of February (28 or 29 depending on leap year).
///
/// Per ISDA 2006 §4.16(f), the last day of February receives special treatment
/// in 30/360 US convention calculations.
#[inline]
fn is_last_day_of_february(date: Date) -> bool {
    date.month() == Month::February && date.day() == date.month().length(date.year())
}

// (Wrappers removed in favor of the public `days_30_360` with `Thirty360Convention`.)

// -------------------------------------------------------------------------------------------------
// ACT/ACT (ISDA) helper
// -------------------------------------------------------------------------------------------------
fn year_fraction_act_act_isda(start: Date, end: Date) -> crate::Result<f64> {
    if start == end {
        return Ok(0.0);
    }

    if start.year() == end.year() {
        let denom = days_in_year(start.year()) as f64;
        let days = (end - start).whole_days() as f64;
        return Ok(days / denom);
    }

    // Days from start to 31-Dec of start year (inclusive of start, exclusive of next year 1-Jan).
    let start_year_end = crate::dates::create_date(start.year() + 1, Month::January, 1)?;
    let days_start_year = (start_year_end - start).whole_days() as f64;
    let mut frac = days_start_year / days_in_year(start.year()) as f64;

    // Full intermediate years
    for _year in (start.year() + 1)..end.year() {
        frac += 1.0; // each full year counts as exactly 1.0
    }

    // Days from 1-Jan of end year to end date
    let start_of_end_year = crate::dates::create_date(end.year(), Month::January, 1)?;
    let days_end_year = (end - start_of_end_year).whole_days() as f64;
    frac += days_end_year / days_in_year(end.year()) as f64;

    Ok(frac)
}

// -------------------------------------------------------------------------------------------------
// Context-aware helpers for year_fraction_impl
// -------------------------------------------------------------------------------------------------

/// ACT/ACT (ISMA) with context extraction - validates frequency is present.
fn year_fraction_act_act_isma_with_ctx(
    start: Date,
    end: Date,
    ctx: DayCountCtx<'_>,
) -> crate::Result<f64> {
    let freq = ctx
        .frequency
        .ok_or(InputError::MissingFrequencyForActActIsma)?;
    year_fraction_act_act_isma(start, end, freq)
}

/// Bus/252 with context extraction - validates calendar is present and basis is non-zero.
fn year_fraction_bus252(start: Date, end: Date, ctx: DayCountCtx<'_>) -> crate::Result<f64> {
    let cal = ctx.calendar.ok_or(InputError::MissingCalendarForBus252)?;
    let basis = ctx.bus_basis.unwrap_or(252);
    if basis == 0 {
        return Err(InputError::InvalidBusBasis { basis }.into());
    }
    let biz_days = count_business_days(start, end, cal) as f64;
    Ok(biz_days / f64::from(basis))
}

// -------------------------------------------------------------------------------------------------
// ACT/ACT (ISMA/ICMA) helper
// -------------------------------------------------------------------------------------------------
/// Calculate year fraction for ACT/ACT (ISMA/ICMA) convention with coupon-period awareness.
fn year_fraction_act_act_isma(start: Date, end: Date, freq: Tenor) -> crate::Result<f64> {
    if start == end {
        return Ok(0.0);
    }

    // Coupon length in years based on frequency (e.g., 0.5 for semi-annual, 0.25 for quarterly).
    // ISMA/ICMA is defined for regular coupon periods; treat Week/Day frequencies as invalid.
    let coupon_length_years = match freq.unit {
        TenorUnit::Months => freq.count as f64 / 12.0,
        TenorUnit::Years => freq.count as f64,
        TenorUnit::Weeks | TenorUnit::Days => {
            return Err(InputError::ActActIsmaUnsupportedFrequency {
                frequency: freq.to_string(),
            }
            .into());
        }
    };

    // For ISMA, we need to work with quasi-coupon periods
    // We'll generate a schedule that encompasses the period and then
    // calculate the year fraction for each sub-period

    let mut total_fraction = 0.0;

    // Generate schedule to find quasi-coupon periods
    // We need to extend backward/forward to capture the full coupon periods
    let extended_start = extend_backward_for_coupon_period(start, freq);
    let extended_end = extend_forward_for_coupon_period(end, freq)?;

    // Optimization: Manually generate dates to avoid heap allocation of ScheduleBuilder
    // Most ISMA calculations involve very few periods, but long-dated bonds (15+ years)
    // with semi-annual coupons can have 30+ periods. Using 32 elements covers ~16 years
    // of semi-annual coupons without heap allocation.
    let mut periods: SmallVec<[Date; 32]> = SmallVec::new();
    let mut current = extended_start;
    periods.push(current);

    while current < extended_end {
        let next = freq.add_to_date(current, None, BusinessDayConvention::Unadjusted)?;

        current = if next > extended_end {
            extended_end
        } else {
            next
        };
        periods.push(current);
    }

    // Find the periods that overlap with our [start, end) interval
    for window in periods.windows(2) {
        let period_start = window[0];
        let period_end = window[1];

        // Check if this period overlaps with our target interval
        let overlap_start = start.max(period_start);
        let overlap_end = end.min(period_end);

        if overlap_start < overlap_end {
            // Numerator: actual days in the overlapping slice
            let days_in_overlap = (overlap_end - overlap_start).whole_days() as f64;

            // Denominator (ISMA): actual days in the coupon period that contains this slice
            let coupon_days = (period_end - period_start).whole_days() as f64;
            if coupon_days <= 0.0 {
                return Err(InputError::Invalid.into());
            }

            // Year fraction = (days in slice / days in coupon period) × coupon period in years
            total_fraction += (days_in_overlap / coupon_days) * coupon_length_years;
        }
    }

    Ok(total_fraction)
}

/// Extend start date backward to find the beginning of its coupon period.
fn extend_backward_for_coupon_period(date: Date, freq: Tenor) -> Date {
    match freq.unit {
        TenorUnit::Months => date.add_months(-(freq.count as i32)),
        TenorUnit::Years => date.add_months(-(freq.count as i32) * 12),
        TenorUnit::Weeks => date - Duration::weeks(freq.count as i64),
        TenorUnit::Days => date - Duration::days(freq.count as i64),
    }
}

/// Extend end date forward to find the end of its coupon period.
fn extend_forward_for_coupon_period(date: Date, freq: Tenor) -> crate::Result<Date> {
    // ISMA uses unadjusted quasi-coupon periods. This call should be infallible for
    // valid tenor inputs, but we keep it fallible to avoid silently degrading results.
    freq.add_to_date(date, None, BusinessDayConvention::Unadjusted)
}

// -------------------------------------------------------------------------------------------------
// ACT/365L helper
// -------------------------------------------------------------------------------------------------
/// Calculate year fraction for Act/365L convention.
///
/// Act/365L uses 366 as denominator if February 29 falls in the closed interval
/// `[start, end]`, otherwise uses 365.
fn year_fraction_act_365l(start: Date, end: Date) -> f64 {
    if start == end {
        return 0.0;
    }

    let actual_days = (end - start).whole_days() as f64;

    // ACT/365L uses a closed interval for the leap-day denominator rule,
    // even though actual days are still counted using the library's
    // standard [start, end) convention.
    let denominator = if contains_feb_29(start, end) {
        366.0
    } else {
        365.0
    };

    actual_days / denominator
}

/// Check if February 29 falls in the closed interval `[start, end]`.
fn contains_feb_29(start: Date, end: Date) -> bool {
    let start_year = start.year();
    let end_year = end.year();

    // Check each year in the range for Feb 29
    for year in start_year..=end_year {
        if time::util::is_leap_year(year) {
            // Try to create Feb 29 for this year
            if let Ok(feb_29) = Date::from_calendar_date(year, Month::February, 29) {
                if feb_29 >= start && feb_29 <= end {
                    return true;
                }
            }
        }
    }
    false
}

// -------------------------------------------------------------------------------------------------
// Bus/252 helper
// -------------------------------------------------------------------------------------------------
/// Count business days between start (inclusive) and end (exclusive) using the given calendar.
fn count_business_days<C: HolidayCalendar + ?Sized>(start: Date, end: Date, calendar: &C) -> i32 {
    BusinessDayIter::new(start, end, calendar).count() as i32
}

#[inline]
const fn days_in_year(year: i32) -> i32 {
    if time::util::is_leap_year(year) {
        366
    } else {
        365
    }
}

// -------------------------------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use time::Duration;

    fn make_date(y: i32, m: u8, d: u8) -> Date {
        Date::from_calendar_date(y, Month::try_from(m).expect("Valid month (1-12)"), d)
            .expect("Valid test date")
    }

    #[test]
    fn act360_basic() {
        let start = make_date(2025, 1, 1);
        let end = start + Duration::days(360);
        let yf = DayCount::Act360
            .year_fraction(start, end, DayCountCtx::default())
            .expect("Year fraction calculation should succeed in test");
        assert!((yf - 1.0).abs() < 1e-9);
    }

    #[test]
    fn act365f_year_fraction() {
        let start = make_date(2025, 3, 1);
        let end = make_date(2026, 3, 1);
        let yf = DayCount::Act365F
            .year_fraction(start, end, DayCountCtx::default())
            .expect("Year fraction calculation should succeed in test");
        // ACT/365F uses actual days / 365. For 2025-03-01 -> 2026-03-01 there are
        // 365 actual days (no leap day), so expected = 365 / 365.
        let expected = (end - start).whole_days() as f64 / 365.0;
        assert!((yf - expected).abs() < 1e-9);
    }

    #[test]
    fn thirty_360_end_of_month() {
        let start = make_date(2025, 1, 31);
        let end = make_date(2025, 2, 28);
        let days = DayCount::Thirty360
            .days(start, end)
            .expect("Days calculation should succeed in test");
        assert_eq!(days, 28);
        let yf = DayCount::Thirty360
            .year_fraction(start, end, DayCountCtx::default())
            .expect("Year fraction calculation should succeed in test");
        assert!((yf * 360.0 - days as f64).abs() < 1e-9);
    }

    #[test]
    fn actact_spanning_years() {
        let start = make_date(2024, 7, 1); // includes leap year 2024
        let end = make_date(2026, 1, 1);
        let yf = DayCount::ActAct
            .year_fraction(start, end, DayCountCtx::default())
            .expect("Year fraction calculation should succeed in test");
        // compute expected manually: part of 2024 (184 days from Jul1 to Jan1 2025), 2025 full year (365 days), part of 2026 (0). Actually end Jan1 so 0.
        let expected = 184.0 / 366.0 + 1.0; // plus full 2025 year 365/365 =1
        assert!((yf - expected).abs() < 1e-9);
    }

    #[test]
    fn error_on_inverted_dates() {
        let start = make_date(2025, 1, 1);
        let end = make_date(2024, 1, 1);
        assert!(DayCount::Act360
            .year_fraction(start, end, DayCountCtx::default())
            .is_err());
    }

    #[test]
    fn act365l_without_leap_day() {
        // Period that doesn't contain Feb 29
        let start = make_date(2025, 3, 1); // 2025 is not a leap year
        let end = make_date(2025, 9, 1);
        let yf = DayCount::Act365L
            .year_fraction(start, end, DayCountCtx::default())
            .expect("Year fraction calculation should succeed in test");
        let actual_days = (end - start).whole_days() as f64;
        let expected = actual_days / 365.0; // Should use 365 denominator
        assert!((yf - expected).abs() < 1e-9);
    }

    #[test]
    fn act365l_with_leap_day() {
        // Period that contains Feb 29, 2024 (leap year)
        let start = make_date(2024, 2, 28); // Feb 28, 2024
        let end = make_date(2024, 3, 2); // Mar 2, 2024 (contains Feb 29)
        let yf = DayCount::Act365L
            .year_fraction(start, end, DayCountCtx::default())
            .expect("Year fraction calculation should succeed in test");
        let actual_days = (end - start).whole_days() as f64; // 3 days
        let expected = actual_days / 366.0; // Should use 366 denominator due to Feb 29
        assert!((yf - expected).abs() < 1e-9);
    }

    #[test]
    fn act365l_includes_starting_on_feb_29() {
        // Period that starts on Feb 29 should still use 366 denominator
        let start = make_date(2024, 2, 29);
        let end = make_date(2024, 3, 2);
        let yf = DayCount::Act365L
            .year_fraction(start, end, DayCountCtx::default())
            .expect("Year fraction calculation should succeed in test");
        let actual_days = (end - start).whole_days() as f64; // 2 days
        let expected = actual_days / 366.0;
        assert!((yf - expected).abs() < 1e-9);
    }

    #[test]
    fn act365l_leap_year_boundary() {
        // Start in leap year, end after leap year
        let start = make_date(2024, 2, 20); // Before Feb 29
        let end = make_date(2025, 1, 15); // After leap year
        let yf = DayCount::Act365L
            .year_fraction(start, end, DayCountCtx::default())
            .expect("Year fraction calculation should succeed in test");
        let actual_days = (end - start).whole_days() as f64;
        let expected = actual_days / 366.0; // Should use 366 due to Feb 29 in period
        assert!((yf - expected).abs() < 1e-9);
    }

    #[test]
    fn act365l_leap_year_before_period() {
        // Feb 29 exists in year but falls before start date
        let start = make_date(2024, 3, 1); // After Feb 29, 2024
        let end = make_date(2024, 6, 1); // Later in same leap year
        let yf = DayCount::Act365L
            .year_fraction(start, end, DayCountCtx::default())
            .expect("Year fraction calculation should succeed in test");
        let actual_days = (end - start).whole_days() as f64;
        let expected = actual_days / 365.0; // Should use 365 since Feb 29 not in (start, end]
        assert!((yf - expected).abs() < 1e-9);
    }

    // Simple test-only calendar that treats only weekends as holidays
    #[derive(Debug, Clone, Copy)]
    struct WeekendsOnly;

    impl crate::dates::HolidayCalendar for WeekendsOnly {
        fn is_holiday(&self, _date: Date) -> bool {
            // Return false for all dates; business day logic will still exclude weekends
            false
        }
    }

    #[test]
    fn bus252_with_calendar() {
        // Simple test with weekends-only calendar (Monday to Friday)
        let calendar = WeekendsOnly;
        let start = make_date(2025, 1, 6); // Monday
        let end = make_date(2025, 1, 13); // Next Monday (7 calendar days, 5 business days)

        let biz_days = DayCount::Bus252
            .year_fraction(
                start,
                end,
                DayCountCtx {
                    calendar: Some(&calendar),
                    frequency: None,
                    bus_basis: None,
                },
            )
            .expect("Year fraction calculation should succeed in test")
            * 252.0;
        assert_eq!(biz_days.round() as i32, 5);

        let yf = DayCount::Bus252
            .year_fraction(
                start,
                end,
                DayCountCtx {
                    calendar: Some(&calendar),
                    frequency: None,
                    bus_basis: None,
                },
            )
            .expect("Year fraction calculation should succeed in test");
        let expected = 5.0 / 252.0;
        assert!((yf - expected).abs() < 1e-9);
    }

    #[test]
    fn bus252_with_nyse_calendar() {
        use crate::dates::calendar::NYSE;

        // Test with a real calendar that has holidays
        let calendar = NYSE;
        let start = make_date(2025, 1, 2); // Thu (after New Year holiday)
        let end = make_date(2025, 1, 6); // Mon (4 calendar days)

        let biz_days = DayCount::Bus252
            .year_fraction(
                start,
                end,
                DayCountCtx {
                    calendar: Some(&calendar),
                    frequency: None,
                    bus_basis: None,
                },
            )
            .expect("Year fraction calculation should succeed in test")
            * 252.0;
        // Should count Thu, Fri (Sat, Sun are weekends)
        assert_eq!(biz_days.round() as i32, 2);

        let yf = DayCount::Bus252
            .year_fraction(
                start,
                end,
                DayCountCtx {
                    calendar: Some(&calendar),
                    frequency: None,
                    bus_basis: None,
                },
            )
            .expect("Year fraction calculation should succeed in test");
        let expected = 2.0 / 252.0;
        assert!((yf - expected).abs() < 1e-9);
    }

    #[test]
    fn bus252_error_without_calendar() {
        // Bus/252 should error when using regular methods without calendar
        let start = make_date(2025, 1, 1);
        let end = make_date(2025, 1, 8);

        assert!(DayCount::Bus252.days(start, end).is_err());
        assert!(DayCount::Bus252
            .year_fraction(start, end, DayCountCtx::default())
            .is_err());
    }

    #[test]
    fn bus252_equal_dates() {
        let calendar = WeekendsOnly;
        let date = make_date(2025, 1, 1);

        let biz_days = DayCount::Bus252
            .year_fraction(
                date,
                date,
                DayCountCtx {
                    calendar: Some(&calendar),
                    frequency: None,
                    bus_basis: None,
                },
            )
            .expect("Year fraction calculation should succeed in test")
            * 252.0;
        assert_eq!(biz_days.round() as i32, 0);

        let yf = DayCount::Bus252
            .year_fraction(
                date,
                date,
                DayCountCtx {
                    calendar: Some(&calendar),
                    frequency: None,
                    bus_basis: None,
                },
            )
            .expect("Year fraction calculation should succeed in test");
        assert_eq!(yf, 0.0);
    }

    #[test]
    fn actact_isma_semi_annual() {
        // Test ACT/ACT (ISMA) with semi-annual frequency
        let start = make_date(2025, 1, 15);
        let end = make_date(2025, 7, 15);
        let freq = Tenor::semi_annual(); // Semi-annual

        let yf = DayCount::ActActIsma
            .year_fraction(
                start,
                end,
                DayCountCtx {
                    calendar: None,
                    frequency: Some(freq),
                    bus_basis: None,
                },
            )
            .expect("Year fraction calculation should succeed in test");

        // Under ISMA, a full semi-annual period = 0.5 year fraction
        assert!((yf - 0.5).abs() < 1e-6, "Expected 0.5, got {}", yf);
    }

    #[test]
    fn actact_isma_quarterly() {
        // Test ACT/ACT (ISMA) with quarterly frequency
        let start = make_date(2025, 1, 1);
        let end = make_date(2025, 4, 1);
        let freq = Tenor::quarterly(); // Quarterly

        let yf = DayCount::ActActIsma
            .year_fraction(
                start,
                end,
                DayCountCtx {
                    calendar: None,
                    frequency: Some(freq),
                    bus_basis: None,
                },
            )
            .expect("Year fraction calculation should succeed in test");

        // Under ISMA, a full quarterly period = 0.25 year fraction
        assert!((yf - 0.25).abs() < 1e-6, "Expected 0.25, got {}", yf);
    }

    #[test]
    fn actact_isma_annual() {
        // Test ACT/ACT (ISMA) with annual frequency
        let start = make_date(2025, 1, 1);
        let end = make_date(2026, 1, 1);
        let freq = Tenor::annual(); // Annual

        let yf = DayCount::ActActIsma
            .year_fraction(
                start,
                end,
                DayCountCtx {
                    calendar: None,
                    frequency: Some(freq),
                    bus_basis: None,
                },
            )
            .expect("Year fraction calculation should succeed in test");

        // For a full year period, this should be exactly 1.0
        assert!((yf - 1.0).abs() < 1e-9, "Expected 1.0, got {}", yf);
    }

    #[test]
    fn actact_isma_spanning_leap_year() {
        // Test ACT/ACT (ISMA) spanning a leap year boundary
        let start = make_date(2023, 7, 1); // Mid-2023 (non-leap)
        let end = make_date(2024, 7, 1); // Mid-2024 (leap year)
        let freq = Tenor::semi_annual(); // Semi-annual

        let yf = DayCount::ActActIsma
            .year_fraction(
                start,
                end,
                DayCountCtx {
                    calendar: None,
                    frequency: Some(freq),
                    bus_basis: None,
                },
            )
            .expect("Year fraction calculation should succeed in test");

        // Two full semi-annual coupon periods = 2 × 0.5 = 1.0 year fraction
        assert!((yf - 1.0).abs() < 1e-6, "Expected 1.0, got {}", yf);
    }

    #[test]
    fn actact_isma_partial_period() {
        // Test ACT/ACT (ISMA) for a partial coupon period
        let start = make_date(2025, 1, 15); // Mid-month start
        let end = make_date(2025, 3, 15); // Two months later
        let freq = Tenor::semi_annual(); // Semi-annual coupons

        let yf = DayCount::ActActIsma
            .year_fraction(
                start,
                end,
                DayCountCtx {
                    calendar: None,
                    frequency: Some(freq),
                    bus_basis: None,
                },
            )
            .expect("Year fraction calculation should succeed in test");

        // Two months out of a 6-month coupon, as year fraction:
        // ~0.33 coupon fraction × 0.5 (semi-annual) = ~0.167 year fraction
        assert!(yf > 0.15 && yf < 0.18, "Expected ~0.167, got {}", yf);
    }

    #[test]
    fn actact_isma_monthly_frequency() {
        // Test ACT/ACT (ISMA) with monthly frequency
        let start = make_date(2025, 1, 1);
        let end = make_date(2025, 2, 1);
        let freq = Tenor::monthly(); // Monthly

        let yf = DayCount::ActActIsma
            .year_fraction(
                start,
                end,
                DayCountCtx {
                    calendar: None,
                    frequency: Some(freq),
                    bus_basis: None,
                },
            )
            .expect("Year fraction calculation should succeed in test");

        // For a full monthly period = 1/12 year fraction ≈ 0.0833
        let expected = 1.0 / 12.0;
        assert!(
            (yf - expected).abs() < 1e-6,
            "Expected {}, got {}",
            expected,
            yf
        );
    }

    #[test]
    fn actact_isma_error_on_inverted_dates() {
        // ACT/ACT (ISMA) should error on inverted dates
        let start = make_date(2025, 1, 1);
        let end = make_date(2024, 1, 1);
        let freq = Tenor::semi_annual();

        assert!(DayCount::ActActIsma
            .year_fraction(
                start,
                end,
                DayCountCtx {
                    calendar: None,
                    frequency: Some(freq),
                    bus_basis: None,
                }
            )
            .is_err());
    }

    #[test]
    fn actact_isma_equal_dates() {
        // ACT/ACT (ISMA) should return 0.0 for equal dates
        let date = make_date(2025, 1, 1);
        let freq = Tenor::semi_annual();

        let yf = DayCount::ActActIsma
            .year_fraction(
                date,
                date,
                DayCountCtx {
                    calendar: None,
                    frequency: Some(freq),
                    bus_basis: None,
                },
            )
            .expect("Year fraction calculation should succeed in test");
        assert_eq!(yf, 0.0);
    }

    #[test]
    fn actact_isma_vs_isda_comparison() {
        // Compare ACT/ACT (ISMA) vs ACT/ACT (ISDA) for the same period
        let start = make_date(2024, 6, 15);
        let end = make_date(2025, 6, 15);
        let freq = Tenor::semi_annual(); // Semi-annual

        let yf_isda = DayCount::ActAct
            .year_fraction(start, end, DayCountCtx::default())
            .expect("Year fraction calculation should succeed in test");
        let yf_isma = DayCount::ActActIsma
            .year_fraction(
                start,
                end,
                DayCountCtx {
                    calendar: None,
                    frequency: Some(freq),
                    bus_basis: None,
                },
            )
            .expect("Year fraction calculation should succeed in test");

        // Both ISDA and ISMA should return ~1.0 for a full year
        // ISDA splits by calendar year → ~1.0
        // ISMA sums 2 semi-annual periods × 0.5 each → ~1.0
        assert!(
            yf_isda > 0.99 && yf_isda < 1.01,
            "ISDA: Expected ~1.0, got {}",
            yf_isda
        );
        assert!(
            yf_isma > 0.99 && yf_isma < 1.01,
            "ISMA: Expected ~1.0, got {}",
            yf_isma
        );
        // Both methods should give approximately the same result for a full year
        assert!((yf_isma - yf_isda).abs() < 0.02);
    }
}
