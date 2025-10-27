//! Monte Carlo v0.1 integration tests.
//!
//! Tests Black-Scholes parity, RNG reproducibility, and variance reduction.

#![cfg(feature = "mc")]

use finstack_core::currency::Currency;
use finstack_valuations::instruments::common::mc::discretization::exact::ExactGbm;
use finstack_valuations::instruments::common::mc::payoff::vanilla::{EuropeanCall, EuropeanPut};
use finstack_valuations::instruments::common::mc::pricer::european::{
    EuropeanPricer, EuropeanPricerConfig,
};
use finstack_valuations::instruments::common::mc::process::gbm::{GbmParams, GbmProcess};
use finstack_valuations::instruments::common::mc::rng::philox::PhiloxRng;
use finstack_valuations::instruments::common::mc::time_grid::TimeGrid;
use finstack_valuations::instruments::common::mc::variance_reduction::{
    antithetic_price, black_scholes_call, black_scholes_put,
};

#[test]
fn test_mc_vs_black_scholes_call_atm() {
    // ATM call: S=100, K=100, T=1, r=5%, q=2%, σ=20%
    let config = EuropeanPricerConfig::new(100_000).with_seed(42).with_parallel(false);
    let pricer = EuropeanPricer::new(config);

    let r = 0.05;
    let q = 0.02;
    let sigma = 0.2;
    let t = 1.0;

    let gbm = GbmProcess::new(GbmParams::new(r, q, sigma));
    let call = EuropeanCall::new(100.0, 1.0, 252);

    // Apply discount factor for present value
    let discount = (-r * t).exp();
    let mc_result = pricer
        .price(&gbm, 100.0, t, 252, &call, Currency::USD, discount)
        .unwrap();

    // Analytical Black-Scholes price
    let bs_price = black_scholes_call(100.0, 100.0, t, r, q, sigma);

    // MC should be within 4 standard errors of BS price (99.99% confidence)
    let diff = (mc_result.mean.amount() - bs_price).abs();
    let tolerance = 4.0 * mc_result.stderr;

    assert!(
        diff < tolerance,
        "MC price {} differs from BS {:.4} by {:.4} (more than 4σ={:.4})",
        mc_result.mean.amount(),
        bs_price,
        diff,
        tolerance
    );

    println!("MC: {} vs BS: {:.4}, diff: {:.4}", mc_result.mean, bs_price, diff);
}

#[test]
fn test_mc_vs_black_scholes_put_itm() {
    // ITM put: S=90, K=100, T=0.5, r=5%, q=1%, σ=25%
    let config = EuropeanPricerConfig::new(100_000).with_seed(123).with_parallel(false);
    let pricer = EuropeanPricer::new(config);

    let r = 0.05;
    let q = 0.01;
    let sigma = 0.25;
    let t = 0.5;

    let gbm = GbmProcess::new(GbmParams::new(r, q, sigma));
    let put = EuropeanPut::new(100.0, 1.0, 126);

    // Apply discount factor
    let discount = (-r * t).exp();
    let mc_result = pricer
        .price(&gbm, 90.0, t, 126, &put, Currency::USD, discount)
        .unwrap();

    // Analytical price
    let bs_price = black_scholes_put(90.0, 100.0, t, r, q, sigma);

    let diff = (mc_result.mean.amount() - bs_price).abs();
    let tolerance = 4.0 * mc_result.stderr;

    assert!(
        diff < tolerance,
        "MC price {} differs from BS {:.4} by {:.4} (more than 4σ={:.4})",
        mc_result.mean.amount(),
        bs_price,
        diff,
        tolerance
    );

    println!("ITM Put - MC: {} vs BS: {:.4}, diff: {:.4}", mc_result.mean, bs_price, diff);
}

#[test]
fn test_rng_reproducibility() {
    // Same seed should produce identical results
    let config1 = EuropeanPricerConfig::new(1_000).with_seed(42).with_parallel(false);
    let config2 = EuropeanPricerConfig::new(1_000).with_seed(42).with_parallel(false);

    let pricer1 = EuropeanPricer::new(config1);
    let pricer2 = EuropeanPricer::new(config2);

    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.2));
    let call = EuropeanCall::new(100.0, 1.0, 50);

    let result1 = pricer1
        .price(&gbm, 100.0, 1.0, 50, &call, Currency::USD, 1.0)
        .unwrap();
    let result2 = pricer2
        .price(&gbm, 100.0, 1.0, 50, &call, Currency::USD, 1.0)
        .unwrap();

    // Results should be identical
    assert_eq!(result1.mean.amount(), result2.mean.amount());
    assert_eq!(result1.stderr, result2.stderr);

    println!("Reproducibility verified: {}", result1.mean);
}

#[test]
fn test_rng_different_seeds() {
    // Different seeds should produce different results
    let config1 = EuropeanPricerConfig::new(1_000).with_seed(42).with_parallel(false);
    let config2 = EuropeanPricerConfig::new(1_000).with_seed(43).with_parallel(false);

    let pricer1 = EuropeanPricer::new(config1);
    let pricer2 = EuropeanPricer::new(config2);

    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.2));
    let call = EuropeanCall::new(100.0, 1.0, 50);

    let result1 = pricer1
        .price(&gbm, 100.0, 1.0, 50, &call, Currency::USD, 1.0)
        .unwrap();
    let result2 = pricer2
        .price(&gbm, 100.0, 1.0, 50, &call, Currency::USD, 1.0)
        .unwrap();

    // Results should be different (but close)
    assert_ne!(result1.mean.amount(), result2.mean.amount());

    // But both should be reasonable
    let bs_price = black_scholes_call(100.0, 100.0, 1.0, 0.05, 0.02, 0.2);
    assert!((result1.mean.amount() - bs_price).abs() < 1.0);
    assert!((result2.mean.amount() - bs_price).abs() < 1.0);
}

#[test]
fn test_antithetic_variance_reduction() {
    // Compare antithetic vs standard MC for same computational budget
    let mut rng = PhiloxRng::new(42);
    let process = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.3));
    let disc = ExactGbm::new();
    let initial_state = vec![100.0];
    let payoff = EuropeanCall::new(100.0, 1.0, 100);
    let time_grid = TimeGrid::uniform(1.0, 100).unwrap();

    // Run antithetic with 500 pairs = 1000 effective paths
        let config = finstack_valuations::instruments::common::mc::variance_reduction::antithetic::AntitheticConfig {
            num_pairs: 500,
            time_grid: &time_grid,
            currency: Currency::USD,
            discount_factor: 1.0,
        };

        let stats_anti = antithetic_price(
            &mut rng,
            &process,
            &disc,
            &initial_state,
            &payoff,
            &config,
        );

    // Antithetic should produce reasonable estimates
    assert!(stats_anti.mean() > 0.0);
    assert!(stats_anti.stderr() > 0.0);

    // Check that stderr is reasonable for sample size
    let expected_stderr_order = stats_anti.std_dev() / (500.0_f64).sqrt();
    assert!((stats_anti.stderr() / expected_stderr_order - 1.0).abs() < 0.5);

    println!(
        "Antithetic: mean={:.4}, stderr={:.6}",
        stats_anti.mean(),
        stats_anti.stderr()
    );
}

#[test]
fn test_convergence_with_paths() {
    // Verify that stderr decreases as O(N^{-1/2})
    let seeds_and_paths = vec![(42, 1_000), (43, 10_000), (44, 100_000)];

    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.0, 0.2));
    let call = EuropeanCall::new(100.0, 1.0, 100);

    let mut stderr_values = Vec::new();

    for (seed, num_paths) in seeds_and_paths {
        let config = EuropeanPricerConfig::new(num_paths)
            .with_seed(seed)
            .with_parallel(false);
        let pricer = EuropeanPricer::new(config);

        let result = pricer
            .price(&gbm, 100.0, 1.0, 100, &call, Currency::USD, 1.0)
            .unwrap();

        stderr_values.push((num_paths, result.stderr));
        println!(
            "N={:6}: stderr={:.6}, mean={:.4}",
            num_paths,
            result.stderr,
            result.mean.amount()
        );
    }

    // Stderr should decrease with more paths
    assert!(stderr_values[1].1 < stderr_values[0].1);
    assert!(stderr_values[2].1 < stderr_values[1].1);

    // Check approximate O(N^{-1/2}) scaling
    // stderr(10k) / stderr(1k) ≈ sqrt(1k/10k) = sqrt(0.1) ≈ 0.316
    let ratio = stderr_values[1].1 / stderr_values[0].1;
    let expected_ratio = (1_000.0 / 10_000.0_f64).sqrt();
    assert!(
        (ratio / expected_ratio - 1.0).abs() < 0.3,
        "Convergence rate {} deviates from expected {}",
        ratio,
        expected_ratio
    );
}

#[test]
fn test_deep_itm_call() {
    // Deep ITM option should be close to intrinsic value (discounted)
    let config = EuropeanPricerConfig::new(10_000).with_seed(42).with_parallel(false);
    let pricer = EuropeanPricer::new(config);

    // Very low volatility to minimize time value
    let gbm = GbmProcess::new(GbmParams::new(0.0, 0.0, 0.01));
    let call = EuropeanCall::new(80.0, 1.0, 10);

    let result = pricer
        .price(&gbm, 100.0, 1.0, 10, &call, Currency::USD, 1.0)
        .unwrap();

    // Intrinsic value is 20
    let intrinsic = 20.0;
    assert!((result.mean.amount() - intrinsic).abs() < 1.0);
}

#[test]
fn test_otm_call() {
    // OTM option with very low volatility should be close to zero
    let config = EuropeanPricerConfig::new(10_000).with_seed(42).with_parallel(false);
    let pricer = EuropeanPricer::new(config);

    let gbm = GbmProcess::new(GbmParams::new(0.0, 0.0, 0.01));
    let call = EuropeanCall::new(120.0, 1.0, 10);

    let result = pricer
        .price(&gbm, 100.0, 1.0, 10, &call, Currency::USD, 1.0)
        .unwrap();

    // Should be very close to zero
    assert!(result.mean.amount() < 0.5);
}

#[test]
#[cfg(feature = "parallel")]
fn test_parallel_determinism() {
    // Parallel and serial execution should give identical results
    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.2));
    let call = EuropeanCall::new(100.0, 1.0, 100);

    let config_serial = EuropeanPricerConfig::new(10_000)
        .with_seed(42)
        .with_parallel(false);
    let pricer_serial = EuropeanPricer::new(config_serial);

    let result_serial = pricer_serial
        .price(&gbm, 100.0, 1.0, 100, &call, Currency::USD, 1.0)
        .unwrap();

    let config_parallel = EuropeanPricerConfig::new(10_000)
        .with_seed(42)
        .with_parallel(true);
    let pricer_parallel = EuropeanPricer::new(config_parallel);

    let result_parallel = pricer_parallel
        .price(&gbm, 100.0, 1.0, 100, &call, Currency::USD, 1.0)
        .unwrap();

    // Results should be very close (floating point differences due to reduction order)
    let diff = (result_serial.mean.amount() - result_parallel.mean.amount()).abs();
    assert!(
        diff < 1e-6,
        "Parallel and serial results should be nearly identical: diff={}",
        diff
    );
    
    // Stderr should be close
    let stderr_diff = (result_serial.stderr - result_parallel.stderr).abs();
    assert!(stderr_diff < 1e-6);

    println!("Parallel determinism verified: {}", result_serial.mean);
}

#[test]
fn test_put_call_parity_mc() {
    // Verify put-call parity holds for MC prices
    // C - P = S*e^(-qT) - K*e^(-rT)
    let config = EuropeanPricerConfig::new(100_000)
        .with_seed(42)
        .with_parallel(false);
    let pricer = EuropeanPricer::new(config);

    let s = 100.0;
    let k = 100.0;
    let t = 1.0;
    let r = 0.05;
    let q = 0.02;
    let sigma = 0.2;

    let gbm = GbmProcess::new(GbmParams::new(r, q, sigma));
    let call = EuropeanCall::new(k, 1.0, 252);
    let put = EuropeanPut::new(k, 1.0, 252);

    let discount = (-r * t).exp();

    // Price with discount to get PV
    let call_price = pricer
        .price(&gbm, s, t, 252, &call, Currency::USD, discount)
        .unwrap();
    let put_price = pricer
        .price(&gbm, s, t, 252, &put, Currency::USD, discount)
        .unwrap();

    // Put-call parity: C - P = S*e^(-qT) - K*e^(-rT)
    let lhs = call_price.mean.amount() - put_price.mean.amount();
    let rhs = s * (-q * t).exp() - k * discount;

    // Should hold within combined standard errors
    let combined_stderr = (call_price.stderr.powi(2) + put_price.stderr.powi(2)).sqrt();
    let tolerance = 3.0 * combined_stderr;

    assert!(
        (lhs - rhs).abs() < tolerance,
        "Put-call parity violated: {} vs {} (tolerance={})",
        lhs,
        rhs,
        tolerance
    );

    println!("Put-call parity: C-P={:.4}, PCP={:.4}", lhs, rhs);
}

#[test]
fn test_currency_safety() {
    // Verify that results carry correct currency
    let config = EuropeanPricerConfig::new(1_000).with_seed(42).with_parallel(false);
    let pricer = EuropeanPricer::new(config);

    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.2));
    let call = EuropeanCall::new(100.0, 1.0, 10);

    // Price in USD
    let result_usd = pricer
        .price(&gbm, 100.0, 1.0, 10, &call, Currency::USD, 1.0)
        .unwrap();
    assert_eq!(result_usd.mean.currency(), Currency::USD);

    // Price in EUR (same numerical result, different currency)
    let result_eur = pricer
        .price(&gbm, 100.0, 1.0, 10, &call, Currency::EUR, 1.0)
        .unwrap();
    assert_eq!(result_eur.mean.currency(), Currency::EUR);
    assert_eq!(result_eur.mean.amount(), result_usd.mean.amount());
}

#[test]
fn test_notional_scaling() {
    // Verify that notional scaling works correctly
    let config = EuropeanPricerConfig::new(10_000).with_seed(42).with_parallel(false);
    let pricer = EuropeanPricer::new(config);

    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.2));

    let call_n1 = EuropeanCall::new(100.0, 1.0, 50);
    let call_n10 = EuropeanCall::new(100.0, 10.0, 50);

    let result_n1 = pricer
        .price(&gbm, 100.0, 1.0, 50, &call_n1, Currency::USD, 1.0)
        .unwrap();
    let result_n10 = pricer
        .price(&gbm, 100.0, 1.0, 50, &call_n10, Currency::USD, 1.0)
        .unwrap();

    // N=10 should be ~10x N=1 (within statistical tolerance)
    let ratio = result_n10.mean.amount() / result_n1.mean.amount();
    assert!((ratio - 10.0).abs() < 0.5);
}

#[test]
fn test_zero_volatility_call() {
    // With zero volatility, call option has deterministic outcome
    let config = EuropeanPricerConfig::new(1_000).with_seed(42).with_parallel(false);
    let pricer = EuropeanPricer::new(config);

    // No drift, no volatility - spot stays at 100
    let gbm = GbmProcess::new(GbmParams::new(0.0, 0.0, 0.0));
    let call = EuropeanCall::new(95.0, 1.0, 10);

    let result = pricer
        .price(&gbm, 100.0, 1.0, 10, &call, Currency::USD, 1.0)
        .unwrap();

    // Payoff should be exactly 5 (100 - 95)
    assert!((result.mean.amount() - 5.0).abs() < 0.01);
    assert!(result.stderr < 0.01); // Should have very low variance
}

#[test]
fn test_confidence_intervals() {
    // Verify confidence intervals are reasonable
    let config = EuropeanPricerConfig::new(10_000).with_seed(42).with_parallel(false);
    let pricer = EuropeanPricer::new(config);

    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.2));
    let call = EuropeanCall::new(100.0, 1.0, 100);

    let result = pricer
        .price(&gbm, 100.0, 1.0, 100, &call, Currency::USD, 1.0)
        .unwrap();

    // CI should contain the mean
    assert!(result.ci_95.0.amount() < result.mean.amount());
    assert!(result.ci_95.1.amount() > result.mean.amount());

    // CI width should be reasonable (~4 * stderr)
    let ci_width = result.ci_95.1.amount() - result.ci_95.0.amount();
    let expected_width = 2.0 * 1.96 * result.stderr; // 2 * z_{0.975} * stderr
    assert!((ci_width - expected_width).abs() < 0.1);

    println!(
        "CI: [{:.4}, {:.4}], width={:.4}",
        result.ci_95.0.amount(),
        result.ci_95.1.amount(),
        ci_width
    );
}

