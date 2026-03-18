//! Factor-model integration helpers for the valuations crate.

mod decompose;
pub mod sensitivity;

pub use decompose::decompose;
pub use sensitivity::{
    DeltaBasedEngine, FactorPnlProfile, FactorSensitivityEngine, FullRepricingEngine, ScenarioGrid,
    SensitivityMatrix,
};
