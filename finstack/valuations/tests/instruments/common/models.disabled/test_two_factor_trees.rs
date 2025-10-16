//! Tests for two-factor tree models.

use finstack_core::market_data::MarketContext;
use finstack_core::Result;
use finstack_valuations::instruments::common::models::tree_framework::*;
use finstack_valuations::instruments::common::models::two_factor_binomial::TwoFactorBinomialTree;
use finstack_valuations::instruments::common::models::BinomialTree;

use super::super::test_helpers::*;

struct TestCallValuator {
    strike: f64,
}

impl TreeValuator for TestCallValuator {
    fn value_at_maturity(&self, state: &NodeState) -> Result<f64> {
        let s = state.spot().unwrap_or(0.0);
        Ok((s - self.strike).max(0.0))
    }

    fn value_at_node(&self, _state: &NodeState, continuation_value: f64) -> Result<f64> {
        Ok(continuation_value)
    }
}

#[test]
fn test_two_factor_basic_sanity() {
    // Arrange
    let tree = TwoFactorBinomialTree::equity_and_rates(50, 0.2, 0.0, 0.0, 0.05, 0.0);
    let ctx = MarketContext::new();
    let initial = single_factor_equity_state(100.0, 0.05, 0.0, 0.2);
    let val = TestCallValuator { strike: 100.0 };

    // Act
    let price = tree.price(initial, 1.0, &ctx, &val);

    // Assert
    assert!(price.is_ok());
    assert!(price.unwrap().is_finite() && price.unwrap() > 0.0);
}

#[test]
fn test_two_factor_matches_one_factor_when_rate_vol_zero() {
    // Arrange
    let steps = 75;
    let t = 1.0;
    let ctx = MarketContext::new();
    let initial = single_factor_equity_state(100.0, 0.05, 0.0, 0.2);
    let val = TestCallValuator { strike: 100.0 };

    // Act
    let one_factor = BinomialTree::crr(steps)
        .price(initial.clone(), t, &ctx, &val)
        .unwrap();
    let two_factor = TwoFactorBinomialTree::equity_and_rates(steps, 0.2, 0.0, 0.0, 0.05, 0.0)
        .price(initial, t, &ctx, &val)
        .unwrap();

    // Assert: Should be close when rate volatility is zero
    assert_approx_eq(one_factor, two_factor, 0.1, "Match when rate vol = 0");
}

#[test]
fn test_correlation_impact() {
    // Arrange
    let steps = 50;
    let ctx = MarketContext::new();
    let initial = single_factor_equity_state(100.0, 0.05, 0.0, 0.2);
    let val = TestCallValuator { strike: 100.0 };

    // Act: Price with different correlations
    let no_corr = TwoFactorBinomialTree::equity_and_rates(steps, 0.2, 0.0, 0.015, 0.05, 0.0)
        .price(initial.clone(), 1.0, &ctx, &val)
        .unwrap();

    let pos_corr = TwoFactorBinomialTree::equity_and_rates(steps, 0.2, 0.0, 0.015, 0.05, 0.5)
        .price(initial.clone(), 1.0, &ctx, &val)
        .unwrap();

    let neg_corr = TwoFactorBinomialTree::equity_and_rates(steps, 0.2, 0.0, 0.015, 0.05, -0.5)
        .price(initial, 1.0, &ctx, &val)
        .unwrap();

    // Assert: Prices should differ
    assert!(
        (no_corr - pos_corr).abs() > 0.01 || (no_corr - neg_corr).abs() > 0.01,
        "Correlation affects pricing"
    );
}
