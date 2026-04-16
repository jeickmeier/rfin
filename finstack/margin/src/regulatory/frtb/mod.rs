//! FRTB Sensitivity-Based Approach (SBA) for standardized market risk capital.
//!
//! Implements the Basel III standardized market risk capital charge per
//! BCBS d457. Computes delta, vega, curvature, default risk, and residual
//! risk add-on components across prescribed risk classes.

pub mod aggregation;
pub mod curvature;
pub mod delta;
pub mod drc;
pub mod engine;
pub mod params;
pub mod rrao;
pub mod types;
pub mod vega;

pub use engine::{FrtbSbaEngine, FrtbSbaEngineBuilder};
pub use types::{
    CorrelationScenario, DrcAssetType, DrcPosition, DrcSector, DrcSeniority, FrtbRiskClass,
    FrtbSbaResult, FrtbSensitivities, RraoPosition,
};
