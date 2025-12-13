//! Solver configuration for root-finding algorithms.
//!
//! This module provides configuration for numerical solvers used throughout finstack,
//! including Newton-Raphson and Brent's method. Configuration can be provided via
//! the `FinstackConfig` extensions mechanism using the key [`SOLVER_CONFIG_KEY_V1`].
//!
//! # Example
//!
//! ```rust
//! use finstack_core::config::FinstackConfig;
//! use finstack_core::solver_config::{SolverConfig, SOLVER_CONFIG_KEY_V1};
//! use serde_json::json;
//!
//! let mut cfg = FinstackConfig::default();
//! cfg.extensions.insert(SOLVER_CONFIG_KEY_V1, json!({
//!     "newton": { "tolerance": 1e-10, "max_iterations": 100 },
//!     "brent": { "tolerance": 1e-10 }
//! }));
//!
//! let solver_cfg = SolverConfig::from_finstack_config(&cfg)
//!     .expect("valid config");
//! assert_eq!(solver_cfg.newton.tolerance, 1e-10);
//! ```

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::config::FinstackConfig;

/// Extension key for solver configuration in `FinstackConfig.extensions`.
pub const SOLVER_CONFIG_KEY_V1: &str = "core.solver.v1";

/// Configuration for Newton-Raphson solver.
///
/// Controls convergence criteria and numerical parameters for the Newton-Raphson
/// root-finding algorithm used in implied volatility, yield-to-maturity, and
/// other financial calculations.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NewtonSolverConfig {
    /// Convergence tolerance for function value (default: 1e-12).
    #[cfg_attr(feature = "serde", serde(default = "default_newton_tolerance"))]
    pub tolerance: f64,
    /// Maximum number of iterations (default: 50).
    #[cfg_attr(feature = "serde", serde(default = "default_newton_max_iterations"))]
    pub max_iterations: usize,
    /// Base finite difference step for derivative estimation (default: 1e-8).
    #[cfg_attr(feature = "serde", serde(default = "default_newton_fd_step"))]
    pub fd_step: f64,
    /// Minimum derivative threshold (default: 1e-14).
    #[cfg_attr(feature = "serde", serde(default = "default_newton_min_derivative"))]
    pub min_derivative: f64,
    /// Relative minimum derivative threshold (default: 1e-6).
    #[cfg_attr(feature = "serde", serde(default = "default_newton_min_derivative_rel"))]
    pub min_derivative_rel: f64,
}

fn default_newton_tolerance() -> f64 {
    1e-12
}

fn default_newton_max_iterations() -> usize {
    50
}

fn default_newton_fd_step() -> f64 {
    1e-8
}

fn default_newton_min_derivative() -> f64 {
    1e-14
}

fn default_newton_min_derivative_rel() -> f64 {
    1e-6
}

impl Default for NewtonSolverConfig {
    fn default() -> Self {
        Self {
            tolerance: default_newton_tolerance(),
            max_iterations: default_newton_max_iterations(),
            fd_step: default_newton_fd_step(),
            min_derivative: default_newton_min_derivative(),
            min_derivative_rel: default_newton_min_derivative_rel(),
        }
    }
}

/// Configuration for Brent's method solver.
///
/// Controls convergence criteria for Brent's bracketing method, which is
/// used when robust convergence is required and a bracketing interval is available.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BrentSolverConfig {
    /// Convergence tolerance (default: 1e-12).
    #[cfg_attr(feature = "serde", serde(default = "default_brent_tolerance"))]
    pub tolerance: f64,
    /// Maximum number of iterations (default: 100).
    #[cfg_attr(feature = "serde", serde(default = "default_brent_max_iterations"))]
    pub max_iterations: usize,
    /// Bracket expansion factor when searching for sign change (default: 2.0).
    #[cfg_attr(feature = "serde", serde(default = "default_brent_bracket_expansion"))]
    pub bracket_expansion: f64,
}

fn default_brent_tolerance() -> f64 {
    1e-12
}

fn default_brent_max_iterations() -> usize {
    100
}

fn default_brent_bracket_expansion() -> f64 {
    2.0
}

impl Default for BrentSolverConfig {
    fn default() -> Self {
        Self {
            tolerance: default_brent_tolerance(),
            max_iterations: default_brent_max_iterations(),
            bracket_expansion: default_brent_bracket_expansion(),
        }
    }
}

/// Combined solver configuration.
///
/// Provides configuration for all supported numerical solvers. This can be
/// stored in `FinstackConfig.extensions` under the key [`SOLVER_CONFIG_KEY_V1`].
#[derive(Clone, Copy, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SolverConfig {
    /// Newton-Raphson solver configuration.
    #[cfg_attr(feature = "serde", serde(default))]
    pub newton: NewtonSolverConfig,
    /// Brent's method solver configuration.
    #[cfg_attr(feature = "serde", serde(default))]
    pub brent: BrentSolverConfig,
}

/// Partial override struct for serde deserialization.
/// All fields are optional to allow partial overrides.
#[cfg(feature = "serde")]
#[derive(Clone, Debug, Deserialize)]
struct SolverConfigV1 {
    #[serde(default)]
    newton: Option<NewtonSolverConfig>,
    #[serde(default)]
    brent: Option<BrentSolverConfig>,
}

impl SolverConfig {
    /// Build solver config from a `FinstackConfig` extension section.
    ///
    /// If the extension section `core.solver.v1` is present, its fields override
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
    /// use finstack_core::solver_config::SolverConfig;
    ///
    /// let cfg = FinstackConfig::default();
    /// let solver_cfg = SolverConfig::from_finstack_config(&cfg)
    ///     .expect("valid config");
    /// assert_eq!(solver_cfg.newton.tolerance, 1e-12); // default
    /// ```
    #[cfg(feature = "serde")]
    pub fn from_finstack_config(cfg: &FinstackConfig) -> crate::Result<Self> {
        let mut base = Self::default();

        if let Some(raw) = cfg.extensions.get(SOLVER_CONFIG_KEY_V1) {
            let overrides: SolverConfigV1 =
                serde_json::from_value(raw.clone()).map_err(|e| crate::Error::Calibration {
                    message: format!(
                        "Failed to parse extension '{}': {}",
                        SOLVER_CONFIG_KEY_V1, e
                    ),
                    category: "config".to_string(),
                })?;

            if let Some(newton) = overrides.newton {
                base.newton = newton;
            }
            if let Some(brent) = overrides.brent {
                base.brent = brent;
            }
        }

        Ok(base)
    }

    /// Build solver config from a `FinstackConfig` (non-serde fallback).
    ///
    /// When the `serde` feature is disabled, extensions are not available and
    /// this method always returns `SolverConfig::default()`.
    #[cfg(not(feature = "serde"))]
    pub fn from_finstack_config(_cfg: &FinstackConfig) -> crate::Result<Self> {
        Ok(Self::default())
    }
}

