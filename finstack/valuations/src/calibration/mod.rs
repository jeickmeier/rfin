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
//! - **Plan-Driven API**: Uses `"finstack.calibration"` schema for structured calibration plans.
//! - **Flexible Solvers**: Supports both sequential bootstrapping and global optimization (Newton/LM).
//! - **Market Standards**: Implements post-2008 multi-curve frameworks and strict pricing conventions.
//! - **Extensible Architecture**: Easy to add new instrument types and calibration targets.
//!
//! # Quick Example
//!
//! ```rust
//! use finstack_valuations::calibration::api::engine;
//! use finstack_valuations::calibration::api::schema::{
//!     CalibrationEnvelope, CalibrationPlan, CalibrationStep, StepParams,
//!     DiscountCurveParams, CALIBRATION_SCHEMA,
//! };
//! use finstack_valuations::calibration::CalibrationMethod;
//! use finstack_valuations::market::quotes::rates::RateQuote;
//! use finstack_valuations::market::quotes::market_quote::MarketQuote;
//! use finstack_core::HashMap;
//!
//! # fn example() -> finstack_core::Result<()> {
//! let quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::default();
//! let steps: Vec<CalibrationStep> = Vec::new();
//!
//! let plan = CalibrationPlan {
//!     id: "plan".to_string(),
//!     description: None,
//!     quote_sets,
//!     steps,
//!     settings: Default::default(),
//! };
//! let envelope = CalibrationEnvelope {
//!     schema: CALIBRATION_SCHEMA.to_string(),
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
//! - [`crate::market::quotes`] for market data representation.

/// Plan-driven calibration API (schema + execution engine).
pub mod api;
/// Prepared quotes for calibration.
pub(crate) mod prepared;
/// Solver utilities and implementations used by calibration.
pub(crate) mod solver;
/// Calibration targets mapping API steps to domain execution.
pub(crate) mod targets;

// Shared infrastructure
mod config;
mod report;
pub(crate) mod step_runtime;
pub(crate) mod validation;

/// Curve bumping helpers used by scenarios and risk metrics (re-calibration).
#[doc(hidden)]
pub mod bumps;

/// Shared constants (tolerances, magic numbers).
pub(crate) mod constants;

/// Convexity adjustment logic.
// Re-exports: Configuration (kept public but not part of the supported surface)
#[doc(hidden)]
pub use config::{
    CalibrationConfig, CalibrationMethod, DiscountCurveSolveConfig, HazardCurveSolveConfig,
    InflationCurveSolveConfig, RatesStepConventions, ResidualWeightingScheme,
    CALIBRATION_CONFIG_KEY,
};
#[doc(hidden)]
pub use solver::SolverConfig;
#[doc(hidden)]
pub use validation::curves::CurveValidator;
#[doc(hidden)]
pub use validation::surfaces::SurfaceValidator;
#[doc(hidden)]
pub use validation::{RateBounds, RateBoundsPolicy, ValidationConfig, ValidationMode};

// Re-exports: Reports (internal)
#[doc(hidden)]
pub use report::CalibrationReport;
