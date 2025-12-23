//! Integration tests for the golden test loader infrastructure.
//!
//! These tests verify that the CSV-based golden test loader works correctly
//! with the test data files in `tests/golden_tests/data/`.

#[path = "golden_tests/mod.rs"]
mod golden_tests;

pub use golden_tests::{
    golden_data_dir, load_golden_tests, load_barrier_tests, load_asian_tests,
    GoldenTestCase, OptionType, BarrierTestCase, BarrierType, AsianTestCase, AveragingType,
};

/// Smoke test: load European option test cases from CSV
#[test]
fn test_european_options_csv_smoke() {
    let path = golden_data_dir().join("european_options.csv");
    let cases = load_golden_tests(&path).expect("Failed to load European options");
    
    // Should have 7 test cases
    assert_eq!(cases.len(), 7, "Expected 7 European option test cases");
    
    // Verify structure of first case
    let first = &cases[0];
    assert_eq!(first.name, "BS_ATM_1Y_Call");
    assert_eq!(first.spot, 100.0);
    assert_eq!(first.strike, 100.0);
    assert_eq!(first.time, 1.0);
    assert_eq!(first.rate, 0.05);
    assert_eq!(first.div_yield, 0.02);
    assert_eq!(first.volatility, 0.20);
    assert!((first.expected_price - 8.916).abs() < 0.001);
    assert_eq!(first.option_type, OptionType::Call);
}

/// Smoke test: load barrier option test cases from CSV
#[test]
fn test_barrier_options_csv_smoke() {
    let path = golden_data_dir().join("barrier_options.csv");
    let cases = load_barrier_tests(&path).expect("Failed to load barrier options");
    
    // Should have 4 test cases
    assert_eq!(cases.len(), 4, "Expected 4 barrier option test cases");
    
    // Verify all barrier types are present
    let types: Vec<_> = cases.iter().map(|c| c.barrier_type).collect();
    assert!(types.contains(&BarrierType::UpOut));
    assert!(types.contains(&BarrierType::UpIn));
    assert!(types.contains(&BarrierType::DownOut));
    assert!(types.contains(&BarrierType::DownIn));
}

/// Smoke test: load Asian option test cases from CSV
#[test]
fn test_asian_options_csv_smoke() {
    let path = golden_data_dir().join("asian_options.csv");
    let cases = load_asian_tests(&path).expect("Failed to load Asian options");
    
    // Should have 4 test cases
    assert_eq!(cases.len(), 4, "Expected 4 Asian option test cases");
    
    // Verify both averaging types are present
    let has_geom = cases.iter().any(|c| c.averaging == AveragingType::Geometric);
    let has_arith = cases.iter().any(|c| c.averaging == AveragingType::Arithmetic);
    assert!(has_geom, "Should have geometric averaging cases");
    assert!(has_arith, "Should have arithmetic averaging cases");
}

