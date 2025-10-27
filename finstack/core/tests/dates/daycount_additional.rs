//! Additional tests for day count conventions

use finstack_core::dates::calendar::TARGET2;
use finstack_core::dates::{Date, DayCount, DayCountCtx, Frequency};
use time::Month;

fn make_date(y: i32, m: u8, d: u8) -> Date {
    Date::from_calendar_date(y, Month::try_from(m).unwrap(), d).unwrap()
}

#[test]
fn act360_year_fraction_360_days() {
    let start = make_date(2025, 1, 1);
    let end = start + time::Duration::days(360);

    let yf = DayCount::Act360
        .year_fraction(start, end, DayCountCtx::default())
        .unwrap();

    assert!((yf - 1.0).abs() < 1e-10);
}

#[test]
fn act360_half_year() {
    let start = make_date(2025, 1, 1);
    let end = start + time::Duration::days(180);

    let yf = DayCount::Act360
        .year_fraction(start, end, DayCountCtx::default())
        .unwrap();

    assert!((yf - 0.5).abs() < 1e-10);
}

#[test]
fn act365f_year_fraction_365_days() {
    let start = make_date(2025, 1, 1);
    let end = start + time::Duration::days(365);

    let yf = DayCount::Act365F
        .year_fraction(start, end, DayCountCtx::default())
        .unwrap();

    assert!((yf - 1.0).abs() < 1e-10);
}

#[test]
fn act365f_leap_year_ignores_extra_day() {
    // 2024 is a leap year with 366 days
    let start = make_date(2024, 1, 1);
    let end = make_date(2025, 1, 1);

    let yf = DayCount::Act365F
        .year_fraction(start, end, DayCountCtx::default())
        .unwrap();

    // Act/365F always uses 365, even in leap years
    let actual_days = (end - start).whole_days() as f64;
    let expected = actual_days / 365.0;
    assert!((yf - expected).abs() < 1e-10);
}

#[test]
fn thirty360_same_day_different_months() {
    let start = make_date(2025, 1, 15);
    let end = make_date(2025, 4, 15);

    let yf = DayCount::Thirty360
        .year_fraction(start, end, DayCountCtx::default())
        .unwrap();

    // 3 months = 90 days in 30/360
    assert!((yf - 90.0 / 360.0).abs() < 1e-10);
}

#[test]
fn thirty360_end_of_month_adjustments() {
    // Test 30/360 handling of month-end dates
    let start = make_date(2025, 1, 31);
    let end = make_date(2025, 2, 28);

    let yf = DayCount::Thirty360
        .year_fraction(start, end, DayCountCtx::default())
        .unwrap();

    // Jan 31 -> 30, Feb 28 -> 28, so (2-1)*30 + (28-30) = 30 - 2 = 28 days
    assert!((yf - 28.0 / 360.0).abs() < 1e-10);
}

#[test]
fn thirtye360_end_of_month() {
    let start = make_date(2025, 1, 31);
    let end = make_date(2025, 3, 31);

    let yf = DayCount::ThirtyE360
        .year_fraction(start, end, DayCountCtx::default())
        .unwrap();

    // Both 31s become 30, so 2 months * 30 = 60 days
    assert!((yf - 60.0 / 360.0).abs() < 1e-10);
}

#[test]
fn actact_same_year() {
    let start = make_date(2025, 1, 1);
    let end = make_date(2025, 7, 1);

    let yf = DayCount::ActAct
        .year_fraction(start, end, DayCountCtx::default())
        .unwrap();

    let days = (end - start).whole_days() as f64;
    let expected = days / 365.0; // 2025 is not a leap year
    assert!((yf - expected).abs() < 1e-10);
}

#[test]
fn actact_spanning_multiple_years() {
    let start = make_date(2024, 7, 1);
    let end = make_date(2026, 7, 1);

    let yf = DayCount::ActAct
        .year_fraction(start, end, DayCountCtx::default())
        .unwrap();

    // Should be close to 2.0 years
    assert!((yf - 2.0).abs() < 0.01);
}

#[test]
fn actact_leap_year_handling() {
    // Span across leap year boundary
    let start = make_date(2024, 1, 1);
    let end = make_date(2025, 1, 1);

    let yf = DayCount::ActAct
        .year_fraction(start, end, DayCountCtx::default())
        .unwrap();

    // 2024 has 366 days
    assert!((yf - 1.0).abs() < 1e-10);
}

#[test]
fn actact_isma_requires_frequency() {
    let start = make_date(2025, 1, 1);
    let end = make_date(2025, 7, 1);

    // Without frequency, should error
    let result = DayCount::ActActIsma.year_fraction(start, end, DayCountCtx::default());
    assert!(result.is_err());

    // With frequency, should work
    let ctx = DayCountCtx {
        calendar: None,
        frequency: Some(Frequency::Months(6)),
        bus_basis: None,
    };
    let yf = DayCount::ActActIsma.year_fraction(start, end, ctx).unwrap();
    assert!(yf > 0.0);
}

#[test]
fn actact_isma_full_coupon_period() {
    let start = make_date(2025, 1, 1);
    let end = make_date(2025, 7, 1);

    let ctx = DayCountCtx {
        calendar: None,
        frequency: Some(Frequency::Months(6)),
        bus_basis: None,
    };

    let yf = DayCount::ActActIsma.year_fraction(start, end, ctx).unwrap();

    // Full semi-annual period should be 1.0 under ISMA
    assert!((yf - 1.0).abs() < 1e-6);
}

#[test]
fn bus252_requires_calendar() {
    let start = make_date(2025, 1, 1);
    let end = make_date(2025, 1, 10);

    // Without calendar, should error
    let result = DayCount::Bus252.year_fraction(start, end, DayCountCtx::default());
    assert!(result.is_err());

    // With calendar, should work
    let calendar = TARGET2;
    let ctx = DayCountCtx {
        calendar: Some(&calendar),
        frequency: None,
        bus_basis: None,
    };
    let yf = DayCount::Bus252.year_fraction(start, end, ctx).unwrap();
    assert!(yf > 0.0);
}

#[test]
fn bus252_counts_only_business_days() {
    let calendar = TARGET2;
    let start = make_date(2025, 1, 2); // Thursday
    let end = make_date(2025, 1, 6); // Monday (includes weekend)

    let ctx = DayCountCtx {
        calendar: Some(&calendar),
        frequency: None,
        bus_basis: None,
    };

    let yf = DayCount::Bus252.year_fraction(start, end, ctx).unwrap();

    // Should count: Thu, Fri (skip Sat, Sun) = 2 business days
    let biz_days = yf * 252.0;
    assert!((biz_days - 2.0).abs() < 0.1);
}

#[test]
fn act365l_without_feb29() {
    let start = make_date(2025, 3, 1);
    let end = make_date(2025, 9, 1);

    let yf = DayCount::Act365L
        .year_fraction(start, end, DayCountCtx::default())
        .unwrap();

    let days = (end - start).whole_days() as f64;
    let expected = days / 365.0; // No Feb 29 in range
    assert!((yf - expected).abs() < 1e-10);
}

#[test]
fn act365l_with_feb29() {
    let start = make_date(2024, 2, 1);
    let end = make_date(2024, 3, 1);

    let yf = DayCount::Act365L
        .year_fraction(start, end, DayCountCtx::default())
        .unwrap();

    let days = (end - start).whole_days() as f64;
    let expected = days / 366.0; // Feb 29 in range
    assert!((yf - expected).abs() < 1e-10);
}

#[test]
fn daycount_equal_dates_returns_zero() {
    let date = make_date(2025, 6, 15);
    let ctx = DayCountCtx::default();

    for dc in [
        DayCount::Act360,
        DayCount::Act365F,
        DayCount::Act365L,
        DayCount::Thirty360,
        DayCount::ThirtyE360,
        DayCount::ActAct,
    ] {
        let yf = dc.year_fraction(date, date, ctx).unwrap();
        assert_eq!(yf, 0.0);
    }

    // Bus/252 with calendar
    let calendar = TARGET2;
    let ctx_bus = DayCountCtx {
        calendar: Some(&calendar),
        frequency: None,
        bus_basis: None,
    };
    let yf = DayCount::Bus252.year_fraction(date, date, ctx_bus).unwrap();
    assert_eq!(yf, 0.0);

    // Act/Act ISMA with frequency
    let ctx_isma = DayCountCtx {
        calendar: None,
        frequency: Some(Frequency::Months(6)),
        bus_basis: None,
    };
    let yf = DayCount::ActActIsma
        .year_fraction(date, date, ctx_isma)
        .unwrap();
    assert_eq!(yf, 0.0);
}

#[test]
fn daycount_inverted_dates_error() {
    let start = make_date(2025, 6, 15);
    let end = make_date(2025, 1, 1);

    for dc in [
        DayCount::Act360,
        DayCount::Act365F,
        DayCount::Thirty360,
        DayCount::ThirtyE360,
        DayCount::ActAct,
    ] {
        let result = dc.year_fraction(start, end, DayCountCtx::default());
        assert!(result.is_err());
    }
}

#[test]
fn daycount_one_day_fractions() {
    let start = make_date(2025, 6, 15);
    let end = make_date(2025, 6, 16);

    let yf_360 = DayCount::Act360
        .year_fraction(start, end, DayCountCtx::default())
        .unwrap();
    assert!((yf_360 - 1.0 / 360.0).abs() < 1e-10);

    let yf_365 = DayCount::Act365F
        .year_fraction(start, end, DayCountCtx::default())
        .unwrap();
    assert!((yf_365 - 1.0 / 365.0).abs() < 1e-10);
}

#[test]
fn thirty360_vs_thirtye360_difference() {
    // These should differ when day 31 is involved
    let start = make_date(2025, 1, 31);
    let end = make_date(2025, 3, 31);

    let yf_us = DayCount::Thirty360
        .year_fraction(start, end, DayCountCtx::default())
        .unwrap();

    let yf_eu = DayCount::ThirtyE360
        .year_fraction(start, end, DayCountCtx::default())
        .unwrap();

    // Should produce same result in this case
    assert!((yf_us - yf_eu).abs() < 1e-10);
}

#[test]
fn actact_isma_multiple_frequencies() {
    let start = make_date(2025, 1, 1);
    let end = make_date(2025, 4, 1);

    // Quarterly
    let ctx_q = DayCountCtx {
        calendar: None,
        frequency: Some(Frequency::Months(3)),
        bus_basis: None,
    };
    let yf_q = DayCount::ActActIsma
        .year_fraction(start, end, ctx_q)
        .unwrap();

    // Monthly
    let ctx_m = DayCountCtx {
        calendar: None,
        frequency: Some(Frequency::Months(1)),
        bus_basis: None,
    };
    let yf_m = DayCount::ActActIsma
        .year_fraction(start, end, ctx_m)
        .unwrap();

    // Different frequencies give different results for ISMA
    // Quarterly: 1 full period = 1.0
    assert!((yf_q - 1.0).abs() < 1.0e-6);
    // Monthly: 3 full periods = 3.0
    assert!((yf_m - 3.0).abs() < 1.0e-6);
}

#[test]
fn bus252_full_year_approximately_252() {
    let calendar = TARGET2;
    let start = make_date(2025, 1, 2);
    let end = make_date(2026, 1, 2);

    let ctx = DayCountCtx {
        calendar: Some(&calendar),
        frequency: None,
        bus_basis: None,
    };

    let yf = DayCount::Bus252.year_fraction(start, end, ctx).unwrap();

    // Should be close to 1.0 (252 business days in a year)
    assert!((yf - 1.0).abs() < 0.05);
}

#[test]
fn act365l_boundary_on_feb28_non_leap() {
    // Feb 28 in non-leap year, no Feb 29 in range
    let start = make_date(2025, 2, 28);
    let end = make_date(2025, 3, 1);

    let yf = DayCount::Act365L
        .year_fraction(start, end, DayCountCtx::default())
        .unwrap();

    let days = (end - start).whole_days() as f64;
    let expected = days / 365.0; // No leap day
    assert!((yf - expected).abs() < 1e-10);
}

#[test]
fn act365l_boundary_on_feb28_leap() {
    // Feb 28 to Mar 1 in leap year, Feb 29 in range
    let start = make_date(2024, 2, 28);
    let end = make_date(2024, 3, 1);

    let yf = DayCount::Act365L
        .year_fraction(start, end, DayCountCtx::default())
        .unwrap();

    let days = (end - start).whole_days() as f64;
    let expected = days / 366.0; // Feb 29 in (start, end]
    assert!((yf - expected).abs() < 1e-10);
}

#[test]
fn daycount_short_periods() {
    let start = make_date(2025, 6, 15);
    let end = make_date(2025, 6, 20);

    // All should produce sensible results for 5-day period
    for dc in [
        DayCount::Act360,
        DayCount::Act365F,
        DayCount::Thirty360,
        DayCount::ThirtyE360,
        DayCount::ActAct,
        DayCount::Act365L,
    ] {
        let yf = dc
            .year_fraction(start, end, DayCountCtx::default())
            .unwrap();
        assert!(yf > 0.0 && yf < 0.05); // Small positive fraction
    }
}

#[test]
fn daycount_long_periods() {
    let start = make_date(2020, 1, 1);
    let end = make_date(2030, 1, 1);

    // 10-year period
    for dc in [
        DayCount::Act360,
        DayCount::Act365F,
        DayCount::Thirty360,
        DayCount::ThirtyE360,
        DayCount::ActAct,
    ] {
        let yf = dc
            .year_fraction(start, end, DayCountCtx::default())
            .unwrap();
        assert!(yf > 9.9 && yf < 10.2, "Failed for {:?}: yf = {}", dc, yf); // Close to 10 years
    }

    // Act365L uses different denominator logic, so allow wider range
    let yf_365l = DayCount::Act365L
        .year_fraction(start, end, DayCountCtx::default())
        .unwrap();
    assert!(yf_365l > 9.8 && yf_365l < 10.3);
}

#[test]
fn actact_isma_partial_period() {
    let start = make_date(2025, 1, 15);
    let end = make_date(2025, 4, 15);

    let ctx = DayCountCtx {
        calendar: None,
        frequency: Some(Frequency::Months(6)),
        bus_basis: None,
    };

    let yf = DayCount::ActActIsma.year_fraction(start, end, ctx).unwrap();

    // 3 months out of 6-month period ≈ 0.5
    assert!(yf > 0.45 && yf < 0.55);
}

#[test]
fn bus252_excludes_holidays() {
    let calendar = TARGET2;

    // Period including Christmas
    let start = make_date(2024, 12, 23); // Monday
    let end = make_date(2024, 12, 30); // Monday next week

    let ctx = DayCountCtx {
        calendar: Some(&calendar),
        frequency: None,
        bus_basis: None,
    };

    let yf = DayCount::Bus252.year_fraction(start, end, ctx).unwrap();
    let biz_days = yf * 252.0;

    // Dec 25, 26 are holidays, plus weekend = only a few business days
    assert!(biz_days < 5.0);
}

#[test]
fn thirty360_february_special_case() {
    // February in 30/360 conventions
    let start = make_date(2025, 2, 1);
    let end = make_date(2025, 2, 28);

    let yf = DayCount::Thirty360
        .year_fraction(start, end, DayCountCtx::default())
        .unwrap();

    // Feb 1 (day 1) to Feb 28 (day 28) = 27 days
    assert!((yf - 27.0 / 360.0).abs() < 1e-10);
}

#[test]
fn actact_vs_actact_isma_comparison() {
    let start = make_date(2025, 1, 1);
    let end = make_date(2026, 1, 1);

    let yf_isda = DayCount::ActAct
        .year_fraction(start, end, DayCountCtx::default())
        .unwrap();

    let ctx_isma = DayCountCtx {
        calendar: None,
        frequency: Some(Frequency::Months(12)),
        bus_basis: None,
    };
    let yf_isma = DayCount::ActActIsma
        .year_fraction(start, end, ctx_isma)
        .unwrap();

    // For a full year period with annual frequency, both should give 1.0
    assert!((yf_isda - 1.0).abs() < 1e-10);
    assert!((yf_isma - 1.0).abs() < 1e-10);
}
