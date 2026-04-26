//! Integration tests for P&L attribution.
//!
//! ## Test Modules
//!
//! - `bond_attribution`: Basic bond P&L attribution tests
//! - `fx_attribution`: FX translation and waterfall attribution tests
//! - `invariants`: Mathematical invariants (sign conventions, scaling, edge cases)
//! - `metrics_based_convexity`: Second-order metrics support
//! - `model_params_attribution`: Prepayment, default, recovery attribution
//! - `quantlib_parity`: Analytical formula validation
//! - `scalars_attribution`: Market scalars extraction/restoration
//! - `serialization_roundtrip`: JSON serialization tests

mod bond_attribution;
mod credit_factor_linear;
mod factors_snapshot;
mod fx_attribution;
mod invariants;
mod metrics_based_convexity;
mod model_params_attribution;
mod quantlib_parity;
mod scalars_attribution;
mod serialization_roundtrip;
mod types_pnl;

// Note: rounding_policy.rs and spec_tests.rs use pub(crate) test utilities
// or have other issues and are tested within the library via `cargo test`
// (not as integration tests)
