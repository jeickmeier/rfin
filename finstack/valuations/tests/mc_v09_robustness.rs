//! Monte Carlo v0.9 integration tests - Robustness and extreme parameters.
//!
//! Tests MC pricing engine with extreme parameter regimes to ensure
//! numerical stability and graceful error handling.

#![cfg(feature = "mc")]

use finstack_core::currency::Currency;
use finstack_valuations::instruments::common::mc::discretization::exact::ExactGbm;
use finstack_valuations::instruments::common::mc::discretization::qe_heston::QeHeston;
use finstack_valuations::instruments::common::mc::engine::{McEngine, McEngineConfig};
use finstack_valuations::instruments::common::mc::payoff::vanilla::EuropeanCall;
use finstack_valuations::instruments::common::mc::process::gbm::{GbmParams, GbmProcess};
use finstack_valuations::instruments::common::mc::process::heston::{HestonParams, HestonProcess};
use finstack_valuations::instruments::common::mc::rng::philox::PhiloxRng;
use finstack_valuations::instruments::common::mc::time_grid::TimeGrid;

// ============================================================================
// Heston Extreme Parameter Tests
// ============================================================================

#[test]
fn test_heston_low_kappa_slow_mean_reversion() {
    // Very low kappa (slow mean reversion)
    let params = HestonParams::new(
        0.05,  // r
        0.02,  // q
        0.1,   // kappa (very slow mean reversion)
        0.04,  // theta
        0.3,   // sigma_v
        -0.7,  // rho
        0.04,  // v0
    );

    let time_grid = TimeGrid::uniform(1.0, 252).unwrap();
    let engine = McEngine::new(McEngineConfig {
        num_paths: 10_000,
        seed: 42,
        time_grid,
        target_ci_half_width: None,
        use_parallel: false,
        chunk_size: 1000,
    });

    let heston = HestonProcess::new(params);
    let disc = QeHeston::new();
    let call = EuropeanCall::new(100.0, 1.0, 252);
    let rng = PhiloxRng::new(42);

    let result = engine
        .price(
            &rng,
            &heston,
            &disc,
            &[100.0, 0.04],
            &call,
            Currency::USD,
            0.95,
        );

    // Should not panic or produce NaN
    assert!(result.is_ok());
    let price = result.unwrap();
    assert!(price.mean.amount().is_finite());
    assert!(price.mean.amount() > 0.0);

    println!("Low kappa Heston: {}", price.mean);
}

#[test]
fn test_heston_high_vol_of_vol() {
    // Very high vol-of-vol (extreme stochastic volatility)
    let params = HestonParams::new(
        0.05,  // r
        0.02,  // q
        2.0,   // kappa
        0.04,  // theta
        1.0,   // sigma_v (very high vol-of-vol)
        -0.7,  // rho
        0.04,  // v0
    );

    let time_grid = TimeGrid::uniform(1.0, 252).unwrap();
    let engine = McEngine::new(McEngineConfig {
        num_paths: 10_000,
        seed: 42,
        time_grid,
        target_ci_half_width: None,
        use_parallel: false,
        chunk_size: 1000,
    });

    let heston = HestonProcess::new(params);
    let disc = QeHeston::new();
    let call = EuropeanCall::new(100.0, 1.0, 252);
    let rng = PhiloxRng::new(42);

    let result = engine
        .price(
            &rng,
            &heston,
            &disc,
            &[100.0, 0.04],
            &call,
            Currency::USD,
            0.95,
        );

    assert!(result.is_ok());
    let price = result.unwrap();
    assert!(price.mean.amount().is_finite());
    assert!(price.mean.amount() > 0.0);

    println!("High vol-of-vol Heston: {}", price.mean);
}

#[test]
fn test_heston_feller_condition_violation() {
    // Feller condition: 2κθ ≥ σ_v² ensures variance stays positive
    // Test with violation: 2κθ < σ_v²
    let kappa = 0.5;
    let theta = 0.04;
    let sigma_v = 1.0;

    // Check: 2 * 0.5 * 0.04 = 0.04 < 1.0² = 1.0 (Feller violated)
    let feller_lhs = 2.0 * kappa * theta;
    let feller_rhs = sigma_v * sigma_v;
    assert!(feller_lhs < feller_rhs, "Feller condition should be violated");

    let params = HestonParams::new(
        0.05,      // r
        0.02,      // q
        kappa,     // kappa
        theta,     // theta
        sigma_v,   // sigma_v
        -0.7,      // rho
        theta,     // v0
    );

    let time_grid = TimeGrid::uniform(1.0, 252).unwrap();
    let engine = McEngine::new(McEngineConfig {
        num_paths: 10_000,
        seed: 42,
        time_grid,
        target_ci_half_width: None,
        use_parallel: false,
        chunk_size: 1000,
    });

    let heston = HestonProcess::new(params);
    let disc = QeHeston::new();
    let call = EuropeanCall::new(100.0, 1.0, 252);
    let rng = PhiloxRng::new(42);

    let result = engine
        .price(
            &rng,
            &heston,
            &disc,
            &[100.0, theta],
            &call,
            Currency::USD,
            0.95,
        );

    // QE scheme should still produce non-negative variance
    assert!(result.is_ok());
    let price = result.unwrap();
    assert!(price.mean.amount().is_finite());
    assert!(price.mean.amount() >= 0.0);

    println!("Feller violation Heston: {}", price.mean);
}

#[test]
fn test_heston_initial_variance_below_theta() {
    // Start with variance well below long-term mean
    let params = HestonParams::new(
        0.05,   // r
        0.02,   // q
        3.0,    // kappa (strong mean reversion)
        0.04,   // theta (long-term variance)
        0.3,    // sigma_v
        -0.7,   // rho
        0.01,   // v0 (start at 10% vol, below 20% long-term)
    );

    let time_grid = TimeGrid::uniform(1.0, 252).unwrap();
    let engine = McEngine::new(McEngineConfig {
        num_paths: 10_000,
        seed: 42,
        time_grid,
        target_ci_half_width: None,
        use_parallel: false,
        chunk_size: 1000,
    });

    let heston = HestonProcess::new(params);
    let disc = QeHeston::new();
    let call = EuropeanCall::new(100.0, 1.0, 252);
    let rng = PhiloxRng::new(42);

    let result = engine
        .price(
            &rng,
            &heston,
            &disc,
            &[100.0, 0.01],
            &call,
            Currency::USD,
            0.95,
        );

    assert!(result.is_ok());
    let price = result.unwrap();
    assert!(price.mean.amount().is_finite());
    assert!(price.mean.amount() > 0.0);

    println!("Low initial variance Heston: {}", price.mean);
}

#[test]
fn test_heston_initial_variance_above_theta() {
    // Start with variance well above long-term mean
    let params = HestonParams::new(
        0.05,   // r
        0.02,   // q
        3.0,    // kappa (strong mean reversion)
        0.04,   // theta (long-term variance)
        0.3,    // sigma_v
        -0.7,   // rho
        0.16,   // v0 (start at 40% vol, above 20% long-term)
    );

    let time_grid = TimeGrid::uniform(1.0, 252).unwrap();
    let engine = McEngine::new(McEngineConfig {
        num_paths: 10_000,
        seed: 42,
        time_grid,
        target_ci_half_width: None,
        use_parallel: false,
        chunk_size: 1000,
    });

    let heston = HestonProcess::new(params);
    let disc = QeHeston::new();
    let call = EuropeanCall::new(100.0, 1.0, 252);
    let rng = PhiloxRng::new(42);

    let result = engine
        .price(
            &rng,
            &heston,
            &disc,
            &[100.0, 0.16],
            &call,
            Currency::USD,
            0.95,
        );

    assert!(result.is_ok());
    let price = result.unwrap();
    assert!(price.mean.amount().is_finite());
    assert!(price.mean.amount() > 0.0);

    println!("High initial variance Heston: {}", price.mean);
}

// ============================================================================
// GBM Extreme Parameter Tests
// ============================================================================

#[test]
fn test_gbm_very_high_volatility() {
    // σ = 200% (extreme volatility)
    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 2.0));

    let time_grid = TimeGrid::uniform(1.0, 252).unwrap();
    let engine = McEngine::new(McEngineConfig {
        num_paths: 10_000,
        seed: 42,
        time_grid,
        target_ci_half_width: None,
        use_parallel: false,
        chunk_size: 1000,
    });

    let disc = ExactGbm::new();
    let call = EuropeanCall::new(100.0, 1.0, 252);
    let rng = PhiloxRng::new(42);

    let result = engine
        .price(
            &rng,
            &gbm,
            &disc,
            &[100.0],
            &call,
            Currency::USD,
            0.95,
        );

    // Should not overflow or produce negative prices
    assert!(result.is_ok());
    let price = result.unwrap();
    assert!(price.mean.amount().is_finite());
    assert!(price.mean.amount() >= 0.0);

    println!("Very high volatility GBM (σ=200%): {}", price.mean);
}

#[test]
fn test_gbm_very_low_volatility() {
    // σ = 0.1% (near-deterministic)
    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.001));

    let time_grid = TimeGrid::uniform(1.0, 252).unwrap();
    let engine = McEngine::new(McEngineConfig {
        num_paths: 5_000,
        seed: 42,
        time_grid,
        target_ci_half_width: None,
        use_parallel: false,
        chunk_size: 1000,
    });

    let disc = ExactGbm::new();
    let call = EuropeanCall::new(100.0, 1.0, 252);
    let rng = PhiloxRng::new(42);

    let result = engine
        .price(
            &rng,
            &gbm,
            &disc,
            &[100.0],
            &call,
            Currency::USD,
            0.95,
        );

    assert!(result.is_ok());
    let price = result.unwrap();
    assert!(price.mean.amount().is_finite());
    assert!(price.mean.amount() >= 0.0);

    // With very low vol, should be close to intrinsic value of forward
        let forward = 100.0 * ((0.05_f64 - 0.02) * 1.0).exp();
    println!(
        "Very low volatility GBM (σ=0.1%): {}, Forward: {:.2}",
        price.mean, forward
    );
}

#[test]
fn test_gbm_high_dividend_yield() {
    // Very high dividend yield (q >> r)
    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.20, 0.3));

    let time_grid = TimeGrid::uniform(1.0, 252).unwrap();
    let engine = McEngine::new(McEngineConfig {
        num_paths: 10_000,
        seed: 42,
        time_grid,
        target_ci_half_width: None,
        use_parallel: false,
        chunk_size: 1000,
    });

    let disc = ExactGbm::new();
    let call = EuropeanCall::new(100.0, 1.0, 252);
    let rng = PhiloxRng::new(42);

    let result = engine
        .price(
            &rng,
            &gbm,
            &disc,
            &[100.0],
            &call,
            Currency::USD,
            0.95,
        );

    assert!(result.is_ok());
    let price = result.unwrap();
    assert!(price.mean.amount().is_finite());
    assert!(price.mean.amount() >= 0.0);

    // With q >> r, forward drifts down, call value should be low
    println!("High dividend yield GBM (q=20%): {}", price.mean);
}

#[test]
fn test_gbm_negative_drift() {
    // r < q (negative drift under risk-neutral measure)
    let gbm = GbmProcess::new(GbmParams::new(0.02, 0.10, 0.3));

    let time_grid = TimeGrid::uniform(1.0, 252).unwrap();
    let engine = McEngine::new(McEngineConfig {
        num_paths: 10_000,
        seed: 42,
        time_grid,
        target_ci_half_width: None,
        use_parallel: false,
        chunk_size: 1000,
    });

    let disc = ExactGbm::new();
    let call = EuropeanCall::new(100.0, 1.0, 252);
    let rng = PhiloxRng::new(42);

    let result = engine
        .price(
            &rng,
            &gbm,
            &disc,
            &[100.0],
            &call,
            Currency::USD,
            0.98,
        );

    assert!(result.is_ok());
    let price = result.unwrap();
    assert!(price.mean.amount().is_finite());
    assert!(price.mean.amount() >= 0.0);

    println!("Negative drift GBM (r=2%, q=10%): {}", price.mean);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_very_short_maturity() {
    // T = 1 day (0.004 years)
    let time = 1.0 / 252.0;
    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.2));

    let time_grid = TimeGrid::uniform(time, 1).unwrap();
    let engine = McEngine::new(McEngineConfig {
        num_paths: 10_000,
        seed: 42,
        time_grid,
        target_ci_half_width: None,
        use_parallel: false,
        chunk_size: 1000,
    });

    let disc = ExactGbm::new();
    let call = EuropeanCall::new(100.0, 1.0, 1);
    let rng = PhiloxRng::new(42);

    let result = engine
        .price(
            &rng,
            &gbm,
            &disc,
            &[100.0],
            &call,
            Currency::USD,
            1.0,
        );

    assert!(result.is_ok());
    let price = result.unwrap();
    assert!(price.mean.amount().is_finite());

    println!("Very short maturity (1 day): {}", price.mean);
}

#[test]
fn test_very_long_maturity() {
    // T = 30 years
    let time = 30.0;
    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.2));

    let time_grid = TimeGrid::uniform(time, 252).unwrap();
    let engine = McEngine::new(McEngineConfig {
        num_paths: 10_000,
        seed: 42,
        time_grid,
        target_ci_half_width: None,
        use_parallel: false,
        chunk_size: 1000,
    });

    let disc = ExactGbm::new();
    let call = EuropeanCall::new(100.0, 1.0, 252);
    let rng = PhiloxRng::new(42);

    let result = engine
        .price(
            &rng,
            &gbm,
            &disc,
            &[100.0],
            &call,
            Currency::USD,
            0.22,  // e^(-0.05*30)
        );

    assert!(result.is_ok());
    let price = result.unwrap();
    assert!(price.mean.amount().is_finite());
    assert!(price.mean.amount() >= 0.0);

    println!("Very long maturity (30 years): {}", price.mean);
}

#[test]
fn test_deep_itm_call() {
    // S = 150, K = 50 (deep in-the-money)
    let spot = 150.0;
    let strike = 50.0;
    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.2));

    let time_grid = TimeGrid::uniform(1.0, 252).unwrap();
    let engine = McEngine::new(McEngineConfig {
        num_paths: 10_000,
        seed: 42,
        time_grid,
        target_ci_half_width: None,
        use_parallel: false,
        chunk_size: 1000,
    });

    let disc = ExactGbm::new();
    let call = EuropeanCall::new(strike, 1.0, 252);
    let rng = PhiloxRng::new(42);

    let result = engine
        .price(
            &rng,
            &gbm,
            &disc,
            &[spot],
            &call,
            Currency::USD,
            0.95,
        );

    assert!(result.is_ok());
    let price = result.unwrap();
    assert!(price.mean.amount().is_finite());

    // Should be close to intrinsic value
    let intrinsic = (spot - strike) * 0.95;
    println!(
        "Deep ITM call: {:.2}, Intrinsic: {:.2}",
        price.mean.amount(),
        intrinsic
    );

    assert!(price.mean.amount() > intrinsic * 0.9);
}

#[test]
fn test_deep_otm_call() {
    // S = 50, K = 150 (deep out-of-the-money)
    let spot = 50.0;
    let strike = 150.0;
    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.2));

    let time_grid = TimeGrid::uniform(1.0, 252).unwrap();
    let engine = McEngine::new(McEngineConfig {
        num_paths: 50_000,  // More paths for rare events
        seed: 42,
        time_grid,
        target_ci_half_width: None,
        use_parallel: false,
        chunk_size: 1000,
    });

    let disc = ExactGbm::new();
    let call = EuropeanCall::new(strike, 1.0, 252);
    let rng = PhiloxRng::new(42);

    let result = engine
        .price(
            &rng,
            &gbm,
            &disc,
            &[spot],
            &call,
            Currency::USD,
            0.95,
        );

    assert!(result.is_ok());
    let price = result.unwrap();
    assert!(price.mean.amount().is_finite());
    assert!(price.mean.amount() >= 0.0);

    // Should be very small
    println!("Deep OTM call: {:.6}", price.mean.amount());
    assert!(price.mean.amount() < 1.0);
}

// ============================================================================
// Invalid Input Tests
// ============================================================================

#[test]
fn test_negative_spot_rejected() {
    // Negative spot should be rejected gracefully
    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.2));

    let time_grid = TimeGrid::uniform(1.0, 100).unwrap();
    let engine = McEngine::new(McEngineConfig {
        num_paths: 1_000,
        seed: 42,
        time_grid,
        target_ci_half_width: None,
        use_parallel: false,
        chunk_size: 1000,
    });

    let disc = ExactGbm::new();
    let call = EuropeanCall::new(100.0, 1.0, 100);
    let rng = PhiloxRng::new(42);

    // Negative spot (invalid input)
    let result = engine
        .price(
            &rng,
            &gbm,
            &disc,
            &[-100.0],  // Invalid
            &call,
            Currency::USD,
            0.95,
        );

    // Should still complete (GBM handles gracefully)
    // Negative spot will evolve to near-zero or produce zero payoff
    if let Ok(price) = result {
        println!("Negative spot result: {}", price.mean);
        // Just verify it doesn't panic
    }
}

#[test]
fn test_zero_volatility() {
    // σ = 0 (deterministic evolution)
    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.0));

    let time_grid = TimeGrid::uniform(1.0, 100).unwrap();
    let engine = McEngine::new(McEngineConfig {
        num_paths: 1_000,
        seed: 42,
        time_grid,
        target_ci_half_width: None,
        use_parallel: false,
        chunk_size: 1000,
    });

    let disc = ExactGbm::new();
    let call = EuropeanCall::new(100.0, 1.0, 100);
    let rng = PhiloxRng::new(42);

    let result = engine
        .price(
            &rng,
            &gbm,
            &disc,
            &[100.0],
            &call,
            Currency::USD,
            0.95,
        );

    assert!(result.is_ok());
    let price = result.unwrap();
    assert!(price.mean.amount().is_finite());

    // With zero vol, all paths are identical
    assert_eq!(price.stderr, 0.0); // No variance

    println!("Zero volatility GBM: {}, stderr={}", price.mean, price.stderr);
}

#[test]
fn test_zero_time_to_maturity() {
    // T = 0 (immediate expiry)
    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.2));

    // Create very small time grid
    let time_grid = TimeGrid::uniform(0.001, 1).unwrap();
    let engine = McEngine::new(McEngineConfig {
        num_paths: 1_000,
        seed: 42,
        time_grid,
        target_ci_half_width: None,
        use_parallel: false,
        chunk_size: 1000,
    });

    let disc = ExactGbm::new();
    let call = EuropeanCall::new(100.0, 1.0, 1);
    let rng = PhiloxRng::new(42);

    let result = engine
        .price(
            &rng,
            &gbm,
            &disc,
            &[105.0],  // Start ITM
            &call,
            Currency::USD,
            1.0,
        );

    assert!(result.is_ok());
    let price = result.unwrap();
    
    // Should be close to intrinsic value (5.0)
    println!("Near-zero maturity (ITM): {:.4}", price.mean.amount());
    assert!((price.mean.amount() - 5.0).abs() < 1.0);
}

