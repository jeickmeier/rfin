# Statements API Reference

**Last updated:** 2025-09-30

---

## Table of Contents

1. [Wire Types](#1-wire-types)
2. [DSL Syntax](#2-dsl-syntax)
3. [Builder API](#3-builder-api)
4. [Evaluator API](#4-evaluator-api)
5. [Results API](#5-results-api)
6. [Registry API](#6-registry-api)

---

## 1. Wire Types

### 1.1 Core Types

#### `FinancialModelSpec`

Top-level model specification.

```rust
pub struct FinancialModelSpec {
    pub id: String,
    pub periods: Vec<Period>,
    pub nodes: IndexMap<String, NodeSpec>,
    pub capital_structure: Option<CapitalStructureSpec>,
    pub meta: IndexMap<String, serde_json::Value>,
    pub schema_version: u32, // default: 1
}
```

**JSON Example:**
```json
{
  "id": "acme_corp_model",
  "periods": [...],
  "nodes": {
    "revenue": {...},
    "cogs": {...}
  },
  "schema_version": 1
}
```

---

#### `NodeSpec`

Individual metric/line item specification.

```rust
pub struct NodeSpec {
    pub node_id: String,
    pub name: Option<String>,
    pub node_type: NodeType,
    pub values: Option<IndexMap<PeriodId, AmountOrScalar>>,
    pub forecasts: Vec<ForecastSpec>,
    pub formula_text: Option<String>,
    pub where_text: Option<String>,
    pub tags: Vec<String>,
    pub meta: IndexMap<String, serde_json::Value>,
}
```

**JSON Example:**
```json
{
  "node_id": "revenue",
  "name": "Total Revenue",
  "node_type": "mixed",
  "values": {
    "2025Q1": 1000000.0,
    "2025Q2": 1100000.0
  },
  "forecasts": [
    {
      "method": "growth_pct",
      "params": {"rate": 0.05}
    }
  ],
  "formula_text": "lag(revenue, 4) * 1.05",
  "tags": ["income_statement", "top_line"]
}
```

---

#### `NodeType`

Node computation type.

```rust
pub enum NodeType {
    Value,       // Only explicit values
    Calculated,  // Only formula
    Mixed,       // Value OR Forecast OR Formula
}
```

**Precedence:** Value > Forecast > Formula

---

#### `AmountOrScalar`

Value that can be currency-aware or unitless.

```rust
pub enum AmountOrScalar {
    Amount(Money),  // Currency-aware: { "amount": 100.0, "currency": "USD" }
    Scalar(f64),    // Unitless: 100.0 or 0.15 (for ratios)
}
```

**JSON Examples:**
```json
// Currency amount
{"amount": 1000000.0, "currency": "USD"}

// Scalar (ratio, percentage, count)
0.15
```

---

### 1.2 Forecast Types

#### `ForecastSpec`

Forecast method specification.

```rust
pub struct ForecastSpec {
    pub method: ForecastMethod,
    pub params: IndexMap<String, serde_json::Value>,
}
```

#### `ForecastMethod`

Available forecast methods.

```rust
pub enum ForecastMethod {
    ForwardFill,   // Carry last value forward
    GrowthPct,     // Compound growth: v[t] = v[t-1] * (1 + rate)
    Normal,        // Sample from normal distribution
    LogNormal,     // Sample from log-normal distribution
    Override,      // Explicit period overrides
    TimeSeries,    // Reference external time series
    Seasonal,      // Seasonal pattern (additive/multiplicative)
}
```

**Examples:**

```rust
// Forward fill
ForecastSpec {
    method: ForecastMethod::ForwardFill,
    params: indexmap! {},
}

// Growth rate
ForecastSpec {
    method: ForecastMethod::GrowthPct,
    params: indexmap! {
        "rate".into() => json!(0.05),  // 5% growth
    },
}

// Normal distribution (deterministic with seed)
ForecastSpec {
    method: ForecastMethod::Normal,
    params: indexmap! {
        "mean".into() => json!(100_000.0),
        "std_dev".into() => json!(15_000.0),
        "seed".into() => json!(42),
    },
}
```

---

## 2. DSL Syntax

### 2.1 Operators

#### Arithmetic

| Operator | Description | Example |
|----------|-------------|---------|
| `+` | Addition | `revenue + other_income` |
| `-` | Subtraction | `revenue - cogs` |
| `*` | Multiplication | `revenue * 0.6` |
| `/` | Division | `gross_profit / revenue` |
| `%` | Modulo | `period_num % 4` |

#### Comparison

| Operator | Description | Example |
|----------|-------------|---------|
| `==` | Equal | `revenue == 1000000` |
| `!=` | Not equal | `revenue != 0` |
| `<` | Less than | `margin < 0.1` |
| `<=` | Less than or equal | `revenue <= 1000000` |
| `>` | Greater than | `revenue > 1000000` |
| `>=` | Greater than or equal | `margin >= 0.2` |

#### Logical

| Operator | Description | Example |
|----------|-------------|---------|
| `and` | Logical AND | `revenue > 1000000 and margin > 0.15` |
| `or` | Logical OR | `revenue < 100000 or expenses > 50000` |

---

### 2.2 Time-Series Functions

| Function | Description | Example |
|----------|-------------|---------|
| `lag(node, n)` | Value from n periods ago | `lag(revenue, 1)` |
| `lead(node, n)` | Value from n periods ahead | `lead(revenue, 1)` |
| `diff(node, n)` | First difference | `diff(revenue, 1)` |
| `pct_change(node, n)` | Percentage change | `pct_change(revenue, 1)` |
| `rolling_mean(node, window)` | Rolling average | `rolling_mean(revenue, 4)` |
| `rolling_sum(node, window)` | Rolling sum | `rolling_sum(revenue, 12)` |
| `rolling_std(node, window)` | Rolling std dev | `rolling_std(revenue, 4)` |
| `cumsum(node)` | Cumulative sum | `cumsum(net_income)` |
| `cumprod(node)` | Cumulative product | `cumprod(1 + growth_rate)` |

**Example:**
```rust
// Year-over-year growth
.compute("yoy_growth", "pct_change(revenue, 4)")  // Quarterly data

// Trailing twelve months
.compute("ttm_revenue", "rolling_sum(revenue, 4)")
```

---

### 2.3 Statistical Functions

| Function | Description | Example |
|----------|-------------|---------|
| `mean(node)` | Average across periods | `mean(revenue)` |
| `median(node)` | Median value | `median(revenue)` |
| `std(node)` | Standard deviation | `std(revenue)` |
| `var(node)` | Variance | `var(revenue)` |
| `min(a, b, ...)` | Minimum value | `min(revenue, 1000000)` |
| `max(a, b, ...)` | Maximum value | `max(revenue, expenses)` |

---

### 2.4 Financial Functions

| Function | Description | Example |
|----------|-------------|---------|
| `sum(a, b, ...)` | Sum multiple nodes | `sum(revenue, other_income)` |
| `annualize(node, periods)` | Annualize a value | `annualize(net_income, 4)` |
| `ttm(node)` | Trailing twelve months | `ttm(ebitda)` |
| `coalesce(node, default)` | Null coalescing | `coalesce(bonus, 0)` |

**Examples:**
```rust
// Return on equity (annualized)
.compute("roe", "annualize(net_income, 4) / total_equity")

// Debt to EBITDA (leverage ratio)
.compute("debt_to_ebitda", "total_debt / ttm(ebitda)")
```

---

### 2.5 Conditional Expressions

```rust
// If-then-else
.compute("bonus", "if(revenue > 1000000, revenue * 0.1, 0)")

// Where clause (period masking)
.compute("quarterly_bonus", "revenue * 0.05")
.where_clause("quarterly_bonus", "period_type == 'quarter'")
```

---

### 2.6 Capital Structure References

When capital structure is enabled, you can reference cashflows:

```rust
// Reference specific instrument
"cs.interest_expense.BOND-001"
"cs.principal_payment.BOND-001"

// Aggregate across all instruments
"cs.interest_expense.total"
"cs.principal_payment.total"

// Example
.compute("total_interest", "cs.interest_expense.total")
```

---

## 3. Builder API

### 3.1 Model Construction

```rust
use finstack_statements::prelude::*;

let model = ModelBuilder::new("model_id")
    // 1. Define periods (required first)
    .periods("2025Q1..2025Q4", Some("2025Q1..Q2"))?
    
    // 2. Add nodes
    .value("revenue", &[
        (PeriodId::quarter(2025, 1), AmountOrScalar::Scalar(100.0)),
    ])
    .compute("cogs", "revenue * 0.6")?
    .compute("gross_profit", "revenue - cogs")?
    
    // 3. Add forecasts
    .forecast("revenue", ForecastSpec {
        method: ForecastMethod::GrowthPct,
        params: indexmap! { "rate".into() => json!(0.05) },
    })
    
    // 4. Add metrics from registry
    .with_builtin_metrics()?
    
    // 5. Build
    .build()?;
```

### 3.2 Builder Methods

#### Period Configuration

```rust
// Parse period range
.periods("2025Q1..2025Q4", Some("2025Q1..Q2"))?

// Or explicit periods
.periods_explicit(vec![
    Period { id: PeriodId::quarter(2025, 1), ... },
    Period { id: PeriodId::quarter(2025, 2), ... },
])?
```

#### Node Configuration

```rust
// Explicit values
.value("node_id", &[(period_id, AmountOrScalar::Scalar(100.0))])

// Calculated nodes
.compute("node_id", "formula")?

// Mixed nodes (value + forecast + formula)
.mixed("node_id")
    .values(&[(period_id, value)])
    .forecasts(vec![forecast_spec])
    .formula("fallback_formula")?
```

#### Forecast Configuration

```rust
.forecast("node_id", ForecastSpec {
    method: ForecastMethod::GrowthPct,
    params: indexmap! { "rate".into() => json!(0.05) },
})
```

#### Conditional Logic

```rust
.where_clause("node_id", "boolean_formula")?
```

#### Registry Integration

```rust
// Load built-in metrics (fin.*)
.with_builtin_metrics()?

// Load custom metrics from JSON
.with_metrics("path/to/metrics.json")?

// Add specific metric from registry
.add_metric("fin.gross_margin")?
```

#### Capital Structure (feature: `capital_structure`)

```rust
// Add bond
.add_bond(
    "BOND-001",
    Money::new(10_000_000.0, Currency::USD),
    0.05,  // coupon rate
    issue_date,
    maturity_date,
    "USD-OIS",  // discount curve ID
)?

// Add swap
.add_swap(
    "SWAP-001",
    Money::new(5_000_000.0, Currency::USD),
    0.04,  // fixed rate
    start_date,
    maturity_date,
    "USD-OIS",   // discount curve
    "USD-LIBOR", // forward curve
)?
```

---

## 4. Evaluator API

### 4.1 Basic Evaluation

```rust
use finstack_statements::Evaluator;

let mut evaluator = Evaluator::new();
let results = evaluator.evaluate(&model, parallel: false)?;
```

### 4.2 With Market Context (for capital structure)

```rust
let market_ctx = MarketContext::new()
    .insert_discount(discount_curve)
    .insert_forward(forward_curve);

let mut evaluator = Evaluator::with_market_context(Arc::new(market_ctx));
let results = evaluator.evaluate(&model, false)?;
```

### 4.3 Selective Evaluation

```rust
// Only evaluate specific nodes (future)
let results = evaluator.evaluate_selective(&model, &["revenue", "gross_profit"])?;
```

---

## 5. Results API

### 5.1 Results Structure

```rust
pub struct Results {
    pub nodes: IndexMap<String, IndexMap<PeriodId, f64>>,
    pub periods: Vec<Period>,
    pub meta: ResultsMeta,
}
```

### 5.2 Access Results

```rust
// By node and period
let revenue_q1 = results.nodes["revenue"][&PeriodId::quarter(2025, 1)];

// Iterate nodes
for (node_id, period_map) in &results.nodes {
    for (period_id, value) in period_map {
        println!("{} {} = {}", node_id, period_id, value);
    }
}
```

### 5.3 Export to DataFrame

```rust
// Long format: (node_id, period_id, value)
let df = results.to_polars_long()?;

// Wide format: periods as rows, nodes as columns
let df = results.to_polars_wide()?;

// Filter nodes
let df = results.to_polars_long_filtered(&["revenue", "cogs"])?;
```

---

## 6. Registry API

### 6.1 Loading Metrics

```rust
use finstack_statements::registry::Registry;

let mut registry = Registry::new();

// Load built-in metrics
registry.load_builtins()?;

// Load from JSON file
registry.load_from_json("path/to/metrics.json")?;
```

### 6.2 Querying Metrics

```rust
// Get specific metric
let metric = registry.get("fin.gross_margin")?;

// List all metrics in namespace
for (id, metric) in registry.namespace("fin") {
    println!("{}: {}", id, metric.definition.name);
}

// List all namespaces
let namespaces = registry.namespaces();
```

### 6.3 Custom Metric Definition (JSON)

```json
{
  "namespace": "custom",
  "schema_version": 1,
  "metrics": [
    {
      "id": "gross_margin",
      "name": "Gross Margin %",
      "formula": "gross_profit / revenue",
      "description": "Gross profit as percentage of revenue",
      "category": "margins",
      "unit_type": "percentage",
      "requires": ["gross_profit", "revenue"]
    }
  ]
}
```

---

## 7. Error Types

### 7.1 Common Errors

```rust
pub enum Error {
    /// Model building error (e.g., invalid period range)
    Build(String),
    
    /// Formula parsing error
    FormulaParse(String),
    
    /// Evaluation error (e.g., circular dependency)
    Eval(String),
    
    /// Node not found
    NodeNotFound { node_id: String },
    
    /// Circular dependency detected
    CircularDependency { path: Vec<String> },
    
    /// Currency mismatch
    CurrencyMismatch { expected: Currency, found: Currency },
    
    // ... more variants
}
```

---

## 8. Complete Example

```rust
use finstack_statements::prelude::*;

fn build_pl_model() -> Result<FinancialModel> {
    ModelBuilder::new("Acme Corp P&L")
        // Define periods
        .periods("2025Q1..2025Q4", Some("2025Q1..Q2"))?
        
        // Revenue (actuals + forecast)
        .value("revenue", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::Scalar(10_000_000.0)),
            (PeriodId::quarter(2025, 2), AmountOrScalar::Scalar(11_000_000.0)),
        ])
        .forecast("revenue", ForecastSpec {
            method: ForecastMethod::GrowthPct,
            params: indexmap! { "rate".into() => json!(0.05) },
        })
        
        // Cost of goods sold (60% of revenue)
        .compute("cogs", "revenue * 0.6")?
        
        // Operating expenses
        .value("operating_expenses", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::Scalar(2_000_000.0)),
            (PeriodId::quarter(2025, 2), AmountOrScalar::Scalar(2_100_000.0)),
        ])
        .forecast("operating_expenses", ForecastSpec {
            method: ForecastMethod::ForwardFill,
            params: indexmap! {},
        })
        
        // Load standard financial metrics
        .with_builtin_metrics()?
        
        .build()
}

fn main() -> Result<()> {
    let model = build_pl_model()?;
    
    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false)?;
    
    // Export to DataFrame
    let df = results.to_polars_long()?;
    println!("{}", df);
    
    Ok(())
}
```

---

## References

- [Architecture](./ARCHITECTURE.md)
- [Implementation Plan](./IMPLEMENTATION_PLAN.md)
- [Examples](./examples/)
- [Rust API Docs](https://docs.rs/finstack-statements) (when published)
