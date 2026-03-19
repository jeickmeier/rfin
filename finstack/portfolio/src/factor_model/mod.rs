//! Portfolio-level factor risk decomposition outputs and engines.

mod assignment;
mod model;
mod optimization;
mod parametric;
mod simulation;
mod traits;
mod types;
mod whatif;

pub use assignment::{FactorAssignmentReport, PositionAssignment, UnmatchedEntry};
pub use model::{FactorModel, FactorModelBuilder};
pub use optimization::{FactorConstraint, FactorOptimizationResult};
pub use parametric::ParametricDecomposer;
pub use simulation::SimulationDecomposer;
pub use traits::RiskDecomposer;
pub use types::{FactorContribution, PositionFactorContribution, RiskDecomposition};
pub use whatif::{
    FactorContributionDelta, PositionChange, StressResult, WhatIfEngine, WhatIfResult,
};
