//! Calibration framework (plan-driven API).
//!
//! Provides market-standard calibration methodologies for:
//! - Interest rate curves (discount/forward)
//! - Credit curves (survival/hazard)
//! - Inflation curves
//! - Volatility surfaces
//! - Base correlation curves
//!
//! # Features
//! - **Plan-Driven API**: Uses schema version `"finstack.calibration/2"` for structured calibration plans.
//! - **Flexible Solvers**: Supports both sequential bootstrapping and global optimization (Newton/LM).
//! - **Market Standards**: Implements post-2008 multi-curve frameworks and strict pricing conventions.
//! - **Extensible Architecture**: Easy to add new instrument types and calibration targets.
//!
//! # Quick Example
//!
//! ```rust
//! use finstack_valuations::calibration::api::engine;
//! use finstack_valuations::calibration::api::schema::{
//!     CalibrationEnvelopeV2, CalibrationPlanV2, CalibrationStepV2, StepParams,
//!     DiscountCurveParams, CalibrationMethod, CALIBRATION_SCHEMA_V2,
//! };
//! use finstack_valuations::calibration::quotes::{MarketQuote, RatesQuote};
//! use std::collections::HashMap;
//!
//! # fn example() -> finstack_core::Result<()> {
//! let quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::new();
//! let steps: Vec<CalibrationStepV2> = Vec::new();
//!
//! let plan = CalibrationPlanV2 {
//!     id: "plan".to_string(),
//!     description: None,
//!     quote_sets,
//!     steps,
//!     settings: Default::default(),
//! };
//! let envelope = CalibrationEnvelopeV2 {
//!     schema: CALIBRATION_SCHEMA_V2.to_string(),
//!     plan,
//!     initial_market: None,
//! };
//!
//! // Execute the calibration plan
//! let result = engine::execute(&envelope)?;
//! # Ok(())
//! # }
//! ```
//!
//! # See Also
//! - [`api`] for the plan schema and engine.
//! - [`solver`] for the underlying numerical solvers.
//! - [`quotes`] for market data representation.

/// Adapters mapping API steps to domain execution.
pub mod adapters;
/// Plan-driven calibration API (schema + execution engine).
pub mod api;
/// Pricing infrastructure used by the plan-driven engine.
pub mod pricing;
/// Market quote types and extraction used by the plan-driven engine.
pub mod quotes;
/// Solver utilities and implementations used by calibration.
pub mod solver;

// Shared infrastructure
mod config;
mod report;
mod validation;

/// Curve bumping helpers used by scenarios and risk metrics (v2 re-calibration).
pub mod bumps;

/// Shared constants (tolerances, magic numbers).
pub mod constants;

// Re-exports: Configuration
pub use config::{
    CalibrationConfig, CalibrationMethod as CalibrationSolveMethod, MultiCurveConfig,
    CALIBRATION_CONFIG_KEY_V2,
};

// Re-exports: SABR derivatives (from instruments module)
pub use crate::instruments::common::models::volatility::sabr_derivatives::{
    SABRCalibrationDerivatives, SABRMarketData,
};

// Re-exports: Reports
pub use report::CalibrationReport;
pub use solver::{
    create_simple_solver, solve_1d, BracketDiagnostics, SolverConfig, OBJECTIVE_VALID_ABS_MAX,
    PENALTY, RESIDUAL_PENALTY_ABS_MIN,
};

// Re-exports: Validation
pub use validation::{
    CurveValidator, RateBounds, RateBoundsPolicy, SurfaceValidator, ValidationConfig,
    ValidationMode,
};
