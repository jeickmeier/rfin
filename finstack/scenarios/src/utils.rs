//! Utility helpers for converting tenor and period strings.
//!
//! Adapters rely on these parsing helpers to turn human-readable inputs such as
//! `"5Y"` or `"3M"` into normalised numeric representations. The functions
//! return [`Result`](crate::error::Result) so they can bubble up friendly error
//! messages into the higher-level adapters.

use crate::error::{Error, Result};

/// Parse a tenor string to a fractional number of years.
///
/// Supports formats like:
/// - "1D", "7D" → days
/// - "1W", "4W" → weeks
/// - "1M", "6M" → months (30-day approximation)
/// - "1Y", "5Y", "10Y" → years
///
/// # Arguments
/// - `tenor`: Tenor string in the formats listed above. Leading/trailing
///   whitespace is ignored, and input is case-insensitive.
///
/// # Returns
/// Number of years represented by the tenor. For example `"6M"` produces
/// `0.5` and `"1W"` produces roughly `0.01918`.
///
/// # Errors
/// Returns [`Error::InvalidTenor`](crate::error::Error::InvalidTenor) if the
/// string is empty, lacks a unit component, contains a non-numeric value, or
/// specifies an unsupported unit.
///
/// # Examples
/// ```
/// # use finstack_scenarios::utils::parse_tenor_to_years;
/// assert!((parse_tenor_to_years("1Y").unwrap() - 1.0).abs() < 1e-6);
/// assert!((parse_tenor_to_years("6M").unwrap() - 0.5).abs() < 1e-6);
/// assert!((parse_tenor_to_years("1W").unwrap() - (7.0 / 365.0)).abs() < 1e-3);
/// ```
pub fn parse_tenor_to_years(tenor: &str) -> Result<f64> {
    let tenor = tenor.trim().to_uppercase();

    if tenor.is_empty() {
        return Err(Error::InvalidTenor("Empty tenor string".to_string()));
    }

    // Split into number and unit
    let (num_part, unit) = if let Some(pos) = tenor.find(|c: char| c.is_alphabetic()) {
        (&tenor[..pos], &tenor[pos..])
    } else {
        return Err(Error::InvalidTenor(format!("No unit found in: {}", tenor)));
    };

    let value: f64 = num_part
        .trim()
        .parse()
        .map_err(|_| Error::InvalidTenor(format!("Invalid number in tenor: {}", tenor)))?;

    let years = match unit {
        "D" => value / 365.0,
        "W" => value * 7.0 / 365.0,
        "M" => value / 12.0,
        "Y" => value,
        _ => return Err(Error::InvalidTenor(format!("Unknown unit: {}", unit))),
    };

    Ok(years)
}

/// Parse a period string to an integer number of days.
///
/// Supports formats like:
/// - "1D", "7D" → days
/// - "1W" → 7 days
/// - "1M" → 30 days
/// - "1Y" → 365 days
///
/// # Arguments
/// - `period`: Period string matching one of the supported formats.
///
/// # Returns
/// Number of days represented by the period.
///
/// # Errors
/// Returns [`Error::InvalidPeriod`](crate::error::Error::InvalidPeriod) if the
/// string cannot be parsed.
///
/// # Examples
/// ```
/// # use finstack_scenarios::utils::parse_period_to_days;
/// assert_eq!(parse_period_to_days("1D").unwrap(), 1);
/// assert_eq!(parse_period_to_days("1W").unwrap(), 7);
/// assert_eq!(parse_period_to_days("1M").unwrap(), 30);
/// assert_eq!(parse_period_to_days("1Y").unwrap(), 365);
/// ```
pub fn parse_period_to_days(period: &str) -> Result<i64> {
    let period = period.trim().to_uppercase();

    if period.is_empty() {
        return Err(Error::InvalidPeriod("Empty period string".to_string()));
    }

    // Split into number and unit
    let (num_part, unit) = if let Some(pos) = period.find(|c: char| c.is_alphabetic()) {
        (&period[..pos], &period[pos..])
    } else {
        return Err(Error::InvalidPeriod(format!(
            "No unit found in: {}",
            period
        )));
    };

    let value: i64 = num_part
        .trim()
        .parse()
        .map_err(|_| Error::InvalidPeriod(format!("Invalid number in period: {}", period)))?;

    let days = match unit {
        "D" => value,
        "W" => value * 7,
        "M" => value * 30,
        "Y" => value * 365,
        _ => return Err(Error::InvalidPeriod(format!("Unknown unit: {}", unit))),
    };

    Ok(days)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tenor_years() {
        assert!((parse_tenor_to_years("1Y").expect("valid tenor") - 1.0).abs() < 1e-6);
        assert!((parse_tenor_to_years("5Y").expect("valid tenor") - 5.0).abs() < 1e-6);
        assert!((parse_tenor_to_years("6M").expect("valid tenor") - 0.5).abs() < 1e-6);
        assert!((parse_tenor_to_years("3M").expect("valid tenor") - 0.25).abs() < 1e-6);
    }

    #[test]
    fn test_parse_period_days() {
        assert_eq!(parse_period_to_days("1D").expect("valid period"), 1);
        assert_eq!(parse_period_to_days("7D").expect("valid period"), 7);
        assert_eq!(parse_period_to_days("1W").expect("valid period"), 7);
        assert_eq!(parse_period_to_days("1M").expect("valid period"), 30);
        assert_eq!(parse_period_to_days("1Y").expect("valid period"), 365);
    }

    #[test]
    fn test_invalid_tenor() {
        assert!(parse_tenor_to_years("").is_err());
        assert!(parse_tenor_to_years("XYZ").is_err());
        assert!(parse_tenor_to_years("1X").is_err());
    }
}
