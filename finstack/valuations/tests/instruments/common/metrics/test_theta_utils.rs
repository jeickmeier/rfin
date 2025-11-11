//! Comprehensive tests for theta (time decay) calculation utilities.

use super::super::test_helpers::*;
use finstack_valuations::metrics::theta_utils::*;
use time::Month;

// ============================================================================
// Period Parsing Tests
// ============================================================================

#[test]
fn test_parse_period_days_standard() {
    // Arrange & Act & Assert
    assert_eq!(parse_period_days("1D").unwrap(), 1);
    assert_eq!(parse_period_days("7D").unwrap(), 7);
    assert_eq!(parse_period_days("30D").unwrap(), 30);
}

#[test]
fn test_parse_period_weeks() {
    // Arrange & Act & Assert
    assert_eq!(parse_period_days("1W").unwrap(), 7);
    assert_eq!(parse_period_days("2W").unwrap(), 14);
    assert_eq!(parse_period_days("4W").unwrap(), 28);
}

#[test]
fn test_parse_period_months() {
    // Arrange & Act & Assert
    assert_eq!(parse_period_days("1M").unwrap(), 30);
    assert_eq!(parse_period_days("3M").unwrap(), 90);
    assert_eq!(parse_period_days("6M").unwrap(), 180);
    assert_eq!(parse_period_days("12M").unwrap(), 360);
}

#[test]
fn test_parse_period_years() {
    // Arrange & Act & Assert
    assert_eq!(parse_period_days("1Y").unwrap(), 365);
    assert_eq!(parse_period_days("2Y").unwrap(), 730);
    assert_eq!(parse_period_days("5Y").unwrap(), 1825);
}

#[test]
fn test_parse_period_lowercase() {
    // Arrange & Act & Assert: Should handle lowercase
    assert_eq!(parse_period_days("1d").unwrap(), 1);
    assert_eq!(parse_period_days("1w").unwrap(), 7);
    assert_eq!(parse_period_days("1m").unwrap(), 30);
    assert_eq!(parse_period_days("1y").unwrap(), 365);
}

#[test]
fn test_parse_period_with_whitespace() {
    // Arrange & Act & Assert: Should trim whitespace
    assert_eq!(parse_period_days(" 1D ").unwrap(), 1);
    assert_eq!(parse_period_days(" 3M ").unwrap(), 90);
    assert_eq!(parse_period_days("  1Y  ").unwrap(), 365);
}

#[test]
fn test_parse_period_invalid_format() {
    // Arrange & Act & Assert: Invalid formats should error
    assert!(parse_period_days("").is_err());
    assert!(parse_period_days("1X").is_err());
    assert!(parse_period_days("XYZ").is_err());
    assert!(parse_period_days("D").is_err());
    assert!(parse_period_days("1").is_err());
    assert!(parse_period_days("abc").is_err());
}

#[test]
fn test_parse_period_edge_cases() {
    // Arrange & Act & Assert
    assert_eq!(parse_period_days("0D").unwrap(), 0);
    assert_eq!(parse_period_days("100D").unwrap(), 100);
    assert_eq!(parse_period_days("10Y").unwrap(), 3650);
}

// ============================================================================
// Theta Date Calculation Tests
// ============================================================================

#[test]
fn test_calculate_theta_date_no_expiry() {
    // Arrange
    let base = test_date();

    // Act: Roll forward 1 day
    let rolled = calculate_theta_date(base, "1D", None).unwrap();

    // Assert
    let expected = finstack_core::dates::Date::from_calendar_date(2025, Month::January, 2).unwrap();
    assert_eq!(rolled, expected);
}

#[test]
fn test_calculate_theta_date_one_week() {
    // Arrange
    let base = test_date();

    // Act
    let rolled = calculate_theta_date(base, "1W", None).unwrap();

    // Assert
    let expected = finstack_core::dates::Date::from_calendar_date(2025, Month::January, 8).unwrap();
    assert_eq!(rolled, expected);
}

#[test]
fn test_calculate_theta_date_one_month() {
    // Arrange
    let base = test_date();

    // Act
    let rolled = calculate_theta_date(base, "1M", None).unwrap();

    // Assert
    let expected =
        finstack_core::dates::Date::from_calendar_date(2025, Month::January, 31).unwrap();
    assert_eq!(rolled, expected);
}

#[test]
fn test_calculate_theta_date_with_expiry_cap() {
    // Arrange
    let base = test_date();
    let expiry = finstack_core::dates::Date::from_calendar_date(2025, Month::January, 5).unwrap();

    // Act: Rolling 1 week would go past expiry, should cap
    let rolled = calculate_theta_date(base, "1W", Some(expiry)).unwrap();

    // Assert: Should be capped at expiry
    assert_eq!(rolled, expiry);
}

#[test]
fn test_calculate_theta_date_before_expiry() {
    // Arrange
    let base = test_date();
    let expiry = finstack_core::dates::Date::from_calendar_date(2025, Month::February, 1).unwrap();

    // Act: Rolling 1 day is well before expiry
    let rolled = calculate_theta_date(base, "1D", Some(expiry)).unwrap();

    // Assert: Should not be capped
    let expected = finstack_core::dates::Date::from_calendar_date(2025, Month::January, 2).unwrap();
    assert_eq!(rolled, expected);
}

#[test]
fn test_calculate_theta_date_exactly_at_expiry() {
    // Arrange
    let base = test_date();
    let expiry = finstack_core::dates::Date::from_calendar_date(2025, Month::January, 31).unwrap();

    // Act: Rolling exactly to expiry (30 days)
    let rolled = calculate_theta_date(base, "30D", Some(expiry)).unwrap();

    // Assert
    assert_eq!(rolled, expiry);
}

#[test]
fn test_calculate_theta_date_already_past_expiry() {
    // Arrange: Base date after expiry (unusual but should handle)
    let base = finstack_core::dates::Date::from_calendar_date(2025, Month::February, 1).unwrap();
    let expiry = test_date(); // Expiry in the past

    // Act: Should cap at expiry
    let rolled = calculate_theta_date(base, "1D", Some(expiry)).unwrap();

    // Assert: Should still return expiry (the cap)
    assert_eq!(rolled, expiry);
}

#[test]
fn test_calculate_theta_date_various_periods() {
    // Arrange
    let base = test_date();

    // Act & Assert: 3 months
    let rolled_3m = calculate_theta_date(base, "3M", None).unwrap();
    let expected_3m = base + time::Duration::days(90);
    assert_eq!(rolled_3m, expected_3m);

    // Act & Assert: 1 year
    let rolled_1y = calculate_theta_date(base, "1Y", None).unwrap();
    let expected_1y = base + time::Duration::days(365);
    assert_eq!(rolled_1y, expected_1y);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_theta_workflow_short_dated_option() {
    // Arrange: Option with 5 days to expiry
    let base = test_date();
    let expiry = finstack_core::dates::Date::from_calendar_date(2025, Month::January, 6).unwrap();

    // Act: Calculate 1D theta (should work)
    let theta_date_1d = calculate_theta_date(base, "1D", Some(expiry)).unwrap();
    assert_eq!(
        theta_date_1d,
        finstack_core::dates::Date::from_calendar_date(2025, Month::January, 2).unwrap()
    );

    // Act: Calculate 1W theta (should cap at expiry)
    let theta_date_1w = calculate_theta_date(base, "1W", Some(expiry)).unwrap();
    assert_eq!(theta_date_1w, expiry);
}

#[test]
fn test_theta_workflow_long_dated_option() {
    // Arrange: Option with 1 year to expiry
    let base = test_date();
    let expiry = finstack_core::dates::Date::from_calendar_date(2026, Month::January, 1).unwrap();

    // Act: Various theta periods
    let theta_1d = calculate_theta_date(base, "1D", Some(expiry)).unwrap();
    let theta_1w = calculate_theta_date(base, "1W", Some(expiry)).unwrap();
    let theta_1m = calculate_theta_date(base, "1M", Some(expiry)).unwrap();

    // Assert: All should be before expiry
    assert!(theta_1d < expiry);
    assert!(theta_1w < expiry);
    assert!(theta_1m < expiry);

    // Assert: Ordering
    assert!(theta_1d < theta_1w);
    assert!(theta_1w < theta_1m);
}

#[test]
fn test_period_parsing_robustness() {
    // Arrange & Act & Assert: Various valid formats
    assert_eq!(
        parse_period_days("1D").unwrap(),
        parse_period_days("1d").unwrap()
    );
    assert_eq!(
        parse_period_days("1W").unwrap(),
        parse_period_days("1w").unwrap()
    );
    assert_eq!(
        parse_period_days("1M").unwrap(),
        parse_period_days("1m").unwrap()
    );
    assert_eq!(
        parse_period_days("1Y").unwrap(),
        parse_period_days("1y").unwrap()
    );

    // Whitespace handling
    assert_eq!(
        parse_period_days("1D").unwrap(),
        parse_period_days(" 1D ").unwrap()
    );
}
