//! Tenor parsing and calendar-aware year fraction computation.
//!
//! This module provides market-standard tenor parsing that respects business day
//! conventions and holiday calendars. Unlike simple approximations (1M = 30 days),
//! these functions compute actual year fractions using proper date arithmetic.
//!
//! # Examples
//!
//! ```rust
//! use finstack_core::dates::{Tenor, TenorUnit, Date, DayCount, DayCountCtx};
//! use finstack_core::dates::{BusinessDayConvention, HolidayCalendar};
//! use time::Month;
//!
//! // Parse a tenor string
//! let tenor = Tenor::parse("3M").expect("Valid tenor");
//! assert_eq!(tenor.count, 3);
//! assert_eq!(tenor.unit, TenorUnit::Months);
//!
//! // Convert to years with default settings (simple approximation)
//! let years = tenor.to_years_simple();
//! assert!((years - 0.25).abs() < 1e-6);
//! ```
//!
//! # Calendar-Aware Computation
//!
//! For accurate day counting that respects holidays and business day conventions:
//!
//! ```rust,no_run
//! use finstack_core::dates::{Tenor, Date, DayCount, DayCountCtx, BusinessDayConvention};
//! use finstack_core::dates::calendar::TARGET2;
//! use time::Month;
//!
//! let as_of = Date::from_calendar_date(2025, Month::January, 31).unwrap();
//! let tenor = Tenor::parse("1M").unwrap();
//!
//! // Calendar-aware: 1M from Jan 31 -> Feb 28 (end of month)
//! let end_date = tenor.add_to_date(
//!     as_of,
//!     Some(&TARGET2),
//!     BusinessDayConvention::ModifiedFollowing,
//! ).unwrap();
//!
//! let years = tenor.to_years_with_context(
//!     as_of,
//!     Some(&TARGET2),
//!     BusinessDayConvention::ModifiedFollowing,
//!     DayCount::ActAct,
//! ).unwrap();
//! ```

use crate::dates::{adjust, BusinessDayConvention, Date, DayCount, DayCountCtx, HolidayCalendar};
use crate::error::InputError;
use time::Duration;

/// Unit of a tenor period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum TenorUnit {
    /// Days (D)
    Days,
    /// Weeks (W)
    Weeks,
    /// Months (M)
    Months,
    /// Years (Y)
    Years,
}

impl TenorUnit {
    /// Parse a single character unit code.
    ///
    /// # Arguments
    /// * `c` - Unit character: 'D', 'W', 'M', or 'Y' (case-insensitive)
    ///
    /// # Returns
    /// The parsed `TenorUnit` or an error if the character is invalid.
    pub fn from_char(c: char) -> crate::Result<Self> {
        match c.to_ascii_uppercase() {
            'D' => Ok(Self::Days),
            'W' => Ok(Self::Weeks),
            'M' => Ok(Self::Months),
            'Y' => Ok(Self::Years),
            _ => Err(InputError::InvalidTenor {
                tenor: c.to_string(),
                reason: "unknown unit; expected D, W, M, or Y".to_string(),
            }
            .into()),
        }
    }
}

/// A parsed tenor representing a time period.
///
/// Tenors are commonly used in financial markets to specify maturities,
/// payment frequencies, and rate fixing periods.
///
/// # Examples
///
/// ```rust
/// use finstack_core::dates::{Tenor, TenorUnit};
///
/// let tenor = Tenor::new(3, TenorUnit::Months);
/// assert_eq!(tenor.count, 3);
/// assert_eq!(tenor.unit, TenorUnit::Months);
///
/// // Parse from string
/// let parsed = Tenor::parse("6M").unwrap();
/// assert_eq!(parsed.count, 6);
/// assert_eq!(parsed.unit, TenorUnit::Months);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Tenor {
    /// Number of units.
    pub count: u32,
    /// Unit type (days, weeks, months, years).
    pub unit: TenorUnit,
}

impl Tenor {
    /// Create a new tenor with the specified count and unit.
    ///
    /// # Arguments
    /// * `count` - Number of periods
    /// * `unit` - Period unit type
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::dates::{Tenor, TenorUnit};
    ///
    /// let quarterly = Tenor::new(3, TenorUnit::Months);
    /// let annual = Tenor::new(1, TenorUnit::Years);
    /// ```
    #[inline]
    pub const fn new(count: u32, unit: TenorUnit) -> Self {
        Self { count, unit }
    }

    /// Get the number of months if the tenor is month-based or year-based.
    ///
    /// Returns `Some(months)` for Month and Year units (converting years to months).
    /// Returns `None` for Day and Week units.
    pub const fn months(&self) -> Option<u32> {
        match self.unit {
            TenorUnit::Months => Some(self.count),
            TenorUnit::Years => Some(self.count * 12),
            _ => None,
        }
    }

    /// Get the number of days if the tenor is day-based or week-based.
    ///
    /// Returns `Some(days)` for Day and Week units (converting weeks to days).
    /// Returns `None` for Month and Year units.
    pub const fn days(&self) -> Option<u32> {
        match self.unit {
            TenorUnit::Days => Some(self.count),
            TenorUnit::Weeks => Some(self.count * 7),
            _ => None,
        }
    }

    /// Create a Tenor from a year fraction using a day count convention.
    ///
    /// If the year fraction corresponds to an integer number of months (within a small epsilon),
    /// it returns a Month-based tenor. Otherwise, it converts to days using the provided
    /// day count convention.
    ///
    /// # Arguments
    /// * `years` - The time period in years
    /// * `day_count` - The day count convention to use for day conversion
    pub fn from_years(years: f64, day_count: DayCount) -> Self {
        let months = years * 12.0;
        let rounded_months = months.round();

        if (months - rounded_months).abs() < 1e-4 {
            // It's effectively an integer number of months
            let m = rounded_months as u32;
            if m > 0 && m.is_multiple_of(12) {
                Self::new(m / 12, TenorUnit::Years)
            } else {
                Self::new(m, TenorUnit::Months)
            }
        } else {
            // Convert to days
            let days = match day_count {
                DayCount::Thirty360 | DayCount::ThirtyE360 => (years * 360.0).round(),
                DayCount::Act360 => (years * 360.0).round(),
                DayCount::Act365F => (years * 365.0).round(),
                _ => (years * 365.25).round(),
            };
            Self::new(days as u32, TenorUnit::Days)
        }
    }

    /// Parse a tenor string like "1D", "1W", "3M", "5Y".
    ///
    /// # Format
    ///
    /// `<count><unit>` where:
    /// - `count` is a positive integer
    /// - `unit` is one of: D (days), W (weeks), M (months), Y (years)
    ///
    /// Parsing is case-insensitive and trims whitespace.
    ///
    /// # Arguments
    /// * `s` - Tenor string to parse
    ///
    /// # Returns
    /// Parsed `Tenor` or an error if the format is invalid.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::dates::{Tenor, TenorUnit};
    ///
    /// assert!(Tenor::parse("1D").is_ok());
    /// assert!(Tenor::parse("3m").is_ok()); // Case-insensitive
    /// assert!(Tenor::parse("10Y").is_ok());
    /// assert!(Tenor::parse("").is_err());
    /// assert!(Tenor::parse("XY").is_err());
    /// ```
    pub fn parse(s: &str) -> crate::Result<Self> {
        let s = s.trim();

        if s.is_empty() {
            return Err(InputError::InvalidTenor {
                tenor: s.to_string(),
                reason: "empty tenor string".to_string(),
            }
            .into());
        }

        // Find where the unit character starts (last character)
        let s_upper = s.to_uppercase();

        // Find position of first alphabetic character
        let unit_pos =
            s_upper
                .find(|c: char| c.is_alphabetic())
                .ok_or_else(|| InputError::InvalidTenor {
                    tenor: s.to_string(),
                    reason: "no unit found; expected D, W, M, or Y suffix".to_string(),
                })?;

        let (count_str, unit_str) = s_upper.split_at(unit_pos);

        if count_str.is_empty() {
            return Err(InputError::InvalidTenor {
                tenor: s.to_string(),
                reason: "no count found; expected format like '3M' or '1Y'".to_string(),
            }
            .into());
        }

        let count: u32 = count_str.parse().map_err(|_| InputError::InvalidTenor {
            tenor: s.to_string(),
            reason: format!("invalid count '{}'; expected a positive integer", count_str),
        })?;

        if count == 0 {
            return Err(InputError::InvalidTenor {
                tenor: s.to_string(),
                reason: "count must be positive".to_string(),
            }
            .into());
        }

        // Unit should be exactly one character
        if unit_str.len() != 1 {
            return Err(InputError::InvalidTenor {
                tenor: s.to_string(),
                reason: format!(
                    "invalid unit '{}'; expected single character D, W, M, or Y",
                    unit_str
                ),
            }
            .into());
        }

        let unit_char = unit_str.chars().next().expect("checked length above");
        let unit = TenorUnit::from_char(unit_char)?;

        Ok(Self { count, unit })
    }

    /// Convert tenor to a simple year fraction approximation.
    ///
    /// This uses fixed approximations:
    /// - 1D = 1/365 years
    /// - 1W = 7/365 years
    /// - 1M = 1/12 years
    /// - 1Y = 1 year
    ///
    /// For more accurate calculations, use [`to_years_with_context`](Self::to_years_with_context).
    ///
    /// # Returns
    /// Year fraction as f64.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::dates::Tenor;
    ///
    /// let tenor = Tenor::parse("6M").unwrap();
    /// assert!((tenor.to_years_simple() - 0.5).abs() < 1e-6);
    ///
    /// let tenor = Tenor::parse("1Y").unwrap();
    /// assert!((tenor.to_years_simple() - 1.0).abs() < 1e-6);
    /// ```
    #[inline]
    pub fn to_years_simple(&self) -> f64 {
        let count = f64::from(self.count);
        match self.unit {
            TenorUnit::Days => count / 365.0,
            TenorUnit::Weeks => count * 7.0 / 365.0,
            TenorUnit::Months => count / 12.0,
            TenorUnit::Years => count,
        }
    }

    /// Add the tenor to a date, optionally respecting a business day calendar.
    ///
    /// # Arguments
    /// * `date` - Starting date
    /// * `calendar` - Optional holiday calendar for business day adjustment
    /// * `bdc` - Business day convention to apply if calendar is provided
    ///
    /// # Returns
    /// The resulting date after adding the tenor period.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::dates::{Tenor, Date, BusinessDayConvention};
    /// use time::Month;
    ///
    /// let start = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    /// let tenor = Tenor::parse("1M").unwrap();
    ///
    /// let end = tenor.add_to_date(start, None, BusinessDayConvention::ModifiedFollowing).unwrap();
    /// assert_eq!(end, Date::from_calendar_date(2025, Month::February, 15).unwrap());
    /// ```
    pub fn add_to_date(
        &self,
        date: Date,
        calendar: Option<&dyn HolidayCalendar>,
        bdc: BusinessDayConvention,
    ) -> crate::Result<Date> {
        use crate::dates::date_extensions::DateExt;

        let raw_date = match self.unit {
            TenorUnit::Days => date + Duration::days(i64::from(self.count)),
            TenorUnit::Weeks => date + Duration::weeks(i64::from(self.count)),
            TenorUnit::Months => date.add_months(self.count as i32),
            TenorUnit::Years => date.add_months((self.count as i32) * 12),
        };

        // Apply business day convention if calendar provided
        if let Some(cal) = calendar {
            adjust(raw_date, bdc, cal)
        } else {
            Ok(raw_date)
        }
    }

    /// Convert tenor to year fraction using calendar-aware date computation.
    ///
    /// This method computes the actual year fraction by:
    /// 1. Adding the tenor to the as-of date using proper date arithmetic
    /// 2. Applying business day adjustment if a calendar is provided
    /// 3. Computing the year fraction using the specified day count convention
    ///
    /// # Arguments
    /// * `as_of` - Starting date
    /// * `calendar` - Optional holiday calendar for business day adjustment
    /// * `bdc` - Business day convention to apply
    /// * `day_count` - Day count convention for year fraction calculation
    ///
    /// # Returns
    /// Year fraction computed using the specified conventions.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::dates::{Tenor, Date, DayCount, DayCountCtx, BusinessDayConvention};
    /// use time::Month;
    ///
    /// let as_of = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    /// let tenor = Tenor::parse("1Y").unwrap();
    ///
    /// let years = tenor.to_years_with_context(
    ///     as_of,
    ///     None,
    ///     BusinessDayConvention::ModifiedFollowing,
    ///     DayCount::ActAct,
    /// ).unwrap();
    ///
    /// assert!((years - 1.0).abs() < 0.01);
    /// ```
    pub fn to_years_with_context(
        &self,
        as_of: Date,
        calendar: Option<&dyn HolidayCalendar>,
        bdc: BusinessDayConvention,
        day_count: DayCount,
    ) -> crate::Result<f64> {
        let end_date = self.add_to_date(as_of, calendar, bdc)?;

        let ctx = DayCountCtx {
            calendar,
            frequency: None,
            bus_basis: None,
        };

        day_count.year_fraction(as_of, end_date, ctx)
    }

    /// Convenience constructor for overnight (1D) tenor.
    #[inline]
    pub const fn overnight() -> Self {
        Self::new(1, TenorUnit::Days)
    }

    /// Convenience constructor for 1 week tenor.
    #[inline]
    pub const fn one_week() -> Self {
        Self::new(1, TenorUnit::Weeks)
    }

    /// Convenience constructor for 1 month tenor.
    #[inline]
    pub const fn one_month() -> Self {
        Self::new(1, TenorUnit::Months)
    }

    /// Convenience constructor for 3 month (quarterly) tenor.
    #[inline]
    pub const fn three_months() -> Self {
        Self::new(3, TenorUnit::Months)
    }

    /// Convenience constructor for 6 month (semi-annual) tenor.
    #[inline]
    pub const fn six_months() -> Self {
        Self::new(6, TenorUnit::Months)
    }

    /// Convenience constructor for 1 year tenor.
    #[inline]
    pub const fn one_year() -> Self {
        Self::new(1, TenorUnit::Years)
    }

    /// Convenience constructor for Annual frequency (1 Year).
    #[inline]
    pub const fn annual() -> Self {
        Self::new(1, TenorUnit::Years)
    }

    /// Convenience constructor for Semi-Annual frequency (6 Months).
    #[inline]
    pub const fn semi_annual() -> Self {
        Self::new(6, TenorUnit::Months)
    }

    /// Convenience constructor for Quarterly frequency (3 Months).
    #[inline]
    pub const fn quarterly() -> Self {
        Self::new(3, TenorUnit::Months)
    }

    /// Convenience constructor for Bi-Monthly frequency (2 Months).
    #[inline]
    pub const fn bimonthly() -> Self {
        Self::new(2, TenorUnit::Months)
    }

    /// Convenience constructor for Monthly frequency (1 Month).
    #[inline]
    pub const fn monthly() -> Self {
        Self::new(1, TenorUnit::Months)
    }

    /// Convenience constructor for Bi-Weekly frequency (14 Days).
    #[inline]
    pub const fn biweekly() -> Self {
        Self::new(14, TenorUnit::Days)
    }

    /// Convenience constructor for Weekly frequency (7 Days).
    #[inline]
    pub const fn weekly() -> Self {
        Self::new(7, TenorUnit::Days)
    }

    /// Convenience constructor for Daily frequency (1 Day).
    #[inline]
    pub const fn daily() -> Self {
        Self::new(1, TenorUnit::Days)
    }

    /// Create a Tenor from payments per year.
    ///
    /// Returns an error if payments_per_year is 0 or does not divide 12 evenly.
    pub fn from_payments_per_year(payments: u32) -> crate::Result<Self> {
        if payments == 0 {
            return Err(InputError::InvalidTenor {
                tenor: format!("payments={}", payments),
                reason: "payments_per_year must be positive".to_string(),
            }
            .into());
        }

        // Try to fit into months first
        if 12 % payments == 0 {
            let months = 12 / payments;
            Ok(Self::new(months, TenorUnit::Months))
        } else {
            // If it doesn't fit into months, try roughly into weeks (52)
            // But standard market convention usually implies months.
            // Frequency::from_payments_per_year used to fail if not dividing 12.
            // We'll stick to that behavior for now to match strictness.
            Err(InputError::InvalidTenor {
                tenor: format!("payments={}", payments),
                reason: "payments_per_year for Tenor currently requires even division of 12 months"
                    .to_string(),
            }
            .into())
        }
    }
}

impl std::fmt::Display for Tenor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let unit_char = match self.unit {
            TenorUnit::Days => 'D',
            TenorUnit::Weeks => 'W',
            TenorUnit::Months => 'M',
            TenorUnit::Years => 'Y',
        };
        write!(f, "{}{}", self.count, unit_char)
    }
}

impl std::str::FromStr for Tenor {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_parse_valid_tenors() {
        let cases = vec![
            ("1D", 1, TenorUnit::Days),
            ("7D", 7, TenorUnit::Days),
            ("1W", 1, TenorUnit::Weeks),
            ("2W", 2, TenorUnit::Weeks),
            ("1M", 1, TenorUnit::Months),
            ("3M", 3, TenorUnit::Months),
            ("6M", 6, TenorUnit::Months),
            ("12M", 12, TenorUnit::Months),
            ("1Y", 1, TenorUnit::Years),
            ("5Y", 5, TenorUnit::Years),
            ("10Y", 10, TenorUnit::Years),
            ("30Y", 30, TenorUnit::Years),
        ];

        for (input, expected_count, expected_unit) in cases {
            let tenor = Tenor::parse(input).expect(input);
            assert_eq!(tenor.count, expected_count, "count mismatch for {}", input);
            assert_eq!(tenor.unit, expected_unit, "unit mismatch for {}", input);
        }
    }

    #[test]
    fn test_parse_case_insensitive() {
        assert_eq!(
            Tenor::parse("3m").expect("3m"),
            Tenor::parse("3M").expect("3M")
        );
        assert_eq!(
            Tenor::parse("1y").expect("1y"),
            Tenor::parse("1Y").expect("1Y")
        );
    }

    #[test]
    fn test_parse_with_whitespace() {
        let tenor = Tenor::parse("  3M  ").expect("trimmed");
        assert_eq!(tenor.count, 3);
        assert_eq!(tenor.unit, TenorUnit::Months);
    }

    #[test]
    fn test_parse_invalid_tenors() {
        assert!(Tenor::parse("").is_err());
        assert!(Tenor::parse("M").is_err()); // No count
        assert!(Tenor::parse("3").is_err()); // No unit
        assert!(Tenor::parse("3X").is_err()); // Invalid unit
        assert!(Tenor::parse("0M").is_err()); // Zero count
        assert!(Tenor::parse("-1M").is_err()); // Negative (parsed as invalid)
        assert!(Tenor::parse("3MM").is_err()); // Multiple unit chars
    }

    #[test]
    fn test_to_years_simple() {
        assert!((Tenor::parse("1D").expect("valid").to_years_simple() - 1.0 / 365.0).abs() < 1e-10);
        assert!((Tenor::parse("7D").expect("valid").to_years_simple() - 7.0 / 365.0).abs() < 1e-10);
        assert!((Tenor::parse("1W").expect("valid").to_years_simple() - 7.0 / 365.0).abs() < 1e-10);
        assert!((Tenor::parse("1M").expect("valid").to_years_simple() - 1.0 / 12.0).abs() < 1e-10);
        assert!((Tenor::parse("3M").expect("valid").to_years_simple() - 0.25).abs() < 1e-10);
        assert!((Tenor::parse("6M").expect("valid").to_years_simple() - 0.5).abs() < 1e-10);
        assert!((Tenor::parse("1Y").expect("valid").to_years_simple() - 1.0).abs() < 1e-10);
        assert!((Tenor::parse("5Y").expect("valid").to_years_simple() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_add_to_date_months() {
        let start = Date::from_calendar_date(2025, Month::January, 15).expect("valid");
        let tenor = Tenor::parse("1M").expect("valid");

        let end = tenor
            .add_to_date(start, None, BusinessDayConvention::Unadjusted)
            .expect("add");
        assert_eq!(
            end,
            Date::from_calendar_date(2025, Month::February, 15).expect("valid")
        );
    }

    #[test]
    fn test_add_to_date_end_of_month() {
        // Jan 31 + 1M should go to Feb 28 (or 29 in leap year)
        let start = Date::from_calendar_date(2025, Month::January, 31).expect("valid");
        let tenor = Tenor::parse("1M").expect("valid");

        let end = tenor
            .add_to_date(start, None, BusinessDayConvention::Unadjusted)
            .expect("add");
        // 2025 is not a leap year, so Feb has 28 days
        assert_eq!(
            end,
            Date::from_calendar_date(2025, Month::February, 28).expect("valid")
        );
    }

    #[test]
    fn test_add_to_date_years() {
        let start = Date::from_calendar_date(2024, Month::February, 29).expect("valid"); // Leap day
        let tenor = Tenor::parse("1Y").expect("valid");

        let end = tenor
            .add_to_date(start, None, BusinessDayConvention::Unadjusted)
            .expect("add");
        // 2025 is not a leap year, so Feb 29 -> Feb 28
        assert_eq!(
            end,
            Date::from_calendar_date(2025, Month::February, 28).expect("valid")
        );
    }

    #[test]
    fn test_to_years_with_context_act_act() {
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid");
        let tenor = Tenor::parse("1Y").expect("valid");

        let years = tenor
            .to_years_with_context(
                as_of,
                None,
                BusinessDayConvention::Unadjusted,
                DayCount::ActAct,
            )
            .expect("year fraction");

        // 2025 is not a leap year, so 365 days / 365 = 1.0
        assert!((years - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_display() {
        assert_eq!(
            format!("{}", Tenor::parse("3M").expect("valid tenor")),
            "3M"
        );
        assert_eq!(
            format!("{}", Tenor::parse("1Y").expect("valid tenor")),
            "1Y"
        );
        assert_eq!(
            format!("{}", Tenor::parse("7D").expect("valid tenor")),
            "7D"
        );
    }

    #[test]
    fn test_from_str() {
        let tenor: Tenor = "6M".parse().expect("valid");
        assert_eq!(tenor.count, 6);
        assert_eq!(tenor.unit, TenorUnit::Months);
    }

    #[test]
    fn test_convenience_constructors() {
        assert_eq!(Tenor::overnight(), Tenor::new(1, TenorUnit::Days));
        assert_eq!(Tenor::one_week(), Tenor::new(1, TenorUnit::Weeks));
        assert_eq!(Tenor::one_month(), Tenor::new(1, TenorUnit::Months));
        assert_eq!(Tenor::three_months(), Tenor::new(3, TenorUnit::Months));
        assert_eq!(Tenor::six_months(), Tenor::new(6, TenorUnit::Months));
        assert_eq!(Tenor::one_year(), Tenor::new(1, TenorUnit::Years));
    }
}
