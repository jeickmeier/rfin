# Simplicity & Deduplication Fixes Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate duplicated logic and multiple paths to the same functionality across the portfolio and scenarios crates.

**Architecture:** Six targeted refactors — each collapses duplicated code into a single source of truth. Changes are ordered from lowest to highest risk so that each commit leaves the codebase green. No public API changes; all modifications are internal.

**Tech Stack:** Rust, `finstack_core::market_data::context::CurveStorage`, `rayon` (optional feature)

---

## Chunk 1: Low-Risk Deduplication (Tasks 1–3)

### Task 1: Consolidate `Portfolio` index-building into `rebuild_index`

Three sites build `position_index` + `dependency_index` with identical logic:
- `portfolio.rs::rebuild_index` (lines 131-139) — the canonical single method
- `portfolio.rs::from_spec` (lines 297-302) — hand-inlined copy
- `builder.rs::build` (lines 312-318) — hand-inlined copy

If `rebuild_index` ever gains additional derived state, these two copies will silently drift.

**Files:**
- Modify: `finstack/portfolio/src/portfolio.rs:289-322`
- Modify: `finstack/portfolio/src/builder.rs:309-331`

- [ ] **Step 1: Run existing tests as baseline**

Run: `cargo test -p finstack-portfolio --lib --tests 2>&1 | tail -5`
Expected: all tests pass

- [ ] **Step 2: Refactor `from_spec` to use `rebuild_index`**

In `finstack/portfolio/src/portfolio.rs`, replace the manual index building in `from_spec`:

```rust
    pub fn from_spec(spec: PortfolioSpec) -> Result<Self> {
        let positions: Result<Vec<_>> = spec
            .positions
            .into_iter()
            .map(crate::position::Position::from_spec)
            .collect();

        let positions = positions?;

        let mut portfolio = Self {
            id: spec.id,
            name: spec.name,
            base_ccy: spec.base_ccy,
            as_of: spec.as_of,
            entities: spec.entities,
            positions,
            position_index: HashMap::default(),
            dependency_index: DependencyIndex::default(),
            books: spec.books,
            tags: spec.tags,
            meta: spec.meta,
        };

        portfolio.rebuild_index();

        // Validate the reconstructed portfolio
        portfolio.validate()?;

        Ok(portfolio)
    }
```

The key change: construct with default indices, then call `rebuild_index()` instead of duplicating the logic.

- [ ] **Step 3: Refactor `PortfolioBuilder::build` to use `rebuild_index`**

In `finstack/portfolio/src/builder.rs`, replace lines 312-318:

Change from:

```rust
        let position_index = self
            .positions
            .iter()
            .enumerate()
            .map(|(i, p)| (p.position_id.clone(), i))
            .collect();
        let dependency_index = crate::dependencies::DependencyIndex::build(&self.positions);

        let portfolio = Portfolio {
            id: self.id,
            ...
            position_index,
            dependency_index,
            ...
        };
```

To:

```rust
        let mut portfolio = Portfolio {
            id: self.id,
            ...
            position_index: finstack_core::HashMap::default(),
            dependency_index: crate::dependencies::DependencyIndex::default(),
            ...
        };

        portfolio.rebuild_index();
```

- [ ] **Step 4: Run tests to verify no regressions**

Run: `cargo test -p finstack-portfolio --lib --tests 2>&1 | tail -5`
Expected: all tests pass (identical behaviour)

- [ ] **Step 5: Commit**

```bash
git add finstack/portfolio/src/portfolio.rs finstack/portfolio/src/builder.rs
git commit -m "refactor(portfolio): consolidate index building into rebuild_index

from_spec and PortfolioBuilder::build now delegate to rebuild_index()
instead of duplicating position_index + dependency_index construction.
Eliminates drift risk across three independent build sites."
```

---

### Task 2: Replace `set_scalar_rate` with `apply_forecast_assign`

`set_scalar_rate` (lines 189-203) is a hand-inlined copy of `apply_forecast_assign` (lines 87-95). They both iterate over a node's values and set each to `AmountOrScalar::Scalar(value)`.

**Files:**
- Modify: `finstack/scenarios/src/adapters/statements.rs:106-187`

- [ ] **Step 1: Run existing tests as baseline**

Run: `cargo test -p finstack-scenarios --lib --tests 2>&1 | tail -5`
Expected: all tests pass

- [ ] **Step 2: Replace `set_scalar_rate` call sites with `apply_forecast_assign`**

In `finstack/scenarios/src/adapters/statements.rs`, find the two call sites of `set_scalar_rate`:

Line 133: `return set_scalar_rate(model, binding.node_id.as_str(), converted);`
Line 181: `return set_scalar_rate(model, binding.node_id.as_str(), converted);`

Replace both with:

```rust
return apply_forecast_assign(model, binding.node_id.as_str(), converted);
```

- [ ] **Step 3: Delete the `set_scalar_rate` function**

Remove the entire function (lines 189-203):

```rust
fn set_scalar_rate(model: &mut FinancialModelSpec, node_id: &str, rate: f64) -> Result<()> {
    ...
}
```

- [ ] **Step 4: Run tests to verify**

Run: `cargo test -p finstack-scenarios --lib --tests 2>&1 | tail -5`
Expected: all tests pass

- [ ] **Step 5: Commit**

```bash
git add finstack/scenarios/src/adapters/statements.rs
git commit -m "refactor(scenarios): replace set_scalar_rate with apply_forecast_assign

set_scalar_rate was a hand-inlined copy of apply_forecast_assign.
Both set all node values to AmountOrScalar::Scalar(v). Using the
existing public function eliminates the duplication."
```

---

### Task 3: Collapse serial/parallel valuation into a single function

`value_portfolio_serial` and `value_portfolio_parallel` (lines 293-329) are identical except for `.iter()` vs `.par_iter()`. Use a `cfg`-conditional import pattern to collapse them into one function body.

**Files:**
- Modify: `finstack/portfolio/src/valuation.rs:273-329`

- [ ] **Step 1: Run existing tests as baseline**

Run: `cargo test -p finstack-portfolio --lib --tests 2>&1 | tail -5`
Expected: all tests pass

- [ ] **Step 2: Replace the three functions with a single implementation**

Replace `value_portfolio_with_options`, `value_portfolio_serial`, and `value_portfolio_parallel` (lines 274-329) with a single function that uses inline `cfg` blocks. We use `par_iter()` (not `par_bridge()`) inside the parallel branch to preserve deterministic ordering:

```rust
/// Value all positions in a portfolio with full metrics.
///
/// When the `parallel` feature is enabled, position valuations are computed
/// in parallel using rayon. Results are deterministically reduced.
pub fn value_portfolio_with_options(
    portfolio: &Portfolio,
    market: &MarketContext,
    _config: &FinstackConfig,
    options: &PortfolioValuationOptions,
) -> Result<PortfolioValuation> {
    let metrics = resolve_metrics(options);

    let position_values_vec: Vec<PositionValue> = {
        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            portfolio
                .positions
                .par_iter()
                .map(|position| {
                    value_single_position(position, market, portfolio, &metrics, options.strict_risk)
                })
                .collect::<Result<Vec<_>>>()?
        }
        #[cfg(not(feature = "parallel"))]
        {
            portfolio
                .positions
                .iter()
                .map(|position| {
                    value_single_position(position, market, portfolio, &metrics, options.strict_risk)
                })
                .collect::<Result<Vec<_>>>()?
        }
    };

    assemble_valuation(position_values_vec, portfolio.base_ccy, portfolio.as_of)
}
```

The `.map(...)` call is duplicated inside the `cfg` blocks, but the surrounding function boilerplate (`resolve_metrics`, `assemble_valuation`, doc comment, signature) is not.

- [ ] **Step 3: Remove the `#[cfg]` attributes from the old function signatures**

Delete the now-unused `value_portfolio_serial` and `value_portfolio_parallel` functions entirely. Keep only the doc comment on `value_portfolio_with_options`.

- [ ] **Step 4: Run tests with both feature configurations**

Run: `cargo test -p finstack-portfolio --lib --tests 2>&1 | tail -5`
Run: `cargo test -p finstack-portfolio --lib --tests --no-default-features --features scenarios,dataframes 2>&1 | tail -5`
Expected: both pass

- [ ] **Step 5: Commit**

```bash
git add finstack/portfolio/src/valuation.rs
git commit -m "refactor(portfolio): collapse serial/parallel valuation into single function

value_portfolio_serial and value_portfolio_parallel were identical
except for .iter() vs .par_iter(). Merged into a single
value_portfolio_with_options with an inline cfg block."
```

---

## Chunk 2: Scenario Engine Deduplication (Tasks 4–5)

### Task 4: Collapse five `UpdateXxxCurve` variants into `UpdateCurve`

The `ScenarioEffect` enum has five structurally identical curve update variants. They are all handled identically in the engine: `*ctx.market = std::mem::take(ctx.market).insert(curve.as_ref().clone())`. The `MarketContext::insert` method accepts anything implementing `Into<CurveStorage>`, and all five curve types have `From<Arc<T>> for CurveStorage` impls. Collapse them into a single `UpdateCurve(CurveStorage)` variant.

**Files:**
- Modify: `finstack/scenarios/src/adapters/traits.rs:14-55`
- Modify: `finstack/scenarios/src/engine.rs:348-382`
- Modify: `finstack/scenarios/src/adapters/curves.rs` (all `ScenarioEffect::UpdateXxxCurve` construction sites)

- [ ] **Step 1: Run existing tests as baseline**

Run: `cargo test -p finstack-scenarios --lib --tests 2>&1 | tail -5`
Expected: all tests pass

- [ ] **Step 2: Add `CurveStorage` import and replace the five variants in `traits.rs`**

In `finstack/scenarios/src/adapters/traits.rs`, replace:

```rust
use finstack_core::market_data::term_structures::{
    DiscountCurve, ForwardCurve, HazardCurve, InflationCurve, VolatilityIndexCurve,
};
use std::sync::Arc;
```

With:

```rust
use finstack_core::market_data::context::CurveStorage;
```

Then replace the five `UpdateXxxCurve` variants (lines 21-55):

```rust
    /// Update a discount curve in the market.
    UpdateDiscountCurve { id: String, curve: Arc<DiscountCurve> },
    /// Update a forward curve in the market.
    UpdateForwardCurve { id: String, curve: Arc<ForwardCurve> },
    /// Update a hazard curve in the market.
    UpdateHazardCurve { id: String, curve: Arc<HazardCurve> },
    /// Update an inflation curve in the market.
    UpdateInflationCurve { id: String, curve: Arc<InflationCurve> },
    /// Update a volatility index curve in the market.
    UpdateVolIndexCurve { id: String, curve: Arc<VolatilityIndexCurve> },
```

With a single variant:

```rust
    /// Update a curve in the market context.
    ///
    /// Wraps any curve type via [`CurveStorage`], which the engine inserts
    /// into the market context via `MarketContext::insert`.
    UpdateCurve(CurveStorage),
```

- [ ] **Step 3: Update engine.rs to handle the single variant**

In `finstack/scenarios/src/engine.rs`, replace the five match arms (lines 348-382):

```rust
                        crate::adapters::traits::ScenarioEffect::UpdateDiscountCurve {
                            id: _id,
                            curve,
                        } => {
                            *ctx.market = std::mem::take(ctx.market).insert(curve.as_ref().clone());
                            applied += 1;
                        }
                        // ... 4 more identical arms ...
```

With one arm:

```rust
                        crate::adapters::traits::ScenarioEffect::UpdateCurve(storage) => {
                            *ctx.market = std::mem::take(ctx.market).insert(storage);
                            applied += 1;
                        }
```

Note: `CurveStorage` implements `Into<CurveStorage>` trivially (identity), so `insert(storage)` works directly.

- [ ] **Step 4: Update all construction sites in `adapters/curves.rs`**

In `finstack/scenarios/src/adapters/curves.rs`, find-and-replace each construction. Every instance follows the pattern:

```rust
// Before (5 variations):
ScenarioEffect::UpdateDiscountCurve {
    id: curve_id.clone(),
    curve: std::sync::Arc::new(new_curve),
}

// After (all become):
ScenarioEffect::UpdateCurve(
    CurveStorage::from(std::sync::Arc::new(new_curve))
)
```

There are exactly **10 construction sites** in `curves.rs`. Each follows the same transform — replace the typed variant with `ScenarioEffect::UpdateCurve(CurveStorage::from(...))`:

| Line | Old Variant | Context (CurveKind × OpKind) |
|------|-------------|------------------------------|
| 222  | `UpdateDiscountCurve` | Discount × ParallelBp |
| 273  | `UpdateHazardCurve` | ParCDS × ParallelBp |
| 304  | `UpdateInflationCurve` | Inflation × ParallelBp |
| 353  | `UpdateVolIndexCurve` | VolIndex × ParallelBp |
| 408  | `UpdateDiscountCurve` | Discount × NodeBp |
| 462  | `UpdateForwardCurve` | Forward × NodeBp |
| 502  | `UpdateHazardCurve` | ParCDS × NodeBp |
| 563  | `UpdateInflationCurve` | Inflation × NodeBp |
| 618  | `UpdateDiscountCurve` | Commodity × NodeBp |
| 675  | `UpdateVolIndexCurve` | VolIndex × NodeBp |

Example transform (applies to all 10):

```rust
// Before:
ScenarioEffect::UpdateDiscountCurve {
    id: curve_id.clone(),
    curve: std::sync::Arc::new(new_curve),
}
// After:
ScenarioEffect::UpdateCurve(
    CurveStorage::from(std::sync::Arc::new(new_curve))
)
```

Add the import at the top of `curves.rs`:

```rust
use finstack_core::market_data::context::CurveStorage;
```

Remove the now-unused import of `Arc` from `traits.rs` (if it was the only user).

- [ ] **Step 5: Check for any other files referencing the old variants**

Run: `grep -r "UpdateDiscountCurve\|UpdateForwardCurve\|UpdateHazardCurve\|UpdateInflationCurve\|UpdateVolIndexCurve" finstack/`

Fix any remaining references (likely none outside `curves.rs` and `engine.rs`).

- [ ] **Step 6: Run tests to verify**

Run: `cargo test -p finstack-scenarios --lib --tests 2>&1 | tail -5`
Expected: all tests pass

- [ ] **Step 7: Build the downstream crates that depend on scenarios**

Run: `cargo build -p finstack-portfolio --features scenarios 2>&1 | tail -5`
Run: `cargo build -p finstack-py 2>&1 | tail -10`
Run: `cargo build -p finstack-wasm 2>&1 | tail -10`
Expected: all compile cleanly

- [ ] **Step 8: Commit**

```bash
git add finstack/scenarios/src/adapters/traits.rs finstack/scenarios/src/engine.rs finstack/scenarios/src/adapters/curves.rs
git commit -m "refactor(scenarios): collapse five UpdateXxxCurve variants into UpdateCurve(CurveStorage)

All five curve update variants were handled identically in the engine
via MarketContext::insert(). CurveStorage already provides a unified
enum with From impls for all curve types. This eliminates five match
arms in the engine and five enum variants in ScenarioEffect."
```

---

### Task 5: Deduplicate `InstrumentPriceShock` / `InstrumentSpreadShock` handling in engine

The engine handles `InstrumentPriceShock` and `InstrumentSpreadShock` with ~90 lines of near-identical code. The only differences: which `apply_instrument_*` function is called, and the field name (`pct` vs `bp`). Extract a helper.

**Files:**
- Modify: `finstack/scenarios/src/engine.rs:383-472`

- [ ] **Step 1: Run existing tests as baseline**

Run: `cargo test -p finstack-scenarios --lib --tests 2>&1 | tail -5`
Expected: all tests pass

- [ ] **Step 2: Add helper function above `apply_correlation_effect`**

Add this function in `engine.rs`, before `apply_correlation_effect`:

```rust
/// Apply an instrument shock (price or spread) using the appropriate adapter functions.
///
/// This helper unifies the near-identical handling of `InstrumentPriceShock` and
/// `InstrumentSpreadShock` by accepting closures for the type-based and attr-based
/// shock application functions.
fn apply_instrument_shock<FType, FAttr>(
    types: Option<Vec<finstack_valuations::pricer::InstrumentType>>,
    attrs: Option<indexmap::IndexMap<String, String>>,
    value: f64,
    ctx: &mut ExecutionContext,
    apply_type_shock: FType,
    apply_attr_shock: FAttr,
    shock_kind: &str,
) -> (usize, Vec<String>)
where
    FType: FnOnce(
        &mut [Box<dyn finstack_valuations::instruments::Instrument>],
        &[finstack_valuations::pricer::InstrumentType],
        f64,
    ) -> crate::error::Result<usize>,
    FAttr: FnOnce(
        &mut [Box<dyn finstack_valuations::instruments::Instrument>],
        &indexmap::IndexMap<String, String>,
        f64,
    ) -> crate::error::Result<(usize, Vec<String>)>,
{
    let mut applied = 0usize;
    let mut warnings = Vec::new();

    if let Some(ts) = types {
        if let Some(instruments) = &mut ctx.instruments {
            match apply_type_shock(instruments, &ts, value) {
                Ok(c) => applied += c,
                Err(e) => warnings.push(format!("Instrument {} shock error: {}", shock_kind, e)),
            }
        } else {
            warnings.push(format!(
                "Instrument type {} shock requested but no instruments provided",
                shock_kind
            ));
        }
    }

    if let Some(ats) = attrs {
        if let Some(instruments) = &mut ctx.instruments {
            match apply_attr_shock(instruments, &ats, value) {
                Ok((count, w)) => {
                    applied += count;
                    warnings.extend(w);
                }
                Err(e) => warnings.push(format!("Instrument {} shock error: {}", shock_kind, e)),
            }
        } else {
            warnings.push(format!(
                "Instrument attribute {} shock requested but no instruments provided",
                shock_kind
            ));
        }
    }

    (applied, warnings)
}
```

- [ ] **Step 3: Replace the two match arms in the engine with calls to the helper**

Replace the `InstrumentPriceShock` arm (lines 383-426) with:

```rust
                        crate::adapters::traits::ScenarioEffect::InstrumentPriceShock {
                            types,
                            attrs,
                            pct,
                        } => {
                            let (count, ws) = apply_instrument_shock(
                                types,
                                attrs,
                                pct,
                                ctx,
                                crate::adapters::instruments::apply_instrument_type_price_shock,
                                crate::adapters::instruments::apply_instrument_attr_price_shock,
                                "price",
                            );
                            applied += count;
                            warnings.extend(ws);
                        }
```

Replace the `InstrumentSpreadShock` arm (lines 428-472) with:

```rust
                        crate::adapters::traits::ScenarioEffect::InstrumentSpreadShock {
                            types,
                            attrs,
                            bp,
                        } => {
                            let (count, ws) = apply_instrument_shock(
                                types,
                                attrs,
                                bp,
                                ctx,
                                crate::adapters::instruments::apply_instrument_type_spread_shock,
                                crate::adapters::instruments::apply_instrument_attr_spread_shock,
                                "spread",
                            );
                            applied += count;
                            warnings.extend(ws);
                        }
```

- [ ] **Step 4: Run tests to verify**

Run: `cargo test -p finstack-scenarios --lib --tests 2>&1 | tail -5`
Expected: all tests pass

- [ ] **Step 5: Commit**

```bash
git add finstack/scenarios/src/engine.rs
git commit -m "refactor(scenarios): deduplicate instrument price/spread shock handling

Extract apply_instrument_shock helper that accepts closures for the
type-based and attr-based shock functions. Replaces ~90 lines of
near-identical code with two concise call sites."
```

---

## Chunk 3: Prelude Cleanup (Task 6)

### Task 6: Make `prelude.rs` re-export from `crate::*` instead of duplicating `lib.rs`

Every `pub use` in `portfolio/src/lib.rs` is mirrored line-for-line in `prelude.rs`. Any new type added to `lib.rs` must be manually added to `prelude.rs` too. Fix by having `prelude.rs` glob-import from the crate root.

**Caveat:** Both `crate::*` and `finstack_core::prelude::*` export `Error` and `Result`. Two glob imports providing the same name is a Rust ambiguity error. Fix: keep `pub use crate::*` as the glob, and add explicit `pub use` for the portfolio-specific `Error`/`Result` to shadow the core prelude's versions. Direct re-exports take precedence over glob imports.

**Files:**
- Modify: `finstack/portfolio/src/prelude.rs`

- [ ] **Step 1: Run existing tests as baseline**

Run: `cargo test -p finstack-portfolio --lib --tests 2>&1 | tail -5`
Expected: all tests pass

- [ ] **Step 2: Replace `prelude.rs` contents**

Replace the entire file with:

```rust
//! Commonly used types and functions.
//!
//! Import this module to get quick access to the most common types:
//!
//! ```rust
//! use finstack_portfolio::prelude::*;
//! ```

// Re-export the full core prelude for a unified foundation.
// This is a glob, so direct re-exports below take precedence.
pub use finstack_core::prelude::*;

// Re-export everything from the crate root.
pub use crate::*;

// Explicit re-exports to disambiguate names that appear in both
// `crate::*` and `finstack_core::prelude::*`.
pub use crate::error::{Error, Result};
```

The explicit `pub use crate::error::{Error, Result}` shadows the glob from `finstack_core::prelude::*`, resolving the ambiguity. This matches the current behaviour where the portfolio's own `Error`/`Result` are the ones available in scope.

- [ ] **Step 3: Run tests to verify**

Run: `cargo test -p finstack-portfolio --lib --tests 2>&1 | tail -5`
Expected: all tests pass

- [ ] **Step 4: Check downstream consumers compile**

Run: `cargo build -p finstack-portfolio --features scenarios,dataframes 2>&1 | tail -5`
Expected: compiles cleanly

- [ ] **Step 5: Commit**

```bash
git add finstack/portfolio/src/prelude.rs
git commit -m "refactor(portfolio): prelude re-exports from crate root instead of duplicating lib.rs

Replaces 30+ lines of manual pub-use statements with glob re-exports.
Explicit Error/Result re-exports disambiguate crate vs core prelude.
Ensures prelude stays in sync as new types are added."
```

---

## Verification

After all tasks are complete, run the full workspace test suite:

```bash
cargo test --workspace 2>&1 | tail -20
```

Expected: all tests pass, no regressions.

## Summary

| Task | What | Lines Removed (est.) | Risk |
|------|------|---------------------|------|
| 1 | Consolidate index building → `rebuild_index` | ~12 | Low |
| 2 | `set_scalar_rate` → `apply_forecast_assign` | ~15 | Low |
| 3 | Merge serial/parallel valuation | ~20 | Low |
| 4 | 5 `UpdateXxxCurve` → 1 `UpdateCurve(CurveStorage)` | ~30 | Medium |
| 5 | Instrument shock handler dedup | ~45 | Medium |
| 6 | Prelude glob re-export | ~25 | Low |
| **Total** | | **~147 lines removed** | |
