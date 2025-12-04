//! Analysis tools for financial statement models.
//!
//! This module provides tools for:
//! - **Sensitivity analysis** - Parameter sweeps and tornado charts
//! - **Goal seeking** - Find input values that achieve target outputs
//! - **Corporate valuation** - DCF and enterprise value calculations
//! - **Reports** - Formatted output for P&L summaries and credit assessment
//! - **Dependency tracing** - Identify direct and transitive dependencies
//! - **Formula explanation** - Break down calculations step-by-step
//! - **Forecast backtesting** - Evaluate forecast accuracy
//! - **Covenant analysis** - Detect covenant breaches
//! - **Scenario management** - Named scenario sets with diff/comparison helpers

pub mod backtesting;
pub mod corporate;
pub mod covenants;
pub mod dependency_trace;
pub mod formula_explain;
pub mod goal_seek;
pub mod reports;
pub mod sensitivity;
pub mod scenario_set;
pub mod types;
pub mod visualization;
pub mod variance;

pub use backtesting::{backtest_forecast, ForecastMetrics};
pub use corporate::{evaluate_dcf, CorporateValuationResult};
pub use covenants::forecast_breaches;
pub use dependency_trace::{DependencyTracer, DependencyTree};
pub use formula_explain::{Explanation, ExplanationStep, FormulaExplainer};
pub use goal_seek::goal_seek;
pub use reports::{Alignment, CreditAssessmentReport, PLSummaryReport, Report, TableBuilder};
pub use sensitivity::SensitivityAnalyzer;
pub use scenario_set::{ScenarioDefinition, ScenarioDiff, ScenarioResults, ScenarioSet};
pub use types::{ParameterSpec, SensitivityConfig, SensitivityMode, SensitivityResult};
pub use visualization::{render_tree_ascii, render_tree_detailed};
pub use variance::{
    BridgeChart, BridgeStep, VarianceAnalyzer, VarianceConfig, VarianceReport, VarianceRow,
};
