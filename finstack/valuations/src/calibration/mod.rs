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
mod quote;
mod report;
mod traits;
mod validation;

// Re-exports
pub use config::{CalibrationConfig, MultiCurveConfig, MultiCurveMode, SolverKind};
pub use constraints::{CalibrationConstraint, ConstraintType, InequalityDirection};
pub use quote::{
    CreditQuote, FutureSpecs, InflationQuote, MarketQuote, QuoteWithMetadata, RatesQuote,
    VolQuote,
};
pub use report::CalibrationReport;
pub use simple_calibration::SimpleCalibration;
pub use traits::Calibrator;
pub use validation::{CurveValidator, MarketValidator, SurfaceValidator, ValidationConfig, ValidationError};

/// Finite penalty value used in objective functions instead of infinity.
/// Using a large finite value helps solvers behave more predictably and
/// documents intent while keeping diagnostics reasonable.
pub const PENALTY: F = 1e12;

#[inline]
pub fn penalize() -> F {
    PENALTY
}