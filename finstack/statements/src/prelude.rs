//! Commonly used types and traits.
//!
//! Import this module to get quick access to the most common types:
//!
//! ```rust
//! use finstack_statements::prelude::*;
//! ```
//!
//! This prelude is intentionally broad: it re-exports the most common
//! `finstack-statements` types plus the full `finstack_core::prelude::*`.
//! Prefer importing from the source module directly when you want a narrower API
//! boundary in libraries or examples aimed at teaching a specific subsystem.

pub use crate::builder::{MixedNodeBuilder, ModelBuilder};
pub use crate::checks::builtins::{
    BalanceSheetArticulation, CashReconciliation, MissingValueCheck, NonFiniteCheck,
    RetainedEarningsReconciliation, SignConventionCheck,
};
pub use crate::checks::{
    Check, CheckCategory, CheckConfig, CheckFinding, CheckReport, CheckResult, CheckSuite,
    CheckSuiteSpec, CheckSummary, Materiality, PeriodScope, Severity,
};
pub use crate::error::{Error, Result};
pub use crate::evaluator::{Evaluator, NumericMode, PreparedEvaluation, StatementResult};
pub use crate::registry::Registry;
pub use crate::types::{
    AmountOrScalar, FinancialModelSpec, ForecastMethod, ForecastSpec, NodeId, NodeSpec, NodeType,
    NodeValueType, SeasonalMode,
};

// Re-export the full core prelude for a unified foundation
pub use finstack_core::prelude::*;

// Additional date types used by statements but not in the core prelude
pub use finstack_core::dates::{build_periods, Period, PeriodId, PeriodKind};
