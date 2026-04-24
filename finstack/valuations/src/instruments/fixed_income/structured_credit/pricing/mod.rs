//! Pricing and cashflow projection for structured credit instruments.
//!
//! This module contains pure functions for:
//! - Deterministic cashflow simulation
//! - Waterfall execution
//! - Coverage test evaluation
//! - Diversion rule processing
//! - Stochastic pricing

pub(crate) mod coverage_tests;
pub(crate) mod deterministic;
pub(crate) mod diversion;
pub(crate) mod simulation_engine;
pub(crate) mod stochastic;
pub(crate) mod waterfall;

// Re-export deterministic functions
pub use deterministic::{generate_cashflows, generate_tranche_cashflows, run_simulation};
pub use waterfall::{execute_waterfall, execute_waterfall_with_workspace};

// Re-export stochastic types (accessible via stochastic module if needed)
#[allow(unused_imports)] // May be used by external bindings
pub use stochastic::CorrelationStructure;
#[allow(unused_imports)] // May be used by external bindings
pub use stochastic::StochasticDefaultSpec;
#[allow(unused_imports)] // May be used by external bindings
pub use stochastic::StochasticPrepaySpec;
