# Audit Hardening: Production Robustness Improvements

**Date**: 2026-03-21
**Status**: Draft
**Scope**: 6 changes across finstack core, valuations, statements, and portfolio crates

## Context

A deep production audit of the finstack workspace identified several hardening opportunities. After investigation, many originally flagged issues (cashflows builder panics, statements waterfall unwraps) were found to be test-code-only. The remaining genuine production concerns are documented here.

## Change 1: Expression Arena Bounds Check

### Problem

`finstack/core/src/expr/eval.rs:214` allocates `vec![0.0; len * plan.nodes.len()]` with no size cap. A pathological expression (1000 DAG nodes evaluated over 1M rows) would attempt an 8GB allocation, causing OOM without a useful error message.

### Design

Add a pre-flight size check in `CompiledExpr::eval()` before the arena allocation.

**New field in `EvalOpts`:**

```rust
// In finstack/core/src/expr/types.rs (or wherever EvalOpts lives)
pub struct EvalOpts {
    // ... existing fields ...
    /// Maximum arena allocation in bytes. Defaults to 1GB.
    /// Set to 0 to disable the check.
    pub max_arena_bytes: usize,
}

impl Default for EvalOpts {
    fn default() -> Self {
        Self {
            // ... existing defaults ...
            max_arena_bytes: 1_073_741_824, // 1 GB
        }
    }
}
```

**New error variant in `InputError`:**

```rust
TooLarge {
    what: &'static str,
    requested_bytes: usize,
    limit_bytes: usize,
},
```

**Bounds check in `eval()`** (inserted before line 214):

```rust
let node_count = plan_to_use.nodes.len();
let arena_elements = len.checked_mul(node_count).ok_or_else(|| {
    Error::from(InputError::TooLarge {
        what: "expression arena",
        requested_bytes: usize::MAX,
        limit_bytes: opts.max_arena_bytes,
    })
})?;
let arena_bytes = arena_elements.checked_mul(std::mem::size_of::<f64>()).unwrap_or(usize::MAX);
if opts.max_arena_bytes > 0 && arena_bytes > opts.max_arena_bytes {
    return Err(InputError::TooLarge {
        what: "expression arena",
        requested_bytes: arena_bytes,
        limit_bytes: opts.max_arena_bytes,
    }
    .into());
}
let mut arena = vec![0.0; arena_elements];
```

### Files

- `finstack/core/src/expr/eval.rs` — bounds check before allocation
- `finstack/core/src/expr/types.rs` — `max_arena_bytes` field on `EvalOpts`
- `finstack/core/src/error.rs` — `InputError::TooLarge` variant

### Breaking Changes

None. `EvalOpts` gains a new field with a default value. Existing callers using `EvalOpts::default()` get the 1GB cap automatically.

### Tests

- Test that an expression with `len * nodes > 1GB / 8` returns `InputError::TooLarge`
- Test that `max_arena_bytes = 0` disables the check
- Test that normal expressions (small arena) are unaffected

---

## Change 2: Minimum Knot Spacing in Interpolators

### Problem

`validate_knots()` in `interp/utils.rs` checks strictly-increasing but not minimum gap. Two knots at `1.0` and `1.0 + 1e-16` pass validation, but slope calculations `(v[i+1] - v[i]) / (k[i+1] - k[i])` produce numerically unstable results in strategies that compute derivatives.

### Design

Add a `validate_knot_spacing()` function. Strategies that compute slopes call it during `from_raw()`; strategies that only do flat lookups skip it.

**New function in `interp/utils.rs`:**

```rust
/// Default minimum relative gap between consecutive knots.
///
/// Knots closer than `gap < MIN_RELATIVE_KNOT_GAP * max(|k[i]|, 1.0)` are
/// rejected to prevent numerical instability in slope/derivative calculations.
pub const MIN_RELATIVE_KNOT_GAP: f64 = 1e-10;

/// Validate that consecutive knots have sufficient spacing for stable interpolation.
///
/// The minimum gap is relative to knot magnitude: `gap >= min_relative_gap * max(|k[i]|, 1.0)`.
/// This prevents division-by-near-zero in slope calculations while allowing tight spacing
/// for small-magnitude knots.
pub fn validate_knot_spacing(knots: &[f64], min_relative_gap: f64) -> crate::Result<()> {
    for w in knots.windows(2) {
        let gap = w[1] - w[0];
        let scale = w[0].abs().max(1.0);
        if gap < min_relative_gap * scale {
            return Err(InputError::KnotSpacingTooSmall.into());
        }
    }
    Ok(())
}
```

**New error variant:**

```rust
/// Consecutive knots are too close together for stable interpolation.
KnotSpacingTooSmall,
```

**Strategy integration:** Call `validate_knot_spacing(knots, MIN_RELATIVE_KNOT_GAP)` in `from_raw()` for:
- `LinearStrategy`
- `PiecewiseQuadraticForwardStrategy`
- `MonotoneConvexStrategy`
- Any other strategy that divides by `knots[i+1] - knots[i]`

Strategies that do not compute slopes (e.g., `FlatForwardStrategy`, `LogLinearStrategy` if it only uses log-value interpolation) skip the call.

### Files

- `finstack/core/src/math/interp/utils.rs` — new function + constant
- `finstack/core/src/error.rs` — `InputError::KnotSpacingTooSmall` variant
- `finstack/core/src/math/interp/strategies.rs` (or per-strategy files) — call in `from_raw()`

### Breaking Changes

None in the public API. Internally, previously-accepted pathological knot sets (gap < 1e-10) will now be rejected at construction time. This could break existing code that happens to construct interpolators with extremely close knots, but such interpolators were already producing garbage results.

### Tests

- Test that knots with gap < threshold are rejected
- Test that knots with gap >= threshold are accepted
- Test with real-world curve tenors (1D, 1W, 1M, 3M, ..., 30Y) — must pass
- Test with knots near zero (e.g., 0.001, 0.002) — gap is absolute, not relative to tiny knots

---

## Change 3: CashflowBreakdown `.expect()` → `Result`

### Problem

`capital_structure/types.rs:148` uses `.expect()` on `checked_add()`, panicking if the currency invariant is violated. The invariant ("all Money fields have same currency") is enforced by convention via `with_currency()` constructor, but not by the type system. A bug in waterfall logic that sets fields with mismatched currencies would cause an unrecoverable panic.

### Design

Change `interest_expense_total()` (and any similar methods on `CashflowBreakdown`) to return `Result<Money>`.

**Before:**

```rust
#[allow(clippy::expect_used)]
pub fn interest_expense_total(&self) -> Money {
    self.interest_expense_cash
        .checked_add(self.interest_expense_pik)
        .expect("CashflowBreakdown values should have same currency")
}
```

**After:**

```rust
pub fn interest_expense_total(&self) -> crate::Result<Money> {
    self.interest_expense_cash
        .checked_add(self.interest_expense_pik)
        .ok_or_else(|| Error::CurrencyMismatch {
            expected: self.interest_expense_cash.currency(),
            got: self.interest_expense_pik.currency(),
        })
}
```

Audit all methods on `CashflowBreakdown` for the same pattern and convert any others found.

### Files

- `finstack/statements/src/capital_structure/types.rs` — method signature change
- Call sites in `capital_structure/waterfall.rs`, `capital_structure/integration.rs` — add `?` propagation

### Breaking Changes

Yes — `interest_expense_total()` return type changes from `Money` to `Result<Money>`. This is internal to the statements crate; no public Python/WASM API change. Callers within the crate add `?`.

### Tests

- Existing tests should continue to pass (they use valid currency combinations)
- Add a test that constructs a `CashflowBreakdown` with mismatched currencies and verifies `interest_expense_total()` returns `Err`

---

## Change 4: XIRR `irr_detailed()` with Root Metadata

### Problem

`InternalRateOfReturn::irr()` returns `Result<f64>` with no metadata about root ambiguity. Callers with complex cashflow patterns (multiple sign changes) have no programmatic way to know the result might not be the economically meaningful root.

### Design

Add an `IrrResult` struct and `irr_detailed()` method. The existing `irr()` is unchanged.

**New types in `cashflow/xirr.rs`:**

```rust
/// Extended result from IRR calculation with root-ambiguity metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct IrrResult {
    /// The computed internal rate of return.
    pub rate: f64,
    /// Number of sign changes in the cashflow sequence.
    ///
    /// By Descartes' rule of signs, this is an upper bound on the number of
    /// positive real roots of the NPV polynomial.
    pub sign_changes: usize,
    /// Whether multiple roots are possible (sign_changes > 1).
    ///
    /// When true, the returned `rate` is the first valid root found from the
    /// search order documented in the module-level docs. Users should verify
    /// the result makes economic sense.
    pub multiple_roots_possible: bool,
}
```

**New trait methods (with default impls):**

```rust
pub trait InternalRateOfReturn {
    // ... existing methods unchanged ...

    /// Calculate IRR with root-ambiguity metadata.
    fn irr_detailed(&self, guess: Option<f64>) -> crate::Result<IrrResult> {
        let rate = self.irr(guess)?;
        let sign_changes = self.count_sign_changes();
        Ok(IrrResult {
            rate,
            sign_changes,
            multiple_roots_possible: sign_changes > 1,
        })
    }

    /// Count the number of sign changes in the cashflow sequence.
    fn count_sign_changes(&self) -> usize;
}
```

Sign change counting is O(n), filters out zero-valued cashflows, and counts transitions between positive and negative values.

### Files

- `finstack/core/src/cashflow/xirr.rs` — `IrrResult` struct, `irr_detailed()` + `count_sign_changes()` methods, impls for `[f64]` and `[(Date, f64)]`

### Breaking Changes

None. Additive methods with default implementations. Existing trait implementors may need to implement `count_sign_changes()` (it's a required method), but the only implementors are `[f64]` and `[(Date, f64)]` in this file, so no external breakage.

### Tests

- Test `irr_detailed()` on simple cashflows (1 sign change) → `multiple_roots_possible: false`
- Test on cashflows with 3+ sign changes → `multiple_roots_possible: true`
- Test that `rate` matches `irr()` output exactly
- Test sign change counting with zero-valued cashflows (zeros should be skipped)

---

## Change 5: Portfolio Metrics Arc Clone Reduction

### Problem

In `pricer/registry.rs`, `price_with_metrics()` wraps `market` in `Arc::new(market.clone())` on every call (lines 266, 302). For portfolio pricing of N instruments, this clones the entire `MarketContext` N times. `MarketContext` contains HashMaps of curves, surfaces, and scalar data — non-trivial to clone.

### Design

Add a `price_with_metrics_arc()` variant that accepts pre-wrapped `Arc<MarketContext>`, and have the existing method delegate to it.

**New method on `PricerRegistry`:**

```rust
/// Price with metrics using a shared market context.
///
/// Prefer this over `price_with_metrics()` when pricing multiple instruments
/// against the same market to avoid redundant cloning.
pub fn price_with_metrics_arc(
    &self,
    instrument: &dyn Instrument,
    market: &Arc<MarketContext>,
    as_of: Date,
    metrics: &[MetricId],
    cfg: Option<&FinstackConfig>,
    market_history: Option<&MarketHistory>,
) -> Result<ValuationResult> {
    // ... same logic as price_with_metrics, but uses Arc::clone(market) instead of Arc::new(market.clone())
}
```

**Refactor existing method:**

```rust
pub fn price_with_metrics(
    &self,
    instrument: &dyn Instrument,
    market: &MarketContext,
    as_of: Date,
    metrics: &[MetricId],
    cfg: Option<&FinstackConfig>,
    market_history: Option<&MarketHistory>,
) -> Result<ValuationResult> {
    let market_arc = Arc::new(market.clone()); // single clone
    self.price_with_metrics_arc(instrument, &market_arc, as_of, metrics, cfg, market_history)
}
```

The internal `build_with_metrics_dyn()` already accepts `Arc<MarketContext>`, so this just lifts the `Arc::new()` up. Portfolio-level callers can wrap once and reuse.

### Files

- `finstack/valuations/src/pricer/registry.rs` — new method, refactor existing
- `finstack/portfolio/src/valuation.rs` — use `price_with_metrics_arc()` in portfolio pricing loop

### Breaking Changes

None. Additive method. Existing callers unaffected.

### Tests

- Existing pricing tests should pass unchanged
- Add a test that `price_with_metrics()` and `price_with_metrics_arc()` produce identical results

---

## Change 6: Numeric Precision & Concurrency Documentation

### Problem

Precision guarantees and concurrency contracts are implicit. A new engineer encountering a 1e-9 discrepancy in day-count fractions or a subtle race condition in portfolio repricing has no reference to consult.

### Design

Add targeted doc comments (no code changes) to 5 files:

1. **`finstack/core/src/dates/daycount.rs`** — module-level doc: "Year fractions are computed as f64 with typical precision ~1e-9 for standard tenors (< 50 years). Precision degrades for very long tenors due to floating-point accumulation."

2. **`finstack/core/src/math/solver.rs`** — doc on solver constants: "Convergence uses dual tolerances: the residual (|f(x)| < tol) and the step size (|x_new - x_old| < tol). Default `SOLVER_TOLERANCE = 1e-8` matches QuantLib's professional-grade standard."

3. **`finstack/portfolio/src/portfolio.rs`** — doc on `positions` field: "Instruments behind `Arc` must be immutable after construction. The portfolio assumes no interior mutability — concurrent reads are safe, but modifying an instrument after adding it to a portfolio is undefined behavior at the application level."

4. **`finstack/core/src/expr/compiled.rs`** (or wherever `CompiledExpr` is defined) — doc: "CompiledExpr is `Send` but not `Sync`. Each instance holds a `Mutex<ScratchArena>` for single-threaded evaluation. For parallel evaluation, clone the expression — each clone gets an independent scratch buffer."

5. **`finstack/core/src/money/fx.rs`** — doc on `FxMatrix`: "Uses interior `Mutex` for rate caching. Under high concurrency, cache lookups serialize through the lock. For performance-critical parallel pricing, consider pre-fetching rates or using one `FxMatrix` per thread."

### Files

5 files, doc comments only.

### Breaking Changes

None.

### Tests

None required (documentation only).
