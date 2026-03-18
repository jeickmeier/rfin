//! Portfolio-level factor risk decomposition outputs and engines.

mod parametric;
mod simulation;
mod traits;
mod types;

pub use parametric::ParametricDecomposer;
pub use simulation::SimulationDecomposer;
pub use traits::RiskDecomposer;
pub use types::{FactorContribution, PositionFactorContribution, RiskDecomposition};
