# Statements Crate — Implementation Plan

**Last updated:** 2025-09-30  
**Timeline:** 8-10 weeks to v1.0  
**Team:** 2-3 engineers

---

## Overview

This document outlines the phased implementation strategy for the `finstack-statements` crate. Each phase is broken into small PRs with clear acceptance criteria.

**Key Principles:**
- Small, reviewable PRs (<500 lines)
- Test-driven development (write tests first)
- No breaking changes between phases
- Each PR should be independently valuable

---

## Phase Summary

| Phase | Duration | Focus | Deliverables |
|-------|----------|-------|--------------|
| **Phase 1** | Week 1-2 | Foundation | Wire types, builder, value nodes |
| **Phase 2** | Week 2-3 | DSL Engine | Parser, compiler, operators |
| **Phase 3** | Week 3-4 | Evaluator | Context, DAG, precedence |
| **Phase 4** | Week 4-5 | Forecasting | Forward fill, growth, statistical |
| **Phase 5** | Week 5-6 | Dynamic Registry | JSON metrics, namespaces |
| **Phase 6** | Week 6-7 | Capital Structure | Debt instruments, cashflows |
| **Phase 7** | Week 7 | Results & Export | DataFrame exports, metadata |
| **Phase 8** | Week 8+ | Extensions | Plugin system, placeholders |

---

## Phase 1: Foundation (Week 1-2)

### Goals
- Set up crate infrastructure
- Implement core wire types
- Build type-state builder pattern
- Support value nodes (explicit values)

### PR #1.1 — Crate Bootstrap

**Deliverables:**
- [ ] Create `finstack/statements/` directory
- [ ] Add `Cargo.toml` with dependencies
- [ ] Create `src/lib.rs` with module structure
- [ ] Define `Error` type hierarchy in `error.rs`
- [ ] Wire types: `NodeSpec`, `NodeType`, `AmountOrScalar`
- [ ] Type-state builder skeleton (`ModelBuilder<NeedPeriods>`, `ModelBuilder<Ready>`)

**Dependencies:**
```toml
[dependencies]
finstack-core = { path = "../core", version = "0.2" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
indexmap = { version = "2", features = ["serde"] }
thiserror = "1"
```

**Acceptance:**
- `cargo check` passes
- CI green (lint + test)
- Can create `ModelBuilder::new("test")`

---

### PR #1.2 — Period Integration

**Deliverables:**
- [ ] Implement `ModelBuilder::periods()` using core's `build_periods()`
- [ ] Add `FinancialModelSpec` wire type
- [ ] Add period validation (non-empty, sorted)
- [ ] Basic builder tests

**Example:**
```rust
let builder = ModelBuilder::new("test")
    .periods("2025Q1..Q4", Some("2025Q1..Q2"))?;
```

**Acceptance:**
- Can create model with periods
- Periods are parsed correctly (actuals vs forecast)
- Model can be serialized/deserialized via serde

---

### PR #1.3 — Value Nodes

**Deliverables:**
- [ ] Implement `ModelBuilder::value()` for explicit values
- [ ] Add value storage in `NodeSpec::values`
- [ ] Add value precedence resolution (values always win)
- [ ] Unit tests for value storage and retrieval

**Example:**
```rust
.value("revenue", &[
    (PeriodId::quarter(2025, 1), AmountOrScalar::Scalar(100.0)),
    (PeriodId::quarter(2025, 2), AmountOrScalar::Scalar(110.0)),
])
```

**Acceptance:**
- Can set explicit values per period
- Values are retrieved correctly
- `AmountOrScalar` supports both `Money` and scalar

---

## Phase 2: DSL Engine (Week 2-3)

### Goals
- Parse formula text into AST
- Compile AST to core `Expr`
- Support time-series and statistical operators

### PR #2.1 — DSL Parser

**Deliverables:**
- [ ] Implement `StmtExpr` AST (see [API_REFERENCE.md](./API_REFERENCE.md))
- [ ] Parser for basic arithmetic: `+`, `-`, `*`, `/`
- [ ] Node references and literals
- [ ] Unit tests for parser

**Dependencies:**
```toml
nom = "7"
```

**Acceptance:**
- Can parse `"revenue - cogs"`
- Can parse `"revenue * 1.05"`
- Parser errors are clear and actionable

---

### PR #2.2 — DSL Compiler

**Deliverables:**
- [ ] Implement `compile()` to convert `StmtExpr` → core `Expr`
- [ ] Handle binary operations (map to core functions)
- [ ] Unit tests for compilation

**Acceptance:**
- Compiled expressions evaluate correctly
- Compilation errors include node context

---

### PR #2.3 — Time-Series Operators

**Deliverables:**
- [ ] Add `lag`, `lead`, `diff`, `pct_change` to parser
- [ ] Map to core's `Function` enum
- [ ] Integration tests

**Acceptance:**
- Can evaluate `"lag(revenue, 1)"`
- Can evaluate `"pct_change(revenue, 1)"`

---

### PR #2.4 — Rolling Window Functions

**Deliverables:**
- [ ] Add `rolling_mean`, `rolling_sum`, `rolling_std`
- [ ] Map to core's rolling functions
- [ ] Unit tests for window semantics

**Acceptance:**
- Can calculate `"rolling_mean(revenue, 4)"`
- Window boundaries handled correctly

---

### PR #2.5 — Statistical Functions

**Deliverables:**
- [ ] Add `mean`, `median`, `std`, `var`
- [ ] Map to core's statistical functions
- [ ] Unit tests

**Acceptance:**
- Can calculate `"std(revenue)"` across periods
- Functions work with partial data

---

### PR #2.6 — Custom Functions

**Deliverables:**
- [ ] Implement `sum()`, `mean()`, `annualize()`, `ttm()`
- [ ] Function argument validation
- [ ] Integration tests

**Acceptance:**
- Can use `"ttm(revenue)"` for trailing twelve months
- Can use `"annualize(ebitda, 4)"`

---

## Phase 3: Evaluator (Week 3-4)

### Goals
- Build evaluation context
- Construct dependency graph (DAG)
- Implement precedence resolution
- Evaluate models period-by-period

### PR #3.1 — Evaluation Context

**Deliverables:**
- [ ] Implement `StatementContext`
- [ ] Implement `ExpressionContext` trait from core
- [ ] Column mapping for node references
- [ ] Prior period results tracking

**Acceptance:**
- Context resolves node references correctly
- Can access prior period values

---

### PR #3.2 — Basic Evaluator

**Deliverables:**
- [ ] Implement `Evaluator::evaluate()`
- [ ] Per-period evaluation loop
- [ ] Formula evaluation via core's `CompiledExpr`
- [ ] Simple node evaluation (no dependencies yet)

**Acceptance:**
- Can evaluate simple calculated nodes
- Results stored correctly

---

### PR #3.3 — DAG Construction

**Deliverables:**
- [ ] Build dependency graph from node formulas
- [ ] Topological sort for evaluation order
- [ ] Circular dependency detection

**Example Error:**
```
Error: Circular dependency detected
Path: revenue → cogs → gross_profit → revenue
```

**Acceptance:**
- Detects cycles and reports full path
- Evaluates nodes in correct topological order

---

### PR #3.4 — Precedence Resolution

**Deliverables:**
- [ ] Implement Value > Forecast > Formula precedence
- [ ] Per-period precedence logic
- [ ] Unit tests for each precedence level

**Acceptance:**
- Value always wins over forecast/formula
- Forecast wins over formula
- Formula is fallback

---

### PR #3.5 — Where Clause Masking

**Deliverables:**
- [ ] Implement where clause evaluation
- [ ] Boolean mask application
- [ ] Tests for conditional inclusion

**Example:**
```rust
.compute("bonus", "revenue * 0.1")
.where_clause("bonus", "revenue > 1000000")
```

**Acceptance:**
- Where clause filters periods correctly
- Masked periods return None or 0

---

## Phase 4: Forecasting (Week 4-5)

### Goals
- Implement deterministic forecast methods
- Implement statistical forecast methods
- Ensure determinism with seeds

### PR #4.1 — Forward Fill

**Deliverables:**
- [ ] Implement `ForwardFill` method
- [ ] Carry last actual value into forecast periods
- [ ] Unit tests

**Acceptance:**
- Forward fill extends values correctly
- Works with partial actuals

---

### PR #4.2 — Growth Percentage

**Deliverables:**
- [ ] Implement `GrowthPct` method
- [ ] Apply compound growth: `v[t] = v[t-1] * (1 + g)`
- [ ] Unit tests with various growth rates

**Acceptance:**
- Growth calculations match expected values
- Handles negative growth

---

### PR #4.3 — Statistical Forecasting (Normal)

**Deliverables:**
- [ ] Implement `Normal` forecast method
- [ ] Use `finstack-core::math::random::SimpleRng` for sampling
- [ ] Parameters: `mean`, `std_dev`, `seed`
- [ ] Deterministic with seed

**Dependencies:**
```toml
[features]
stats = ["dep:rand"]
```

**Acceptance:**
- Samples from normal distribution deterministically with seed
- Parameters validated

---

### PR #4.4 — Log-Normal Forecasting

**Deliverables:**
- [ ] Implement `LogNormal` method
- [ ] Use for positive-only values (revenue, prices)
- [ ] Unit tests

**Acceptance:**
- Log-normal samples are always positive
- Mean/std_dev parameters work correctly

---

### PR #4.5 — Override Method

**Deliverables:**
- [ ] Implement `Override` with sparse period map
- [ ] Allow explicit overrides per period
- [ ] Unit tests

**Acceptance:**
- Overrides work correctly
- Can override specific forecast periods

---

## Phase 5: Dynamic Registry (Week 5-6)

### Goals
- Define JSON schema for metrics
- Load metrics from JSON files
- Support namespace management

### PR #5.1 — JSON Schema

**Deliverables:**
- [ ] Define `MetricDefinition` and `MetricRegistry` types
- [ ] Create JSON schema documentation
- [ ] Validation helpers

**Acceptance:**
- JSON schema is well-documented
- Schema includes examples

---

### PR #5.2 — Registry Loader

**Deliverables:**
- [ ] Implement `Registry::load_from_json()`
- [ ] Compile formulas from JSON
- [ ] Error handling for invalid formulas

**Acceptance:**
- Can load metrics from JSON file
- Clear errors for invalid formulas

---

### PR #5.3 — Built-in Metrics JSON

**Deliverables:**
- [ ] Create `data/metrics/fin_basic.json`
- [ ] Create `fin_margins.json`, `fin_returns.json`, `fin_leverage.json`
- [ ] Embed in crate using `include_str!`
- [ ] Implement `Registry::load_builtins()`

**Example Metrics:**
- `fin.gross_profit = revenue - cogs`
- `fin.gross_margin = gross_profit / revenue`
- `fin.ebitda = operating_income + depreciation + amortization`

**Acceptance:**
- Built-in metrics load correctly
- All standard financial metrics included

---

### PR #5.4 — Registry Integration

**Deliverables:**
- [ ] Add registry to `FinancialModel`
- [ ] ModelBuilder methods: `.with_metrics(path)` and `.with_builtin_metrics()`
- [ ] Integration tests

**Acceptance:**
- Can add metrics from registry to model
- Metrics evaluate correctly

---

### PR #5.5 — Namespace Management

**Deliverables:**
- [ ] Implement namespace scoping (`fin.*`, custom namespaces)
- [ ] Collision detection
- [ ] List available metrics per namespace

**Acceptance:**
- Namespaces prevent collisions
- Can query available metrics

---

## Phase 6: Capital Structure Integration (Week 6-7)

### Goals
- Build debt instruments from specs
- Aggregate cashflows by period
- Calculate interest expense and principal schedules

### PR #6.1 — Instrument Construction

**Deliverables:**
- [ ] Implement `DebtInstrumentSpec` types (Bond, Swap, Generic)
- [ ] Build instruments from specs using valuations
- [ ] Unit tests for each instrument type

**Dependencies:**
```toml
[features]
capital_structure = ["dep:finstack-valuations"]
```

**Acceptance:**
- Can construct Bond from spec
- Can construct Swap from spec

---

### PR #6.2 — Cashflow Aggregation

**Deliverables:**
- [ ] Use `finstack_valuations::cashflow::aggregate_by_period`
- [ ] Map cashflow kinds to statement categories
- [ ] Integration tests

**Acceptance:**
- Cashflows aggregate correctly by period
- Multi-currency handled with FX

---

### PR #6.3 — Interest Expense Calculation

**Deliverables:**
- [ ] Calculate interest expense per period
- [ ] Handle fixed and floating coupons
- [ ] Unit tests

**Acceptance:**
- Interest expense matches instrument schedules
- Day count conventions respected

---

### PR #6.4 — Principal Schedule

**Deliverables:**
- [ ] Calculate principal payments (amortization)
- [ ] Track outstanding balance
- [ ] Unit tests

**Acceptance:**
- Principal schedules match instrument specs
- Works for bullet, amortizing, and custom schedules

---

### PR #6.5 — Capital Structure Builder API

**Deliverables:**
- [ ] Implement `ModelBuilder::add_debt()`, `ModelBuilder::add_bond()`
- [ ] Fluent API for capital structure
- [ ] Integration tests

**Example:**
```rust
.add_bond("BOND-001", Money::new(10_000_000.0, Currency::USD), 
          0.05, issue_date, maturity_date, "USD-OIS")?
.compute("interest_expense", "cs.interest_expense.BOND-001")?
```

**Acceptance:**
- Can build capital structure fluently
- DSL can reference capital structure nodes

---

## Phase 7: Results & Export (Week 7)

### Goals
- Implement results structure
- Export to Polars DataFrame (long/wide format)
- Stamp metadata

### PR #7.1 — Results Structure

**Deliverables:**
- [ ] Implement `Results` type
- [ ] Period-by-node value storage (`IndexMap<String, IndexMap<PeriodId, f64>>`)
- [ ] Metadata tracking

**Acceptance:**
- Results store all evaluated values
- Efficient lookups

---

### PR #7.2 — Long-Format Export

**Deliverables:**
- [ ] Implement `Results::to_polars_long()`
- [ ] Schema: `(node_id, period_id, value)`
- [ ] Unit tests

**Acceptance:**
- Long format matches expected schema
- Can filter by node or period

---

### PR #7.3 — Wide-Format Export

**Deliverables:**
- [ ] Implement `Results::to_polars_wide()`
- [ ] Schema: periods as rows, nodes as columns
- [ ] Unit tests

**Acceptance:**
- Wide format matches expected schema
- Easy to visualize

---

### PR #7.4 — Metadata Stamping

**Deliverables:**
- [ ] Include `ResultsMeta` from core
- [ ] Track FX policies, rounding context
- [ ] Serialize to JSON

**Acceptance:**
- Metadata is complete and serializable
- Includes execution time, node count

---

## Phase 8: Extensions (Week 8+)

### Goals
- Finalize extension plugin system
- Create placeholder extensions

### PR #8.1 — Extension Plugin System

**Deliverables:**
- [ ] Finalize `Extension` trait
- [ ] Implement `ExtensionRegistry`
- [ ] Registration and execution
- [ ] Documentation

**Acceptance:**
- Can register and execute extensions
- Extensions receive model and results

---

### PR #8.2 — Corkscrew Extension (Placeholder)

**Deliverables:**
- [ ] Create skeleton `CorkscrewExtension`
- [ ] Validate it loads correctly
- [ ] Documentation for future implementation

**Acceptance:**
- Extension compiles
- No-ops gracefully with "not_implemented" status

---

### PR #8.3 — Credit Scorecard Extension (Placeholder)

**Deliverables:**
- [ ] Create skeleton `CreditScorecardExtension`
- [ ] Define `ScorecardConfig` schema
- [ ] Documentation

**Acceptance:**
- Extension compiles
- Validates config structure

---

## Release Criteria

### MVP Release (v0.1.0)

**Must Have:**
- [ ] Core wire types stable
- [ ] Builder pattern works (phases 1-3)
- [ ] Basic DSL (arithmetic, node references)
- [ ] Value/Calculate/Mixed node types
- [ ] Forward fill and growth forecasts
- [ ] Simple evaluator (no capital structure yet)
- [ ] DataFrame export (long/wide)
- [ ] 50+ passing tests
- [ ] Documentation with examples

**Timeline:** Week 1-4

---

### Production Release (v1.0.0)

**Must Have:**
- [ ] All MVP features
- [ ] Complete DSL with all operators
- [ ] Statistical forecasting (Normal, LogNormal)
- [ ] Dynamic metric registry (JSON)
- [ ] Capital structure integration
- [ ] Extension plugin system
- [ ] Python bindings (separate doc)
- [ ] WASM bindings (separate doc)
- [ ] 200+ passing tests
- [ ] Complete documentation
- [ ] Performance benchmarks met

**Timeline:** Week 1-10

---

## Risk Mitigation

### Technical Risks

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| DSL parser complexity | High | Medium | Start simple, iterate; use nom for robust parsing |
| Expression engine limitations | High | Low | Verify core expr supports all needed functions first |
| Capital structure integration | Medium | Medium | Prototype integration in Phase 0; validate with valuations team |
| Performance regression | Medium | Low | Benchmark early and often; optimize hot paths |
| Registry JSON schema drift | Medium | Medium | Version JSON schema; validate on load |

### Dependency Risks

| Dependency | Risk | Mitigation |
|------------|------|------------|
| finstack-core | API changes | Pin version, coordinate releases |
| finstack-valuations | Breaking changes | Use optional feature, isolate integration |
| Polars | Version compatibility | Use core's re-export, test integration |

---

## Success Metrics

### Code Quality
- [ ] 90%+ test coverage
- [ ] Zero clippy warnings
- [ ] All public APIs documented
- [ ] Examples for every feature

### Performance
- [ ] 100 nodes × 24 periods < 10ms
- [ ] 1000 nodes × 60 periods < 100ms
- [ ] 10k nodes × 120 periods < 1s

### Usability
- [ ] Can build simple model in < 10 lines
- [ ] Clear error messages with suggestions
- [ ] Documentation covers common use cases

---

## Communication Plan

**Weekly Updates:**
- Team meeting: Friday 2pm
- Status: "Completed Phase X.Y, starting X.Z"

**Blockers:**
- Slack #finstack-dev
- Tag @team-lead for urgent issues

**Design Discussions:**
- GitHub Discussions for proposals
- RFC for breaking changes

**Code Reviews:**
- Max 24h turnaround for PRs < 500 lines
- All PRs require 1 approval minimum

---

## Next Steps

1. **Week 1:** Start Phase 1.1 (Crate Bootstrap)
2. **Week 1:** Set up CI/CD pipeline
3. **Week 2:** Complete Phase 1, start Phase 2
4. **Ongoing:** Update this doc with actuals vs estimates

---

## References

- [Architecture](./ARCHITECTURE.md)
- [API Reference](./API_REFERENCE.md)
- [Testing Strategy](./TESTING_STRATEGY.md)
- [Original PRD](../04_statements_prd.md)
