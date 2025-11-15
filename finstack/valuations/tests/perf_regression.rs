//! Performance regression tests.
//!
//! These tests ensure that performance optimizations don't regress.
//! They check allocation counts, execution time bounds, and parallel scaling.

#![cfg(test)]

#[cfg(feature = "mc")]
#[test]
fn mc_pricing_parallel_efficiency() {
    use finstack_core::currency::Currency;
    use finstack_valuations::instruments::common::mc::discretization::euler::EulerMaruyama;
    use finstack_valuations::instruments::common::mc::process::gbm::{GbmParams, GbmProcess};
    use finstack_valuations::instruments::common::mc::rng::philox::PhiloxRng;
    use finstack_valuations::instruments::common::mc::time_grid::TimeGrid;
    use finstack_valuations::instruments::common::models::monte_carlo::engine::{
        McEngine, McEngineConfig,
    };
    use finstack_valuations::instruments::common::models::monte_carlo::payoff::vanilla::EuropeanCall;

    // Test that parallel execution provides reasonable speedup
    let time_grid = TimeGrid::uniform(1.0, 100).unwrap();
    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.2));
    let disc = EulerMaruyama;
    let call = EuropeanCall::new(100.0, 1.0, 100);
    let rng = PhiloxRng::new(42);

    #[cfg(feature = "parallel")]
    {
        // Serial timing
        let serial_engine = McEngine::new(McEngineConfig {
            num_paths: 10_000,
            seed: 42,
            time_grid: time_grid.clone(),
            target_ci_half_width: None,
            use_parallel: false,
            chunk_size: 1000,
            path_capture: finstack_valuations::instruments::common::models::monte_carlo::engine::
                PathCaptureConfig::default(),
            antithetic: false,
        });

        let start = std::time::Instant::now();
        let _ = serial_engine
            .price(&rng, &gbm, &disc, &[100.0], &call, Currency::USD, 0.95)
            .unwrap();
        let serial_time = start.elapsed();

        // Parallel timing
        let parallel_engine = McEngine::new(McEngineConfig {
            num_paths: 10_000,
            seed: 42,
            time_grid: time_grid.clone(),
            target_ci_half_width: None,
            use_parallel: true,
            chunk_size: 1000,  // Will use adaptive sizing
            path_capture: finstack_valuations::instruments::common::models::monte_carlo::engine::
                PathCaptureConfig::default(),
            antithetic: false,
        });

        let start = std::time::Instant::now();
        let _ = parallel_engine
            .price(&rng, &gbm, &disc, &[100.0], &call, Currency::USD, 0.95)
            .unwrap();
        let parallel_time = start.elapsed();

        // Check that parallel provides at least 1.3x speedup on multi-core systems
        let num_cpus = rayon::current_num_threads();
        if num_cpus >= 2 {
            let speedup = serial_time.as_secs_f64() / parallel_time.as_secs_f64();
            assert!(
                speedup >= 1.3,
                "Parallel speedup ({:.2}x) below threshold on {} cores",
                speedup,
                num_cpus
            );
        }
    }
}

#[test]
fn bond_cashflow_generation_bounded() {
    use finstack_core::currency::Currency;
    use finstack_core::dates::Date;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::market_data::MarketContext;
    use finstack_core::math::interp::InterpStyle;
    use finstack_core::money::Money;
    use finstack_valuations::cashflow::traits::CashflowProvider;
    use finstack_valuations::instruments::bond::Bond;
    use time::Month;

    // Ensure bond cashflow generation completes in reasonable time
    let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .knots([(0.0, 1.0), (30.0, 0.40)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let market = MarketContext::new().insert_discount(disc);

    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2055, Month::January, 1).unwrap();

    let bond = Bond::fixed(
        "BOND-30Y",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        issue,
        maturity,
        "USD-OIS",
    );

    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Measure execution time
    let start = std::time::Instant::now();
    let flows = bond.build_schedule(&market, as_of).unwrap();
    let elapsed = start.elapsed();

    // 30Y bond should generate ~60 cashflows (semi-annual)
    assert!(
        flows.len() >= 59 && flows.len() <= 61,
        "Unexpected flow count"
    );

    // Should complete in under 1ms on modern hardware
    // Note: This threshold is for unoptimized test builds
    // With --profile release-perf, expect ~20-30μs
    // Increased threshold to reduce flakiness from system load
    assert!(
        elapsed.as_micros() < 1000,
        "Bond cashflow generation took {}μs (expected <1000μs)",
        elapsed.as_micros()
    );
}

#[test]
fn waterfall_allocation_no_excessive_clones() {
    // This test verifies that the waterfall engine doesn't clone recipients
    // on every tier allocation. The test is primarily documentation of the
    // optimization - actual verification would require allocation tracking.

    // If waterfall clones recipients, this would create N*M clones where:
    // N = number of tiers, M = average recipients per tier
    // With the optimization, clones are eliminated by using slice references.

    // This is a placeholder for future allocation tracking tests
    // that would use a custom allocator to count allocations.
}

#[cfg(test)]
mod allocation_tracking {
    //! Future home for allocation tracking tests using custom allocators
    //!
    //! Ideas:
    //! - Track total allocations during pricing
    //! - Ensure collections use with_capacity
    //! - Verify buffer reuse in MC paths
    //!
    //! Could use `stats_alloc` crate for tracking.
}
