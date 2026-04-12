//! Interest expense reconciliation check.

use serde::{Deserialize, Serialize};

use super::super::get_node_value;
use finstack_statements::checks::{
    Check, CheckCategory, CheckContext, CheckFinding, CheckResult, Materiality, Severity,
};
use finstack_statements::types::NodeId;
use finstack_statements::Result;

/// Reconciles interest expense against debt balances or the capital-structure
/// interest schedule.
///
/// If `cs_interest_node` is provided, compares the income-statement interest
/// expense to the capital-structure interest amount.  Otherwise, when
/// debt-balance / rate pairs are provided, verifies that the implied rate on
/// average debt is within a reasonable band.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterestExpenseReconciliation {
    /// Interest expense node (income statement).
    pub interest_expense_node: NodeId,
    /// Debt-balance / rate pairs: `(balance_node, optional_rate_node)`.
    pub debt_balance_nodes: Vec<(NodeId, Option<NodeId>)>,
    /// Optional capital-structure interest node for direct comparison.
    pub cs_interest_node: Option<NodeId>,
    /// Tolerance expressed as a fraction (default 0.05 = 5 %).
    pub tolerance_pct: Option<f64>,
}

impl Check for InterestExpenseReconciliation {
    fn id(&self) -> &str {
        "interest_expense_reconciliation"
    }

    fn name(&self) -> &str {
        "Interest Expense Reconciliation"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::CrossStatementReconciliation
    }

    fn execute(&self, context: &CheckContext) -> Result<CheckResult> {
        let tol = self.tolerance_pct.unwrap_or(0.05);
        let mut findings = Vec::new();
        let periods = &context.model.periods;

        for period_spec in periods {
            let pid = &period_spec.id;

            let Some(interest) = get_node_value(context.results, &self.interest_expense_node, pid)
            else {
                continue;
            };

            // Path 1: direct comparison with CS interest node.
            if let Some(ref cs_node) = self.cs_interest_node {
                let Some(cs_interest) = get_node_value(context.results, cs_node, pid) else {
                    continue;
                };

                let reference = cs_interest.abs().max(1.0);
                let diff = (interest - cs_interest).abs();
                let relative = diff / reference;

                if relative > tol {
                    findings.push(CheckFinding {
                        check_id: self.id().to_string(),
                        severity: Severity::Warning,
                        message: format!(
                            "Interest expense ({interest:.2}) differs from capital-structure \
                             interest ({cs_interest:.2}) in {pid} by {:.1}%",
                            relative * 100.0,
                        ),
                        period: Some(*pid),
                        materiality: Some(Materiality {
                            absolute: diff,
                            relative_pct: relative * 100.0,
                            reference_value: cs_interest,
                            reference_label: "cs_interest".to_string(),
                        }),
                        nodes: vec![self.interest_expense_node.clone(), cs_node.clone()],
                    });
                }
                continue;
            }

            // Path 2: implied-rate reasonableness against debt balances.
            if interest.abs() < f64::EPSILON {
                continue;
            }

            let mut total_implied_interest = 0.0_f64;
            let mut has_rate = false;

            for (balance_node, rate_node) in &self.debt_balance_nodes {
                let Some(balance) = get_node_value(context.results, balance_node, pid) else {
                    continue;
                };
                if let Some(rn) = rate_node {
                    if let Some(rate) = get_node_value(context.results, rn, pid) {
                        total_implied_interest += balance * rate;
                        has_rate = true;
                    }
                }
            }

            if has_rate && total_implied_interest.abs() > f64::EPSILON {
                let reference = total_implied_interest.abs().max(1.0);
                let diff = (interest - total_implied_interest).abs();
                let relative = diff / reference;

                if relative > tol {
                    let mut nodes = vec![self.interest_expense_node.clone()];
                    nodes.extend(self.debt_balance_nodes.iter().map(|(b, _)| b.clone()));

                    findings.push(CheckFinding {
                        check_id: self.id().to_string(),
                        severity: Severity::Warning,
                        message: format!(
                            "Interest expense ({interest:.2}) differs from implied interest \
                             ({total_implied_interest:.2}) in {pid} by {:.1}%",
                            relative * 100.0,
                        ),
                        period: Some(*pid),
                        materiality: Some(Materiality {
                            absolute: diff,
                            relative_pct: relative * 100.0,
                            reference_value: total_implied_interest,
                            reference_label: "implied_interest".to_string(),
                        }),
                        nodes,
                    });
                }
            }
        }

        let passed = !findings.iter().any(|f| f.severity >= Severity::Error);

        Ok(CheckResult {
            check_id: self.id().to_string(),
            check_name: self.name().to_string(),
            category: self.category(),
            passed,
            findings,
        })
    }
}
