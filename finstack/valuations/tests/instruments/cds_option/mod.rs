//! CDS Option comprehensive test suite.
//!
//! Test organization follows market-standard practices:
//!
//! - `common`: Shared fixtures and utilities for DRY test setup
//! - Unit tests: Individual component testing
//!   - `test_parameters`: Builder and parameter validation
//!   - `test_types`: CdsOption struct construction and methods
//! - Integration tests: End-to-end workflows
//!   - `test_pricing`: Full pricing scenarios
//!   - `test_greeks`: Greeks calculations and sensitivities
//!   - `test_implied_vol`: Implied volatility solver
//! - Market validation: Financial theory compliance
//!   - `test_black_model_properties`: Black-76 model properties
//!   - `test_option_bounds`: Value bounds and no-arbitrage conditions
//!   - `test_moneyness`: ITM/ATM/OTM behavior
//!   - `test_index_options`: Index-specific features
//! - Metrics tests:
//!   - `test_metrics_registry`: Metric framework integration

mod common;

// Unit tests
mod test_parameters;
mod test_types;

// Integration tests
mod test_greeks;
mod test_implied_vol;
mod test_pricing;

// Market validation tests
mod test_black_model_properties;
mod test_index_options;
mod test_moneyness;
mod test_option_bounds;

// Metrics tests
mod test_metrics_registry;

// QuantLib parity tests
mod quantlib_parity;
