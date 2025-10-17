//! QuantLib Parity Testing Framework
//!
//! This module provides common infrastructure for comparing finstack valuations
//! with QuantLib test suite results to ensure feature parity.
//!
//! # QuantLib Reference
//!
//! - Version: 1.34 (2024)
//! - Test Suite: https://github.com/lballabio/QuantLib/tree/master/test-suite
//!
//! # Tolerance Configuration
//!
//! Default tolerance is 0.01% relative (1 basis point), which is appropriate for
//! most financial calculations. This accounts for:
//! - Rounding differences between Decimal (finstack) and double (QuantLib)
//! - Minor numerical method differences
//! - Day count convention edge cases
//!
//! Tolerance can be tightened to 6 decimal places for high-precision validation.

use std::fmt;

/// Configuration for parity test tolerance
#[derive(Debug, Clone, Copy)]
pub struct ParityConfig {
    /// Relative tolerance (e.g., 0.0001 for 0.01%)
    pub relative_tolerance: f64,
    /// Absolute tolerance for near-zero values
    pub absolute_tolerance: f64,
    /// Whether to use decimal place matching instead of relative tolerance
    pub use_decimal_places: Option<usize>,
}

impl Default for ParityConfig {
    /// Default configuration: 0.01% relative tolerance (1 basis point)
    fn default() -> Self {
        Self {
            relative_tolerance: 0.0001, // 0.01%
            absolute_tolerance: 1e-8,
            use_decimal_places: None,
        }
    }
}

impl ParityConfig {

    /// Create configuration with specific relative tolerance
    pub fn with_relative_tolerance(tolerance: f64) -> Self {
        Self {
            relative_tolerance: tolerance,
            absolute_tolerance: 1e-8,
            use_decimal_places: None,
        }
    }

    /// Create configuration with decimal place matching (e.g., 6 decimals)
    pub fn with_decimal_places(places: usize) -> Self {
        let tolerance = 10_f64.powi(-(places as i32));
        Self {
            relative_tolerance: 0.0,
            absolute_tolerance: tolerance,
            use_decimal_places: Some(places),
        }
    }

    /// Tight tolerance for high-precision tests (0.001% = 0.1 basis points)
    pub fn tight() -> Self {
        Self {
            relative_tolerance: 0.00001, // 0.001%
            absolute_tolerance: 1e-10,
            use_decimal_places: None,
        }
    }

    /// Loose tolerance for tests with known numerical instabilities (0.1% = 10 basis points)
    pub fn loose() -> Self {
        Self {
            relative_tolerance: 0.001, // 0.1%
            absolute_tolerance: 1e-6,
            use_decimal_places: None,
        }
    }
}

/// Result of a parity comparison
#[derive(Debug)]
pub struct ParityResult {
    pub passed: bool,
    pub finstack_value: f64,
    pub quantlib_value: f64,
    pub difference: f64,
    pub relative_diff: f64,
    pub tolerance_used: ParityConfig,
}

impl fmt::Display for ParityResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ParityResult {{ passed: {}, finstack: {:.10}, quantlib: {:.10}, diff: {:.2e}, rel_diff: {:.4}% }}",
            self.passed,
            self.finstack_value,
            self.quantlib_value,
            self.difference,
            self.relative_diff * 100.0
        )
    }
}

/// Compare two numeric values with configured tolerance
pub fn compare_values(
    finstack_value: f64,
    quantlib_value: f64,
    config: ParityConfig,
) -> ParityResult {
    let difference = (finstack_value - quantlib_value).abs();
    let relative_diff = if quantlib_value.abs() > config.absolute_tolerance {
        difference / quantlib_value.abs()
    } else {
        0.0
    };

    let passed = if let Some(_places) = config.use_decimal_places {
        // Use absolute tolerance based on decimal places
        difference <= config.absolute_tolerance
    } else {
        // Use relative tolerance, falling back to absolute for near-zero values
        if quantlib_value.abs() < config.absolute_tolerance {
            difference <= config.absolute_tolerance
        } else {
            relative_diff <= config.relative_tolerance
        }
    };

    ParityResult {
        passed,
        finstack_value,
        quantlib_value,
        difference,
        relative_diff,
        tolerance_used: config,
    }
}

/// Compare Decimal value with f64 QuantLib value
pub fn compare_decimal(
    finstack_value: f64,
    quantlib_value: f64,
    config: ParityConfig,
) -> ParityResult {
    compare_values(finstack_value, quantlib_value, config)
}

/// Assert that two values match within tolerance
#[allow(unused_macros)]
macro_rules! assert_parity {
    ($finstack:expr, $quantlib:expr) => {
        assert_parity!($finstack, $quantlib, Default::default())
    };
    ($finstack:expr, $quantlib:expr, $config:expr) => {
        {
            let result = compare_values($finstack, $quantlib, $config);
            assert!(
                result.passed,
                "Parity check failed:\n  Finstack: {:.10}\n  QuantLib: {:.10}\n  Difference: {:.2e}\n  Rel Diff: {:.4}%\n  Tolerance: {:.4}%",
                result.finstack_value,
                result.quantlib_value,
                result.difference,
                result.relative_diff * 100.0,
                result.tolerance_used.relative_tolerance * 100.0
            );
            result
        }
    };
    ($finstack:expr, $quantlib:expr, $config:expr, $msg:expr) => {
        {
            let result = compare_values($finstack, $quantlib, $config);
            assert!(
                result.passed,
                "Parity check failed for '{}': {}", $msg, result
            );
            result
        }
    };
}

/// Assert parity for Decimal types
#[allow(unused_macros)]
macro_rules! assert_parity_decimal {
    ($finstack:expr, $quantlib:expr) => {
        assert_parity_decimal!($finstack, $quantlib, Default::default())
    };
    ($finstack:expr, $quantlib:expr, $config:expr) => {
        {
            let result = compare_decimal($finstack, $quantlib, $config);
            assert!(
                result.passed,
                "Parity check failed (Decimal):\n  Finstack: {:.10}\n  QuantLib: {:.10}\n  Difference: {:.2e}\n  Rel Diff: {:.4}%",
                result.finstack_value,
                result.quantlib_value,
                result.difference,
                result.relative_diff * 100.0
            );
            result
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parity_exact_match() {
        let result = compare_values(100.0, 100.0, ParityConfig::default());
        assert!(result.passed);
        assert_eq!(result.difference, 0.0);
    }

    #[test]
    fn test_parity_within_tolerance() {
        // 0.005% difference, within 0.01% tolerance
        let result = compare_values(100.005, 100.0, ParityConfig::default());
        assert!(result.passed);
    }

    #[test]
    fn test_parity_exceeds_tolerance() {
        // 0.02% difference, exceeds 0.01% tolerance
        let result = compare_values(100.02, 100.0, ParityConfig::default());
        assert!(!result.passed);
    }

    #[test]
    fn test_parity_decimal_places() {
        // Exact to 6 decimal places
        let config = ParityConfig::with_decimal_places(6);
        let result = compare_values(100.0000005, 100.0, config);
        assert!(result.passed);

        // Different at 6 decimal places
        let result = compare_values(100.000005, 100.0, config);
        assert!(!result.passed);
    }

    #[test]
    fn test_parity_near_zero() {
        // Near-zero values use absolute tolerance
        let result = compare_values(1e-9, 0.0, ParityConfig::default());
        assert!(result.passed);
    }

    #[test]
    fn test_assert_parity_macro() {
        assert_parity!(100.0, 100.0);
        assert_parity!(100.005, 100.0);
    }

    #[test]
    #[should_panic]
    fn test_assert_parity_macro_fails() {
        assert_parity!(100.02, 100.0);
    }

    #[test]
    fn test_tight_tolerance() {
        let config = ParityConfig::tight();
        let result = compare_values(100.0005, 100.0, config);
        assert!(result.passed);
        
        let result = compare_values(100.002, 100.0, config);
        assert!(!result.passed);
    }

    #[test]
    fn test_loose_tolerance() {
        let config = ParityConfig::loose();
        let result = compare_values(100.05, 100.0, config);
        assert!(result.passed);
        
        let result = compare_values(100.2, 100.0, config);
        assert!(!result.passed);
    }
}

