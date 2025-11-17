//! Bond pricing entrypoints and pricers.
//!
//! Bond pricing methods are now included in the explicit Instrument trait implementation.

/// Bond pricing engine (discount curve-based valuation logic)
pub mod discount_engine;
/// Bond pricer implementation (registry integration)
pub mod pricer;
/// Quote engine for mapping between price, yields, and spreads
pub mod quote_engine;
/// Tree-based pricing for callable/putable bonds and OAS
pub mod tree_engine;
pub mod ytm_solver;
