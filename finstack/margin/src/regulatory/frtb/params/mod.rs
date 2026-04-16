//! Prescribed FRTB risk weights, correlations, and other regulatory parameters.
//!
//! All values are `const` tables compiled into the binary. They are
//! regulator-specified and not configurable at runtime.

pub mod commodity;
pub mod correlation_scenarios;
pub mod csr;
pub mod equity;
pub mod fx;
pub mod girr;
