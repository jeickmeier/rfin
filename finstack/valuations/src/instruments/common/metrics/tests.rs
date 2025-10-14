//! Tests for common metrics utilities

#[cfg(test)]
mod theta_utils_tests {
    use super::super::theta_utils::*;

    #[test]
    fn test_parse_period_days() {
        assert_eq!(parse_period_days("1D").unwrap(), 1);
        assert_eq!(parse_period_days("7D").unwrap(), 7);
        assert_eq!(parse_period_days("1W").unwrap(), 7);
        assert_eq!(parse_period_days("2W").unwrap(), 14);
        assert_eq!(parse_period_days("1M").unwrap(), 30);
        assert_eq!(parse_period_days("3M").unwrap(), 90);
        assert_eq!(parse_period_days("6M").unwrap(), 180);
        assert_eq!(parse_period_days("1Y").unwrap(), 365);
        assert_eq!(parse_period_days("2Y").unwrap(), 730);
    }

    #[test]
    fn test_parse_period_lowercase() {
        assert_eq!(parse_period_days("1d").unwrap(), 1);
        assert_eq!(parse_period_days("1w").unwrap(), 7);
        assert_eq!(parse_period_days("1m").unwrap(), 30);
        assert_eq!(parse_period_days("1y").unwrap(), 365);
    }

    #[test]
    fn test_parse_period_with_whitespace() {
        assert_eq!(parse_period_days(" 1D ").unwrap(), 1);
        assert_eq!(parse_period_days(" 3M ").unwrap(), 90);
    }

    #[test]
    fn test_parse_period_invalid() {
        assert!(parse_period_days("").is_err());
        assert!(parse_period_days("1X").is_err());
        assert!(parse_period_days("XYZ").is_err());
        assert!(parse_period_days("D").is_err());
    }

    #[test]
    fn test_calculate_theta_date_no_expiry() {
        use finstack_core::dates::Date;
        use time::Month;

        let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let rolled = calculate_theta_date(base, "1D", None).unwrap();
        assert_eq!(
            rolled,
            Date::from_calendar_date(2025, Month::January, 2).unwrap()
        );

        let rolled_week = calculate_theta_date(base, "1W", None).unwrap();
        assert_eq!(
            rolled_week,
            Date::from_calendar_date(2025, Month::January, 8).unwrap()
        );
    }

    #[test]
    fn test_calculate_theta_date_with_expiry_cap() {
        use finstack_core::dates::Date;
        use time::Month;

        let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let expiry = Date::from_calendar_date(2025, Month::January, 5).unwrap();

        // Rolling 1 week would go past expiry, should cap at expiry
        let rolled = calculate_theta_date(base, "1W", Some(expiry)).unwrap();
        assert_eq!(rolled, expiry);

        // Rolling 1 day is before expiry, should not cap
        let rolled_day = calculate_theta_date(base, "1D", Some(expiry)).unwrap();
        assert_eq!(
            rolled_day,
            Date::from_calendar_date(2025, Month::January, 2).unwrap()
        );
    }
}
