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
pub mod primitives;
pub mod simple_calibration;

// Internal modules
mod config;
mod macros;
mod quote;
mod report;
mod traits;

#[cfg(test)]
mod tests;

// Re-exports
pub use config::{CalibrationConfig, SolverKind};
pub use quote::MarketQuote;
pub use report::CalibrationReport;
pub use simple_calibration::SimpleCalibration;
pub use traits::Calibrator;