//! Period system: `Period`, `PeriodId`, `PeriodKey`, `PeriodPlan`, and range parser.
//!
//! Supports quarterly, monthly, weekly, semi-annual and annual identifiers
//! (e.g., "2025Q1", "2025M03", "2025W05", "2025H2", "2025") and
//! range expressions like "2025Q1..Q2" (relative end within the same year) or
//! "2024Q4..2025Q2" (absolute). Tracks actual vs forecast flags per period.

use crate::dates::utils::add_months;
use crate::dates::{Date, Frequency};
use core::fmt;
use core::str::FromStr;
use time::Month;

// Map schedule frequency to period categories we support in this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum PeriodKind {
    Quarterly,
    Monthly,
    Weekly,
    SemiAnnual,
    Annual,
}

#[inline]
fn period_kind_from_frequency(freq: Frequency) -> crate::Result<PeriodKind> {
    if let Some(m) = freq.months() {
        return match m {
            1 => Ok(PeriodKind::Monthly),
            3 => Ok(PeriodKind::Quarterly),
            6 => Ok(PeriodKind::SemiAnnual),
            12 => Ok(PeriodKind::Annual),
            _ => Err(crate::error::InputError::Invalid.into()),
        };
    }
    if let Some(d) = freq.days() {
        return match d {
            7 => Ok(PeriodKind::Weekly),
            _ => Err(crate::error::InputError::Invalid.into()),
        };
    }
    Err(crate::error::InputError::Invalid.into())
}

// Removed arbitrary frequency sort key; ordering across mixed frequencies now
// uses concrete calendar bounds (start, then end) with a deterministic tie-breaker.

/// Identifier for a period like 2025Q1 or 2025M03.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PeriodId {
    /// Gregorian calendar year.
    pub year: i32,
    /// Ordinal index within the year (depends on `freq`).
    /// - Quarter: 1..=4
    /// - Month:   1..=12
    /// - Week:    1..=53 (anchored at Jan-01 in 7-day blocks, differs from ISO 8601 week numbering)
    /// - Half:    1..=2
    /// - Annual:  1
    pub index: u8,
    /// Frequency of the period.
    pub freq: Frequency,
}

impl PeriodId {
    /// Build a quarterly identifier.
    pub fn quarter(year: i32, q: u8) -> Self {
        Self {
            year,
            index: q,
            freq: Frequency::quarterly(),
        }
    }
    /// Build a monthly identifier.
    pub fn month(year: i32, m: u8) -> Self {
        Self {
            year,
            index: m,
            freq: Frequency::monthly(),
        }
    }
    /// Build a weekly identifier.
    pub fn week(year: i32, w: u8) -> Self {
        Self {
            year,
            index: w,
            freq: Frequency::weekly(),
        }
    }
    /// Build a semi-annual identifier.
    pub fn half(year: i32, h: u8) -> Self {
        Self {
            year,
            index: h,
            freq: Frequency::semi_annual(),
        }
    }
    /// Build an annual identifier.
    pub fn annual(year: i32) -> Self {
        Self {
            year,
            index: 1,
            freq: Frequency::annual(),
        }
    }
}

/// Key usable for maps; currently identical to `PeriodId` but kept for future extension.
pub type PeriodKey = PeriodId;

/// Configuration for fiscal year periods.
#[derive(Debug, Clone, Copy)]
pub struct FiscalConfig {
    /// The month when the fiscal year starts (1-12).
    pub start_month: u8,
    /// The day of the month when the fiscal year starts (1-31).
    pub start_day: u8,
}

impl FiscalConfig {
    /// Create a new fiscal configuration.
    pub fn new(start_month: u8, start_day: u8) -> crate::Result<Self> {
        if !(1..=12).contains(&start_month) {
            return Err(crate::error::InputError::Invalid.into());
        }
        if !(1..=31).contains(&start_day) {
            return Err(crate::error::InputError::Invalid.into());
        }
        Ok(Self {
            start_month,
            start_day,
        })
    }

    /// Standard calendar year (January 1).
    pub fn calendar_year() -> Self {
        Self {
            start_month: 1,
            start_day: 1,
        }
    }

    /// US Federal fiscal year (October 1).
    pub fn us_federal() -> Self {
        Self {
            start_month: 10,
            start_day: 1,
        }
    }

    /// UK fiscal year (April 6).
    pub fn uk() -> Self {
        Self {
            start_month: 4,
            start_day: 6,
        }
    }

    /// Japanese fiscal year (April 1).
    pub fn japan() -> Self {
        Self {
            start_month: 4,
            start_day: 1,
        }
    }
}

/// A concrete period with start/end dates and actual/forecast flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Period {
    /// Identifier of this period.
    pub id: PeriodId,
    /// Inclusive start date.
    pub start: Date,
    /// Exclusive end date.
    pub end: Date,
    /// True when this period is part of the "actuals" subset.
    pub is_actual: bool,
}

/// Builder/plan for a contiguous sequence of periods and their actual/forecast split.
#[derive(Debug, Clone)]
pub struct PeriodPlan {
    pub periods: Vec<Period>,
}

impl PeriodPlan {
    pub fn iter(&self) -> impl Iterator<Item = &Period> {
        self.periods.iter()
    }
}

/// Build periods from a range expression (e.g., "2025Q1..Q4" or "2024Q4..2025Q2").
/// If `actuals_until` is Some(id string), periods <= that id are marked actual, rest forecast.
pub fn build_periods(range: &str, actuals_until: Option<&str>) -> crate::Result<PeriodPlan> {
    let (start, end) = parse_range(range)?;
    let mut ids = enumerate_ids(start, end)?;

    let actual_cut = actuals_until.map(parse_id).transpose()?;
    let periods = ids
        .drain(..)
        .map(|pid| make_period(pid, actual_cut.as_ref()))
        .collect::<crate::Result<Vec<_>>>()?;
    Ok(PeriodPlan { periods })
}

/// Build fiscal periods from a range expression with custom fiscal year configuration.
/// The period IDs (e.g., "FY2025Q1") refer to fiscal periods, not calendar periods.
pub fn build_fiscal_periods(
    range: &str,
    fiscal_config: FiscalConfig,
    actuals_until: Option<&str>,
) -> crate::Result<PeriodPlan> {
    let (start, end) = parse_range(range)?;
    let mut ids = enumerate_ids(start, end)?;

    let actual_cut = actuals_until.map(parse_id).transpose()?;
    let periods = ids
        .drain(..)
        .map(|pid| make_fiscal_period(pid, fiscal_config, actual_cut.as_ref()))
        .collect::<crate::Result<Vec<_>>>()?;
    Ok(PeriodPlan { periods })
}

fn make_period(pid: PeriodId, cut: Option<&PeriodId>) -> crate::Result<Period> {
    let (start, end) = match period_kind_from_frequency(pid.freq)? {
        PeriodKind::Quarterly => quarter_bounds(pid.year, pid.index),
        PeriodKind::Monthly => month_bounds(pid.year, pid.index),
        PeriodKind::Weekly => week_bounds(pid.year, pid.index),
        PeriodKind::SemiAnnual => half_bounds(pid.year, pid.index),
        PeriodKind::Annual => annual_bounds(pid.year),
    };
    let is_actual = cut.map(|c| pid <= *c).unwrap_or(false);
    Ok(Period {
        id: pid,
        start,
        end,
        is_actual,
    })
}

fn make_fiscal_period(
    pid: PeriodId,
    config: FiscalConfig,
    cut: Option<&PeriodId>,
) -> crate::Result<Period> {
    let (start, end) = match period_kind_from_frequency(pid.freq)? {
        PeriodKind::Quarterly => fiscal_quarter_bounds(pid.year, pid.index, config),
        PeriodKind::Monthly => fiscal_month_bounds(pid.year, pid.index, config),
        PeriodKind::Weekly => fiscal_week_bounds(pid.year, pid.index, config),
        PeriodKind::SemiAnnual => fiscal_half_bounds(pid.year, pid.index, config),
        PeriodKind::Annual => fiscal_annual_bounds(pid.year, config),
    };
    let is_actual = cut.map(|c| pid <= *c).unwrap_or(false);
    Ok(Period {
        id: pid,
        start,
        end,
        is_actual,
    })
}

fn quarter_bounds(year: i32, q: u8) -> (Date, Date) {
    let (sm, em) = match q {
        1 => (Month::January, Month::April),
        2 => (Month::April, Month::July),
        3 => (Month::July, Month::October),
        _ => (Month::October, Month::January),
    };
    let start = Date::from_calendar_date(year, sm, 1).unwrap();
    let end_year = if q == 4 { year + 1 } else { year };
    let end = Date::from_calendar_date(end_year, em, 1).unwrap();
    (start, end)
}

fn month_bounds(year: i32, m: u8) -> (Date, Date) {
    let sm = Month::try_from(m).unwrap();
    let start = Date::from_calendar_date(year, sm, 1).unwrap();
    let (ey, em) = if m == 12 {
        (year + 1, Month::January)
    } else {
        (year, Month::try_from(m + 1).unwrap())
    };
    let end = Date::from_calendar_date(ey, em, 1).unwrap();
    (start, end)
}

/// Calculate week bounds using simple Jan-01 anchoring.
///
/// This differs from ISO 8601 week numbering which uses Monday as the first day
/// and may include days from the previous/next year. This implementation simply
/// divides the year into 7-day blocks starting from January 1st, regardless of
/// which day of the week that falls on.
fn week_bounds(year: i32, w: u8) -> (Date, Date) {
    use time::Duration;
    let start_of_year = Date::from_calendar_date(year, Month::January, 1).unwrap();
    let start = start_of_year + Duration::days(((w - 1) as i64) * 7);
    let end = start + Duration::days(7);
    (start, end)
}

fn half_bounds(year: i32, h: u8) -> (Date, Date) {
    match h {
        1 => (
            Date::from_calendar_date(year, Month::January, 1).unwrap(),
            Date::from_calendar_date(year, Month::July, 1).unwrap(),
        ),
        _ => (
            Date::from_calendar_date(year, Month::July, 1).unwrap(),
            Date::from_calendar_date(year + 1, Month::January, 1).unwrap(),
        ),
    }
}

fn annual_bounds(year: i32) -> (Date, Date) {
    (
        Date::from_calendar_date(year, Month::January, 1).unwrap(),
        Date::from_calendar_date(year + 1, Month::January, 1).unwrap(),
    )
}

// Fiscal year bounds functions

fn fiscal_quarter_bounds(fiscal_year: i32, q: u8, config: FiscalConfig) -> (Date, Date) {
    // Calculate the start of the fiscal year
    let fy_start = fiscal_year_start(fiscal_year, config);

    // Each quarter is 3 months
    let quarter_start_month_offset = (q - 1) * 3;
    let quarter_end_month_offset = q * 3;

    // Calculate start and end dates for the quarter
    let start = add_months(fy_start, quarter_start_month_offset as i32);
    let end = add_months(fy_start, quarter_end_month_offset as i32);

    (start, end)
}

fn fiscal_month_bounds(fiscal_year: i32, m: u8, config: FiscalConfig) -> (Date, Date) {
    // Calculate the start of the fiscal year
    let fy_start = fiscal_year_start(fiscal_year, config);

    // Calculate start and end dates for the month
    let start = add_months(fy_start, (m - 1) as i32);
    let end = add_months(fy_start, m as i32);

    (start, end)
}

/// Calculate fiscal week bounds using simple fiscal year start anchoring.
///
/// Like regular week_bounds, this uses simple 7-day blocks starting from the
/// fiscal year start date, not ISO 8601 week numbering.
fn fiscal_week_bounds(fiscal_year: i32, w: u8, config: FiscalConfig) -> (Date, Date) {
    use time::Duration;

    // Calculate the start of the fiscal year
    let fy_start = fiscal_year_start(fiscal_year, config);

    // Calculate start and end dates for the week
    let start = fy_start + Duration::days(((w - 1) as i64) * 7);
    let end = start + Duration::days(7);

    (start, end)
}

fn fiscal_half_bounds(fiscal_year: i32, h: u8, config: FiscalConfig) -> (Date, Date) {
    // Calculate the start of the fiscal year
    let fy_start = fiscal_year_start(fiscal_year, config);

    // Each half is 6 months
    let half_start_month_offset = (h - 1) * 6;
    let half_end_month_offset = h * 6;

    let start = add_months(fy_start, half_start_month_offset as i32);
    let end = add_months(fy_start, half_end_month_offset as i32);

    (start, end)
}

fn fiscal_annual_bounds(fiscal_year: i32, config: FiscalConfig) -> (Date, Date) {
    let start = fiscal_year_start(fiscal_year, config);
    let end = fiscal_year_start(fiscal_year + 1, config);
    (start, end)
}

/// Calculate the start date of a fiscal year
fn fiscal_year_start(fiscal_year: i32, config: FiscalConfig) -> Date {
    // For fiscal years that start in months other than January,
    // we need to determine the correct calendar year
    let calendar_year = if config.start_month == 1 {
        fiscal_year
    } else {
        // Fiscal year starts in the previous calendar year
        // E.g., FY2025 starting Oct 1 begins on Oct 1, 2024
        // E.g., FY2025 starting Apr 1 begins on Apr 1, 2024
        fiscal_year - 1
    };

    let month = Month::try_from(config.start_month).unwrap();
    Date::from_calendar_date(calendar_year, month, config.start_day).unwrap_or_else(|_| {
        // If the day doesn't exist (e.g., Feb 30), use the last day of the month
        let last_day = days_in_month(calendar_year, config.start_month);
        Date::from_calendar_date(calendar_year, month, last_day).unwrap()
    })
}

/// Get the number of days in a month
pub fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            // Check for leap year
            if crate::dates::utils::is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => panic!("Invalid month: {}", month),
    }
}

fn parse_range(s: &str) -> crate::Result<(PeriodId, PeriodId)> {
    let parts: Vec<&str> = s.split("..").collect();
    if parts.len() != 2 {
        return Err(crate::error::InputError::Invalid.into());
    }
    let start = parse_id(parts[0])?;
    let rhs = parts[1].trim();
    // Relative if RHS starts with a letter (Q/M/W/H/A). Absolute if it starts with a digit (YYYY...).
    let end = if rhs
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        parse_id(rhs)?
    } else {
        // relative form like "..Q4" / "..M12" / "..W52" / "..H2" / "..A"
        match period_kind_from_frequency(start.freq)? {
            PeriodKind::Quarterly => PeriodId {
                year: start.year,
                index: rhs
                    .trim_start_matches('Q')
                    .parse()
                    .map_err(|_| crate::error::InputError::Invalid)?,
                freq: Frequency::quarterly(),
            },
            PeriodKind::Monthly => PeriodId {
                year: start.year,
                index: rhs
                    .trim_start_matches('M')
                    .parse()
                    .map_err(|_| crate::error::InputError::Invalid)?,
                freq: Frequency::monthly(),
            },
            PeriodKind::Weekly => PeriodId {
                year: start.year,
                index: rhs
                    .trim_start_matches('W')
                    .parse()
                    .map_err(|_| crate::error::InputError::Invalid)?,
                freq: Frequency::weekly(),
            },
            PeriodKind::SemiAnnual => PeriodId {
                year: start.year,
                index: rhs
                    .trim_start_matches('H')
                    .parse()
                    .map_err(|_| crate::error::InputError::Invalid)?,
                freq: Frequency::semi_annual(),
            },
            PeriodKind::Annual => PeriodId {
                year: start.year,
                index: 1,
                freq: Frequency::annual(),
            },
        }
    };
    // Validate period kind consistency and non-inverted ranges
    if period_kind_from_frequency(start.freq)? != period_kind_from_frequency(end.freq)? {
        return Err(crate::error::InputError::Invalid.into());
    }
    if start > end {
        return Err(crate::error::InputError::InvalidDateRange.into());
    }
    Ok((start, end))
}

fn parse_id(s: &str) -> crate::Result<PeriodId> {
    let s = s.trim();
    if let Some(i) = s.find('Q') {
        // quarterly
        let year: i32 = s[..i]
            .parse()
            .map_err(|_| crate::error::InputError::Invalid)?;
        let q: u8 = s[i + 1..]
            .parse()
            .map_err(|_| crate::error::InputError::Invalid)?;
        if !(1..=4).contains(&q) {
            return Err(crate::error::InputError::Invalid.into());
        }
        return Ok(PeriodId::quarter(year, q));
    }
    if let Some(i) = s.find('M') {
        // monthly
        let year: i32 = s[..i]
            .parse()
            .map_err(|_| crate::error::InputError::Invalid)?;
        let m: u8 = s[i + 1..]
            .parse()
            .map_err(|_| crate::error::InputError::Invalid)?;
        if !(1..=12).contains(&m) {
            return Err(crate::error::InputError::Invalid.into());
        }
        return Ok(PeriodId::month(year, m));
    }
    if let Some(i) = s.find('W') {
        // weekly
        let year: i32 = s[..i]
            .parse()
            .map_err(|_| crate::error::InputError::Invalid)?;
        let w: u8 = s[i + 1..]
            .parse()
            .map_err(|_| crate::error::InputError::Invalid)?;
        if !(1..=53).contains(&w) {
            return Err(crate::error::InputError::Invalid.into());
        }
        return Ok(PeriodId::week(year, w));
    }
    if let Some(i) = s.find('H') {
        // half-year
        let year: i32 = s[..i]
            .parse()
            .map_err(|_| crate::error::InputError::Invalid)?;
        let h: u8 = s[i + 1..]
            .parse()
            .map_err(|_| crate::error::InputError::Invalid)?;
        if !(1..=2).contains(&h) {
            return Err(crate::error::InputError::Invalid.into());
        }
        return Ok(PeriodId::half(year, h));
    }
    if s.chars().all(|c| c.is_ascii_digit()) {
        // annual
        let year: i32 = s.parse().map_err(|_| crate::error::InputError::Invalid)?;
        return Ok(PeriodId::annual(year));
    }
    Err(crate::error::InputError::Invalid.into())
}

fn enumerate_ids(mut cur: PeriodId, end: PeriodId) -> crate::Result<Vec<PeriodId>> {
    let mut out = Vec::new();
    while cur <= end {
        out.push(cur);
        cur = step(cur)?;
    }
    Ok(out)
}

fn step(mut id: PeriodId) -> crate::Result<PeriodId> {
    match period_kind_from_frequency(id.freq)? {
        PeriodKind::Quarterly => {
            if id.index == 4 {
                id.year += 1;
                id.index = 1;
            } else {
                id.index += 1;
            }
        }
        PeriodKind::Monthly => {
            if id.index == 12 {
                id.year += 1;
                id.index = 1;
            } else {
                id.index += 1;
            }
        }
        PeriodKind::Weekly => {
            if id.index == 53 {
                id.year += 1;
                id.index = 1;
            } else {
                id.index += 1;
            }
        }
        PeriodKind::SemiAnnual => {
            if id.index == 2 {
                id.year += 1;
                id.index = 1;
            } else {
                id.index += 1;
            }
        }
        PeriodKind::Annual => {
            id.year += 1;
            id.index = 1;
        }
    }
    Ok(id)
}

#[inline]
fn bounds_for_id(pid: &PeriodId) -> crate::Result<(Date, Date)> {
    match period_kind_from_frequency(pid.freq)? {
        PeriodKind::Quarterly => Ok(quarter_bounds(pid.year, pid.index)),
        PeriodKind::Monthly => Ok(month_bounds(pid.year, pid.index)),
        PeriodKind::Weekly => Ok(week_bounds(pid.year, pid.index)),
        PeriodKind::SemiAnnual => Ok(half_bounds(pid.year, pid.index)),
        PeriodKind::Annual => Ok(annual_bounds(pid.year)),
    }
}

// Ordering helpers for PeriodId
impl PartialOrd for PeriodId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for PeriodId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // First compare by year for a fast path across different calendar years.
        if self.year != other.year {
            return self.year.cmp(&other.year);
        }

        let self_kind = period_kind_from_frequency(self.freq).unwrap_or(PeriodKind::Quarterly);
        let other_kind = period_kind_from_frequency(other.freq).unwrap_or(PeriodKind::Quarterly);

        // Within the same frequency kind and year, order by index.
        if self_kind == other_kind {
            return self.index.cmp(&other.index);
        }

        // Mixed frequencies in the same year: order by actual calendar span
        // (start date, then end date). This avoids arbitrary cross-kind ordering.
        let (self_start, self_end) = bounds_for_id(self).unwrap_or_else(|_| {
            // Fallback to year boundaries if frequency is unsupported
            (
                Date::from_calendar_date(self.year, time::Month::January, 1).unwrap(),
                Date::from_calendar_date(self.year + 1, time::Month::January, 1).unwrap(),
            )
        });
        let (other_start, other_end) = bounds_for_id(other).unwrap_or_else(|_| {
            // Fallback to year boundaries if frequency is unsupported
            (
                Date::from_calendar_date(other.year, time::Month::January, 1).unwrap(),
                Date::from_calendar_date(other.year + 1, time::Month::January, 1).unwrap(),
            )
        });

        let by_start = self_start.cmp(&other_start);
        if by_start != std::cmp::Ordering::Equal {
            return by_start;
        }
        let by_end = self_end.cmp(&other_end);
        if by_end != std::cmp::Ordering::Equal {
            return by_end;
        }

        // Deterministic tie-breaker (should be extremely rare): stable kind then index.
        let by_kind = self_kind.cmp(&other_kind);
        if by_kind != std::cmp::Ordering::Equal {
            return by_kind;
        }
        self.index.cmp(&other.index)
    }
}

impl fmt::Display for PeriodId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match period_kind_from_frequency(self.freq).unwrap_or(PeriodKind::Quarterly) {
            PeriodKind::Quarterly => write!(f, "{}Q{}", self.year, self.index),
            PeriodKind::Monthly => write!(f, "{}M{:02}", self.year, self.index),
            PeriodKind::Weekly => write!(f, "{}W{:02}", self.year, self.index),
            PeriodKind::SemiAnnual => write!(f, "{}H{}", self.year, self.index),
            PeriodKind::Annual => write!(f, "{}", self.year),
        }
    }
}

impl FromStr for PeriodId {
    type Err = crate::error::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_id(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_and_enumerate_quarters() {
        let plan = build_periods("2025Q1..Q3", Some("2025Q2")).unwrap();
        assert_eq!(plan.periods.len(), 3);
        assert!(plan.periods[0].is_actual);
        assert!(plan.periods[1].is_actual);
        assert!(!plan.periods[2].is_actual);
        assert_eq!(
            plan.periods[0].start,
            Date::from_calendar_date(2025, Month::January, 1).unwrap()
        );
        assert_eq!(
            plan.periods[2].end,
            Date::from_calendar_date(2025, Month::October, 1).unwrap()
        );
    }

    #[test]
    fn parse_and_enumerate_months_across_year() {
        let plan = build_periods("2024M11..2025M02", None).unwrap();
        assert_eq!(plan.periods.len(), 4);
        assert_eq!(
            plan.periods[0].start,
            Date::from_calendar_date(2024, Month::November, 1).unwrap()
        );
        assert_eq!(
            plan.periods[3].end,
            Date::from_calendar_date(2025, Month::March, 1).unwrap()
        );
    }

    #[test]
    fn parse_and_enumerate_weekly() {
        let plan = build_periods("2025W01..W04", None).unwrap();
        assert_eq!(plan.periods.len(), 4);
        assert_eq!(
            plan.periods[0].start,
            Date::from_calendar_date(2025, Month::January, 1).unwrap()
        );
    }

    #[test]
    fn parse_and_enumerate_half_and_annual() {
        let h = build_periods("2025H1..H2", Some("2025H1")).unwrap();
        assert_eq!(h.periods.len(), 2);
        assert!(h.periods[0].is_actual);
        assert!(!h.periods[1].is_actual);
        let y = build_periods("2024..2026", None).unwrap();
        assert_eq!(y.periods.len(), 3);
        assert_eq!(
            y.periods[0].start,
            Date::from_calendar_date(2024, Month::January, 1).unwrap()
        );
        assert_eq!(
            y.periods[2].end,
            Date::from_calendar_date(2027, Month::January, 1).unwrap()
        );
    }

    #[test]
    fn test_fiscal_quarters_us_federal() {
        // US Federal fiscal year starts October 1
        let config = FiscalConfig::us_federal();
        let plan = build_fiscal_periods("2025Q1..Q4", config, Some("2025Q2")).unwrap();

        assert_eq!(plan.periods.len(), 4);
        assert!(plan.periods[0].is_actual);
        assert!(plan.periods[1].is_actual);
        assert!(!plan.periods[2].is_actual);
        assert!(!plan.periods[3].is_actual);

        // FY2025Q1 starts October 1, 2024
        assert_eq!(
            plan.periods[0].start,
            Date::from_calendar_date(2024, Month::October, 1).unwrap()
        );
        assert_eq!(
            plan.periods[0].end,
            Date::from_calendar_date(2025, Month::January, 1).unwrap()
        );

        // FY2025Q2 is January-March 2025
        assert_eq!(
            plan.periods[1].start,
            Date::from_calendar_date(2025, Month::January, 1).unwrap()
        );
        assert_eq!(
            plan.periods[1].end,
            Date::from_calendar_date(2025, Month::April, 1).unwrap()
        );

        // FY2025Q4 ends September 30, 2025 (October 1, 2025)
        assert_eq!(
            plan.periods[3].end,
            Date::from_calendar_date(2025, Month::October, 1).unwrap()
        );
    }

    #[test]
    fn test_fiscal_annual_japan() {
        // Japanese fiscal year starts April 1
        let config = FiscalConfig::japan();
        let plan = build_fiscal_periods("2025..2026", config, None).unwrap();

        assert_eq!(plan.periods.len(), 2);

        // FY2025 starts April 1, 2024
        assert_eq!(
            plan.periods[0].start,
            Date::from_calendar_date(2024, Month::April, 1).unwrap()
        );
        assert_eq!(
            plan.periods[0].end,
            Date::from_calendar_date(2025, Month::April, 1).unwrap()
        );

        // FY2026 starts April 1, 2025
        assert_eq!(
            plan.periods[1].start,
            Date::from_calendar_date(2025, Month::April, 1).unwrap()
        );
        assert_eq!(
            plan.periods[1].end,
            Date::from_calendar_date(2026, Month::April, 1).unwrap()
        );
    }

    #[test]
    fn test_fiscal_config_custom() {
        // Custom fiscal year starting July 1
        let config = FiscalConfig::new(7, 1).unwrap();
        let plan = build_fiscal_periods("2025H1..H2", config, None).unwrap();

        assert_eq!(plan.periods.len(), 2);

        // FY2025H1 is July-December 2024
        assert_eq!(
            plan.periods[0].start,
            Date::from_calendar_date(2024, Month::July, 1).unwrap()
        );
        assert_eq!(
            plan.periods[0].end,
            Date::from_calendar_date(2025, Month::January, 1).unwrap()
        );

        // FY2025H2 is January-June 2025
        assert_eq!(
            plan.periods[1].start,
            Date::from_calendar_date(2025, Month::January, 1).unwrap()
        );
        assert_eq!(
            plan.periods[1].end,
            Date::from_calendar_date(2025, Month::July, 1).unwrap()
        );
    }

    #[test]
    fn test_fiscal_months() {
        let config = FiscalConfig::uk(); // April 6
        let plan = build_fiscal_periods("2025M01..M03", config, None).unwrap();

        assert_eq!(plan.periods.len(), 3);

        // FY2025M01 starts April 6, 2024
        assert_eq!(
            plan.periods[0].start,
            Date::from_calendar_date(2024, Month::April, 6).unwrap()
        );

        // Months should be consecutive
        assert_eq!(plan.periods[0].end, plan.periods[1].start);
        assert_eq!(plan.periods[1].end, plan.periods[2].start);
    }

    #[test]
    fn inverted_range_errors() {
        let err = build_periods("2025Q3..Q2", None).unwrap_err();
        match err {
            crate::error::Error::Input(crate::error::InputError::InvalidDateRange) => {}
            other => panic!("expected InvalidDateRange, got {:?}", other),
        }
    }

    #[test]
    fn contiguity_quarters() {
        let plan = build_periods("2025Q1..Q4", None).unwrap();
        for w in plan.periods.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
    }

    #[test]
    fn contiguity_months() {
        let plan = build_periods("2025M01..M06", None).unwrap();
        for w in plan.periods.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
    }

    #[test]
    fn contiguity_weeks() {
        let plan = build_periods("2025W01..W10", None).unwrap();
        for w in plan.periods.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
    }

    #[test]
    fn contiguity_halves_and_annual() {
        let halves = build_periods("2025H1..H2", None).unwrap();
        for w in halves.periods.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
        let annual = build_periods("2024..2026", None).unwrap();
        for w in annual.periods.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
    }

    #[test]
    fn contiguity_fiscal_quarters_us_federal() {
        let plan = build_fiscal_periods("2025Q1..Q4", FiscalConfig::us_federal(), None).unwrap();
        for w in plan.periods.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
    }

    #[test]
    fn contiguity_fiscal_months_uk() {
        let plan = build_fiscal_periods("2025M01..M06", FiscalConfig::uk(), None).unwrap();
        for w in plan.periods.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
    }
}
