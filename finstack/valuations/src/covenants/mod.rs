//! Covenant evaluation, testing, and breach forecasting.
//!
//! This module provides infrastructure for defining, testing, and forecasting
//! financial covenants commonly found in credit agreements, loan documents,
//! and structured product indentures.
//!
//! # Features
//!
//! - **Covenant Engine**: Rule-based evaluation of covenant compliance
//! - **Threshold Schedules**: Time-varying covenant levels
//! - **Breach Detection**: Identify current covenant violations
//! - **Forward Forecasting**: Project future breaches under scenarios
//! - **Consequence Modeling**: Trigger actions on breach (springing liens, etc.)
//!
//! # Covenant Types
//!
//! Common financial covenants supported:
//! - **Leverage Ratios**: Debt/EBITDA, Net Debt/EBITDA
//! - **Coverage Ratios**: Interest coverage, fixed charge coverage
//! - **Liquidity Tests**: Minimum cash, current ratio
//! - **Capital Covenants**: Maximum capex, minimum equity
//!
//! # Quick Example
//!
//! ```rust,no_run
//! use finstack_valuations::covenants::{
//!     CovenantEngine, CovenantSpec, CovenantType, ThresholdTest
//! };
//!
//! // Define a leverage covenant
//! let leverage_covenant = CovenantSpec {
//!     id: "MAX-LEVERAGE".to_string(),
//!     covenant_type: CovenantType::Leverage,
//!     test: ThresholdTest::LessThanOrEqual,
//!     threshold: 4.5,  // Max 4.5x Debt/EBITDA
//!     // ... other fields
//! };
//!
//! // Evaluate against current metrics
//! // let result = engine.test_covenant(&leverage_covenant, current_metrics);
//! ```
//!
//! # Breach Forecasting
//!
//! Project potential future breaches under different scenarios:
//!
//! ```rust,no_run
//! use finstack_valuations::covenants::{
//!     forecast_breaches_generic, CovenantForecastConfig
//! };
//!
//! // Configure forecasting
//! let config = CovenantForecastConfig::default();
//!
//! // Forecast breaches over forecast horizon
//! // let breaches = forecast_breaches_generic(&instrument, &covenants, &scenarios, config);
//! ```
//!
//! # See Also
//!
//! - [`CovenantEngine`] for covenant evaluation
//! - [`CovenantSpec`] for covenant definition
//! - [`ThresholdSchedule`] for time-varying thresholds
//! - [`forecast_breaches_generic`] for breach forecasting

pub mod engine;
pub mod forward;
/// Covenant report types and structures
pub mod mod_types;
/// Covenant threshold schedules and interpolation
pub mod schedule;

pub use engine::{
    ConsequenceApplication, Covenant, CovenantBreach, CovenantConsequence, CovenantEngine,
    CovenantScope, CovenantSpec, CovenantTestSpec, CovenantType, CovenantWindow, InstrumentMutator,
    SpringingCondition, ThresholdTest,
};
pub use forward::{
    forecast_breaches_generic, forecast_covenant_generic,
    CovenantForecast as GenericCovenantForecast, CovenantForecastConfig, FutureBreach, McConfig,
    ModelTimeSeries,
};
pub use mod_types::CovenantReport;
pub use schedule::{threshold_for_date, ThresholdSchedule};
