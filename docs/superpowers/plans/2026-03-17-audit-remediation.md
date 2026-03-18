# Audit Remediation Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all production-readiness issues identified in the comprehensive crate audit, prioritized by severity.

**Architecture:** Changes are isolated per-crate with no cross-crate dependencies between tasks. Each task targets a specific finding: WASM panic/correctness fixes first (safety-critical), then statements performance, then valuations/core/portfolio improvements. All changes follow existing patterns (TDD, `#![deny(clippy::unwrap_used)]`, `thiserror` errors).

**Tech Stack:** Rust, wasm-bindgen, wasm-bindgen-test, cargo-nextest, PyO3

---

## Chunk 1: WASM Correctness Fixes (Safety-Critical)

These fixes prevent unrecoverable WASM traps and silent data corruption in financial calculations.

### Task 1: Replace `panic!` with Result in WASM math callbacks

The two `panic!("JS callback error")` calls in `integration.rs` and `solver.rs` use a panic-catch pattern (`run_with_panic_catch`) to propagate JS errors. While the panics are caught, this pattern is fragile — if `catch_unwind` fails or the panic hook does something unexpected, the WASM instance becomes unrecoverable. The fix replaces the panic-based control flow with a `Result`-based approach using a sentinel return value + post-check.

**Files:**
- Modify: `finstack-wasm/src/core/math/integration.rs:22-55`
- Modify: `finstack-wasm/src/core/math/solver.rs:21-55`
- Test: `finstack-wasm/tests/` (existing parity tests)

- [ ] **Step 1: Understand the current panic-catch pattern**

Read both files to understand how `JsClosureAdapter`, `run_with_panic_catch`, and the calling code interact. The pattern is:
1. `JsClosureAdapter::invoke()` calls a JS function
2. If JS returns an error, it stores the error in `error_cell` and panics
3. `run_with_panic_catch` catches the panic and returns the error from `error_cell`

The fix: Instead of panicking, return `f64::NAN` as a sentinel value. The calling code (integration/solver) already checks for finite values, so NAN will halt the computation. After the computation, check `error_cell` for any stored JS error.

- [ ] **Step 2: Refactor `integration.rs` — replace panic with NAN sentinel**

In `finstack-wasm/src/core/math/integration.rs`, replace:

```rust
#[allow(clippy::panic)]
impl JsClosureAdapter<'_> {
    fn invoke(&self, x: f64) -> f64 {
        match call_js_fn_safe(self.func, x) {
            Ok(value) => value,
            Err(err) => {
                *self.error_cell.borrow_mut() = Some(err);
                panic!("JS callback error");
            }
        }
    }
}
```

With:

```rust
impl JsClosureAdapter<'_> {
    fn invoke(&self, x: f64) -> f64 {
        match call_js_fn_safe(self.func, x) {
            Ok(value) => value,
            Err(err) => {
                *self.error_cell.borrow_mut() = Some(err);
                f64::NAN // Sentinel — caller checks error_cell after computation
            }
        }
    }
}
```

Remove the `#[allow(clippy::panic)]` attribute from the impl block.

Then replace the `run_with_panic_catch` helper and **all its call sites** with `run_with_error_check`. There are 7 call sites in `integration.rs` (lines ~202, ~283, ~329, ~372, ~418, ~473, ~514) and 2 in `solver.rs` (lines ~198, ~382). Replace the helper:

```rust
fn run_with_error_check<R>(
    error_cell: &RefCell<Option<JsValue>>,
    eval: impl FnOnce() -> R,
) -> Result<R, JsValue> {
    let result = eval();
    if let Some(err) = error_cell.borrow_mut().take() {
        return Err(err);
    }
    Ok(result)
}
```

Then update every `run_with_panic_catch(` call to `run_with_error_check(` — the signature is identical so only the function name changes at each call site.

- [ ] **Step 3: Apply the same refactor to `solver.rs`**

The code in `finstack-wasm/src/core/math/solver.rs` is identical. Apply the same changes:
- Replace `panic!("JS callback error")` with `f64::NAN`
- Remove `#[allow(clippy::panic)]`
- Replace `run_with_panic_catch` helper with `run_with_error_check`
- Update both call sites (~lines 198, 382) to use the new function name

- [ ] **Step 4: Remove `std::panic::catch_unwind` imports if no longer used**

Check both files for unused imports after the refactor. Remove `use std::panic::{catch_unwind, AssertUnwindSafe};` if present and no longer needed.

- [ ] **Step 5: Run WASM tests**

Run: `make test-wasm`
Expected: All existing tests pass. The NAN sentinel propagates correctly through integration/solver algorithms because they already check for finite intermediate values.

- [ ] **Step 6: Run clippy**

Run: `cd finstack-wasm && cargo clippy --all-targets -- -D warnings`
Expected: No warnings (the `#[allow(clippy::panic)]` removal should not cause new warnings since the panic is gone).

- [ ] **Step 7: Commit**

```bash
git add finstack-wasm/src/core/math/integration.rs finstack-wasm/src/core/math/solver.rs
git commit -m "fix(wasm): replace panic! with NAN sentinel in JS callback adapters

Panics in WASM become unrecoverable traps. Replace the panic-catch
pattern with a NAN sentinel + post-check pattern that achieves the
same error propagation without risking WASM instance corruption."
```

---

### Task 2: Replace `.expect()` calls with `.ok_or_else()` in WASM

Three `.expect()` calls can panic at the WASM boundary, causing unrecoverable traps.

**Files:**
- Modify: `finstack-wasm/src/core/math/linalg.rs:102`
- Modify: `finstack-wasm/src/valuations/metrics/ids.rs:70`
- Modify: `finstack-wasm/src/valuations/metrics/registry.rs:138`

- [ ] **Step 1: Fix `linalg.rs` expect**

In `finstack-wasm/src/core/math/linalg.rs`, line 102, replace:

```rust
apply_correlation(chol, independent, &mut correlated)
    .expect("apply_correlation: dimensions pre-validated");
```

With:

```rust
apply_correlation(chol, independent, &mut correlated)
    .map_err(|e| js_error(format!("apply_correlation failed: {e}")))?;
```

Ensure the enclosing function returns `Result<_, JsValue>`. If it doesn't, adjust the signature.

- [ ] **Step 2: Fix `ids.rs` expect**

In `finstack-wasm/src/valuations/metrics/ids.rs`, line 70, replace:

```rust
.expect("MetricId::from_str never fails, creates Custom for unknown names")
```

With:

```rust
.unwrap_or_else(|_| MetricId::Custom(s.to_string()))
```

This preserves the intent (MetricId::from_str always succeeds via Custom fallback) but avoids a panic path entirely. Alternatively, if `MetricId::from_str` truly cannot fail, use `unwrap_or_else` with the same Custom fallback to make the intent explicit without risk.

- [ ] **Step 3: Fix `registry.rs` expect**

In `finstack-wasm/src/valuations/metrics/registry.rs`, line 138, apply the same fix as Step 2:

```rust
.unwrap_or_else(|_| MetricId::Custom(name.to_string()))
```

- [ ] **Step 4: Run WASM tests and clippy**

Run: `make test-wasm && cd finstack-wasm && cargo clippy --all-targets -- -D warnings`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
git add finstack-wasm/src/core/math/linalg.rs finstack-wasm/src/valuations/metrics/ids.rs finstack-wasm/src/valuations/metrics/registry.rs
git commit -m "fix(wasm): replace .expect() calls with fallible alternatives

Expect panics become unrecoverable WASM traps. Replace with
map_err/?  or unwrap_or_else to keep the same behavior without
panic risk."
```

---

### Task 3: Replace `Decimal::from_f64_retain(...).unwrap_or_default()` with explicit errors

Silent NaN→0 conversion is a correctness risk for financial software. The Python bindings already handle this correctly with explicit error messages.

There are two categories of `unwrap_or_default()` in the WASM crate:
- **`Decimal::from_f64_retain(x).unwrap_or_default()`** — f64→Decimal (input path). NaN/Infinity silently becomes zero. **This is the dangerous pattern.** 8 instances across 2 files.
- **`ToPrimitive::to_f64(&decimal).unwrap_or_default()`** — Decimal→f64 (output/getter path). These are getters returning rates/strikes to JS. Failure is extremely unlikely (any Decimal that fits in 128 bits fits in f64 for financial values). **Lower priority but should also be addressed.**

**Files (f64→Decimal, high priority):**
- Modify: `finstack-wasm/src/valuations/cashflow/builder.rs` (6 instances: lines 72, 73, 170, 209, 450, 451)
- Modify: `finstack-wasm/src/valuations/instruments/irs.rs` (3 instances: lines 192, 196, 357)

**Files (Decimal→f64 getters, lower priority):**
- Modify: `finstack-wasm/src/valuations/instruments/bond.rs` (4 instances: lines 169, 202, 543, 581)
- Modify: `finstack-wasm/src/valuations/instruments/commodity_swap.rs` (line 369)
- Modify: `finstack-wasm/src/valuations/instruments/swaption.rs` (line 422)
- Modify: `finstack-wasm/src/valuations/instruments/fra.rs` (line 331)
- Modify: `finstack-wasm/src/valuations/instruments/inflation_linked_bond.rs` (line 315)
- Modify: `finstack-wasm/src/valuations/instruments/inflation_cap_floor.rs` (line 340)
- Modify: `finstack-wasm/src/valuations/instruments/inflation_swap.rs` (lines 278, 314)
- Modify: `finstack-wasm/src/valuations/instruments/repo.rs` (line 344)
- Modify: `finstack-wasm/src/valuations/instruments/yoy_inflation_swap.rs` (lines 217, 321)
- Modify: `finstack-wasm/src/valuations/instruments/cap_floor.rs` (line 322)
- Modify: `finstack-wasm/src/valuations/instruments/bond_future.rs` (line 421)

- [ ] **Step 1: Create helper functions**

Add helpers in `finstack-wasm/src/utils/decimal.rs` (create if needed, re-export from `utils/mod.rs`):

```rust
use wasm_bindgen::JsValue;
use crate::core::error::js_error;

/// Convert f64 to Decimal, returning a JS error for non-finite values.
pub(crate) fn decimal_from_f64(value: f64, field_name: &str) -> Result<rust_decimal::Decimal, JsValue> {
    rust_decimal::Decimal::from_f64_retain(value).ok_or_else(|| {
        js_error(format!(
            "{field_name}: cannot convert {value} to Decimal (NaN or Infinity)"
        ))
    })
}

/// Convert Decimal to f64, returning 0.0 only for values that genuinely are zero.
/// Returns a JS error if the Decimal cannot be represented as f64 (should never happen
/// for financial values, but avoids silent data loss).
pub(crate) fn decimal_to_f64(value: &rust_decimal::Decimal, field_name: &str) -> Result<f64, JsValue> {
    rust_decimal::prelude::ToPrimitive::to_f64(value).ok_or_else(|| {
        js_error(format!(
            "{field_name}: cannot convert Decimal {value} to f64"
        ))
    })
}
```

- [ ] **Step 2: Replace f64→Decimal instances in `cashflow/builder.rs`**

Replace each `Decimal::from_f64_retain(x).unwrap_or_default()` with `decimal_from_f64(x, "field")?`. Repeat for all 6 instances (lines 72, 73, 170, 209, 450, 451). Ensure enclosing functions return `Result<_, JsValue>`.

- [ ] **Step 3: Replace f64→Decimal instances in `instruments/irs.rs`**

Replace all 3 instances (lines 192, 196, 357) with `decimal_from_f64(value, "field")?`.

- [ ] **Step 4: Replace Decimal→f64 getter instances across all instrument files**

For each `ToPrimitive::to_f64(&self.inner.field).unwrap_or_default()`, replace with `decimal_to_f64(&self.inner.field, "field")?`. This requires changing getter return types from `f64` to `Result<f64, JsValue>` — verify that wasm-bindgen supports `Result` returns on `#[wasm_bindgen(getter)]` methods. If not, use `unwrap_or(0.0)` with a tracing::warn for the getter case only.

Update all ~15 files listed above.

- [ ] **Step 5: Run WASM tests and clippy**

Run: `make test-wasm && cd finstack-wasm && cargo clippy --all-targets -- -D warnings`
Expected: All pass.

- [ ] **Step 6: Commit**

```bash
git add finstack-wasm/src/utils/ finstack-wasm/src/valuations/
git commit -m "fix(wasm): reject NaN/Infinity in Decimal conversions instead of silent zero

Replace all Decimal::from_f64_retain().unwrap_or_default() (8 input
sites) and ToPrimitive::to_f64().unwrap_or_default() (15+ getter
sites) with explicit error handling. Matches Python binding behavior."
```

---

## Chunk 2: Statements Crate Fixes

### Task 4: Add recursion depth limit to DSL parser

The recursive descent parser has no depth limit. Deeply nested expressions like `if(if(if(...)))` × 1000 will overflow the stack. Add a depth counter threaded through the recursive calls.

**Files:**
- Modify: `finstack/statements/src/dsl/parser.rs`
- Test: `finstack/statements/tests/dsl/` (add a test for deep nesting)

- [ ] **Step 1: Write a failing test for deep recursion**

Create or add to an existing test file in `finstack/statements/tests/`:

```rust
#[test]
fn parser_rejects_deeply_nested_expressions() {
    // Build a deeply nested expression: if(1, if(1, if(1, ... , 0) ... ))
    let depth = 300;
    let mut expr = String::new();
    for _ in 0..depth {
        expr.push_str("if(1, ");
    }
    expr.push('0');
    for _ in 0..depth {
        expr.push_str(", 0)");
    }
    let result = finstack_statements::dsl::parse_formula(&expr);
    assert!(result.is_err(), "Should reject expressions nested beyond depth limit");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("depth") || err_msg.contains("nesting"),
        "Error should mention depth/nesting limit, got: {err_msg}"
    );
}
```

- [ ] **Step 2: Run test to verify it fails (stack overflow or passes when it shouldn't)**

Run: `cargo nextest run -p finstack-statements parser_rejects_deeply_nested_expressions`
Expected: Either stack overflow (segfault) or the test fails because the parse succeeds. Both confirm the bug.

- [ ] **Step 3: Add depth tracking to the parser**

The parser uses **nom combinators with free functions** (not a Parser struct). The entry point is:

```rust
fn expression(input: &str) -> IResult<&str, StmtExpr> {
    logical_or(input)
}
```

Since nom free functions don't carry state, use a thread-local `Cell<usize>` for the depth counter. This avoids changing the signature of every combinator function:

```rust
use std::cell::Cell;

const MAX_PARSE_DEPTH: usize = 256;

thread_local! {
    static PARSE_DEPTH: Cell<usize> = const { Cell::new(0) };
}

/// RAII guard that increments depth on creation and decrements on drop.
struct DepthGuard;

impl DepthGuard {
    fn enter() -> Result<Self, nom::Err<nom::error::Error<&'static str>>> {
        PARSE_DEPTH.with(|d| {
            let current = d.get();
            if current >= MAX_PARSE_DEPTH {
                return Err(nom::Err::Failure(nom::error::Error::new(
                    "", // input not available here, but error message is clear
                    nom::error::ErrorKind::TooLarge,
                )));
            }
            d.set(current + 1);
            Ok(DepthGuard)
        })
    }
}

impl Drop for DepthGuard {
    fn drop(&mut self) {
        PARSE_DEPTH.with(|d| d.set(d.get() - 1));
    }
}
```

Then instrument `expression()` — the single re-entry point for all recursive calls:

```rust
fn expression(input: &str) -> IResult<&str, StmtExpr> {
    let _guard = DepthGuard::enter()
        .map_err(|_| nom::Err::Failure(nom::error::Error::new(
            input,
            nom::error::ErrorKind::TooLarge,
        )))?;
    logical_or(input)
}
```

Also reset the depth counter in `parse_formula()` before starting:

```rust
pub fn parse_formula(input: &str) -> Result<StmtExpr> {
    PARSE_DEPTH.with(|d| d.set(0)); // Reset for each top-level parse
    match expression(input) {
        // ... existing match arms ...
        Err(nom::Err::Failure(e)) if e.code == nom::error::ErrorKind::TooLarge => {
            Err(Error::formula_parse("Expression nesting depth exceeds limit (256)".to_string()))
        }
        Err(e) => Err(Error::formula_parse(format!("Parse error: {}", e))),
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo nextest run -p finstack-statements parser_rejects_deeply_nested_expressions`
Expected: PASS — parser returns a depth error.

- [ ] **Step 5: Run all statements tests**

Run: `cargo nextest run -p finstack-statements`
Expected: All existing tests still pass (normal expressions don't hit 256 depth).

- [ ] **Step 6: Commit**

```bash
git add finstack/statements/src/dsl/parser.rs finstack/statements/tests/
git commit -m "fix(statements): add recursion depth limit to DSL parser

Prevents stack overflow on adversarial input with deeply nested
expressions. Limit set to 256 levels, which exceeds any practical
financial model formula."
```

---

### Task 5: Eliminate per-period `historical.clone()` in evaluator

At line 652 of `evaluator/engine.rs`, the entire historical data `IndexMap` is cloned into an `Arc` for every period evaluation. For 100 periods with 500 line items, this is O(P × N) unnecessary allocations.

**Files:**
- Modify: `finstack/statements/src/evaluator/engine.rs:649-653`

- [ ] **Step 1: Understand the data flow**

Read `evaluate_period` (line 638) and its caller to understand:
1. How `historical` grows — it accumulates results from prior periods
2. Whether `EvaluationContext` needs owned data or just a read reference
3. Whether `EvaluationContext` is used across threads (if so, `Arc` is needed)

The `historical` map grows as each period is evaluated — period N's results are added before period N+1 starts. The `EvaluationContext` wraps it in `Arc` for shared access within a single period's evaluation.

- [ ] **Step 2: Identify the caller's loop and data flow**

Find the function that calls `evaluate_period` in a loop. Read it to understand:
- How `historical` is built up (is it an `&IndexMap` borrowed from a growing local?)
- Whether `EvaluationContext` holds the `Arc` beyond `evaluate_period`'s return
- Whether the `Arc` is shared with other threads (would prevent `Arc::make_mut` from working)

The signature of `evaluate_period` takes `historical: &IndexMap<...>`. The change will:
1. Move the `Arc` creation from inside `evaluate_period` to the caller's loop
2. Change `evaluate_period`'s parameter from `&IndexMap` to `Arc<IndexMap>` (or pass the Arc by reference)
3. Have the caller use `Arc::make_mut` to append each period's results

- [ ] **Step 3: Refactor to build the Arc incrementally**

In the **caller** of `evaluate_period` (the period loop), replace:

```rust
// Current pattern (pseudocode):
let mut historical = IndexMap::new();
for period in periods {
    let results = self.evaluate_period(..., &historical, ...)?;
    historical.insert(period_id, results);
}
```

With:

```rust
let mut historical_arc = Arc::new(IndexMap::new());
for period in periods {
    let results = self.evaluate_period(..., Arc::clone(&historical_arc), ...)?;
    Arc::make_mut(&mut historical_arc).insert(period_id, results);
}
```

Then update `evaluate_period`'s signature to accept `Arc<IndexMap<...>>` and pass it directly to `EvaluationContext::new()` without cloning:

```rust
fn evaluate_period(
    &mut self,
    // ... other params ...
    historical: Arc<IndexMap<PeriodId, IndexMap<String, f64>>>,
    // ...
) -> Result<...> {
    let context = EvaluationContext::new(
        *period_id,
        Arc::clone(node_to_column),
        historical,  // Moved in, no clone
    );
    // ...
}
```

Apply the same pattern to `historical_cs` on line 653.

**Important:** Verify `EvaluationContext` only reads from `historical` during evaluation. If it writes, use a COW pattern or keep the clone but document why.

- [ ] **Step 4: Run statements tests**

Run: `cargo nextest run -p finstack-statements`
Expected: All pass with identical results.

- [ ] **Step 5: Commit**

```bash
git add finstack/statements/src/evaluator/engine.rs
git commit -m "perf(statements): eliminate per-period historical.clone() in evaluator

Use Arc::make_mut for incremental updates instead of cloning the
entire historical IndexMap for each period. Reduces O(P*N) allocations
to O(P) Arc reference bumps."
```

---

## Chunk 3: Valuations Crate Fixes

### Task 6: Replace `BestEffort` zero-fill with `Option<f64>` or NaN

The `BestEffort` mode in the metric registry silently inserts `0.0` for failed metrics. A zero DV01 could be mistaken for "no rate sensitivity" when it actually means "computation failed."

**Files:**
- Modify: `finstack/valuations/src/metrics/core/registry.rs` (lines 281-286, 303-310, 328-335)

- [ ] **Step 1: Assess the impact of changing the sentinel value**

Before changing, search for all consumers of `context.computed` to understand what downstream code expects:
- Does anything check for `0.0` specifically?
- Does anything use `get()` vs `get_or_default()`?
- Are results exported to DataFrames where NaN would cause issues?

Run: `grep -rn "computed\." finstack/valuations/src/` to find all access patterns.

- [ ] **Step 2: Change sentinel from 0.0 to f64::NAN**

In `finstack/valuations/src/metrics/core/registry.rs`, replace all three `BestEffort` fallback blocks. For example, lines 281-286:

Before:

```rust
StrictMode::BestEffort => {
    tracing::warn!(
        metric_id = %metric_id.as_str(),
        "Metric not registered, inserting 0.0 as fallback"
    );
    context.computed.insert(metric_id, 0.0);
}
```

After:

```rust
StrictMode::BestEffort => {
    tracing::warn!(
        metric_id = %metric_id.as_str(),
        "Metric not registered, inserting NaN as fallback"
    );
    context.computed.insert(metric_id, f64::NAN);
}
```

Apply to all three blocks (lines ~281, ~303, ~328).

- [ ] **Step 3: Update any downstream code that assumes 0.0**

If Step 1 revealed code that checks `== 0.0` or uses the value arithmetically without NaN guards, update those sites to check `.is_nan()` first or use `.unwrap_or(0.0)` explicitly where zero is the correct default.

- [ ] **Step 4: Add a test for NaN propagation in BestEffort mode**

Add a test (in the valuations test suite or inline `#[cfg(test)]` in `registry.rs`) that verifies failed metrics produce NaN:

```rust
#[test]
fn best_effort_inserts_nan_for_unknown_metrics() {
    let registry = MetricRegistry::new(); // empty registry
    let mut context = MetricContext::new(/* ... setup ... */);
    // Request a metric that isn't registered, in BestEffort mode
    registry.compute_metrics(
        &[MetricId::from_str("nonexistent").unwrap()],
        &mut context,
        StrictMode::BestEffort,
    );
    let value = context.computed.get(&MetricId::from_str("nonexistent").unwrap());
    assert!(value.unwrap().is_nan(), "Failed BestEffort metrics should be NaN, not 0.0");
}
```

- [ ] **Step 5: Run all valuations tests**

Run: `cargo nextest run -p finstack-valuations`
Expected: All pass including the new NaN test.

- [ ] **Step 6: Commit**

```bash
git add finstack/valuations/src/metrics/core/registry.rs
git commit -m "fix(valuations): use NaN instead of 0.0 for failed BestEffort metrics

Silent zeros can mask genuine risk computation failures. NaN
propagates through calculations and is easily detected, making
failed metrics visible rather than silently wrong."
```

---

### Task 7: Audit `.unwrap()` calls in bond cashflow_spec.rs

The audit flagged ~28 `.unwrap()` calls. Investigation reveals:
- **Lines 545, 617-619, 748+**: All in **doc-tests or test code** with `#[allow(clippy::unwrap_used)]` — these are fine
- **Lines 182, 198, 374, 563, 571**: These appear to be `unwrap_or(Decimal::ZERO)` not bare `unwrap()` — need verification

**Files:**
- Audit: `finstack/valuations/src/instruments/fixed_income/bond/cashflow_spec.rs`

- [ ] **Step 1: Verify the actual pattern used in production code**

Read lines 182, 198, 374, 563, 571 and confirm whether they are:
- `unwrap_or(Decimal::ZERO)` — safe (no panic) but silently loses data
- Bare `.unwrap()` — panic risk

- [ ] **Step 2: If `unwrap_or(Decimal::ZERO)`, add documenting comments**

If the production code uses `unwrap_or(Decimal::ZERO)`, these are not panic risks but are the same silent-zero pattern. Since the enclosing functions may not return `Result`, add clarifying comments:

```rust
// from_f64_retain only fails for NaN/Infinity; zero is a safe fallback for coupon rates
```

If any are bare `.unwrap()`, replace with `?` propagation or `unwrap_or` with documentation.

- [ ] **Step 3: Run tests**

Run: `cargo nextest run -p finstack-valuations`
Expected: All pass.

- [ ] **Step 4: Commit (only if changes made)**

```bash
git add finstack/valuations/src/instruments/fixed_income/bond/cashflow_spec.rs
git commit -m "docs(valuations): document unwrap_or rationale in bond cashflow spec"
```

---

## Chunk 4: Core Crate Fixes

### Task 8: Add context string to `Error::Internal`

The `Internal` error variant carries no diagnostic information, making it impossible to debug in production.

**Files:**
- Modify: `finstack/core/src/error/mod.rs` (line 258-260)
- Modify: All call sites that construct `Error::Internal`

- [ ] **Step 1: Find all `Error::Internal` construction sites**

Run: `grep -rn "Error::Internal" finstack/core/src/ finstack/valuations/src/ finstack/statements/src/ finstack/portfolio/src/`

- [ ] **Step 2: Change the variant to carry a String**

In `finstack/core/src/error/mod.rs`, replace:

```rust
/// Catch-all for unexpected internal failures.
#[error("Internal system error")]
Internal,
```

With:

```rust
/// Catch-all for unexpected internal failures.
#[error("Internal error: {0}")]
Internal(String),
```

- [ ] **Step 3: Update all construction sites**

Every `Error::Internal` becomes `Error::Internal("description".into())`. Each call site should describe what went wrong. For example:

```rust
// Before:
Err(Error::Internal)
// After:
Err(Error::Internal("curve bootstrap produced non-finite value".into()))
```

- [ ] **Step 4: Update any pattern matches on `Error::Internal`**

Find all `Error::Internal =>` or `Error::Internal if` patterns and update to `Error::Internal(_) =>` or destructure the string for logging.

- [ ] **Step 5: Add a test for the new context string**

Add a test (doctest or unit test) verifying the Display output:

```rust
#[test]
fn internal_error_carries_context() {
    let err = Error::Internal("bootstrap diverged".into());
    let msg = err.to_string();
    assert!(msg.contains("bootstrap diverged"), "Error should contain context: {msg}");
}
```

- [ ] **Step 6: Run all workspace tests**

Run: `make test-rust`
Expected: All pass.

- [ ] **Step 7: Commit**

```bash
git add finstack/core/src/error/mod.rs
git commit -m "feat(core): add context string to Error::Internal for production debugging

The Internal variant was a unit enum with no diagnostic information.
Now carries a String message describing what internal invariant was
violated, making production debugging feasible."
```

---

### Task 9: Add defensive bound to `shift_to_weekday` loop

The unbounded `while` loop in `shift_to_weekday` is safe in theory (max 6 iterations) but has no defensive guard against corrupted state.

**Files:**
- Modify: `finstack/core/src/dates/calendar/rule.rs:552-566`

- [ ] **Step 1: Add a loop bound**

Replace:

```rust
fn shift_to_weekday(mut d: Date, weekday: Weekday, dir: Direction) -> Date {
    match dir {
        Direction::After => {
            while d.weekday() != weekday {
                d += Duration::days(1);
            }
        }
        Direction::Before => {
            while d.weekday() != weekday {
                d -= Duration::days(1);
            }
        }
    }
    d
}
```

With:

```rust
fn shift_to_weekday(mut d: Date, weekday: Weekday, dir: Direction) -> Date {
    match dir {
        Direction::After => {
            for _ in 0..7 {
                if d.weekday() == weekday {
                    return d;
                }
                d += Duration::days(1);
            }
        }
        Direction::Before => {
            for _ in 0..7 {
                if d.weekday() == weekday {
                    return d;
                }
                d -= Duration::days(1);
            }
        }
    }
    d // Unreachable for valid weekdays, but avoids infinite loop
}
```

- [ ] **Step 2: Run core tests**

Run: `cargo nextest run -p finstack-core`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add finstack/core/src/dates/calendar/rule.rs
git commit -m "fix(core): bound shift_to_weekday loop to 7 iterations

Defensive guard against infinite loop in case of corrupted date state.
Functionally equivalent — a valid weekday is always found within 7 days."
```

---

### Task 10: Add overflow check to tenor `count as i32` cast

The `u32` to `i32` cast on line 431 of `tenor.rs` can silently wrap for values > `i32::MAX`.

**Files:**
- Modify: `finstack/core/src/dates/tenor.rs:431`

- [ ] **Step 1: Write a test for overflow**

```rust
#[test]
fn tenor_rejects_count_exceeding_i32_max() {
    // u32 value that would overflow i32
    let count = (i32::MAX as u32) + 1;
    // Constructing a Tenor with this count and applying it should error
    // (exact API depends on Tenor's constructor)
}
```

- [ ] **Step 2: Replace both casts with checked conversion**

In `finstack/core/src/dates/tenor.rs`, lines 431-432, both lines need fixing:

```rust
// Line 431 — Months (no multiplier):
TenorUnit::Months => date.add_months(self.count as i32),
// Line 432 — Years (multiply by 12):
TenorUnit::Years => date.add_months((self.count as i32) * 12),
```

Replace with:

```rust
TenorUnit::Months => {
    let count_i32 = i32::try_from(self.count)
        .map_err(|_| Error::input(format!("Tenor count {} exceeds i32::MAX", self.count)))?;
    date.add_months(count_i32)
}
TenorUnit::Years => {
    let count_i32 = i32::try_from(self.count)
        .map_err(|_| Error::input(format!("Tenor count {} exceeds i32::MAX", self.count)))?;
    let months = count_i32.checked_mul(12)
        .ok_or_else(|| Error::input(format!("Tenor {} years overflows month count", self.count)))?;
    date.add_months(months)
}
```

Note: Lines 429-430 use `i64::from(self.count)` for Days/Weeks which is always safe (u32 fits in i64).

- [ ] **Step 3: Run core tests**

Run: `cargo nextest run -p finstack-core`
Expected: All pass.

- [ ] **Step 4: Commit**

```bash
git add finstack/core/src/dates/tenor.rs
git commit -m "fix(core): use checked i32 conversion in Tenor to prevent silent overflow"
```

---

## Chunk 5: Portfolio, Monte Carlo, and Correlation Fixes

### Task 11: Encapsulate `positions` field in Portfolio

The `pub positions: Vec<Position>` field allows direct mutation that desynchronizes the internal `position_index` and `dependency_index`.

**Files:**
- Modify: `finstack/portfolio/src/portfolio.rs:43`
- Modify: All direct `portfolio.positions` access sites

- [ ] **Step 1: Find all direct access sites**

Run: `grep -rn "\.positions" finstack/portfolio/src/ finstack/portfolio/tests/`

Categorize:
- Read access (`.positions.iter()`, `.positions.len()`) — these can use a getter
- Write access (`.positions.push()`, `.positions = ...`) — these need mutation methods
- Index access (`.positions[i]`) — these can use a getter returning a slice

- [ ] **Step 2: Change field visibility and add accessor methods**

In `finstack/portfolio/src/portfolio.rs`, change:

```rust
pub positions: Vec<Position>,
```

To:

```rust
pub(crate) positions: Vec<Position>,
```

Add methods:

```rust
impl Portfolio {
    /// Returns a reference to all positions.
    pub fn positions(&self) -> &[Position] {
        &self.positions
    }

    /// Add a position and update internal indices.
    pub fn add_position(&mut self, position: Position) {
        let idx = self.positions.len();
        self.position_index.insert(position.id().clone(), idx);
        // Update dependency_index if needed
        self.positions.push(position);
    }

    /// Replace all positions and rebuild indices.
    pub fn set_positions(&mut self, positions: Vec<Position>) {
        self.positions = positions;
        self.rebuild_index();
    }
}
```

- [ ] **Step 3: Update all external access sites**

Replace `portfolio.positions` reads with `portfolio.positions()` and writes with `portfolio.add_position()` or `portfolio.set_positions()`.

- [ ] **Step 4: Run portfolio tests**

Run: `cargo nextest run -p finstack-portfolio`
Expected: All pass.

- [ ] **Step 5: Check downstream crates**

Run: `cargo nextest run --workspace --exclude finstack-py --features mc,test-utils`
Expected: All pass. If other crates accessed `portfolio.positions` directly, they'll get compile errors that need fixing.

- [ ] **Step 6: Commit**

```bash
git add finstack/portfolio/
git commit -m "refactor(portfolio): encapsulate positions field to prevent index desync

Make positions pub(crate) and provide accessor methods that
automatically maintain position_index and dependency_index.
Eliminates the rebuild_index footgun."
```

---

### Task 12: Make `MultiFactorModel::new()` return `Result`

The silent fallback to identity correlation on invalid input can mask configuration errors.

**Files:**
- Modify: `finstack/correlation/src/factor_model.rs:524-531`
- Modify: All call sites of `MultiFactorModel::new()`

- [ ] **Step 1: Find all callers**

Run: `grep -rn "MultiFactorModel::new" finstack/`

- [ ] **Step 2: Change signature to return Result**

In `finstack/correlation/src/factor_model.rs`, replace:

```rust
pub fn new(num_factors: usize, volatilities: Vec<f64>, correlations: Vec<f64>) -> Self {
    Self::validated(num_factors, volatilities.clone(), correlations).unwrap_or_else(|err| {
        tracing::warn!(...);
        // fallback to identity
    })
}
```

With:

```rust
pub fn new(num_factors: usize, volatilities: Vec<f64>, correlations: Vec<f64>) -> Result<Self, CorrelationError> {
    Self::validated(num_factors, volatilities, correlations)
}
```

If callers genuinely need the fallback behavior, provide a separate method:

```rust
/// Like `new()`, but falls back to identity correlation on invalid input.
pub fn new_or_identity(num_factors: usize, volatilities: Vec<f64>, correlations: Vec<f64>) -> Self {
    Self::new(num_factors, volatilities.clone(), correlations).unwrap_or_else(|err| {
        tracing::warn!(num_factors, %err, "Falling back to identity correlation");
        let identity_corr = /* ... identity matrix ... */;
        Self::validated(num_factors, volatilities, identity_corr)
            .expect("identity matrix should always be valid")
    })
}
```

- [ ] **Step 3: Update all callers**

Each `MultiFactorModel::new(...)` becomes `MultiFactorModel::new(...)?` or `MultiFactorModel::new_or_identity(...)` depending on whether the caller should propagate the error.

- [ ] **Step 4: Run correlation and workspace tests**

Run: `cargo nextest run -p finstack-correlation && make test-rust`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
git add finstack/correlation/src/factor_model.rs
git commit -m "fix(correlation): make MultiFactorModel::new() return Result

Silent fallback to identity correlation masked configuration errors.
Callers can now use new() for strict validation or new_or_identity()
for the legacy fallback behavior."
```

---

## Chunk 6: WASM Feature Parity

### Task 13: Make WASM EquityOption builder fully configurable

The WASM `build_equity_option` helper hardcodes USD currency, "EQUITY-SPOT" spot ID, and "USD-OIS" discount curve. The Python bindings are fully configurable.

**Files:**
- Modify: `finstack-wasm/src/valuations/instruments/equity_option.rs:15-45`

- [ ] **Step 1: Add optional parameters to the builder**

Replace the `build_equity_option` helper with configurable builder methods. Add optional fields to `EquityOptionBuilder`:

```rust
#[wasm_bindgen]
pub struct EquityOptionBuilder {
    // ... existing fields ...
    currency: Option<Currency>,
    spot_id: Option<String>,
    discount_curve_id: Option<String>,
    vol_surface_id: Option<String>,
    day_count: Option<DayCount>,
    exercise_style: Option<ExerciseStyle>,
    settlement: Option<SettlementType>,
    div_yield_id: Option<String>,
}
```

Add setter methods following the existing builder pattern in the crate (consuming `self` and returning `Self`):

```rust
#[wasm_bindgen]
impl EquityOptionBuilder {
    #[wasm_bindgen(js_name = currency)]
    pub fn currency(mut self, currency: &str) -> Self {
        self.currency = Some(parse_currency(currency));
        self
    }

    #[wasm_bindgen(js_name = spotId)]
    pub fn spot_id(mut self, spot_id: &str) -> Self {
        self.spot_id = Some(spot_id.to_string());
        self
    }

    #[wasm_bindgen(js_name = discountCurveId)]
    pub fn discount_curve_id(mut self, id: &str) -> Self {
        self.discount_curve_id = Some(id.to_string());
        self
    }
    // ... etc for vol_surface_id, day_count, exercise_style, settlement, div_yield_id
}
```

Update `build()` to use the builder fields with existing defaults as fallbacks:

```rust
let currency = self.currency.unwrap_or(Currency::USD);
let spot_id = self.spot_id.unwrap_or_else(|| "EQUITY-SPOT".to_string());
let discount_curve = self.discount_curve_id.unwrap_or_else(|| "USD-OIS".to_string());
```

- [ ] **Step 2: Deprecate the old convenience constructor if it exists**

Add `#[deprecated]` to the old `build_equity_option` function and redirect to the builder.

- [ ] **Step 3: Run WASM tests**

Run: `make test-wasm`
Expected: All pass. Existing code using defaults is unchanged.

- [ ] **Step 4: Commit**

```bash
git add finstack-wasm/src/valuations/instruments/equity_option.rs
git commit -m "feat(wasm): make EquityOption builder fully configurable

Add optional currency, spotId, discountCurveId, volSurfaceId,
dayCount, exerciseStyle, settlement, and divYieldId to the
WASM EquityOptionBuilder, matching Python binding feature parity.
Existing defaults preserved for backward compatibility."
```

---

### Task 14: Return string enum for `instrumentType()` instead of u16

Returning a raw `u16` discriminant is an anti-pattern for JS consumers. Return a string name instead.

**Files:**
- Modify: All WASM instrument wrappers that define `instrument_type() -> u16`

- [ ] **Step 1: Find all `instrumentType` definitions**

Run: `grep -rn "fn instrument_type" finstack-wasm/src/`

- [ ] **Step 2: Change return type from u16 to String**

For each instrument wrapper, replace:

```rust
#[wasm_bindgen(js_name = instrumentType)]
pub fn instrument_type(&self) -> u16 {
    InstrumentType::EquityOption as u16
}
```

With:

```rust
#[wasm_bindgen(js_name = instrumentType)]
pub fn instrument_type(&self) -> String {
    InstrumentType::EquityOption.to_string()
}
```

`InstrumentType` already implements `Display` (via `strum`), so `.to_string()` returns the human-readable name.

- [ ] **Step 3: Update any JS-side code that compares against u16 values**

Check `finstack-wasm/tests/` for any tests that compare `instrumentType()` against numeric values and update them to compare against strings.

- [ ] **Step 4: Run WASM tests**

Run: `make test-wasm`
Expected: All pass (or test updates from Step 3 make them pass).

- [ ] **Step 5: Commit**

```bash
git add finstack-wasm/src/valuations/instruments/
git commit -m "feat(wasm): return string enum name from instrumentType() instead of u16

Raw u16 discriminants require JS consumers to maintain a magic number
mapping. String names like 'EquityOption' are self-documenting and
stable across Rust enum reordering."
```

---

## Chunk 7: Deferred / Lower Priority

These findings from the audit are valid but lower impact. They can be addressed in a follow-up.

### Task 15: Document or deprecate `ConvergenceDiagnostics` in Monte Carlo

The `ConvergenceDiagnostics` struct in `monte_carlo/src/estimate.rs` (lines 145-152) has three `Option` fields (`stderr_decay_rate`, `effective_sample_size`, `variance_reduction_factor`) that are never populated by any code path.

**Files:**
- Modify: `finstack/monte_carlo/src/estimate.rs:145-186`

- [ ] **Step 1: Search for usage**

Run: `grep -rn "ConvergenceDiagnostics" finstack/monte_carlo/src/`

Determine if any code path sets the fields. If none do, either:
- Add `#[deprecated(note = "Fields not yet populated — use SimulationResult statistics instead")]`
- Or remove the struct entirely if no public API depends on it

- [ ] **Step 2: Apply deprecation or removal**

If the struct is part of a public return type (e.g., inside `SimulationResult`), deprecate it. If it's only internal, remove it.

- [ ] **Step 3: Run tests and commit**

Run: `cargo nextest run -p finstack-monte-carlo`

```bash
git commit -m "chore(mc): deprecate unused ConvergenceDiagnostics struct"
```

---

### Task 16: Optimize min/max compilation in statements DSL

The compiler in `finstack/statements/src/dsl/compiler.rs` (lines 282-286) builds `min(a,b,c,d)` by nesting conditionals, which clones the accumulator expression at each step — O(n²) expression tree growth for n arguments.

**Files:**
- Modify: `finstack/statements/src/dsl/compiler.rs:261-286`
- Modify: `finstack/statements/src/dsl/ast.rs` (if adding a native `Min`/`Max` variant)

- [ ] **Step 1: Assess the practical impact**

Check how many arguments `min`/`max` calls typically have in the test suite and real financial models. If they rarely exceed 4 arguments, the quadratic cost is negligible and this can be deferred.

- [ ] **Step 2: Option A — Add native Min/Max core expression (if justified)**

Add `Min(Vec<Expr>)` and `Max(Vec<Expr>)` variants to the core AST. The evaluator handles them directly without nesting. This eliminates the cloning entirely but requires changes to the AST, evaluator, and any expression visitors.

- [ ] **Step 3: Option B — Use Rc/Arc to share expression nodes (lighter touch)**

Wrap expressions in `Rc<Expr>` so cloning shares the subtree instead of deep-copying:

```rust
let mut result = Rc::new(args[0].clone());
for arg in &args[1..] {
    let arg_rc = Rc::new(arg.clone());
    let condition = Expr::bin_op(comparison_op, Expr::Shared(Rc::clone(&result)), Expr::Shared(Rc::clone(&arg_rc)));
    result = Rc::new(Expr::if_then_else(condition, Expr::Shared(result), Expr::Shared(arg_rc)));
}
```

This reduces the cost from O(n²) to O(n) without changing the AST enum.

- [ ] **Step 4: Run tests and commit**

Run: `cargo nextest run -p finstack-statements`

---

### Task 17: COW / arena for MarketContext in bump-and-reprice (Deferred)

The audit found that `bump_scalar_price` and `bump_discount_curve_parallel` clone the entire `MarketContext` for each bump. For bucketed DV01 with 20+ buckets, this means 20+ full market clones per instrument.

This is an architectural change that requires careful design. **Defer to a separate design document.**

**Approach options:**
1. **Copy-on-Write (COW)**: Wrap curve data in `Arc` inside MarketContext. Bump operations clone only the affected curve, not the entire context.
2. **Arena allocator**: Use a bump allocator for temporary market contexts during finite-difference calculations.
3. **Cursor pattern**: Pass a `MarketBump` descriptor to the pricer instead of a cloned context, letting the pricer read the original context with a single curve override.

**No implementation steps here** — this requires a design spike first. File as a separate planning task.

---

## Execution Order

Tasks are independent within each chunk but chunks should be executed in order:

1. **Chunk 1** (Tasks 1-3): WASM correctness — highest priority, safety-critical
2. **Chunk 2** (Tasks 4-5): Statements — parser safety + performance
3. **Chunk 3** (Tasks 6-7): Valuations — metric correctness + code hygiene
4. **Chunk 4** (Tasks 8-10): Core — error diagnostics + defensive coding
5. **Chunk 5** (Tasks 11-12): Portfolio/Correlation — API safety
6. **Chunk 6** (Tasks 13-14): WASM feature parity — nice-to-have
7. **Chunk 7** (Tasks 15-17): Lower priority — deprecations, optimizations, architecture

Within each chunk, tasks can be parallelized across subagents since they touch different files.

---

## Verification

After all tasks are complete:

```bash
# Full workspace test suite
make test-rust

# WASM tests
make test-wasm

# Clippy (treats warnings as errors)
cargo clippy --workspace --all-targets -- -D warnings

# Ensure no new unwrap/expect/panic in non-test code
grep -rn '\.unwrap()' finstack/*/src/ --include='*.rs' | grep -v '#\[cfg(test)\]' | grep -v '#\[allow'
grep -rn '\.expect(' finstack/*/src/ --include='*.rs' | grep -v '#\[cfg(test)\]' | grep -v '#\[allow'
```
