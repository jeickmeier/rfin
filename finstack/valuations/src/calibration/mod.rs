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
//!     DiscountCurveParams, CalibrationMethod, CALIBRATION_SCHEMA,
//! };
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
pub mod prepared;
/// Solver utilities and implementations used by calibration.
pub mod solver;
/// Calibration targets mapping API steps to domain execution.
pub mod targets;

// Shared infrastructure
mod config;
mod report;
mod validation;

/// Curve bumping helpers used by scenarios and risk metrics (re-calibration).
pub mod bumps;

/// Shared constants (tolerances, magic numbers).
pub mod constants;

/// Convexity adjustment logic.
// Re-exports: Configuration
pub use config::{
    CalibrationConfig, CalibrationMethod as CalibrationSolveMethod, DiscountCurveSolveConfig,
    ResidualWeightingScheme, CALIBRATION_CONFIG_KEY,
};

// Re-exports: Reports
pub use report::CalibrationReport;
pub use solver::SolverConfig;

// Re-exports: Validation
pub use validation::{
    CurveValidator, RateBounds, RateBoundsPolicy, SurfaceValidator, ValidationConfig,
    ValidationMode,
};

// Bump helpers (stable façade)
pub use bumps::{
    hazard::bump_hazard_spreads, inflation::bump_inflation_rates, rates::bump_discount_curve,
    rates::bump_discount_curve_synthetic, BumpRequest,
};

pub use solver::{OBJECTIVE_VALID_ABS_MAX, PENALTY, RESIDUAL_PENALTY_ABS_MIN};
