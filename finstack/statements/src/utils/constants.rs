//! Shared numeric constants for the statements crate.
//!
//! This module centralizes floating-point constants used across the crate to ensure
//! consistent numerical behavior in comparisons, near-zero checks, and precision guards.

/// Epsilon value for floating-point comparisons and near-zero detection.
///
/// # Value: 1e-10
///
/// This epsilon is used throughout the statements crate for:
///
/// - **Equality comparisons** (`==`, `!=`): Values within ±1e-10 are considered equal
/// - **Near-zero guards**: Division, percentage change, and variance ratio operations
/// - **Numerical stability**: Protecting against catastrophic cancellation
///
/// # Precision Guarantees
///
/// - Rate/ratio comparisons: ±0.01 basis points (0.0001%)
/// - Interest rate comparisons: sub-basis point precision
/// - Monetary values: Suitable for values up to ~$1 trillion before precision loss
///
/// # Examples
///
/// ```text
/// 100_000_000.0 == 100_000_000.0000000001  // true (within epsilon)
/// 100_000_000.0 == 100_000_001.0            // false (exceeds epsilon)
/// 0.0001 == 0.00010000000001                // true (within epsilon)
/// ```
///
/// # Usage
///
/// ```rust,ignore
/// use finstack_statements::utils::constants::EPSILON;
///
/// // Near-zero check for safe division
/// if denominator.abs() < EPSILON {
///     return 0.0; // or handle gracefully
/// }
///
/// // Approximate equality
/// if (a - b).abs() < EPSILON {
///     // treat as equal
/// }
/// ```
///
/// # Note
///
/// For currency-safe comparisons with explicit rounding, use the `Money` type from
/// `finstack_core::money` instead of raw f64 comparisons.
pub const EPSILON: f64 = 1e-10;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epsilon_is_correct_value() {
        assert_eq!(EPSILON, 1e-10);
    }

    #[test]
    fn epsilon_provides_sub_basis_point_precision() {
        // 1 basis point = 0.0001 = 1e-4
        // Our epsilon is 1e-10, which is 6 orders of magnitude smaller
        // This ensures sub-basis-point precision for rate comparisons
        let one_basis_point = 0.0001;
        // EPSILON (1e-10) should be much smaller than 1 basis point (1e-4)
        // In fact, EPSILON is 1 millionth of a basis point
        assert!(EPSILON < one_basis_point);
        assert!(EPSILON <= one_basis_point / 1_000_000.0);
    }
}
