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
pub mod credit_context;
pub mod goal_seek;
pub mod introspection;
pub mod monte_carlo;
pub mod orchestrator;
pub mod reports;
pub mod scenario_set;
pub mod sensitivity;
pub mod types;
pub mod variance;

pub use backtesting::{backtest_forecast, ForecastMetrics};
pub use corporate::{
    evaluate_dcf, evaluate_dcf_with_market, evaluate_dcf_with_options, CorporateValuationResult,
    DcfOptions,
};
pub use covenants::forecast_breaches;
pub use credit_context::{compute_credit_context, CreditContextMetrics};
pub use goal_seek::goal_seek;
pub use introspection::{
    render_tree_ascii, render_tree_detailed, DependencyTracer, DependencyTree, Explanation,
    ExplanationStep, FormulaExplainer,
};
pub use monte_carlo::{MonteCarloConfig, MonteCarloResults, PercentileSeries};
pub use orchestrator::{CorporateAnalysis, CorporateAnalysisBuilder, CreditInstrumentAnalysis};
pub use reports::{Alignment, CreditAssessmentReport, PLSummaryReport, Report, TableBuilder};
pub use scenario_set::{ScenarioDefinition, ScenarioDiff, ScenarioResults, ScenarioSet};
pub use sensitivity::SensitivityAnalyzer;
pub use types::{ParameterSpec, SensitivityConfig, SensitivityMode, SensitivityResult};
pub use variance::{
    BridgeChart, BridgeStep, VarianceAnalyzer, VarianceConfig, VarianceReport, VarianceRow,
};
