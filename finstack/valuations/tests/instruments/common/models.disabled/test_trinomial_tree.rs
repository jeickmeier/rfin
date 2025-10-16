//! Comprehensive tests for trinomial tree pricing models.
//!
//! Tests organized by:
//! - Parameter calculation and validation
//! - European option pricing and convergence
//! - American option pricing and early exercise
//! - Greeks calculations
//! - Tree type comparisons (Standard vs Boyle)
//! - Edge cases and numerical stability

use finstack_core::market_data::MarketContext;
use finstack_core::{Error, Result};
use finstack_valuations::instruments::common::models::tree_framework::*;
use finstack_valuations::instruments::common::models::{TrinomialTree, TrinomialTreeType};
use std::collections::HashSet;

use super::super::test_helpers::*;

// ============================================================================
// Test Valuators
// ============================================================================

/// European call option valuator
struct EuropeanCallValuator {
    strike: f64,
}

impl TreeValuator for EuropeanCallValuator {
    fn value_at_maturity(&self, state: &NodeState) -> Result<f64> {
        let spot = state.spot().ok_or(Error::Internal)?;
        Ok((spot - self.strike).max(0.0))
    }

    fn value_at_node(&self, _state: &NodeState, continuation_value: f64) -> Result<f64> {
        Ok(continuation_value)
    }
}

/// European put option valuator
struct EuropeanPutValuator {
    strike: f64,
}

impl TreeValuator for EuropeanPutValuator {
    fn value_at_maturity(&self, state: &NodeState) -> Result<f64> {
        let spot = state.spot().ok_or(Error::Internal)?;
        Ok((self.strike - spot).max(0.0))
    }

    fn value_at_node(&self, _state: &NodeState, continuation_value: f64) -> Result<f64> {
        Ok(continuation_value)
    }
}

/// American call option valuator
struct AmericanCallValuator {
    strike: f64,
}

impl TreeValuator for AmericanCallValuator {
    fn value_at_maturity(&self, state: &NodeState) -> Result<f64> {
        let spot = state.spot().ok_or(Error::Internal)?;
        Ok((spot - self.strike).max(0.0))
    }

    fn value_at_node(&self, state: &NodeState, continuation_value: f64) -> Result<f64> {
        let spot = state.spot().ok_or(Error::Internal)?;
        let intrinsic = (spot - self.strike).max(0.0);
        Ok(continuation_value.max(intrinsic))
    }
}

/// American put option valuator
struct AmericanPutValuator {
    strike: f64,
}

impl TreeValuator for AmericanPutValuator {
    fn value_at_maturity(&self, state: &NodeState) -> Result<f64> {
        let spot = state.spot().ok_or(Error::Internal)?;
        Ok((self.strike - spot).max(0.0))
    }

    fn value_at_node(&self, state: &NodeState, continuation_value: f64) -> Result<f64> {
        let spot = state.spot().ok_or(Error::Internal)?;
        let intrinsic = (self.strike - spot).max(0.0);
        Ok(continuation_value.max(intrinsic))
    }
}

/// Bermudan put option valuator
struct BermudanPutValuator {
    strike: f64,
    exercise_steps: HashSet<usize>,
}

impl TreeValuator for BermudanPutValuator {
    fn value_at_maturity(&self, state: &NodeState) -> Result<f64> {
        let spot = state.spot().ok_or(Error::Internal)?;
        Ok((self.strike - spot).max(0.0))
    }

    fn value_at_node(&self, state: &NodeState, continuation_value: f64) -> Result<f64> {
        let spot = state.spot().ok_or(Error::Internal)?;
        let intrinsic = (self.strike - spot).max(0.0);

        // Can only exercise at specific steps
        if self.exercise_steps.contains(&state.step) {
            Ok(continuation_value.max(intrinsic))
        } else {
            Ok(continuation_value)
        }
    }
}

// ============================================================================
// Parameter Calculation Tests
// ============================================================================

#[test]
fn test_standard_parameters_basic() {
    // Arrange
    let tree = TrinomialTree::standard(100);
    let r = 0.05;
    let sigma = 0.20;
    let t = 1.0;
    let q = 0.0;

    // Act
    let params = tree.calculate_parameters(r, sigma, t, q);

    // Assert
    assert!(params.is_ok());
    let (u, d, m, p_u, p_d, p_m) = params.unwrap();

    // Trinomial constraints
    assert!(u > 1.0, "Up factor should be > 1");
    assert!(d < 1.0, "Down factor should be < 1");
    assert!(d > 0.0, "Down factor should be positive");
    assert_approx_eq(m, 1.0, TOLERANCE, "Middle factor should be 1");

    // Probability constraints
    assert!(p_u >= 0.0 && p_u <= 1.0, "Up probability in [0,1]");
    assert!(p_d >= 0.0 && p_d <= 1.0, "Down probability in [0,1]");
    assert!(p_m >= 0.0 && p_m <= 1.0, "Middle probability in [0,1]");
    assert_approx_eq(
        p_u + p_d + p_m,
        1.0,
        TIGHT_TOLERANCE,
        "Probabilities sum to 1",
    );

    // Recombining property
    assert_approx_eq(u * d, 1.0, TOLERANCE, "Trinomial recombining property");
}

#[test]
fn test_boyle_parameters_basic() {
    // Arrange
    let tree = TrinomialTree::boyle(100);
    let r = 0.05;
    let sigma = 0.20;
    let t = 1.0;
    let q = 0.0;

    // Act
    let params = tree.calculate_parameters(r, sigma, t, q);

    // Assert
    assert!(params.is_ok());
    let (u, d, m, p_u, p_d, p_m) = params.unwrap();

    assert!(u > 1.0 && d < 1.0, "Valid up/down factors");
    assert_approx_eq(m, 1.0, TOLERANCE, "Middle factor is 1");
    assert_approx_eq(
        p_u + p_d + p_m,
        1.0,
        TIGHT_TOLERANCE,
        "Probabilities sum to 1",
    );
}

#[test]
fn test_parameters_short_maturity() {
    // Arrange: Very short maturity
    let tree = TrinomialTree::standard(10);
    let t = 1.0 / 365.0; // 1 day

    // Act
    let params = tree.calculate_parameters(0.05, 0.20, t, 0.0);

    // Assert
    assert!(params.is_ok());
    let (u, d, m, p_u, p_d, p_m) = params.unwrap();

    // Should have small moves for short maturity
    assert!(u < 1.01, "Small up move for short maturity");
    assert!(d > 0.99, "Small down move for short maturity");
    assert_approx_eq(m, 1.0, TOLERANCE, "Middle factor is 1");
    assert!(p_m > 0.5, "High middle probability for short maturity");
}

#[test]
fn test_parameters_high_volatility() {
    // Arrange: High volatility scenario
    let tree = TrinomialTree::standard(50);
    let sigma = 0.80; // 80% vol

    // Act
    let params = tree.calculate_parameters(0.05, sigma, 1.0, 0.0);

    // Assert
    assert!(params.is_ok());
    let (u, d, m, p_u, p_d, p_m) = params.unwrap();

    // Higher vol means larger moves
    assert!(u > 1.5, "Large up move for high vol");
    assert!(d < 0.7, "Large down move for high vol");
    assert_approx_eq(p_u + p_d + p_m, 1.0, TIGHT_TOLERANCE, "Probs sum to 1");
}

#[test]
fn test_parameters_with_dividend() {
    // Arrange
    let tree = TrinomialTree::standard(100);
    let r = 0.05;
    let q = 0.02; // 2% dividend yield

    // Act
    let params_no_div = tree.calculate_parameters(r, 0.20, 1.0, 0.0);
    let params_with_div = tree.calculate_parameters(r, 0.20, 1.0, q);

    // Assert
    assert!(params_no_div.is_ok());
    assert!(params_with_div.is_ok());

    let (_, _, _, p_u_no_div, p_d_no_div, _) = params_no_div.unwrap();
    let (_, _, _, p_u_div, p_d_div, _) = params_with_div.unwrap();

    // With dividends, drift is lower, so down probability should be higher
    assert!(p_d_div > p_d_no_div, "Higher down prob with dividends");
    assert!(p_u_div < p_u_no_div, "Lower up prob with dividends");
}

#[test]
fn test_parameters_invalid_inputs() {
    // Arrange
    let tree = TrinomialTree::standard(50);

    // Act & Assert: Zero or negative time
    assert!(tree.calculate_parameters(0.05, 0.20, 0.0, 0.0).is_err());
    assert!(tree.calculate_parameters(0.05, 0.20, -1.0, 0.0).is_err());

    // Act & Assert: Zero or negative volatility
    assert!(tree.calculate_parameters(0.05, 0.0, 1.0, 0.0).is_err());
    assert!(tree.calculate_parameters(0.05, -0.20, 1.0, 0.0).is_err());
}

// ============================================================================
// European Option Pricing Tests
// ============================================================================

#[test]
fn test_european_call_atm_convergence() {
    // Arrange
    let ctx = MarketContext::new();
    let bs_value = black_scholes_call(100.0, 100.0, 0.05, 0.20, 1.0, 0.0);

    // Act: Price with increasing steps
    let tree_50 = TrinomialTree::standard(50);
    let tree_100 = TrinomialTree::standard(100);
    let tree_200 = TrinomialTree::standard(200);

    let vars = single_factor_equity_state(100.0, 0.05, 0.0, 0.20);
    let valuator = EuropeanCallValuator { strike: 100.0 };

    let price_50 = tree_50.price(vars.clone(), 1.0, &ctx, &valuator).unwrap();
    let price_100 = tree_100.price(vars.clone(), 1.0, &ctx, &valuator).unwrap();
    let price_200 = tree_200.price(vars, 1.0, &ctx, &valuator).unwrap();

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
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(100.0, 0.05, 0.0, 0.20);
    let bs_value = black_scholes_put(100.0, 100.0, 0.05, 0.20, 1.0, 0.0);

    // Act
    let tree = TrinomialTree::standard(200);
    let valuator = EuropeanPutValuator { strike: 100.0 };
    let tree_price = tree.price(vars, 1.0, &ctx, &valuator).unwrap();

    // Assert
    assert_relative_eq(tree_price, bs_value, 0.01, "Put price within 1% of BS");
}

#[test]
fn test_european_call_itm() {
    // Arrange: In-the-money call (spot > strike)
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(110.0, 0.05, 0.0, 0.20);
    let bs_value = black_scholes_call(110.0, 100.0, 0.05, 0.20, 1.0, 0.0);

    // Act
    let tree = TrinomialTree::standard(150);
    let valuator = EuropeanCallValuator { strike: 100.0 };
    let tree_price = tree.price(vars, 1.0, &ctx, &valuator).unwrap();

    // Assert
    assert!(tree_price > 10.0, "ITM call should have intrinsic value");
    assert_relative_eq(tree_price, bs_value, 0.01, "ITM call within 1% of BS");
}

#[test]
fn test_european_put_itm() {
    // Arrange: In-the-money put (spot < strike)
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(90.0, 0.05, 0.0, 0.20);
    let bs_value = black_scholes_put(90.0, 100.0, 0.05, 0.20, 1.0, 0.0);

    // Act
    let tree = TrinomialTree::standard(150);
    let valuator = EuropeanPutValuator { strike: 100.0 };
    let tree_price = tree.price(vars, 1.0, &ctx, &valuator).unwrap();

    // Assert
    assert!(tree_price > 10.0, "ITM put should have intrinsic value");
    assert_relative_eq(tree_price, bs_value, 0.01, "ITM put within 1% of BS");
}

#[test]
fn test_european_call_otm() {
    // Arrange: Out-of-the-money call (spot < strike)
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(90.0, 0.05, 0.0, 0.20);
    let bs_value = black_scholes_call(90.0, 100.0, 0.05, 0.20, 1.0, 0.0);

    // Act
    let tree = TrinomialTree::standard(150);
    let valuator = EuropeanCallValuator { strike: 100.0 };
    let tree_price = tree.price(vars, 1.0, &ctx, &valuator).unwrap();

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

    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(spot, rate, 0.0, vol);

    // Act
    let tree = TrinomialTree::standard(200);
    let call_valuator = EuropeanCallValuator { strike };
    let put_valuator = EuropeanPutValuator { strike };

    let call_price = tree
        .price(vars.clone(), time, &ctx, &call_valuator)
        .unwrap();
    let put_price = tree.price(vars, time, &ctx, &put_valuator).unwrap();

    // Assert: C - P = S - K*e^(-rT)
    let lhs = call_price - put_price;
    let rhs = spot - strike * (-rate * time).exp() as f64;

    assert_approx_eq(lhs, rhs, 0.1, "Put-call parity");
}

// ============================================================================
// American Option Pricing Tests
// ============================================================================

#[test]
fn test_american_put_early_exercise_premium() {
    // Arrange: Deep ITM put with high rates (favors early exercise)
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(80.0, 0.10, 0.0, 0.20);

    // Act
    let tree = TrinomialTree::standard(150);
    let american_valuator = AmericanPutValuator { strike: 100.0 };
    let european_valuator = EuropeanPutValuator { strike: 100.0 };

    let american = tree
        .price(vars.clone(), 1.0, &ctx, &american_valuator)
        .unwrap();
    let european = tree.price(vars, 1.0, &ctx, &european_valuator).unwrap();

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
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(100.0, 0.05, 0.0, 0.20);

    // Act
    let tree = TrinomialTree::standard(150);
    let american_valuator = AmericanCallValuator { strike: 100.0 };
    let european_valuator = EuropeanCallValuator { strike: 100.0 };

    let american = tree
        .price(vars.clone(), 1.0, &ctx, &american_valuator)
        .unwrap();
    let european = tree.price(vars, 1.0, &ctx, &european_valuator).unwrap();

    // Assert: No early exercise optimal for call without dividends
    assert_approx_eq(american, european, 0.1, "American call ~ European (no div)");
}

#[test]
fn test_american_put_bounds() {
    // Arrange
    let spot = 90.0;
    let strike = 100.0;
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(spot, 0.05, 0.0, 0.20);

    // Act
    let tree = TrinomialTree::standard(100);
    let valuator = AmericanPutValuator { strike };
    let american = tree.price(vars, 1.0, &ctx, &valuator).unwrap();

    // Assert: Value bounds
    let intrinsic = strike - spot;
    assert!(american >= intrinsic, "American >= intrinsic value");
    assert!(american <= strike, "American <= strike (max payoff)");
}

#[test]
fn test_american_put_immediate_exercise() {
    // Arrange: Very deep ITM put
    let spot = 50.0;
    let strike = 100.0;
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(spot, 0.15, 0.0, 0.20);

    // Act
    let tree = TrinomialTree::standard(100);
    let valuator = AmericanPutValuator { strike };
    let american = tree.price(vars, 0.5, &ctx, &valuator).unwrap();

    // Assert: Should be very close to intrinsic (optimal to exercise immediately)
    let intrinsic = strike - spot;
    assert_relative_eq(american, intrinsic, 0.01, "Deep ITM ~ intrinsic");
}

// ============================================================================
// Bermudan Option Tests
// ============================================================================

#[test]
fn test_bermudan_between_european_and_american() {
    // Arrange
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(100.0, 0.05, 0.0, 0.20);
    let total_time = 1.0;
    let steps = 100;

    // Quarterly exercise: steps 25, 50, 75, 100
    let exercise_steps: HashSet<usize> = vec![25, 50, 75, 100].into_iter().collect();

    // Act
    let tree = TrinomialTree::standard(steps);
    let european_valuator = EuropeanPutValuator { strike: 110.0 };
    let bermudan_valuator = BermudanPutValuator {
        strike: 110.0,
        exercise_steps,
    };
    let american_valuator = AmericanPutValuator { strike: 110.0 };

    let european = tree
        .price(vars.clone(), total_time, &ctx, &european_valuator)
        .unwrap();
    let bermudan = tree
        .price(vars.clone(), total_time, &ctx, &bermudan_valuator)
        .unwrap();
    let american = tree
        .price(vars, total_time, &ctx, &american_valuator)
        .unwrap();

    // Assert
    assert!(bermudan >= european, "Bermudan >= European");
    assert!(bermudan <= american, "Bermudan <= American");
}

#[test]
fn test_bermudan_single_date_equals_european() {
    // Arrange
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(100.0, 0.05, 0.0, 0.20);
    let steps = 100;

    // Only exercise at maturity
    let exercise_steps: HashSet<usize> = vec![steps].into_iter().collect();

    // Act
    let tree = TrinomialTree::standard(steps);
    let bermudan_valuator = BermudanPutValuator {
        strike: 100.0,
        exercise_steps,
    };
    let european_valuator = EuropeanPutValuator { strike: 100.0 };

    let bermudan = tree
        .price(vars.clone(), 1.0, &ctx, &bermudan_valuator)
        .unwrap();
    let european = tree.price(vars, 1.0, &ctx, &european_valuator).unwrap();

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
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(100.0, 0.05, 0.0, 0.20);
    let steps = 50;

    // Exercise at every step
    let exercise_steps: HashSet<usize> = (0..=steps).collect();

    // Act
    let tree = TrinomialTree::standard(steps);
    let bermudan_valuator = BermudanPutValuator {
        strike: 110.0,
        exercise_steps,
    };
    let american_valuator = AmericanPutValuator { strike: 110.0 };

    let bermudan = tree
        .price(vars.clone(), 0.5, &ctx, &bermudan_valuator)
        .unwrap();
    let american = tree.price(vars, 0.5, &ctx, &american_valuator).unwrap();

    // Assert
    assert_approx_eq(
        bermudan,
        american,
        0.05,
        "Bermudan with all dates ~ American",
    );
}

// ============================================================================
// Greeks Calculation Tests
// ============================================================================

#[test]
fn test_greeks_call_delta_positive() {
    // Arrange
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(100.0, 0.05, 0.0, 0.20);
    let valuator = EuropeanCallValuator { strike: 100.0 };

    // Act
    let tree = TrinomialTree::standard(100);
    let greeks = tree
        .calculate_greeks(vars, 1.0, &ctx, &valuator, None)
        .unwrap();

    // Assert
    assert!(greeks.delta > 0.0, "Call delta is positive");
    assert!(greeks.delta < 1.0, "Call delta is less than 1");
    assert_approx_eq(greeks.delta, 0.5, 0.2, "ATM call delta ~ 0.5");
}

#[test]
fn test_greeks_put_delta_negative() {
    // Arrange
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(100.0, 0.05, 0.0, 0.20);
    let valuator = EuropeanPutValuator { strike: 100.0 };

    // Act
    let tree = TrinomialTree::standard(100);
    let greeks = tree
        .calculate_greeks(vars, 1.0, &ctx, &valuator, None)
        .unwrap();

    // Assert
    assert!(greeks.delta < 0.0, "Put delta is negative");
    assert!(greeks.delta > -1.0, "Put delta is greater than -1");
    assert_approx_eq(greeks.delta, -0.5, 0.2, "ATM put delta ~ -0.5");
}

#[test]
fn test_greeks_gamma_positive() {
    // Arrange: ATM option has highest gamma
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(100.0, 0.05, 0.0, 0.20);
    let valuator = EuropeanCallValuator { strike: 100.0 };

    // Act
    let tree = TrinomialTree::standard(100);
    let greeks = tree
        .calculate_greeks(vars, 1.0, &ctx, &valuator, None)
        .unwrap();

    // Assert
    assert!(greeks.gamma > 0.0, "Gamma is positive");
    assert!(greeks.gamma < 0.1, "Gamma is reasonable magnitude");
}

#[test]
fn test_greeks_vega_positive() {
    // Arrange
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(100.0, 0.05, 0.0, 0.20);
    let valuator = EuropeanCallValuator { strike: 100.0 };

    // Act
    let tree = TrinomialTree::standard(100);
    let greeks = tree
        .calculate_greeks(vars, 1.0, &ctx, &valuator, None)
        .unwrap();

    // Assert: Long options have positive vega
    assert!(greeks.vega > 0.0, "Long option vega is positive");
    assert!(greeks.vega < 100.0, "Vega is reasonable magnitude");
}

#[test]
fn test_greeks_theta_negative() {
    // Arrange
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(100.0, 0.05, 0.0, 0.20);
    let valuator = EuropeanCallValuator { strike: 100.0 };

    // Act
    let tree = TrinomialTree::standard(100);
    let greeks = tree
        .calculate_greeks(vars, 1.0, &ctx, &valuator, None)
        .unwrap();

    // Assert: Long option has negative theta (time decay)
    assert!(greeks.theta < 0.0, "Long option theta is negative");
}

#[test]
fn test_greeks_rho_call_positive() {
    // Arrange
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(100.0, 0.05, 0.0, 0.20);
    let valuator = EuropeanCallValuator { strike: 100.0 };

    // Act
    let tree = TrinomialTree::standard(100);
    let greeks = tree
        .calculate_greeks(vars, 1.0, &ctx, &valuator, None)
        .unwrap();

    // Assert: Long call has positive rho
    assert!(greeks.rho > 0.0, "Call rho is positive");
}

#[test]
fn test_greeks_consistency_with_price() {
    // Arrange
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(100.0, 0.05, 0.0, 0.20);
    let valuator = EuropeanCallValuator { strike: 100.0 };

    // Act
    let tree = TrinomialTree::standard(100);
    let greeks = tree
        .calculate_greeks(vars.clone(), 1.0, &ctx, &valuator, None)
        .unwrap();
    let price = tree.price(vars, 1.0, &ctx, &valuator).unwrap();

    // Assert
    assert_approx_eq(
        greeks.price,
        price,
        TOLERANCE,
        "Greeks price matches direct price",
    );
}

// ============================================================================
// Tree Type Comparison Tests
// ============================================================================

#[test]
fn test_standard_vs_boyle_convergence() {
    // Arrange
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(100.0, 0.05, 0.0, 0.20);
    let bs_value = black_scholes_call(100.0, 100.0, 0.05, 0.20, 1.0, 0.0);

    // Act: Compare Standard vs Boyle
    let standard = TrinomialTree::standard(200);
    let boyle = TrinomialTree::boyle(200);

    let valuator = EuropeanCallValuator { strike: 100.0 };
    let std_price = standard.price(vars.clone(), 1.0, &ctx, &valuator).unwrap();
    let boyle_price = boyle.price(vars, 1.0, &ctx, &valuator).unwrap();

    // Assert: Both should converge to BS
    let std_error = (std_price - bs_value).abs();
    let boyle_error = (boyle_price - bs_value).abs();

    assert!(std_error < 0.1, "Standard within 10 cents of BS");
    assert!(boyle_error < 0.1, "Boyle within 10 cents of BS");
}

#[test]
fn test_tree_type_parameter_differences() {
    // Arrange
    let standard = TrinomialTree::standard(100);
    let boyle = TrinomialTree::boyle(100);

    // Act
    let std_params = standard.calculate_parameters(0.05, 0.20, 1.0, 0.0).unwrap();
    let boyle_params = boyle.calculate_parameters(0.05, 0.20, 1.0, 0.0).unwrap();

    // Assert: Different parameterizations
    let (std_u, _, _, std_p_u, _, _) = std_params;
    let (boyle_u, _, _, boyle_p_u, _, _) = boyle_params;

    // Parameters should differ between models
    assert!((std_u - boyle_u).abs() > TOLERANCE, "Different up factors");
    assert!(
        (std_p_u - boyle_p_u).abs() > TOLERANCE,
        "Different probabilities"
    );
}

// ============================================================================
// Trinomial vs Binomial Comparison
// ============================================================================

#[test]
fn test_trinomial_vs_binomial_convergence() {
    use finstack_valuations::instruments::common::models::BinomialTree;

    // Arrange
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(100.0, 0.05, 0.0, 0.20);

    // Act
    let binomial = BinomialTree::crr(100);
    let trinomial = TrinomialTree::standard(100);

    let valuator = EuropeanCallValuator { strike: 100.0 };
    let bin_price = binomial.price(vars.clone(), 1.0, &ctx, &valuator).unwrap();
    let tri_price = trinomial.price(vars, 1.0, &ctx, &valuator).unwrap();

    // Assert: Should converge to similar values
    assert_relative_eq(tri_price, bin_price, 0.05, "Trinomial ~ Binomial");
}

#[test]
fn test_trinomial_better_convergence_fewer_steps() {
    // Arrange: Trinomial should give better accuracy with fewer steps
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(100.0, 0.05, 0.0, 0.20);
    let bs_value = black_scholes_call(100.0, 100.0, 0.05, 0.20, 1.0, 0.0);

    // Act: Compare 50 steps
    let trinomial = TrinomialTree::standard(50);
    let valuator = EuropeanCallValuator { strike: 100.0 };
    let tri_price = trinomial.price(vars, 1.0, &ctx, &valuator).unwrap();

    // Assert: Should be quite close to BS even with only 50 steps
    let error = (tri_price - bs_value).abs();
    assert!(error < 0.2, "Trinomial gives good accuracy with 50 steps");
}

// ============================================================================
// Edge Cases and Numerical Stability
// ============================================================================

#[test]
fn test_deep_otm_option() {
    // Arrange: Very deep OTM
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(50.0, 0.05, 0.0, 0.20);

    // Act
    let tree = TrinomialTree::standard(100);
    let valuator = EuropeanCallValuator { strike: 100.0 };
    let price = tree.price(vars, 0.25, &ctx, &valuator).unwrap();

    // Assert
    assert!(price >= 0.0, "Price is non-negative");
    assert!(price < 0.1, "Deep OTM has negligible value");
}

#[test]
fn test_deep_itm_option() {
    // Arrange: Very deep ITM put
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(50.0, 0.05, 0.0, 0.20);

    // Act
    let tree = TrinomialTree::standard(100);
    let valuator = EuropeanPutValuator { strike: 100.0 };
    let price = tree.price(vars, 1.0, &ctx, &valuator).unwrap();

    // Assert: Should be close to intrinsic value
    let intrinsic = 100.0 - 50.0;
    assert!(price >= intrinsic, "Price >= intrinsic");
    assert!(price <= 100.0, "Price <= max payoff");
}

#[test]
fn test_very_short_maturity() {
    // Arrange: 1 hour to expiry
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(100.0, 0.05, 0.0, 0.20);

    // Act
    let tree = TrinomialTree::standard(10);
    let valuator = EuropeanCallValuator { strike: 100.0 };
    let price = tree
        .price(vars, 1.0 / (365.0 * 24.0), &ctx, &valuator)
        .unwrap();

    // Assert: Should be close to intrinsic (near zero for ATM)
    assert!(price < 0.5, "Very short maturity has little time value");
}

#[test]
fn test_very_long_maturity() {
    // Arrange: 10 years
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(100.0, 0.05, 0.0, 0.20);

    // Act
    let tree = TrinomialTree::standard(200);
    let valuator = EuropeanCallValuator { strike: 100.0 };
    let price = tree.price(vars, 10.0, &ctx, &valuator).unwrap();

    // Assert
    assert!(price > 50.0, "Long maturity call has high value");
    assert!(price < 100.0, "But less than spot");
}

#[test]
fn test_extreme_volatility() {
    // Arrange: 200% volatility
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(100.0, 0.05, 0.0, 2.0);

    // Act
    let tree = TrinomialTree::standard(150);
    let valuator = EuropeanCallValuator { strike: 100.0 };
    let price = tree.price(vars, 1.0, &ctx, &valuator).unwrap();

    // Assert
    assert!(price.is_finite(), "Price is finite even with extreme vol");
    assert!(price > 50.0, "High vol creates high option value");
}

#[test]
fn test_low_volatility() {
    // Arrange: Very low vol
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(105.0, 0.05, 0.0, 0.001);

    // Act
    let tree = TrinomialTree::standard(50);
    let valuator = EuropeanCallValuator { strike: 100.0 };
    let price = tree.price(vars, 1.0, &ctx, &valuator).unwrap();

    // Assert: Should be close to discounted intrinsic
    let intrinsic = 5.0;
    let discounted = intrinsic * (-0.05 * 1.0).exp() as f64;
    assert_relative_eq(price, discounted, 0.1, "Low vol ~ discounted intrinsic");
}

#[test]
fn test_high_dividend_yield() {
    // Arrange: High dividend yield (10%)
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(100.0, 0.05, 0.10, 0.20);

    // Act
    let tree = TrinomialTree::standard(100);
    let call_valuator = EuropeanCallValuator { strike: 100.0 };
    let put_valuator = EuropeanPutValuator { strike: 100.0 };

    let call_price = tree.price(vars.clone(), 1.0, &ctx, &call_valuator).unwrap();
    let put_price = tree.price(vars, 1.0, &ctx, &put_valuator).unwrap();

    // Assert: High dividends reduce call value, increase put value
    let bs_call = black_scholes_call(100.0, 100.0, 0.05, 0.20, 1.0, 0.10);
    let bs_put = black_scholes_put(100.0, 100.0, 0.05, 0.20, 1.0, 0.10);

    assert_relative_eq(call_price, bs_call, 0.05, "Call with dividends");
    assert_relative_eq(put_price, bs_put, 0.05, "Put with dividends");
}

#[test]
fn test_zero_strike_call() {
    // Arrange: Call with zero strike (should be worth discounted spot)
    let ctx = MarketContext::new();
    let spot = 100.0;
    let rate = 0.05;
    let time = 1.0;
    let vars = single_factor_equity_state(spot, rate, 0.0, 0.20);

    // Act
    let tree = TrinomialTree::standard(50);
    let valuator = EuropeanCallValuator { strike: 0.0 };
    let price = tree.price(vars, time, &ctx, &valuator).unwrap();

    // Assert: Should equal discounted spot
    let expected = spot * ((-rate * time).exp() as f64);
    assert_relative_eq(price, expected, 0.01, "Zero strike call ~ discounted spot");
}

#[test]
fn test_high_interest_rate() {
    // Arrange: Very high interest rate (50%)
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(100.0, 0.50, 0.0, 0.20);

    // Act
    let tree = TrinomialTree::standard(100);
    let valuator = EuropeanCallValuator { strike: 100.0 };
    let price = tree.price(vars, 1.0, &ctx, &valuator).unwrap();

    // Assert
    assert!(price.is_finite(), "Price is finite with high rates");
    assert!(price > 0.0, "Positive price");
}

#[test]
fn test_minimal_steps() {
    // Arrange: Very few steps (edge case)
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(100.0, 0.05, 0.0, 0.20);

    // Act
    let tree = TrinomialTree::standard(5);
    let valuator = EuropeanCallValuator { strike: 100.0 };
    let price = tree.price(vars, 1.0, &ctx, &valuator).unwrap();

    // Assert: Should still produce valid result
    assert!(price > 0.0, "Valid price with minimal steps");
    assert!(price < 100.0, "Reasonable bounds");
}

#[test]
fn test_many_steps_stability() {
    // Arrange: Many steps (test for numerical stability)
    let ctx = MarketContext::new();
    let vars = single_factor_equity_state(100.0, 0.05, 0.0, 0.20);
    let bs_value = black_scholes_call(100.0, 100.0, 0.05, 0.20, 1.0, 0.0);

    // Act
    let tree = TrinomialTree::standard(500);
    let valuator = EuropeanCallValuator { strike: 100.0 };
    let price = tree.price(vars, 1.0, &ctx, &valuator).unwrap();

    // Assert: Should be very close to BS with many steps
    assert!(price.is_finite(), "Price is finite");
    assert_relative_eq(price, bs_value, 0.005, "Very close to BS with many steps");
}
