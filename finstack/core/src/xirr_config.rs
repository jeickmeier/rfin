//! XIRR (Extended Internal Rate of Return) configuration.
//!
//! This module provides configuration for the XIRR solver used for calculating
//! internal rates of return on irregular cashflows. Configuration can be provided
//! via the `FinstackConfig` extensions mechanism using the key [`XIRR_CONFIG_KEY_V1`].
//!
//! # Example
//!
//! ```rust
//! use finstack_core::config::FinstackConfig;
//! use finstack_core::xirr_config::{XirrConfig, XIRR_CONFIG_KEY_V1};
//! use serde_json::json;
//!
//! let mut cfg = FinstackConfig::default();
//! cfg.extensions.insert(XIRR_CONFIG_KEY_V1, json!({
//!     "tolerance": 1e-8,
//!     "max_iterations": 200,
//!     "default_guess": 0.05
//! }));
//!
//! let xirr_cfg = XirrConfig::from_finstack_config(&cfg)
//!     .expect("valid config");
//! assert_eq!(xirr_cfg.tolerance, 1e-8);
//! ```

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::config::FinstackConfig;

/// Extension key for XIRR configuration in `FinstackConfig.extensions`.
pub const XIRR_CONFIG_KEY_V1: &str = "core.xirr.v1";

/// Configuration for XIRR (Extended Internal Rate of Return) solver.
///
/// Controls the numerical parameters for solving the internal rate of return
/// equation for irregular cashflows.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct XirrConfig {
    /// Convergence tolerance for the solver (default: 1e-6).
    ///
    /// The solver stops when the NPV is within this tolerance of zero.
    #[cfg_attr(feature = "serde", serde(default = "default_xirr_tolerance"))]
    pub tolerance: f64,
    /// Maximum number of iterations (default: 100).
    #[cfg_attr(feature = "serde", serde(default = "default_xirr_max_iterations"))]
    pub max_iterations: usize,
    /// Default initial guess for the rate (default: 0.1 = 10%).
    ///
    /// The solver tries multiple starting points if the initial guess fails.
    #[cfg_attr(feature = "serde", serde(default = "default_xirr_guess"))]
    pub default_guess: f64,
}

fn default_xirr_tolerance() -> f64 {
    1e-6
}

fn default_xirr_max_iterations() -> usize {
    100
}

fn default_xirr_guess() -> f64 {
    0.1
}

impl Default for XirrConfig {
    fn default() -> Self {
        Self {
            tolerance: default_xirr_tolerance(),
            max_iterations: default_xirr_max_iterations(),
            default_guess: default_xirr_guess(),
        }
    }
}

/// Partial override struct for serde deserialization.
/// All fields are optional to allow partial overrides.
#[cfg(feature = "serde")]
#[derive(Clone, Debug, Deserialize)]
struct XirrConfigV1 {
    #[serde(default)]
    tolerance: Option<f64>,
    #[serde(default)]
    max_iterations: Option<usize>,
    #[serde(default)]
    default_guess: Option<f64>,
}

impl XirrConfig {
    /// Build XIRR config from a `FinstackConfig` extension section.
    ///
    /// If the extension section `core.xirr.v1` is present, its fields override
    /// the defaults; otherwise defaults are used.
    ///
    /// # Errors
    ///
    /// Returns an error if the extension section is present but malformed.
    ///
    /// # Example
    ///
    /// ```rust
    /// use finstack_core::config::FinstackConfig;
    /// use finstack_core::xirr_config::XirrConfig;
    ///
    /// let cfg = FinstackConfig::default();
    /// let xirr_cfg = XirrConfig::from_finstack_config(&cfg)
    ///     .expect("valid config");
    /// assert_eq!(xirr_cfg.tolerance, 1e-6); // default
    /// ```
    #[cfg(feature = "serde")]
    pub fn from_finstack_config(cfg: &FinstackConfig) -> crate::Result<Self> {
        let mut base = Self::default();

        if let Some(raw) = cfg.extensions.get(XIRR_CONFIG_KEY_V1) {
            let overrides: XirrConfigV1 =
                serde_json::from_value(raw.clone()).map_err(|e| crate::Error::Calibration {
                    message: format!("Failed to parse extension '{}': {}", XIRR_CONFIG_KEY_V1, e),
                    category: "config".to_string(),
                })?;

            if let Some(v) = overrides.tolerance {
                base.tolerance = v;
            }
            if let Some(v) = overrides.max_iterations {
                base.max_iterations = v;
            }
            if let Some(v) = overrides.default_guess {
                base.default_guess = v;
            }
        }

        Ok(base)
    }

    /// Build XIRR config from a `FinstackConfig` (non-serde fallback).
    ///
    /// When the `serde` feature is disabled, extensions are not available and
    /// this method always returns `XirrConfig::default()`.
    #[cfg(not(feature = "serde"))]
    pub fn from_finstack_config(_cfg: &FinstackConfig) -> crate::Result<Self> {
        Ok(Self::default())
    }
}

