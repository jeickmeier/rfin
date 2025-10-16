//! Tests for generic tree framework.

use finstack_core::market_data::MarketContext;
use finstack_core::Result;
use finstack_valuations::instruments::common::models::tree_framework::*;

use super::super::test_helpers::*;

// Test valuator for simple options
struct TestCallValuator {
    strike: f64,
}

impl TreeValuator for TestCallValuator {
    fn value_at_maturity(&self, state: &NodeState) -> Result<f64> {
        let spot = state.spot().unwrap_or(0.0);
        Ok((spot - self.strike).max(0.0))
    }

    fn value_at_node(&self, _state: &NodeState, continuation_value: f64) -> Result<f64> {
        Ok(continuation_value) // European style
    }
}

#[test]
fn test_state_variable_creation() {
    // Arrange
    let vars = single_factor_equity_state(100.0, 0.05, 0.02, 0.20);

    // Assert
    assert_approx_eq(
        *vars.get(state_keys::SPOT).unwrap(),
        100.0,
        TIGHT_TOLERANCE,
        "Spot",
    );
    assert_approx_eq(
        *vars.get(state_keys::INTEREST_RATE).unwrap(),
        0.05,
        TIGHT_TOLERANCE,
        "Rate",
    );
    assert_approx_eq(
        *vars.get(state_keys::DIVIDEND_YIELD).unwrap(),
        0.02,
        TIGHT_TOLERANCE,
        "Div",
    );
    assert_approx_eq(
        *vars.get(state_keys::VOLATILITY).unwrap(),
        0.20,
        TIGHT_TOLERANCE,
        "Vol",
    );
}

#[test]
fn test_two_factor_state_creation() {
    // Arrange
    let vars = two_factor_equity_rates_state(100.0, 0.05, 0.02, 0.20, 0.015);

    // Assert
    assert_approx_eq(
        *vars.get(state_keys::SPOT).unwrap(),
        100.0,
        TIGHT_TOLERANCE,
        "Spot",
    );
    assert_approx_eq(
        *vars.get("rate_volatility").unwrap(),
        0.015,
        TIGHT_TOLERANCE,
        "Rate vol",
    );
}

#[test]
fn test_node_state_basic() {
    // Arrange
    let ctx = MarketContext::new();
    let mut vars = single_factor_equity_state(100.0, 0.05, 0.02, 0.20);
    vars.insert(state_keys::SPOT, 105.0);

    // Act
    let state = NodeState::new(5, 0.5, vars, &ctx);

    // Assert
    assert_eq!(state.step, 5);
    assert_approx_eq(state.time, 0.5, TIGHT_TOLERANCE, "Time");
    assert_approx_eq(state.spot().unwrap(), 105.0, TIGHT_TOLERANCE, "Spot");
}

#[test]
fn test_map_exercise_dates_to_steps() {
    // Arrange
    let dates = vec![0.0, 0.25, 0.5, 0.75, 1.0];
    let total_time = 1.0;
    let steps = 4;

    // Act
    let step_indices = map_exercise_dates_to_steps(&dates, total_time, steps);

    // Assert
    assert_eq!(step_indices, vec![0, 1, 2, 3, 4]);
}

#[test]
fn test_map_irregular_dates() {
    // Arrange
    let dates = vec![0.12, 0.37, 0.62, 0.88];
    let steps = 4;

    // Act
    let step_indices = map_exercise_dates_to_steps(&dates, 1.0, steps);

    // Assert: Should round to nearest steps
    assert!(step_indices.iter().all(|&s| s <= steps));
}
