# Design: Split `finstack-statements` into Core + Analytics

**Date:** 2026-03-21
**Status:** Draft

## Problem

The `finstack-statements` crate combines two distinct responsibilities: the core financial statement evaluation engine (types, DSL, evaluator, forecasting, registry) and higher-level analytics that sit on top (sensitivity analysis, scenario sets, DCF, templates, scoring extensions). Separating these into two crates improves modularity, reduces compile times for consumers that only need the engine, and clarifies the API boundary.

## Decision

Split into two crates using a feature-flag re-export for backwards compatibility.

## Architecture

### Crate 1: `finstack-statements` (Core Engine)

Retains all modules needed to define, build, and evaluate financial models:

| Module | Purpose |
|--------|---------|
| `types/` | Wire types: `NodeSpec`, `FinancialModelSpec`, `NodeId`, `AmountOrScalar` |
| `builder/` | Type-safe `ModelBuilder` DSL with compile-time state enforcement |
| `dsl/` | Formula parser (text → AST → `core::Expr`) |
| `evaluator/` | DAG dependency resolution, topological sort, precedence (Value > Forecast > Formula), Monte Carlo engine, DataFrame export |
| `forecast/` | Deterministic and statistical projection methods |
| `registry/` | Dynamic metric registry with `fin.*` built-ins |
| `capital_structure/` | Debt/equity modeling with `finstack-valuations` integration |
| `adjustments/` | EBITDA normalization engine with add-back tracking |
| `extensions/plugin.rs` | `Extension` trait, `ExtensionContext`, `ExtensionResult`, `ExtensionMetadata`, `ExtensionStatus` |
| `extensions/registry.rs` | `ExtensionRegistry` (register, execute, lifecycle) |
| `error.rs` | Error types |
| `prelude.rs` | Re-exports (conditionally includes analytics when feature enabled) |
| `utils/` | Constants, formula helpers, graph utilities |

### Crate 2: `finstack-statements-analytics`

All higher-level analysis, reporting, and concrete extension implementations:

| Module | Purpose |
|--------|---------|
| `analysis/corporate.rs` | DCF valuation |
| `analysis/sensitivity.rs` | Parameter sensitivity and tornado charts |
| `analysis/goal_seek.rs` | Target-seeking optimization |
| `analysis/scenario_set.rs` | Multi-scenario comparison |
| `analysis/variance.rs` | Variance bridge analysis |
| `analysis/monte_carlo.rs` | High-level Monte Carlo config and results |
| `analysis/covenants.rs` | Covenant breach detection |
| `analysis/backtesting.rs` | Forecast accuracy evaluation |
| `analysis/introspection.rs` | Dependency tracing and formula explanation |
| `analysis/credit_context.rs` | Credit metrics computation |
| `analysis/reports.rs` | Formatted output generation |
| `analysis/orchestrator.rs` | Unified analysis pipeline |
| `analysis/types.rs` | Shared analytics types |
| `extensions/corkscrew.rs` | Balance sheet roll-forward validation |
| `extensions/scorecards.rs` | Credit rating assignment |
| `templates/` | Real estate, roll-forward, vintage model templates |

### Dependency Graph

```
finstack-core
    │
    ▼
finstack-statements (core engine)
    │
    ├──────────────────────────────────────┐
    ▼                                      ▼
finstack-statements-analytics    finstack-scenarios
    │                                      │
    └──────────────┬───────────────────────┘
                   ▼
           finstack-portfolio
                   │
                   ▼
              finstack (aggregator)
```

## Backwards Compatibility

`finstack-statements` gains a new optional feature `"analytics"`:

```toml
# finstack-statements/Cargo.toml
[features]
analytics = ["dep:finstack-statements-analytics"]

[dependencies]
finstack-statements-analytics = { path = "../statements-analytics", optional = true }
```

When enabled, core re-exports the analytics modules:

```rust
// finstack-statements/src/lib.rs
#[cfg(feature = "analytics")]
pub use finstack_statements_analytics::analysis;

#[cfg(feature = "analytics")]
pub use finstack_statements_analytics::templates;
```

The prelude conditionally includes analytics types:

```rust
// finstack-statements/src/prelude.rs
#[cfg(feature = "analytics")]
pub use finstack_statements_analytics::prelude::*;
```

The `finstack` aggregator enables the feature by default:

```toml
# finstack/Cargo.toml
statements = ["core", "dep:finstack_statements", "finstack_statements/analytics", "dep:indexmap"]
```

This means existing `use finstack_statements::analysis::*` paths continue to work for all current consumers. Consumers who only need the core engine can depend on `finstack-statements` without the `"analytics"` feature.

## New Crate Structure

```
finstack/statements-analytics/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── prelude.rs
│   ├── analysis/
│   │   ├── mod.rs
│   │   ├── backtesting.rs
│   │   ├── corporate.rs
│   │   ├── covenants.rs
│   │   ├── credit_context.rs
│   │   ├── goal_seek.rs
│   │   ├── introspection.rs
│   │   ├── monte_carlo.rs
│   │   ├── orchestrator.rs
│   │   ├── reports.rs
│   │   ├── scenario_set.rs
│   │   ├── sensitivity.rs
│   │   ├── types.rs
│   │   └── variance.rs
│   ├── extensions/
│   │   ├── mod.rs
│   │   ├── corkscrew.rs
│   │   └── scorecards.rs
│   └── templates/
│       ├── mod.rs
│       ├── builder.rs
│       ├── real_estate.rs
│       ├── roll_forward.rs
│       └── vintage.rs
└── tests/
    └── (migrated analytics tests)
```

### Cargo.toml

```toml
[package]
name = "finstack-statements-analytics"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
finstack-core = { path = "../core", features = ["golden"] }
finstack-statements = { path = "../statements" }
finstack-valuations = { path = "../valuations" }
serde = { workspace = true }
serde_json = { workspace = true }
indexmap = { workspace = true, features = ["serde"] }
thiserror = { workspace = true }
time = { workspace = true }
rust_decimal = { workspace = true }

[features]
default = []
dataframes = ["finstack-statements/dataframes"]
parallel = ["finstack-statements/parallel"]
```

## Extension Trait Split

The `Extension` trait and supporting types stay in `finstack-statements` under `extensions/`:
- `plugin.rs`: `Extension`, `ExtensionContext`, `ExtensionResult`, `ExtensionMetadata`, `ExtensionStatus`
- `registry.rs`: `ExtensionRegistry`

Concrete implementations move to `finstack-statements-analytics`:
- `corkscrew.rs`: `CorkscrewExtension`, `CorkscrewConfig`, `CorkscrewAccount`, `AccountType`
- `scorecards.rs`: `CreditScorecardExtension`, `ScorecardConfig`, `ScorecardMetric`

The analytics crate imports the trait from core:
```rust
use finstack_statements::extensions::{Extension, ExtensionContext, ExtensionResult};
```

## Import Path Changes

For consumers using the re-export feature (default), no changes needed:
```rust
// Still works with analytics feature enabled
use finstack_statements::analysis::CorporateAnalysisBuilder;
use finstack_statements::prelude::*;
```

For consumers importing directly from the new crate:
```rust
use finstack_statements_analytics::analysis::CorporateAnalysisBuilder;
```

## Test Migration

Tests are categorized by which crate they belong to:

**Stay in `finstack-statements`:** All tests for core modules — evaluator, DSL, forecast, registry, builder, capital structure, types, adjustments, extension trait/registry.

**Move to `finstack-statements-analytics`:** Tests for analysis modules (corporate, goal seek, monte carlo config, orchestrator, scenario set), extension implementations (corkscrew, scorecards), and templates.

**Integration tests** that span both (e.g., build a model, evaluate it, run analysis) live in the analytics crate since they need both dependencies.

## Workspace Changes

1. Add `"finstack/statements-analytics"` to `[workspace.members]` in root `Cargo.toml`
2. Add `finstack-statements-analytics` to `[workspace.dependencies]` if using workspace dep management
3. Update `finstack` aggregator features to forward the analytics feature
4. No changes needed to `finstack-scenarios` or `finstack-portfolio` — they only import core types

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Circular dependency | Not possible — analytics depends on core, never the reverse |
| Breaking downstream | Feature-flag re-export preserves all existing import paths |
| Test coverage gap | Run full test suite after migration to verify nothing dropped |
| Compile time regression | Feature re-export adds one extra crate to the dep graph when analytics is enabled, but consumers who only need core get faster builds |
