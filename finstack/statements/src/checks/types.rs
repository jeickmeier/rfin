//! Core types for the financial statement checks framework.

use finstack_core::dates::PeriodId;
use serde::{Deserialize, Serialize};

use crate::types::NodeId;

// ---------------------------------------------------------------------------
// Severity
// ---------------------------------------------------------------------------

/// Severity level for a check finding, ordered from least to most severe.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Informational finding — no action required.
    #[default]
    Info,
    /// Warning — review recommended.
    Warning,
    /// Error — indicates a likely problem that should be addressed.
    Error,
}

// ---------------------------------------------------------------------------
// CheckCategory
// ---------------------------------------------------------------------------

/// Category that groups related checks together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckCategory {
    /// Balance sheet balances, retained earnings flow-through, cash ties
    AccountingIdentity,
    /// Depreciation ties to PP&E, interest ties to debt schedule, etc.
    CrossStatementReconciliation,
    /// Growth rate plausibility, effective tax rate range, WC consistency
    InternalConsistency,
    /// Leverage ranges, coverage floors, FCF sign, liquidity runway
    CreditReasonableness,
    /// Missing values, NaN/Inf, sign conventions
    DataQuality,
}

// ---------------------------------------------------------------------------
// PeriodScope
// ---------------------------------------------------------------------------

/// Scope that determines which periods a check applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeriodScope {
    /// Run the check on every period.
    AllPeriods,
    /// Run only on actual (historical) periods.
    ActualsOnly,
    /// Run only on forecast periods.
    ForecastOnly,
}

// ---------------------------------------------------------------------------
// Materiality
// ---------------------------------------------------------------------------

/// Materiality context attached to a finding, describing its quantitative
/// significance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Materiality {
    /// Absolute amount of the discrepancy.
    pub absolute: f64,
    /// Discrepancy as a percentage of the reference value.
    pub relative_pct: f64,
    /// The reference (denominator) value used when computing `relative_pct`.
    pub reference_value: f64,
    /// Human-readable label for the reference (e.g. "total_assets").
    pub reference_label: String,
}

// ---------------------------------------------------------------------------
// CheckFinding
// ---------------------------------------------------------------------------

/// A single finding produced by a check for a specific period or node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckFinding {
    /// Identifier of the check that produced this finding.
    pub check_id: String,
    /// Severity of the finding.
    pub severity: Severity,
    /// Human-readable description of the issue.
    pub message: String,
    /// Period the finding relates to, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<PeriodId>,
    /// Materiality context, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub materiality: Option<Materiality>,
    /// Node identifiers involved in the finding.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<NodeId>,
}

// ---------------------------------------------------------------------------
// CheckResult
// ---------------------------------------------------------------------------

/// Outcome of a single check execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckResult {
    /// Identifier of the check.
    pub check_id: String,
    /// Human-readable name of the check.
    pub check_name: String,
    /// Category this check belongs to.
    pub category: CheckCategory,
    /// Whether the check passed (no error-severity findings).
    pub passed: bool,
    /// Individual findings produced by the check.
    pub findings: Vec<CheckFinding>,
}

// ---------------------------------------------------------------------------
// CheckConfig
// ---------------------------------------------------------------------------

fn default_check_tolerance() -> f64 {
    0.01
}

/// Configuration parameters that govern check execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckConfig {
    /// Default numeric tolerance for equality comparisons (fraction, not percent).
    #[serde(default = "default_check_tolerance")]
    pub default_tolerance: f64,
    /// Findings below this absolute materiality threshold are excluded from reports.
    #[serde(default)]
    pub materiality_threshold: f64,
    /// Minimum severity a finding must have to appear in the report.
    #[serde(default)]
    pub min_severity: Severity,
}

impl Default for CheckConfig {
    fn default() -> Self {
        Self {
            default_tolerance: 0.01,
            materiality_threshold: 0.0,
            min_severity: Severity::Info,
        }
    }
}

// ---------------------------------------------------------------------------
// CheckSummary
// ---------------------------------------------------------------------------

/// Aggregate counts for a completed check run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckSummary {
    /// Total number of checks executed.
    pub total_checks: usize,
    /// Number of checks that passed.
    pub passed: usize,
    /// Number of checks that failed (at least one error-severity finding).
    pub failed: usize,
    /// Total number of error-severity findings across all checks.
    pub errors: usize,
    /// Total number of warning-severity findings across all checks.
    pub warnings: usize,
    /// Total number of info-severity findings across all checks.
    pub infos: usize,
}

// ---------------------------------------------------------------------------
// CheckReport
// ---------------------------------------------------------------------------

/// Full report aggregating all [`CheckResult`]s from a check run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckReport {
    /// Individual results for each check.
    pub results: Vec<CheckResult>,
    /// Aggregate summary.
    pub summary: CheckSummary,
}

impl CheckReport {
    /// Return all findings matching the given severity.
    pub fn findings_by_severity(&self, severity: Severity) -> Vec<&CheckFinding> {
        self.results
            .iter()
            .flat_map(|r| &r.findings)
            .filter(|f| f.severity == severity)
            .collect()
    }

    /// Return all findings from checks in the given category.
    pub fn findings_by_category(&self, category: CheckCategory) -> Vec<&CheckFinding> {
        self.results
            .iter()
            .filter(|r| r.category == category)
            .flat_map(|r| &r.findings)
            .collect()
    }

    /// Return all findings that reference the given period.
    pub fn findings_by_period(&self, period: &PeriodId) -> Vec<&CheckFinding> {
        self.results
            .iter()
            .flat_map(|r| &r.findings)
            .filter(|f| f.period.as_ref() == Some(period))
            .collect()
    }

    /// Return all findings that reference a specific node.
    pub fn findings_by_node(&self, node_id: &NodeId) -> Vec<&CheckFinding> {
        self.results
            .iter()
            .flat_map(|r| &r.findings)
            .filter(|f| f.nodes.contains(node_id))
            .collect()
    }

    /// True if the report contains at least one error-severity finding.
    pub fn has_errors(&self) -> bool {
        self.summary.errors > 0
    }

    /// True if the report contains at least one warning-severity finding.
    pub fn has_warnings(&self) -> bool {
        self.summary.warnings > 0
    }

    /// Return findings whose absolute materiality is at least the given threshold.
    pub fn material_findings(&self, threshold: f64) -> Vec<&CheckFinding> {
        self.results
            .iter()
            .flat_map(|r| &r.findings)
            .filter(|f| {
                f.materiality
                    .as_ref()
                    .is_some_and(|m| m.absolute.abs() >= threshold)
            })
            .collect()
    }
}
