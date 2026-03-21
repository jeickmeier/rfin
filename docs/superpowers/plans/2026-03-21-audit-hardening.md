# Audit Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement 6 production robustness improvements identified during deep codebase audit.

**Architecture:** Changes are isolated to individual crates with no cross-cutting dependencies between tasks. Each task can be implemented and tested independently. Tasks are ordered by dependency (error types first, then consumers).

**Tech Stack:** Rust (finstack workspace), `thiserror` for errors, `serde` for serialization, standard `cargo test`

**Spec:** `docs/superpowers/specs/2026-03-21-audit-hardening-design.md`

---

## File Map

| Task | Files Modified | Files Created |
|------|---------------|---------------|
| 1 | `finstack/core/src/error/inputs.rs` | — |
| 2 | `finstack/core/src/expr/eval.rs` | — |
| 3 | `finstack/core/src/math/interp/utils.rs`, `finstack/core/src/math/interp/strategies.rs` | — |
| 4 | `finstack/statements/src/capital_structure/types.rs`, `finstack/statements/src/capital_structure/waterfall.rs` | — |
| 5 | `finstack/core/src/cashflow/xirr.rs`, `finstack/core/src/cashflow/mod.rs` | — |
| 6 | `finstack/valuations/src/pricer/registry.rs`, `finstack/valuations/src/instruments/common/helpers.rs` | — |
| 7 | `finstack/core/src/dates/daycount.rs`, `finstack/core/src/math/solver.rs`, `finstack/core/src/expr/eval.rs`, `finstack/core/src/money/fx.rs`, `finstack/portfolio/src/portfolio.rs` | — |

---

### Task 1: Add New Error Variants to `InputError`

Two new variants needed by Tasks 2 and 3. Do this first so downstream tasks compile.

**Files:**
- Modify: `finstack/core/src/error/inputs.rs`

- [ ] **Step 1: Add `TooLarge` variant**

In `finstack/core/src/error/inputs.rs`, add after the `Invalid` variant (line ~88), inside the "Basic Validation" section:

```rust
    /// Requested allocation or data structure exceeds configured limit.
    #[error("Allocation too large for {what}: requested {requested_bytes} bytes, limit {limit_bytes} bytes")]
    TooLarge {
        /// Description of what exceeded the limit (e.g., "expression arena").
        what: String,
        /// Number of bytes requested.
        requested_bytes: usize,
        /// Configured limit in bytes.
        limit_bytes: usize,
    },

    /// Consecutive knots are too close together for stable interpolation.
    #[error("Consecutive knots are too close together for stable interpolation")]
    KnotSpacingTooSmall,
```

- [ ] **Step 2: Run tests to verify compilation**

Run: `cargo test -p finstack-core --lib -- input 2>&1 | head -20`
Expected: All existing tests pass. The new variants are `#[non_exhaustive]` covered by the enum attribute on line 61.

- [ ] **Step 3: Commit**

```
git add finstack/core/src/error/inputs.rs
git commit -m "feat(core): add TooLarge and KnotSpacingTooSmall error variants"
```

---

### Task 2: Expression Arena Bounds Check

**Files:**
- Modify: `finstack/core/src/expr/eval.rs:54-64` (EvalOpts) and `eval.rs:209-214` (arena allocation)

- [ ] **Step 1: Write the failing test**

Add to the test module at the bottom of `finstack/core/src/expr/eval.rs`:

```rust
#[test]
fn arena_rejects_oversized_allocation() {
    use super::*;
    // Create a compiled expression with multiple nodes (Add requires a plan with 3+ nodes)
    let ast = Expr::BinOp {
        op: BinOp::Add,
        left: Box::new(Expr::Column(0)),
        right: Box::new(Expr::Column(1)),
    };
    let expr = CompiledExpr::new(ast).expect("should compile");

    // Create large column data
    let col: Vec<f64> = vec![1.0; 1000];
    let cols: Vec<&[f64]> = vec![&col, &col];
    let ctx = SimpleContext::default();

    // Set a very small arena limit (e.g., 100 bytes)
    let opts = EvalOpts {
        max_arena_bytes: 100,
        ..EvalOpts::default()
    };
    let result = expr.eval(&ctx, &cols, opts);
    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    assert!(err_str.contains("too large") || err_str.contains("TooLarge"),
        "Expected TooLarge error, got: {err_str}");
}

#[test]
fn arena_accepts_normal_allocation() {
    use super::*;
    let ast = Expr::Column(0);
    let expr = CompiledExpr::new(ast).expect("should compile");
    let col = vec![1.0, 2.0, 3.0];
    let cols: Vec<&[f64]> = vec![&col];
    let ctx = SimpleContext::default();
    let opts = EvalOpts::default(); // 1GB limit
    let result = expr.eval(&ctx, &cols, opts);
    assert!(result.is_ok());
}

#[test]
fn arena_check_disabled_when_zero() {
    use super::*;
    let ast = Expr::Column(0);
    let expr = CompiledExpr::new(ast).expect("should compile");
    let col = vec![1.0, 2.0, 3.0];
    let cols: Vec<&[f64]> = vec![&col];
    let ctx = SimpleContext::default();
    let opts = EvalOpts {
        max_arena_bytes: 0, // disabled
        ..EvalOpts::default()
    };
    let result = expr.eval(&ctx, &cols, opts);
    assert!(result.is_ok());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p finstack-core --lib -- arena_rejects 2>&1 | tail -5`
Expected: FAIL — `max_arena_bytes` field doesn't exist yet.

- [ ] **Step 3: Add `max_arena_bytes` to `EvalOpts`**

In `finstack/core/src/expr/eval.rs`, modify `EvalOpts` (line 54):

1. Remove `Default` from the derive list on line 54:
```rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EvalOpts {
```

2. Add the new field after `cache_budget_mb`:
```rust
    /// Maximum arena allocation in bytes. Defaults to 1 GB.
    /// Set to 0 to disable the check.
    #[serde(default = "default_max_arena_bytes")]
    pub max_arena_bytes: usize,
```

3. Add the default function and manual `Default` impl after the struct:
```rust
fn default_max_arena_bytes() -> usize {
    1_073_741_824 // 1 GB
}

impl Default for EvalOpts {
    fn default() -> Self {
        Self {
            plan: None,
            cache_budget_mb: None,
            max_arena_bytes: default_max_arena_bytes(),
        }
    }
}
```

- [ ] **Step 4: Add bounds check before arena allocation**

In `finstack/core/src/expr/eval.rs`, insert before line 214 (`let mut arena = vec![0.0; ...`):

```rust
            // Pre-flight arena size check
            let node_count = plan_to_use.nodes.len();
            let arena_elements = len.checked_mul(node_count).ok_or_else(|| {
                crate::Error::from(crate::InputError::TooLarge {
                    what: "expression arena".into(),
                    requested_bytes: usize::MAX,
                    limit_bytes: opts.max_arena_bytes,
                })
            })?;
            let arena_bytes = arena_elements
                .checked_mul(std::mem::size_of::<f64>())
                .unwrap_or(usize::MAX);
            if opts.max_arena_bytes > 0 && arena_bytes > opts.max_arena_bytes {
                return Err(crate::InputError::TooLarge {
                    what: "expression arena".into(),
                    requested_bytes: arena_bytes,
                    limit_bytes: opts.max_arena_bytes,
                }
                .into());
            }
```

Then change the existing allocation line from:
```rust
            let mut arena = vec![0.0; len * plan_to_use.nodes.len()];
```
to:
```rust
            let mut arena = vec![0.0; arena_elements];
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p finstack-core --lib -- arena_ 2>&1 | tail -10`
Expected: All 3 new tests PASS. No existing test regressions.

- [ ] **Step 6: Run full core test suite**

Run: `cargo test -p finstack-core 2>&1 | tail -5`
Expected: All tests pass.

- [ ] **Step 7: Commit**

```
git add finstack/core/src/expr/eval.rs
git commit -m "feat(core): add arena bounds check in expression evaluator

Prevents OOM from pathological expressions by checking arena size
against configurable max_arena_bytes (default 1GB) before allocation."
```

---

### Task 3: Minimum Knot Spacing Validation

**Files:**
- Modify: `finstack/core/src/math/interp/utils.rs`
- Modify: `finstack/core/src/math/interp/strategies.rs`

- [ ] **Step 1: Write the failing test**

Add to test module in `finstack/core/src/math/interp/utils.rs` (or create one if none exists):

```rust
#[cfg(test)]
mod knot_spacing_tests {
    use super::*;

    #[test]
    fn rejects_knots_too_close() {
        let knots = [1.0, 1.0 + 1e-16]; // gap = 1e-16, way below threshold
        let result = validate_knot_spacing(&knots, MIN_RELATIVE_KNOT_GAP);
        assert!(result.is_err());
    }

    #[test]
    fn accepts_knots_with_sufficient_spacing() {
        let knots = [0.0, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0];
        let result = validate_knot_spacing(&knots, MIN_RELATIVE_KNOT_GAP);
        assert!(result.is_ok());
    }

    #[test]
    fn near_zero_knots_use_absolute_floor() {
        // For knots near zero, max(|k|, 1.0) = 1.0, so threshold is MIN_RELATIVE_KNOT_GAP
        let knots = [0.001, 0.002]; // gap = 0.001 >> 1e-10
        let result = validate_knot_spacing(&knots, MIN_RELATIVE_KNOT_GAP);
        assert!(result.is_ok());
    }

    #[test]
    fn large_knots_use_relative_threshold() {
        // For knots at 1e6, threshold = 1e-10 * 1e6 = 1e-4
        let knots = [1_000_000.0, 1_000_000.00001]; // gap = 1e-5 < 1e-4
        let result = validate_knot_spacing(&knots, MIN_RELATIVE_KNOT_GAP);
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p finstack-core --lib -- knot_spacing 2>&1 | tail -5`
Expected: FAIL — `validate_knot_spacing` and `MIN_RELATIVE_KNOT_GAP` don't exist.

- [ ] **Step 3: Add validation function and constant**

In `finstack/core/src/math/interp/utils.rs`, add after the `validate_knots` function (after line 63):

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
/// for small-magnitude knots. The `max(|k[i]|, 1.0)` floor ensures the threshold never
/// shrinks below `min_relative_gap` for knots near zero.
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

- [ ] **Step 4: Run tests**

Run: `cargo test -p finstack-core --lib -- knot_spacing 2>&1 | tail -10`
Expected: All 4 tests PASS.

- [ ] **Step 5: Integrate into slope-computing strategies**

In `finstack/core/src/math/interp/strategies.rs`, add to the import block at the top (line ~7):

```rust
use super::utils::{validate_knot_spacing, MIN_RELATIVE_KNOT_GAP};
```

Then add `validate_knot_spacing(knots, MIN_RELATIVE_KNOT_GAP)?;` as the first validation call in `from_raw()` for these strategies:

1. **`PiecewiseQuadraticForwardStrategy::from_raw()`** (line ~342): Add after `validate_positive_series(values)?;` (line 348):
   ```rust
   validate_knot_spacing(knots, MIN_RELATIVE_KNOT_GAP)?;
   ```

2. **`CubicHermiteStrategy::from_raw()`** (line ~563): Add after the opening brace:
   ```rust
   validate_knot_spacing(knots, MIN_RELATIVE_KNOT_GAP)?;
   ```

3. **`MonotoneConvexStrategy::from_raw()`** (line ~832): Add after `validate_monotone_nonincreasing(values)?;` (line 838):
   ```rust
   validate_knot_spacing(knots, MIN_RELATIVE_KNOT_GAP)?;
   ```

Do NOT add to `LinearStrategy` (line ~25) — its `from_raw()` returns `Ok(Self)` with no precomputation and doesn't use knots. The knot spacing check runs during `Interpolator::new()` → strategy `from_raw()`, so it will be checked for any strategy that actually needs it.

Do NOT add to `LogLinearStrategy` (line ~161) — it uses log-value interpolation and does not divide by knot gaps directly.

- [ ] **Step 6: Run full interpolation test suite**

Run: `cargo test -p finstack-core --lib -- interp 2>&1 | tail -10`
Expected: All existing interpolation tests still pass. Real-world curve tenors have gaps >> 1e-10.

- [ ] **Step 7: Run full core test suite**

Run: `cargo test -p finstack-core 2>&1 | tail -5`
Expected: All tests pass.

- [ ] **Step 8: Commit**

```
git add finstack/core/src/math/interp/utils.rs finstack/core/src/math/interp/strategies.rs
git commit -m "feat(core): add minimum knot spacing validation for interpolators

Prevents numerical instability from near-zero knot gaps in strategies
that compute slopes/derivatives (Linear, PiecewiseQuadraticForward,
CubicHermite). Uses relative threshold with absolute floor."
```

---

### Task 4: CashflowBreakdown Validated Construction

**Files:**
- Modify: `finstack/statements/src/capital_structure/types.rs:143-149`
- Modify: `finstack/statements/src/capital_structure/waterfall.rs` (validation calls)

- [ ] **Step 1: Write the failing test**

Add to the test module in `finstack/statements/src/capital_structure/types.rs`:

```rust
#[test]
fn validate_currency_invariant_catches_mismatch() {
    let mut cf = CashflowBreakdown::with_currency(Currency::USD);
    // Manually violate the invariant
    cf.interest_expense_pik = Money::new(100.0, Currency::EUR);
    let result = cf.validate_currency_invariant();
    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    assert!(err_str.contains("Currency mismatch"), "Expected currency mismatch error, got: {err_str}");
}

#[test]
fn validate_currency_invariant_passes_for_valid() {
    let cf = CashflowBreakdown::with_currency(Currency::USD);
    assert!(cf.validate_currency_invariant().is_ok());
}
```

(Add necessary imports: `use finstack_core::currency::Currency;` and `use finstack_core::money::Money;` — check if already imported in test module.)

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p finstack-statements --lib -- validate_currency 2>&1 | tail -5`
Expected: FAIL — `validate_currency_invariant` doesn't exist.

- [ ] **Step 3: Replace `.expect()` and add validation method**

In `finstack/statements/src/capital_structure/types.rs`, replace lines 143-149:

**Before:**
```rust
    #[allow(clippy::expect_used)] // Type invariant: all Money fields have same currency
    pub fn interest_expense_total(&self) -> Money {
        // SAFETY: Both values in a CashflowBreakdown have the same currency by construction
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
        // Currency invariant enforced by with_currency() constructor and validated
        // at construction boundaries. Fallback returns cash component if violated.
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
        let fields: [(&str, finstack_core::currency::Currency); 5] = [
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

- [ ] **Step 4: Run tests**

Run: `cargo test -p finstack-statements --lib -- validate_currency 2>&1 | tail -10`
Expected: Both new tests PASS.

- [ ] **Step 5: Add validation calls in waterfall mutation points**

In `finstack/statements/src/capital_structure/waterfall.rs`, the waterfall logic mutates `CashflowBreakdown` fields at several points (PIK toggle, sweep calculations, balance updates). After the final mutations for each instrument's period breakdown are complete (before the breakdown is inserted into the result map), add a debug-mode validation call. Find the key mutation sites by searching for assignments to `breakdown.interest_expense_*`, `breakdown.principal_payment`, `breakdown.debt_balance` etc.

At the end of each complete breakdown assembly (where a `CashflowBreakdown` is fully populated), add:

```rust
debug_assert!(staged_breakdown.validate_currency_invariant().is_ok(),
    "Currency invariant violated after waterfall mutation");
```

Use `debug_assert!` (not `?` propagation) to avoid changing function signatures. This catches invariant violations in test/debug builds without affecting release performance.

- [ ] **Step 6: Run full statements test suite**

Run: `cargo test -p finstack-statements 2>&1 | tail -5`
Expected: All tests pass. The `interest_expense_total()` return type is unchanged (`Money`), so all call sites still compile. Debug assertions pass because all test data uses consistent currencies.

- [ ] **Step 7: Commit**

```
git add finstack/statements/src/capital_structure/types.rs finstack/statements/src/capital_structure/waterfall.rs
git commit -m "fix(statements): replace .expect() with debug_assert + validation

Add validate_currency_invariant() for explicit checking at construction
boundaries. Replace panicking .expect() with debug_assert + fallback.
Add debug_assert validation in waterfall mutation paths."
```

---

### Task 5: XIRR `irr_detailed()` with Root Metadata

**Files:**
- Modify: `finstack/core/src/cashflow/xirr.rs`
- Modify: `finstack/core/src/cashflow/mod.rs:119` (re-exports)

- [ ] **Step 1: Write the failing tests**

Add to the existing test module in `finstack/core/src/cashflow/xirr.rs` (after `mod tests {`):

```rust
    #[test]
    fn irr_detailed_simple_cashflow() {
        // Single sign change: -100, +110
        let flows = [-100.0, 110.0];
        let result = irr_detailed(&flows, None).expect("should converge");
        assert!((result.rate - 0.1).abs() < 1e-6, "rate={}", result.rate);
        assert_eq!(result.sign_changes, 1);
        assert!(!result.multiple_roots_possible);
    }

    #[test]
    fn irr_detailed_multiple_sign_changes() {
        // Three sign changes: -, +, -, +
        let flows = [-100.0, 230.0, -132.0, 5.0];
        let result = irr_detailed(&flows, None).expect("should converge");
        assert!(result.sign_changes >= 3);
        assert!(result.multiple_roots_possible);
    }

    #[test]
    fn count_sign_changes_skips_zeros() {
        assert_eq!(count_sign_changes([1.0, 0.0, -1.0].iter().copied()), 1);
        assert_eq!(count_sign_changes([1.0, -1.0, 0.0, 1.0].iter().copied()), 2);
        assert_eq!(count_sign_changes([0.0, 0.0, 1.0].iter().copied()), 0);
    }

    #[test]
    fn irr_detailed_matches_irr() {
        let flows = [-1000.0, 100.0, 100.0, 100.0, 1100.0];
        let irr_simple = flows.as_slice().irr(None).expect("should converge");
        let detailed = irr_detailed(&flows, None).expect("should converge");
        assert!((irr_simple - detailed.rate).abs() < 1e-12);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p finstack-core --lib -- irr_detailed 2>&1 | tail -5`
Expected: FAIL — `irr_detailed`, `count_sign_changes`, `IrrResult` don't exist.

- [ ] **Step 3: Add `IrrResult`, `count_sign_changes`, `irr_detailed`, `xirr_detailed`**

In `finstack/core/src/cashflow/xirr.rs`, add before the `#[cfg(test)]` block (before line ~471):

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

/// Count the number of sign changes in a numeric sequence.
///
/// Zero values are skipped. This count is used by Descartes' rule of signs
/// to bound the number of positive real roots.
pub fn count_sign_changes<I>(iter: I) -> usize
where
    I: IntoIterator<Item = f64>,
{
    let mut prev_sign = 0i8;
    let mut changes = 0usize;
    for value in iter {
        let sign = if value > 0.0 {
            1
        } else if value < 0.0 {
            -1
        } else {
            0
        };
        if sign == 0 {
            continue;
        }
        if prev_sign != 0 && sign != prev_sign {
            changes += 1;
        }
        prev_sign = sign;
    }
    changes
}

/// Calculate IRR with root-ambiguity metadata for periodic cashflows.
///
/// Returns the same rate as [`InternalRateOfReturn::irr()`] along with
/// metadata about how many roots the NPV polynomial may have.
pub fn irr_detailed(cashflows: &[f64], guess: Option<f64>) -> crate::Result<IrrResult> {
    let rate = cashflows.irr(guess)?;
    let sign_changes = count_sign_changes(cashflows.iter().copied());
    Ok(IrrResult {
        rate,
        sign_changes,
        multiple_roots_possible: sign_changes > 1,
    })
}

/// Calculate XIRR with root-ambiguity metadata for dated cashflows.
///
/// Returns the same rate as [`InternalRateOfReturn::irr_with_daycount()`] along
/// with metadata about how many roots the NPV equation may have.
pub fn xirr_detailed(
    cashflows: &[(Date, f64)],
    day_count: DayCount,
    guess: Option<f64>,
) -> crate::Result<IrrResult> {
    let rate = cashflows.irr_with_daycount(day_count, guess)?;
    let sign_changes = count_sign_changes(cashflows.iter().map(|(_, v)| *v));
    Ok(IrrResult {
        rate,
        sign_changes,
        multiple_roots_possible: sign_changes > 1,
    })
}
```

- [ ] **Step 4: Update re-exports**

In `finstack/core/src/cashflow/mod.rs`, update line 119 to include the new public items:

```rust
pub use xirr::{count_sign_changes, irr_detailed, xirr_detailed, xirr_with_daycount_ctx, InternalRateOfReturn, IrrResult};
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p finstack-core --lib -- irr_detailed 2>&1 | tail -10`
Expected: All 4 new tests PASS.

Run: `cargo test -p finstack-core --lib -- count_sign 2>&1 | tail -5`
Expected: PASS.

- [ ] **Step 6: Run full core test suite**

Run: `cargo test -p finstack-core 2>&1 | tail -5`
Expected: All tests pass.

- [ ] **Step 7: Commit**

```
git add finstack/core/src/cashflow/xirr.rs finstack/core/src/cashflow/mod.rs
git commit -m "feat(core): add irr_detailed() with root-ambiguity metadata

New IrrResult struct includes sign_changes count and
multiple_roots_possible flag. Free functions irr_detailed() and
xirr_detailed() wrap existing IRR methods with metadata."
```

---

### Task 6: Portfolio Metrics Arc Clone Reduction

**Note:** In `registry.rs`, `Market` is a type alias for `MarketContext` (line 11: `use finstack_core::market_data::context::MarketContext as Market;`). In `helpers.rs`, `MarketContext` is used directly. Both refer to the same type.

**Files:**
- Modify: `finstack/valuations/src/pricer/registry.rs:237-276`
- Modify: `finstack/valuations/src/instruments/common/helpers.rs:372-392`

- [ ] **Step 1: Add `price_with_metrics_arc` to `PricerRegistry`**

In `finstack/valuations/src/pricer/registry.rs`, after the existing `price_with_metrics` method (which ends around line ~400), add a new method. Then refactor the existing method to delegate.

First, rename the existing `price_with_metrics` body to `price_with_metrics_arc` with `market: &Arc<Market>` parameter:

```rust
    /// Price with metrics using a pre-wrapped shared market context.
    ///
    /// Prefer this over [`price_with_metrics()`](Self::price_with_metrics) when pricing
    /// multiple instruments against the same market to avoid redundant cloning.
    pub fn price_with_metrics_arc(
        &self,
        instrument: &dyn Priceable,
        model: ModelKey,
        market: &Arc<Market>,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
        options: crate::instruments::PricingOptions,
    ) -> PricingResult<crate::results::ValuationResult> {
        // ... move the existing body here, replacing:
        //   Arc::new(market.clone())  →  Arc::clone(market)
        //   market (as &Market)       →  market.as_ref() where needed
    }
```

Then make the original delegate:

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
        let market_arc = Arc::new(market.clone());
        self.price_with_metrics_arc(instrument, model, &market_arc, as_of, metrics, options)
    }
```

Inside `price_with_metrics_arc`, change every `Arc::new(market.clone())` and `std::sync::Arc::new(market.clone())` to `Arc::clone(market)`. The `self.price()` call on line 253 still takes `&Market` — pass `market.as_ref()` there.

- [ ] **Step 2: Add `price_with_metrics_arc` to instrument trait default impl**

In `finstack/valuations/src/instruments/common/helpers.rs`, at line 380-382, the default `price_with_metrics` impl does `Arc::new(market.clone())`. Add a parallel method:

```rust
        fn price_with_metrics_arc(
            &self,
            market: &std::sync::Arc<MarketContext>,
            as_of: Date,
            metrics: &[MetricId],
            options: crate::instruments::common_impl::traits::PricingOptions,
        ) -> finstack_core::Result<crate::results::ValuationResult> {
            let base = self.value(market.as_ref(), as_of)?;
            build_with_metrics_dyn(
                Arc::from(self.clone_box()),
                Arc::clone(market),
                as_of,
                base,
                metrics,
                MetricBuildOptions {
                    cfg: options.config,
                    market_history: options.market_history,
                    ..MetricBuildOptions::default()
                },
            )
        }
```

And refactor the existing `price_with_metrics` to delegate:

```rust
        fn price_with_metrics(
            &self,
            market: &MarketContext,
            as_of: Date,
            metrics: &[MetricId],
            options: crate::instruments::common_impl::traits::PricingOptions,
        ) -> finstack_core::Result<crate::results::ValuationResult> {
            self.price_with_metrics_arc(&Arc::new(market.clone()), as_of, metrics, options)
        }
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p finstack-valuations 2>&1 | tail -10`
Expected: All existing tests pass — behavior unchanged, just clone elimination.

- [ ] **Step 4: Run workspace-wide tests**

Run: `cargo test --workspace 2>&1 | tail -10`
Expected: All tests pass across all crates.

**Note on portfolio consumer:** The portfolio crate (`finstack/portfolio/src/valuation.rs`) calls `position.instrument.price_with_metrics(market, ...)` on the trait, not on `PricerRegistry` directly. To realize the N-1 clone savings at the portfolio level, the portfolio loop would need to wrap `market` in an `Arc` once and call `price_with_metrics_arc` on each instrument. This is a follow-up optimization — the portfolio crate currently takes `&MarketContext`, and changing it to `Arc<MarketContext>` is a larger API change. For now, the new `_arc` variants are available for callers who opt in.

- [ ] **Step 5: Commit**

```
git add finstack/valuations/src/pricer/registry.rs finstack/valuations/src/instruments/common/helpers.rs
git commit -m "perf(valuations): add price_with_metrics_arc to eliminate redundant market clones

New Arc-accepting variants avoid cloning MarketContext on every call.
Existing price_with_metrics() delegates to the Arc variant.
Portfolio-level integration is a follow-up optimization."
```

---

### Task 7: Numeric Precision & Concurrency Documentation

**Files:**
- Modify: `finstack/core/src/dates/daycount.rs` (module doc)
- Modify: `finstack/core/src/math/solver.rs` (constant docs)
- Modify: `finstack/core/src/expr/eval.rs:85-87` (CompiledExpr doc)
- Modify: `finstack/core/src/money/fx.rs` (FxMatrix doc)
- Modify: `finstack/portfolio/src/portfolio.rs` (positions field doc)

- [ ] **Step 1: Add day-count precision doc**

In `finstack/core/src/dates/daycount.rs`, add to the module-level doc comment (typically at top of file):

```rust
//! # Precision
//!
//! Year fractions are computed as `f64` with typical precision ~1e-9 for
//! standard tenors (< 50 years). Precision degrades for very long tenors
//! due to floating-point accumulation. For most bond and swap applications,
//! this precision is well within market conventions.
```

- [ ] **Step 2: Add solver tolerance doc**

In `finstack/core/src/math/solver.rs`, find the `SOLVER_TOLERANCE` constant (or equivalent) and add/enhance its doc comment:

```rust
/// Default solver tolerance for root-finding algorithms.
///
/// Convergence uses dual tolerances: the residual (`|f(x)| < tol`) and the
/// step size (`|x_new - x_old| < tol`). Set to `1e-8` to match QuantLib's
/// professional-grade standard. Previous value (`1e-6`) was Excel-grade.
```

- [ ] **Step 3: Fix CompiledExpr thread-safety doc**

In `finstack/core/src/expr/eval.rs`, replace lines 85-87:

**Before:**
```rust
/// # Thread Safety
///
/// Not `Sync` due to mutable scratch buffers. Clone to share across threads.
```

**After:**
```rust
/// # Thread Safety
///
/// `CompiledExpr` is both `Send` and `Sync`. Internal scratch buffers and
/// caches are protected by `Mutex`. For parallel evaluation, either share a
/// single instance (concurrent `eval()` calls will serialize on the scratch
/// `Mutex`) or clone for independent scratch buffers per thread.
```

- [ ] **Step 4: Add FxMatrix concurrency doc**

In `finstack/core/src/money/fx.rs`, find the `FxMatrix` struct and add to its doc comment:

```rust
/// # Thread Safety
///
/// Uses interior `Mutex` for rate caching. Under high concurrency, cache
/// lookups serialize through the lock. For performance-critical parallel
/// pricing, consider pre-fetching rates or using one `FxMatrix` per thread.
```

- [ ] **Step 5: Add portfolio positions immutability doc**

In `finstack/portfolio/src/portfolio.rs`, find the `positions` field on the `Portfolio` struct and add:

```rust
    /// Instruments behind `Arc` must be immutable after construction.
    /// The portfolio assumes no interior mutability — concurrent reads are
    /// safe, but modifying an instrument after adding it to a portfolio is
    /// undefined behavior at the application level.
    pub positions: Vec<Position>,
```

- [ ] **Step 6: Verify compilation**

Run: `cargo check --workspace 2>&1 | tail -5`
Expected: Clean compilation, no warnings from doc changes.

- [ ] **Step 7: Commit**

```
git add finstack/core/src/dates/daycount.rs finstack/core/src/math/solver.rs \
  finstack/core/src/expr/eval.rs finstack/core/src/money/fx.rs \
  finstack/portfolio/src/portfolio.rs
git commit -m "docs: add numeric precision and concurrency documentation

Document day-count precision (~1e-9), solver dual-tolerance convergence,
CompiledExpr thread safety (is Sync, not isn't), FxMatrix cache
contention, and portfolio instrument immutability contract."
```

---

## Final Verification

- [ ] **Run full workspace test suite**

```
cargo test --workspace 2>&1 | tail -20
```

Expected: All tests pass across all crates.

- [ ] **Run clippy**

```
cargo clippy --workspace 2>&1 | tail -20
```

Expected: No new warnings. The `#[allow(clippy::expect_used)]` removal in Task 4 should not introduce new lint failures.
