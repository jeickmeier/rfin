//! Solver utilities and implementations for calibration.
//!
//! This module provides the central interface for numerical optimization in the
//! calibration framework. It supports two primary methodologies:
//!
//! # Features
//! - **Sequential Bootstrapping**: Solves for curve knots one by one, ensuring high
//!   performance and exact fits for liquid market instruments.
//! - **Global Least-Squares**: Simultaneous optimization of all parameters,
//!   suitable for smoothing and multi-dimensional surface fitting.
//!
//! # Quick Example
//! ```rust
//! # use finstack_valuations::calibration::SolverConfig;
//! // Create a Brent solver config with default tolerances
//! let solver = SolverConfig::brent_default();
//! ```
//!
//! # See Also
//! - [`bootstrap`] for the sequential bootstrapping algorithm.
//! - [`global`] for the global optimization framework.
//! - [`traits`] for the target interfaces.

pub mod bootstrap;
mod config;
pub mod global;
mod helpers;
/// Solver traits for bootstrap and global optimization.
pub mod traits;

pub use config::SolverConfig;
pub(crate) use helpers::bracket_solve_1d_with_diagnostics;

pub use bootstrap::SequentialBootstrapper;
pub use global::GlobalFitOptimizer;
pub use traits::{BootstrapTarget, GlobalSolveTarget};
