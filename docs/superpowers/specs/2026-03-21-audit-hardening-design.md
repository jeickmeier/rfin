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
// In finstack/core/src/expr/eval.rs (EvalOpts is defined at line 55)
// NOTE: EvalOpts currently uses #[derive(Default)]. Adding max_arena_bytes
// requires switching to a manual Default impl so we can set 1GB instead of 0.
// Remove `Default` from the derive list and add:
impl Default for EvalOpts {
    fn default() -> Self {
        Self {
            plan: None,
            cache_budget_mb: None,
            max_arena_bytes: 1_073_741_824, // 1 GB
        }
    }
}
```

**New error variant in `InputError`:**

Note: `InputError` derives `serde::Deserialize`, so field types must be owned. Use `String` (not `&'static str`).

```rust
/// Requested allocation exceeds configured limit.
#[error("Allocation too large for {what}: requested {requested_bytes} bytes, limit {limit_bytes} bytes")]
TooLarge {
    what: String,
    requested_bytes: usize,
    limit_bytes: usize,
},
```

**Bounds check in `eval()`** (inserted before line 214):

```rust
let node_count = plan_to_use.nodes.len();
let arena_elements = len.checked_mul(node_count).ok_or_else(|| {
    Error::from(InputError::TooLarge {
        what: "expression arena".into(),
        requested_bytes: usize::MAX,
        limit_bytes: opts.max_arena_bytes,
    })
})?;
let arena_bytes = arena_elements.checked_mul(std::mem::size_of::<f64>()).unwrap_or(usize::MAX);
if opts.max_arena_bytes > 0 && arena_bytes > opts.max_arena_bytes {
    return Err(InputError::TooLarge {
        what: "expression arena".into(),
        requested_bytes: arena_bytes,
        limit_bytes: opts.max_arena_bytes,
    }
    .into());
}
let mut arena = vec![0.0; arena_elements];
```

### Files

- `finstack/core/src/expr/eval.rs` — bounds check before allocation + `max_arena_bytes` field on `EvalOpts` (defined at line 55)
- `finstack/core/src/error/inputs.rs` — `InputError::TooLarge` variant

### Breaking Changes

Minor structural change: `EvalOpts` currently uses `#[derive(Default)]`. Adding `max_arena_bytes` with a non-zero default requires switching to a manual `Default` impl. This is source-compatible — `EvalOpts::default()` callers are unaffected — but it changes the derive list. Existing callers using `EvalOpts::default()` get the 1GB cap automatically. Callers constructing `EvalOpts { plan: None, cache_budget_mb: None }` without `..Default::default()` will get a compile error until they add the new field.

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
#[error("Consecutive knots are too close together for stable interpolation")]
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
- `finstack/core/src/error/inputs.rs` — `InputError::KnotSpacingTooSmall` variant
- `finstack/core/src/math/interp/strategies.rs` (or per-strategy files) — call in `from_raw()`

### Breaking Changes

None in the public API. Internally, previously-accepted pathological knot sets (gap < 1e-10) will now be rejected at construction time. This could break existing code that happens to construct interpolators with extremely close knots, but such interpolators were already producing garbage results.

### Tests

- Test that knots with gap < threshold are rejected
- Test that knots with gap >= threshold are accepted
- Test with real-world curve tenors (1D, 1W, 1M, 3M, ..., 30Y) — must pass
- Test with knots near zero (e.g., 0.001, 0.002) — the `max(|k[i]|, 1.0)` floor ensures the threshold does not shrink below `min_relative_gap`, so near-zero knots use an effectively absolute minimum gap

---

## Change 3: CashflowBreakdown `.expect()` → Validated Construction

### Problem

`capital_structure/types.rs:148` uses `.expect()` on `checked_add()`, panicking if the currency invariant is violated. The invariant ("all Money fields have same currency") is enforced by convention via `with_currency()` constructor, but not by the type system. A bug in waterfall logic that sets fields with mismatched currencies would cause an unrecoverable panic.

### Design

Changing the return type to `Result<Money>` is impractical because `interest_expense_total()` is called inside closures passed to `get_instrument_field()` and `reporting_total()`, both of which take `Fn(&CashflowBreakdown) -> f64`. Changing those closure signatures would cascade through the entire capital structure API.

Instead, take a two-pronged approach:

1. **Add a `validate()` method** to `CashflowBreakdown` that checks the currency invariant and returns `Result<()>`. Call it at construction boundaries (the `with_currency()` constructor already guarantees this, but also call it after any waterfall mutation that sets fields).

2. **Replace `.expect()` with `debug_assert!` + unchecked arithmetic** in `interest_expense_total()`. In debug builds, a currency mismatch is caught immediately with a clear assertion message. In release builds, the operation proceeds (producing a potentially incorrect result rather than panicking). This trades a subtle bug for avoiding a production crash.

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
pub fn interest_expense_total(&self) -> Money {
    debug_assert_eq!(
        self.interest_expense_cash.currency(),
        self.interest_expense_pik.currency(),
        "CashflowBreakdown currency invariant violated: cash={}, pik={}",
        self.interest_expense_cash.currency(),
        self.interest_expense_pik.currency(),
    );
    // SAFETY: Currency invariant enforced by with_currency() constructor
    // and validated at construction boundaries. In release builds, if the
    // invariant is violated, this returns the sum treating both as the
    // same currency (incorrect but non-panicking).
    self.interest_expense_cash
        .checked_add(self.interest_expense_pik)
        .unwrap_or(self.interest_expense_cash)
}

/// Validate that all Money fields share the same currency.
///
/// Call after any mutation that sets Money fields to catch invariant
/// violations early (in tests and debug builds).
pub fn validate_currency_invariant(&self) -> crate::Result<()> {
    let expected = self.interest_expense_cash.currency();
    let fields = [
        ("interest_expense_pik", self.interest_expense_pik.currency()),
        ("principal_payment", self.principal_payment.currency()),
        ("debt_balance", self.debt_balance.currency()),
        ("fees", self.fees.currency()),
        ("accrued_interest", self.accrued_interest.currency()),
    ];
    for (name, actual) in fields {
        if actual != expected {
            return Err(crate::error::Error::capital_structure(format!(
                "Currency mismatch in CashflowBreakdown: {name} is {actual}, expected {expected}"
            )));
        }
    }
    Ok(())
}
```

3. **Call `validate_currency_invariant()?`** at key mutation points in `waterfall.rs` where `CashflowBreakdown` fields are set (after sweep calculations, PIK toggle, etc.).

### Files

- `finstack/statements/src/capital_structure/types.rs` — replace `.expect()`, add `validate_currency_invariant()`
- `finstack/statements/src/capital_structure/waterfall.rs` — add validation calls after mutations
- No changes to `get_instrument_field` or `reporting_total` closure signatures

### Breaking Changes

None. The method signature stays `-> Money`. The `#[allow(clippy::expect_used)]` annotation is removed. New `validate_currency_invariant()` is additive.

### Tests

- Existing tests should continue to pass unchanged
- Add a test that constructs a `CashflowBreakdown` with mismatched currencies and verifies `validate_currency_invariant()` returns `Err`
- Add a test that `interest_expense_total()` fires `debug_assert` in debug mode with mismatched currencies

---

## Change 4: XIRR `irr_detailed()` with Root Metadata

### Problem

`InternalRateOfReturn::irr()` returns `Result<f64>` with no metadata about root ambiguity. Callers with complex cashflow patterns (multiple sign changes) have no programmatic way to know the result might not be the economically meaningful root.

### Design

Add an `IrrResult` struct and standalone `irr_detailed()` free functions. The existing `InternalRateOfReturn` trait and `irr()` method are unchanged — no new required methods on the trait.

**Note:** The codebase already has `has_sign_change()` and `has_multiple_sign_changes()` helper functions (lines 421-469 of `xirr.rs`) that count sign changes using `u8`. We'll build on these existing helpers.

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

**New free function (not a trait method):**

```rust
/// Count the number of sign changes in a cashflow sequence.
///
/// Builds on the existing `has_multiple_sign_changes()` helper but returns
/// the exact count instead of a boolean. Zeros are skipped.
pub fn count_sign_changes<I>(iter: I) -> usize
where
    I: IntoIterator<Item = f64>,
{
    let mut prev_sign = 0i8;
    let mut changes = 0usize;
    for value in iter {
        let sign = if value > 0.0 { 1 } else if value < 0.0 { -1 } else { 0 };
        if sign == 0 { continue; }
        if prev_sign != 0 && sign != prev_sign { changes += 1; }
        prev_sign = sign;
    }
    changes
}

/// Calculate IRR with root-ambiguity metadata for periodic cashflows.
pub fn irr_detailed(cashflows: &[f64], guess: Option<f64>) -> crate::Result<IrrResult> {
    let rate = cashflows.irr(guess)?;
    let sign_changes = count_sign_changes(cashflows.iter().copied());
    Ok(IrrResult { rate, sign_changes, multiple_roots_possible: sign_changes > 1 })
}

/// Calculate XIRR with root-ambiguity metadata for dated cashflows.
pub fn xirr_detailed(
    cashflows: &[(Date, f64)],
    day_count: DayCount,
    guess: Option<f64>,
) -> crate::Result<IrrResult> {
    let rate = cashflows.irr_with_daycount(day_count, guess)?;
    let sign_changes = count_sign_changes(cashflows.iter().map(|(_, v)| *v));
    Ok(IrrResult { rate, sign_changes, multiple_roots_possible: sign_changes > 1 })
}
```

### Files

- `finstack/core/src/cashflow/xirr.rs` — `IrrResult` struct, `count_sign_changes()`, `irr_detailed()`, `xirr_detailed()` free functions

### Breaking Changes

None. All additions are new public items. The `InternalRateOfReturn` trait is unchanged — no new required methods, no breakage for downstream implementors.

### Tests

- Test `irr_detailed()` on simple cashflows (1 sign change) → `multiple_roots_possible: false`
- Test on cashflows with 3+ sign changes → `multiple_roots_possible: true`
- Test that `rate` matches `irr()` output exactly
- Test sign change counting with zero-valued cashflows (zeros should be skipped)

---

## Change 5: Portfolio Metrics Arc Clone Reduction

### Problem

In `pricer/registry.rs`, `price_with_metrics()` wraps `market` in `Arc::new(market.clone())` once per call (line 266 or 302, depending on the discounting/non-discounting branch). For portfolio pricing of N instruments, this clones the entire `MarketContext` N times. `MarketContext` contains HashMaps of curves, surfaces, and scalar data — non-trivial to clone.

### Design

Add a `price_with_metrics_arc()` variant that accepts pre-wrapped `Arc<Market>`, and have the existing method delegate to it. The actual method signature must match the current `price_with_metrics` parameters.

**New method on `PricerRegistry`:**

```rust
/// Price with metrics using a shared market context.
///
/// Prefer this over `price_with_metrics()` when pricing multiple instruments
/// against the same market to avoid redundant cloning.
pub fn price_with_metrics_arc(
    &self,
    instrument: &dyn Priceable,
    model: ModelKey,
    market: &Arc<Market>,
    as_of: finstack_core::dates::Date,
    metrics: &[crate::metrics::MetricId],
    options: crate::instruments::PricingOptions,
) -> PricingResult<crate::results::ValuationResult> {
    // ... same logic as price_with_metrics, but uses Arc::clone(market) instead of Arc::new(market.clone())
}
```

**Refactor existing method to delegate:**

```rust
pub fn price_with_metrics(
    &self,
    instrument: &dyn Priceable,
    model: ModelKey,
    market: &Market,
    as_of: finstack_core::dates::Date,
    metrics: &[crate::metrics::MetricId],
    options: crate::instruments::PricingOptions,
) -> PricingResult<crate::results::ValuationResult> {
    let market_arc = Arc::new(market.clone()); // single clone per call
    self.price_with_metrics_arc(instrument, model, &market_arc, as_of, metrics, options)
}
```

The internal `build_with_metrics_dyn()` already accepts `Arc<Market>`, so this just lifts the `Arc::new()` call up one level. Portfolio-level callers can wrap once and share the `Arc` across all instrument pricings, saving N-1 clones.

**Note:** Line 261 also does `Arc::new(self.clone())` — cloning the `PricerRegistry` itself on every call. This is a separate concern (the registry is typically lightweight — a HashMap of function pointers). If profiling shows it's significant, the same `Arc`-lifting pattern can be applied, but it's out of scope for this change.

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

4. **`finstack/core/src/expr/eval.rs`** (line 87-89, `CompiledExpr` struct definition) — **Fix existing incorrect doc.** The current doc says "Not `Sync` due to mutable scratch buffers" but this is wrong: `CompiledExpr` holds `Mutex<ScratchArena>`, `Arc<Mutex<ExpressionCache>>`, and `OnceLock<ExecutionPlan>` — all `Sync`. Replace with: "`CompiledExpr` is both `Send` and `Sync`. Internal scratch buffers and caches are protected by `Mutex`. For parallel evaluation, either share a single instance (concurrent `eval()` calls will serialize on the scratch `Mutex`) or clone for independent scratch buffers per thread."

5. **`finstack/core/src/money/fx.rs`** — doc on `FxMatrix`: "Uses interior `Mutex` for rate caching. Under high concurrency, cache lookups serialize through the lock. For performance-critical parallel pricing, consider pre-fetching rates or using one `FxMatrix` per thread."

### Files

5 files, doc comments only.

### Breaking Changes

None.

### Tests

None required (documentation only).
