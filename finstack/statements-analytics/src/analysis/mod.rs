//! Analysis tools for financial statement models.
//!
//! This module provides tools for:
//!
//! - **[`crate::analysis::valuation`]** — DCF corporate valuation and the orchestrated analysis pipeline
//! - **[`crate::analysis::credit`]** — Covenant forecasting and credit coverage metrics
//! - **[`crate::analysis::scenarios`]** — Scenario sets, sensitivity sweeps, variance, and Monte Carlo
//! - **[`crate::analysis::introspection`]** — Dependency tracing and formula explanation
//! - **[`crate::analysis::reports`]** — Formatted P&L summaries and credit assessment reports
//! - **[`mod@crate::analysis::goal_seek`]** — Root-finding for target metric values
//! - **[`crate::analysis::backtesting`]** — Forecast accuracy metrics
//! - **[`crate::analysis::ecl`]** — Expected credit loss (IFRS 9 staging, CECL, portfolio aggregation) *(feature: `ecl`)*
//!
//! ## Where To Start
//!
//! - Use [`crate::analysis::CorporateAnalysisBuilder`] when you want one orchestrated pipeline
//!   that evaluates statements and optionally adds equity plus credit analysis.
//! - Use [`crate::analysis::evaluate_dcf_with_market`] for direct DCF valuation from a statement
//!   model.
//! - Use [`crate::analysis::ScenarioSet`] and [`crate::analysis::VarianceAnalyzer`] when comparing multiple
//!   operating cases.
//! - Use [`crate::analysis::forecast_breaches`] and [`crate::analysis::compute_credit_context`] for lender-style
//!   compliance and coverage analysis.
//!
//! ## Conventions
//!
//! - Ratios such as DSCR, coverage, leverage, and valuation multiples are
//!   returned as plain scalars, so `2.0` means `2.0x`.
//! - Percentage-style inputs, such as WACC or growth assumptions, follow the
//!   crate-wide decimal convention: `0.10` means `10%`.
//! - Scenario overrides are deterministic full-period scalar overrides unless a
//!   lower-level API states otherwise.

// ---- Grouped submodules ----

/// Corporate valuation and orchestration.
pub mod valuation;

/// Credit analysis (covenants, coverage ratios).
pub mod credit;

/// Scenario, sensitivity, variance, and Monte Carlo analysis.
pub mod scenarios;

/// Domain-level validation checks (reconciliation, consistency, credit).
pub mod checks;

/// Expected credit loss: IFRS 9 staging, ECL calculation, CECL, portfolio aggregation.
pub mod ecl;

// ---- Flat submodules ----

pub mod backtesting;
pub mod goal_seek;
pub mod introspection;
pub mod reports;

// ---- Flattened module re-exports ----
// These expose `analysis::corporate::*`, `analysis::covenants::*`, etc.
// as the canonical public paths.

pub use valuation::corporate;
pub use valuation::orchestrator;

pub use credit::covenants;
pub use credit::credit_context;

pub use scenarios::monte_carlo;
pub use scenarios::scenario_set;
pub use scenarios::sensitivity;
pub use scenarios::types;
pub use scenarios::variance;

// ---- Type-level re-exports (unchanged public API) ----

pub use backtesting::{backtest_forecast, ForecastMetrics};
pub use corporate::{evaluate_dcf_with_market, CorporateValuationResult, DcfOptions};
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
pub use sensitivity::{generate_tornado_entries, SensitivityAnalyzer};
pub use types::{
    ParameterSpec, SensitivityConfig, SensitivityMode, SensitivityResult, TornadoEntry,
};
pub use variance::{
    BridgeChart, BridgeStep, VarianceAnalyzer, VarianceConfig, VarianceReport, VarianceRow,
};

// ---- Check-framework re-exports ----

pub use checks::{
    corkscrew_as_checks, credit_underwriting_checks, lbo_model_checks, three_statement_checks,
    CheckReportRenderer, CreditMapping, FormulaCheck, ThreeStatementMapping, TrendDirection,
};

// ---- ECL re-exports ----

pub use ecl::{
    classify_stage, compute_ecl_single, compute_ecl_weighted, compute_waterfall, CeclConfig,
    CeclEngine, CeclMethodology, CeclResult, EclBucket, EclConfig, EclConfigBuilder, EclEngine,
    EclResult, Exposure, ExposureEclResult, LgdType, MacroScenario, PdTermStructure,
    PortfolioEclResult, ProvisionWaterfall, QualitativeFlags, RawPdCurve, ReversionMethod, Stage,
    StageResult, StagingConfig, StagingTrigger, WeightedEclResult,
};
