//! Domain-level validation checks for financial statement models.
//!
//! These checks go beyond the core accounting-identity and data-quality checks
//! in [`finstack_statements::checks::builtins`] and test cross-statement
//! reconciliation, internal consistency, and credit reasonableness.

pub mod consistency;
pub mod credit;
pub mod reconciliation;

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
