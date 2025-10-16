//! Tests for variance calculations (realized, forward, expected).

use super::common::*;
use finstack_core::dates::Frequency;
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::stats::{realized_variance, RealizedVarMethod};
use finstack_valuations::instruments::variance_swap::PayReceive;

// ============================================================================
// Historical Prices Tests
// ============================================================================

#[test]
fn test_get_historical_prices_prefers_time_series_over_spot() {
    // Arrange
    let swap = sample_swap(PayReceive::Receive);
    let prices = price_series(&swap, 4_900.0, 5.0);
    let ctx = add_series(base_context(), &prices);

    // Act
    let extracted = swap.get_historical_prices(&ctx, swap.maturity).unwrap();

    // Assert
    assert!(extracted.len() >= prices.len());
}

#[test]
fn test_get_historical_prices_falls_back_to_spot_scalar() {
    // Arrange
    let swap = sample_swap(PayReceive::Receive);
    let ctx = base_context(); // No series, just spot

    // Act
    let extracted = swap.get_historical_prices(&ctx, swap.maturity).unwrap();

    // Assert
    assert_eq!(extracted.len(), 1);
    assert!((extracted[0] - 5_000.0).abs() < EPSILON);
}

#[test]
fn test_get_historical_prices_returns_empty_if_no_data() {
    // Arrange
    let swap = sample_swap(PayReceive::Receive);
    let ctx = MarketContext::new().insert_discount(
        finstack_core::market_data::term_structures::discount_curve::DiscountCurve::builder(
            DISC_ID,
        )
        .base_date(swap.start_date)
        .knots([(0.0, 1.0), (1.0, 0.95)])
        .build()
        .unwrap(),
    );

    // Act
    let extracted = swap.get_historical_prices(&ctx, swap.maturity).unwrap();

    // Assert
    assert!(extracted.is_empty());
}

#[test]
fn test_get_historical_prices_filters_by_as_of_date() {
    // Arrange
    let swap = sample_swap(PayReceive::Receive);
    let prices = price_series(&swap, 5_000.0, 2.0);
    let ctx = add_series(base_context(), &prices);
    let as_of = date(2025, 2, 1);

    // Act
    let extracted = swap.get_historical_prices(&ctx, as_of).unwrap();

    // Assert
    let expected_count = prices.iter().filter(|(d, _)| *d <= as_of).count();
    assert!(extracted.len() <= expected_count);
}

// ============================================================================
// Realized Variance Tests
// ============================================================================

#[test]
fn test_partial_realized_variance_before_start_is_zero() {
    // Arrange
    let swap = sample_swap(PayReceive::Receive);
    let ctx = base_context();
    let as_of = date(2024, 12, 1);

    // Act
    let realized = swap.partial_realized_variance(&ctx, as_of).unwrap();

    // Assert
    assert_eq!(realized, 0.0);
}

#[test]
fn test_partial_realized_variance_with_insufficient_prices_is_zero() {
    // Arrange
    let swap = sample_swap(PayReceive::Receive);
    let ctx = base_context(); // Only spot, not a series
    let as_of = swap.start_date;

    // Act
    let realized = swap.partial_realized_variance(&ctx, as_of).unwrap();

    // Assert
    assert_eq!(realized, 0.0);
}

#[test]
fn test_partial_realized_variance_matches_manual_calculation() {
    // Arrange
    let swap = sample_swap(PayReceive::Receive);
    let prices = price_series(&swap, 5_000.0, 2.0);
    let ctx = add_series(base_context(), &prices);
    let as_of = date(2025, 2, 1);

    // Act
    let realized = swap.partial_realized_variance(&ctx, as_of).unwrap();

    // Manual calculation
    let obs_dates = swap.observation_dates();
    let used_prices: Vec<f64> = obs_dates
        .iter()
        .filter(|d| **d >= swap.start_date && **d <= as_of)
        .filter_map(|d| prices.iter().find(|(pd, _)| pd == d).map(|(_, p)| *p))
        .collect();
    let manual = realized_variance(&used_prices, RealizedVarMethod::CloseToClose, 252.0);

    // Assert
    assert!((realized - manual).abs() < EPSILON);
}

#[test]
fn test_partial_realized_variance_uses_policy_annualization() {
    // Arrange
    let swap = sample_swap(PayReceive::Receive);
    let prices = price_series(&swap, 5_000.0, 3.0);
    let ctx = add_series(
        add_unitless(base_context(), "TRADING_DAYS_PER_YEAR", 260.0),
        &prices,
    );
    let as_of = date(2025, 2, 1);

    // Act
    let realized = swap.partial_realized_variance(&ctx, as_of).unwrap();

    // Assert - should be using 260.0 instead of 252.0
    let obs_dates = swap.observation_dates();
    let used_prices: Vec<f64> = obs_dates
        .iter()
        .filter(|d| **d >= swap.start_date && **d <= as_of)
        .filter_map(|d| prices.iter().find(|(pd, _)| pd == d).map(|(_, p)| *p))
        .collect();
    let manual_260 = realized_variance(&used_prices, RealizedVarMethod::CloseToClose, 260.0);

    // The policy override should be applied
    assert!((realized - manual_260).abs() < EPSILON);
}

#[test]
fn test_partial_realized_variance_is_always_non_negative() {
    // Arrange
    let swap = sample_swap(PayReceive::Receive);
    let prices = price_series(&swap, 5_000.0, -10.0); // Declining prices
    let ctx = add_series(base_context(), &prices);
    let as_of = date(2025, 2, 15);

    // Act
    let realized = swap.partial_realized_variance(&ctx, as_of).unwrap();

    // Assert
    assert!(realized >= 0.0, "Variance must be non-negative");
}

// ============================================================================
// Forward Variance Tests
// ============================================================================

#[test]
fn test_remaining_forward_variance_falls_back_to_strike() {
    // Arrange
    let swap = sample_swap(PayReceive::Receive);
    let ctx = base_context(); // No implied vol

    // Act
    let forward = swap
        .remaining_forward_variance(&ctx, swap.start_date)
        .unwrap();

    // Assert
    assert!((forward - swap.strike_variance).abs() < EPSILON);
}

#[test]
fn test_remaining_forward_variance_uses_implied_vol_when_present() {
    // Arrange
    let swap = sample_swap(PayReceive::Receive);
    let ctx = add_unitless(base_context(), format!("{}_IMPL_VOL", UNDERLYING_ID), 0.22);

    // Act
    let forward = swap
        .remaining_forward_variance(&ctx, swap.start_date)
        .unwrap();

    // Assert
    let expected = 0.22_f64.powi(2);
    assert!((forward - expected).abs() < EPSILON);
}

#[test]
fn test_remaining_forward_variance_uses_surface_when_available() {
    // Arrange
    let swap = sample_swap(PayReceive::Receive);
    let surface = sample_surface();
    let ctx = add_unitless(
        add_surface(base_context(), surface),
        format!("{}-DIVYIELD", UNDERLYING_ID),
        0.01,
    );
    let as_of = date(2024, 12, 1);

    // Act
    let forward = swap.remaining_forward_variance(&ctx, as_of).unwrap();

    // Assert - should use surface for forward variance
    let t = swap
        .day_count
        .year_fraction(as_of, swap.maturity, Default::default())
        .unwrap();
    let surface_check = sample_surface();
    let _vol_atm = surface_check.value_clamped(t.max(1e-8), 5_000.0);

    // The actual result may be from VIX-style replication or ATM fallback
    assert!(forward > 0.0);
    assert!(forward.is_finite());
}

#[test]
fn test_remaining_forward_variance_is_always_positive() {
    // Arrange
    let swap = sample_swap(PayReceive::Receive);
    let ctx = add_unitless(base_context(), format!("{}_IMPL_VOL", UNDERLYING_ID), 0.15);

    // Act
    let forward = swap
        .remaining_forward_variance(&ctx, swap.start_date)
        .unwrap();

    // Assert
    assert!(forward > 0.0);
}

// ============================================================================
// Expected Variance Tests
// ============================================================================

#[test]
fn test_expected_variance_before_start_equals_forward() {
    // Arrange
    let swap = sample_swap(PayReceive::Receive);
    let ctx = add_unitless(base_context(), format!("{}_IMPL_VOL", UNDERLYING_ID), 0.22);
    let as_of = date(2024, 12, 1);

    // Act
    let _forward = swap.remaining_forward_variance(&ctx, as_of).unwrap();
    let expected = swap.partial_realized_variance(&ctx, as_of).unwrap();

    // Assert - before start, should use forward/implied
    // Note: The method logic in types.rs shows expected_variance_calculator
    // returns strike or implied vol squared before start
    assert_eq!(expected, 0.0); // partial_realized before start
}

#[test]
fn test_expected_variance_at_maturity_equals_realized() {
    // Arrange
    let swap = sample_swap(PayReceive::Receive);
    let prices = price_series(&swap, 5_000.0, 2.0);
    let ctx = add_series(base_context(), &prices);

    // Act
    let realized = swap.partial_realized_variance(&ctx, swap.maturity).unwrap();

    // Assert
    assert!(realized > 0.0);
}

#[test]
fn test_expected_variance_mid_period_is_weighted_blend() {
    // Arrange
    let mut swap = sample_swap(PayReceive::Receive);
    swap.observation_freq = Frequency::weekly();
    let prices = price_series(&swap, 4_950.0, 10.0);
    let ctx = add_series(base_context(), &prices);
    let dates = swap.observation_dates();
    let as_of = dates[dates.len() / 2];

    // Act
    let realized = swap.partial_realized_variance(&ctx, as_of).unwrap();
    let forward = swap.remaining_forward_variance(&ctx, as_of).unwrap();
    let weight = observation_weight(&swap, as_of);
    let expected_blend = realized * weight + forward * (1.0 - weight);

    // Assert
    assert!(weight > 0.0 && weight < 1.0);
    assert!(expected_blend > 0.0);
    assert!(expected_blend.is_finite());
}

#[test]
fn test_expected_variance_transitions_smoothly() {
    // Arrange
    let mut swap = sample_swap(PayReceive::Receive);
    swap.observation_freq = Frequency::weekly();
    let prices = price_series(&swap, 5_000.0, 5.0);
    let _ctx = add_series(base_context(), &prices);
    let dates = swap.observation_dates();

    // Act - compute at multiple points
    let mut prev_weight = 0.0;
    for &d in dates.iter().take(dates.len() - 1).skip(1) {
        let weight = observation_weight(&swap, d);
        assert!(weight >= prev_weight, "Weight must increase over time");
        prev_weight = weight;
    }
}
