//! Multi-curve framework configuration and mode selection.
//!
//! This module provides configuration for multi-curve vs single-curve calibration
//! to support both post-2008 methodology and legacy compatibility.

use serde::{Deserialize, Serialize};

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
