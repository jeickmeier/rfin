# Financial Statement Checks & Reconciliation Framework

**Date:** 2026-04-12
**Status:** Design approved, pending implementation plan

## Overview

A customizable, composable framework for validating financial statement models — from structural accounting identities to credit-specific reasonableness checks. Designed for credit analysts who need to ensure model quality and surface issues efficiently during underwriting.

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Execution mode | Both inline (on evaluation) and on-demand (standalone audit) | Maximum flexibility for different workflows |
| Check categories | All 5: accounting identity, cross-statement reconciliation, internal consistency, credit reasonableness, data quality | Comprehensive coverage from model mechanics to credit judgment |
| Failure reporting | Three-tier severity (Error/Warning/Info) with materiality amounts | Lets analysts filter by what actually matters for their credit view |
| Architecture | Split: structural checks in `finstack-statements`, domain checks in `finstack-statements-analytics` | Structural checks are about model correctness (close to evaluator), domain checks require credit judgment and configurable thresholds |
| Extensibility | Hybrid: `Check` trait for built-in checks, `FormulaCheck` DSL adapter for user-defined checks | Rust expressiveness for complex logic, DSL for Python/WASM users |
| Composability | Named check suites as templates, composable via merge, plus ad-hoc inline checks | Reusable patterns (3-statement, credit underwriting) with per-issuer overrides |

## Core Types (`finstack-statements::checks`)

### Severity

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}
```

- **Error**: Hard failure — likely a model construction bug (e.g., balance sheet doesn't balance).
- **Warning**: Something looks wrong but could be intentional (e.g., leverage > 8x).
- **Info**: Advisory (e.g., "coverage ratio trending down over 3 periods").

Ord is intentional: `Info < Warning < Error` enables filtering by `min_severity`.

### CheckCategory

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckCategory {
    AccountingIdentity,
    CrossStatementReconciliation,
    InternalConsistency,
    CreditReasonableness,
    DataQuality,
}
```

### Materiality

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Materiality {
    /// Absolute amount of the discrepancy (e.g., $2.3M)
    pub absolute: f64,
    /// Discrepancy as a percentage of the reference value (e.g., 0.0004 = 0.04%)
    pub relative_pct: f64,
    /// The denominator used for the relative calculation (e.g., total assets = $5.8B)
    pub reference_value: f64,
    /// Human-readable label for the reference (e.g., "total_assets")
    pub reference_label: String,
}
```

### CheckFinding

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckFinding {
    /// Which check produced this finding
    pub check_id: String,
    /// Severity level
    pub severity: Severity,
    /// Human-readable description
    pub message: String,
    /// Which period this applies to (None = model-level / all periods)
    pub period: Option<PeriodId>,
    /// Materiality context (None for non-quantifiable findings)
    pub materiality: Option<Materiality>,
    /// Which nodes are involved
    pub nodes: Vec<NodeId>,
}
```

### CheckResult

```rust
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
```

### Check Trait

```rust
pub trait Check: Send + Sync {
    /// Unique identifier (e.g., "balance_sheet_articulation")
    fn id(&self) -> &str;
    /// Human-readable name (e.g., "Balance Sheet Articulation")
    fn name(&self) -> &str;
    /// Which category this check belongs to
    fn category(&self) -> CheckCategory;
    /// Execute the check against evaluated results
    fn execute(&self, context: &CheckContext) -> Result<CheckResult>;
}
```

### CheckContext

```rust
pub struct CheckContext<'a> {
    /// The financial model spec
    pub model: &'a FinancialModelSpec,
    /// Evaluated results
    pub results: &'a StatementResult,
    /// Global check configuration
    pub config: CheckConfig,
}
```

### CheckConfig

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckConfig {
    /// Default absolute tolerance for equality checks (default: 0.01)
    #[serde(default = "default_check_tolerance")]
    pub default_tolerance: f64,
    /// Ignore findings with materiality below this absolute amount (default: 0.0)
    #[serde(default)]
    pub materiality_threshold: f64,
    /// Only report findings at or above this severity (default: Info)
    #[serde(default)]
    pub min_severity: Severity,
}
```

### CheckRunner

```rust
pub struct CheckRunner {
    checks: Vec<Box<dyn Check>>,
}

impl CheckRunner {
    pub fn new() -> Self;
    pub fn add_check(&mut self, check: impl Check + 'static) -> &mut Self;
    pub fn run(&self, context: &CheckContext) -> CheckReport;
}
```

### CheckReport

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckReport {
    /// Individual check results
    pub results: Vec<CheckResult>,
    /// Aggregated summary
    pub summary: CheckSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckSummary {
    pub total_checks: usize,
    pub passed: usize,
    pub failed: usize,
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
}

impl CheckReport {
    pub fn findings_by_severity(&self, severity: Severity) -> Vec<&CheckFinding>;
    pub fn findings_by_category(&self, category: CheckCategory) -> Vec<&CheckFinding>;
    pub fn findings_by_period(&self, period: &PeriodId) -> Vec<&CheckFinding>;
    pub fn findings_by_node(&self, node: &NodeId) -> Vec<&CheckFinding>;
    pub fn has_errors(&self) -> bool;
    pub fn has_warnings(&self) -> bool;
    pub fn material_findings(&self, threshold: f64) -> Vec<&CheckFinding>;
}
```

## Built-in Structural Checks (`finstack-statements::checks::builtins`)

These check model mechanics — they belong close to the evaluator.

### Accounting Identity Checks

#### BalanceSheetArticulation

Verifies Assets = Liabilities + Equity per period.

```rust
pub struct BalanceSheetArticulation {
    pub assets_nodes: Vec<NodeId>,
    pub liabilities_nodes: Vec<NodeId>,
    pub equity_nodes: Vec<NodeId>,
    pub tolerance: Option<f64>,
}
```

- Runs per period. Severity: `Error`. Materiality: imbalance vs total assets.

#### RetainedEarningsReconciliation

Verifies RE(t) = RE(t-1) + NI(t) - Dividends(t) ± Adjustments(t).

```rust
pub struct RetainedEarningsReconciliation {
    pub retained_earnings_node: NodeId,
    pub net_income_node: NodeId,
    pub dividends_node: Option<NodeId>,
    pub other_adjustments: Vec<NodeId>,
    pub tolerance: Option<f64>,
}
```

- Runs per period (skips first). Severity: `Error`.

#### CashReconciliation

Verifies Cash(t) = Cash(t-1) + TotalCF(t). Optionally checks TotalCF = CFO + CFI + CFF.

```rust
pub struct CashReconciliation {
    pub cash_balance_node: NodeId,
    pub total_cash_flow_node: NodeId,
    pub cfo_node: Option<NodeId>,
    pub cfi_node: Option<NodeId>,
    pub cff_node: Option<NodeId>,
    pub tolerance: Option<f64>,
}
```

- Runs per period. Severity: `Error`.

### Data Quality Checks

#### MissingValueCheck

Verifies that critical nodes have values for required periods.

```rust
pub struct MissingValueCheck {
    pub required_nodes: Vec<NodeId>,
    pub scope: PeriodScope,
}
```

`PeriodScope` lives in `checks::types` (shared across both crates):

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeriodScope {
    AllPeriods,
    ActualsOnly,
    ForecastOnly,
}
```

The actuals/forecast boundary is determined by the model's `actuals_until` field on `FinancialModelSpec` (set during `ModelBuilder::periods()`). Periods at or before `actuals_until` are actuals; periods after are forecast.

- Severity: `Error` for actuals, `Warning` for forecast.

#### SignConventionCheck

Flags unexpected sign flips in nodes with expected polarity.

```rust
pub struct SignConventionCheck {
    pub positive_nodes: Vec<NodeId>,
    pub negative_nodes: Vec<NodeId>,
}
```

- Severity: `Warning`.

#### NonFiniteCheck

Flags NaN or infinity values in results.

```rust
pub struct NonFiniteCheck {
    pub nodes: Vec<NodeId>, // empty = check all nodes
}
```

- Severity: `Error`.

## Domain Checks (`finstack-statements-analytics::checks`)

These require credit judgment and configurable thresholds.

### Cross-Statement Reconciliation

#### DepreciationReconciliation

Verifies PP&E(t) = PP&E(t-1) + Capex(t) - D&A(t) - Disposals(t).

```rust
pub struct DepreciationReconciliation {
    pub depreciation_expense_node: NodeId,
    pub ppe_node: NodeId,
    pub capex_node: NodeId,
    pub disposals_node: Option<NodeId>,
    pub tolerance: Option<f64>,
}
```

- Severity: `Warning` (reclassifications and impairments may cause legitimate differences).

#### InterestExpenseReconciliation

Cross-references interest expense with debt schedule or capital structure cashflows.

```rust
pub struct InterestExpenseReconciliation {
    pub interest_expense_node: NodeId,
    pub debt_balance_nodes: Vec<(NodeId, Option<NodeId>)>,
    pub cs_interest_node: Option<NodeId>,
    pub tolerance_pct: Option<f64>,
}
```

- Severity: `Warning`.

#### CapexReconciliation

Verifies cash flow capex ties to PP&E and intangible additions.

```rust
pub struct CapexReconciliation {
    pub capex_cf_node: NodeId,
    pub ppe_additions_node: Option<NodeId>,
    pub intangible_additions_node: Option<NodeId>,
    pub tolerance: Option<f64>,
}
```

- Severity: `Warning`.

#### DividendReconciliation

Verifies dividends on cash flow statement equal dividends in equity roll-forward.

```rust
pub struct DividendReconciliation {
    pub dividends_cf_node: NodeId,
    pub dividends_equity_node: NodeId,
    pub tolerance: Option<f64>,
}
```

- Severity: `Warning`.

### Internal Consistency

#### GrowthRateConsistency

Flags implausible period-over-period jumps in key metrics.

```rust
pub struct GrowthRateConsistency {
    pub nodes: Vec<NodeId>,
    pub max_period_growth_pct: f64,
    pub max_decline_pct: f64,
    pub scope: PeriodScope,
}
```

- Severity: `Warning`. Materiality: absolute change amount.

#### EffectiveTaxRateCheck

Flags periods where effective tax rate falls outside expected range.

```rust
pub struct EffectiveTaxRateCheck {
    pub tax_expense_node: NodeId,
    pub pretax_income_node: NodeId,
    pub expected_range: (f64, f64),
}
```

- Severity: `Info` (many legitimate reasons for unusual tax rates).

#### WorkingCapitalConsistency

Verifies working capital changes on cash flow statement match balance sheet deltas.

```rust
pub struct WorkingCapitalConsistency {
    pub wc_change_cf_node: NodeId,
    pub current_assets_nodes: Vec<NodeId>,
    pub current_liabilities_nodes: Vec<NodeId>,
    pub tolerance: Option<f64>,
}
```

- Severity: `Warning`.

### Credit Reasonableness

#### LeverageRangeCheck

Flags leverage ratios outside expected bounds.

```rust
pub struct LeverageRangeCheck {
    pub debt_node: NodeId,
    pub ebitda_node: NodeId,
    pub warn_range: (f64, f64),
    pub error_range: (f64, f64),
}
```

- Per-period. Severity: `Warning` or `Error` depending on which range is breached.

#### CoverageFloorCheck

Flags coverage ratios below minimum thresholds.

```rust
pub struct CoverageFloorCheck {
    pub numerator_node: NodeId,
    pub denominator_node: NodeId,
    pub min_warning: f64,
    pub min_error: f64,
}
```

- Severity scales with how far below the floor.

#### FcfSignCheck

Flags periods where free cash flow is negative, with escalation for consecutive periods.

```rust
pub struct FcfSignCheck {
    pub fcf_node: NodeId,
    pub consecutive_negative_warning: usize,
    pub consecutive_negative_error: usize,
}
```

#### TrendCheck

Flags deteriorating trends in key credit metrics.

```rust
pub struct TrendCheck {
    pub node: NodeId,
    pub direction: TrendDirection,
    pub lookback_periods: usize,
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrendDirection {
    IncreasingIsGood,
    DecreasingIsGood,
}
```

#### LiquidityRunwayCheck

Estimates months of cash remaining and flags low runway.

```rust
pub struct LiquidityRunwayCheck {
    pub cash_node: NodeId,
    pub cash_burn_node: NodeId,
    pub min_months_warning: f64,
    pub min_months_error: f64,
}
```

## Check Suites (`finstack-statements-analytics::checks::suites`)

### CheckSuite

```rust
pub struct CheckSuite {
    pub name: String,
    pub description: String,
    checks: Vec<Box<dyn Check>>,
    pub config: CheckConfig,
}

impl CheckSuite {
    pub fn builder(name: &str) -> CheckSuiteBuilder;
    pub fn merge(self, other: CheckSuite) -> Self;
    pub fn run(&self, model: &FinancialModelSpec, results: &StatementResult) -> CheckReport;
}
```

`CheckSuite` itself is **not** `Serialize`/`Deserialize` because it contains trait objects. The output `CheckReport` is fully serializable. For suite definitions that need to be saved/loaded as JSON (e.g., team-wide check policies), use `CheckSuiteSpec` — a serializable descriptor that resolves to a `CheckSuite` at runtime:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckSuiteSpec {
    pub name: String,
    pub description: String,
    pub builtin_checks: Vec<BuiltinCheckSpec>,
    pub formula_checks: Vec<FormulaCheck>,
    pub config: CheckConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BuiltinCheckSpec {
    BalanceSheetArticulation { assets_nodes: Vec<NodeId>, liabilities_nodes: Vec<NodeId>, equity_nodes: Vec<NodeId>, tolerance: Option<f64> },
    CashReconciliation { cash_balance_node: NodeId, total_cash_flow_node: NodeId, /* ... */ },
    LeverageRange { debt_node: NodeId, ebitda_node: NodeId, warn_range: (f64, f64), error_range: (f64, f64) },
    // ... one variant per built-in check
}

impl CheckSuiteSpec {
    pub fn resolve(&self) -> Result<CheckSuite>;
}
```
```

### CheckSuiteBuilder

```rust
pub struct CheckSuiteBuilder { /* ... */ }

impl CheckSuiteBuilder {
    pub fn description(self, desc: &str) -> Self;
    pub fn tolerance(self, tolerance: f64) -> Self;
    pub fn materiality_threshold(self, threshold: f64) -> Self;
    pub fn min_severity(self, severity: Severity) -> Self;
    pub fn add_check(self, check: impl Check + 'static) -> Self;
    pub fn build(self) -> CheckSuite;
}
```

### Pre-built Suite: `three_statement_checks`

```rust
pub fn three_statement_checks(mapping: ThreeStatementMapping) -> CheckSuite
```

```rust
pub struct ThreeStatementMapping {
    pub assets_nodes: Vec<NodeId>,
    pub liabilities_nodes: Vec<NodeId>,
    pub equity_nodes: Vec<NodeId>,
    pub cash_node: NodeId,
    pub retained_earnings_node: NodeId,
    pub ppe_node: Option<NodeId>,
    pub net_income_node: NodeId,
    pub depreciation_node: Option<NodeId>,
    pub interest_expense_node: Option<NodeId>,
    pub tax_expense_node: Option<NodeId>,
    pub pretax_income_node: Option<NodeId>,
    pub cfo_node: Option<NodeId>,
    pub cfi_node: Option<NodeId>,
    pub cff_node: Option<NodeId>,
    pub total_cf_node: Option<NodeId>,
    pub capex_node: Option<NodeId>,
    pub dividends_node: Option<NodeId>,
}
```

Automatically wires up all applicable checks based on which `Option` fields are `Some`. Includes: `BalanceSheetArticulation`, `RetainedEarningsReconciliation`, `CashReconciliation`, `DepreciationReconciliation`, `InterestExpenseReconciliation`, `WorkingCapitalConsistency`, `NonFiniteCheck`, `MissingValueCheck`.

### Pre-built Suite: `credit_underwriting_checks`

```rust
pub fn credit_underwriting_checks(mapping: CreditMapping) -> CheckSuite
```

```rust
pub struct CreditMapping {
    pub debt_node: NodeId,
    pub ebitda_node: NodeId,
    pub interest_expense_node: NodeId,
    pub fcf_node: Option<NodeId>,
    pub cash_node: Option<NodeId>,
    pub cash_burn_node: Option<NodeId>,
    pub leverage_warn: Option<(f64, f64)>,
    pub coverage_min_warn: Option<f64>,
}
```

Includes: `LeverageRangeCheck`, `CoverageFloorCheck`, `FcfSignCheck`, `TrendCheck` (on leverage + coverage), `LiquidityRunwayCheck`.

### Pre-built Suite: `lbo_model_checks`

```rust
pub fn lbo_model_checks(mapping: ThreeStatementMapping, credit: CreditMapping) -> CheckSuite
```

Merges `three_statement_checks` with `credit_underwriting_checks`, overrides leverage warning range to `(0.0, 8.0)`, adds `GrowthRateConsistency` on revenue and EBITDA.

### Composition

```rust
let suite = three_statement_checks(my_mapping)
    .merge(credit_underwriting_checks(my_credit_mapping))
    .merge(CheckSuite::builder("custom")
        .add_check(EffectiveTaxRateCheck {
            tax_expense_node: "tax_expense".into(),
            pretax_income_node: "pretax_income".into(),
            expected_range: (0.20, 0.30),
        })
        .build());
```

## DSL-Based Custom Checks (`FormulaCheck`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormulaCheck {
    pub id: String,
    pub name: String,
    pub category: CheckCategory,
    pub severity: Severity,
    /// DSL expression that should evaluate to a boolean (true = pass)
    pub formula: String,
    /// Template for the failure message (supports {actual}, {diff} placeholders)
    pub message_template: String,
    pub tolerance: Option<f64>,
}
```

`FormulaCheck` implements the `Check` trait. The formula is evaluated using the existing DSL engine per period. Since the DSL produces `f64` values (not booleans), the convention is: the formula evaluates to a numeric value where **non-zero = pass, zero = fail**. Comparison operators (`>=`, `<`, `==`, etc.) produce `1.0` (true) or `0.0` (false) in the DSL, so expressions like `revenue > 0` work naturally. For tolerance-based equality checks, use `abs(lhs - rhs) < tolerance`.

Examples:

```json
{
  "id": "custom_margin_floor",
  "name": "Gross margin above 20%",
  "category": "credit_reasonableness",
  "severity": "warning",
  "formula": "gross_profit / revenue >= 0.20",
  "message_template": "Gross margin {actual:.1%} is below 20% floor"
}
```

## Integration Points

### Inline Mode — Evaluator Integration

`StatementResult` gains an optional `check_report` field:

```rust
pub struct StatementResult {
    // ... existing fields ...
    pub check_report: Option<CheckReport>,
}
```

`Evaluator` gains a `with_checks` method:

```rust
impl Evaluator {
    pub fn with_checks(self, suite: CheckSuite) -> Self;
}
```

When checks are attached, `evaluate()` runs the DAG evaluation first, then runs the check suite. Checks never block evaluation — they are advisory. Analysts inspect `result.check_report` to decide how to proceed.

### On-Demand Mode

```rust
let results = evaluator.evaluate(&model)?;
let report = suite.run(&model, &results);
let errors = report.findings_by_severity(Severity::Error);
```

### Corkscrew Migration

- `CorkscrewExtension` stays as-is for roll-forward schedule computation.
- `check_articulation` is deprecated in favor of `BalanceSheetArticulation`.
- An adapter function converts corkscrew roll-forward validations into `CheckFinding`s:

```rust
pub fn corkscrew_as_checks(config: &CorkscrewConfig) -> Vec<Box<dyn Check>>
```

### Reporting

`CheckReportRenderer` provides text and HTML rendering:

```rust
pub struct CheckReportRenderer;

impl CheckReportRenderer {
    pub fn render_text(report: &CheckReport) -> String;
    pub fn render_html(report: &CheckReport) -> String;
}
```

### Python/WASM Bindings

All core types (`CheckSuite`, `CheckReport`, `CheckFinding`, `Severity`, `Materiality`, built-in checks, suite factories, `FormulaCheck`) are exposed through both binding layers following existing Rust-canonical naming conventions.

## Module Layout

### `finstack-statements`

```
src/checks/
├── mod.rs           # pub types, trait, runner
├── types.rs         # Severity, CheckCategory, Materiality, CheckFinding, CheckResult, CheckReport, CheckConfig, CheckSummary, PeriodScope
├── traits.rs        # Check trait, CheckContext
├── runner.rs        # CheckRunner
├── suite.rs         # CheckSuite, CheckSuiteBuilder, CheckSuiteSpec, BuiltinCheckSpec
└── builtins/
    ├── mod.rs
    ├── balance_sheet.rs      # BalanceSheetArticulation
    ├── retained_earnings.rs  # RetainedEarningsReconciliation
    ├── cash_reconciliation.rs # CashReconciliation
    ├── missing_values.rs     # MissingValueCheck
    ├── sign_convention.rs    # SignConventionCheck
    └── non_finite.rs         # NonFiniteCheck
```

### `finstack-statements-analytics`

```
src/analysis/checks/
├── mod.rs
├── reconciliation/
│   ├── mod.rs
│   ├── depreciation.rs
│   ├── interest.rs
│   ├── capex.rs
│   └── dividends.rs
├── consistency/
│   ├── mod.rs
│   ├── growth_rate.rs
│   ├── tax_rate.rs
│   └── working_capital.rs
├── credit/
│   ├── mod.rs
│   ├── leverage.rs
│   ├── coverage.rs
│   ├── fcf_sign.rs
│   ├── trend.rs
│   └── liquidity.rs
├── formula_check.rs    # FormulaCheck DSL adapter
├── suites.rs           # three_statement_checks, credit_underwriting_checks, lbo_model_checks
├── mappings.rs         # ThreeStatementMapping, CreditMapping
├── corkscrew_adapter.rs # corkscrew_as_checks
└── renderer.rs         # CheckReportRenderer
```

## Testing Strategy

- **Unit tests** per check: known-good and known-bad models, tolerance edge cases.
- **Suite integration tests**: build a 3-statement model, run suite, assert correct findings.
- **DSL check tests**: formula parsing + evaluation against mock results.
- **Golden tests**: JSON snapshots of `CheckReport` for regression.
- **Property tests**: random node values, verify structural checks detect violations.
- **Binding parity**: Python and WASM produce identical `CheckReport` JSON for same model.
