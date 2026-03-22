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
finstack-valuations
    │
    ├────────────────────────────────────────────┐
    ▼                                            │
finstack-statements (core engine)                │
    │                                            │
    ├──────────────────────────────────────┐      │
    ▼                                      ▼     │
finstack-statements-analytics ◄────────── (also depends on valuations)
    │                          finstack-scenarios
    │                                      │
    └──────────────┬───────────────────────┘
                   ▼
           finstack-portfolio
                   │
                   ▼
              finstack (aggregator)
                   │
                   ├──► finstack-py
                   └──► finstack-wasm
```

Note: Both `finstack-statements` and `finstack-statements-analytics` depend on `finstack-valuations` (diamond dependency, not circular).

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

The `extensions/mod.rs` conditionally re-exports concrete extension types:

```rust
// finstack-statements/src/extensions/mod.rs
mod plugin;
mod registry;

pub use plugin::{Extension, ExtensionContext, ExtensionMetadata, ExtensionResult, ExtensionStatus};
pub use registry::ExtensionRegistry;

// Re-export concrete extension impls from analytics when available
#[cfg(feature = "analytics")]
pub use finstack_statements_analytics::extensions::{
    AccountType, CorkscrewAccount, CorkscrewConfig, CorkscrewExtension,
    CreditScorecardExtension, ScorecardConfig, ScorecardMetric,
};
```

The prelude conditionally includes analytics types:

```rust
// finstack-statements/src/prelude.rs

// Core-only prelude items (always available)
pub use crate::builder::{MixedNodeBuilder, ModelBuilder, NeedPeriods, Ready};
pub use crate::error::{Error, Result};
pub use crate::evaluator::{Evaluator, EvaluatorWithContext, NumericMode, StatementResult};
pub use crate::extensions::{Extension, ExtensionContext, ExtensionMetadata, ExtensionRegistry, ExtensionResult, ExtensionStatus};
pub use crate::registry::Registry;
pub use crate::types::{AmountOrScalar, FinancialModelSpec, ForecastMethod, ForecastSpec, NodeId, NodeSpec, NodeType, NodeValueType, SeasonalMode};
pub use finstack_core::prelude::*;
pub use finstack_core::dates::{build_periods, Period, PeriodId, PeriodKind};

// Analytics prelude (available when feature enabled)
#[cfg(feature = "analytics")]
pub use finstack_statements_analytics::prelude::*;
```

The analytics crate's `prelude.rs` exports:

```rust
// finstack-statements-analytics/src/prelude.rs
pub use crate::analysis::{
    BridgeChart, BridgeStep, CorporateAnalysis, CorporateAnalysisBuilder,
    CreditInstrumentAnalysis, MonteCarloConfig, MonteCarloResults,
    ScenarioDefinition, ScenarioDiff, ScenarioResults, ScenarioSet,
    VarianceAnalyzer, VarianceConfig, VarianceReport, VarianceRow,
};
pub use crate::extensions::{CorkscrewExtension, CreditScorecardExtension};
pub use crate::templates::{RealEstateExtension, TemplatesExtension, VintageExtension};
```

The `finstack` aggregator enables the feature by default:

```toml
# finstack/Cargo.toml
statements = ["core", "dep:finstack_statements", "finstack_statements/analytics", "dep:indexmap"]
```

This means existing `use finstack_statements::analysis::*` paths continue to work for all current consumers. Consumers who only need the core engine can depend on `finstack-statements` without the `"analytics"` feature.

## ModelBuilder Visibility

The `templates/` module currently accesses `ModelBuilder`'s `pub(crate) nodes` field directly in `roll_forward.rs` and `vintage.rs`. This prevents a clean move to an external crate.

**Resolution:** Add a public method to `ModelBuilder` for inserting raw nodes:

```rust
// finstack-statements/src/builder/model_builder.rs
impl<State> ModelBuilder<State> {
    /// Insert a pre-built node into the model.
    ///
    /// This is an advanced API for template builders that need to construct
    /// nodes programmatically. Prefer `.compute()` and `.value()` for
    /// standard model construction.
    pub fn insert_node(&mut self, id: NodeId, spec: NodeSpec) -> &mut Self {
        self.nodes.insert(id, spec);
        self
    }
}
```

The templates code then changes from:
```rust
builder.nodes.insert(NodeId::from(name), node);
```
to:
```rust
builder.insert_node(NodeId::from(name), node);
```

All other `ModelBuilder` methods called by templates (`.compute()`, `.value()`) are already `pub`.

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
license.workspace = true
repository.workspace = true
authors.workspace = true

[dependencies]
finstack-core = { path = "../core", features = ["golden"] }
finstack-statements = { path = "../statements" }
finstack-valuations = { path = "../valuations" }
serde = { workspace = true }
serde_json = { workspace = true }
indexmap = { workspace = true, features = ["serde"] }
thiserror = { workspace = true }
rust_decimal = { workspace = true }

[features]
default = []
dataframes = ["finstack-statements/dataframes"]
parallel = ["finstack-statements/parallel"]
```

Dependencies that stay in core only: `nom` (DSL parser), `tracing`, `polars` (optional), `rayon` (optional). The analytics crate does not need `time` directly — it gets date types transitively through `finstack-core` and `finstack-statements`.

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

## Downstream Consumer Updates

### `finstack-scenarios` and `finstack-portfolio`

No changes needed — they only import core types (`NodeId`, `FinancialModelSpec`, `Evaluator`, etc.).

### `finstack-py`

Heavy user of analytics. Currently imports from `finstack_statements::analysis::*`, `finstack_statements::extensions::*`, and `finstack_statements::templates::*`. Two options:

1. **Preferred:** Add `finstack_statements/analytics` feature to its dependency — all existing import paths continue to work via re-exports.
2. **Alternative:** Add a direct dependency on `finstack-statements-analytics` and update import paths.

Option 1 requires zero code changes in `finstack-py`.

### `finstack-wasm`

Uses `DependencyTracer`, `DependencyTree` from analysis, `CorkscrewExtension` from extensions, and `evaluate_dcf_with_market` from `analysis::corporate`. Same approach as `finstack-py` — enable the `analytics` feature.

### `finstack` aggregator

The `statements` feature already depends on `finstack_statements`. Update to also enable the analytics feature:

```toml
statements = ["core", "dep:finstack_statements", "finstack_statements/analytics", "dep:indexmap"]
```

This automatically makes analytics available to all consumers that use the aggregator with the `statements` feature.

## Import Path Changes

For consumers using the re-export feature (default for all current consumers), no changes needed:
```rust
// Still works with analytics feature enabled
use finstack_statements::analysis::CorporateAnalysisBuilder;
use finstack_statements::extensions::CorkscrewExtension;
use finstack_statements::templates::TemplatesExtension;
use finstack_statements::prelude::*;
```

For consumers importing directly from the new crate:
```rust
use finstack_statements_analytics::analysis::CorporateAnalysisBuilder;
```

## Doc-Test Migration

Source files moving to the analytics crate have doc-tests using `crate::` paths and `use finstack_statements::` paths. These must be updated:

- `crate::analysis::*` → stays as `crate::analysis::*` (now refers to the analytics crate)
- `crate::evaluator::*` → `finstack_statements::evaluator::*` (core types)
- `crate::types::*` → `finstack_statements::types::*` (core types)
- `use finstack_statements::prelude::*` in doc examples → `use finstack_statements::prelude::*` (still works if analytics feature is enabled for doc-tests)

The analytics crate's `[dev-dependencies]` should include `finstack-statements` with `analytics` feature for integration doc-tests.

## Test Migration

Tests are categorized by which crate they belong to:

**Stay in `finstack-statements`:** All tests for core modules — evaluator, DSL, forecast, registry, builder, capital structure, types, adjustments, extension trait/registry.

**Move to `finstack-statements-analytics`:** Tests for analysis modules (corporate, goal seek, monte carlo config, orchestrator, scenario set), extension implementations (corkscrew, scorecards), and templates.

**Integration tests** that span both (e.g., build a model, evaluate it, run analysis) live in the analytics crate since they need both dependencies.

## Workspace Changes

1. Add `"finstack/statements-analytics"` to `[workspace.members]` and default members in root `Cargo.toml`
2. Update `finstack` aggregator `statements` feature to enable `finstack_statements/analytics`
3. Update `finstack-py` and `finstack-wasm` to enable `finstack_statements/analytics` feature
4. No changes needed to `finstack-scenarios` or `finstack-portfolio`

## Migration Order

1. Add `insert_node()` public method to `ModelBuilder` and update templates to use it (preparation while still in one crate)
2. Create `finstack/statements-analytics/` with `Cargo.toml` and `src/lib.rs`
3. Move `analysis/` files, update `crate::` paths to `finstack_statements::` for core types
4. Move `extensions/corkscrew.rs` and `extensions/scorecards.rs`
5. Move `templates/` files, update `crate::builder` → `finstack_statements::builder`
6. Update `extensions/mod.rs` in core: remove concrete impls, add `#[cfg(feature = "analytics")]` re-exports
7. Update `prelude.rs` in core: split into always-available and `#[cfg(feature = "analytics")]` sections
8. Add `analytics` feature to core `Cargo.toml` with optional dep on `finstack-statements-analytics`
9. Move analytics tests to the new crate
10. Update `finstack-py`, `finstack-wasm`, and `finstack` aggregator dependencies
11. Update workspace `Cargo.toml` members
12. Run full test suite (`cargo test --workspace`)
13. Update doc-tests in moved files

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Circular dependency | Not possible — analytics depends on core, never the reverse |
| Breaking downstream | Feature-flag re-export preserves all existing import paths |
| `ModelBuilder` visibility | Add `insert_node()` public method before moving templates |
| Extension re-export gap | Conditional re-export in `extensions/mod.rs` covers concrete types |
| Test coverage gap | Run full test suite after migration to verify nothing dropped |
| Doc-test breakage | Update `crate::` paths in moved files; analytics uses `finstack-statements` as dev-dep |
| Compile time regression | Feature re-export adds one extra crate to the dep graph when analytics is enabled, but consumers who only need core get faster builds |
