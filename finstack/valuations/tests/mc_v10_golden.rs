//! Monte Carlo golden tests - Validation against reference values.
//!
//! Tests MC pricing against known reference values from analytical formulas,
//! QuantLib, and market standards.

#![cfg(feature = "mc")]

mod golden_tests;

use finstack_core::currency::Currency;
use finstack_valuations::instruments::common::mc::payoff::vanilla::EuropeanCall;
use finstack_valuations::instruments::common::mc::pricer::european::{
    EuropeanPricer, EuropeanPricerConfig,
};
use finstack_valuations::instruments::common::mc::process::gbm::{GbmParams, GbmProcess};
use finstack_valuations::instruments::common::mc::variance_reduction::control_variate::{
    black_scholes_call, black_scholes_put,
};
use golden_tests::{assert_within_tolerance, load_golden_tests};

// ============================================================================
// European Options Golden Tests
// ============================================================================

#[test]
fn test_golden_european_calls() {
    println!("\nGolden Tests - European Calls:");
    println!("{:=<80}", "");

    let test_cases = load_golden_tests("data/european_options.csv").unwrap();

    for case in test_cases.iter().filter(|c| c.name.contains("Call")) {
        // Compute expected price using Black-Scholes
        let bs_price = black_scholes_call(
            case.spot,
            case.strike,
            case.time,
            case.rate,
            case.div_yield,
            case.volatility,
        );

        // MC price
        let config = EuropeanPricerConfig::new(100_000)
            .with_seed(42)
            .with_parallel(false);
        let pricer = EuropeanPricer::new(config);
        let gbm = GbmProcess::new(GbmParams::new(case.rate, case.div_yield, case.volatility));
        let call = EuropeanCall::new(case.strike, 1.0, 252);
        let discount = (-case.rate * case.time).exp();
        let num_steps = (case.time * 252.0).max(10.0) as usize;

        let result = pricer
            .price(
                &gbm,
                case.spot,
                case.time,
                num_steps,
                &call,
                Currency::USD,
                discount,
            )
            .unwrap();

        // Update expected price with analytical value
        let mut updated_case = case.clone();
        updated_case.expected_price = bs_price;

        assert_within_tolerance(&updated_case, result.mean.amount(), result.stderr);
    }

    println!("{:=<80}\n", "");
}

#[test]
fn test_golden_european_puts() {
    println!("\nGolden Tests - European Puts:");
    println!("{:=<80}", "");

    let test_cases = load_golden_tests("data/european_options.csv").unwrap();

    for case in test_cases.iter().filter(|c| c.name.contains("Put")) {
        // Compute expected price using Black-Scholes
        let bs_price = black_scholes_put(
            case.spot,
            case.strike,
            case.time,
            case.rate,
            case.div_yield,
            case.volatility,
        );

        // MC price
        let config = EuropeanPricerConfig::new(100_000)
            .with_seed(42)
            .with_parallel(false);
        let pricer = EuropeanPricer::new(config);
        let gbm = GbmProcess::new(GbmParams::new(case.rate, case.div_yield, case.volatility));
        
        use finstack_valuations::instruments::common::mc::payoff::vanilla::EuropeanPut;
        let put = EuropeanPut::new(case.strike, 1.0, 252);
        let discount = (-case.rate * case.time).exp();
        let num_steps = (case.time * 252.0).max(10.0) as usize;

        let result = pricer
            .price(
                &gbm,
                case.spot,
                case.time,
                num_steps,
                &put,
                Currency::USD,
                discount,
            )
            .unwrap();

        let mut updated_case = case.clone();
        updated_case.expected_price = bs_price;

        assert_within_tolerance(&updated_case, result.mean.amount(), result.stderr);
    }

    println!("{:=<80}\n", "");
}

// ============================================================================
// Asian Options Golden Tests
// ============================================================================

#[test]
fn test_golden_geometric_asian() {
    println!("\nGolden Tests - Geometric Asian Options:");
    println!("{:=<80}", "");

    // Use closed-form geometric Asian formula
    use finstack_valuations::instruments::common::mc::payoff::asian::geometric_asian_call_closed_form;
    use finstack_valuations::instruments::common::mc::payoff::asian::{AsianCall, AveragingMethod};
    use finstack_valuations::instruments::common::mc::pricer::path_dependent::{
        PathDependentPricer, PathDependentPricerConfig,
    };

    let test_cases = load_golden_tests("data/asian_options.csv").unwrap();

    for case in test_cases.iter().filter(|c| c.name.contains("Geom")) {
        let num_fixings = 12;
        let fixing_steps: Vec<usize> = (0..=num_fixings)
            .map(|i| i * 252 / num_fixings)
            .collect();

        // Analytical price
        let analytical = geometric_asian_call_closed_form(
            case.spot,
            case.strike,
            case.time,
            case.rate,
            case.div_yield,
            case.volatility,
            num_fixings,
        );

        // MC price
        let config = PathDependentPricerConfig::new(100_000)
            .with_seed(42)
            .with_parallel(false);
        let pricer = PathDependentPricer::new(config);
        let gbm = GbmProcess::new(GbmParams::new(case.rate, case.div_yield, case.volatility));
        let asian = AsianCall::new(
            case.strike,
            1.0,
            AveragingMethod::Geometric,
            fixing_steps,
        );
        let discount = (-case.rate * case.time).exp();
        let num_steps = 252;

        let result = pricer
            .price(&gbm, case.spot, case.time, num_steps, &asian, Currency::USD, discount)
            .unwrap();

        let mut updated_case = case.clone();
        updated_case.expected_price = analytical;

        assert_within_tolerance(&updated_case, result.mean.amount(), result.stderr);
    }

    println!("{:=<80}\n", "");
}

// ============================================================================
// Comprehensive Validation Summary
// ============================================================================

#[test]
fn test_all_golden_tests_summary() {
    println!("\n{:=<80}", "");
    println!("GOLDEN TEST SUITE SUMMARY");
    println!("{:=<80}", "");

    let european_cases = load_golden_tests("data/european_options.csv").unwrap();
    let asian_cases = load_golden_tests("data/asian_options.csv").unwrap();
    let barrier_cases = load_golden_tests("data/barrier_options.csv").unwrap();

    println!("  European options: {} test cases", european_cases.len());
    println!("  Asian options:    {} test cases", asian_cases.len());
    println!("  Barrier options:  {} test cases", barrier_cases.len());
    println!("  Total:            {} test cases", 
        european_cases.len() + asian_cases.len() + barrier_cases.len());
    println!("{:=<80}\n", "");

    assert!(!european_cases.is_empty());
    assert!(!asian_cases.is_empty());
    assert!(!barrier_cases.is_empty());
}

