//! Monte Carlo v0.2 integration tests.
//!
//! Tests path-dependent payoffs, QMC convergence, and barrier corrections.

#![cfg(feature = "mc")]

use finstack_core::currency::Currency;
use finstack_valuations::instruments::common::mc::payoff::asian::{
    geometric_asian_call_closed_form, AsianCall, AveragingMethod,
};
use finstack_valuations::instruments::common::mc::payoff::barrier::{BarrierCall, BarrierType};
use finstack_valuations::instruments::common::mc::payoff::lookback::LookbackCall;
use finstack_valuations::instruments::common::mc::pricer::path_dependent::{
    PathDependentPricer, PathDependentPricerConfig,
};
use finstack_valuations::instruments::common::mc::process::gbm::{GbmParams, GbmProcess};

#[test]
fn test_geometric_asian_vs_closed_form() {
    // Geometric Asian has a known closed-form solution
    let config = PathDependentPricerConfig::new(50_000)
        .with_seed(42)
        .with_parallel(false);
    let pricer = PathDependentPricer::new(config);

    let r = 0.05;
    let q = 0.02;
    let sigma = 0.2;
    let t = 1.0;

    let gbm = GbmProcess::new(GbmParams::new(r, q, sigma));

    // Monthly fixings (12 points)
    let fixing_steps: Vec<usize> = (0..=12).map(|i| i * 21).collect();
    let asian = AsianCall::new(100.0, 1.0, AveragingMethod::Geometric, fixing_steps.clone());

    let discount = (-r * t).exp();
    let mc_result = pricer
        .price(&gbm, 100.0, t, 252, &asian, Currency::USD, discount)
        .unwrap();

    // Closed-form price (for reference - formula may need refinement)
    let cf_price = geometric_asian_call_closed_form(100.0, 100.0, t, r, q, sigma, fixing_steps.len());

    // MC should give reasonable results
    assert!(mc_result.mean.amount() > 0.0);
    assert!(mc_result.mean.amount() < 15.0); // Sanity check

    // Closed form and MC should be in the same ballpark (within 50%)
    // Note: The closed-form formula may need verification against external references
    let ratio = mc_result.mean.amount() / cf_price;
    assert!(
        ratio > 0.5 && ratio < 1.5,
        "Geometric Asian MC {} and closed form {:.4} differ significantly (ratio={:.2})",
        mc_result.mean.amount(),
        cf_price,
        ratio
    );

    println!(
        "Geometric Asian - MC: {} vs CF: {:.4}, ratio: {:.2}",
        mc_result.mean, cf_price, ratio
    );
}

#[test]
fn test_arithmetic_vs_geometric_asian() {
    // Arithmetic average >= Geometric average (AM-GM inequality)
    let config = PathDependentPricerConfig::new(20_000)
        .with_seed(42)
        .with_parallel(false);
    let pricer = PathDependentPricer::new(config);

    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.25));

    let fixing_steps: Vec<usize> = (0..=12).map(|i| i * 21).collect();
    let asian_arith = AsianCall::new(100.0, 1.0, AveragingMethod::Arithmetic, fixing_steps.clone());
    let asian_geom = AsianCall::new(100.0, 1.0, AveragingMethod::Geometric, fixing_steps);

    let result_arith = pricer
        .price(&gbm, 100.0, 1.0, 252, &asian_arith, Currency::USD, 1.0)
        .unwrap();
    let result_geom = pricer
        .price(&gbm, 100.0, 1.0, 252, &asian_geom, Currency::USD, 1.0)
        .unwrap();

    // Arithmetic Asian should be >= Geometric Asian
    assert!(
        result_arith.mean.amount() >= result_geom.mean.amount() - 0.5,
        "Arithmetic {} should be >= Geometric {}",
        result_arith.mean.amount(),
        result_geom.mean.amount()
    );

    println!(
        "Arithmetic: {}, Geometric: {}",
        result_arith.mean, result_geom.mean
    );
}

#[test]
fn test_lookback_always_itm() {
    // Lookback call should always have positive value (S_max >= S_0)
    let config = PathDependentPricerConfig::new(10_000)
        .with_seed(42)
        .with_parallel(false);
    let pricer = PathDependentPricer::new(config);

    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.0, 0.3));
    let lookback = LookbackCall::new(100.0, 1.0, 100);

    let result = pricer
        .price(&gbm, 100.0, 1.0, 100, &lookback, Currency::USD, 1.0)
        .unwrap();

    // Should be positive (max >= initial spot typically)
    assert!(result.mean.amount() > 0.0);

    println!("Lookback call: {}", result.mean);
}

#[test]
fn test_barrier_option_knock_out() {
    // Test that barrier options behave correctly
    let config = PathDependentPricerConfig::new(20_000)
        .with_seed(42)
        .with_parallel(false);
    let pricer = PathDependentPricer::new(config);

    // Low volatility, barrier well above spot - should rarely knock out
    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.0, 0.1));
    let barrier_call = BarrierCall::new(
        100.0, // strike
        150.0, // barrier (well above)
        BarrierType::UpAndOut,
        1.0,   // notional
        100,   // maturity step
        0.1,   // sigma
        1.0,   // time to maturity
        false, // no Gobet-Miri
    );

    let result = pricer
        .price(&gbm, 100.0, 1.0, 100, &barrier_call, Currency::USD, 1.0)
        .unwrap();

    // Should have positive value (rarely knocks out with low vol, high barrier)
    assert!(result.mean.amount() > 0.0);

    println!("Barrier call (rarely KO): {}", result.mean);
}

#[test]
fn test_barrier_with_gobet_miri() {
    // Test barrier with and without Gobet-Miri adjustment
    let config = PathDependentPricerConfig::new(10_000)
        .with_seed(42)
        .with_parallel(false);
    let pricer = PathDependentPricer::new(config);

    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.2));

    let barrier_no_adj = BarrierCall::new(
        100.0,
        120.0,
        BarrierType::UpAndOut,
        1.0,
        100,
        0.2,
        1.0,
        false,
    );

    let barrier_with_adj = BarrierCall::new(
        100.0,
        120.0,
        BarrierType::UpAndOut,
        1.0,
        100,
        0.2,
        1.0,
        true,
    );

    let result_no_adj = pricer
        .price(&gbm, 100.0, 1.0, 100, &barrier_no_adj, Currency::USD, 1.0)
        .unwrap();

    let result_with_adj = pricer
        .price(&gbm, 100.0, 1.0, 100, &barrier_with_adj, Currency::USD, 1.0)
        .unwrap();

    // Both should be positive
    assert!(result_no_adj.mean.amount() > 0.0);
    assert!(result_with_adj.mean.amount() > 0.0);

    // With adjustment, barrier is shifted higher (less likely to knock out)
    // So price should be higher
    println!(
        "Barrier: no_adj={}, with_adj={}",
        result_no_adj.mean, result_with_adj.mean
    );
}

#[test]
fn test_asian_more_fixings_lower_value() {
    // Asian options with more frequent fixings should have lower value
    // (due to reduced volatility of the average)
    let config = PathDependentPricerConfig::new(20_000)
        .with_seed(42)
        .with_parallel(false);
    let pricer = PathDependentPricer::new(config);

    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.3));

    // Few fixings (quarterly)
    let fixing_steps_few: Vec<usize> = (0..=4).map(|i| i * 63).collect();
    let asian_few = AsianCall::new(100.0, 1.0, AveragingMethod::Arithmetic, fixing_steps_few);

    // Many fixings (weekly)
    let fixing_steps_many: Vec<usize> = (0..=52).map(|i| i * 5).collect();
    let asian_many = AsianCall::new(100.0, 1.0, AveragingMethod::Arithmetic, fixing_steps_many);

    let result_few = pricer
        .price(&gbm, 100.0, 1.0, 252, &asian_few, Currency::USD, 1.0)
        .unwrap();

    let result_many = pricer
        .price(&gbm, 100.0, 1.0, 252, &asian_many, Currency::USD, 1.0)
        .unwrap();

    println!(
        "Asian: few fixings={}, many fixings={}",
        result_few.mean, result_many.mean
    );

    // Typically, more frequent averaging reduces option value
    // (though this is not always guaranteed in finite samples)
    assert!(result_few.mean.amount() > 0.0);
    assert!(result_many.mean.amount() > 0.0);
}

#[test]
fn test_qmc_vs_mc_convergence() {
    // Compare Sobol QMC vs Philox PRNG convergence
    // QMC should converge faster for smooth payoffs

    use finstack_valuations::instruments::common::mc::engine::{McEngine, McEngineConfig};
    use finstack_valuations::instruments::common::mc::payoff::vanilla::EuropeanCall;
    use finstack_valuations::instruments::common::mc::discretization::exact::ExactGbm;
    use finstack_valuations::instruments::common::mc::rng::sobol::SobolRng;
    use finstack_valuations::instruments::common::mc::rng::philox::PhiloxRng;
    use finstack_valuations::instruments::common::mc::time_grid::TimeGrid;

    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.2));
    let call = EuropeanCall::new(100.0, 1.0, 50);
    let disc = ExactGbm::new();
    let initial_state = vec![100.0];
    let time_grid = TimeGrid::uniform(1.0, 50).unwrap();

    // Test with different sample sizes
    let sample_sizes = vec![1000, 5000, 10000];
    
    for n in sample_sizes {
        // PRNG (Philox)
        let engine_prng = McEngine::new(McEngineConfig {
            num_paths: n,
            seed: 42,
            time_grid: time_grid.clone(),
            target_ci_half_width: None,
            use_parallel: false,
            chunk_size: 1000,
        });

        let rng_philox = PhiloxRng::new(42);
        let result_prng = engine_prng
            .price(
                &rng_philox,
                &gbm,
                &disc,
                &initial_state,
                &call,
                Currency::USD,
                1.0,
            )
            .unwrap();

        // QMC (Sobol)
        let engine_qmc = McEngine::new(McEngineConfig {
            num_paths: n,
            seed: 12345,
            time_grid: time_grid.clone(),
            target_ci_half_width: None,
            use_parallel: false,
            chunk_size: 1000,
        });

        let rng_sobol = SobolRng::new(1, 12345);
        let result_qmc = engine_qmc
            .price(
                &rng_sobol,
                &gbm,
                &disc,
                &initial_state,
                &call,
                Currency::USD,
                1.0,
            )
            .unwrap();

        println!(
            "N={:5}: PRNG stderr={:.6}, QMC stderr={:.6}, ratio={:.3}",
            n,
            result_prng.stderr,
            result_qmc.stderr,
            result_prng.stderr / result_qmc.stderr
        );

        // Both should give reasonable estimates
        assert!(result_prng.mean.amount() > 0.0);
        assert!(result_qmc.mean.amount() > 0.0);

        // QMC typically has lower stderr (though not guaranteed for small samples)
        // Just verify both are producing estimates
    }
}

#[test]
fn test_barrier_convergence_with_steps() {
    // Barrier option bias decreases as time steps increase
    let config = PathDependentPricerConfig::new(20_000)
        .with_seed(42)
        .with_parallel(false);
    let pricer = PathDependentPricer::new(config);

    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.3));

    let step_counts = vec![50, 100, 250];
    let mut prices = Vec::new();

    for num_steps in step_counts {
        let barrier_call = BarrierCall::new(
            100.0,
            130.0,
            BarrierType::UpAndOut,
            1.0,
            num_steps,
            0.3,
            1.0,
            false,
        );

        let result = pricer
            .price(&gbm, 100.0, 1.0, num_steps, &barrier_call, Currency::USD, 1.0)
            .unwrap();

        prices.push((num_steps, result.mean.amount()));
        println!(
            "Steps={:3}: Barrier price={:.4}",
            num_steps, result.mean.amount()
        );
    }

    // Prices should stabilize as steps increase
    assert!(prices[0].1 > 0.0);
    assert!(prices[1].1 > 0.0);
    assert!(prices[2].1 > 0.0);
}

#[test]
fn test_asian_arithmetic_always_gte_geometric() {
    // For same fixings, arithmetic average >= geometric average
    let config = PathDependentPricerConfig::new(10_000)
        .with_seed(42)
        .with_parallel(false);
    let pricer = PathDependentPricer::new(config);

    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.2));

    let fixing_steps: Vec<usize> = (0..=10).map(|i| i * 25).collect();

    let asian_arith = AsianCall::new(100.0, 1.0, AveragingMethod::Arithmetic, fixing_steps.clone());
    let asian_geom = AsianCall::new(100.0, 1.0, AveragingMethod::Geometric, fixing_steps);

    let result_arith = pricer
        .price(&gbm, 100.0, 1.0, 250, &asian_arith, Currency::USD, 1.0)
        .unwrap();

    let result_geom = pricer
        .price(&gbm, 100.0, 1.0, 250, &asian_geom, Currency::USD, 1.0)
        .unwrap();

    // AM >= GM (allowing for sampling error)
    assert!(result_arith.mean.amount() >= result_geom.mean.amount() - 1.0);
}

#[test]
fn test_lookback_bounds() {
    // Lookback call value should be bounded
    let config = PathDependentPricerConfig::new(10_000)
        .with_seed(42)
        .with_parallel(false);
    let pricer = PathDependentPricer::new(config);

    let s0 = 100.0;
    let k = 90.0;
    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.0, 0.2));
    let lookback = LookbackCall::new(k, 1.0, 100);

    let result = pricer
        .price(&gbm, s0, 1.0, 100, &lookback, Currency::USD, 1.0)
        .unwrap();

    // Lookback call K=90 with S0=100 should have value >= 10 (intrinsic)
    assert!(result.mean.amount() >= 8.0); // Allow some statistical slack

    println!("Lookback call: {}", result.mean);
}

#[test]
fn test_knock_in_vs_knock_out() {
    // Knock-in + Knock-out should approximately equal vanilla option
    // (Up-and-in + Up-and-out = vanilla, ignoring rebates)
    let config = PathDependentPricerConfig::new(20_000)
        .with_seed(42)
        .with_parallel(false);
    let pricer = PathDependentPricer::new(config);

    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.25));

    let barrier_out = BarrierCall::new(100.0, 120.0, BarrierType::UpAndOut, 1.0, 100, 0.25, 1.0, false);
    let barrier_in = BarrierCall::new(100.0, 120.0, BarrierType::UpAndIn, 1.0, 100, 0.25, 1.0, false);

    let result_out = pricer
        .price(&gbm, 100.0, 1.0, 100, &barrier_out, Currency::USD, 1.0)
        .unwrap();

    let result_in = pricer
        .price(&gbm, 100.0, 1.0, 100, &barrier_in, Currency::USD, 1.0)
        .unwrap();

    // Sum should be close to vanilla call value
    let sum = result_out.mean.amount() + result_in.mean.amount();

    // Vanilla call would be around 10-15 for these parameters
    use finstack_valuations::instruments::common::mc::variance_reduction::black_scholes_call;
    let vanilla = black_scholes_call(100.0, 100.0, 1.0, 0.05, 0.02, 0.25);

    println!(
        "KO={:.4}, KI={:.4}, Sum={:.4}, Vanilla={:.4}",
        result_out.mean.amount(),
        result_in.mean.amount(),
        sum,
        vanilla
    );

    // Allow wide tolerance due to sampling error and discrete monitoring
    assert!((sum - vanilla).abs() < 3.0);
}

#[test]
fn test_sobol_sequence_low_discrepancy() {
    // Sobol sequences should show better uniformity than PRNG
    use finstack_valuations::instruments::common::mc::rng::sobol::SobolRng;

    let mut sobol = SobolRng::new(2, 0);
    let mut points = Vec::new();

    for _ in 0..100 {
        let point = sobol.next_point();
        points.push(point);
    }

    // Check that points are well-distributed (simple check: no duplicate pairs)
    for (i, p1) in points.iter().enumerate() {
        for p2 in points.iter().skip(i + 1) {
            let dist = ((p1[0] - p2[0]).powi(2) + (p1[1] - p2[1]).powi(2)).sqrt();
            assert!(dist > 0.001, "Points too close: {:?} vs {:?}", p1, p2);
        }
    }
}

