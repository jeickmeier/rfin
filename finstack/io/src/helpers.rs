//! Common helper functions for string-based backends (SQLite, Turso).
//!
//! These backends store dates and timestamps as ISO 8601 strings for
//! lexicographic ordering in SQL queries.

use crate::{Error, Result};
use finstack_core::dates::Date;
use time::OffsetDateTime;

/// Convert metadata to JSON string.
///
/// Returns an empty JSON object string if no metadata is provided.
pub(crate) fn meta_json_string(meta: Option<&serde_json::Value>) -> Result<String> {
    match meta {
        Some(v) => Ok(serde_json::to_string(v)?),
        None => Ok("{}".to_string()),
    }
}

/// Convert metadata to optional JSON string (for time-series where null is allowed).
#[cfg(feature = "turso")]
pub(crate) fn meta_json_optional_string(
    meta: Option<&serde_json::Value>,
) -> Result<Option<String>> {
    match meta {
        Some(v) => Ok(Some(serde_json::to_string(v)?)),
        None => Ok(None),
    }
}

/// Format a date as ISO 8601 (YYYY-MM-DD) for use as a database key.
///
/// This format is critical for correct lexicographic ordering in SQL `BETWEEN` queries.
///
/// # Examples
///
/// ```ignore
/// let key = format_date_key(Date::from_calendar_date(2024, 1, 15)?);
/// assert_eq!(key, "2024-01-15");
/// ```
pub(crate) fn format_date_key(date: Date) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        date.month() as u8,
        date.day()
    )
}

/// Parse a date from ISO 8601 (YYYY-MM-DD) format.
///
/// # Errors
///
/// Returns an error if the string is not a valid ISO 8601 date.
pub(crate) fn parse_date_key(s: &str) -> Result<Date> {
    Date::parse(s, &time::format_description::well_known::Iso8601::DATE)
        .map_err(|e| Error::Invariant(format!("Invalid date format in database: {s} ({e})")))
}

/// Format a timestamp as a fixed-width ISO 8601 string for use as a database key.
///
/// Uses format: `YYYY-MM-DDTHH:MM:SS.fffffffffZ` (always 30 characters)
///
/// This fixed-width format is critical for correct lexicographic ordering in SQL queries.
/// Unlike RFC3339, which may omit fractional seconds when they are zero, this format always
/// includes 9 decimal places for nanoseconds, ensuring consistent string width and
/// correct chronological ordering when sorted lexicographically.
///
/// # Examples
///
/// - `2024-01-01T12:00:00.000000000Z` (no fractional seconds)
/// - `2024-01-01T12:00:00.123456789Z` (with nanoseconds)
pub(crate) fn format_timestamp_key(ts: OffsetDateTime) -> Result<String> {
    // Convert to UTC for consistent storage
    let ts_utc = ts.to_offset(time::UtcOffset::UTC);
    Ok(format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:09}Z",
        ts_utc.year(),
        ts_utc.month() as u8,
        ts_utc.day(),
        ts_utc.hour(),
        ts_utc.minute(),
        ts_utc.second(),
        ts_utc.nanosecond(),
    ))
}

/// Parse a timestamp from a fixed-width ISO 8601 string.
///
/// Accepts RFC3339 format for backwards compatibility with existing data.
///
/// This includes the fixed-width variant used by this crate.
///
/// # Errors
///
/// Returns an error if the string is not a valid RFC3339 timestamp.
pub(crate) fn parse_timestamp_key(s: &str) -> Result<OffsetDateTime> {
    use time::format_description::well_known::Rfc3339;
    OffsetDateTime::parse(s, &Rfc3339)
        .map_err(|e| Error::Invariant(format!("Invalid timestamp format in database: {s} ({e})")))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn meta_json_string_with_value() {
        let meta = serde_json::json!({"key": "value"});
        let result = meta_json_string(Some(&meta)).unwrap();
        assert_eq!(result, r#"{"key":"value"}"#);
    }

    #[test]
    fn meta_json_string_without_value() {
        let result = meta_json_string(None).unwrap();
        assert_eq!(result, "{}");
    }

    #[test]
    fn format_and_parse_date_roundtrip() {
        let date = Date::from_calendar_date(2024, Month::January, 15).unwrap();
        let key = format_date_key(date);
        assert_eq!(key, "2024-01-15");

        let parsed = parse_date_key(&key).unwrap();
        assert_eq!(parsed, date);
    }

    #[test]
    fn format_and_parse_timestamp_roundtrip() {
        let ts = OffsetDateTime::from_unix_timestamp(1704110400).unwrap();
        let key = format_timestamp_key(ts).unwrap();
        assert!(key.ends_with('Z'));
        assert_eq!(key.len(), 30); // Fixed width with nanoseconds

        let parsed = parse_timestamp_key(&key).unwrap();
        assert_eq!(parsed.unix_timestamp(), ts.unix_timestamp());
    }

    #[test]
    fn timestamp_preserves_nanoseconds() {
        let base = OffsetDateTime::from_unix_timestamp(1704110400).unwrap();
        let ts = base.replace_nanosecond(123_456_789).unwrap();
        let key = format_timestamp_key(ts).unwrap();
        assert!(key.contains(".123456789Z"));

        let parsed = parse_timestamp_key(&key).unwrap();
        assert_eq!(parsed.nanosecond(), 123_456_789);
    }
}
