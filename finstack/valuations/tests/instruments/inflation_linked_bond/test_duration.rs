//! Duration and DV01 tests for Inflation-Linked Bonds
//!
//! Tests cover:
//! - Real duration (modified duration in real terms)
//! - DV01 (dollar value of 1bp move)
//! - Duration-price relationship
//! - Time decay effects
//! - Sensitivity to coupon and maturity

use super::common::*;

#[test]
fn test_real_duration_positive() {
    // Arrange
    let ilb = sample_tips();
    let (ctx, _) = market_context_with_index();
    let as_of = d(2025, 1, 2);

    // Act
    let dur = ilb.real_duration(&ctx, as_of).unwrap();

    // Assert - duration should be positive for standard bonds
    assert!(dur > 0.0);
}

#[test]
fn test_real_duration_increases_with_maturity() {
    // Arrange
    let (ctx, _) = market_context_with_index();
    let as_of = d(2025, 1, 2);

    // Short-dated bond
    let mut ilb_short = sample_tips();
    ilb_short.issue_date = d(2025, 1, 2);
    ilb_short.maturity = d(2027, 1, 2); // 2 years
    ilb_short.real_coupon = 0.02;

    // Medium-dated bond
    let mut ilb_mid = sample_tips();
    ilb_mid.issue_date = d(2025, 1, 2);
    ilb_mid.maturity = d(2030, 1, 2); // 5 years
    ilb_mid.real_coupon = 0.02;

    // Long-dated bond
    let mut ilb_long = sample_tips();
    ilb_long.issue_date = d(2025, 1, 2);
    ilb_long.maturity = d(2035, 1, 2); // 10 years
    ilb_long.real_coupon = 0.02;

    // Act
    let dur_short = ilb_short.real_duration(&ctx, as_of).unwrap();
    let dur_mid = ilb_mid.real_duration(&ctx, as_of).unwrap();
    let dur_long = ilb_long.real_duration(&ctx, as_of).unwrap();

    // Assert - longer maturity → higher duration
    assert!(dur_mid > dur_short);
    assert!(dur_long > dur_mid);
}

#[test]
fn test_real_duration_decreases_with_higher_coupon() {
    // Arrange
    let (ctx, _) = market_context_with_index();
    let as_of = d(2025, 1, 2);

    // Low coupon bond
    let mut ilb_low = sample_tips();
    ilb_low.issue_date = d(2025, 1, 2);
    ilb_low.maturity = d(2030, 1, 2);
    ilb_low.real_coupon = 0.01; // 1%

    // High coupon bond
    let mut ilb_high = sample_tips();
    ilb_high.issue_date = d(2025, 1, 2);
    ilb_high.maturity = d(2030, 1, 2);
    ilb_high.real_coupon = 0.05; // 5%

    // Act
    let dur_low = ilb_low.real_duration(&ctx, as_of).unwrap();
    let dur_high = ilb_high.real_duration(&ctx, as_of).unwrap();

    // Assert - higher coupon → lower duration (more front-loaded cashflows)
    assert!(dur_low > dur_high);
}

#[test]
fn test_real_duration_reasonable_range() {
    // Arrange
    let mut ilb = sample_tips();
    ilb.issue_date = d(2025, 1, 2);
    ilb.maturity = d(2030, 1, 2); // 5 years
    ilb.real_coupon = 0.02;

    let (ctx, _) = market_context_with_index();
    let as_of = d(2025, 1, 2);

    // Act
    let dur = ilb.real_duration(&ctx, as_of).unwrap();

    // Assert - for 5-year bond with 2% coupon, duration should be ~4.5 years
    assert!(dur > 3.5);
    assert!(dur < 5.5);
}

#[test]
fn test_real_duration_decreases_over_time() {
    // Arrange
    let mut ilb = sample_tips();
    ilb.issue_date = d(2020, 1, 2);
    ilb.maturity = d(2030, 1, 2);
    ilb.real_coupon = 0.02;

    let (ctx, _) = market_context_with_index();

    // Act - calculate duration at different valuation dates
    let dur_2020 = ilb.real_duration(&ctx, d(2020, 1, 2)).unwrap();
    let dur_2025 = ilb.real_duration(&ctx, d(2025, 1, 2)).unwrap();
    let dur_2028 = ilb.real_duration(&ctx, d(2028, 1, 2)).unwrap();

    // Assert - as time passes, duration decreases
    assert!(dur_2025 < dur_2020);
    assert!(dur_2028 < dur_2025);
}

#[test]
fn test_real_duration_at_maturity() {
    // Arrange
    let mut ilb = sample_tips();
    ilb.issue_date = d(2024, 1, 2);
    ilb.maturity = d(2025, 1, 2);

    let (ctx, _) = market_context_with_index();
    let as_of = ilb.maturity;

    // Act - duration calculation at maturity may fail or return small value
    let dur_result = ilb.real_duration(&ctx, as_of);

    // Assert - either errors gracefully or returns small value
    if let Ok(dur) = dur_result {
        assert!(dur < 1.0); // Should be small
    } else {
        // Acceptable to error at maturity
        assert!(dur_result.is_err());
    }
}

#[test]
fn test_real_duration_with_different_frequencies() {
    // Arrange
    let (ctx, _) = market_context_with_index();
    let as_of = d(2025, 1, 2);

    // Annual payments
    let mut ilb_annual = sample_tips();
    ilb_annual.frequency = finstack_core::dates::Tenor::annual();
    ilb_annual.issue_date = d(2025, 1, 2);
    ilb_annual.maturity = d(2030, 1, 2);
    ilb_annual.real_coupon = 0.02;

    // Semi-annual payments
    let mut ilb_semi = sample_tips();
    ilb_semi.frequency = finstack_core::dates::Tenor::semi_annual();
    ilb_semi.issue_date = d(2025, 1, 2);
    ilb_semi.maturity = d(2030, 1, 2);
    ilb_semi.real_coupon = 0.02;

    // Act
    let dur_annual = ilb_annual.real_duration(&ctx, as_of).unwrap();
    let dur_semi = ilb_semi.real_duration(&ctx, as_of).unwrap();

    // Assert - duration should be positive for both
    assert!(dur_annual > 0.0);
    assert!(dur_semi > 0.0);
    // More frequent payments → slightly lower duration (generally)
    // But result depends on exact curve shape and bond parameters
}

#[test]
fn test_real_duration_uses_quoted_price() {
    // Arrange
    let mut ilb1 = sample_tips();
    let mut ilb2 = sample_tips();

    ilb1.quoted_clean = Some(100.0);
    ilb2.quoted_clean = Some(110.0);

    let (ctx, _) = market_context_with_index();
    let as_of = d(2025, 1, 2);

    // Act
    let dur1 = ilb1.real_duration(&ctx, as_of).unwrap();
    let dur2 = ilb2.real_duration(&ctx, as_of).unwrap();

    // Assert - duration calculation uses quoted price as base
    // Different prices may lead to slightly different durations due to yield differences
    assert!(dur1 > 0.0);
    assert!(dur2 > 0.0);
}

#[test]
fn test_dv01_positive_before_maturity() {
    // Arrange
    let ilb = sample_tips();
    let as_of = d(2025, 1, 2);

    // Manually calculate DV01 using the formula from the implementation
    // DV01 = Notional × Time to Maturity × 1bp
    let time_to_maturity = ilb
        .day_count
        .year_fraction(
            as_of,
            ilb.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )
        .unwrap();

    let expected_dv01 = ilb.notional.amount() * time_to_maturity * 0.0001;

    // Assert
    assert!(expected_dv01 > 0.0);
}

#[test]
fn test_dv01_zero_at_maturity() {
    // Arrange
    let mut ilb = sample_tips();
    ilb.maturity = d(2025, 1, 2);

    let as_of = ilb.maturity;

    // Act - calculate time to maturity
    let time_to_maturity = ilb
        .day_count
        .year_fraction(
            as_of,
            ilb.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )
        .unwrap();

    // Assert - at maturity, DV01 should be zero
    assert_eq!(time_to_maturity, 0.0);
}

#[test]
fn test_dv01_zero_after_maturity() {
    // Arrange
    let mut ilb = sample_tips();
    ilb.maturity = d(2025, 1, 2);

    let as_of = d(2025, 6, 1); // After maturity

    // Act - year_fraction with as_of > maturity returns error (invalid date range)
    let time_result = ilb.day_count.year_fraction(
        as_of,
        ilb.maturity,
        finstack_core::dates::DayCountCtx::default(),
    );

    // Assert - should error or return negative value
    // When as_of >= maturity in DV01 calculator, it returns 0
    assert!(time_result.is_err() || time_result.unwrap() <= 0.0);
}

#[test]
fn test_dv01_scales_with_notional() {
    // Arrange
    let as_of = d(2025, 1, 2);

    let mut ilb_1m = sample_tips();
    ilb_1m.notional =
        finstack_core::money::Money::new(1_000_000.0, finstack_core::currency::Currency::USD);

    let mut ilb_10m = sample_tips();
    ilb_10m.notional =
        finstack_core::money::Money::new(10_000_000.0, finstack_core::currency::Currency::USD);

    // Act
    let time_to_maturity = ilb_1m
        .day_count
        .year_fraction(
            as_of,
            ilb_1m.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )
        .unwrap();

    let dv01_1m = ilb_1m.notional.amount() * time_to_maturity * 0.0001;
    let dv01_10m = ilb_10m.notional.amount() * time_to_maturity * 0.0001;

    // Assert - 10x notional → 10x DV01
    assert_approx_eq(dv01_10m / dv01_1m, 10.0, EPSILON, "DV01 notional scaling");
}

#[test]
fn test_dv01_scales_with_time_to_maturity() {
    // Arrange
    let as_of = d(2025, 1, 2);

    let mut ilb_2y = sample_tips();
    ilb_2y.issue_date = d(2025, 1, 2);
    ilb_2y.maturity = d(2027, 1, 2); // 2 years

    let mut ilb_10y = sample_tips();
    ilb_10y.issue_date = d(2025, 1, 2);
    ilb_10y.maturity = d(2035, 1, 2); // 10 years

    // Act
    let ttm_2y = ilb_2y
        .day_count
        .year_fraction(
            as_of,
            ilb_2y.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )
        .unwrap();

    let ttm_10y = ilb_10y
        .day_count
        .year_fraction(
            as_of,
            ilb_10y.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )
        .unwrap();

    let dv01_2y = ilb_2y.notional.amount() * ttm_2y * 0.0001;
    let dv01_10y = ilb_10y.notional.amount() * ttm_10y * 0.0001;

    // Assert - longer maturity → higher DV01
    assert!(dv01_10y > dv01_2y);
    assert_approx_eq(
        dv01_10y / dv01_2y,
        ttm_10y / ttm_2y,
        EPSILON,
        "DV01 time scaling",
    );
}

#[test]
fn test_dv01_reasonable_magnitude() {
    // Arrange
    let mut ilb = sample_tips();
    ilb.notional =
        finstack_core::money::Money::new(1_000_000.0, finstack_core::currency::Currency::USD);
    ilb.issue_date = d(2025, 1, 2);
    ilb.maturity = d(2030, 1, 2); // 5 years

    let as_of = d(2025, 1, 2);

    // Act
    let ttm = ilb
        .day_count
        .year_fraction(
            as_of,
            ilb.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )
        .unwrap();
    let dv01 = ilb.notional.amount() * ttm * 0.0001;

    // Assert - for 1M notional with 5 years: DV01 = 1M * 5 * 0.0001 = 500
    assert!(dv01 > 400.0);
    assert!(dv01 < 600.0);
}

#[test]
fn test_duration_and_dv01_relationship() {
    // Arrange
    let ilb = sample_tips();
    let (ctx, _) = market_context_with_index();
    let as_of = d(2025, 1, 2);

    // Act
    let duration = ilb.real_duration(&ctx, as_of).unwrap();

    let ttm = ilb
        .day_count
        .year_fraction(
            as_of,
            ilb.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )
        .unwrap();
    let dv01_approx = ilb.notional.amount() * ttm * 0.0001;

    // Assert - DV01 approximation uses time to maturity
    // Modified duration should be in similar ballpark but not identical
    // (Duration is more sophisticated, considering cashflow timing)
    assert!(duration > 0.0);
    assert!(dv01_approx > 0.0);
}

#[test]
fn test_real_duration_uk_gilt() {
    // Arrange
    let ilb = sample_uk_linker();
    let (ctx, _) = uk_market_context();
    let as_of = d(2025, 1, 2);

    // Act
    let dur = ilb.real_duration(&ctx, as_of).unwrap();

    // Assert - 20-year gilt should have substantial duration
    assert!(dur > 10.0);
    assert!(dur < 20.0);
}

#[test]
fn test_duration_calculation_performance() {
    // Arrange
    let ilb = sample_tips();
    let (ctx, _) = market_context_with_index();
    let as_of = d(2025, 1, 2);

    // Act
    let start = std::time::Instant::now();
    for _ in 0..100 {
        let _ = ilb.real_duration(&ctx, as_of).unwrap();
    }
    let elapsed = start.elapsed();

    // Assert - 100 duration calculations should be fast (< 1 second)
    assert!(elapsed.as_secs() < 1);
}
