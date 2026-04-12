# Financial Statement Checks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a composable, customizable framework for validating financial statement models — from accounting identities to credit reasonableness checks — across `finstack-statements` (structural) and `finstack-statements-analytics` (domain).

**Architecture:** A `Check` trait and typed result types in `finstack-statements::checks` provide the core abstraction. Six built-in structural checks live in `checks::builtins`. Eleven domain checks, pre-built suites, `FormulaCheck` DSL adapter, and a report renderer live in `finstack-statements-analytics::analysis::checks`. The `Evaluator` optionally runs a `CheckSuite` inline and attaches the report to `StatementResult`.

**Tech Stack:** Rust (serde, indexmap), existing `finstack-statements` DSL engine, existing `finstack-core` types (`PeriodId`, `NodeId`, `Period`).

**Spec:** `docs/superpowers/specs/2026-04-12-financial-statement-checks-design.md`

---

## File Structure

### `finstack-statements` (new files)

| File | Responsibility |
|------|---------------|
| `src/checks/mod.rs` | Module declaration, re-exports |
| `src/checks/types.rs` | `Severity`, `CheckCategory`, `Materiality`, `CheckFinding`, `CheckResult`, `CheckReport`, `CheckSummary`, `CheckConfig`, `PeriodScope` |
| `src/checks/traits.rs` | `Check` trait, `CheckContext` |
| `src/checks/runner.rs` | `CheckRunner` |
| `src/checks/suite.rs` | `CheckSuite`, `CheckSuiteBuilder`, `CheckSuiteSpec`, `BuiltinCheckSpec` |
| `src/checks/builtins/mod.rs` | Built-in check re-exports |
| `src/checks/builtins/balance_sheet.rs` | `BalanceSheetArticulation` |
| `src/checks/builtins/retained_earnings.rs` | `RetainedEarningsReconciliation` |
| `src/checks/builtins/cash_reconciliation.rs` | `CashReconciliation` |
| `src/checks/builtins/missing_values.rs` | `MissingValueCheck` |
| `src/checks/builtins/sign_convention.rs` | `SignConventionCheck` |
| `src/checks/builtins/non_finite.rs` | `NonFiniteCheck` |
| `tests/checks/mod.rs` | Test module wiring |
| `tests/checks/types_tests.rs` | Type serialization tests |
| `tests/checks/balance_sheet_tests.rs` | BS articulation tests |
| `tests/checks/retained_earnings_tests.rs` | RE reconciliation tests |
| `tests/checks/cash_reconciliation_tests.rs` | Cash reconciliation tests |
| `tests/checks/data_quality_tests.rs` | Missing values, sign, non-finite tests |
| `tests/checks/runner_tests.rs` | CheckRunner tests |
| `tests/checks/suite_tests.rs` | CheckSuite composition tests |
| `tests/checks_all.rs` | Top-level aggregator |

### `finstack-statements` (modified files)

| File | Change |
|------|--------|
| `src/lib.rs` | Add `pub mod checks;` and re-export key types |
| `src/prelude.rs` | Add check types to prelude |
| `src/evaluator/results.rs` | Add `check_report: Option<CheckReport>` to `StatementResult` |
| `src/evaluator/engine.rs` | Add `with_checks()` method to `Evaluator` |

### `finstack-statements-analytics` (new files)

| File | Responsibility |
|------|---------------|
| `src/analysis/checks/mod.rs` | Module declaration, re-exports |
| `src/analysis/checks/reconciliation/mod.rs` | Reconciliation check re-exports |
| `src/analysis/checks/reconciliation/depreciation.rs` | `DepreciationReconciliation` |
| `src/analysis/checks/reconciliation/interest.rs` | `InterestExpenseReconciliation` |
| `src/analysis/checks/reconciliation/capex.rs` | `CapexReconciliation` |
| `src/analysis/checks/reconciliation/dividends.rs` | `DividendReconciliation` |
| `src/analysis/checks/consistency/mod.rs` | Consistency check re-exports |
| `src/analysis/checks/consistency/growth_rate.rs` | `GrowthRateConsistency` |
| `src/analysis/checks/consistency/tax_rate.rs` | `EffectiveTaxRateCheck` |
| `src/analysis/checks/consistency/working_capital.rs` | `WorkingCapitalConsistency` |
| `src/analysis/checks/credit/mod.rs` | Credit check re-exports |
| `src/analysis/checks/credit/leverage.rs` | `LeverageRangeCheck` |
| `src/analysis/checks/credit/coverage.rs` | `CoverageFloorCheck` |
| `src/analysis/checks/credit/fcf_sign.rs` | `FcfSignCheck` |
| `src/analysis/checks/credit/trend.rs` | `TrendCheck`, `TrendDirection` |
| `src/analysis/checks/credit/liquidity.rs` | `LiquidityRunwayCheck` |
| `src/analysis/checks/formula_check.rs` | `FormulaCheck` DSL adapter |
| `src/analysis/checks/suites.rs` | `three_statement_checks`, `credit_underwriting_checks`, `lbo_model_checks` |
| `src/analysis/checks/mappings.rs` | `ThreeStatementMapping`, `CreditMapping` |
| `src/analysis/checks/corkscrew_adapter.rs` | `corkscrew_as_checks` |
| `src/analysis/checks/renderer.rs` | `CheckReportRenderer` |
| `tests/checks/mod.rs` | Test module wiring |
| `tests/checks/reconciliation_tests.rs` | Reconciliation check tests |
| `tests/checks/consistency_tests.rs` | Consistency check tests |
| `tests/checks/credit_tests.rs` | Credit check tests |
| `tests/checks/formula_check_tests.rs` | FormulaCheck tests |
| `tests/checks/suite_tests.rs` | Pre-built suite integration tests |
| `tests/checks/renderer_tests.rs` | Report rendering tests |
| `tests/checks_all.rs` | Top-level aggregator |

### `finstack-statements-analytics` (modified files)

| File | Change |
|------|--------|
| `src/analysis/mod.rs` | Add `pub mod checks;` and re-exports |
| `src/prelude.rs` | Add check types to prelude |

---

## Task 1: Core Check Types

**Files:**
- Create: `finstack/statements/src/checks/mod.rs`
- Create: `finstack/statements/src/checks/types.rs`
- Create: `finstack/statements/src/checks/traits.rs`
- Create: `finstack/statements/tests/checks/mod.rs`
- Create: `finstack/statements/tests/checks/types_tests.rs`
- Create: `finstack/statements/tests/checks_all.rs`
- Modify: `finstack/statements/src/lib.rs`

- [ ] **Step 1: Write failing tests for core types**

Create `finstack/statements/tests/checks_all.rs`:

```rust
#[path = "checks/mod.rs"]
mod checks;
```

Create `finstack/statements/tests/checks/mod.rs`:

```rust
mod types_tests;
```

Create `finstack/statements/tests/checks/types_tests.rs`:

```rust
use finstack_statements::checks::{
    CheckCategory, CheckConfig, CheckFinding, CheckReport, CheckResult, CheckSummary, Materiality,
    PeriodScope, Severity,
};

#[test]
fn test_severity_ordering() {
    assert!(Severity::Info < Severity::Warning);
    assert!(Severity::Warning < Severity::Error);
}

#[test]
fn test_severity_serde_roundtrip() {
    let json = serde_json::to_string(&Severity::Warning).unwrap();
    assert_eq!(json, r#""warning""#);
    let deserialized: Severity = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, Severity::Warning);
}

#[test]
fn test_check_category_serde_roundtrip() {
    let json = serde_json::to_string(&CheckCategory::AccountingIdentity).unwrap();
    assert_eq!(json, r#""accounting_identity""#);
    let deserialized: CheckCategory = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, CheckCategory::AccountingIdentity);
}

#[test]
fn test_period_scope_serde_roundtrip() {
    let json = serde_json::to_string(&PeriodScope::ActualsOnly).unwrap();
    assert_eq!(json, r#""actuals_only""#);
}

#[test]
fn test_materiality_serde_roundtrip() {
    let m = Materiality {
        absolute: 2_300_000.0,
        relative_pct: 0.0004,
        reference_value: 5_800_000_000.0,
        reference_label: "total_assets".into(),
    };
    let json = serde_json::to_string(&m).unwrap();
    let deserialized: Materiality = serde_json::from_str(&json).unwrap();
    assert!((deserialized.absolute - 2_300_000.0).abs() < f64::EPSILON);
}

#[test]
fn test_check_config_defaults() {
    let config: CheckConfig = serde_json::from_str("{}").unwrap();
    assert!((config.default_tolerance - 0.01).abs() < f64::EPSILON);
    assert!((config.materiality_threshold - 0.0).abs() < f64::EPSILON);
    assert_eq!(config.min_severity, Severity::Info);
}

#[test]
fn test_check_finding_serde() {
    let finding = CheckFinding {
        check_id: "bs_articulation".into(),
        severity: Severity::Error,
        message: "Balance sheet imbalance".into(),
        period: None,
        materiality: None,
        nodes: vec![],
    };
    let json = serde_json::to_string(&finding).unwrap();
    let deserialized: CheckFinding = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.check_id, "bs_articulation");
}

#[test]
fn test_check_result_passed_when_no_errors() {
    let result = CheckResult {
        check_id: "test".into(),
        check_name: "Test Check".into(),
        category: CheckCategory::DataQuality,
        passed: true,
        findings: vec![CheckFinding {
            check_id: "test".into(),
            severity: Severity::Info,
            message: "Advisory".into(),
            period: None,
            materiality: None,
            nodes: vec![],
        }],
    };
    assert!(result.passed);
}

#[test]
fn test_check_summary() {
    let summary = CheckSummary {
        total_checks: 5,
        passed: 3,
        failed: 2,
        errors: 1,
        warnings: 1,
        infos: 0,
    };
    assert_eq!(summary.total_checks, 5);
    assert_eq!(summary.failed, 2);
}

#[test]
fn test_check_report_findings_by_severity() {
    let report = CheckReport {
        results: vec![CheckResult {
            check_id: "test".into(),
            check_name: "Test".into(),
            category: CheckCategory::DataQuality,
            passed: false,
            findings: vec![
                CheckFinding {
                    check_id: "test".into(),
                    severity: Severity::Error,
                    message: "bad".into(),
                    period: None,
                    materiality: None,
                    nodes: vec![],
                },
                CheckFinding {
                    check_id: "test".into(),
                    severity: Severity::Warning,
                    message: "meh".into(),
                    period: None,
                    materiality: None,
                    nodes: vec![],
                },
            ],
        }],
        summary: CheckSummary {
            total_checks: 1,
            passed: 0,
            failed: 1,
            errors: 1,
            warnings: 1,
            infos: 0,
        },
    };

    let errors = report.findings_by_severity(Severity::Error);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].message, "bad");

    assert!(report.has_errors());
    assert!(report.has_warnings());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p finstack-statements --test checks_all`
Expected: Compilation error — `checks` module doesn't exist yet.

- [ ] **Step 3: Create the checks module and types**

Create `finstack/statements/src/checks/mod.rs`:

```rust
//! Financial statement validation checks.
//!
//! Provides a composable framework for asserting invariants on evaluated
//! financial models, from accounting identities to data quality.

pub mod builtins;
mod runner;
mod suite;
mod traits;
mod types;

pub use runner::CheckRunner;
pub use suite::{CheckSuite, CheckSuiteBuilder};
pub use traits::{Check, CheckContext};
pub use types::{
    CheckCategory, CheckConfig, CheckFinding, CheckReport, CheckResult, CheckSummary, Materiality,
    PeriodScope, Severity,
};
```

Create `finstack/statements/src/checks/types.rs`:

```rust
//! Core types for the check framework.

use crate::types::NodeId;
use finstack_core::dates::PeriodId;
use serde::{Deserialize, Serialize};

/// Severity of a check finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Advisory — no action required
    Info,
    /// Potential issue — review recommended
    Warning,
    /// Likely model error — investigation required
    Error,
}

impl Default for Severity {
    fn default() -> Self {
        Self::Info
    }
}

/// Category of financial statement check.
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

/// Which periods a check applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeriodScope {
    /// Check all periods
    AllPeriods,
    /// Check only actual (historical) periods
    ActualsOnly,
    /// Check only forecast (projected) periods
    ForecastOnly,
}

/// Materiality context for a check finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Materiality {
    /// Absolute amount of the discrepancy
    pub absolute: f64,
    /// Discrepancy as a fraction of the reference value (e.g., 0.0004 = 0.04%)
    pub relative_pct: f64,
    /// The denominator used for the relative calculation
    pub reference_value: f64,
    /// Human-readable label for the reference (e.g., "total_assets")
    pub reference_label: String,
}

/// A single observation produced by a check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckFinding {
    /// Which check produced this finding
    pub check_id: String,
    /// Severity level
    pub severity: Severity,
    /// Human-readable description
    pub message: String,
    /// Which period this applies to (None = model-level)
    pub period: Option<PeriodId>,
    /// Materiality context (None for non-quantifiable findings)
    pub materiality: Option<Materiality>,
    /// Which nodes are involved
    pub nodes: Vec<NodeId>,
}

/// Output of a single check execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    /// Unique check identifier
    pub check_id: String,
    /// Human-readable check name
    pub check_name: String,
    /// Which category this check belongs to
    pub category: CheckCategory,
    /// Whether the check passed (no Error-severity findings)
    pub passed: bool,
    /// All findings from this check
    pub findings: Vec<CheckFinding>,
}

/// Global check configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckConfig {
    /// Default absolute tolerance for equality checks
    #[serde(default = "default_check_tolerance")]
    pub default_tolerance: f64,
    /// Ignore findings with materiality below this absolute amount
    #[serde(default)]
    pub materiality_threshold: f64,
    /// Only report findings at or above this severity
    #[serde(default)]
    pub min_severity: Severity,
}

fn default_check_tolerance() -> f64 {
    0.01
}

impl Default for CheckConfig {
    fn default() -> Self {
        Self {
            default_tolerance: default_check_tolerance(),
            materiality_threshold: 0.0,
            min_severity: Severity::Info,
        }
    }
}

/// Aggregated summary of a check run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckSummary {
    /// Total number of checks executed
    pub total_checks: usize,
    /// Number of checks that passed
    pub passed: usize,
    /// Number of checks that failed
    pub failed: usize,
    /// Total Error-severity findings
    pub errors: usize,
    /// Total Warning-severity findings
    pub warnings: usize,
    /// Total Info-severity findings
    pub infos: usize,
}

/// Aggregated check report from running a suite of checks.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckReport {
    /// Individual check results
    pub results: Vec<CheckResult>,
    /// Aggregated summary
    pub summary: CheckSummary,
}

impl CheckReport {
    /// Get all findings with a specific severity.
    pub fn findings_by_severity(&self, severity: Severity) -> Vec<&CheckFinding> {
        self.results
            .iter()
            .flat_map(|r| r.findings.iter())
            .filter(|f| f.severity == severity)
            .collect()
    }

    /// Get all findings in a specific category.
    pub fn findings_by_category(&self, category: CheckCategory) -> Vec<&CheckFinding> {
        self.results
            .iter()
            .filter(|r| r.category == category)
            .flat_map(|r| r.findings.iter())
            .collect()
    }

    /// Get all findings for a specific period.
    pub fn findings_by_period(&self, period: &PeriodId) -> Vec<&CheckFinding> {
        self.results
            .iter()
            .flat_map(|r| r.findings.iter())
            .filter(|f| f.period.as_ref() == Some(period))
            .collect()
    }

    /// Get all findings involving a specific node.
    pub fn findings_by_node(&self, node: &NodeId) -> Vec<&CheckFinding> {
        self.results
            .iter()
            .flat_map(|r| r.findings.iter())
            .filter(|f| f.nodes.contains(node))
            .collect()
    }

    /// Whether any finding has Error severity.
    pub fn has_errors(&self) -> bool {
        self.summary.errors > 0
    }

    /// Whether any finding has Warning severity.
    pub fn has_warnings(&self) -> bool {
        self.summary.warnings > 0
    }

    /// Get findings where materiality exceeds a threshold.
    pub fn material_findings(&self, threshold: f64) -> Vec<&CheckFinding> {
        self.results
            .iter()
            .flat_map(|r| r.findings.iter())
            .filter(|f| {
                f.materiality
                    .as_ref()
                    .is_some_and(|m| m.absolute >= threshold)
            })
            .collect()
    }
}
```

Create `finstack/statements/src/checks/traits.rs`:

```rust
//! Check trait and context.

use crate::checks::types::{CheckConfig, CheckResult};
use crate::evaluator::StatementResult;
use crate::types::FinancialModelSpec;
use crate::Result;

/// Context passed to checks during execution.
#[derive(Debug)]
pub struct CheckContext<'a> {
    /// The financial model spec
    pub model: &'a FinancialModelSpec,
    /// Evaluated results
    pub results: &'a StatementResult,
    /// Global check configuration
    pub config: CheckConfig,
}

impl<'a> CheckContext<'a> {
    /// Create a new check context with default configuration.
    pub fn new(model: &'a FinancialModelSpec, results: &'a StatementResult) -> Self {
        Self {
            model,
            results,
            config: CheckConfig::default(),
        }
    }

    /// Create a check context with custom configuration.
    pub fn with_config(
        model: &'a FinancialModelSpec,
        results: &'a StatementResult,
        config: CheckConfig,
    ) -> Self {
        Self {
            model,
            results,
            config,
        }
    }
}

/// Trait for financial statement checks.
///
/// Each check validates a specific invariant on an evaluated model and
/// returns structured findings with severity and materiality.
pub trait Check: Send + Sync {
    /// Unique identifier (e.g., "balance_sheet_articulation").
    fn id(&self) -> &str;
    /// Human-readable name (e.g., "Balance Sheet Articulation").
    fn name(&self) -> &str;
    /// Which category this check belongs to.
    fn category(&self) -> crate::checks::CheckCategory;
    /// Execute the check against evaluated results.
    fn execute(&self, context: &CheckContext) -> Result<CheckResult>;
}
```

Create placeholder `finstack/statements/src/checks/runner.rs`:

```rust
//! Check runner for executing multiple checks.

use crate::checks::traits::CheckContext;
use crate::checks::types::{CheckReport, CheckSummary, Severity};
use crate::checks::Check;

/// Runs a collection of checks and aggregates results.
pub struct CheckRunner {
    checks: Vec<Box<dyn Check>>,
}

impl CheckRunner {
    /// Create an empty runner.
    pub fn new() -> Self {
        Self { checks: vec![] }
    }

    /// Add a check to the runner.
    pub fn add_check(&mut self, check: impl Check + 'static) -> &mut Self {
        self.checks.push(Box::new(check));
        self
    }

    /// Execute all checks and return an aggregated report.
    pub fn run(&self, context: &CheckContext) -> CheckReport {
        let mut results = Vec::with_capacity(self.checks.len());

        for check in &self.checks {
            match check.execute(context) {
                Ok(result) => results.push(result),
                Err(e) => {
                    tracing::warn!(check_id = check.id(), error = %e, "Check execution failed");
                }
            }
        }

        let summary = build_summary(&results);
        CheckReport { results, summary }
    }
}

impl Default for CheckRunner {
    fn default() -> Self {
        Self::new()
    }
}

fn build_summary(results: &[crate::checks::CheckResult]) -> CheckSummary {
    let mut summary = CheckSummary::default();
    summary.total_checks = results.len();

    for result in results {
        if result.passed {
            summary.passed += 1;
        } else {
            summary.failed += 1;
        }
        for finding in &result.findings {
            match finding.severity {
                Severity::Error => summary.errors += 1,
                Severity::Warning => summary.warnings += 1,
                Severity::Info => summary.infos += 1,
            }
        }
    }
    summary
}
```

Create placeholder `finstack/statements/src/checks/suite.rs`:

```rust
//! Check suite composition.

use crate::checks::traits::CheckContext;
use crate::checks::types::{CheckConfig, CheckReport, Severity};
use crate::checks::{Check, CheckRunner};
use crate::evaluator::StatementResult;
use crate::types::FinancialModelSpec;

/// A named, reusable collection of checks with configuration.
pub struct CheckSuite {
    /// Suite name
    pub name: String,
    /// Suite description
    pub description: String,
    checks: Vec<Box<dyn Check>>,
    /// Global configuration for checks in this suite
    pub config: CheckConfig,
}

impl CheckSuite {
    /// Create a suite builder.
    pub fn builder(name: &str) -> CheckSuiteBuilder {
        CheckSuiteBuilder {
            name: name.to_string(),
            description: String::new(),
            checks: vec![],
            config: CheckConfig::default(),
        }
    }

    /// Merge another suite's checks into this one.
    pub fn merge(mut self, other: CheckSuite) -> Self {
        self.checks.extend(other.checks);
        self
    }

    /// Run all checks against a model and its evaluated results.
    pub fn run(&self, model: &FinancialModelSpec, results: &StatementResult) -> CheckReport {
        let context = CheckContext::with_config(model, results, self.config.clone());
        let mut runner = CheckRunner::new();
        for check in &self.checks {
            // Re-box: CheckRunner takes owned checks but we only have refs.
            // We'll refactor runner to accept &dyn Check in implementation.
        }
        // For now, delegate directly
        self.run_internal(&context)
    }

    fn run_internal(&self, context: &CheckContext) -> CheckReport {
        let mut results = Vec::with_capacity(self.checks.len());

        for check in &self.checks {
            match check.execute(context) {
                Ok(mut result) => {
                    // Apply min_severity filter
                    result.findings.retain(|f| f.severity >= context.config.min_severity);
                    // Apply materiality filter
                    if context.config.materiality_threshold > 0.0 {
                        result.findings.retain(|f| {
                            f.materiality
                                .as_ref()
                                .map_or(true, |m| m.absolute >= context.config.materiality_threshold)
                        });
                    }
                    result.passed = !result.findings.iter().any(|f| f.severity == Severity::Error);
                    results.push(result);
                }
                Err(e) => {
                    tracing::warn!(check_id = check.id(), error = %e, "Check execution failed");
                }
            }
        }

        let summary = crate::checks::runner::build_summary(&results);
        CheckReport { results, summary }
    }

    /// Get the number of checks in this suite.
    pub fn len(&self) -> usize {
        self.checks.len()
    }

    /// Whether the suite has no checks.
    pub fn is_empty(&self) -> bool {
        self.checks.is_empty()
    }
}

/// Builder for constructing check suites.
pub struct CheckSuiteBuilder {
    name: String,
    description: String,
    checks: Vec<Box<dyn Check>>,
    config: CheckConfig,
}

impl CheckSuiteBuilder {
    /// Set suite description.
    pub fn description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    /// Set default tolerance.
    pub fn tolerance(mut self, tolerance: f64) -> Self {
        self.config.default_tolerance = tolerance;
        self
    }

    /// Set materiality threshold.
    pub fn materiality_threshold(mut self, threshold: f64) -> Self {
        self.config.materiality_threshold = threshold;
        self
    }

    /// Set minimum severity filter.
    pub fn min_severity(mut self, severity: Severity) -> Self {
        self.config.min_severity = severity;
        self
    }

    /// Add a check to the suite.
    pub fn add_check(mut self, check: impl Check + 'static) -> Self {
        self.checks.push(Box::new(check));
        self
    }

    /// Build the suite.
    pub fn build(self) -> CheckSuite {
        CheckSuite {
            name: self.name,
            description: self.description,
            checks: self.checks,
            config: self.config,
        }
    }
}
```

Create empty `finstack/statements/src/checks/builtins/mod.rs`:

```rust
//! Built-in structural checks for financial model validation.
```

- [ ] **Step 4: Wire up the module in lib.rs**

Add to `finstack/statements/src/lib.rs` after the existing module declarations:

```rust
/// Financial statement validation checks.
pub mod checks;
```

Add to the `pub use` block at the bottom of `lib.rs`:

```rust
pub use checks::{Check, CheckCategory, CheckConfig, CheckContext, CheckFinding, CheckReport,
    CheckResult, CheckRunner, CheckSuite, CheckSuiteBuilder, CheckSummary, Materiality,
    PeriodScope, Severity};
```

Also expose `build_summary` as `pub(crate)` in `runner.rs` (change `fn build_summary` to `pub(crate) fn build_summary`).

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p finstack-statements --test checks_all`
Expected: All tests pass.

- [ ] **Step 6: Run clippy**

Run: `cargo clippy -p finstack-statements -- -D warnings`
Expected: Clean.

- [ ] **Step 7: Commit**

```bash
git add finstack/statements/src/checks/ finstack/statements/tests/checks/ finstack/statements/tests/checks_all.rs finstack/statements/src/lib.rs
git commit -m "feat(statements): add core check framework types and runner"
```

---

## Task 2: Built-in Structural Checks — Balance Sheet Articulation

**Files:**
- Create: `finstack/statements/src/checks/builtins/balance_sheet.rs`
- Create: `finstack/statements/tests/checks/balance_sheet_tests.rs`
- Modify: `finstack/statements/src/checks/builtins/mod.rs`
- Modify: `finstack/statements/tests/checks/mod.rs`

- [ ] **Step 1: Write failing tests**

Create `finstack/statements/tests/checks/balance_sheet_tests.rs`:

```rust
use finstack_core::dates::PeriodId;
use finstack_statements::builder::ModelBuilder;
use finstack_statements::checks::builtins::BalanceSheetArticulation;
use finstack_statements::checks::{Check, CheckCategory, CheckContext, Severity};
use finstack_statements::evaluator::Evaluator;
use finstack_statements::types::{AmountOrScalar, NodeId};

#[test]
fn test_balanced_sheet_passes() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None)
        .unwrap()
        .value("total_assets", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(1_000.0)),
            (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(1_100.0)),
        ])
        .value("total_liabilities", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(600.0)),
            (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(650.0)),
        ])
        .value("total_equity", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(400.0)),
            (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(450.0)),
        ])
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model).unwrap();

    let check = BalanceSheetArticulation {
        assets_nodes: vec![NodeId::new("total_assets")],
        liabilities_nodes: vec![NodeId::new("total_liabilities")],
        equity_nodes: vec![NodeId::new("total_equity")],
        tolerance: None,
    };

    let context = CheckContext::new(&model, &results);
    let result = check.execute(&context).unwrap();

    assert!(result.passed);
    assert!(result.findings.is_empty());
    assert_eq!(check.category(), CheckCategory::AccountingIdentity);
}

#[test]
fn test_imbalanced_sheet_fails() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q1", None)
        .unwrap()
        .value("assets", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(1_000.0)),
        ])
        .value("liabilities", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(500.0)),
        ])
        .value("equity", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(400.0)),
        ])
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model).unwrap();

    let check = BalanceSheetArticulation {
        assets_nodes: vec![NodeId::new("assets")],
        liabilities_nodes: vec![NodeId::new("liabilities")],
        equity_nodes: vec![NodeId::new("equity")],
        tolerance: None,
    };

    let context = CheckContext::new(&model, &results);
    let result = check.execute(&context).unwrap();

    assert!(!result.passed);
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].severity, Severity::Error);
    assert!(result.findings[0].materiality.is_some());

    let mat = result.findings[0].materiality.as_ref().unwrap();
    assert!((mat.absolute - 100.0).abs() < 0.01);
}

#[test]
fn test_within_tolerance_passes() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q1", None)
        .unwrap()
        .value("assets", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(1_000.005)),
        ])
        .value("liabilities", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(600.0)),
        ])
        .value("equity", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(400.0)),
        ])
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model).unwrap();

    let check = BalanceSheetArticulation {
        assets_nodes: vec![NodeId::new("assets")],
        liabilities_nodes: vec![NodeId::new("liabilities")],
        equity_nodes: vec![NodeId::new("equity")],
        tolerance: Some(0.01),
    };

    let context = CheckContext::new(&model, &results);
    let result = check.execute(&context).unwrap();
    assert!(result.passed);
}
```

Add to `finstack/statements/tests/checks/mod.rs`:

```rust
mod balance_sheet_tests;
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p finstack-statements --test checks_all -- balance_sheet`
Expected: Compilation error — `builtins::BalanceSheetArticulation` doesn't exist.

- [ ] **Step 3: Implement BalanceSheetArticulation**

Create `finstack/statements/src/checks/builtins/balance_sheet.rs`:

```rust
//! Balance sheet articulation check (Assets = Liabilities + Equity).

use crate::checks::traits::CheckContext;
use crate::checks::types::{
    CheckCategory, CheckFinding, CheckResult, Materiality, Severity,
};
use crate::checks::Check;
use crate::types::NodeId;
use crate::Result;
use serde::{Deserialize, Serialize};

/// Verifies Assets = Liabilities + Equity for each period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceSheetArticulation {
    /// Node IDs summed to get total assets
    pub assets_nodes: Vec<NodeId>,
    /// Node IDs summed to get total liabilities
    pub liabilities_nodes: Vec<NodeId>,
    /// Node IDs summed to get total equity
    pub equity_nodes: Vec<NodeId>,
    /// Override tolerance (uses CheckConfig default if None)
    pub tolerance: Option<f64>,
}

impl Check for BalanceSheetArticulation {
    fn id(&self) -> &str {
        "balance_sheet_articulation"
    }

    fn name(&self) -> &str {
        "Balance Sheet Articulation"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::AccountingIdentity
    }

    fn execute(&self, context: &CheckContext) -> Result<CheckResult> {
        let tolerance = self.tolerance.unwrap_or(context.config.default_tolerance);
        let mut findings = Vec::new();

        for period in &context.model.periods {
            let period_id = &period.id;

            let assets = sum_nodes(&self.assets_nodes, period_id, context);
            let liabilities = sum_nodes(&self.liabilities_nodes, period_id, context);
            let equity = sum_nodes(&self.equity_nodes, period_id, context);

            let imbalance = assets - (liabilities + equity);

            if imbalance.abs() > tolerance {
                let materiality = if assets.abs() > f64::EPSILON {
                    Some(Materiality {
                        absolute: imbalance.abs(),
                        relative_pct: imbalance.abs() / assets.abs(),
                        reference_value: assets,
                        reference_label: "total_assets".into(),
                    })
                } else {
                    Some(Materiality {
                        absolute: imbalance.abs(),
                        relative_pct: 0.0,
                        reference_value: 0.0,
                        reference_label: "total_assets".into(),
                    })
                };

                let mut nodes: Vec<NodeId> = Vec::new();
                nodes.extend(self.assets_nodes.iter().cloned());
                nodes.extend(self.liabilities_nodes.iter().cloned());
                nodes.extend(self.equity_nodes.iter().cloned());

                findings.push(CheckFinding {
                    check_id: self.id().into(),
                    severity: Severity::Error,
                    message: format!(
                        "Balance sheet not articulated in {}: Assets ({:.2}) != Liabilities ({:.2}) + Equity ({:.2}), imbalance: {:.2}",
                        period_id, assets, liabilities, equity, imbalance.abs()
                    ),
                    period: Some(*period_id),
                    materiality,
                    nodes,
                });
            }
        }

        let passed = !findings.iter().any(|f| f.severity == Severity::Error);

        Ok(CheckResult {
            check_id: self.id().into(),
            check_name: self.name().into(),
            category: self.category(),
            passed,
            findings,
        })
    }
}

fn sum_nodes(nodes: &[NodeId], period_id: &finstack_core::dates::PeriodId, context: &CheckContext) -> f64 {
    nodes
        .iter()
        .filter_map(|node| {
            context
                .results
                .nodes
                .get(node.as_str())
                .and_then(|periods| periods.get(period_id))
        })
        .sum()
}
```

Update `finstack/statements/src/checks/builtins/mod.rs`:

```rust
//! Built-in structural checks for financial model validation.

mod balance_sheet;

pub use balance_sheet::BalanceSheetArticulation;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p finstack-statements --test checks_all -- balance_sheet`
Expected: All 3 tests pass.

- [ ] **Step 5: Run clippy**

Run: `cargo clippy -p finstack-statements -- -D warnings`
Expected: Clean.

- [ ] **Step 6: Commit**

```bash
git add finstack/statements/src/checks/builtins/ finstack/statements/tests/checks/balance_sheet_tests.rs finstack/statements/tests/checks/mod.rs
git commit -m "feat(statements): add BalanceSheetArticulation check"
```

---

## Task 3: Built-in Structural Checks — Retained Earnings & Cash Reconciliation

**Files:**
- Create: `finstack/statements/src/checks/builtins/retained_earnings.rs`
- Create: `finstack/statements/src/checks/builtins/cash_reconciliation.rs`
- Create: `finstack/statements/tests/checks/retained_earnings_tests.rs`
- Create: `finstack/statements/tests/checks/cash_reconciliation_tests.rs`
- Modify: `finstack/statements/src/checks/builtins/mod.rs`
- Modify: `finstack/statements/tests/checks/mod.rs`

- [ ] **Step 1: Write failing tests for RetainedEarningsReconciliation**

Create `finstack/statements/tests/checks/retained_earnings_tests.rs` with tests that:
- Verify RE(t) = RE(t-1) + NI(t) - Dividends(t) passes when correct
- Verify it fails when retained earnings don't flow through
- Verify first period is skipped (no prior RE to compare)
- Verify optional dividends_node and other_adjustments work

Follow the same test structure as `balance_sheet_tests.rs`: build a model, evaluate, construct check, run, assert findings.

- [ ] **Step 2: Write failing tests for CashReconciliation**

Create `finstack/statements/tests/checks/cash_reconciliation_tests.rs` with tests that:
- Verify Cash(t) = Cash(t-1) + TotalCF(t) passes when correct
- Verify it fails when cash doesn't reconcile
- Verify optional CFO + CFI + CFF sub-check works

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p finstack-statements --test checks_all -- retained_earnings cash_reconciliation`
Expected: Compilation errors.

- [ ] **Step 4: Implement RetainedEarningsReconciliation**

Create `finstack/statements/src/checks/builtins/retained_earnings.rs` implementing the `Check` trait. Logic: iterate periods starting from index 1, get RE(t-1), NI(t), optional dividends and adjustments, compare expected RE(t) vs actual. Same pattern as `BalanceSheetArticulation` — use `sum_nodes` helper (extract to a shared utility in `builtins/mod.rs`).

- [ ] **Step 5: Implement CashReconciliation**

Create `finstack/statements/src/checks/builtins/cash_reconciliation.rs`. Two checks per period: (1) Cash(t) = Cash(t-1) + TotalCF(t), (2) if CFO/CFI/CFF provided, TotalCF = sum of components.

- [ ] **Step 6: Wire up in builtins/mod.rs and tests/checks/mod.rs**

- [ ] **Step 7: Run tests, clippy, commit**

Run: `cargo test -p finstack-statements --test checks_all && cargo clippy -p finstack-statements -- -D warnings`

```bash
git commit -m "feat(statements): add RetainedEarningsReconciliation and CashReconciliation checks"
```

---

## Task 4: Built-in Data Quality Checks

**Files:**
- Create: `finstack/statements/src/checks/builtins/missing_values.rs`
- Create: `finstack/statements/src/checks/builtins/sign_convention.rs`
- Create: `finstack/statements/src/checks/builtins/non_finite.rs`
- Create: `finstack/statements/tests/checks/data_quality_tests.rs`
- Modify: `finstack/statements/src/checks/builtins/mod.rs`
- Modify: `finstack/statements/tests/checks/mod.rs`

- [ ] **Step 1: Write failing tests for all three data quality checks**

Create `finstack/statements/tests/checks/data_quality_tests.rs` with tests for:
- `MissingValueCheck`: node present for all periods, node missing for one period, scope filtering (actuals vs forecast using `Period::is_actual`)
- `SignConventionCheck`: positive node stays positive (passes), positive node goes negative (warning)
- `NonFiniteCheck`: all finite (passes), NaN detected (error), check-all-nodes mode

- [ ] **Step 2: Run tests to verify they fail**

- [ ] **Step 3: Implement MissingValueCheck**

Uses `PeriodScope` and `Period::is_actual` to filter periods. For each required node and in-scope period, checks if the node has a value. Severity: `Error` for actuals, `Warning` for forecast.

- [ ] **Step 4: Implement SignConventionCheck**

For each node in `positive_nodes`, flags periods where value < 0. For `negative_nodes`, flags periods where value > 0. Severity: `Warning`.

- [ ] **Step 5: Implement NonFiniteCheck**

If `nodes` is empty, checks all nodes in results. Flags NaN or infinite values. Severity: `Error`.

- [ ] **Step 6: Wire up, test, clippy, commit**

```bash
git commit -m "feat(statements): add data quality checks (MissingValue, SignConvention, NonFinite)"
```

---

## Task 5: Evaluator Integration — Inline Checks

**Files:**
- Modify: `finstack/statements/src/evaluator/results.rs`
- Modify: `finstack/statements/src/evaluator/engine.rs`
- Create: `finstack/statements/tests/checks/runner_tests.rs`
- Modify: `finstack/statements/tests/checks/mod.rs`

- [ ] **Step 1: Write failing tests for inline check execution**

Create `finstack/statements/tests/checks/runner_tests.rs`:
- Build a model with an imbalanced balance sheet
- Construct an Evaluator with `with_checks(suite)`
- Call `evaluate()`
- Assert `result.check_report` is `Some` and contains the expected failure

- [ ] **Step 2: Run tests to verify they fail**

- [ ] **Step 3: Add `check_report` field to StatementResult**

In `finstack/statements/src/evaluator/results.rs`, add to `StatementResult`:

```rust
/// Check report from inline validation (None if no checks configured)
#[serde(default, skip_serializing_if = "Option::is_none")]
pub check_report: Option<crate::checks::CheckReport>,
```

- [ ] **Step 4: Add `with_checks` to Evaluator**

In `finstack/statements/src/evaluator/engine.rs`, add a field `check_suite: Option<CheckSuite>` to `Evaluator` and a method:

```rust
pub fn with_checks(mut self, suite: crate::checks::CheckSuite) -> Self {
    self.check_suite = Some(suite);
    self
}
```

At the end of `evaluate()`, after computing results, if `check_suite` is `Some`, run the suite and attach the report to the result.

- [ ] **Step 5: Run tests, clippy, commit**

```bash
git commit -m "feat(statements): integrate CheckSuite into Evaluator with inline execution"
```

---

## Task 6: Prelude and Public API Updates

**Files:**
- Modify: `finstack/statements/src/lib.rs`
- Modify: `finstack/statements/src/prelude.rs`

- [ ] **Step 1: Update prelude**

Add to `finstack/statements/src/prelude.rs`:

```rust
pub use crate::checks::{
    Check, CheckCategory, CheckConfig, CheckContext, CheckFinding, CheckReport, CheckResult,
    CheckRunner, CheckSuite, CheckSuiteBuilder, CheckSummary, Materiality, PeriodScope, Severity,
};
pub use crate::checks::builtins::{
    BalanceSheetArticulation, CashReconciliation, MissingValueCheck, NonFiniteCheck,
    RetainedEarningsReconciliation, SignConventionCheck,
};
```

- [ ] **Step 2: Run full test suite**

Run: `cargo test -p finstack-statements`
Expected: All tests pass (existing + new).

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(statements): expose check types in prelude and public API"
```

---

## Task 7: Domain Checks — Cross-Statement Reconciliation (Analytics)

**Files:**
- Create: `finstack/statements-analytics/src/analysis/checks/mod.rs`
- Create: `finstack/statements-analytics/src/analysis/checks/reconciliation/mod.rs`
- Create: `finstack/statements-analytics/src/analysis/checks/reconciliation/depreciation.rs`
- Create: `finstack/statements-analytics/src/analysis/checks/reconciliation/interest.rs`
- Create: `finstack/statements-analytics/src/analysis/checks/reconciliation/capex.rs`
- Create: `finstack/statements-analytics/src/analysis/checks/reconciliation/dividends.rs`
- Create: `finstack/statements-analytics/tests/checks/mod.rs`
- Create: `finstack/statements-analytics/tests/checks/reconciliation_tests.rs`
- Create: `finstack/statements-analytics/tests/checks_all.rs`
- Modify: `finstack/statements-analytics/src/analysis/mod.rs`

- [ ] **Step 1: Write failing tests for all 4 reconciliation checks**

Create `finstack/statements-analytics/tests/checks_all.rs`:

```rust
#[path = "checks/mod.rs"]
mod checks;
```

Create `finstack/statements-analytics/tests/checks/mod.rs`:

```rust
mod reconciliation_tests;
```

Create tests for:
- `DepreciationReconciliation`: PP&E roll-forward correct (passes), incorrect (warning)
- `InterestExpenseReconciliation`: interest matches debt schedule (passes), mismatch (warning)
- `CapexReconciliation`: capex ties to PP&E additions (passes), mismatch (warning)
- `DividendReconciliation`: dividends match across statements (passes), mismatch (warning)

Each test builds a model with the relevant nodes, evaluates, runs the check, asserts findings.

- [ ] **Step 2: Run tests to verify they fail**

- [ ] **Step 3: Create the checks module structure in analytics**

Wire up `pub mod checks;` in `finstack/statements-analytics/src/analysis/mod.rs`. Create `mod.rs` files for the checks module and reconciliation submodule.

- [ ] **Step 4: Implement all 4 reconciliation checks**

Each follows the same pattern as the structural checks: implement `Check` trait, iterate periods, compare expected vs actual, produce `CheckFinding` with `Severity::Warning` and materiality.

- [ ] **Step 5: Run tests, clippy, commit**

```bash
git commit -m "feat(statements-analytics): add cross-statement reconciliation checks"
```

---

## Task 8: Domain Checks — Internal Consistency (Analytics)

**Files:**
- Create: `finstack/statements-analytics/src/analysis/checks/consistency/mod.rs`
- Create: `finstack/statements-analytics/src/analysis/checks/consistency/growth_rate.rs`
- Create: `finstack/statements-analytics/src/analysis/checks/consistency/tax_rate.rs`
- Create: `finstack/statements-analytics/src/analysis/checks/consistency/working_capital.rs`
- Create: `finstack/statements-analytics/tests/checks/consistency_tests.rs`
- Modify: `finstack/statements-analytics/tests/checks/mod.rs`

- [ ] **Step 1: Write failing tests for all 3 consistency checks**

Tests for:
- `GrowthRateConsistency`: steady growth (passes), 60% spike (warning), scope filtering
- `EffectiveTaxRateCheck`: 25% rate (passes), 5% rate (info), negative pretax edge case
- `WorkingCapitalConsistency`: WC change matches BS delta (passes), mismatch (warning)

- [ ] **Step 2: Run tests to verify they fail**

- [ ] **Step 3: Implement all 3 consistency checks**

- [ ] **Step 4: Run tests, clippy, commit**

```bash
git commit -m "feat(statements-analytics): add internal consistency checks"
```

---

## Task 9: Domain Checks — Credit Reasonableness (Analytics)

**Files:**
- Create: `finstack/statements-analytics/src/analysis/checks/credit/mod.rs`
- Create: `finstack/statements-analytics/src/analysis/checks/credit/leverage.rs`
- Create: `finstack/statements-analytics/src/analysis/checks/credit/coverage.rs`
- Create: `finstack/statements-analytics/src/analysis/checks/credit/fcf_sign.rs`
- Create: `finstack/statements-analytics/src/analysis/checks/credit/trend.rs`
- Create: `finstack/statements-analytics/src/analysis/checks/credit/liquidity.rs`
- Create: `finstack/statements-analytics/tests/checks/credit_tests.rs`
- Modify: `finstack/statements-analytics/tests/checks/mod.rs`

- [ ] **Step 1: Write failing tests for all 5 credit checks**

Tests for:
- `LeverageRangeCheck`: 4x leverage (passes), 7x (warning), 11x (error)
- `CoverageFloorCheck`: 2.5x coverage (passes), 1.2x (warning), 0.8x (error)
- `FcfSignCheck`: all positive (passes), 2 consecutive negative (warning), 4 consecutive (error)
- `TrendCheck`: improving coverage (passes), 3 consecutive declines (finding at configured severity)
- `LiquidityRunwayCheck`: 12 months (passes), 5 months (warning), 2 months (error)

- [ ] **Step 2: Run tests to verify they fail**

- [ ] **Step 3: Implement all 5 credit checks**

`TrendDirection` enum lives in `trend.rs`. Each check follows the same `Check` trait pattern.

- [ ] **Step 4: Run tests, clippy, commit**

```bash
git commit -m "feat(statements-analytics): add credit reasonableness checks"
```

---

## Task 10: FormulaCheck DSL Adapter

**Files:**
- Create: `finstack/statements-analytics/src/analysis/checks/formula_check.rs`
- Create: `finstack/statements-analytics/tests/checks/formula_check_tests.rs`
- Modify: `finstack/statements-analytics/src/analysis/checks/mod.rs`
- Modify: `finstack/statements-analytics/tests/checks/mod.rs`

- [ ] **Step 1: Write failing tests**

Tests for:
- Simple boolean formula: `revenue > 0` passes when revenue is positive
- Comparison formula: `gross_profit / revenue >= 0.20` passes when margin is 30%
- Failing formula: `gross_profit / revenue >= 0.20` fails when margin is 10%
- Tolerance formula: `abs(total_assets - total_liabilities - total_equity) < 0.01`
- JSON deserialization of `FormulaCheck` from config

- [ ] **Step 2: Run tests to verify they fail**

- [ ] **Step 3: Implement FormulaCheck**

`FormulaCheck` struct with `Serialize`/`Deserialize`. Implements `Check` trait. For each period, uses `finstack_statements::dsl` to parse and evaluate the formula expression against node values from `StatementResult`. Convention: result == 0.0 means fail, non-zero means pass.

The implementation needs to:
1. Parse the formula once (cache the compiled expression)
2. For each period, build a lookup of node values
3. Evaluate the compiled expression
4. If result is 0.0, produce a `CheckFinding` with configured severity

- [ ] **Step 4: Run tests, clippy, commit**

```bash
git commit -m "feat(statements-analytics): add FormulaCheck DSL adapter for user-defined checks"
```

---

## Task 11: Pre-built Suites and Mappings

**Files:**
- Create: `finstack/statements-analytics/src/analysis/checks/mappings.rs`
- Create: `finstack/statements-analytics/src/analysis/checks/suites.rs`
- Create: `finstack/statements-analytics/tests/checks/suite_tests.rs`
- Modify: `finstack/statements-analytics/src/analysis/checks/mod.rs`
- Modify: `finstack/statements-analytics/tests/checks/mod.rs`

- [ ] **Step 1: Write failing tests for pre-built suites**

Tests for:
- `three_statement_checks`: provide full mapping → suite has expected number of checks, run against a balanced 3-statement model → all pass
- `three_statement_checks`: provide partial mapping (no PP&E) → depreciation check is excluded
- `credit_underwriting_checks`: 4x leverage model passes, 7x model triggers warning
- Suite composition: merge `three_statement_checks` + `credit_underwriting_checks` → combined check count
- `lbo_model_checks`: inherits from both suites, has tighter leverage range

- [ ] **Step 2: Run tests to verify they fail**

- [ ] **Step 3: Implement mappings**

Create `mappings.rs` with `ThreeStatementMapping` and `CreditMapping` structs (both `Serialize`/`Deserialize`).

- [ ] **Step 4: Implement suite factory functions**

Create `suites.rs` with `three_statement_checks()`, `credit_underwriting_checks()`, and `lbo_model_checks()`. Each inspects its mapping and conditionally adds checks using `CheckSuite::builder()`.

- [ ] **Step 5: Run tests, clippy, commit**

```bash
git commit -m "feat(statements-analytics): add pre-built check suites and mappings"
```

---

## Task 12: Corkscrew Adapter and Report Renderer

**Files:**
- Create: `finstack/statements-analytics/src/analysis/checks/corkscrew_adapter.rs`
- Create: `finstack/statements-analytics/src/analysis/checks/renderer.rs`
- Create: `finstack/statements-analytics/tests/checks/renderer_tests.rs`
- Modify: `finstack/statements-analytics/src/analysis/checks/mod.rs`
- Modify: `finstack/statements-analytics/tests/checks/mod.rs`

- [ ] **Step 1: Write failing tests for corkscrew adapter**

Test that `corkscrew_as_checks()` converts a `CorkscrewConfig` into a `Vec<Box<dyn Check>>` that produces expected findings when run.

- [ ] **Step 2: Write failing tests for renderer**

Test that `CheckReportRenderer::render_text()` produces expected output format for a report with errors, warnings, and infos.

- [ ] **Step 3: Implement corkscrew adapter**

`corkscrew_as_checks()` creates `BalanceSheetArticulation` checks from the account types in the corkscrew config, plus per-account roll-forward checks.

- [ ] **Step 4: Implement CheckReportRenderer**

`render_text()`: header with model info and summary, then sections for ERRORS, WARNINGS, INFO. Each finding shows severity badge, check name, period, message, materiality, and nodes.

`render_html()`: same structure wrapped in styled HTML tags for notebook display.

- [ ] **Step 5: Run tests, clippy, commit**

```bash
git commit -m "feat(statements-analytics): add corkscrew adapter and CheckReportRenderer"
```

---

## Task 13: Analytics Prelude and Re-exports

**Files:**
- Modify: `finstack/statements-analytics/src/analysis/mod.rs`
- Modify: `finstack/statements-analytics/src/analysis/checks/mod.rs`
- Modify: `finstack/statements-analytics/src/prelude.rs`

- [ ] **Step 1: Wire up all re-exports in checks/mod.rs**

Add all check types, suite factories, mappings, `FormulaCheck`, renderer, and adapter to `analysis::checks` re-exports.

- [ ] **Step 2: Add check types to analysis/mod.rs re-exports**

Add `pub use checks::*` or selective re-exports for the key public types.

- [ ] **Step 3: Update prelude.rs**

Add key check types to the analytics prelude: suite factories, mappings, `FormulaCheck`, `CheckReportRenderer`.

- [ ] **Step 4: Run full test suite for both crates**

Run: `cargo test -p finstack-statements -p finstack-statements-analytics`
Expected: All tests pass.

- [ ] **Step 5: Run clippy on both crates**

Run: `cargo clippy -p finstack-statements -p finstack-statements-analytics -- -D warnings`

- [ ] **Step 6: Commit**

```bash
git commit -m "feat(statements-analytics): expose check types in prelude and public API"
```

---

## Task 14: Full Integration Test — 3-Statement Model Audit

**Files:**
- Create: `finstack/statements-analytics/tests/checks/integration_tests.rs`
- Modify: `finstack/statements-analytics/tests/checks/mod.rs`

- [ ] **Step 1: Write a comprehensive integration test**

Build a realistic 3-statement model with:
- Balance sheet: assets, liabilities, equity, cash, retained earnings, PP&E
- Income statement: revenue, COGS, gross profit, D&A, interest, pretax, tax, net income
- Cash flow: CFO, CFI (capex), CFF, total CF

Introduce one intentional error (balance sheet off by $5M) and one intentional warning (leverage at 7x).

Run `three_statement_checks` merged with `credit_underwriting_checks`.

Assert:
- Report has exactly 1 error (BS articulation)
- Report has exactly 1 warning (leverage)
- `report.findings_by_severity(Severity::Error)` returns 1 finding
- `report.material_findings(1_000_000.0)` returns the BS finding
- `CheckReportRenderer::render_text()` produces non-empty output

- [ ] **Step 2: Run the integration test**

Run: `cargo test -p finstack-statements-analytics --test checks_all -- integration`
Expected: Pass.

- [ ] **Step 3: Commit**

```bash
git commit -m "test(statements-analytics): add full 3-statement model audit integration test"
```

---

## Deferred Items

These spec items are intentionally deferred from this plan:

- **`CheckSuiteSpec` / `BuiltinCheckSpec`** — serializable suite descriptors for saving/loading check policies as JSON. Add after the core framework is validated.
- **Python/WASM bindings** — separate plan once the Rust API is stable.
- **Golden tests** — JSON snapshots of `CheckReport` for regression. Add after the report format stabilizes.
- **Property tests** — random node values to verify structural checks detect violations. Add as hardening pass.

---

## Verification

After all tasks are complete:

- [ ] `cargo test -p finstack-statements` — all pass
- [ ] `cargo test -p finstack-statements-analytics` — all pass
- [ ] `cargo clippy -p finstack-statements -p finstack-statements-analytics -- -D warnings` — clean
- [ ] `cargo doc -p finstack-statements -p finstack-statements-analytics --no-deps` — no warnings
