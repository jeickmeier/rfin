//! Equity option pricing facade.
//!
//! Exposes the core pricing engine for `EquityOption`, keeping numerics
//! out of the instrument type and enabling reuse by metrics.

pub mod engine;
pub mod pricer;
