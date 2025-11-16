//! Bond pricing entrypoints and pricers.
//!
//! Bond pricing methods are now included in the explicit Instrument trait implementation.

/// Bond pricing engine (valuation logic)
pub mod engine;
pub mod helpers;
/// Bond pricer implementation (registry integration)
pub mod pricer;
pub mod schedule_helpers;
/// Tree-based pricing for callable/putable bonds and OAS
pub mod tree_pricer;
pub mod ytm_solver;
/// Quote engine for mapping between price, yields, and spreads
pub mod quote_engine;
