//! Determinism test suite for market standards compliance.
//!
//! These tests verify:
//! 1. **Determinism**: Same inputs produce bitwise-identical outputs
//! 2. **Correctness**: Outputs match expected market-standard values
//!
//! All tests in this module should:
//! - Use fixed seeds for any randomness
//! - Verify bitwise identity (not just approximate equality)
//! - Run the same calculation multiple times (typically 10-100 iterations)
//! - Cover major instrument types and pricing paths
//! - Validate results against theoretical expectations
//!
//! All tests use the standardized tolerances defined below.

/// Tolerance tiers for different test categories.
///
/// These mirror the tolerances from `instruments/common/test_helpers.rs`
/// to ensure consistency across the test suite.
pub mod tolerances {
    /// Analytical calculations (e.g., put-call parity, zero-coupon YTM).
    /// These have closed-form solutions and should be very precise.
    #[allow(dead_code)]
    pub const ANALYTICAL: f64 = 1e-6; // 0.0001%

    /// Numerical methods (e.g., tree pricing, Newton-Raphson solvers).
    /// These involve iterative convergence and may have small residual errors.
    pub const NUMERICAL: f64 = 1e-4; // 0.01%

    /// Curve-based pricing with potential convention mismatches.
    /// Accounts for compounding convention differences (e.g., semi-annual vs continuous).
    #[allow(dead_code)]
    pub const CURVE_PRICING: f64 = 5e-3; // 0.5%

    /// Statistical/Monte Carlo methods.
    /// These have inherent sampling variance.
    #[allow(dead_code)]
    pub const STATISTICAL: f64 = 1e-2; // 1%

    /// Relative tolerance for scaling comparisons.
    /// Used when comparing proportional changes across different scales.
    #[allow(dead_code)]
    pub const RELATIVE: f64 = 1e-2; // 1%
}

mod test_bond_pricing;
mod test_calibration;
mod test_cds_pricing;
mod test_option_pricing;
mod test_swap_pricing;
