//! Domain-level validation checks for financial statement models.
//!
//! These checks go beyond the core accounting-identity and data-quality checks
//! in [`finstack_statements::checks::builtins`] and test cross-statement
//! reconciliation, internal consistency, and credit reasonableness.
//!
//! Higher-level conveniences:
//!
//! - [`crate::analysis::checks::formula_check::FormulaCheck`] — user-defined expression checks evaluated per period
//! - [`crate::analysis::checks::mappings`] — typed node-id mappings for common model patterns
//! - [`crate::analysis::checks::suites`] — pre-built check suites (three-statement, credit, LBO)
//! - [`crate::analysis::checks::corkscrew_adapter`] — convert corkscrew configs into structural checks
//! - [`crate::analysis::checks::renderer`] — render [`finstack_statements::checks::CheckReport`] as
//!   text or HTML

pub mod consistency;
pub mod corkscrew_adapter;
pub mod credit;
pub mod formula_check;
pub mod mappings;
pub mod reconciliation;
pub mod renderer;
pub mod suites;

// Re-export all check structs at the `checks` level.

pub use consistency::{EffectiveTaxRateCheck, GrowthRateConsistency, WorkingCapitalConsistency};
pub use credit::{
    CoverageFloorCheck, FcfSignCheck, LeverageRangeCheck, LiquidityRunwayCheck, TrendCheck,
    TrendDirection,
};
pub use reconciliation::{
    CapexReconciliation, DepreciationReconciliation, DividendReconciliation,
    InterestExpenseReconciliation,
};

pub use corkscrew_adapter::corkscrew_as_checks;
pub use formula_check::FormulaCheck;
pub use mappings::{CreditMapping, ThreeStatementMapping};
pub use renderer::CheckReportRenderer;
pub use suites::{credit_underwriting_checks, lbo_model_checks, three_statement_checks};

use finstack_core::dates::PeriodId;
use finstack_statements::evaluator::StatementResult;
use finstack_statements::types::NodeId;

/// Look up a single node's value for a given period.
fn get_node_value(results: &StatementResult, node: &NodeId, period: &PeriodId) -> Option<f64> {
    results
        .nodes
        .get(node.as_str())
        .and_then(|m| m.get(period).copied())
}

/// Sum several nodes' values for a given period, treating missing values as zero.
fn sum_nodes(results: &StatementResult, nodes: &[NodeId], period: &PeriodId) -> f64 {
    nodes
        .iter()
        .filter_map(|n| get_node_value(results, n, period))
        .sum()
}
