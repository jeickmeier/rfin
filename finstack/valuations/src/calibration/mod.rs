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
//!
//! ## Multi-Curve Framework
//!
//! The calibration framework follows the post-2008 multi-curve methodology where
//! discount and forward curves are calibrated separately to capture basis spreads.
//!
//! ```ignore
//! use finstack_core::config::FinstackConfig;
//! use finstack_valuations::calibration::methods::{
//!     DiscountCurveCalibrator, ForwardCurveCalibrator
//! };
//!
//! let mut cfg = FinstackConfig::default();
//! cfg.extensions.insert(
//!     "valuations.calibration.v1",
//!     serde_json::json!({
//!         "multi_curve": { "calibrate_basis": true, "enforce_separation": true }
//!     })
//! );
//!
//! // Step 1: Calibrate OIS discount curve using deposits and OIS swaps
//! let ois_quotes = vec![/* deposits and OIS swaps */];
//! let disc_calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
//!     .with_finstack_config(&cfg)?;
//! let (discount_curve, _) = disc_calibrator.calibrate(&ois_quotes, &context)?;
//!
//! // Step 2: Add discount curve to context
//! let context = context.insert_discount(discount_curve);
//!
//! // Step 3: Calibrate forward curves using FRAs, futures, and tenor-specific swaps
//! let libor_quotes = vec![/* FRAs, futures, and LIBOR swaps */];
//! let fwd_calibrator = ForwardCurveCalibrator::new(
//!     "USD-SOFR-3M-FWD", 0.25, base_date, Currency::USD, "USD-OIS"
//! );
//! let (forward_curve, _) = fwd_calibrator.calibrate(&libor_quotes, &context)?;
//! ```
//!
//! ### Multi-Curve Calibration Notes
//!
//! 1. **Instrument Selection**:
//!    - For discount curve: Use deposits and OIS swaps (instruments that don't require forward curves)
//!    - For forward curves: Use FRAs, futures, and tenor-specific swaps
//!
//! 2. **Calibration Order**:
//!    - Always calibrate discount curve first
//!    - Then calibrate forward curves with discount curve in context

// Submodules
pub mod bumps;
/// Version 2 of the calibration API with plan-driven execution.
pub mod v2;
mod config;
pub mod methods;
pub mod pricing;
pub mod quotes;
mod report;
mod solver;
pub mod spec;
mod traits;
mod validation;

// Re-exports: Configuration
pub use config::{
    CalibrationConfig, CalibrationMethod, MultiCurveConfig, RateBounds, RateBoundsPolicy,
    SolverKind, ValidationMode, CALIBRATION_CONFIG_KEY_V1,
};

// Re-exports: SABR derivatives (from instruments module)
pub use crate::instruments::common::models::volatility::sabr_derivatives::{
    SABRCalibrationDerivatives, SABRMarketData,
};

// Re-exports: Quote schemas (from quotes module)
pub use quotes::{
    CreditQuote, FutureSpecs, InflationQuote, InstrumentConventions, MarketQuote, RatesQuote,
    VolQuote,
};

// Re-exports: Pricing infrastructure
pub use pricing::{CalibrationPricer, ConvexityParameters, RatesQuoteUseCase};

// Re-exports: Reports and specs
pub use report::CalibrationReport;
pub(crate) use solver::bracket_solve_1d_with_diagnostics;
pub use solver::{create_simple_solver, solve_1d, BracketDiagnostics, PENALTY, SolverConfig};
pub use spec::{
    CalibrationEnvelope, CalibrationResult, CalibrationResultEnvelope, CalibrationSpec,
    CalibrationStep, CALIBRATION_SCHEMA_V1,
};

// Re-exports: Traits and validation
pub use traits::Calibrator;
pub use validation::{CurveValidator, SurfaceValidator, ValidationConfig};