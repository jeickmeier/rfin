# Statements Crate Split Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split `finstack-statements` into a core engine crate and a new `finstack-statements-analytics` crate, with feature-flag re-exports for backwards compatibility.

**Architecture:** The core crate retains types, builder, DSL, evaluator, forecast, registry, capital structure, adjustments, and the extension trait/registry. The new analytics crate gets analysis, templates, and concrete extension implementations (corkscrew, scorecards). A `"analytics"` feature on core re-exports analytics modules so existing import paths keep working.

**Tech Stack:** Rust workspace, Cargo features, conditional compilation (`#[cfg(feature = "...")]`)

**Spec:** `docs/superpowers/specs/2026-03-21-statements-crate-split-design.md`

---

### Task 1: Add `insert_node()` public method to `ModelBuilder`

**Files:**
- Modify: `finstack/statements/src/builder/model_builder.rs:114-119` (struct definition area)
- Modify: `finstack/statements/src/templates/roll_forward.rs:55-56`
- Modify: `finstack/statements/src/templates/vintage.rs:64`

**Context:** Templates currently access `pub(crate) nodes` directly on `ModelBuilder`. Before moving templates to an external crate, we need a public method for inserting nodes.

- [ ] **Step 1: Add `insert_node()` method to `ModelBuilder`**

In `finstack/statements/src/builder/model_builder.rs`, add this method to the `impl<State> ModelBuilder<State>` block (find the generic impl block, not the state-specific ones):

```rust
    /// Insert a pre-built node into the model.
    ///
    /// This is an advanced API for template builders that need to construct
    /// nodes programmatically. Prefer `.compute()` and `.value()` for
    /// standard model construction.
    pub fn insert_node(&mut self, id: NodeId, spec: NodeSpec) -> &mut Self {
        self.nodes.insert(id, spec);
        self
    }
```

- [ ] **Step 2: Update `roll_forward.rs` to use `insert_node()`**

In `finstack/statements/src/templates/roll_forward.rs`, change lines 55-56 from:

```rust
    builder.nodes.insert(NodeId::from(beg_node_id), beg_node);
    builder.nodes.insert(NodeId::from(end_node_id), end_node);
```

to:

```rust
    builder.insert_node(NodeId::from(beg_node_id), beg_node);
    builder.insert_node(NodeId::from(end_node_id), end_node);
```

- [ ] **Step 3: Update `vintage.rs` to use `insert_node()`**

In `finstack/statements/src/templates/vintage.rs`, change line 64 from:

```rust
    builder.nodes.insert(NodeId::from(name), node);
```

to:

```rust
    builder.insert_node(NodeId::from(name), node);
```

- [ ] **Step 4: Run tests to verify nothing broke**

Run: `cargo test -p finstack-statements`
Expected: All tests pass — behavior is identical, only the access path changed.

- [ ] **Step 5: Commit**

```bash
git add finstack/statements/src/builder/model_builder.rs finstack/statements/src/templates/roll_forward.rs finstack/statements/src/templates/vintage.rs
git commit -m "refactor: add public insert_node() to ModelBuilder for template builders"
```

---

### Task 2: Create the `finstack-statements-analytics` crate skeleton

**Files:**
- Create: `finstack/statements-analytics/Cargo.toml`
- Create: `finstack/statements-analytics/src/lib.rs`
- Create: `finstack/statements-analytics/src/prelude.rs`
- Modify: `Cargo.toml` (workspace root, lines 10-25 members, lines 29-40 default-members)

- [ ] **Step 1: Create the crate directory**

```bash
mkdir -p finstack/statements-analytics/src
```

- [ ] **Step 2: Create `Cargo.toml`**

Create `finstack/statements-analytics/Cargo.toml`:

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

[dev-dependencies]
time = { workspace = true, features = ["macros"] }
rust_decimal_macros = { workspace = true }

[features]
default = []
dataframes = ["finstack-statements/dataframes"]
parallel = ["finstack-statements/parallel"]
```

- [ ] **Step 3: Create initial `src/lib.rs`**

Create `finstack/statements-analytics/src/lib.rs`:

```rust
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![doc(test(attr(allow(clippy::expect_used))))]

//! # Finstack Statements Analytics
//!
//! Higher-level analysis, reporting, and extension implementations that build
//! on the core [`finstack_statements`] evaluation engine.
//!
//! This crate provides:
//!
//! - **Analysis** — sensitivity, scenario sets, variance, DCF, goal seek,
//!   covenants, backtesting, Monte Carlo, and introspection
//! - **Extensions** — concrete `Extension` implementations (corkscrew,
//!   credit scorecard)
//! - **Templates** — real estate, roll-forward, and vintage model builders

/// Convenient re-exports for common analytics types.
pub mod prelude;
```

- [ ] **Step 4: Create initial `src/prelude.rs`**

Create `finstack/statements-analytics/src/prelude.rs`:

```rust
//! Re-exports of the most common analytics types.
```

(Empty for now — will be populated as modules are moved.)

- [ ] **Step 5: Add to workspace members**

In the root `Cargo.toml`, add `"finstack/statements-analytics"` to `[workspace.members]` (after line 17, the `"finstack/statements"` entry) and to `default-members` (after line 35):

```toml
# In [workspace] members:
    "finstack/statements-analytics",

# In default-members:
    "finstack/statements-analytics",
```

- [ ] **Step 6: Verify the skeleton compiles**

Run: `cargo check -p finstack-statements-analytics`
Expected: Compiles successfully (empty crate with just prelude).

- [ ] **Step 7: Commit**

```bash
git add finstack/statements-analytics/ Cargo.toml
git commit -m "feat: create finstack-statements-analytics crate skeleton"
```

---

### Task 3: Move the `analysis/` module to the analytics crate

**Files:**
- Move: `finstack/statements/src/analysis/` → `finstack/statements-analytics/src/analysis/`
  - `mod.rs`, `backtesting.rs`, `corporate.rs`, `covenants.rs`, `credit_context.rs`, `goal_seek.rs`, `introspection.rs`, `monte_carlo.rs`, `orchestrator.rs`, `reports.rs`, `scenario_set.rs`, `sensitivity.rs`, `types.rs`, `variance.rs`
- Modify: `finstack/statements-analytics/src/lib.rs`
- Remove: `finstack/statements/src/analysis/` directory

**Context:** All 14 files in `analysis/` move to the analytics crate. The key change is updating `crate::` references to `finstack_statements::` for types that remain in core (evaluator, types, error, builder, forecast, registry, extensions, capital_structure, adjustments, utils).

- [ ] **Step 1: Copy the analysis module**

```bash
cp -r finstack/statements/src/analysis finstack/statements-analytics/src/analysis
```

- [ ] **Step 2: Update `crate::` imports in all analysis files**

In every file under `finstack/statements-analytics/src/analysis/`, replace `crate::` references to core modules with `finstack_statements::`. The modules that remain in core are: `evaluator`, `types`, `error`, `builder`, `forecast`, `registry`, `extensions`, `capital_structure`, `adjustments`, `utils`, `dsl`.

Common replacements across all files:
- `crate::evaluator` → `finstack_statements::evaluator`
- `crate::types` → `finstack_statements::types`
- `crate::error` → `finstack_statements::error`
- `crate::builder` → `finstack_statements::builder`
- `crate::forecast` → `finstack_statements::forecast`
- `crate::extensions` → `finstack_statements::extensions`
- `crate::capital_structure` → `finstack_statements::capital_structure`
- `crate::registry` → `finstack_statements::registry`
- `crate::adjustments` → `finstack_statements::adjustments`
- `crate::dsl` → `finstack_statements::dsl`
- `crate::utils` → `finstack_statements::utils`
- `crate::Error` → `finstack_statements::Error`
- `crate::Result` → `finstack_statements::Result`

References within analysis itself (`crate::analysis::*`) stay as `crate::analysis::*` since analysis is now local to the analytics crate.

Also update any doc-test `use` statements:
- `use finstack_statements::analysis::*` → keep as-is (will work via re-export)
- `use crate::analysis::*` → stays as-is (local to analytics crate)
- Doc references like `[`crate::analysis::CorporateAnalysisBuilder`]` → `[`CorporateAnalysisBuilder`](crate::analysis::CorporateAnalysisBuilder)` (stays local)
- Doc references to core types like `[`crate::evaluator::StatementResult`]` → `[`StatementResult`](finstack_statements::evaluator::StatementResult)`

- [ ] **Step 3: Register the analysis module in `lib.rs`**

In `finstack/statements-analytics/src/lib.rs`, add:

```rust
/// Analysis tools for financial statement models.
pub mod analysis;
```

- [ ] **Step 4: Remove the analysis module from core**

Remove the `finstack/statements/src/analysis/` directory entirely.

In `finstack/statements/src/lib.rs`, remove lines 64-65:

```rust
/// Analysis helpers and post-processing utilities.
pub mod analysis;
```

- [ ] **Step 5: Check compilation of the analytics crate**

Run: `cargo check -p finstack-statements-analytics`
Expected: Compiles. Fix any remaining `crate::` references that should be `finstack_statements::`.

- [ ] **Step 6: Commit**

```bash
git add finstack/statements-analytics/src/analysis/ finstack/statements-analytics/src/lib.rs
git add finstack/statements/src/lib.rs
git rm -r finstack/statements/src/analysis/
git commit -m "refactor: move analysis module to finstack-statements-analytics"
```

---

### Task 4: Move concrete extension implementations to the analytics crate

**Files:**
- Move: `finstack/statements/src/extensions/corkscrew.rs` → `finstack/statements-analytics/src/extensions/corkscrew.rs`
- Move: `finstack/statements/src/extensions/scorecards.rs` → `finstack/statements-analytics/src/extensions/scorecards.rs`
- Create: `finstack/statements-analytics/src/extensions/mod.rs`
- Modify: `finstack/statements/src/extensions/mod.rs`
- Modify: `finstack/statements-analytics/src/lib.rs`

- [ ] **Step 1: Create the extensions directory and copy files**

```bash
mkdir -p finstack/statements-analytics/src/extensions
cp finstack/statements/src/extensions/corkscrew.rs finstack/statements-analytics/src/extensions/
cp finstack/statements/src/extensions/scorecards.rs finstack/statements-analytics/src/extensions/
```

- [ ] **Step 2: Update `crate::` imports in copied extension files**

In both `corkscrew.rs` and `scorecards.rs` under `finstack/statements-analytics/src/extensions/`:
- `crate::error` → `finstack_statements::error`
- `crate::evaluator` → `finstack_statements::evaluator`
- `crate::types` → `finstack_statements::types`
- `crate::extensions::{Extension, ExtensionContext, ...}` → `finstack_statements::extensions::{Extension, ExtensionContext, ...}`
- `crate::Result` → `finstack_statements::Result`

- [ ] **Step 3: Create `finstack/statements-analytics/src/extensions/mod.rs`**

```rust
//! Concrete extension implementations.
//!
//! These extensions implement the [`finstack_statements::extensions::Extension`] trait
//! to provide analysis and validation capabilities.

mod corkscrew;
mod scorecards;

pub use corkscrew::{AccountType, CorkscrewAccount, CorkscrewConfig, CorkscrewExtension};
pub use scorecards::{CreditScorecardExtension, ScorecardConfig, ScorecardMetric};
```

- [ ] **Step 4: Register the extensions module in analytics `lib.rs`**

Add to `finstack/statements-analytics/src/lib.rs`:

```rust
/// Concrete extension implementations (corkscrew, credit scorecard).
pub mod extensions;
```

- [ ] **Step 5: Update core `extensions/mod.rs` — remove concrete modules only**

Remove the concrete module declarations and re-exports. Do NOT add `#[cfg(feature = "analytics")]` re-exports yet — those are added in Task 7 when the Cargo.toml dependency is established.

Change `finstack/statements/src/extensions/mod.rs` from:

```rust
mod corkscrew;
mod plugin;
mod registry;
mod scorecards;

pub use corkscrew::{AccountType, CorkscrewAccount, CorkscrewConfig, CorkscrewExtension};
pub use plugin::{
    Extension, ExtensionContext, ExtensionMetadata, ExtensionResult, ExtensionStatus,
};
pub use registry::ExtensionRegistry;
pub use scorecards::{CreditScorecardExtension, ScorecardConfig, ScorecardMetric};
```

to:

```rust
//! Extension plugin system for the statements engine.
//!
//! This module provides the [`Extension`] trait and [`ExtensionRegistry`] for
//! building custom analysis and validation plugins.
//!
//! For built-in extensions (corkscrew, credit scorecard), enable the `analytics`
//! feature or depend on `finstack-statements-analytics` directly.

mod plugin;
mod registry;

pub use plugin::{
    Extension, ExtensionContext, ExtensionMetadata, ExtensionResult, ExtensionStatus,
};
pub use registry::ExtensionRegistry;
```

- [ ] **Step 6: Remove concrete extension files from core**

```bash
rm finstack/statements/src/extensions/corkscrew.rs
rm finstack/statements/src/extensions/scorecards.rs
```

- [ ] **Step 7: Check both crates compile**

Run: `cargo check -p finstack-statements-analytics && cargo check -p finstack-statements`
Expected: Both compile. The analytics crate resolves the trait from core; core compiles without the concrete extensions (no conditional re-exports yet — those come in Task 7).

- [ ] **Step 8: Commit**

```bash
git add finstack/statements-analytics/src/extensions/
git add finstack/statements/src/extensions/mod.rs
git rm finstack/statements/src/extensions/corkscrew.rs finstack/statements/src/extensions/scorecards.rs
git commit -m "refactor: move corkscrew and scorecards to analytics crate"
```

---

### Task 5: Move the `templates/` module to the analytics crate

**Files:**
- Move: `finstack/statements/src/templates/` → `finstack/statements-analytics/src/templates/`
  - `mod.rs`, `builder.rs`, `real_estate.rs`, `roll_forward.rs`, `vintage.rs`
- Modify: `finstack/statements-analytics/src/lib.rs`
- Modify: `finstack/statements/src/lib.rs`

- [ ] **Step 1: Copy the templates module**

```bash
cp -r finstack/statements/src/templates finstack/statements-analytics/src/templates
```

- [ ] **Step 2: Update `crate::` imports in all templates files**

In every file under `finstack/statements-analytics/src/templates/`:
- `crate::builder` → `finstack_statements::builder`
- `crate::types` → `finstack_statements::types`
- `crate::error` → `finstack_statements::error`
- `crate::evaluator` → `finstack_statements::evaluator`
- `crate::extensions` → `finstack_statements::extensions`
- `crate::forecast` → `finstack_statements::forecast`
- `crate::Result` → `finstack_statements::Result`

In `templates/mod.rs`, update doc references:
- `[`ModelBuilder`](crate::builder::ModelBuilder)` → `[`ModelBuilder`](finstack_statements::builder::ModelBuilder)`
- `[`CorkscrewExtension`](crate::extensions::CorkscrewExtension)` → `[`CorkscrewExtension`](crate::extensions::CorkscrewExtension)` (stays local — corkscrew is now in analytics crate too)

- [ ] **Step 3: Register the templates module in analytics `lib.rs`**

Add to `finstack/statements-analytics/src/lib.rs`:

```rust
/// Templates for common financial model structures.
pub mod templates;
```

- [ ] **Step 4: Remove templates from core**

Remove the `finstack/statements/src/templates/` directory entirely.

In `finstack/statements/src/lib.rs`, remove:

```rust
/// Templates for common models and schemas.
pub mod templates;
```

- [ ] **Step 5: Check analytics crate compiles**

Run: `cargo check -p finstack-statements-analytics`
Expected: Compiles.

- [ ] **Step 6: Commit**

```bash
git add finstack/statements-analytics/src/templates/
git add finstack/statements-analytics/src/lib.rs
git add finstack/statements/src/lib.rs
git rm -r finstack/statements/src/templates/
git commit -m "refactor: move templates module to analytics crate"
```

---

### Task 6: Populate the analytics prelude

**Files:**
- Modify: `finstack/statements-analytics/src/prelude.rs`

- [ ] **Step 1: Write the analytics prelude**

Replace `finstack/statements-analytics/src/prelude.rs` with:

```rust
//! Commonly used analytics types.
//!
//! Import this module to get quick access to the most common analytics types:
//!
//! ```rust,ignore
//! use finstack_statements_analytics::prelude::*;
//! ```

pub use crate::analysis::{
    BridgeChart, BridgeStep, CorporateAnalysis, CorporateAnalysisBuilder,
    CreditInstrumentAnalysis, MonteCarloConfig, MonteCarloResults,
    ScenarioDefinition, ScenarioDiff, ScenarioResults, ScenarioSet,
    VarianceAnalyzer, VarianceConfig, VarianceReport, VarianceRow,
};
pub use crate::extensions::{CorkscrewExtension, CreditScorecardExtension};
pub use crate::templates::{RealEstateExtension, TemplatesExtension, VintageExtension};
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p finstack-statements-analytics`
Expected: Compiles.

- [ ] **Step 3: Commit**

```bash
git add finstack/statements-analytics/src/prelude.rs
git commit -m "feat: populate analytics prelude with common re-exports"
```

---

### Task 7: Add `analytics` feature to core crate and wire re-exports

**Files:**
- Modify: `finstack/statements/Cargo.toml` (add analytics feature + optional dep)
- Modify: `finstack/statements/src/lib.rs` (add conditional re-exports for analysis + templates)
- Modify: `finstack/statements/src/extensions/mod.rs` (add conditional re-exports for concrete extensions)
- Modify: `finstack/statements/src/prelude.rs` (split into core + conditional analytics)

**Context:** This is the critical backwards-compatibility step. After this task, enabling `features = ["analytics"]` on `finstack-statements` makes all the old import paths work again.

- [ ] **Step 1: Add the analytics feature to core `Cargo.toml`**

In `finstack/statements/Cargo.toml`, add to `[dependencies]`:

```toml
finstack-statements-analytics = { path = "../statements-analytics", optional = true }
```

And update `[features]`:

```toml
[features]
default = []
analytics = ["dep:finstack-statements-analytics"]
dataframes = ["dep:polars"]
parallel = ["dep:rayon"]
```

- [ ] **Step 2: Add conditional module re-exports in `lib.rs`**

In `finstack/statements/src/lib.rs`, after the existing `pub(crate) mod utils;` line, add:

```rust
// Re-export analytics modules when the feature is enabled
#[cfg(feature = "analytics")]
pub use finstack_statements_analytics::analysis;

#[cfg(feature = "analytics")]
pub use finstack_statements_analytics::templates;
```

- [ ] **Step 2b: Add conditional extension re-exports in `extensions/mod.rs`**

In `finstack/statements/src/extensions/mod.rs`, after the existing `pub use registry::ExtensionRegistry;` line, add:

```rust
// Re-export concrete extension impls from analytics crate when available
#[cfg(feature = "analytics")]
pub use finstack_statements_analytics::extensions::{
    AccountType, CorkscrewAccount, CorkscrewConfig, CorkscrewExtension,
    CreditScorecardExtension, ScorecardConfig, ScorecardMetric,
};
```

- [ ] **Step 3: Update `prelude.rs` for conditional analytics**

Replace `finstack/statements/src/prelude.rs` with:

```rust
//! Commonly used types and traits.
//!
//! Import this module to get quick access to the most common types:
//!
//! ```rust,ignore
//! use finstack_statements::prelude::*;
//! ```
//!
//! This prelude is intentionally broad: it re-exports the most common
//! `finstack-statements` types plus the full `finstack_core::prelude::*`.
//! When the `analytics` feature is enabled, analytics types are included too.

pub use crate::builder::{MixedNodeBuilder, ModelBuilder, NeedPeriods, Ready};
pub use crate::error::{Error, Result};
pub use crate::evaluator::{Evaluator, EvaluatorWithContext, NumericMode, StatementResult};
pub use crate::extensions::{
    Extension, ExtensionContext, ExtensionMetadata, ExtensionRegistry, ExtensionResult,
    ExtensionStatus,
};
pub use crate::registry::Registry;
pub use crate::types::{
    AmountOrScalar, FinancialModelSpec, ForecastMethod, ForecastSpec, NodeId, NodeSpec, NodeType,
    NodeValueType, SeasonalMode,
};

// Re-export the full core prelude for a unified foundation
pub use finstack_core::prelude::*;

// Additional date types used by statements but not in the core prelude
pub use finstack_core::dates::{build_periods, Period, PeriodId, PeriodKind};

// Analytics prelude (available when feature enabled)
#[cfg(feature = "analytics")]
pub use finstack_statements_analytics::prelude::*;
```

- [ ] **Step 4: Verify core compiles without the analytics feature**

Run: `cargo check -p finstack-statements`
Expected: Compiles — no analytics modules referenced without the feature flag.

- [ ] **Step 5: Verify core compiles with the analytics feature**

Run: `cargo check -p finstack-statements --features analytics`
Expected: Compiles — re-exports resolve to the analytics crate.

- [ ] **Step 6: Commit**

```bash
git add finstack/statements/Cargo.toml finstack/statements/src/lib.rs finstack/statements/src/extensions/mod.rs finstack/statements/src/prelude.rs
git commit -m "feat: add analytics feature with conditional re-exports for backwards compatibility"
```

---

### Task 8: Update downstream consumers

**Files:**
- Modify: `finstack/Cargo.toml` (aggregator, line 17 — statements feature)
- Modify: `finstack-py/Cargo.toml` (line 29 — add analytics feature)
- Modify: `finstack-wasm/Cargo.toml` (line 17 — add analytics feature)

- [ ] **Step 1: Update the `finstack` aggregator**

In `finstack/Cargo.toml`, change the `statements` feature (line 17) from:

```toml
statements = ["core", "dep:finstack_statements", "dep:indexmap"]
```

to:

```toml
statements = ["core", "dep:finstack_statements", "finstack_statements/analytics", "dep:indexmap"]
```

- [ ] **Step 2: Update `finstack-py`**

In `finstack-py/Cargo.toml`, change the statements dependency (line 29) from:

```toml
finstack-statements = { path = "../finstack/statements", features = ["dataframes"] }
```

to:

```toml
finstack-statements = { path = "../finstack/statements", features = ["dataframes", "analytics"] }
```

- [ ] **Step 3: Update `finstack-wasm`**

In `finstack-wasm/Cargo.toml`, change the statements dependency (line 17) from:

```toml
finstack-statements = { path = "../finstack/statements" }
```

to:

```toml
finstack-statements = { path = "../finstack/statements", features = ["analytics"] }
```

- [ ] **Step 4: Verify all downstream crates compile**

Run: `cargo check -p finstack && cargo check -p finstack-py && cargo check -p finstack-wasm`
Expected: All compile. Existing import paths resolve via re-exports.

- [ ] **Step 5: Commit**

```bash
git add finstack/Cargo.toml finstack-py/Cargo.toml finstack-wasm/Cargo.toml
git commit -m "chore: enable analytics feature in downstream consumers"
```

---

### Task 9: Move analytics tests to the new crate

**Files:**
- Move to `finstack/statements-analytics/tests/`:
  - `finstack/statements/tests/analysis_corporate.rs`
  - `finstack/statements/tests/analysis_goal_seek.rs`
  - `finstack/statements/tests/analysis_monte_carlo.rs`
  - `finstack/statements/tests/analysis_orchestrator.rs`
  - `finstack/statements/tests/analysis_scenario_set.rs`
  - `finstack/statements/tests/extensions_scorecards.rs`
  - `finstack/statements/tests/extensions/extensions_tests.rs`
  - `finstack/statements/tests/extensions/extensions_full_execution_tests.rs`
  - `finstack/statements/tests/feature_completeness_tests.rs`
  - `finstack/statements/tests/integration/real_estate_template_tests.rs`
  - `finstack/statements/tests/forecast/forecast_backtesting_tests.rs`
  - `finstack/statements/tests/common.rs` (copy — needed by `forecast_backtesting_tests.rs` via `forecast_all.rs`)
- Update harness files (Cargo discovers tests via `*_all.rs`, NOT `mod.rs`):
  - `finstack/statements/tests/extensions_all.rs` — remove both extension test modules (file becomes empty, remove it)
  - `finstack/statements/tests/integration_all.rs` — remove `real_estate_template_tests` entry
  - `finstack/statements/tests/forecast_all.rs` — remove `forecast_backtesting_tests` entry
- Keep in core: `finstack/statements/tests/integration/patterns_tests.rs` (only uses core APIs)

**Context:** Tests that import from `analysis::*`, `templates::*`, or concrete extensions (`CorkscrewExtension`, `CreditScorecardExtension`) move to the analytics crate. Tests that only use core types stay. The project uses `*_all.rs` harness files (not `mod.rs`) for Cargo test discovery of nested test modules.

- [ ] **Step 1: Create tests directory and copy top-level test files**

```bash
mkdir -p finstack/statements-analytics/tests
cp finstack/statements/tests/analysis_corporate.rs finstack/statements-analytics/tests/
cp finstack/statements/tests/analysis_goal_seek.rs finstack/statements-analytics/tests/
cp finstack/statements/tests/analysis_monte_carlo.rs finstack/statements-analytics/tests/
cp finstack/statements/tests/analysis_orchestrator.rs finstack/statements-analytics/tests/
cp finstack/statements/tests/analysis_scenario_set.rs finstack/statements-analytics/tests/
cp finstack/statements/tests/extensions_scorecards.rs finstack/statements-analytics/tests/
cp finstack/statements/tests/feature_completeness_tests.rs finstack/statements-analytics/tests/
cp finstack/statements/tests/common.rs finstack/statements-analytics/tests/
```

- [ ] **Step 2: Copy extension integration tests with harness**

```bash
mkdir -p finstack/statements-analytics/tests/extensions
cp finstack/statements/tests/extensions/extensions_tests.rs finstack/statements-analytics/tests/extensions/
cp finstack/statements/tests/extensions/extensions_full_execution_tests.rs finstack/statements-analytics/tests/extensions/
```

Create `finstack/statements-analytics/tests/extensions_all.rs` (harness file — Cargo discovers this):

```rust
// Extension tests.
//
// Note: Cargo only discovers integration tests that are direct children of `tests/`.
// This file wires in the nested extensions test modules so they run.

#[path = "extensions/extensions_tests.rs"]
mod extensions_tests;

#[path = "extensions/extensions_full_execution_tests.rs"]
mod extensions_full_execution_tests;
```

- [ ] **Step 3: Copy template integration tests with harness**

```bash
mkdir -p finstack/statements-analytics/tests/integration
cp finstack/statements/tests/integration/real_estate_template_tests.rs finstack/statements-analytics/tests/integration/
```

Create `finstack/statements-analytics/tests/integration_all.rs` (harness file):

```rust
// Integration tests for analytics components.

#[path = "integration/real_estate_template_tests.rs"]
mod real_estate_template_tests;
```

- [ ] **Step 4: Copy forecast backtesting test with harness**

```bash
mkdir -p finstack/statements-analytics/tests/forecast
cp finstack/statements/tests/forecast/forecast_backtesting_tests.rs finstack/statements-analytics/tests/forecast/
```

Create `finstack/statements-analytics/tests/forecast_all.rs` (harness file):

```rust
// Forecast analytics tests.

#[path = "common.rs"]
mod common;

#[path = "forecast/forecast_backtesting_tests.rs"]
mod forecast_backtesting_tests;
```

- [ ] **Step 5: Update imports in moved test files**

In all moved test files, update imports:
- `use finstack_statements::analysis::*` → `use finstack_statements_analytics::analysis::*`
- `use finstack_statements::extensions::{CorkscrewExtension, CreditScorecardExtension, ...}` → `use finstack_statements_analytics::extensions::{CorkscrewExtension, CreditScorecardExtension, ...}`
- `use finstack_statements::templates::*` → `use finstack_statements_analytics::templates::*`
- Core imports like `use finstack_statements::prelude::*` stay as-is (core types)
- `use finstack_statements::evaluator::*` stays as-is (core types)

- [ ] **Step 6: Remove moved test files from core**

```bash
rm finstack/statements/tests/analysis_corporate.rs
rm finstack/statements/tests/analysis_goal_seek.rs
rm finstack/statements/tests/analysis_monte_carlo.rs
rm finstack/statements/tests/analysis_orchestrator.rs
rm finstack/statements/tests/analysis_scenario_set.rs
rm finstack/statements/tests/extensions_scorecards.rs
rm finstack/statements/tests/feature_completeness_tests.rs
rm finstack/statements/tests/extensions/extensions_tests.rs
rm finstack/statements/tests/extensions/extensions_full_execution_tests.rs
rm finstack/statements/tests/integration/real_estate_template_tests.rs
rm finstack/statements/tests/forecast/forecast_backtesting_tests.rs
```

- [ ] **Step 7: Update core harness files**

**`extensions_all.rs`:** Both extension test modules moved. Remove the file entirely:
```bash
rm finstack/statements/tests/extensions_all.rs
rm -r finstack/statements/tests/extensions/
```

**`integration_all.rs`:** Remove the `real_estate_template_tests` entry (lines 9-10). Keep `patterns_tests` and all other entries:

```rust
// Integration tests for statements components.

#[path = "integration/dated_cashflow_export_tests.rs"]
mod dated_cashflow_export_tests;

#[path = "integration/money_integration_tests.rs"]
mod money_integration_tests;

#[path = "integration/patterns_tests.rs"]
mod patterns_tests;

#[path = "integration/term_loan_integration_tests.rs"]
mod term_loan_integration_tests;

#[path = "integration/waterfall_tests.rs"]
mod waterfall_tests;
```

**`forecast_all.rs`:** Remove the `forecast_backtesting_tests` entry (lines 12-13). Keep `common`, `forecast_tests`, and `time_series_tests`:

```rust
// Forecast and time-series tests.

#[path = "common.rs"]
mod common;

#[path = "forecast/forecast_tests.rs"]
mod forecast_tests;

#[path = "forecast/time_series_tests.rs"]
mod time_series_tests;
```

- [ ] **Step 8: Verify analytics tests pass**

Run: `cargo test -p finstack-statements-analytics`
Expected: All moved tests pass.

- [ ] **Step 9: Verify core tests still pass**

Run: `cargo test -p finstack-statements`
Expected: Remaining core tests pass (no compilation errors from removed test references).

- [ ] **Step 10: Commit**

```bash
git add finstack/statements-analytics/tests/
git add finstack/statements/tests/
git commit -m "refactor: move analytics tests to finstack-statements-analytics"
```

---

### Task 10: Full workspace verification

**Files:** No file changes — verification only.

- [ ] **Step 1: Run full workspace test suite**

Run: `cargo test --workspace`
Expected: All tests pass across all crates.

- [ ] **Step 2: Run workspace clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: No warnings or errors.

- [ ] **Step 3: Verify core compiles without analytics**

Run: `cargo check -p finstack-statements --no-default-features`
Expected: Compiles — core is fully self-contained without analytics.

- [ ] **Step 4: Verify doc-tests**

Run: `cargo test --doc -p finstack-statements-analytics`
Expected: All doc-tests pass. If any fail due to stale `crate::` references, fix them.

Run: `cargo test --doc -p finstack-statements --features analytics`
Expected: Core doc-tests pass with analytics enabled.

- [ ] **Step 5: Fix any issues found in steps 1-4**

Address any compilation errors, test failures, or clippy warnings. Common issues:
- Stale `crate::` references in doc-tests
- Missing `use` imports in test files
- Feature-gated code that needs adjustment

- [ ] **Step 6: Final commit**

```bash
git add -A
git commit -m "chore: fix remaining issues from crate split verification"
```

(Only if step 5 required changes. Skip if everything passed clean.)

---

### Task 11: Update lib.rs doc comments and README

**Files:**
- Modify: `finstack/statements/src/lib.rs` (module-level doc comment)
- Modify: `finstack/statements-analytics/src/lib.rs` (expand doc comment)

- [ ] **Step 1: Update core `lib.rs` doc comment**

The module doc in `finstack/statements/src/lib.rs` (lines 12-60) currently mentions analysis and extensions. Update it to reflect the split — remove references to analysis modules, add a note about the `analytics` feature:

Replace the `## Architecture` section to remove `analysis` and `extensions` (concrete impls) from the list. Add a section:

```rust
//! ## Analytics
//!
//! Higher-level analysis tools (sensitivity, scenario sets, DCF, etc.) and
//! concrete extension implementations are in the [`finstack-statements-analytics`]
//! crate. Enable the `analytics` feature to re-export them here:
//!
//! ```toml
//! finstack-statements = { version = "0.4", features = ["analytics"] }
//! ```
```

- [ ] **Step 2: Commit**

```bash
git add finstack/statements/src/lib.rs finstack/statements-analytics/src/lib.rs
git commit -m "docs: update lib.rs doc comments to reflect crate split"
```
