//! Prescribed FRTB risk weights, correlations, and other regulatory parameters.
//!
//! The per-risk-class submodules (`commodity`, `csr`, `equity`, `fx`,
//! `girr`) expose `pub const` tables matching BCBS d457 (January 2019)
//! and are read directly by the charge-calculation helpers.
//!
//! [`registry::FrtbParams`] bundles the same values into a serializable,
//! revision-tagged struct with a JSON-overlay loader and range
//! validation so alternate parameter sets (e.g. d554) can be tested
//! without recompiling.

pub mod commodity;
pub mod correlation_scenarios;
pub mod csr;
pub mod equity;
pub mod fx;
pub mod girr;
pub mod registry;

pub use registry::{
    CommodityParams, CorrelationScenarioParams, EquityParams, FrtbParams, FrtbRevision, FxParams,
    GirrParams,
};
