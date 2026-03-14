# Portfolio Public API Simplification Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate duplication, dead code, and inconsistency in the `finstack-portfolio` crate's public API surface to make it simpler and more consistent.

**Architecture:** 10 targeted tasks ordered from safe non-breaking changes (dead code removal, visibility fixes) to potentially breaking changes (error variant consolidation, function renames). Each task is independently committable and leaves the workspace green. WASM and Python binding call sites are updated alongside core changes.

**Tech Stack:** Rust, `finstack-portfolio`, `finstack-py`, `finstack-wasm`

---

## Chunk 1: Dead Code & Visibility Fixes (Tasks 1–3)

These are non-breaking changes that remove unused code or tighten visibility.

### Task 1: Delete unused `Error::BuilderError` variant

`BuilderError(String)` and its constructor `Error::builder_error()` are never used anywhere in the portfolio crate. The Python binding has a match arm for it but it's dead code — no portfolio code path ever produces this variant.

**Files:**
- Modify: `finstack/portfolio/src/error.rs:79-80,122-124`
- Modify: `finstack-py/src/portfolio/error.rs:41` (remove match arm)

- [ ] **Step 1: Run existing tests as baseline**

Run: `cargo test -p finstack-portfolio --lib --tests 2>&1 | tail -5`
Expected: all tests pass

- [ ] **Step 2: Remove `BuilderError` variant from error.rs**

In `finstack/portfolio/src/error.rs`, delete these lines:

```rust
    /// Builder construction error
    #[error("Builder error: {0}")]
    BuilderError(String),
```

And delete the constructor:

```rust
    /// Create a builder error
    pub fn builder_error(msg: impl Into<String>) -> Self {
        Self::BuilderError(msg.into())
    }
```

- [ ] **Step 3: Remove dead match arm from Python bindings**

In `finstack-py/src/portfolio/error.rs`, find and remove the match arm:

```rust
        Error::BuilderError(msg) => {
```

and its body. The `#[non_exhaustive]` attribute means a wildcard arm should already exist.

- [ ] **Step 4: Run tests and build downstream**

Run: `cargo test -p finstack-portfolio --lib --tests 2>&1 | tail -5`
Run: `cargo build -p finstack-py 2>&1 | tail -5`
Run: `cargo build -p finstack-wasm 2>&1 | tail -5`
Expected: all pass/compile

- [ ] **Step 5: Commit**

```bash
git add finstack/portfolio/src/error.rs finstack-py/src/portfolio/error.rs
git commit -m "refactor(portfolio): remove unused BuilderError variant

Never constructed anywhere in the crate. Dead match arm in Python
bindings also removed."
```

---

### Task 2: Merge `Error::IndexError` into `Error::InvalidInput`

`IndexError(String)` is used in exactly one place: `finstack/portfolio/src/optimization/decision.rs:113`. It's semantically identical to `InvalidInput`. Merge them.

**Files:**
- Modify: `finstack/portfolio/src/error.rs:83-85,127-129`
- Modify: `finstack/portfolio/src/optimization/decision.rs:113`
- Modify: `finstack-py/src/portfolio/error.rs` (remove match arm if present)

- [ ] **Step 1: Find the single usage of `IndexError`**

In `finstack/portfolio/src/optimization/decision.rs`, line 113, change:

```rust
Error::index_error(format!(
```

to:

```rust
Error::invalid_input(format!(
```

- [ ] **Step 2: Remove `IndexError` variant and constructor from error.rs**

Delete from `finstack/portfolio/src/error.rs`:

```rust
    /// Index/collection access error
    #[error("Index error: {0}")]
    IndexError(String),
```

And the constructor:

```rust
    /// Create an index error
    pub fn index_error(msg: impl Into<String>) -> Self {
        Self::IndexError(msg.into())
    }
```

- [ ] **Step 3: Remove from Python bindings if present**

Check `finstack-py/src/portfolio/error.rs` for an `IndexError` match arm and remove it if found.

- [ ] **Step 4: Run tests and build downstream**

Run: `cargo test -p finstack-portfolio --lib --tests 2>&1 | tail -5`
Run: `cargo build -p finstack-py 2>&1 | tail -5`
Run: `cargo build -p finstack-wasm 2>&1 | tail -5`
Expected: all pass/compile

- [ ] **Step 5: Commit**

```bash
git add finstack/portfolio/src/error.rs finstack/portfolio/src/optimization/decision.rs finstack-py/src/portfolio/error.rs
git commit -m "refactor(portfolio): merge IndexError into InvalidInput

IndexError was used in exactly one place. Semantically identical to
InvalidInput. Reduces error variant count."
```

---

### Task 3: Remove redundant `Book` accessor methods

`Book::positions()` returns `&self.position_ids` and `Book::children()` returns `&self.child_book_ids`. Both fields are already `pub`. These methods are pure aliases that duplicate field access.

**Files:**
- Modify: `finstack/portfolio/src/book.rs:223-232`
- Modify: `finstack/portfolio/tests/book_hierarchy_test.rs` (update call sites)
- Modify: `finstack/portfolio/src/book.rs` (tests at bottom)

- [ ] **Step 1: Find all call sites of `.positions()` and `.children()` on Book**

These are used in:
- `finstack/portfolio/src/book.rs` tests (lines 259, 260, 278, 282, 292, 296, 309, 322)
- `finstack/portfolio/tests/book_hierarchy_test.rs` (lines 168, 169, 174, 175, 180, 183, 184, 185, 403, 404, 454, 455, 456)

- [ ] **Step 2: Replace all `.positions()` with `.position_ids` and `.children()` with `.child_book_ids`**

In all files listed above, replace:
- `book.positions()` → `book.position_ids`  (when used as slice: `&book.position_ids`)
- `book.children()` → `book.child_book_ids` (when used as slice: `&book.child_book_ids`)

Note: `.positions()` returned `&[PositionId]` while `.position_ids` is `Vec<PositionId>`. For `.len()` and indexing, `Vec` works the same. For slice comparison like `== &[...]`, use `book.child_book_ids.as_slice()` or `&book.child_book_ids[..]`.

- [ ] **Step 3: Delete the two methods from Book impl**

Remove from `finstack/portfolio/src/book.rs`:

```rust
    /// Get all positions in this book (non-recursive).
    pub fn positions(&self) -> &[PositionId] {
        &self.position_ids
    }

    /// Get all child books (non-recursive).
    pub fn children(&self) -> &[BookId] {
        &self.child_book_ids
    }
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p finstack-portfolio --lib --tests 2>&1 | tail -5`
Expected: all pass

- [ ] **Step 5: Commit**

```bash
git add finstack/portfolio/src/book.rs finstack/portfolio/tests/book_hierarchy_test.rs
git commit -m "refactor(portfolio): remove redundant Book::positions/children accessors

Both fields are pub. The methods were pure aliases adding no value."
```

---

## Chunk 2: API Consistency Fixes (Tasks 4–6)

Fix inconsistencies in what's re-exported from the crate root.

### Task 4: Re-export missing cashflow types from `lib.rs`

`collapse_cashflows_to_base_by_date`, `cashflows_to_base_by_period`, and `PortfolioCashflowBuckets` are public and used by Python/WASM bindings but not re-exported from `lib.rs`. The other cashflow types (`aggregate_cashflows`, `PortfolioCashflows`) are re-exported.

**Files:**
- Modify: `finstack/portfolio/src/lib.rs:115`

- [ ] **Step 1: Update the cashflows re-export line**

In `finstack/portfolio/src/lib.rs`, change line 115:

```rust
pub use cashflows::{aggregate_cashflows, PortfolioCashflows};
```

to:

```rust
pub use cashflows::{
    aggregate_cashflows, cashflows_to_base_by_period, collapse_cashflows_to_base_by_date,
    PortfolioCashflowBuckets, PortfolioCashflows,
};
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p finstack-portfolio --lib --tests 2>&1 | tail -5`
Expected: all pass

- [ ] **Step 3: Commit**

```bash
git add finstack/portfolio/src/lib.rs
git commit -m "refactor(portfolio): re-export missing cashflow types from crate root

collapse_cashflows_to_base_by_date, cashflows_to_base_by_period, and
PortfolioCashflowBuckets were public but only accessible via the
cashflows module path."
```

---

### Task 5: Re-export missing grouping function from `lib.rs`

`aggregate_by_multiple_attributes` is public in `grouping.rs` and used in tests and Python bindings, but not re-exported from `lib.rs`. The other grouping functions are all re-exported.

**Files:**
- Modify: `finstack/portfolio/src/lib.rs:118`

- [ ] **Step 1: Update the grouping re-export line**

In `finstack/portfolio/src/lib.rs`, change line 118:

```rust
pub use grouping::{aggregate_by_attribute, aggregate_by_book, group_by_attribute};
```

to:

```rust
pub use grouping::{
    aggregate_by_attribute, aggregate_by_book, aggregate_by_multiple_attributes,
    group_by_attribute,
};
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p finstack-portfolio --lib --tests 2>&1 | tail -5`
Expected: all pass

- [ ] **Step 3: Commit**

```bash
git add finstack/portfolio/src/lib.rs
git commit -m "refactor(portfolio): re-export aggregate_by_multiple_attributes from crate root

Was public in grouping module but inconsistently missing from lib.rs
re-exports."
```

---

### Task 6: Re-export `PortfolioValuationOptions` and `value_portfolio_with_options` from `lib.rs`

These are public in `valuation.rs` and used by the optimization module and integration tests, but not re-exported from `lib.rs`. Users who need custom metric sets or strict risk mode must import via the module path.

**Files:**
- Modify: `finstack/portfolio/src/lib.rs:135`

- [ ] **Step 1: Update the valuation re-export line**

In `finstack/portfolio/src/lib.rs`, change line 135:

```rust
pub use valuation::{revalue_affected, value_portfolio, PortfolioValuation, PositionValue};
```

to:

```rust
pub use valuation::{
    revalue_affected, value_portfolio, value_portfolio_with_options, PortfolioValuation,
    PortfolioValuationOptions, PositionValue,
};
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p finstack-portfolio --lib --tests 2>&1 | tail -5`
Expected: all pass

- [ ] **Step 3: Commit**

```bash
git add finstack/portfolio/src/lib.rs
git commit -m "refactor(portfolio): re-export PortfolioValuationOptions and value_portfolio_with_options

Both were public in valuation module but not accessible from crate root."
```

---

## Chunk 3: Reduce Optimization Re-Export Bloat (Task 7)

### Task 7: Narrow optimization re-exports from `lib.rs`

The optimization module re-exports 16 types through `lib.rs`. Most users won't use optimization. Keep only the high-level entry points at the crate root; users who need the full API can import via `finstack_portfolio::optimization::*`.

**Files:**
- Modify: `finstack/portfolio/src/lib.rs:124-129`
- Check: `finstack-py/src/portfolio/optimization.rs` and `finstack-wasm/src/portfolio/optimization.rs` — these import via `finstack_portfolio::optimization::*` already, so they're unaffected.

- [ ] **Step 1: Verify WASM and Python import paths**

Check that `finstack-py/src/portfolio/optimization.rs` and `finstack-wasm/src/portfolio/optimization.rs` use `finstack_portfolio::optimization::*` style imports (not `finstack_portfolio::CandidatePosition` etc.). If they use crate-root imports, those must be updated to use the module path.

Run: `grep "use finstack_portfolio::" finstack-py/src/portfolio/optimization.rs finstack-wasm/src/portfolio/optimization.rs`

- [ ] **Step 2: Narrow the optimization re-exports**

In `finstack/portfolio/src/lib.rs`, replace lines 124-129:

```rust
pub use optimization::{
    optimize_max_yield_with_ccc_limit, CandidatePosition, Constraint, DefaultLpOptimizer,
    Inequality, MaxYieldWithCccLimitResult, MetricExpr, MissingMetricPolicy, Objective,
    PerPositionMetric, PortfolioOptimizationProblem, PortfolioOptimizationResult, PositionFilter,
    TradeDirection, TradeSpec, TradeType, TradeUniverse, WeightingScheme,
};
```

with:

```rust
pub use optimization::{
    optimize_max_yield_with_ccc_limit, MaxYieldWithCccLimitResult,
    PortfolioOptimizationProblem, PortfolioOptimizationResult,
};
```

The remaining 12 types (`CandidatePosition`, `Constraint`, `DefaultLpOptimizer`, `Inequality`, `MetricExpr`, `MissingMetricPolicy`, `Objective`, `PerPositionMetric`, `PositionFilter`, `TradeDirection`, `TradeSpec`, `TradeType`, `TradeUniverse`, `WeightingScheme`) remain accessible via `finstack_portfolio::optimization::*`.

- [ ] **Step 3: Fix any downstream compilation errors**

Check that no code uses the crate-root path for the removed re-exports:

Run: `cargo build -p finstack-portfolio -p finstack-py -p finstack-wasm 2>&1 | grep "error" | head -20`

If any errors, update the import to use `finstack_portfolio::optimization::TypeName` instead.

Also check examples and benches:

Run: `grep -rn "finstack_portfolio::{" finstack/portfolio/examples/ finstack/portfolio/benches/ | grep -E "CandidatePosition|Constraint|DefaultLpOptimizer|Inequality|MetricExpr|MissingMetricPolicy|Objective|PerPositionMetric|PositionFilter|TradeDirection|TradeSpec|TradeType|TradeUniverse|WeightingScheme"`

Update any found to import from `finstack_portfolio::optimization::*`.

- [ ] **Step 4: Run tests**

Run: `cargo test -p finstack-portfolio --lib --tests 2>&1 | tail -5`
Expected: all pass

- [ ] **Step 5: Commit**

```bash
git add finstack/portfolio/src/lib.rs
git commit -m "refactor(portfolio): narrow optimization re-exports from crate root

Only re-export the 4 high-level optimization entry points from the
crate root. The full 16-type API remains accessible via the
optimization module for users who need it."
```

---

## Chunk 4: Consolidate `Book` Constructor Pattern (Task 8)

### Task 8: Replace `Book::with_parent` with `Book::new().with_parent()`

`Book::with_parent()` is a second constructor that creates a book with a parent. This is inconsistent with `Entity` and `Position` which use the `with_*` builder pattern. Replace with a chainable method.

**Files:**
- Modify: `finstack/portfolio/src/book.rs:137-158` (replace `with_parent` static method with chainable instance method)
- Modify: `finstack/portfolio/src/book.rs` (tests)
- Modify: `finstack/portfolio/tests/book_hierarchy_test.rs` (update call sites)
- Modify: `finstack-py/src/portfolio/book.rs:82` (update call site)

- [ ] **Step 1: Replace the static `with_parent` method with a chainable `with_parent` method**

In `finstack/portfolio/src/book.rs`, replace:

```rust
    /// Create a new book with a parent.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique book identifier.
    /// * `name` - Optional human-readable name.
    /// * `parent_id` - Parent book identifier.
    pub fn with_parent(
        id: impl Into<BookId>,
        name: Option<String>,
        parent_id: impl Into<BookId>,
    ) -> Self {
        Self {
            id: id.into(),
            name,
            parent_id: Some(parent_id.into()),
            position_ids: Vec::new(),
            child_book_ids: Vec::new(),
            tags: IndexMap::new(),
            meta: IndexMap::new(),
        }
    }
```

with:

```rust
    /// Set the parent book, returning self for chaining.
    ///
    /// # Arguments
    ///
    /// * `parent_id` - Parent book identifier.
    pub fn with_parent(mut self, parent_id: impl Into<BookId>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }
```

- [ ] **Step 2: Update all call sites**

Replace all `Book::with_parent(id, name, parent)` calls with `Book::new(id, name).with_parent(parent)`.

In `finstack/portfolio/src/book.rs` tests (line 265):

```rust
// Before:
let book = Book::with_parent("credit", Some("Credit".to_string()), "americas");
// After:
let book = Book::new("credit", Some("Credit".to_string())).with_parent("americas");
```

In `finstack/portfolio/tests/book_hierarchy_test.rs` (lines 132, 133, 381, 382):

```rust
// Before:
let credit = Book::with_parent("credit", Some("Credit".to_string()), "americas");
let ig = Book::with_parent("ig", Some("Investment Grade".to_string()), "credit");
// After:
let credit = Book::new("credit", Some("Credit".to_string())).with_parent("americas");
let ig = Book::new("ig", Some("Investment Grade".to_string())).with_parent("credit");
```

In `finstack-py/src/portfolio/book.rs` (line 82):

```rust
// Before:
Some(parent) => Book::with_parent(book_id, name, extract_book_id(parent)?),
// After:
Some(parent) => Book::new(book_id, name).with_parent(extract_book_id(parent)?),
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p finstack-portfolio --lib --tests 2>&1 | tail -5`
Run: `cargo build -p finstack-py 2>&1 | tail -5`
Expected: all pass/compile

- [ ] **Step 4: Commit**

```bash
git add finstack/portfolio/src/book.rs finstack/portfolio/tests/book_hierarchy_test.rs finstack-py/src/portfolio/book.rs
git commit -m "refactor(portfolio): replace Book::with_parent constructor with chainable method

Aligns Book API with Entity and Position which use the with_* builder
pattern. Book::with_parent(id, name, parent) becomes
Book::new(id, name).with_parent(parent)."
```

---

## Chunk 5: Prelude Deduplication (Task 9)

### Task 9: Make `prelude.rs` delegate to crate root instead of duplicating all re-exports

Every `pub use` in `lib.rs` is copied line-for-line in `prelude.rs`. Any new type added to `lib.rs` must be manually added to `prelude.rs`. Fix by glob-importing from the crate root.

**Important:** Both `crate::*` and `finstack_core::prelude::*` export `Error` and `Result`. Explicit re-exports take precedence over globs, resolving the ambiguity.

**Files:**
- Modify: `finstack/portfolio/src/prelude.rs`

- [ ] **Step 1: Run existing tests as baseline**

Run: `cargo test -p finstack-portfolio --lib --tests 2>&1 | tail -5`
Expected: all tests pass

- [ ] **Step 2: Replace `prelude.rs` contents**

Replace the entire file body (keeping the doc comment) with:

```rust
//! Commonly used types and functions.
//!
//! Import this module to get quick access to the most common types:
//!
//! ```rust
//! use finstack_portfolio::prelude::*;
//! ```

// Re-export the full core prelude for a unified foundation.
pub use finstack_core::prelude::*;

// Re-export everything from the crate root.
pub use crate::*;

// Explicit re-exports to disambiguate names that appear in both
// `crate::*` and `finstack_core::prelude::*`.
pub use crate::error::{Error, Result};
```

- [ ] **Step 3: Run tests and build downstream**

Run: `cargo test -p finstack-portfolio --lib --tests 2>&1 | tail -5`
Run: `cargo build -p finstack-portfolio --features scenarios,dataframes 2>&1 | tail -5`
Expected: all pass

- [ ] **Step 4: Commit**

```bash
git add finstack/portfolio/src/prelude.rs
git commit -m "refactor(portfolio): prelude delegates to crate root instead of duplicating re-exports

Replaces 30+ lines of manual pub-use with glob re-export from crate
root. Explicit Error/Result re-exports resolve ambiguity with
finstack_core::prelude."
```

---

## Chunk 6: Consolidate Valuation Entry Point (Task 10)

### Task 10: Remove `value_portfolio` wrapper — rename `value_portfolio_with_options` to `value_portfolio`

`value_portfolio` is a 4-line wrapper that calls `value_portfolio_with_options` with default options. Since `PortfolioValuationOptions` implements `Default`, this wrapper adds an extra name with no ergonomic win. Remove the wrapper and rename the full function.

**This is a breaking change.** All call sites of `value_portfolio(portfolio, market, config)` must add `, &PortfolioValuationOptions::default()` or `, &Default::default()`.

**Files:**
- Modify: `finstack/portfolio/src/valuation.rs` (delete wrapper, rename function)
- Modify: `finstack/portfolio/src/lib.rs` (update re-export)
- Modify: All call sites (listed below)

**Call sites to update** (each adds `&Default::default()` as 4th arg):

Internal (portfolio crate):
- `finstack/portfolio/src/valuation.rs` tests (2 sites)
- `finstack/portfolio/src/grouping.rs` test (1 site) + doc example
- `finstack/portfolio/src/metrics.rs` test (1 site)
- `finstack/portfolio/src/results.rs` test (1 site)
- `finstack/portfolio/src/dataframe.rs` tests (2 sites)
- `finstack/portfolio/src/scenarios.rs:126` (internal call)

Integration tests:
- `finstack/portfolio/tests/selective_repricing.rs` (5 sites)
- `finstack/portfolio/tests/integration_scenarios.rs` (1 site)
- `finstack/portfolio/tests/valuation_fallback.rs` (1 site)

Benchmarks:
- `finstack/portfolio/benches/portfolio_valuation.rs` (5 sites)

Examples:
- `finstack/portfolio/examples/portfolio_optimization.rs` (1 site)

WASM bindings:
- `finstack-wasm/src/portfolio/valuation.rs` (`js_value_portfolio` calls it)

Python bindings:
- Check `finstack-py/src/portfolio/valuation.rs` for call sites

- [ ] **Step 1: In `valuation.rs`, delete the `value_portfolio` wrapper function**

Remove the entire `value_portfolio` function (lines 224-240).

- [ ] **Step 2: Rename `value_portfolio_with_options` to `value_portfolio`**

In `finstack/portfolio/src/valuation.rs`, rename the function:

```rust
pub fn value_portfolio(
    portfolio: &Portfolio,
    market: &MarketContext,
    config: &FinstackConfig,
    options: &PortfolioValuationOptions,
) -> Result<PortfolioValuation> {
```

- [ ] **Step 3: Update lib.rs re-export**

In `finstack/portfolio/src/lib.rs`, update the valuation re-export to remove `value_portfolio_with_options`:

```rust
pub use valuation::{
    revalue_affected, value_portfolio, PortfolioValuation,
    PortfolioValuationOptions, PositionValue,
};
```

- [ ] **Step 4: Update all internal call sites**

For every `value_portfolio(portfolio, market, config)` call, change to:

```rust
value_portfolio(portfolio, market, config, &Default::default())
```

For `value_portfolio_with_options(...)` calls (optimization module), rename to `value_portfolio(...)` (args unchanged).

Work through each file listed above systematically. Use `cargo build -p finstack-portfolio 2>&1 | grep "error"` to find remaining sites.

- [ ] **Step 5: Update WASM bindings**

In `finstack-wasm/src/portfolio/valuation.rs`, update `js_value_portfolio` to call:

```rust
finstack_portfolio::value_portfolio(&portfolio.inner, market_ctx, cfg_ref, &Default::default())
```

- [ ] **Step 6: Update Python bindings**

Check and update `finstack-py/src/portfolio/valuation.rs` similarly.

- [ ] **Step 7: Run full test suite**

Run: `cargo test -p finstack-portfolio --lib --tests 2>&1 | tail -5`
Run: `cargo build -p finstack-py -p finstack-wasm 2>&1 | tail -10`
Expected: all pass/compile

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "refactor(portfolio): consolidate value_portfolio and value_portfolio_with_options

Remove the default-options wrapper. Callers who want defaults pass
&Default::default(). Reduces API surface to a single valuation
entry point."
```

---

## Verification

After all tasks are complete, run the full workspace test suite:

```bash
cargo test --workspace 2>&1 | tail -20
```

Expected: all tests pass, no regressions.

## Summary

| Task | What | Type | Risk |
|------|------|------|------|
| 1 | Delete unused `BuilderError` variant | Dead code | Low |
| 2 | Merge `IndexError` into `InvalidInput` | Dead code | Low |
| 3 | Remove redundant `Book` accessors | Duplication | Low |
| 4 | Re-export missing cashflow types | Consistency | Low |
| 5 | Re-export missing grouping function | Consistency | Low |
| 6 | Re-export valuation options types | Consistency | Low |
| 7 | Narrow optimization re-exports | API bloat | Medium |
| 8 | Consolidate `Book::with_parent` pattern | Consistency | Low |
| 9 | Prelude delegates to crate root | Duplication | Low |
| 10 | Merge `value_portfolio` + `_with_options` | Duplication | Medium |
