//! Portfolio-level factor risk decomposition outputs and engines.
//!
//! This module lifts instrument-level market dependencies and sensitivities into
//! portfolio-level factor analytics. Typical usage is:
//!
//! 1. Build a [`FactorModel`] from a declarative
//!    [`finstack_core::factor_model::FactorModelConfig`].
//! 2. Use [`FactorModel::assign_factors`] to inspect how portfolio positions map
//!    to configured factors.
//! 3. Use [`FactorModel::compute_sensitivities`] to produce a weighted
//!    sensitivity matrix.
//! 4. Use [`FactorModel::analyze`] to decompose portfolio risk.
//!
//! The module exposes both closed-form covariance-based decomposition
//! ([`ParametricDecomposer`]) and simulation-based tail-risk decomposition
//! ([`SimulationDecomposer`]). All engines assume the upstream sensitivity
//! engine has already scaled rows by position quantity, so downstream
//! decomposition works on portfolio exposures directly.
//!
//! # Conventions
//!
//! - Factor IDs and covariance axes must match exactly in content and order.
//! - Risk outputs are reported in the units implied by the configured
//!   [`finstack_core::factor_model::RiskMeasure`].
//! - Strict unmatched-dependency handling should be used when factor coverage is
//!   treated as part of the model contract rather than a best-effort mapping.
//!
//! # References
//!
//! - Meucci, factor risk and covariance aggregation:
//!   `docs/REFERENCES.md#meucci-risk-and-asset-allocation`
//! - Parametric VaR conventions:
//!   `docs/REFERENCES.md#jpmorgan1996RiskMetrics`
//! - Coherent/tail-risk measures:
//!   `docs/REFERENCES.md#artzner1999CoherentRisk`

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
