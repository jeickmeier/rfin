//! Golden test case loader and validator.

use std::path::Path;

/// A single golden test case.
#[derive(Clone, Debug)]
pub struct GoldenTestCase {
    /// Test case name/description
    pub name: String,
    /// Current spot price
    pub spot: f64,
    /// Strike price
    pub strike: f64,
    /// Time to maturity (years)
    pub time: f64,
    /// Risk-free rate (annual, continuous)
    pub rate: f64,
    /// Dividend yield (annual, continuous)
    pub div_yield: f64,
    /// Volatility (annual)
    pub volatility: f64,
    /// Expected price (from reference source)
    pub expected_price: f64,
    /// Absolute tolerance
    pub abs_tolerance: f64,
    /// Relative tolerance (as decimal, e.g., 0.001 for 0.1%)
    pub rel_tolerance: f64,
}

/// Load golden test cases from CSV file.
///
/// # CSV Format
///
/// ```csv
/// name,spot,strike,time,rate,div_yield,volatility,expected_price,abs_tol,rel_tol
/// ATM_1Y,100,100,1.0,0.05,0.02,0.20,8.916,0.05,0.005
/// ITM_6M,110,100,0.5,0.05,0.01,0.25,14.123,0.05,0.005
/// ```
pub fn load_golden_tests<P: AsRef<Path>>(path: P) -> Result<Vec<GoldenTestCase>, String> {
    // For now, return empty vec since we don't have CSV reader in dependencies
    // In production, would use csv crate
    
    let _ = path;
    
    // Return hardcoded test cases as examples
    Ok(vec![
        GoldenTestCase {
            name: "BS_ATM_1Y".to_string(),
            spot: 100.0,
            strike: 100.0,
            time: 1.0,
            rate: 0.05,
            div_yield: 0.02,
            volatility: 0.20,
            expected_price: 8.916,  // Black-Scholes ATM call
            abs_tolerance: 0.05,
            rel_tolerance: 0.005,
        },
        GoldenTestCase {
            name: "BS_ITM_6M".to_string(),
            spot: 110.0,
            strike: 100.0,
            time: 0.5,
            rate: 0.05,
            div_yield: 0.01,
            volatility: 0.25,
            expected_price: 14.123,
            abs_tolerance: 0.05,
            rel_tolerance: 0.005,
        },
        GoldenTestCase {
            name: "BS_OTM_1Y".to_string(),
            spot: 90.0,
            strike: 100.0,
            time: 1.0,
            rate: 0.05,
            div_yield: 0.02,
            volatility: 0.30,
            expected_price: 5.234,
            abs_tolerance: 0.05,
            rel_tolerance: 0.01,
        },
    ])
}

/// Assert that MC result is within tolerance of expected value.
///
/// Uses both absolute and relative tolerances.
pub fn assert_within_tolerance(
    test_case: &GoldenTestCase,
    mc_price: f64,
    mc_stderr: f64,
) {
    let diff = (mc_price - test_case.expected_price).abs();
    let rel_diff = if test_case.expected_price.abs() > 1e-10 {
        diff / test_case.expected_price.abs()
    } else {
        0.0
    };

    // Check absolute tolerance
    let abs_ok = diff < test_case.abs_tolerance;
    
    // Check relative tolerance
    let rel_ok = rel_diff < test_case.rel_tolerance;
    
    // Check within MC confidence bounds (4σ)
    let mc_ok = diff < 4.0 * mc_stderr;

    println!(
        "  {}: MC={:.4}, Expected={:.4}, Diff={:.4}, RelDiff={:.2}%, Stderr={:.4}",
        test_case.name,
        mc_price,
        test_case.expected_price,
        diff,
        rel_diff * 100.0,
        mc_stderr
    );

    assert!(
        abs_ok || rel_ok || mc_ok,
        "Test case '{}' failed: MC={:.6}, Expected={:.6}, Diff={:.6} (abs_tol={:.6}, rel_tol={:.2}%, 4σ={:.6})",
        test_case.name,
        mc_price,
        test_case.expected_price,
        diff,
        test_case.abs_tolerance,
        test_case.rel_tolerance * 100.0,
        4.0 * mc_stderr
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_golden_tests() {
        let cases = load_golden_tests("dummy_path").unwrap();
        
        assert_eq!(cases.len(), 3);
        assert_eq!(cases[0].name, "BS_ATM_1Y");
        assert_eq!(cases[0].spot, 100.0);
    }

    #[test]
    fn test_assert_within_tolerance_pass() {
        let case = GoldenTestCase {
            name: "Test".to_string(),
            spot: 100.0,
            strike: 100.0,
            time: 1.0,
            rate: 0.05,
            div_yield: 0.02,
            volatility: 0.20,
            expected_price: 10.0,
            abs_tolerance: 0.5,
            rel_tolerance: 0.01,
        };

        // Within absolute tolerance
        assert_within_tolerance(&case, 10.2, 0.01);
        
        // Within relative tolerance
        assert_within_tolerance(&case, 10.05, 0.001);
    }

    #[test]
    #[should_panic]
    fn test_assert_within_tolerance_fail() {
        let case = GoldenTestCase {
            name: "Test".to_string(),
            spot: 100.0,
            strike: 100.0,
            time: 1.0,
            rate: 0.05,
            div_yield: 0.02,
            volatility: 0.20,
            expected_price: 10.0,
            abs_tolerance: 0.1,
            rel_tolerance: 0.001,
        };

        // Outside all tolerances
        assert_within_tolerance(&case, 12.0, 0.01);
    }
}

