# Finstack Statements — Financial Statement Modeling Engine

**Status:** Draft (implementation-ready)  
**Last updated:** 2025-09-30  
**MSRV:** 1.75 (target)  
**License:** Apache-2.0

---

## 🎯 What is Finstack Statements?

The `finstack-statements` crate enables users to build financial statement models as directed graphs of metrics evaluated over discrete periods (monthly, quarterly, annually). It provides:

- **Declarative modeling** with a rich DSL for formulas
- **Time-series forecasting** with deterministic and statistical methods
- **Capital structure integration** for debt/equity tracking
- **Dynamic metric registry** (no recompilation needed)
- **Currency-safe arithmetic** with explicit FX handling
- **Deterministic evaluation** (serial ≡ parallel)

---

## 🚀 Quick Start

### Basic Example

```rust
use finstack_statements::prelude::*;

// Build a simple P&L model
let model = ModelBuilder::new("Acme Corp")
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
    
    // Calculated metrics
    .compute("cogs", "revenue * 0.6")?
    .compute("gross_profit", "revenue - cogs")?
    .compute("gross_margin", "gross_profit / revenue")?
    
    .build()?;

// Evaluate the model
let mut evaluator = Evaluator::new();
let results = evaluator.evaluate(&model, false)?;

// Export to DataFrame
let df = results.to_polars_long()?;
println!("{}", df);
```

---

## 🏗️ Core Features

### 1. **Three Node Types**

| Type | Description | Use Case |
|------|-------------|----------|
| **Value** | Explicit values only | Actuals, assumptions |
| **Calculated** | Formula-derived | Computed metrics |
| **Mixed** | Value OR Forecast OR Formula | Flexible modeling |

**Precedence Rule:** Value > Forecast > Formula

### 2. **Rich DSL**

Time-series operators:
```rust
"lag(revenue, 1)"           // Previous period
"rolling_mean(revenue, 4)"  // 4-period moving average
"pct_change(revenue, 1)"    // Period-over-period growth
"ttm(ebitda)"               // Trailing twelve months
```

### 3. **Forecast Methods**

- **Deterministic:** ForwardFill, GrowthPct, Override
- **Statistical:** Normal, LogNormal (with seed for determinism)
- **Time-series:** Seasonal patterns, indexed growth

### 4. **Capital Structure**

Track debt instruments and integrate cashflows:

```rust
.add_bond(
    "BOND-001",
    Money::new(10_000_000.0, Currency::USD),
    0.05,  // 5% coupon
    issue_date,
    maturity_date,
    "USD-OIS",
)?
.compute("interest_expense", "cs.interest_expense.BOND-001")?
```

### 5. **Dynamic Metrics**

Load reusable metrics from JSON:

```json
{
  "namespace": "fin",
  "metrics": [
    {
      "id": "gross_margin",
      "formula": "gross_profit / revenue",
      "description": "Gross profit as % of revenue"
    }
  ]
}
```

---

## 📚 Documentation

- **[Architecture](./ARCHITECTURE.md)** - High-level design and integration points
- **[API Reference](./API_REFERENCE.md)** - Wire types, DSL syntax, functions
- **[Implementation Plan](./IMPLEMENTATION_PLAN.md)** - Phased rollout strategy
- **[Capital Structure](./CAPITAL_STRUCTURE.md)** - Debt/equity integration guide
- **[Testing Strategy](./TESTING_STRATEGY.md)** - Unit, integration, and golden tests
- **[Examples](./examples/)** - Complete working examples

---

## 🎓 Key Concepts

### Glossary

| Term | Definition |
|------|------------|
| **Node** | A single metric/line item (e.g., `revenue`, `ebitda`) |
| **Period** | A time interval (quarter, month, year) |
| **Precedence** | Evaluation priority: Value > Forecast > Formula |
| **Registry** | Collection of reusable metric definitions |
| **Extension** | Plugin that adds analysis capabilities |

### Evaluation Flow

```
1. Load model spec (JSON or builder)
   ↓
2. Compile formulas (DSL → core Expr)
   ↓
3. Build dependency graph (DAG)
   ↓
4. For each period:
   a. Resolve node values (precedence)
   b. Evaluate formulas (topological order)
   c. Store results
   ↓
5. Export to DataFrame
```

---

## 🔗 Integration Points

### From `finstack-core`

- ✅ Period system (`Period`, `PeriodPlan`, `PeriodId`)
- ✅ Expression engine (`Expr`, `CompiledExpr`)
- ✅ Money types (`Money`, `Currency`)
- ✅ Polars DataFrame/Series

### From `finstack-valuations`

- ✅ Instrument types (`Bond`, `InterestRateSwap`)
- ✅ Cashflow aggregation (`aggregate_by_period`)
- ✅ Metric calculation (interest expense, principal payments)

---

## 📦 Features & Dependencies

```toml
[features]
default = ["serde"]
capital_structure = ["dep:finstack-valuations"]
stats = ["dep:rand"]
parallel = ["finstack-core/parallel"]
full = ["capital_structure", "stats", "parallel"]
```

**Key Dependencies:**
- `finstack-core` - Period system, expression engine
- `finstack-valuations` - Instruments (optional)
- `serde`, `serde_json` - Serialization
- `indexmap` - Deterministic maps
- `nom` - Parser combinators
- `rand` - Statistical forecasting (optional)

---

## 🎯 Use Cases

1. **Financial Modeling** - Build P&L, balance sheet, cash flow models
2. **Scenario Analysis** - Forecast with various growth assumptions
3. **Debt Tracking** - Model interest expense and amortization schedules
4. **Portfolio Analysis** - Aggregate results across entities
5. **Credit Analysis** - Calculate leverage ratios and coverage metrics

---

## 🚦 Implementation Status

### ✅ v0.1 (MVP)
- Core wire types
- Basic DSL (arithmetic, node references)
- Value/Calculated/Mixed nodes
- Forward fill & growth forecasts
- DataFrame export

### 🚧 v0.2 (In Progress)
- Complete DSL with time-series operators
- Statistical forecasting
- Dynamic metric registry
- Capital structure integration

### 📋 v1.0 (Planned)
- Extension plugin system
- Python bindings
- WASM bindings
- Advanced performance optimizations

---

## 🔍 How This Differs From...

### vs Python Implementation
- ✅ Type-safe, compiled
- ✅ 10-100x faster evaluation
- ✅ Deterministic by default
- ⚠️ Less flexible DSL (by design)

### vs Excel Models
- ✅ Version controllable (JSON/code)
- ✅ Programmatic construction
- ✅ Scalable (1000+ metrics)
- ⚠️ Steeper learning curve

### vs Traditional BI Tools
- ✅ Code-first, reproducible
- ✅ Currency-safe by default
- ✅ Period-aware semantics
- ⚠️ No GUI builder

---

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --test builder_tests
cargo test --test evaluator_tests

# Check code coverage
cargo tarpaulin --out Html
```

---

## 📖 Examples

- **[Basic P&L](./examples/basic_pl_statement.md)** - Simple profit & loss model
- **[Forecasting Methods](./examples/forecasting_methods.md)** - All forecast types
- **[Capital Structure](./examples/capital_structure.md)** - Debt tracking
- **[Time Series Analysis](./examples/time_series_analysis.md)** - Rolling metrics
- **[Custom Metrics](./examples/custom_metrics.md)** - Dynamic registry

---

## 🤝 Contributing

See [IMPLEMENTATION_PLAN.md](./IMPLEMENTATION_PLAN.md) for phased development strategy and [TESTING_STRATEGY.md](./TESTING_STRATEGY.md) for test requirements.

**Development Workflow:**
1. Pick a phase/PR from implementation plan
2. Write tests first (TDD)
3. Implement feature
4. Ensure CI passes (lint, test, doc)
5. Submit PR with phase tag

---

## 🔗 Quick Links

- [Architecture Overview](./ARCHITECTURE.md)
- [API Reference](./API_REFERENCE.md)
- [Implementation Plan](./IMPLEMENTATION_PLAN.md)
- [Testing Strategy](./TESTING_STRATEGY.md)
- [Original PRD](../04_statements_prd.md)
- [Original TDD](../04_statements_tdd.md)
