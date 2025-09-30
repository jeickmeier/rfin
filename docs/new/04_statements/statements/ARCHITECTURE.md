# Statements Crate — Architecture & Design

**Last updated:** 2025-09-30

---

## Table of Contents

1. [Dependency Graph](#1-dependency-graph)
2. [Integration Points](#2-integration-points)
3. [Crate Structure](#3-crate-structure)
4. [Key Design Decisions](#4-key-design-decisions)
5. [Evaluation Flow](#5-evaluation-flow)
6. [Cross-Cutting Concerns](#6-cross-cutting-concerns)

---

## 1. Dependency Graph

```
┌─────────────────────────────────────────────┐
│         finstack-statements                 │
│                                             │
│  ┌─────────────────────────────────────┐   │
│  │  Builder (Type-State Pattern)       │   │
│  │  - ModelBuilder<NeedPeriods>        │   │
│  │  - ModelBuilder<Ready>              │   │
│  └──────────────┬──────────────────────┘   │
│                 │                           │
│  ┌──────────────▼──────────────────────┐   │
│  │  Core Types (Wire + Runtime)        │   │
│  │  - NodeSpec, FinancialModelSpec     │   │
│  │  - AmountOrScalar                   │   │
│  └──────────────┬──────────────────────┘   │
│                 │                           │
│  ┌──────────────▼──────────────────────┐   │
│  │  Evaluator                          │   │
│  │  - DAG construction                 │   │
│  │  - Precedence resolution            │   │
│  │  - Per-period evaluation            │   │
│  └──────────────┬──────────────────────┘   │
│                 │                           │
│  ┌──────────────▼──────────────────────┐   │
│  │  DSL Engine                         │   │
│  │  - Parser (formula_text → AST)      │   │
│  │  - Time-series operators            │   │
│  │  - Statistical functions            │   │
│  └──────────────┬──────────────────────┘   │
│                 │                           │
│  ┌──────────────▼──────────────────────┐   │
│  │  Forecast Methods                   │   │
│  │  - ForwardFill, GrowthPct           │   │
│  │  - Statistical (Normal, etc.)       │   │
│  └──────────────┬──────────────────────┘   │
│                 │                           │
│  ┌──────────────▼──────────────────────┐   │
│  │  Dynamic Registry (JSON)            │   │
│  │  - Load metrics from JSON           │   │
│  │  - Namespace management (fin.*)     │   │
│  └──────────────┬──────────────────────┘   │
│                 │                           │
│  ┌──────────────▼──────────────────────┐   │
│  │  Extension Plugins                  │   │
│  │  - Extension trait                  │   │
│  │  - Plugin registry                  │   │
│  └─────────────────────────────────────┘   │
└─────────────────────────────────────────────┘
         ▲                      ▲
         │                      │
    ┌────┴─────┐         ┌─────┴──────────┐
    │  core/   │         │  valuations/   │
    │  - Period│         │  - Instruments │
    │  - Expr  │         │  - Cashflow    │
    │  - Money │         │  - Aggregation │
    └──────────┘         └────────────────┘
```

---

## 2. Integration Points

### 2.1 From `finstack-core`

| Component | Purpose | Usage |
|-----------|---------|-------|
| `Period`, `PeriodPlan`, `PeriodId` | Period system | Model time axis, period parsing |
| `Money`, `Currency` | Currency-safe amounts | Node values, aggregation |
| `Date`, `DayCount`, `BusinessDayConvention` | Date utilities | Period boundaries, day fractions |
| `Expr`, `CompiledExpr`, `Function` | Expression AST | Formula compilation & evaluation |
| `ExpressionContext` | Evaluation context trait | Statement context implementation |
| `ResultsMeta`, `FinstackConfig` | Metadata stamping | Result provenance tracking |
| Polars `DataFrame`/`Series` | Vectorization | Result exports, bulk operations |

### 2.2 From `finstack-valuations` (Optional)

| Component | Purpose | Usage |
|-----------|---------|-------|
| Instrument types | Capital structure | Bond, Swap, Loan modeling |
| `aggregate_by_period` | Cashflow aggregation | Period-aligned cashflows |
| `CashflowBuilder` | Debt schedules | Amortization, interest schedules |
| Metric calculation | Financial metrics | Interest expense, principal payments |

### 2.3 Extension Points (Future)

- ⚠️ **Corkscrew schedules** - Roll-forward analysis for balance sheets
- ⚠️ **Credit scorecards** - Rating-based stress testing
- ⚠️ **Real estate** - Property cashflows, equity waterfalls
- ⚠️ **Portfolio aggregation** - Multi-entity consolidation

---

## 3. Crate Structure

```
finstack/statements/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs                    # Public API surface
│   ├── error.rs                  # Typed error hierarchy
│   │
│   ├── types/
│   │   ├── mod.rs
│   │   ├── node.rs               # NodeSpec, Node, NodeType
│   │   ├── value.rs              # AmountOrScalar, ValueType
│   │   ├── forecast.rs           # ForecastSpec, ForecastMethod
│   │   ├── model.rs              # FinancialModelSpec, FinancialModel
│   │   └── capital_structure.rs  # CapitalStructureSpec
│   │
│   ├── dsl/
│   │   ├── mod.rs
│   │   ├── parser.rs             # Formula text → AST
│   │   ├── ast.rs                # Statements DSL AST
│   │   ├── operators.rs          # Time-series operators
│   │   ├── functions.rs          # Built-in functions
│   │   └── compiler.rs           # AST → CompiledExpr
│   │
│   ├── builder/
│   │   ├── mod.rs
│   │   ├── model_builder.rs      # Type-state builder pattern
│   │   ├── node_builder.rs       # Node helper builders
│   │   └── capital_builder.rs    # Capital structure helpers
│   │
│   ├── evaluator/
│   │   ├── mod.rs
│   │   ├── evaluator.rs          # Main orchestrator
│   │   ├── context.rs            # StatementContext
│   │   ├── precedence.rs         # Value > Forecast > Formula
│   │   ├── dag.rs                # Dependency graph
│   │   └── capital_integration.rs # Cashflow aggregation
│   │
│   ├── forecast/
│   │   ├── mod.rs
│   │   ├── deterministic.rs      # ForwardFill, GrowthPct
│   │   ├── statistical.rs        # Normal, LogNormal
│   │   ├── time_series.rs        # Curve growth
│   │   └── override.rs           # Explicit overrides
│   │
│   ├── registry/
│   │   ├── mod.rs
│   │   ├── dynamic.rs            # JSON-based loader
│   │   ├── builtins.rs           # Embedded fin.* metrics
│   │   ├── schema.rs             # JSON schema
│   │   └── validation.rs         # Registry validation
│   │
│   ├── extensions/
│   │   ├── mod.rs
│   │   ├── plugin.rs             # Extension trait & registry
│   │   ├── corkscrew.rs          # Roll-forward validation
│   │   └── scorecards.rs         # Credit scorecards
│   │
│   ├── results/
│   │   ├── mod.rs
│   │   ├── results.rs            # Results struct
│   │   ├── export.rs             # DataFrame exports
│   │   └── metadata.rs           # Result metadata
│   │
│   └── validation/
│       ├── mod.rs
│       └── checks.rs             # Model validation rules
│
├── tests/
│   ├── builder_tests.rs
│   ├── evaluator_tests.rs
│   ├── dsl_tests.rs
│   ├── forecast_tests.rs
│   ├── registry_tests.rs
│   ├── capital_structure_tests.rs
│   ├── integration_tests.rs
│   └── golden/
│       ├── basic_model.json
│       ├── capital_structure_model.json
│       └── statistical_forecast_model.json
│
└── data/
    └── metrics/
        ├── fin_basic.json        # Basic financial metrics
        ├── fin_margins.json      # Margin calculations
        ├── fin_returns.json      # Return metrics
        └── fin_leverage.json     # Leverage ratios
```

---

## 4. Key Design Decisions

### 4.1 Type-State Builder Pattern

**Decision:** Use compile-time type states to enforce correct builder usage.

**Rationale:**
- Prevents invalid states (e.g., adding nodes before defining periods)
- Better ergonomics than runtime validation
- Zero runtime overhead

**Example:**
```rust
let model = ModelBuilder::new("test")
    .periods("2025Q1..Q4", None)?  // Returns ModelBuilder<Ready>
    .value("revenue", &[...])?     // Only available after .periods()
    .build()?;
```

### 4.2 Precedence System: Value > Forecast > Formula

**Decision:** Fixed precedence hierarchy for node resolution.

**Rationale:**
- **Value** (actuals) always win - never override real data
- **Forecast** overrides formulas in future periods - explicit forecasts beat implicit calculations
- **Formula** is fallback - always computable

**Example:**
```rust
// Q1 has actual value (100) → use Value
// Q2 has no actual, but has forecast (110) → use Forecast
// Q3 has no actual or forecast → use Formula
.value("revenue", &[(Q1, 100.0)])
.forecast("revenue", GrowthPct { rate: 0.1 })
.compute("revenue", "lag(revenue, 1) * 1.05")  // Fallback formula
```

### 4.3 Statements DSL vs Core Expr

**Decision:** Statements DSL compiles to core `Expr`, not separate execution engine.

**Rationale:**
- Reuse battle-tested expression evaluation from core
- Single point of optimization
- Consistent behavior across crates

**Flow:**
```
formula_text → [Parser] → StmtExpr → [Compiler] → core::Expr → [Evaluator] → value
```

### 4.4 Dynamic Registry (JSON-based)

**Decision:** Metrics defined in JSON, not Rust code.

**Rationale:**
- Analysts can add metrics without recompiling
- Versioned metric libraries (financial standards)
- Easy to share across teams
- No macros or DSL macros needed

**Example:**
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

### 4.5 Capital Structure as Optional Feature

**Decision:** Capital structure behind feature flag `capital_structure`.

**Rationale:**
- Not all models need debt tracking
- Reduces compile time for simple use cases
- Allows valuations crate to evolve independently

---

## 5. Evaluation Flow

### 5.1 Model Construction

```
1. User calls ModelBuilder::new("Model ID")
2. User calls .periods("2025Q1..Q4", Some("2025Q1..Q2"))
   → Parses periods using core's build_periods()
   → Marks Q1-Q2 as actuals, Q3-Q4 as forecast
3. User adds nodes:
   .value("revenue", &[(Q1, 100.0)])
   .forecast("revenue", GrowthPct { rate: 0.05 })
   .compute("cogs", "revenue * 0.6")
4. User calls .build()
   → Compiles all formulas (DSL → core Expr)
   → Builds dependency graph (DAG)
   → Validates no circular dependencies
   → Returns FinancialModel
```

### 5.2 Evaluation

```
1. User calls evaluator.evaluate(&model, parallel: false)
2. Evaluator loops over each period in sequence:
   For period P:
     a. Create StatementContext for P
        - Includes all prior period results
        - Maps node_id → column index
     b. For each node in topological order:
        - Resolve node value using precedence:
          * If explicit value exists → use Value
          * Else if forecast applicable → use Forecast
          * Else if formula exists → use Formula
          * Else → error (undefined node)
        - Store result: results[node_id][period_id] = value
3. Return Results struct with all period × node values
```

### 5.3 Export

```
1. User calls results.to_polars_long()
   → Converts results to DataFrame:
      node_id | period_id | value
      --------|-----------|-------
      revenue | 2025Q1    | 100.0
      revenue | 2025Q2    | 105.0
      cogs    | 2025Q1    | 60.0
      ...

2. Or user calls results.to_polars_wide()
   → Converts results to wide format:
      period_id | revenue | cogs | gross_profit
      ----------|---------|------|-------------
      2025Q1    | 100.0   | 60.0 | 40.0
      2025Q2    | 105.0   | 63.0 | 42.0
```

---

## 6. Cross-Cutting Concerns

### 6.1 Determinism

**Guarantee:** Same model + same input + same seed → same results.

**Implementation:**
- Use `IndexMap` for deterministic iteration order
- Statistical forecasts require explicit `seed` parameter
- No `HashMap` in hot paths
- Topological sort is stable (consistent tiebreaking)

### 6.2 Currency Safety

**Guarantee:** No implicit cross-currency arithmetic.

**Implementation:**
- `AmountOrScalar::Amount` carries currency
- Operations on different currencies require explicit FX provider
- `Money` arithmetic enforces same-currency rule
- Results metadata tracks FX policies applied

### 6.3 Error Context

**Principle:** Errors must be actionable.

**Implementation:**
- Include node_id, period_id, formula_text in errors
- Suggest fixes where possible
- Rich error types (not strings)

**Example:**
```
Error: Unknown node reference in formula
Node: gross_margin
Formula: "gross_profit / revenu"
                          ^^^^^^^
Error: Unknown node 'revenu'
Hint: Did you mean 'revenue'?
Available nodes: revenue, cogs, gross_profit
```

### 6.4 Performance Considerations

**Goals:**
- 100 nodes × 24 periods < 10ms
- 1000 nodes × 60 periods < 100ms
- 10k nodes × 120 periods < 1s

**Strategies:**
- Compile formulas once (not per period)
- Topological sort once (not per period)
- Vectorized evaluation where possible (future)
- Lazy evaluation for selective outputs (future)

### 6.5 Observability

**Instrumentation:**
- `tracing` spans for major operations
  - `evaluate_model`
  - `evaluate_period`
  - `evaluate_node`
- Metadata in results (evaluation time, node count, etc.)
- Debug mode with detailed logs

---

## 7. Comparison with Alternatives

### vs Python Implementation

| Aspect | Python | Rust (This Crate) |
|--------|--------|-------------------|
| Type safety | Runtime checks | Compile-time + runtime |
| Performance | ~10 nodes/ms | ~100 nodes/ms |
| Determinism | Opt-in | Default |
| Currency safety | Manual | Enforced by type system |
| DSL | Flexible (eval) | Constrained (parsed) |

### vs Excel

| Aspect | Excel | Rust (This Crate) |
|--------|-------|-------------------|
| Version control | Poor (binary format) | Excellent (JSON/code) |
| Programmatic | VBA only | Full Rust API |
| Scale | ~1k formulas max | 10k+ formulas |
| Reproducibility | Manual | Built-in |

---

## 8. Future Enhancements

### Short-term (v0.2-v0.3)
- Vectorized evaluation (evaluate all periods at once)
- Parallel evaluation (Rayon)
- Caching for expensive computations

### Medium-term (v0.4-v1.0)
- Corkscrew extension implementation
- Credit scorecard extension
- Real-time formula validation
- Incremental evaluation (only changed nodes)

### Long-term (v1.x+)
- Constraint solving (balance sheet articulation)
- Optimization (goal seek, solver integration)
- Probabilistic modeling (Monte Carlo)
- Distributed evaluation (multi-machine)

---

## 9. References

- **Core Documentation:** [`finstack/core/`](../../core/)
- **Valuations Documentation:** [`finstack/valuations/`](../../valuations/)
- **Statements PRD:** [04_statements_prd.md](../04_statements_prd.md)
- **Statements TDD:** [04_statements_tdd.md](../04_statements_tdd.md)
