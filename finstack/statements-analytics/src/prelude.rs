//! Commonly used analytics types.
//!
//! Import this module to get quick access to the most common analytics types:
//!
//! ```rust,ignore
//! use finstack_statements_analytics::prelude::*;
//! ```

pub use crate::analysis::{
    BridgeChart, BridgeStep, CorporateAnalysis, CorporateAnalysisBuilder,
    CreditInstrumentAnalysis, MonteCarloConfig, MonteCarloResults,
    ScenarioDefinition, ScenarioDiff, ScenarioResults, ScenarioSet,
    VarianceAnalyzer, VarianceConfig, VarianceReport, VarianceRow,
};
pub use crate::extensions::{CorkscrewExtension, CreditScorecardExtension};
pub use crate::templates::{RealEstateExtension, TemplatesExtension, VintageExtension};
