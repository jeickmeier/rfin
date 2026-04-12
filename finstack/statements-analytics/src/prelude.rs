//! Commonly used analytics types.
//!
//! Import this module to get quick access to the most common analytics types:
//!
//! ```rust,ignore
//! use finstack_statements_analytics::prelude::*;
//! ```

pub use crate::analysis::{
    BridgeChart, BridgeStep, CheckReportRenderer, CorporateAnalysis, CorporateAnalysisBuilder,
    CreditInstrumentAnalysis, CreditMapping, FormulaCheck, MonteCarloConfig, MonteCarloResults,
    ScenarioDefinition, ScenarioDiff, ScenarioResults, ScenarioSet, ThreeStatementMapping,
    TrendDirection, VarianceAnalyzer, VarianceConfig, VarianceReport, VarianceRow,
};
pub use crate::analysis::{
    credit_underwriting_checks, lbo_model_checks, three_statement_checks,
};
pub use crate::extensions::{CorkscrewExtension, CreditScorecardExtension};
pub use crate::templates::{RealEstateExtension, TemplatesExtension, VintageExtension};
