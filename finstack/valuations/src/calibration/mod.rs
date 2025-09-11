//! Comprehensive calibration framework for term structures and surfaces.
//!
//! Provides market-standard calibration methodologies for:
//! - Interest rate curves (discount/forward)
//! - Credit curves (survival/hazard)
//! - Inflation curves
//! - Volatility surfaces
//! - Base correlation curves
//!
//! Supports both sequential bootstrapping and global optimization approaches.

use finstack_core::F;

// Submodules
pub mod bootstrap;
pub mod simple_calibration;

// Internal modules
mod config;
mod constraints;
mod macros;
mod multi_curve_mode;
mod quote;
mod report;
mod traits;

// Re-exports
pub use config::{CalibrationConfig, SolverKind};
pub use constraints::{CalibrationConstraint, ConstraintType, InequalityDirection};
pub use multi_curve_mode::{MultiCurveConfig, MultiCurveMode};
pub use quote::{
    CreditQuote, FutureSpecs, InflationQuote, MarketQuote, QuoteWithMetadata, RatesQuote,
    VolQuote,
};
pub use report::CalibrationReport;
pub use simple_calibration::SimpleCalibration;
pub use traits::Calibrator;

/// Finite penalty value used in objective functions instead of infinity.
/// Using a large finite value helps solvers behave more predictably and
/// documents intent while keeping diagnostics reasonable.
pub const PENALTY: F = 1e12;

#[inline]
pub fn penalize() -> F {
    PENALTY
}