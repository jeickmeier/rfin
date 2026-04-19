//! Calibration framework (plan-driven API).
//!
//! Provides market-standard calibration methodologies for:
//! - Interest rate curves (discount/forward)
//! - Credit curves (survival/hazard)
//! - Inflation curves
//! - Volatility surfaces
//! - Base correlation curves
//!
//! # Documentation Rules For Calibration APIs
//!
//! Calibration docs should make three things explicit:
//!
//! - **Which quotes and conventions are assumed**: quote style, day count, curve
//!   time basis, interpolation, and market-standard construction choices should be
//!   stated near the public API that uses them.
//! - **Which tolerance is being discussed**: solver convergence tolerances and
//!   post-solve validation tolerances are distinct and should not be conflated.
//! - **Which canonical source applies**: model-heavy and convention-heavy APIs
//!   should include `# References` sections pointing to `docs/REFERENCES.md`.
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
//! - `api` for the plan schema and engine.
//! - `solver` for the underlying numerical solvers.
//! - [`crate::market::quotes`] for market data representation.
//!
//! # References
//!
//! - Multi-curve discounting and construction: `docs/REFERENCES.md#andersen-piterbarg-interest-rate-modeling`
//! - Curve interpolation: `docs/REFERENCES.md#hagan-west-monotone-convex`
//! - Core rates/derivatives background: `docs/REFERENCES.md#hull-options-futures`

/// Plan-driven calibration API (schema + execution engine).
pub mod api;
/// Hull-White one-factor model calibration to European swaptions.
pub mod hull_white;
/// LMM/BGM co-terminal swaption calibration.
#[cfg(feature = "mc")]
pub mod lmm;
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

/// Curve and surface bumping helpers for re-calibration.
///
/// Provides the supported surface for "what-if" risk analysis: applying a
/// `BumpRequest` (parallel or per-tenor) to a calibrated market object and
/// re-running the corresponding calibration step. Used by `finstack_scenarios`,
/// per-instrument risk metrics (CS01, key-rate duration, vega), and anything
/// else that needs to re-calibrate under perturbed quotes without redefining
/// the calibration plan from scratch.
///
/// Each asset class has its own entry point; see the `bumps` module docs for
/// the full table.
pub mod bumps;

/// Shared constants (tolerances, magic numbers).
pub(crate) mod constants;

// =============================================================================
// Public Re-exports
// =============================================================================
//
// These types form the supported public API for calibration configuration.
// They are used by wasm/py bindings and external consumers.

/// Configuration types for calibration.
pub use config::{
    CalibrationConfig, CalibrationMethod, DiscountCurveSolveConfig, HazardCurveSolveConfig,
    InflationCurveSolveConfig, RatesStepConventions, ResidualWeightingScheme,
};

/// Solver configuration (Brent/Newton).
pub use solver::SolverConfig;

/// Validation types for curves and surfaces.
pub use validation::curves::CurveValidator;
pub use validation::surfaces::SurfaceValidator;
pub use validation::{RateBounds, RateBoundsPolicy, ValidationConfig, ValidationMode};

/// Calibration diagnostics and results.
pub use report::{CalibrationDiagnostics, CalibrationReport, QuoteQuality};

// Internal/advanced re-exports (not part of typical usage)
#[doc(hidden)]
pub use config::CALIBRATION_CONFIG_KEY;
