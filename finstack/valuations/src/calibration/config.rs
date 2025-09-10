//! Calibration configuration and solver selection.

use finstack_core::market_data::term_structures::hazard_curve::Seniority;
use finstack_core::F;
use std::collections::HashMap;

/// Solver type selection for calibration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SolverKind {
    /// Newton-Raphson solver with automatic derivative estimation
    Newton,
    /// Brent's method solver (robust, bracketing required)
    Brent,
    /// Hybrid solver that tries Newton first, falls back to Brent
    Hybrid,
}

impl Default for SolverKind {
    fn default() -> Self {
        Self::Hybrid
    }
}

/// Configuration for calibration processes.
#[derive(Clone, Debug)]
pub struct CalibrationConfig {
    /// Solver tolerance
    pub tolerance: F,
    /// Maximum iterations
    pub max_iterations: usize,
    /// Use parallel processing when available
    pub use_parallel: bool,
    /// Random seed for reproducible results
    pub random_seed: Option<u64>,
    /// Enable verbose logging
    pub verbose: bool,
    /// Solver type selection
    pub solver_kind: SolverKind,
    /// Entity-specific seniority mappings for credit calibration
    pub entity_seniority: HashMap<String, Seniority>,
}

impl Default for CalibrationConfig {
    fn default() -> Self {
        Self {
            tolerance: 1e-10,
            max_iterations: 100,
            use_parallel: false, // Deterministic by default
            random_seed: Some(42),
            verbose: false,
            solver_kind: SolverKind::default(),
            entity_seniority: HashMap::new(),
        }
    }
}
