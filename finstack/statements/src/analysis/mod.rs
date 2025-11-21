//! Sensitivity analysis and corporate valuation for financial statement models.
//!
//! This module provides tools for running parameter sweeps, analyzing
//! how changes in assumptions affect model outputs, and performing DCF valuations.

pub mod corporate;
pub mod goal_seek;
pub mod sensitivity;
pub mod types;

pub use corporate::{evaluate_dcf, CorporateValuationResult};
pub use goal_seek::goal_seek;
pub use sensitivity::SensitivityAnalyzer;
pub use types::{ParameterSpec, SensitivityConfig, SensitivityMode, SensitivityResult};
