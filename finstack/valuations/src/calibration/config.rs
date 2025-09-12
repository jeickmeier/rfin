//! Calibration configuration and solver selection.
//!
//! This module provides unified configuration for calibration processes including:
//! - Solver selection and numerical parameters
//! - Multi-curve framework mode (post-2008 vs legacy single-curve)
//! - Entity seniority mappings for credit calibration

use finstack_core::market_data::term_structures::hazard_curve::Seniority;
use finstack_core::F;
use serde::{Deserialize, Serialize};
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

/// Multi-curve framework mode
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum MultiCurveMode {
    /// Post-2008 multi-curve framework (default):
    /// - Discount curves (OIS) for present value only
    /// - Forward curves calibrated independently
    /// - Basis swaps capture tenor spreads
    MultiCurve,
    
    /// Pre-2008 single-curve framework (legacy compatibility):
    /// - Discount curve = forward curve
    /// - No tenor basis spreads
    /// - For special cases or simplified modeling
    SingleCurve,
}

impl Default for MultiCurveMode {
    fn default() -> Self {
        Self::MultiCurve
    }
}

/// Multi-curve calibration configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MultiCurveConfig {
    /// Framework mode (multi-curve vs single-curve)
    pub mode: MultiCurveMode,
    
    /// Whether to calibrate basis spreads (only in MultiCurve mode)
    pub calibrate_basis: bool,
    
    /// Default tenor for single-curve mode (in years, e.g., 0.25 for 3M)
    pub single_curve_tenor: f64,
    
    /// Whether to enforce strict separation (fail if trying to derive forward from discount)
    pub enforce_separation: bool,
}

impl Default for MultiCurveConfig {
    fn default() -> Self {
        Self {
            mode: MultiCurveMode::MultiCurve,
            calibrate_basis: true,
            single_curve_tenor: 0.25, // 3M default
            enforce_separation: true,
        }
    }
}

impl MultiCurveConfig {
    /// Create a multi-curve configuration (post-2008 standard)
    pub fn multi_curve() -> Self {
        Self::default()
    }
    
    /// Create a single-curve configuration (pre-2008 legacy)
    pub fn single_curve(tenor_years: f64) -> Self {
        Self {
            mode: MultiCurveMode::SingleCurve,
            calibrate_basis: false,
            single_curve_tenor: tenor_years,
            enforce_separation: false,
        }
    }
    
    /// Check if we're in multi-curve mode
    pub fn is_multi_curve(&self) -> bool {
        matches!(self.mode, MultiCurveMode::MultiCurve)
    }
    
    /// Check if we should derive forward from discount (only in single-curve mode)
    pub fn derive_forward_from_discount(&self) -> bool {
        matches!(self.mode, MultiCurveMode::SingleCurve)
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
    /// Multi-curve framework configuration
    pub multi_curve: MultiCurveConfig,
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
            multi_curve: MultiCurveConfig::default(),
        }
    }
}

impl CalibrationConfig {
    /// Set multi-curve framework configuration.
    /// This is a convenience method for backward compatibility.
    pub fn with_multi_curve_config(mut self, multi_curve_config: MultiCurveConfig) -> Self {
        self.multi_curve = multi_curve_config;
        self
    }
    
    /// Set multi-curve mode directly.
    pub fn with_multi_curve_mode(mut self, mode: MultiCurveMode) -> Self {
        self.multi_curve.mode = mode;
        self
    }
    
    /// Create a configuration for single-curve mode (legacy).
    pub fn single_curve(tenor_years: f64) -> Self {
        Self {
            multi_curve: MultiCurveConfig::single_curve(tenor_years),
            ..Self::default()
        }
    }
    
    /// Create a configuration for multi-curve mode (standard).
    pub fn multi_curve() -> Self {
        Self::default() // Already defaults to multi-curve
    }
}
