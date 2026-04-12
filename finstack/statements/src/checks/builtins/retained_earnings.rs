//! Retained earnings reconciliation check.

use serde::{Deserialize, Serialize};

use super::{get_node_value, sum_nodes};
use crate::checks::{
    Check, CheckCategory, CheckContext, CheckFinding, CheckResult, Materiality, Severity,
};
use crate::types::NodeId;
use crate::Result;

/// Checks that retained earnings flow correctly across periods:
/// RE(t) = RE(t−1) + NI(t) − Dividends(t) ± Adjustments(t).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetainedEarningsReconciliation {
    /// Node for retained earnings balance.
    pub retained_earnings_node: NodeId,
    /// Node for net income.
    pub net_income_node: NodeId,
    /// Optional node for dividends paid.
    pub dividends_node: Option<NodeId>,
    /// Additional adjustment nodes (buybacks, AOCI, etc.).
    pub other_adjustments: Vec<NodeId>,
    /// Tolerance override; falls back to [`crate::CheckConfig::default_tolerance`].
    pub tolerance: Option<f64>,
}

impl Check for RetainedEarningsReconciliation {
    fn id(&self) -> &str {
        "retained_earnings_reconciliation"
    }

    fn name(&self) -> &str {
        "Retained Earnings Reconciliation"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::AccountingIdentity
    }

    fn execute(&self, context: &CheckContext) -> Result<CheckResult> {
        let tolerance = self.tolerance.unwrap_or(context.config.default_tolerance);
        let mut findings = Vec::new();
        let periods = &context.model.periods;

        for i in 1..periods.len() {
            let prev_pid = &periods[i - 1].id;
            let curr_pid = &periods[i].id;

            let Some(re_prev) =
                get_node_value(context.results, &self.retained_earnings_node, prev_pid)
            else {
                continue;
            };
            let Some(re_curr) =
                get_node_value(context.results, &self.retained_earnings_node, curr_pid)
            else {
                continue;
            };
            let Some(ni) = get_node_value(context.results, &self.net_income_node, curr_pid) else {
                continue;
            };

            let dividends = self
                .dividends_node
                .as_ref()
                .and_then(|n| get_node_value(context.results, n, curr_pid))
                .unwrap_or(0.0);

            let adjustments = sum_nodes(context.results, &self.other_adjustments, curr_pid);

            let expected = re_prev + ni - dividends + adjustments;
            let diff = re_curr - expected;

            if diff.abs() > tolerance {
                let reference = re_prev.abs().max(1.0);
                let relative = (diff / reference).abs() * 100.0;

                let mut nodes = vec![
                    self.retained_earnings_node.clone(),
                    self.net_income_node.clone(),
                ];
                if let Some(ref d) = self.dividends_node {
                    nodes.push(d.clone());
                }
                nodes.extend(self.other_adjustments.iter().cloned());

                findings.push(CheckFinding {
                    check_id: self.id().to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "Retained earnings do not reconcile in {curr_pid}: \
                         actual RE ({re_curr:.2}) != expected ({expected:.2}), \
                         difference = {diff:.2}"
                    ),
                    period: Some(*curr_pid),
                    materiality: Some(Materiality {
                        absolute: diff.abs(),
                        relative_pct: relative,
                        reference_value: re_prev,
                        reference_label: "prior_retained_earnings".to_string(),
                    }),
                    nodes,
                });
            }
        }

        let passed = !findings.iter().any(|f| f.severity == Severity::Error);

        Ok(CheckResult {
            check_id: self.id().to_string(),
            check_name: self.name().to_string(),
            category: self.category(),
            passed,
            findings,
        })
    }
}
