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
mod utils;

// Re-exports
pub use config::{CalibrationConfig, SolverKind};
pub use constraints::{CalibrationConstraint, ConstraintType, InequalityDirection};
pub use quote::{
    CreditQuote, FutureSpecs, InflationQuote, MarketQuote, QuoteWithMetadata, RatesQuote,
    VolQuote,
};
pub use report::CalibrationReport;
pub use simple_calibration::SimpleCalibration;
pub use traits::Calibrator;
pub use utils::HashableFloat;