//! Property-based tests for market standards compliance.
//!
//! These tests use proptest to verify mathematical invariants hold across
//! a wide range of inputs. Property tests complement unit tests by:
//! - Testing thousands of random but valid input combinations
//! - Catching edge cases that manual tests might miss
//! - Verifying mathematical relationships hold universally
//!
//! All tests in this module should:
//! - Define clear properties/invariants to verify
//! - Use appropriate input generators with realistic bounds
//! - Include shrinking for minimal counter-examples
//! - Run for sufficient iterations (default: 256 cases)

mod test_curve_monotonicity;
mod test_forward_parity;
mod test_option_bounds;
mod test_swap_symmetry;
