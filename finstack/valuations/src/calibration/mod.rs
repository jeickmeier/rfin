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
mod config;
pub mod derivatives;
pub mod methods;
mod quote;
mod report;
mod solver_helpers;
mod solver_config;
pub mod spec;
mod traits;
mod validation;

// Re-exports
pub use config::{
    CalibrationConfig, CalibrationMethod, MultiCurveConfig, RateBounds, RateBoundsPolicy,
    SolverKind, ValidationMode, CALIBRATION_CONFIG_KEY_V1,
};
pub use derivatives::sabr_derivatives::{SABRCalibrationDerivatives, SABRMarketData};
pub use derivatives::sabr_model_params::SABRModelParams;
pub use quote::{
    CreditQuote, FutureSpecs, InflationQuote, InstrumentConventions, MarketQuote, RatesQuote,
    VolQuote,
};
pub use report::CalibrationReport;
pub(crate) use solver_helpers::bracket_solve_1d_with_diagnostics;
pub use solver_helpers::{create_simple_solver, solve_1d, BracketDiagnostics, PENALTY};
pub use solver_config::SolverConfig;
pub use spec::{
    CalibrationEnvelope, CalibrationResult, CalibrationResultEnvelope, CalibrationSpec,
    CalibrationStep, CALIBRATION_SCHEMA_V1,
};
pub use traits::Calibrator;
pub use validation::{CurveValidator, SurfaceValidator, ValidationConfig};

// Re-export calibration validation types
pub use methods::RatesQuoteUseCase;