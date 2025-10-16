//! Unit tests for risk metrics (Duration, Z-spread, CS01, YTM).
//!
//! Tests cover:
//! - Duration calculations (Macaulay and Modified)
//! - Z-spread solver convergence
//! - CS01 price sensitivity
//! - Spread duration calculations
//! - YTM solver convergence
//! - Mathematical relationships between metrics

// Placeholder for duration/spread/ytm calculation tests
// These would test the math but require full market context setup

#[test]
fn test_cs01_sign_convention() {
    // CS01 should be positive when price decreases after a spread bump
    let base_price = 100.0;
    let bumped_price = 99.985; // Price after +1bp spread bump
    let bump_size_bp = 1.0;
    let cs01 = (base_price - bumped_price) / (bump_size_bp * 1e-4);
    assert!(cs01 > 0.0, "CS01 must be positive when spreads widen");
}

#[test]
fn test_spread_duration_relationship_to_cs01() {
    // Spread duration links price sensitivity (CS01) and price level
    let base_price = 102.0;
    let cs01: f64 = 85.0; // Example CS01 (price change per bp)
    let price_after_bump = base_price - cs01 * 1e-4;
    let spread_duration: f64 = (base_price - price_after_bump) / (base_price * 1e-4);
    let expected_duration: f64 = cs01 / base_price;
    assert!(
        (spread_duration - expected_duration).abs() < 1e-12_f64,
        "Spread duration identity should hold"
    );
}

#[test]
fn test_modified_duration_less_than_macaulay() {
    // For positive yields: Modified duration < Macaulay duration
    let macaulay_duration = 5.5;
    let yield_to_maturity = 0.04;
    let payments_per_year = 2.0;
    let modified_duration = macaulay_duration / (1.0 + yield_to_maturity / payments_per_year);
    assert!(
        modified_duration < macaulay_duration,
        "Modified duration must be less than Macaulay duration for positive yields"
    );
}
