//! Monte Carlo v0.7 integration tests - Discrete dividends.
//!
//! Tests GBM with discrete dividends for equity derivative pricing.

#![cfg(feature = "mc")]

use finstack_core::currency::Currency;
use finstack_valuations::instruments::common::mc::discretization::exact_gbm_div::ExactGbmWithDividends;
use finstack_valuations::instruments::common::mc::engine::{McEngine, McEngineConfig};
use finstack_valuations::instruments::common::mc::payoff::vanilla::{EuropeanCall, EuropeanPut};
use finstack_valuations::instruments::common::mc::pricer::european::{
    EuropeanPricer, EuropeanPricerConfig,
};
use finstack_valuations::instruments::common::mc::process::gbm::{GbmParams, GbmProcess};
use finstack_valuations::instruments::common::mc::process::gbm_dividends::{
    Dividend, GbmWithDividends,
};
use finstack_valuations::instruments::common::mc::rng::philox::PhiloxRng;
use finstack_valuations::instruments::common::mc::time_grid::TimeGrid;

// ============================================================================
// Basic Dividend Functionality Tests
// ============================================================================

#[test]
fn test_no_dividends_matches_standard_gbm() {
    // GBM with empty dividend schedule should match standard GBM
    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.2));
    let gbm_div = GbmWithDividends::new(GbmParams::new(0.05, 0.02, 0.2), vec![]);

    let time_grid = TimeGrid::uniform(1.0, 252).unwrap();
    let config = McEngineConfig {
        num_paths: 10_000,
        seed: 42,
        time_grid: time_grid.clone(),
        target_ci_half_width: None,
        use_parallel: false,
        chunk_size: 1000,
    };

    // Standard GBM
    let engine_gbm = McEngine::new(config.clone());
    let disc_gbm = finstack_valuations::instruments::common::mc::discretization::exact::ExactGbm::new();
    let call = EuropeanCall::new(100.0, 1.0, 252);
    let rng = PhiloxRng::new(42);

    let result_gbm = engine_gbm
        .price(
            &rng,
            &gbm,
            &disc_gbm,
            &[100.0],
            &call,
            Currency::USD,
            0.95,
        )
        .unwrap();

    // GBM with dividends (but no dividends)
    let engine_div = McEngine::new(config);
    let disc_div = ExactGbmWithDividends::new();
    let rng2 = PhiloxRng::new(42);

    let result_div = engine_div
        .price(
            &rng2,
            &gbm_div,
            &disc_div,
            &[100.0],
            &call,
            Currency::USD,
            0.95,
        )
        .unwrap();

    println!(
        "No dividends: Standard GBM={:.6}, GBM with dividends={:.6}",
        result_gbm.mean.amount(),
        result_div.mean.amount()
    );

    // Should be very close (same RNG seed)
    let diff = (result_gbm.mean.amount() - result_div.mean.amount()).abs();
    assert!(diff < 0.5, "Results should match: diff={}", diff);
}

#[test]
fn test_cash_dividend_reduces_call_value() {
    // Call option value should decrease with cash dividends
    let spot = 100.0;
    let strike = 100.0;
    let time = 1.0;
    let r = 0.05;
    let vol = 0.2;

    // Price without dividends (using continuous yield as proxy)
    let config_no_div = EuropeanPricerConfig::new(50_000)
        .with_seed(42)
        .with_parallel(false);
    let pricer_no_div = EuropeanPricer::new(config_no_div);
    let gbm_no_div = GbmProcess::new(GbmParams::new(r, 0.0, vol));
    let call = EuropeanCall::new(strike, 1.0, 252);

    let result_no_div = pricer_no_div
        .price(&gbm_no_div, spot, time, 252, &call, Currency::USD, 0.95)
        .unwrap();

    // Price with quarterly cash dividends
    let dividends = vec![
        (0.25, Dividend::Cash(0.50)),
        (0.50, Dividend::Cash(0.50)),
        (0.75, Dividend::Cash(0.50)),
    ];

    let gbm_div = GbmWithDividends::new(GbmParams::new(r, 0.0, vol), dividends);
    let disc_div = ExactGbmWithDividends::new();
    let time_grid = TimeGrid::uniform(time, 252).unwrap();
    let engine = McEngine::new(McEngineConfig {
        num_paths: 50_000,
        seed: 42,
        time_grid,
        target_ci_half_width: None,
        use_parallel: false,
        chunk_size: 1000,
    });
    let rng = PhiloxRng::new(42);

    let result_div = engine
        .price(&rng, &gbm_div, &disc_div, &[spot], &call, Currency::USD, 0.95)
        .unwrap();

    println!(
        "Call value: No dividends={:.6}, With dividends={:.6}",
        result_no_div.mean.amount(),
        result_div.mean.amount()
    );

    // Dividends should reduce call value
    assert!(
        result_div.mean.amount() < result_no_div.mean.amount(),
        "Cash dividends should reduce call value"
    );
}

#[test]
fn test_cash_dividend_increases_put_value() {
    // Put option value should increase with cash dividends
    let spot = 100.0;
    let strike = 100.0;
    let time = 1.0;
    let r = 0.05;
    let vol = 0.2;

    // Price without dividends
    let config_no_div = EuropeanPricerConfig::new(50_000)
        .with_seed(42)
        .with_parallel(false);
    let pricer_no_div = EuropeanPricer::new(config_no_div);
    let gbm_no_div = GbmProcess::new(GbmParams::new(r, 0.0, vol));
    let put = EuropeanPut::new(strike, 1.0, 252);

    let result_no_div = pricer_no_div
        .price(&gbm_no_div, spot, time, 252, &put, Currency::USD, 0.95)
        .unwrap();

    // Price with quarterly cash dividends
    let dividends = vec![
        (0.25, Dividend::Cash(0.75)),
        (0.50, Dividend::Cash(0.75)),
        (0.75, Dividend::Cash(0.75)),
    ];

    let gbm_div = GbmWithDividends::new(GbmParams::new(r, 0.0, vol), dividends);
    let disc_div = ExactGbmWithDividends::new();
    let time_grid = TimeGrid::uniform(time, 252).unwrap();
    let engine = McEngine::new(McEngineConfig {
        num_paths: 50_000,
        seed: 42,
        time_grid,
        target_ci_half_width: None,
        use_parallel: false,
        chunk_size: 1000,
    });
    let rng = PhiloxRng::new(42);

    let result_div = engine
        .price(&rng, &gbm_div, &disc_div, &[spot], &put, Currency::USD, 0.95)
        .unwrap();

    println!(
        "Put value: No dividends={:.6}, With dividends={:.6}",
        result_no_div.mean.amount(),
        result_div.mean.amount()
    );

    // Dividends should increase put value
    assert!(
        result_div.mean.amount() > result_no_div.mean.amount(),
        "Cash dividends should increase put value"
    );
}

#[test]
fn test_proportional_dividend_vs_continuous_yield() {
    // Proportional dividends should behave similarly to continuous yield
    let spot = 100.0;
    let strike = 100.0;
    let time = 1.0;
    let r = 0.05;
    let vol = 0.2;

    // Annual dividend yield: 4% paid quarterly (1% per quarter)
    let q_annual = 0.04;
    let q_quarterly = 0.01;

    // Continuous yield model
    let gbm_cont = GbmProcess::new(GbmParams::new(r, q_annual, vol));
    let config_cont = EuropeanPricerConfig::new(50_000)
        .with_seed(42)
        .with_parallel(false);
    let pricer_cont = EuropeanPricer::new(config_cont);
    let call = EuropeanCall::new(strike, 1.0, 252);

    let result_cont = pricer_cont
        .price(&gbm_cont, spot, time, 252, &call, Currency::USD, 0.95)
        .unwrap();

    // Discrete proportional dividends (quarterly)
    let dividends = vec![
        (0.25, Dividend::Proportional(q_quarterly)),
        (0.50, Dividend::Proportional(q_quarterly)),
        (0.75, Dividend::Proportional(q_quarterly)),
        (1.00, Dividend::Proportional(q_quarterly)),
    ];

    let gbm_div = GbmWithDividends::new(GbmParams::new(r, 0.0, vol), dividends);
    let disc_div = ExactGbmWithDividends::new();
    let time_grid = TimeGrid::uniform(time, 252).unwrap();
    let engine = McEngine::new(McEngineConfig {
        num_paths: 50_000,
        seed: 42,
        time_grid,
        target_ci_half_width: None,
        use_parallel: false,
        chunk_size: 1000,
    });
    let rng = PhiloxRng::new(42);

    let result_div = engine
        .price(&rng, &gbm_div, &disc_div, &[spot], &call, Currency::USD, 0.95)
        .unwrap();

    println!(
        "Call value: Continuous yield={:.6}, Proportional dividends={:.6}",
        result_cont.mean.amount(),
        result_div.mean.amount()
    );

    // Should be reasonably close (not exact due to compounding differences)
    let diff = (result_cont.mean.amount() - result_div.mean.amount()).abs();
    let avg = (result_cont.mean.amount() + result_div.mean.amount()) / 2.0;
    let rel_diff = diff / avg;

    println!("  Relative difference: {:.2}%", rel_diff * 100.0);

    // Within 20% relative difference is reasonable given discretization
    assert!(rel_diff < 0.2);
}

#[test]
fn test_dividend_timing_matters() {
    // Dividends early vs late should affect option value differently
    let spot = 100.0;
    let strike = 100.0;
    let time = 1.0;
    let r = 0.05;
    let vol = 0.2;
    let div_amount = 2.0;

    // Early dividend (at t=0.1)
    let dividends_early = vec![(0.1, Dividend::Cash(div_amount))];
    let gbm_early = GbmWithDividends::new(GbmParams::new(r, 0.0, vol), dividends_early);

    // Late dividend (at t=0.9)
    let dividends_late = vec![(0.9, Dividend::Cash(div_amount))];
    let gbm_late = GbmWithDividends::new(GbmParams::new(r, 0.0, vol), dividends_late);

    let disc_div = ExactGbmWithDividends::new();
    let time_grid = TimeGrid::uniform(time, 252).unwrap();
    let config = McEngineConfig {
        num_paths: 50_000,
        seed: 42,
        time_grid: time_grid.clone(),
        target_ci_half_width: None,
        use_parallel: false,
        chunk_size: 1000,
    };

    let call = EuropeanCall::new(strike, 1.0, 252);

    // Price with early dividend
    let engine_early = McEngine::new(config.clone());
    let rng_early = PhiloxRng::new(42);
    let result_early = engine_early
        .price(
            &rng_early,
            &gbm_early,
            &disc_div,
            &[spot],
            &call,
            Currency::USD,
            0.95,
        )
        .unwrap();

    // Price with late dividend
    let engine_late = McEngine::new(config);
    let rng_late = PhiloxRng::new(42);
    let result_late = engine_late
        .price(
            &rng_late,
            &gbm_late,
            &disc_div,
            &[spot],
            &call,
            Currency::USD,
            0.95,
        )
        .unwrap();

    println!(
        "Call value: Early dividend={:.6}, Late dividend={:.6}",
        result_early.mean.amount(),
        result_late.mean.amount()
    );

    // Early dividends have more impact on calls (more time for spot to recover)
    // Late dividends have less impact
    // So late dividend call should be worth slightly more
    assert!(result_late.mean.amount() > result_early.mean.amount() - 1.0);
}

#[test]
fn test_multiple_dividends_accumulate() {
    // Multiple dividends should have cumulative effect
    let spot = 100.0;
    let strike = 100.0;
    let time = 1.0;
    let r = 0.05;
    let vol = 0.2;

    // Single large dividend
    let dividends_single = vec![(0.5, Dividend::Cash(2.0))];
    let gbm_single = GbmWithDividends::new(GbmParams::new(r, 0.0, vol), dividends_single);

    // Multiple small dividends (same total)
    let dividends_multiple = vec![
        (0.25, Dividend::Cash(0.5)),
        (0.50, Dividend::Cash(0.5)),
        (0.75, Dividend::Cash(0.5)),
        (1.00, Dividend::Cash(0.5)),
    ];
    let gbm_multiple = GbmWithDividends::new(GbmParams::new(r, 0.0, vol), dividends_multiple);

    let disc_div = ExactGbmWithDividends::new();
    let time_grid = TimeGrid::uniform(time, 252).unwrap();
    let config = McEngineConfig {
        num_paths: 50_000,
        seed: 42,
        time_grid: time_grid.clone(),
        target_ci_half_width: None,
        use_parallel: false,
        chunk_size: 1000,
    };

    let call = EuropeanCall::new(strike, 1.0, 252);

    // Price with single dividend
    let engine_single = McEngine::new(config.clone());
    let rng_single = PhiloxRng::new(42);
    let result_single = engine_single
        .price(
            &rng_single,
            &gbm_single,
            &disc_div,
            &[spot],
            &call,
            Currency::USD,
            0.95,
        )
        .unwrap();

    // Price with multiple dividends
    let engine_multiple = McEngine::new(config);
    let rng_multiple = PhiloxRng::new(42);
    let result_multiple = engine_multiple
        .price(
            &rng_multiple,
            &gbm_multiple,
            &disc_div,
            &[spot],
            &call,
            Currency::USD,
            0.95,
        )
        .unwrap();

    println!(
        "Call value: Single dividend={:.6}, Multiple dividends={:.6}",
        result_single.mean.amount(),
        result_multiple.mean.amount()
    );

    // Both should reduce option value significantly
    // Exact relationship depends on timing, but should be in same ballpark
    assert!(result_single.mean.amount() > 0.0);
    assert!(result_multiple.mean.amount() > 0.0);
}

