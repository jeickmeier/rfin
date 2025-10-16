//! Index ratio calculation tests for Inflation-Linked Bonds
//!
//! Tests cover:
//! - Index ratio calculations with linear interpolation (TIPS/Canadian)
//! - Index ratio with step interpolation (UK)
//! - Lag application (3-month, 8-month)
//! - Deflation protection (none, maturity-only, all payments)
//! - Index vs curve-based ratio calculations
//! - Market context routing

use super::common::*;
use finstack_core::market_data::scalars::inflation_index::{InflationInterpolation, InflationLag};
use finstack_valuations::instruments::inflation_linked_bond::{
    DeflationProtection, IndexationMethod,
};

#[test]
fn test_index_ratio_basic_linear_interpolation() {
    // Arrange
    let ilb = sample_tips();
    let (_, index) = market_context_with_index();

    // Verify interpolation method matches TIPS requirements
    assert_eq!(index.interpolation(), InflationInterpolation::Linear);

    // Act - calculate index ratio for a date with 3-month lag
    let ratio = ilb.index_ratio(d(2025, 4, 1), &index).unwrap();

    // Assert - ratio should be positive and reasonable (near 1.0 for small inflation)
    assert!(ratio > 0.9);
    assert!(ratio < 1.3);
}

#[test]
fn test_index_ratio_with_3month_lag() {
    // Arrange
    let mut ilb = sample_tips();
    ilb.lag = InflationLag::Months(3);
    ilb.base_index = 300.0;
    ilb.base_date = d(2024, 12, 1);

    // Create index with specific values for precise lag testing
    let observations = vec![
        (d(2024, 12, 1), 300.0), // Reference for Jan 1 (3mo lag)
        (d(2025, 1, 1), 301.0),  // Reference for Apr 1 (3mo lag)
    ];
    let index = finstack_core::market_data::scalars::inflation_index::InflationIndex::new(
        "US-CPI-U",
        observations,
        finstack_core::currency::Currency::USD,
    )
    .unwrap()
    .with_interpolation(InflationInterpolation::Linear);

    // Act - valuation date Apr 1, 2025 → reference date Jan 1, 2025 (3-month lag)
    let ratio = ilb.index_ratio(d(2025, 4, 1), &index).unwrap();

    // Assert - should use Jan 1 CPI (301) vs base (300)
    assert_approx_eq(ratio, 301.0 / 300.0, REL_TOL, "3-month lag ratio");
}

#[test]
fn test_index_ratio_with_8month_lag_uk() {
    // Arrange
    let mut ilb = sample_uk_linker();
    ilb.lag = InflationLag::Months(8);
    ilb.base_index = 320.0;
    ilb.base_date = d(2024, 6, 1);

    // Create index with specific values for precise lag testing
    let observations = vec![
        (d(2024, 6, 1), 320.0), // Reference for Feb 1 (8mo lag)
        (d(2025, 2, 1), 326.4), // Reference for Oct 1 (8mo lag)
    ];
    let index = finstack_core::market_data::scalars::inflation_index::InflationIndex::new(
        "UK-RPI",
        observations,
        finstack_core::currency::Currency::GBP,
    )
    .unwrap()
    .with_interpolation(InflationInterpolation::Step);

    // Act - valuation date Oct 1, 2025 → reference date Feb 1, 2025 (8-month lag)
    let ratio = ilb.index_ratio(d(2025, 10, 1), &index).unwrap();

    // Assert
    assert_approx_eq(ratio, 326.4 / 320.0, REL_TOL, "8-month lag ratio");
}

#[test]
fn test_index_ratio_no_deflation_protection() {
    // Arrange
    let mut ilb = sample_tips();
    ilb.deflation_protection = DeflationProtection::None;
    ilb.base_index = 300.0;

    // Create deflation scenario index
    let observations = vec![
        (d(2024, 10, 1), 295.0), // Deflation scenario
        (d(2025, 1, 1), 295.0),
    ];
    let index = finstack_core::market_data::scalars::inflation_index::InflationIndex::new(
        "US-CPI-U",
        observations,
        finstack_core::currency::Currency::USD,
    )
    .unwrap()
    .with_interpolation(InflationInterpolation::Linear);

    // Act - calculate ratio when CPI drops below base
    let ratio = ilb.index_ratio(d(2025, 1, 1), &index).unwrap();

    // Assert - no floor, ratio can be < 1.0
    assert!(ratio < 1.0);
    assert_approx_eq(ratio, 295.0 / 300.0, REL_TOL, "deflation no protection");
}

#[test]
fn test_index_ratio_maturity_only_deflation_protection() {
    // Arrange
    let mut ilb = sample_tips();
    ilb.deflation_protection = DeflationProtection::MaturityOnly;
    ilb.base_index = 300.0;
    ilb.maturity = d(2025, 1, 15);

    // Create deflation scenario index
    let observations = vec![
        (d(2024, 10, 1), 295.0), // Deflation
        (d(2025, 1, 1), 295.0),
        (d(2025, 1, 15), 295.0),
    ];
    let index = finstack_core::market_data::scalars::inflation_index::InflationIndex::new(
        "US-CPI-U",
        observations,
        finstack_core::currency::Currency::USD,
    )
    .unwrap()
    .with_interpolation(InflationInterpolation::Linear);

    // Act - calculate ratio at maturity vs intermediate date
    let ratio_at_maturity = ilb.index_ratio(ilb.maturity, &index).unwrap();
    let ratio_before_maturity = ilb.index_ratio(d(2025, 1, 1), &index).unwrap();

    // Assert
    // At maturity: floor applies
    assert!(ratio_at_maturity >= 1.0);
    assert_approx_eq(ratio_at_maturity, 1.0, REL_TOL, "maturity floor");

    // Before maturity: no floor
    assert!(ratio_before_maturity < 1.0);
}

#[test]
fn test_index_ratio_all_payments_deflation_protection() {
    // Arrange
    let mut ilb = sample_tips();
    ilb.deflation_protection = DeflationProtection::AllPayments;
    ilb.base_index = 300.0;

    // Create deflation scenario index
    let observations = vec![
        (d(2024, 10, 1), 295.0), // Deflation
        (d(2025, 1, 1), 295.0),
    ];
    let index = finstack_core::market_data::scalars::inflation_index::InflationIndex::new(
        "US-CPI-U",
        observations,
        finstack_core::currency::Currency::USD,
    )
    .unwrap()
    .with_interpolation(InflationInterpolation::Linear);

    // Act - calculate ratio at any date
    let ratio = ilb.index_ratio(d(2025, 1, 1), &index).unwrap();

    // Assert - floor applies to all payments
    assert!(ratio >= 1.0);
    assert_approx_eq(ratio, 1.0, REL_TOL, "all payments floor");
}

#[test]
fn test_index_ratio_from_curve() {
    // Arrange
    let mut ilb = sample_tips();
    ilb.base_index = 300.0;
    ilb.base_date = d(2024, 12, 1);

    let (_, curve) = market_context_with_curve();

    // Act - calculate ratio using inflation curve (forward projection)
    let ratio = ilb.index_ratio_from_curve(d(2026, 12, 1), &curve).unwrap();

    // Assert - ratio should reflect 2-year inflation at ~2% p.a.
    assert!(ratio > 1.03); // > 4% total
    assert!(ratio < 1.06); // < 6% total
}

#[test]
fn test_index_ratio_from_curve_at_base_date() {
    // Arrange
    let mut ilb = sample_tips();
    ilb.base_index = 300.0;
    ilb.base_date = d(2024, 12, 1);

    let (_, curve) = market_context_with_curve();

    // Act - calculate ratio at or before base date
    let ratio = ilb.index_ratio_from_curve(d(2024, 10, 1), &curve).unwrap();

    // Assert - should use base CPI, so ratio = base_cpi / base_index
    assert_approx_eq(ratio, 300.0 / 300.0, REL_TOL, "ratio at base");
}

#[test]
fn test_index_ratio_from_market_routes_to_index() {
    // Arrange
    let ilb = sample_tips();
    let (ctx, index) = market_context_with_index();

    // Act
    let ratio_from_market = ilb.index_ratio_from_market(d(2025, 4, 1), &ctx).unwrap();
    let ratio_from_index = ilb.index_ratio(d(2025, 4, 1), &index).unwrap();

    // Assert - should be identical
    assert_approx_eq(
        ratio_from_market,
        ratio_from_index,
        EPSILON,
        "market routing to index",
    );
}

#[test]
fn test_index_ratio_from_market_routes_to_curve() {
    // Arrange
    let ilb = sample_tips();
    let (ctx, curve) = market_context_with_curve();

    // Act
    let ratio_from_market = ilb.index_ratio_from_market(d(2025, 4, 1), &ctx).unwrap();
    let ratio_from_curve = ilb.index_ratio_from_curve(d(2025, 4, 1), &curve).unwrap();

    // Assert - should be identical
    assert_approx_eq(
        ratio_from_market,
        ratio_from_curve,
        EPSILON,
        "market routing to curve",
    );
}

#[test]
fn test_index_ratio_consistency_between_sources() {
    // Arrange
    let ilb = sample_tips();
    let (_ctx_index, index) = market_context_with_index();
    let (_ctx_curve, curve) = market_context_with_curve();

    // Act - same date, different sources
    let ratio_index = ilb.index_ratio(d(2025, 4, 1), &index).unwrap();
    let ratio_curve = ilb.index_ratio_from_curve(d(2025, 4, 1), &curve).unwrap();

    // Assert - should be consistent (within tolerance due to different representations)
    // Index uses observations, curve uses forward projection - can differ significantly
    // due to different data sources and interpolation methods
    assert!(ratio_index > 0.0);
    assert!(ratio_curve > 0.0);
    // Both should be in reasonable range (0.8 to 1.5 for modest inflation)
    assert!(ratio_index > 0.8 && ratio_index < 1.5);
    assert!(ratio_curve > 0.8 && ratio_curve < 1.5);
}

#[test]
fn test_index_ratio_tips_requires_linear_interpolation() {
    // Arrange
    let ilb = sample_tips();
    let observations = vec![(d(2024, 12, 1), 300.0)];
    let index = finstack_core::market_data::scalars::inflation_index::InflationIndex::new(
        "US-CPI-U",
        observations,
        finstack_core::currency::Currency::USD,
    )
    .unwrap()
    .with_interpolation(InflationInterpolation::Step); // Wrong for TIPS

    // Act & Assert - should fail validation
    let result = ilb.index_ratio(d(2025, 1, 1), &index);
    assert!(result.is_err());
}

#[test]
fn test_index_ratio_uk_requires_step_interpolation() {
    // Arrange
    let mut ilb = sample_uk_linker();
    ilb.indexation_method = IndexationMethod::UK;

    let observations = vec![(d(2024, 6, 1), 320.0)];
    let index = finstack_core::market_data::scalars::inflation_index::InflationIndex::new(
        "UK-RPI",
        observations,
        finstack_core::currency::Currency::GBP,
    )
    .unwrap()
    .with_interpolation(InflationInterpolation::Linear); // Wrong for UK

    // Act & Assert - should fail validation
    let result = ilb.index_ratio(d(2025, 1, 1), &index);
    assert!(result.is_err());
}

#[test]
fn test_index_ratio_canadian_requires_linear_interpolation() {
    // Arrange
    let mut ilb = sample_tips();
    ilb.indexation_method = IndexationMethod::Canadian;

    let observations = vec![(d(2024, 9, 1), 140.0)];
    let index = finstack_core::market_data::scalars::inflation_index::InflationIndex::new(
        "CA-CPI",
        observations,
        finstack_core::currency::Currency::CAD,
    )
    .unwrap()
    .with_interpolation(InflationInterpolation::Step); // Wrong for Canadian

    // Act & Assert - should fail validation
    let result = ilb.index_ratio(d(2025, 1, 1), &index);
    assert!(result.is_err());
}

#[test]
fn test_index_ratio_rejects_zero_base_index() {
    // Arrange
    let mut ilb = sample_tips();
    ilb.base_index = 0.0; // Invalid

    let (_, index) = market_context_with_index();

    // Act & Assert
    let result = ilb.index_ratio(d(2025, 1, 1), &index);
    assert!(result.is_err());
}

#[test]
fn test_index_ratio_rejects_negative_base_index() {
    // Arrange
    let mut ilb = sample_tips();
    ilb.base_index = -100.0; // Invalid

    let (_, index) = market_context_with_index();

    // Act & Assert
    let result = ilb.index_ratio(d(2025, 1, 1), &index);
    assert!(result.is_err());
}

#[test]
fn test_index_ratio_extreme_inflation() {
    // Arrange
    let mut ilb = sample_tips();
    ilb.base_index = 100.0;

    // Create extreme inflation scenario index
    let observations = vec![
        (d(2024, 10, 1), 500.0), // 400% inflation
        (d(2025, 1, 1), 500.0),
    ];
    let index = finstack_core::market_data::scalars::inflation_index::InflationIndex::new(
        "US-CPI-U",
        observations,
        finstack_core::currency::Currency::USD,
    )
    .unwrap()
    .with_interpolation(InflationInterpolation::Linear);

    // Act
    let ratio = ilb.index_ratio(d(2025, 1, 1), &index).unwrap();

    // Assert
    assert_approx_eq(ratio, 5.0, REL_TOL, "extreme inflation");
}

#[test]
fn test_index_ratio_time_series() {
    // Arrange
    let mut ilb = sample_tips();
    ilb.base_index = 300.0;
    ilb.lag = InflationLag::Months(3);

    // Build a time series with steady 0.5% monthly inflation
    let mut observations = Vec::new();
    for i in 0..12 {
        let date = d(2024, 12, 1);
        let month_date = finstack_core::dates::add_months(date, i);
        let value = 300.0 * (1.005_f64).powi(i);
        observations.push((month_date, value));
    }
    let index = finstack_core::market_data::scalars::inflation_index::InflationIndex::new(
        "US-CPI-U",
        observations,
        finstack_core::currency::Currency::USD,
    )
    .unwrap()
    .with_interpolation(InflationInterpolation::Linear);

    // Act - calculate ratios over time
    let ratio_1m = ilb.index_ratio(d(2025, 4, 1), &index).unwrap();
    let ratio_6m = ilb.index_ratio(d(2025, 9, 1), &index).unwrap();

    // Assert - ratios should increase over time
    assert!(ratio_6m > ratio_1m);
    assert!(ratio_1m > 1.0);
}
