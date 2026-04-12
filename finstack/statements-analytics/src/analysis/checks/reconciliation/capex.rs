//! Capital-expenditure reconciliation check.

use serde::{Deserialize, Serialize};

use super::super::get_node_value;
use finstack_statements::checks::{
    Check, CheckCategory, CheckContext, CheckFinding, CheckResult, Materiality, Severity,
};
use finstack_statements::types::NodeId;
use finstack_statements::Result;

/// Verifies that cash-flow-statement capex equals the sum of PP&E additions
/// and intangible additions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapexReconciliation {
    /// Capex from the cash flow statement (investing section).
    pub capex_cf_node: NodeId,
    /// Optional PP&E additions node.
    pub ppe_additions_node: Option<NodeId>,
    /// Optional intangible-asset additions node.
    pub intangible_additions_node: Option<NodeId>,
    /// Tolerance override; falls back to
    /// [`finstack_statements::checks::CheckConfig::default_tolerance`].
    pub tolerance: Option<f64>,
}

impl Check for CapexReconciliation {
    fn id(&self) -> &str {
        "capex_reconciliation"
    }

    fn name(&self) -> &str {
        "Capex Reconciliation"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::CrossStatementReconciliation
    }

    fn execute(&self, context: &CheckContext) -> Result<CheckResult> {
        let tolerance = self.tolerance.unwrap_or(context.config.default_tolerance);
        let mut findings = Vec::new();

        let has_components =
            self.ppe_additions_node.is_some() || self.intangible_additions_node.is_some();
        if !has_components {
            return Ok(CheckResult {
                check_id: self.id().to_string(),
                check_name: self.name().to_string(),
                category: self.category(),
                passed: true,
                findings,
            });
        }

        for period_spec in &context.model.periods {
            let pid = &period_spec.id;

            let Some(capex_cf) = get_node_value(context.results, &self.capex_cf_node, pid) else {
                continue;
            };

            let ppe_add = self
                .ppe_additions_node
                .as_ref()
                .and_then(|n| get_node_value(context.results, n, pid))
                .unwrap_or(0.0);

            let intangible_add = self
                .intangible_additions_node
                .as_ref()
                .and_then(|n| get_node_value(context.results, n, pid))
                .unwrap_or(0.0);

            let expected = ppe_add + intangible_add;
            let diff = capex_cf - expected;

            if diff.abs() > tolerance {
                let reference = expected.abs().max(1.0);
                let relative = (diff / reference).abs() * 100.0;

                let mut nodes = vec![self.capex_cf_node.clone()];
                if let Some(ref n) = self.ppe_additions_node {
                    nodes.push(n.clone());
                }
                if let Some(ref n) = self.intangible_additions_node {
                    nodes.push(n.clone());
                }

                findings.push(CheckFinding {
                    check_id: self.id().to_string(),
                    severity: Severity::Warning,
                    message: format!(
                        "CF capex ({capex_cf:.2}) != PPE additions ({ppe_add:.2}) + \
                         intangible additions ({intangible_add:.2}) = {expected:.2} \
                         in {pid}, difference = {diff:.2}"
                    ),
                    period: Some(*pid),
                    materiality: Some(Materiality {
                        absolute: diff.abs(),
                        relative_pct: relative,
                        reference_value: expected,
                        reference_label: "balance_sheet_additions".to_string(),
                    }),
                    nodes,
                });
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
