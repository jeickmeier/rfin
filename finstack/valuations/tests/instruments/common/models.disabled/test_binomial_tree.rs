//! Comprehensive tests for binomial tree pricing models.
//!
//! Tests organized by:
//! - Parameter calculation and validation
//! - European option pricing and convergence
//! - American option pricing and early exercise
//! - Bermudan option pricing
//! - Barrier options (knock-in, knock-out)
//! - Greeks calculations
//! - Edge cases and numerical stability

use finstack_valuations::instruments::common::models::{BinomialTree, TreeType};
use finstack_valuations::instruments::common::parameters::OptionMarketParams;
use finstack_valuations::instruments::ExerciseStyle;

use super::super::test_helpers::*;

// ============================================================================
// Parameter Calculation Tests
// ============================================================================

#[test]
fn test_crr_parameters_basic() {
    // Arrange
    let tree = BinomialTree::crr(100);
    let spot = 100.0;
    let strike = 100.0;
    let r = 0.05;
    let sigma = 0.20;
    let t = 1.0;
    let q = 0.0;

    // Act
    let params = tree.calculate_parameters(spot, strike, r, sigma, t, q);

    // Assert
    assert!(params.is_ok());
    let (u, d, p) = params.unwrap();

    // CRR constraints
    assert!(u > 1.0, "Up factor should be > 1");
    assert!(d < 1.0, "Down factor should be < 1");
    assert!(d > 0.0, "Down factor should be positive");
    assert!((0.0..=1.0).contains(&p), "Probability should be in [0,1]");
    assert_approx_eq(u * d, 1.0, TOLERANCE, "CRR recombining property");
}

#[test]
fn test_leisen_reimer_parameters_atm() {
    // Arrange
    let tree = BinomialTree::leisen_reimer(201); // Odd steps for LR
    let spot = 100.0;
    let strike = 100.0;
    let r = 0.05;
    let sigma = 0.20;
    let t = 1.0;
    let q = 0.02;

    // Act
    let params = tree.calculate_parameters(spot, strike, r, sigma, t, q);

    // Assert
    assert!(params.is_ok());
    let (u, d, p) = params.unwrap();

    assert!(u > 1.0 && d < 1.0, "Valid up/down factors");
    assert!((0.0..=1.0).contains(&p), "Valid probability");
}

#[test]
fn test_parameters_short_maturity() {
    // Arrange: Very short maturity
    let tree = BinomialTree::crr(10);
    let t = 1.0 / 365.0; // 1 day

    // Act
    let params = tree.calculate_parameters(100.0, 100.0, 0.05, 0.20, t, 0.0);

    // Assert
    assert!(params.is_ok());
    let (u, d, p) = params.unwrap();

    // Should have small moves for short maturity
    assert!(u < 1.01, "Small up move for short maturity");
    assert!(d > 0.99, "Small down move for short maturity");
}

#[test]
fn test_parameters_high_volatility() {
    // Arrange: High volatility scenario
    let tree = BinomialTree::crr(50);
    let sigma = 0.80; // 80% vol

    // Act
    let params = tree.calculate_parameters(100.0, 100.0, 0.05, sigma, 1.0, 0.0);

    // Assert
    assert!(params.is_ok());
    let (u, d, p) = params.unwrap();

    // Higher vol means larger moves
    assert!(u > 1.5, "Large up move for high vol");
    assert!(d < 0.7, "Large down move for high vol");
}

#[test]
fn test_parameters_invalid_inputs() {
    // Arrange
    let tree = BinomialTree::crr(50);

    // Act & Assert: Zero or negative time
    assert!(tree
        .calculate_parameters(100.0, 100.0, 0.05, 0.20, 0.0, 0.0)
        .is_err());
    assert!(tree
        .calculate_parameters(100.0, 100.0, 0.05, 0.20, -1.0, 0.0)
        .is_err());

    // Act & Assert: Zero or negative volatility
    assert!(tree
        .calculate_parameters(100.0, 100.0, 0.05, 0.0, 1.0, 0.0)
        .is_err());
    assert!(tree
        .calculate_parameters(100.0, 100.0, 0.05, -0.20, 1.0, 0.0)
        .is_err());
}

// ============================================================================
// European Option Pricing Tests
// ============================================================================

#[test]
fn test_european_call_atm_convergence() {
    // Arrange
    let market = OptionMarketParams::call(100.0, 100.0, 0.05, 0.20, 1.0);
    let bs_value = black_scholes_call(100.0, 100.0, 0.05, 0.20, 1.0, 0.0);

    // Act: Price with increasing steps
    let tree_50 = BinomialTree::crr(50);
    let tree_100 = BinomialTree::crr(100);
    let tree_200 = BinomialTree::crr(200);

    let price_50 = tree_50.price_european(&market).unwrap();
    let price_100 = tree_100.price_european(&market).unwrap();
    let price_200 = tree_200.price_european(&market).unwrap();

    // Assert: Convergence to Black-Scholes
    let error_50 = (price_50 - bs_value).abs();
    let error_100 = (price_100 - bs_value).abs();
    let error_200 = (price_200 - bs_value).abs();

    assert!(error_100 < error_50, "Error decreases with more steps");
    assert!(error_200 < error_100, "Error continues decreasing");
    assert!(error_200 < 0.1, "Final error < 10 cents");
}

#[test]
fn test_european_put_atm_convergence() {
    // Arrange
    let market = OptionMarketParams::put(100.0, 100.0, 0.05, 0.20, 1.0);
    let bs_value = black_scholes_put(100.0, 100.0, 0.05, 0.20, 1.0, 0.0);

    // Act
    let tree = BinomialTree::crr(200);
    let tree_price = tree.price_european(&market).unwrap();

    // Assert
    assert_relative_eq(tree_price, bs_value, 0.01, "Put price within 1% of BS");
}

#[test]
fn test_european_call_itm() {
    // Arrange: In-the-money call (spot > strike)
    let market = OptionMarketParams::call(110.0, 100.0, 0.05, 0.20, 1.0);
    let bs_value = black_scholes_call(110.0, 100.0, 0.05, 0.20, 1.0, 0.0);

    // Act
    let tree = BinomialTree::crr(150);
    let tree_price = tree.price_european(&market).unwrap();

    // Assert
    assert!(tree_price > 10.0, "ITM call should have intrinsic value");
    assert_relative_eq(tree_price, bs_value, 0.01, "ITM call within 1% of BS");
}

#[test]
fn test_european_put_itm() {
    // Arrange: In-the-money put (spot < strike)
    let market = OptionMarketParams::put(90.0, 100.0, 0.05, 0.20, 1.0);
    let bs_value = black_scholes_put(90.0, 100.0, 0.05, 0.20, 1.0, 0.0);

    // Act
    let tree = BinomialTree::crr(150);
    let tree_price = tree.price_european(&market).unwrap();

    // Assert
    assert!(tree_price > 10.0, "ITM put should have intrinsic value");
    assert_relative_eq(tree_price, bs_value, 0.01, "ITM put within 1% of BS");
}

#[test]
fn test_european_call_otm() {
    // Arrange: Out-of-the-money call (spot < strike)
    let market = OptionMarketParams::call(90.0, 100.0, 0.05, 0.20, 1.0);
    let bs_value = black_scholes_call(90.0, 100.0, 0.05, 0.20, 1.0, 0.0);

    // Act
    let tree = BinomialTree::crr(150);
    let tree_price = tree.price_european(&market).unwrap();

    // Assert
    assert!(tree_price < 5.0, "OTM call should have low value");
    assert!(tree_price > 0.0, "OTM call should have positive time value");
    assert_relative_eq(tree_price, bs_value, 0.02, "OTM call within 2% of BS");
}

#[test]
fn test_put_call_parity() {
    // Arrange
    let spot = 100.0;
    let strike = 100.0;
    let rate = 0.05;
    let vol = 0.20;
    let time = 1.0;

    let call_market = OptionMarketParams::call(spot, strike, rate, vol, time);
    let put_market = OptionMarketParams::put(spot, strike, rate, vol, time);

    // Act
    let tree = BinomialTree::crr(200);
    let call_price = tree.price_european(&call_market).unwrap();
    let put_price = tree.price_european(&put_market).unwrap();

    // Assert: C - P = S - K*e^(-rT)
    let lhs = call_price - put_price;
    let rhs = spot - strike * (-rate * time).exp() as f64;

    assert_approx_eq(lhs, rhs, 0.1, "Put-call parity");
}

#[test]
fn test_leisen_reimer_superior_convergence() {
    // Arrange
    let market = OptionMarketParams::call(100.0, 100.0, 0.05, 0.20, 1.0);
    let bs_value = black_scholes_call(100.0, 100.0, 0.05, 0.20, 1.0, 0.0);

    // Act: Compare CRR vs LR at same step count
    let crr = BinomialTree::crr(201);
    let lr = BinomialTree::leisen_reimer(201);

    let crr_price = crr.price_european(&market).unwrap();
    let lr_price = lr.price_european(&market).unwrap();

    // Assert: LR should be closer to BS
    let crr_error = (crr_price - bs_value).abs();
    let lr_error = (lr_price - bs_value).abs();

    assert!(lr_error < crr_error, "LR has better convergence");
    assert!(lr_error < 0.05, "LR within 5 cents of BS");
}

// ============================================================================
// American Option Pricing Tests
// ============================================================================

#[test]
fn test_american_put_early_exercise_premium() {
    // Arrange: Deep ITM put with high rates (favors early exercise)
    let market = OptionMarketParams::put(80.0, 100.0, 0.10, 0.20, 1.0);

    // Act
    let tree = BinomialTree::crr(150);
    let american = tree.price_american(&market).unwrap();
    let european = tree.price_european(&market).unwrap();

    // Assert
    assert!(
        american > european,
        "American put should have early exercise premium"
    );
    assert!(american - european > 0.01, "Premium should be meaningful");
}

#[test]
fn test_american_call_no_dividend_equals_european() {
    // Arrange: American call with no dividends (should equal European)
    let market = OptionMarketParams::call(100.0, 100.0, 0.05, 0.20, 1.0);

    // Act
    let tree = BinomialTree::crr(150);
    let american = tree.price_american(&market).unwrap();
    let european = tree.price_european(&market).unwrap();

    // Assert: No early exercise optimal for call without dividends
    assert_approx_eq(american, european, 0.1, "American call ~ European (no div)");
}

#[test]
fn test_american_put_bounds() {
    // Arrange
    let spot = 90.0;
    let strike = 100.0;
    let market = OptionMarketParams::put(spot, strike, 0.05, 0.20, 1.0);

    // Act
    let tree = BinomialTree::crr(100);
    let american = tree.price_american(&market).unwrap();

    // Assert: Value bounds
    let intrinsic = strike - spot;
    assert!(american >= intrinsic, "American >= intrinsic value");
    assert!(american <= strike, "American <= strike (max payoff)");
}

// ============================================================================
// Bermudan Option Tests
// ============================================================================

#[test]
fn test_bermudan_between_european_and_american() {
    // Arrange
    let market = OptionMarketParams::put(100.0, 110.0, 0.05, 0.20, 1.0);
    let exercise_dates = vec![0.25, 0.5, 0.75, 1.0]; // Quarterly

    // Act
    let tree = BinomialTree::leisen_reimer(100);
    let european = tree.price_european(&market).unwrap();
    let bermudan = tree.price_bermudan(&market, &exercise_dates).unwrap();
    let american = tree.price_american(&market).unwrap();

    // Assert
    assert!(bermudan >= european, "Bermudan >= European");
    assert!(bermudan <= american, "Bermudan <= American");
}

#[test]
fn test_bermudan_single_date_equals_european() {
    // Arrange
    let market = OptionMarketParams::call(100.0, 100.0, 0.05, 0.20, 1.0);
    let exercise_dates = vec![1.0]; // Only at maturity

    // Act
    let tree = BinomialTree::crr(100);
    let bermudan = tree.price_bermudan(&market, &exercise_dates).unwrap();
    let european = tree.price_european(&market).unwrap();

    // Assert
    assert_approx_eq(
        bermudan,
        european,
        0.05,
        "Bermudan with single date ~ European",
    );
}

#[test]
fn test_bermudan_all_dates_equals_american() {
    // Arrange
    let market = OptionMarketParams::put(100.0, 110.0, 0.05, 0.20, 0.5);
    let steps = 50;
    let tree = BinomialTree::crr(steps);

    // Generate exercise dates at every step
    let exercise_dates: Vec<f64> = (0..=steps)
        .map(|i| (i as f64 / steps as f64) * market.time_to_expiry)
        .collect();

    // Act
    let bermudan = tree.price_bermudan(&market, &exercise_dates).unwrap();
    let american = tree.price_american(&market).unwrap();

    // Assert
    assert_approx_eq(
        bermudan,
        american,
        0.05,
        "Bermudan with all dates ~ American",
    );
}

// ============================================================================
// Barrier Option Tests
// ============================================================================

#[test]
fn test_up_and_out_reduces_value() {
    // Arrange
    let market = OptionMarketParams::call(100.0, 100.0, 0.05, 0.20, 1.0);
    let barrier = 120.0; // Above spot

    // Act
    let tree = BinomialTree::crr(150);
    let vanilla = tree.price_european(&market).unwrap();
    let barrier_option = tree.price_up_and_out(&market, barrier, 0.0).unwrap();

    // Assert
    assert!(barrier_option <= vanilla, "Barrier reduces value");
    assert!(barrier_option > 0.0, "Still has positive value");
}

#[test]
fn test_down_and_out_reduces_value() {
    // Arrange
    let market = OptionMarketParams::put(100.0, 100.0, 0.05, 0.20, 1.0);
    let barrier = 80.0; // Below spot

    // Act
    let tree = BinomialTree::crr(150);
    let vanilla = tree.price_european(&market).unwrap();
    let barrier_option = tree.price_down_and_out(&market, barrier, 0.0).unwrap();

    // Assert
    assert!(barrier_option <= vanilla, "Barrier reduces value");
}

#[test]
fn test_barrier_in_out_parity() {
    // Arrange
    let market = OptionMarketParams::call(100.0, 100.0, 0.03, 0.25, 0.5);
    let barrier = 110.0;
    let rebate = 0.0;

    // Act
    let tree = BinomialTree::crr(200);
    let vanilla = tree.price_european(&market).unwrap();
    let knock_out = tree.price_up_and_out(&market, barrier, rebate).unwrap();
    let knock_in = tree.price_up_and_in(&market, barrier, rebate).unwrap();

    // Assert: vanilla = knock_in + knock_out
    let parity_diff = (vanilla - (knock_in + knock_out)).abs();
    assert!(parity_diff < 0.01, "In-out parity holds");
}

#[test]
fn test_barrier_with_rebate() {
    // Arrange
    let market = OptionMarketParams::call(100.0, 100.0, 0.05, 0.20, 1.0);
    let barrier = 105.0; // Close to spot, likely to hit
    let rebate = 5.0;

    // Act
    let tree = BinomialTree::crr(150);
    let no_rebate = tree.price_up_and_out(&market, barrier, 0.0).unwrap();
    let with_rebate = tree.price_up_and_out(&market, barrier, rebate).unwrap();

    // Assert
    assert!(with_rebate > no_rebate, "Rebate increases value");
}

// ============================================================================
// Greeks Calculation Tests
// ============================================================================

#[test]
fn test_greeks_call_delta_positive() {
    // Arrange
    let market = OptionMarketParams::call(100.0, 100.0, 0.05, 0.20, 1.0);

    // Act
    let tree = BinomialTree::crr(100);
    let greeks = tree
        .calculate_greeks(&market, ExerciseStyle::European)
        .unwrap();

    // Assert
    assert!(greeks.delta > 0.0, "Call delta is positive");
    assert!(greeks.delta < 1.0, "Call delta is less than 1");
    assert_approx_eq(greeks.delta, 0.5, 0.2, "ATM call delta ~ 0.5");
}

#[test]
fn test_greeks_put_delta_negative() {
    // Arrange
    let market = OptionMarketParams::put(100.0, 100.0, 0.05, 0.20, 1.0);

    // Act
    let tree = BinomialTree::crr(100);
    let greeks = tree
        .calculate_greeks(&market, ExerciseStyle::European)
        .unwrap();

    // Assert
    assert!(greeks.delta < 0.0, "Put delta is negative");
    assert!(greeks.delta > -1.0, "Put delta is greater than -1");
    assert_approx_eq(greeks.delta, -0.5, 0.2, "ATM put delta ~ -0.5");
}

#[test]
fn test_greeks_gamma_positive() {
    // Arrange: ATM option has highest gamma
    let market = OptionMarketParams::call(100.0, 100.0, 0.05, 0.20, 1.0);

    // Act
    let tree = BinomialTree::crr(100);
    let greeks = tree
        .calculate_greeks(&market, ExerciseStyle::European)
        .unwrap();

    // Assert
    assert!(greeks.gamma > 0.0, "Gamma is positive");
    assert!(greeks.gamma < 0.1, "Gamma is reasonable magnitude");
}

#[test]
fn test_greeks_theta_negative() {
    // Arrange
    let market = OptionMarketParams::call(100.0, 100.0, 0.05, 0.20, 1.0);

    // Act
    let tree = BinomialTree::crr(100);
    let greeks = tree
        .calculate_greeks(&market, ExerciseStyle::European)
        .unwrap();

    // Assert: Long option has negative theta (time decay)
    assert!(greeks.theta < 0.0, "Long option theta is negative");
}

// ============================================================================
// Edge Cases and Numerical Stability
// ============================================================================

#[test]
fn test_deep_otm_option() {
    // Arrange: Very deep OTM
    let market = OptionMarketParams::call(50.0, 100.0, 0.05, 0.20, 0.25);

    // Act
    let tree = BinomialTree::crr(100);
    let price = tree.price_european(&market).unwrap();

    // Assert
    assert!(price >= 0.0, "Price is non-negative");
    assert!(price < 0.1, "Deep OTM has negligible value");
}

#[test]
fn test_deep_itm_option() {
    // Arrange: Very deep ITM put
    let market = OptionMarketParams::put(50.0, 100.0, 0.05, 0.20, 1.0);

    // Act
    let tree = BinomialTree::crr(100);
    let price = tree.price_european(&market).unwrap();

    // Assert: Should be close to intrinsic value
    let intrinsic = 100.0 - 50.0;
    assert!(price >= intrinsic, "Price >= intrinsic");
    assert!(price <= 100.0, "Price <= max payoff");
}

#[test]
fn test_very_short_maturity() {
    // Arrange: 1 hour to expiry
    let market = OptionMarketParams::call(100.0, 100.0, 0.05, 0.20, 1.0 / (365.0 * 24.0));

    // Act
    let tree = BinomialTree::crr(10);
    let price = tree.price_european(&market).unwrap();

    // Assert: Should be close to intrinsic (near zero for ATM)
    assert!(price < 0.5, "Very short maturity has little time value");
}

#[test]
fn test_very_long_maturity() {
    // Arrange: 10 years
    let market = OptionMarketParams::call(100.0, 100.0, 0.05, 0.20, 10.0);

    // Act
    let tree = BinomialTree::crr(200);
    let price = tree.price_european(&market).unwrap();

    // Assert
    assert!(price > 50.0, "Long maturity call has high value");
    assert!(price < 100.0, "But less than spot");
}

#[test]
fn test_extreme_volatility() {
    // Arrange: 200% volatility
    let market = OptionMarketParams::call(100.0, 100.0, 0.05, 2.0, 1.0);

    // Act
    let tree = BinomialTree::crr(150);
    let price = tree.price_european(&market).unwrap();

    // Assert
    assert!(price.is_finite(), "Price is finite even with extreme vol");
    assert!(price > 50.0, "High vol creates high option value");
}

#[test]
fn test_zero_volatility() {
    // Arrange: Zero vol should give deterministic payoff
    // This will fail with zero vol, so use very small vol
    let market = OptionMarketParams::call(105.0, 100.0, 0.05, 0.001, 1.0);

    // Act
    let tree = BinomialTree::crr(50);
    let price = tree.price_european(&market).unwrap();

    // Assert: Should be close to discounted intrinsic
    let intrinsic = 5.0;
    let discounted = intrinsic * (-0.05 * 1.0).exp() as f64;
    assert_relative_eq(price, discounted, 0.1, "Low vol ~ discounted intrinsic");
}
