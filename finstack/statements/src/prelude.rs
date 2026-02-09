//! Commonly used types and traits.
//!
//! Import this module to get quick access to the most common types:
//!
//! ```rust
//! use finstack_statements::prelude::*;
//! ```

pub use crate::analysis::{
    BridgeChart, BridgeStep, MonteCarloConfig, MonteCarloResults, ScenarioDefinition, ScenarioDiff,
    ScenarioResults, ScenarioSet, VarianceAnalyzer, VarianceConfig, VarianceReport, VarianceRow,
};
pub use crate::builder::{MixedNodeBuilder, ModelBuilder, NeedPeriods, Ready};
pub use crate::error::{Error, Result};
pub use crate::evaluator::{Evaluator, EvaluatorWithContext, NumericMode, Results};
pub use crate::extensions::{
    CorkscrewExtension, CreditScorecardExtension, Extension, ExtensionContext, ExtensionMetadata,
    ExtensionRegistry, ExtensionResult, ExtensionStatus,
};
pub use crate::registry::Registry;
pub use crate::templates::{TemplatesExtension, VintageExtension};
pub use crate::types::{
    AmountOrScalar, FinancialModelSpec, ForecastMethod, ForecastSpec, NodeSpec, NodeType,
    NodeValueType, SeasonalMode,
};

// Re-export commonly used types from finstack-core
pub use finstack_core::currency::Currency;
pub use finstack_core::dates::{build_periods, Period, PeriodId, PeriodKind};
pub use finstack_core::money::Money;
