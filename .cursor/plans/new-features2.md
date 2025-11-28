I'll analyze the statements crate to identify impactful new functionality for quants and credit analysts. Let me first explore the existing implementation thoroughly.

[28 tools called]

Let me also check some key areas to identify gaps:

[5 tools called]

Based on my thorough analysis of the statements crate, I'll now provide the comprehensive forward-looking code review identifying new functionality opportunities.

---

# Forward-Looking Code Review: New Functionality for Statements Crate

## 1) Executive Summary

### Most Impactful Missing Capabilities

| **Feature** | **Impact** | **Effort** | **Confidence** |
|-------------|------------|------------|----------------|
| 1. Variance Analysis Engine (Actual vs Budget/Forecast) | **P0** | M | High |
| 2. Multi-Scenario Management & Comparison | **P0** | M | High |
| 3. Working Capital Analytics (DSO/DIO/DPO) | **P0** | S | High |
| 4. Period Aggregation DSL (QTD/YTD/LTM) | **P1** | S | High |
| 5. Cash Flow Statement Derivation (Indirect Method) | **P1** | M | Med |
| 6. Monte Carlo Simulation for Forecasts | **P1** | L | Med |
| 7. Model Audit Trail & Change Tracking | **P2** | M | High |
| 8. SaaS/Cohort LTV Metrics Templates | **P2** | S | Med |

### Key Pain Points Addressed

- **Credit Analysts:** Cannot easily compare management case vs bank case; no systematic variance tracking; working capital ratios require manual setup
- **Quants:** No stochastic forecast outputs; limited scenario branching; no formal model version diffing
- **Portfolio/Risk:** Multi-entity consolidation missing; no systematic quality checks beyond corkscrew

---

## 2) High-Impact Feature Proposals

### Feature 1: Variance Analysis Engine

**Title:** Structured Actual vs. Budget/Forecast Variance Analysis

**Persona Pain:** Credit analysts building IC memos need to compare management projections against bank base case and show period-over-period variance. Today this requires manual DataFrame manipulation outside the model.

**User Story:** *"As a credit analyst, I need to compute variance ($ and %) between actuals and multiple forecasts so that I can track forecast accuracy and identify drivers of deviation in my quarterly monitoring package."*

**Scope:**

**Data:**
```json
{
  "variance_config": {
    "baseline": "management_case",
    "comparisons": ["bank_case", "actuals"],
    "metrics": ["revenue", "ebitda", "free_cash_flow"],
    "periods": ["2025Q1", "2025Q2"]
  }
}
```

Output DataFrame:
| period | metric | baseline | comparison | abs_var | pct_var | driver_contribution |
|--------|--------|----------|------------|---------|---------|---------------------|
| 2025Q1 | revenue | 100M | 95M | -5M | -5.0% | volume: -3M, price: -2M |

**APIs:**

**Rust:**
```rust
pub struct VarianceAnalyzer<'a> {
    baseline: &'a Results,
    comparison: &'a Results,
}

impl<'a> VarianceAnalyzer<'a> {
    pub fn compute(&self, config: &VarianceConfig) -> Result<VarianceReport>;
    pub fn bridge_decomposition(&self, target: &str, drivers: &[&str]) -> Result<BridgeChart>;
    pub fn to_polars(&self) -> Result<DataFrame>;
}
```

**Python:**
```python
from finstack.statements import VarianceAnalyzer, Results

analyzer = VarianceAnalyzer(baseline=mgmt_results, comparison=bank_results)
variance_df = analyzer.compute(metrics=["revenue", "ebitda"]).to_pandas()
bridge = analyzer.bridge("ebitda", drivers=["revenue", "cogs", "opex"])
```

**Explainability:**
- `explain()` returns structured bridge showing contribution of each driver node to the total variance
- Supports nested decomposition (revenue → volume × price)

**Validation:**
- Property test: baseline vs baseline = zero variance
- Golden tests against Excel variance formulas

**Impact & Effort:** P0 / M / High confidence
**Dependencies:** Results, DSL compiler
**Risks:** Driver attribution requires additive decomposition; multiplicative drivers need special handling

**Demo Outline:**
1. Load two model Results (Management vs Bank)
2. Run `VarianceAnalyzer.compute()`
3. Generate EBITDA bridge chart
4. Export to Excel-compatible format

---

### Feature 2: Multi-Scenario Management & Comparison

**Title:** Named Scenario Registry with Diff/Merge Operations

**Persona Pain:** Quants building stress tests need to maintain base case + N scenarios and systematically compare outputs. Current approach requires cloning models and manual tracking.

**User Story:** *"As a quant, I need to define named scenarios (base, upside, downside, stress) with explicit parameter overrides so that I can generate comparative output tables and tornado charts across all scenarios."*

**Scope:**

**Data:**
```json
{
  "scenario_set": {
    "base": { "model_id": "acme-2025", "overrides": {} },
    "downside": {
      "parent": "base",
      "overrides": { "revenue_growth": -0.05, "margin": -0.02 }
    },
    "stress": {
      "parent": "downside", 
      "overrides": { "revenue_growth": -0.15 }
    }
  }
}
```

**APIs:**

**Rust:**
```rust
pub struct ScenarioSet {
    scenarios: IndexMap<String, ScenarioDefinition>,
}

impl ScenarioSet {
    pub fn evaluate_all(&self, base_model: &FinancialModelSpec) -> Result<ScenarioResults>;
    pub fn diff(&self, a: &str, b: &str) -> Result<ScenarioDiff>;
    pub fn to_comparison_df(&self, metrics: &[&str]) -> Result<DataFrame>;
}
```

**Python:**
```python
scenarios = ScenarioSet.from_json("scenarios.json")
results = scenarios.evaluate_all(base_model)

# Comparison table
df = results.to_comparison_df(metrics=["ebitda", "leverage"])
# Output: period | metric | base | downside | stress | stress_vs_base_pct
```

**Explainability:**
- `diff()` shows which overrides drove the delta
- `trace()` shows full lineage from parent scenario

**Impact & Effort:** P0 / M / High confidence
**Dependencies:** Existing `SensitivityAnalyzer`, scenarios crate integration
**Risks:** Cache invalidation with nested scenario overrides

---

### Feature 3: Working Capital Analytics

**Title:** DSO/DIO/DPO/CCC Built-in Formulas and Helpers

**Persona Pain:** Every credit model needs Days Sales Outstanding, Days Inventory Outstanding, Days Payables Outstanding. Analysts manually write the same formulas repeatedly.

**User Story:** *"As a credit analyst, I need pre-built working capital ratio calculations so that I can quickly assess liquidity and cash conversion cycle trends."*

**Scope:**

**DSL Functions:**
```
dso(accounts_receivable, revenue)  // AR / (Revenue / 365)
dio(inventory, cogs)                // Inventory / (COGS / 365)  
dpo(accounts_payable, cogs)         // AP / (COGS / 365)
ccc(dso, dio, dpo)                  // DSO + DIO - DPO
```

**APIs:**

**Rust:**
```rust
// Added to DSL parser
pub fn parse_working_capital_function(name: &str, args: &[StmtExpr]) -> Result<StmtExpr>;

// Registry built-ins
pub struct WorkingCapitalMetrics;
impl WorkingCapitalMetrics {
    pub fn dso(ar: f64, revenue: f64, days: f64) -> f64;
    pub fn cash_conversion_cycle(dso: f64, dio: f64, dpo: f64) -> f64;
}
```

**Python:**
```python
model = ModelBuilder("acme") \
    .compute("dso", "dso(accounts_receivable, revenue)") \
    .compute("dio", "dio(inventory, cogs)") \
    .compute("ccc", "ccc(dso, dio, dpo)") \
    .build()
```

**Impact & Effort:** P0 / S / High confidence
**Dependencies:** DSL parser extension
**Risks:** Annualization assumptions (365 vs 360); need configurable day count

---

### Feature 4: Period Aggregation DSL Functions

**Title:** QTD/YTD/LTM/TTM Flexible Aggregation

**Persona Pain:** `ttm()` exists but credit analysts need YTD, QTD (for quarterly models), and custom rolling windows with fiscal year alignment.

**User Story:** *"As a credit analyst, I need to compute YTD revenue through any quarter so that I can compare against annual budget allocations."*

**Scope:**

**DSL Functions:**
```
ytd(revenue)              // Year-to-date sum
qtd(revenue)              // Quarter-to-date (for monthly models)
ltm(revenue)              // Last twelve months (alias for ttm)
rolling_sum(revenue, 12)  // Exists - but add fiscal_ytd()
fiscal_ytd(revenue, fiscal_start_month=4)  // April fiscal year
```

**APIs:**

**Rust:**
```rust
// Extended DSL
StmtExpr::Call {
    func: "fiscal_ytd",
    args: vec![node_ref, fiscal_start_month],
}

// Evaluator support
fn evaluate_fiscal_ytd(
    node: &str, 
    current_period: &PeriodId,
    fiscal_start: u32,
    context: &EvaluationContext,
) -> Result<f64>;
```

**Python:**
```python
.compute("ytd_revenue", "ytd(revenue)")
.compute("fiscal_ytd_rev", "fiscal_ytd(revenue, 4)")  # April fiscal year
```

**Impact & Effort:** P1 / S / High confidence
**Dependencies:** DSL parser, evaluator historical context

---

### Feature 5: Cash Flow Statement Derivation (Indirect Method)

**Title:** Automated Indirect Cash Flow Statement Generation

**Persona Pain:** Credit analysts building 3-statement models manually wire up the indirect CF statement. This is error-prone and repetitive.

**User Story:** *"As a credit analyst, I need to automatically generate the operating section of an indirect cash flow statement from my P&L and balance sheet nodes so that I can ensure articulation without manual formula setup."*

**Scope:**

**Template:**
```rust
pub struct IndirectCFTemplate {
    net_income_node: String,
    depreciation_nodes: Vec<String>,
    working_capital_nodes: Vec<WorkingCapitalMapping>,
}

pub struct WorkingCapitalMapping {
    bs_node: String,      // e.g., "accounts_receivable"
    direction: WCDirection, // Increase = cash outflow
}
```

**APIs:**

**Rust:**
```rust
impl TemplatesExtension for ModelBuilder<Ready> {
    fn add_indirect_cf_operating(
        self,
        config: IndirectCFTemplate,
    ) -> Result<ModelBuilder<Ready>>;
}
```

**Python:**
```python
model = ModelBuilder("acme") \
    .add_indirect_cf_operating(
        net_income="net_income",
        add_back=["depreciation", "amortization"],
        working_capital={"accounts_receivable": "subtract", "accounts_payable": "add"}
    ) \
    .build()
# Auto-creates: cf_operating = net_income + depreciation - delta(ar) + delta(ap)
```

**Impact & Effort:** P1 / M / Med confidence
**Dependencies:** Roll-forward template, `diff()` DSL function
**Risks:** Requires BS roll-forward to be set up first

---

### Feature 6: Monte Carlo Simulation for Forecasts

**Title:** Stochastic Forecast Distributions with Percentile Outputs

**Persona Pain:** Normal/LogNormal forecasts produce single point estimates. Quants need P5/P50/P95 bands for risk quantification.

**User Story:** *"As a quant, I need to run 1000 Monte Carlo paths on my revenue forecast so that I can report a distribution of outcomes for covenant breach probability."*

**Scope:**

**APIs:**

**Rust:**
```rust
pub struct MonteCarloConfig {
    pub n_paths: usize,
    pub seed: u64,
    pub percentiles: Vec<f64>, // [0.05, 0.25, 0.50, 0.75, 0.95]
}

pub struct MonteCarloResults {
    pub percentile_results: IndexMap<String, PercentileSeries>, // P5, P50, P95 per node
    pub path_data: Option<DataFrame>, // Full paths if requested
}

impl Evaluator {
    pub fn evaluate_monte_carlo(
        &mut self,
        model: &FinancialModelSpec,
        config: &MonteCarloConfig,
    ) -> Result<MonteCarloResults>;
}
```

**Python:**
```python
mc_results = evaluator.evaluate_monte_carlo(model, n_paths=1000, seed=42)
p95_ebitda = mc_results.get_percentile("ebitda", 0.95)

# Covenant breach probability
breach_prob = mc_results.breach_probability("leverage", threshold=4.5)
```

**Impact & Effort:** P1 / L / Med confidence
**Dependencies:** Statistical forecast methods, Rayon for parallelism
**Risks:** Performance at scale; memory for storing paths

---

### Feature 7: Model Audit Trail & Change Tracking

**Title:** Immutable Model Versioning with Structured Diffs

**Persona Pain:** No way to track what changed between model versions for governance/compliance.

**User Story:** *"As a credit analyst, I need to track changes to my model over time so that I can document what assumptions changed between IC presentations."*

**Scope:**

**Data:**
```rust
pub struct ModelVersion {
    pub version_id: String,
    pub timestamp: DateTime<Utc>,
    pub author: Option<String>,
    pub message: Option<String>,
    pub parent_version: Option<String>,
    pub diff_from_parent: Option<ModelDiff>,
}

pub struct ModelDiff {
    pub added_nodes: Vec<String>,
    pub removed_nodes: Vec<String>,
    pub modified_nodes: IndexMap<String, NodeDiff>,
    pub period_changes: Option<PeriodDiff>,
}
```

**APIs:**

**Rust:**
```rust
impl FinancialModelSpec {
    pub fn diff(&self, other: &Self) -> ModelDiff;
    pub fn with_version(self, version: ModelVersion) -> VersionedModel;
}

pub struct VersionedModel {
    pub current: FinancialModelSpec,
    pub history: Vec<ModelVersion>,
}
```

**Python:**
```python
diff = model_v2.diff(model_v1)
print(diff.summary())
# Output: Added: [capex_forecast], Modified: [revenue.forecast.rate: 0.05 → 0.03]
```

**Impact & Effort:** P2 / M / High confidence

---

### Feature 8: SaaS/Cohort LTV Metrics

**Title:** Customer Lifetime Value and Cohort Templates

**Persona Pain:** SaaS-focused credit analysts need LTV, CAC, LTV/CAC ratios with cohort-based decay.

**User Story:** *"As an analyst covering software companies, I need to model cohort-based revenue retention and compute LTV/CAC ratios."*

**Scope:**

**Templates:**
```rust
pub fn add_cohort_ltv(
    builder: ModelBuilder<Ready>,
    new_customer_node: &str,
    arpu_node: &str,
    retention_curve: &[f64], // [1.0, 0.9, 0.85, 0.80, ...]
    discount_rate: f64,
) -> Result<ModelBuilder<Ready>>;
```

**DSL:**
```
ltv(arpu, retention_curve, discount_rate)
ltv_cac_ratio(ltv, cac)
payback_months(cac, monthly_contribution)
```

**Impact & Effort:** P2 / S / Med confidence

---

## 3) Quick Wins ("Fast-Follow")

1. **`full_grid` sensitivity mode** - Currently returns error; implement factorial parameter sweep
2. **`variance()` DSL function** - Compare two nodes: `variance(actual, budget)`
3. **Export to CSV helper** - `results.to_csv("output.csv")` wrapper around Polars
4. **Excel export preset** - Format numbers with accounting notation for copy-paste
5. **Model validation presets** - Built-in checks: "all nodes have values", "no circular refs"
6. **`growth_rate()` DSL** - CAGR between two periods: `growth_rate(revenue, periods=4)`
7. **`annualize()` for quarterly** - Auto-detect period kind and annualize appropriately
8. **Batch goal seek** - Solve multiple targets simultaneously
9. **Scorecard presets** - Built-in S&P/Moody's templates in registry JSON
10. **`abs()` and `sign()` DSL functions** - Currently missing basic math helpers
11. **Period alignment helpers** - `align_to_quarters()` for monthly data
12. **Error message node highlighting** - Include node_id in all eval errors

---

## 4) De-Dup Check

| Feature | Evidence of Absence |
|---------|---------------------|
| Variance Analysis | Searched `analysis/`, `evaluator/` - no `variance`, `compare`, `diff` for Results |
| Multi-Scenario Management | `scenarios/` crate handles market shocks, not statement model forks; no `ScenarioSet` |
| Working Capital DSL | Searched `dsl/parser.rs` - no `dso`, `dio`, `dpo`, `ccc` functions |
| Period Aggregation | `ttm()` exists; no `ytd()`, `qtd()`, `fiscal_ytd()` in parser |
| Indirect CF Template | `templates/` has roll_forward, vintage - no `indirect_cf` |
| Monte Carlo | Statistical forecasts are point estimates; no `MonteCarloConfig` or path storage |
| Model Versioning | `schema_version: u32` exists; no `ModelVersion`, `diff()`, audit trail |
| SaaS LTV | No `cohort`, `ltv`, `cac` in DSL or templates |

---

## 5) Appendix

### A. Variance Analysis API Sketch

```rust
// finstack/statements/src/analysis/variance.rs

use crate::evaluator::Results;
use crate::error::Result;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarianceConfig {
    pub metrics: Vec<String>,
    pub periods: Option<Vec<PeriodId>>,
    pub include_pct: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarianceRow {
    pub period: PeriodId,
    pub metric: String,
    pub baseline: f64,
    pub comparison: f64,
    pub abs_variance: f64,
    pub pct_variance: Option<f64>,
}

pub struct VarianceAnalyzer<'a> {
    baseline: &'a Results,
    comparison: &'a Results,
}

impl<'a> VarianceAnalyzer<'a> {
    pub fn new(baseline: &'a Results, comparison: &'a Results) -> Self {
        Self { baseline, comparison }
    }

    pub fn compute(&self, config: &VarianceConfig) -> Result<Vec<VarianceRow>> {
        let mut rows = Vec::new();
        for metric in &config.metrics {
            let base_values = self.baseline.get_node(metric);
            let comp_values = self.comparison.get_node(metric);
            // ... compute variances
        }
        Ok(rows)
    }

    #[cfg(feature = "dataframes")]
    pub fn to_polars(&self, config: &VarianceConfig) -> Result<DataFrame> {
        let rows = self.compute(config)?;
        // Convert to DataFrame
    }
}
```

### B. Working Capital DSL Extension

```rust
// finstack/statements/src/dsl/parser.rs additions

fn parse_working_capital_func(name: &str, args: Vec<StmtExpr>) -> Result<StmtExpr> {
    match name {
        "dso" => {
            // dso(ar, revenue) = ar / (revenue / 365)
            ensure!(args.len() == 2, "dso requires 2 arguments");
            Ok(StmtExpr::bin_op(
                BinOp::Div,
                args[0].clone(),
                StmtExpr::bin_op(BinOp::Div, args[1].clone(), StmtExpr::literal(365.0)),
            ))
        }
        "dio" => { /* inventory / (cogs / 365) */ }
        "dpo" => { /* ap / (cogs / 365) */ }
        "ccc" => {
            // ccc(dso, dio, dpo) = dso + dio - dpo
            ensure!(args.len() == 3, "ccc requires 3 arguments");
            Ok(StmtExpr::bin_op(
                BinOp::Sub,
                StmtExpr::bin_op(BinOp::Add, args[0].clone(), args[1].clone()),
                args[2].clone(),
            ))
        }
        _ => Err(Error::parse(format!("Unknown function: {}", name))),
    }
}
```

### C. Notebook Demo Outline: Variance Analysis

```python
# variance_analysis_demo.ipynb

# Cell 1: Setup
from finstack.statements import ModelBuilder, Evaluator, VarianceAnalyzer
import polars as pl

# Cell 2: Build Management Case
mgmt_model = ModelBuilder("acme-mgmt") \
    .periods("2025Q1..Q4", "2025Q2") \
    .value("revenue", [...]) \
    .forecast("revenue", growth=0.08) \
    .build()

mgmt_results = Evaluator().evaluate(mgmt_model)

# Cell 3: Build Bank Case (conservative)
bank_model = ModelBuilder("acme-bank") \
    .periods("2025Q1..Q4", "2025Q2") \
    .value("revenue", [...]) \
    .forecast("revenue", growth=0.03) \
    .build()

bank_results = Evaluator().evaluate(bank_model)

# Cell 4: Compute Variance
analyzer = VarianceAnalyzer(baseline=mgmt_results, comparison=bank_results)
variance_df = analyzer.to_polars(metrics=["revenue", "ebitda", "leverage"])
print(variance_df)

# Cell 5: Bridge Chart
bridge = analyzer.bridge("ebitda", drivers=["revenue", "cogs", "opex"])
bridge.plot()  # If viz support added
```

---

This analysis identifies the most impactful gaps in the statements crate that would reduce analyst toil, improve model governance, and support common credit/quant workflows. The proposals are prioritized by immediate pain relief (P0 = variance/scenarios/working capital) vs. longer-term value (P2 = versioning/SaaS metrics).