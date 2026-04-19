//! Factor-model integration helpers for the valuations crate.

mod decompose;
mod positions;
pub mod sensitivity;

pub use decompose::decompose;
pub use positions::{parse_positions_json, pricing_positions, ParsedPosition};
pub use sensitivity::{
    mapping_to_market_bumps, DeltaBasedEngine, FactorPnlProfile, FactorSensitivityEngine,
    FullRepricingEngine, ScenarioGrid, SensitivityMatrix,
};
