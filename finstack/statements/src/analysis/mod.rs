//! Sensitivity analysis for financial statement models.
//!
//! This module provides tools for running parameter sweeps and analyzing
//! how changes in assumptions affect model outputs.

pub mod sensitivity;
pub mod tornado;
pub mod types;

pub use sensitivity::SensitivityAnalyzer;
pub use tornado::{generate_tornado_chart, TornadoEntry};
pub use types::{ParameterSpec, SensitivityConfig, SensitivityMode, SensitivityResult};
