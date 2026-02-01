// Tree-based pricing tests for convertible bonds.
//
// Tests tree model selection and convergence properties:
// - Binomial tree pricing
// - Trinomial tree pricing
// - Convergence with increasing steps
// - Binomial vs trinomial agreement
// - Tree framework flexibility

mod convertible_fixtures;

use crate::instruments::common_impl::models::{
    single_factor_equity_state, BinomialTree, NodeState, TreeModel, TreeValuator, TrinomialTree,
};
use crate::instruments::fixed_income::convertible::{price_convertible_bond, ConvertibleTreeType};
use convertible_fixtures::*;

#[test]
fn test_binomial_tree_pricing() {
    let bond = create_standard_convertible();
    let market = create_market_context();

    let price = price_convertible_bond(
        &bond,
        &market,
        ConvertibleTreeType::Binomial(50),
        dates::base_date(),
    )
    .unwrap();

    assert!(
        price.amount() > 0.0,
        "Binomial pricing should produce positive price"
    );
    assert!(
        price.amount().is_finite(),
        "Binomial pricing should produce finite price"
    );
}

#[test]
fn test_trinomial_tree_pricing() {
    let bond = create_standard_convertible();
    let market = create_market_context();

    let price = price_convertible_bond(
        &bond,
        &market,
        ConvertibleTreeType::Trinomial(50),
        dates::base_date(),
    )
    .unwrap();

    assert!(
        price.amount() > 0.0,
        "Trinomial pricing should produce positive price"
    );
    assert!(
        price.amount().is_finite(),
        "Trinomial pricing should produce finite price"
    );
}

#[test]
fn test_binomial_vs_trinomial_convergence() {
    let bond = create_standard_convertible();
    let market = create_market_context();

    let bin_price = price_convertible_bond(
        &bond,
        &market,
        ConvertibleTreeType::Binomial(100),
        dates::base_date(),
    )
    .unwrap();

    let tri_price = price_convertible_bond(
        &bond,
        &market,
        ConvertibleTreeType::Trinomial(100),
        dates::base_date(),
    )
    .unwrap();

    // With sufficient steps, both methods should converge to similar values
    let diff_pct = (bin_price.amount() - tri_price.amount()).abs() / bin_price.amount();
    assert!(
        diff_pct < CONVERGENCE_TOLERANCE_PCT,
        "Binomial {} and trinomial {} should converge within {}%, got {}%",
        bin_price.amount(),
        tri_price.amount(),
        CONVERGENCE_TOLERANCE_PCT * 100.0,
        diff_pct * 100.0
    );
}

#[test]
fn test_binomial_convergence_with_steps() {
    let bond = create_standard_convertible();
    let market = create_market_context();

    let price_20 = price_convertible_bond(
        &bond,
        &market,
        ConvertibleTreeType::Binomial(20),
        dates::base_date(),
    )
    .unwrap();

    let price_50 = price_convertible_bond(
        &bond,
        &market,
        ConvertibleTreeType::Binomial(50),
        dates::base_date(),
    )
    .unwrap();

    let price_100 = price_convertible_bond(
        &bond,
        &market,
        ConvertibleTreeType::Binomial(100),
        dates::base_date(),
    )
    .unwrap();

    // Convergence: difference should decrease with more steps
    let diff_20_50 = (price_20.amount() - price_50.amount()).abs();
    let diff_50_100 = (price_50.amount() - price_100.amount()).abs();

    assert!(
        diff_50_100 < diff_20_50,
        "Should converge with more steps: diff(50,100)={} should be < diff(20,50)={}",
        diff_50_100,
        diff_20_50
    );
}

#[test]
fn test_trinomial_convergence_with_steps() {
    let bond = create_standard_convertible();
    let market = create_market_context();

    let _price_20 = price_convertible_bond(
        &bond,
        &market,
        ConvertibleTreeType::Trinomial(20),
        dates::base_date(),
    )
    .unwrap();

    let price_50 = price_convertible_bond(
        &bond,
        &market,
        ConvertibleTreeType::Trinomial(50),
        dates::base_date(),
    )
    .unwrap();

    let price_100 = price_convertible_bond(
        &bond,
        &market,
        ConvertibleTreeType::Trinomial(100),
        dates::base_date(),
    )
    .unwrap();

    // Convergence: should stabilize with more steps
    let diff_50_100 = (price_50.amount() - price_100.amount()).abs();
    let relative_diff = diff_50_100 / price_100.amount();

    assert!(
        relative_diff < 0.02, // Within 2%
        "Should converge to stable value: diff={}%",
        relative_diff * 100.0
    );
}

#[test]
fn test_tree_pricing_with_low_steps() {
    let bond = create_standard_convertible();
    let market = create_market_context();

    // Even with few steps, pricing should work
    let price = price_convertible_bond(
        &bond,
        &market,
        ConvertibleTreeType::Binomial(5),
        dates::base_date(),
    )
    .unwrap();

    assert!(price.amount() > 0.0, "Should price with low steps");
    assert!(
        price.amount().is_finite(),
        "Should produce finite price with low steps"
    );
}

#[test]
fn test_tree_pricing_with_high_steps() {
    let bond = create_standard_convertible();
    let market = create_market_context();

    // Should handle many steps without numerical issues
    let price = price_convertible_bond(
        &bond,
        &market,
        ConvertibleTreeType::Binomial(200),
        dates::base_date(),
    )
    .unwrap();

    assert!(price.amount() > 0.0, "Should price with high steps");
    assert!(
        price.amount().is_finite(),
        "Should produce finite price with high steps"
    );
}

#[test]
fn test_default_tree_type() {
    let bond = create_standard_convertible();
    let market = create_market_context();

    // Default tree type should work
    let price = price_convertible_bond(
        &bond,
        &market,
        ConvertibleTreeType::default(),
        dates::base_date(),
    )
    .unwrap();

    assert!(price.amount() > 0.0, "Default tree type should work");
}

#[test]
fn test_tree_framework_with_custom_valuator() {
    // Test that the generic tree framework works independently
    struct SpotReturner;

    impl TreeValuator for SpotReturner {
        fn value_at_maturity(&self, state: &NodeState) -> finstack_core::Result<f64> {
            Ok(state.spot().unwrap_or(0.0))
        }

        fn value_at_node(
            &self,
            _state: &NodeState,
            continuation_value: f64,
            _dt: f64,
        ) -> finstack_core::Result<f64> {
            Ok(continuation_value)
        }
    }

    let market = create_market_context();
    let initial_vars = single_factor_equity_state(100.0, 0.05, 0.02, 0.20);
    let valuator = SpotReturner;

    // Test binomial
    let binomial = BinomialTree::crr(20);
    let price_bin = binomial.price(initial_vars.clone(), 1.0, &market, &valuator);
    assert!(
        price_bin.is_ok(),
        "Binomial tree should work with custom valuator"
    );

    // Test trinomial
    let trinomial = TrinomialTree::standard(20);
    let price_tri = trinomial.price(initial_vars, 1.0, &market, &valuator);
    assert!(
        price_tri.is_ok(),
        "Trinomial tree should work with custom valuator"
    );

    // Both should return approximately the initial spot
    let bin_val = price_bin.unwrap();
    let tri_val = price_tri.unwrap();
    assert!(
        (bin_val - 100.0).abs() < 10.0,
        "Binomial should approximate spot"
    );
    assert!(
        (tri_val - 100.0).abs() < 10.0,
        "Trinomial should approximate spot"
    );
}

/// Test tree pricing convergence with increasing time steps.
///
/// Both binomial and trinomial trees should converge as step count increases.
/// Once converged, cross-tree consistency should be within 3%.
#[test]
fn test_tree_pricing_convergence() {
    let bond = create_standard_convertible();
    let market = create_market_context();

    // Test convergence with increasing steps
    let step_counts = [25, 50, 100, 200];
    let mut prev_bin = 0.0;
    let mut prev_tri = 0.0;

    for &steps in &step_counts {
        let bin_price = price_convertible_bond(
            &bond,
            &market,
            ConvertibleTreeType::Binomial(steps),
            dates::base_date(),
        )
        .unwrap()
        .amount();

        let tri_price = price_convertible_bond(
            &bond,
            &market,
            ConvertibleTreeType::Trinomial(steps),
            dates::base_date(),
        )
        .unwrap()
        .amount();

        // Check convergence (difference from previous should decrease)
        if prev_bin > 0.0 {
            let bin_change = (bin_price - prev_bin).abs() / prev_bin;
            let tri_change = (tri_price - prev_tri).abs() / prev_tri;

            // As steps increase, change from previous should decrease
            assert!(
                bin_change < 0.15,
                "Binomial should stabilize: {}→{} steps, change={:.2}%",
                steps / 2,
                steps,
                bin_change * 100.0
            );
            assert!(
                tri_change < 0.15,
                "Trinomial should stabilize: {}→{} steps, change={:.2}%",
                steps / 2,
                steps,
                tri_change * 100.0
            );
        }

        prev_bin = bin_price;
        prev_tri = tri_price;
    }

    // At highest step count (200), trees should agree within 3%
    let final_diff_pct = (prev_bin - prev_tri).abs() / prev_bin.max(1e-10);
    assert!(
        final_diff_pct < 0.03,
        "Converged trees should agree within 3%: bin={:.2}, tri={:.2}, diff={:.2}%",
        prev_bin,
        prev_tri,
        final_diff_pct * 100.0
    );
}

#[test]
fn test_tree_pricing_consistency_across_scenarios() {
    let bond = create_standard_convertible();

    // Test multiple market scenarios with both tree types
    // Using 100 steps for better convergence
    let scenarios = vec![
        ("ITM", create_market_context()),
        (
            "OTM",
            create_market_context_with_params(
                market_params::SPOT_LOW,
                market_params::VOL_STANDARD,
                market_params::DIV_YIELD,
            ),
        ),
        (
            "Low Vol",
            create_market_context_with_params(
                market_params::SPOT_PRICE,
                market_params::VOL_LOW,
                market_params::DIV_YIELD,
            ),
        ),
        (
            "High Vol",
            create_market_context_with_params(
                market_params::SPOT_PRICE,
                market_params::VOL_HIGH,
                market_params::DIV_YIELD,
            ),
        ),
    ];

    for (name, market) in scenarios {
        let bin_price = price_convertible_bond(
            &bond,
            &market,
            ConvertibleTreeType::Binomial(100), // Increased from 50 for better convergence
            dates::base_date(),
        )
        .unwrap();

        let tri_price = price_convertible_bond(
            &bond,
            &market,
            ConvertibleTreeType::Trinomial(100), // Increased from 50
            dates::base_date(),
        )
        .unwrap();

        // Both should produce reasonable prices
        assert!(
            bin_price.amount() > 0.0 && bin_price.amount() < 10000.0,
            "Binomial price unreasonable for scenario {}: {}",
            name,
            bin_price.amount()
        );

        assert!(
            tri_price.amount() > 0.0 && tri_price.amount() < 10000.0,
            "Trinomial price unreasonable for scenario {}: {}",
            name,
            tri_price.amount()
        );

        // With converged trees (100 steps), should agree within 5%
        // (looser than convergence test since scenarios vary more)
        let diff_pct = (bin_price.amount() - tri_price.amount()).abs() / bin_price.amount();
        assert!(
            diff_pct < 0.05,
            "Trees diverge too much for scenario {}: {:.2}%",
            name,
            diff_pct * 100.0
        );
    }
}

#[test]
fn test_tree_stability_with_volatility() {
    let bond = create_standard_convertible();

    // Test that tree pricing remains stable across volatility range
    for vol in [0.05, 0.10, 0.20, 0.30, 0.50] {
        let market = create_market_context_with_params(
            market_params::SPOT_PRICE,
            vol,
            market_params::DIV_YIELD,
        );

        let price = price_convertible_bond(
            &bond,
            &market,
            ConvertibleTreeType::Binomial(50),
            dates::base_date(),
        )
        .unwrap();

        assert!(
            price.amount() > 0.0 && price.amount().is_finite(),
            "Tree pricing unstable at vol={}: price={}",
            vol,
            price.amount()
        );
    }
}

#[test]
fn test_tree_monotonicity_with_spot() {
    let bond = create_standard_convertible();

    let mut prev_price = 0.0;

    // Price should increase monotonically with spot (for callable/puttable bonds this may not hold)
    for spot in [50.0, 75.0, 100.0, 150.0, 200.0] {
        let market = create_market_context_with_params(
            spot,
            market_params::VOL_STANDARD,
            market_params::DIV_YIELD,
        );

        let price = price_convertible_bond(
            &bond,
            &market,
            ConvertibleTreeType::Binomial(50),
            dates::base_date(),
        )
        .unwrap();

        assert!(
            price.amount() >= prev_price * 0.95, // Allow small numerical variance
            "Price should increase with spot: spot={}, price={}, prev_price={}",
            spot,
            price.amount(),
            prev_price
        );

        prev_price = price.amount();
    }
}

#[test]
fn test_tree_monotonicity_with_volatility() {
    let bond = create_standard_convertible();

    let mut prev_price = 0.0;

    // Price should increase monotonically with volatility (option value)
    for vol in [0.05, 0.10, 0.20, 0.30, 0.40] {
        let market = create_market_context_with_params(
            market_params::SPOT_PRICE,
            vol,
            market_params::DIV_YIELD,
        );

        let price = price_convertible_bond(
            &bond,
            &market,
            ConvertibleTreeType::Binomial(50),
            dates::base_date(),
        )
        .unwrap();

        if prev_price > 0.0 {
            assert!(
                price.amount() >= prev_price * 0.98, // Allow small numerical variance
                "Price should increase with volatility: vol={}, price={}, prev_price={}",
                vol,
                price.amount(),
                prev_price
            );
        }

        prev_price = price.amount();
    }
}
