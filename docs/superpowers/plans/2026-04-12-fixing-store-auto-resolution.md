# Fixing Store Auto-Resolution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Centralize the `FIXING:{curve_id}` convention into a shared utility module with clear error messages, and replace all scattered inline usages across the codebase.

**Architecture:** A new `finstack/core/src/market_data/fixings.rs` module provides `FIXING_PREFIX`, `fixing_series_id()`, `get_fixing_series()`, `require_fixing_value()`, and `require_fixing_value_exact()`. All existing callsites in IRS pricer, cashflow, metrics, basis swap, and cap/floor are migrated to use these helpers. No new types on `MarketContext`; fixings remain as `ScalarTimeSeries` in the existing `series` map.

**Tech Stack:** Rust (finstack-core, finstack-valuations), cargo nextest for testing.

---

### Task 1: Create the fixings utility module with tests

**Files:**
- Create: `finstack/core/src/market_data/fixings.rs`
- Modify: `finstack/core/src/market_data/mod.rs:71-103`
- Test: inline `#[cfg(test)]` module in `fixings.rs`

- [ ] **Step 1: Write the failing tests**

Create `finstack/core/src/market_data/fixings.rs` with tests only (the public functions don't exist yet, so tests won't compile):

```rust
//! Shared utilities for historical rate fixing lookups.
//!
//! Fixings are stored as [`ScalarTimeSeries`] in [`MarketContext`] using the
//! convention `FIXING:{forward_curve_id}`. This module centralizes that
//! convention and provides helpers with clear error messages for seasoned
//! instrument pricing.

use crate::dates::Date;
use crate::market_data::context::MarketContext;
use crate::market_data::scalars::ScalarTimeSeries;
use crate::Result;

/// Canonical prefix for fixing series stored in MarketContext.
pub const FIXING_PREFIX: &str = "FIXING:";

/// Build the canonical series ID for a given forward curve / rate index.
///
/// # Examples
///
/// ```
/// use finstack_core::market_data::fixings::fixing_series_id;
/// assert_eq!(fixing_series_id("USD-SOFR"), "FIXING:USD-SOFR");
/// ```
pub fn fixing_series_id(forward_curve_id: &str) -> String {
    format!("{}{}", FIXING_PREFIX, forward_curve_id)
}

/// Look up the fixing series for a rate index in MarketContext.
///
/// Returns a clear error when the series is missing, directing the user
/// to provide the expected `ScalarTimeSeries`.
pub fn get_fixing_series<'a>(
    context: &'a MarketContext,
    forward_curve_id: &str,
) -> Result<&'a ScalarTimeSeries> {
    let id = fixing_series_id(forward_curve_id);
    context.get_series(&id).map_err(|_| {
        crate::Error::Validation(format!(
            "No fixing series found for index '{forward_curve_id}'. \
             Seasoned instruments require a ScalarTimeSeries with id '{id}' \
             containing historical observations for dates before the valuation date."
        ))
    })
}

/// Require a fixing value from an already-resolved optional series.
///
/// Uses `value_on()` (step interpolation / LOCF), appropriate for overnight
/// RFR fixings in the compounded path.
///
/// Returns a clear error when the series is `None` or the date is missing.
pub fn require_fixing_value(
    series: Option<&ScalarTimeSeries>,
    forward_curve_id: &str,
    date: Date,
    as_of: Date,
) -> Result<f64> {
    let s = series.ok_or_else(|| {
        crate::Error::Validation(format!(
            "Seasoned instrument requires fixings for index '{forward_curve_id}' on {date} \
             (valuation date: {as_of}). Provide a ScalarTimeSeries with id '{}'.",
            fixing_series_id(forward_curve_id)
        ))
    })?;
    s.value_on(date).map_err(|e| {
        crate::Error::Validation(format!(
            "Missing fixing for '{forward_curve_id}' on {date} (valuation date: {as_of}). \
             The fixing series exists but lookup failed: {e}"
        ))
    })
}

/// Require a fixing value using exact-date matching (no interpolation).
///
/// Fails if no observation exists for the exact requested date.
/// Appropriate for term rate fixings (e.g., 3M LIBOR resets).
pub fn require_fixing_value_exact(
    series: Option<&ScalarTimeSeries>,
    forward_curve_id: &str,
    date: Date,
    as_of: Date,
) -> Result<f64> {
    let s = series.ok_or_else(|| {
        crate::Error::Validation(format!(
            "Seasoned instrument requires fixings for index '{forward_curve_id}' on {date} \
             (valuation date: {as_of}). Provide a ScalarTimeSeries with id '{}'.",
            fixing_series_id(forward_curve_id)
        ))
    })?;
    s.value_on_exact(date).map_err(|e| {
        crate::Error::Validation(format!(
            "Missing fixing for '{forward_curve_id}' on {date} (valuation date: {as_of}). \
             The fixing series exists but has no exact observation: {e}"
        ))
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::market_data::scalars::ScalarTimeSeries;
    use time::macros::date;

    fn sample_series() -> ScalarTimeSeries {
        ScalarTimeSeries::new(
            "FIXING:USD-SOFR",
            vec![
                (date!(2024 - 01 - 02), 0.053),
                (date!(2024 - 01 - 03), 0.054),
                (date!(2024 - 01 - 05), 0.052),
            ],
            None,
        )
        .expect("valid series")
    }

    #[test]
    fn fixing_series_id_builds_correct_key() {
        assert_eq!(fixing_series_id("USD-SOFR"), "FIXING:USD-SOFR");
        assert_eq!(fixing_series_id("EUR-ESTR"), "FIXING:EUR-ESTR");
    }

    #[test]
    fn get_fixing_series_returns_series_when_present() {
        let series = sample_series();
        let ctx = MarketContext::new().insert_series(series);
        let result = get_fixing_series(&ctx, "USD-SOFR");
        assert!(result.is_ok());
    }

    #[test]
    fn get_fixing_series_errors_when_missing() {
        let ctx = MarketContext::new();
        let result = get_fixing_series(&ctx, "USD-SOFR");
        assert!(result.is_err());
        let msg = result.expect_err("should error").to_string();
        assert!(msg.contains("FIXING:USD-SOFR"), "error should mention series id: {msg}");
        assert!(msg.contains("USD-SOFR"), "error should mention index: {msg}");
    }

    #[test]
    fn require_fixing_value_returns_rate_via_locf() {
        let series = sample_series();
        let as_of = date!(2024 - 01 - 10);
        // Jan 4 is not observed; LOCF from Jan 3 (0.054)
        let rate = require_fixing_value(
            Some(&series),
            "USD-SOFR",
            date!(2024 - 01 - 04),
            as_of,
        )
        .expect("should resolve via LOCF");
        assert!((rate - 0.054).abs() < 1e-10);
    }

    #[test]
    fn require_fixing_value_errors_when_series_is_none() {
        let result = require_fixing_value(
            None,
            "USD-SOFR",
            date!(2024 - 01 - 02),
            date!(2024 - 01 - 10),
        );
        assert!(result.is_err());
        let msg = result.expect_err("should error").to_string();
        assert!(msg.contains("FIXING:USD-SOFR"), "should mention series id: {msg}");
        assert!(msg.contains("2024-01-02"), "should mention date: {msg}");
    }

    #[test]
    fn require_fixing_value_exact_returns_rate_on_observed_date() {
        let series = sample_series();
        let rate = require_fixing_value_exact(
            Some(&series),
            "USD-SOFR",
            date!(2024 - 01 - 03),
            date!(2024 - 01 - 10),
        )
        .expect("exact date exists");
        assert!((rate - 0.054).abs() < 1e-10);
    }

    #[test]
    fn require_fixing_value_exact_errors_on_unobserved_date() {
        let series = sample_series();
        let result = require_fixing_value_exact(
            Some(&series),
            "USD-SOFR",
            date!(2024 - 01 - 04), // Not observed
            date!(2024 - 01 - 10),
        );
        assert!(result.is_err());
        let msg = result.expect_err("should error").to_string();
        assert!(msg.contains("2024-01-04"), "should mention date: {msg}");
    }

    #[test]
    fn require_fixing_value_exact_errors_when_series_is_none() {
        let result = require_fixing_value_exact(
            None,
            "USD-SOFR",
            date!(2024 - 01 - 02),
            date!(2024 - 01 - 10),
        );
        assert!(result.is_err());
        let msg = result.expect_err("should error").to_string();
        assert!(msg.contains("FIXING:USD-SOFR"), "should mention series id: {msg}");
    }
}
```

- [ ] **Step 2: Register the module**

In `finstack/core/src/market_data/mod.rs`, add between line 85 (`pub mod scalars;`) and line 86 (`pub mod surfaces;`):

```rust
/// Historical rate fixing lookup utilities.
///
/// Provides the canonical `FIXING:{curve_id}` convention and shared helpers
/// for seasoned instrument pricing.
pub mod fixings;
```

- [ ] **Step 3: Run tests to verify they pass**

Run:

```bash
cargo nextest run -p finstack-core --lib --filter-expr 'test(fixings::)'
```

Expected: All 7 tests pass. The module contains both the implementation and the tests, so this is not a red-green-refactor cycle — the utility functions are simple enough to write directly with their tests.

- [ ] **Step 4: Run doc tests**

Run:

```bash
cargo test -p finstack-core --doc -- fixings
```

Expected: The `fixing_series_id` doc test passes.

- [ ] **Step 5: Commit**

```bash
git add finstack/core/src/market_data/fixings.rs finstack/core/src/market_data/mod.rs
git commit -m "feat(fixings): add shared fixing lookup utility module

Centralizes the FIXING:{curve_id} convention with get_fixing_series(),
require_fixing_value(), and require_fixing_value_exact() helpers."
```

---

### Task 2: Migrate IRS pricer to use fixing helpers

**Files:**
- Modify: `finstack/valuations/src/instruments/rates/irs/pricer.rs:354-356`

- [ ] **Step 1: Replace inline format in compute_pv_raw**

In `finstack/valuations/src/instruments/rates/irs/pricer.rs`, replace lines 355-356:

```rust
    let fixings_id = format!("FIXING:{}", irs.float.forward_curve_id.as_str());
    let fixings = context.get_series(&fixings_id).ok();
```

with:

```rust
    let fixings = finstack_core::market_data::fixings::get_fixing_series(context, irs.float.forward_curve_id.as_str()).ok();
```

Also add the `fixings` import if not already present. The function returns `Result<&ScalarTimeSeries>`, and `.ok()` converts to `Option` — matching the existing flow exactly.

- [ ] **Step 2: Run existing IRS tests to verify no regressions**

Run:

```bash
cargo nextest run -p finstack-valuations --lib --filter-expr 'test(irs::pricer::)'
```

Expected: All existing pricer tests pass unchanged.

- [ ] **Step 3: Run the seasoned compounding accuracy tests**

Run:

```bash
cargo nextest run -p finstack-valuations --test test_compounding_accuracy
```

Expected: All tests pass (these exercise the fixing path end-to-end).

- [ ] **Step 4: Commit**

```bash
git add finstack/valuations/src/instruments/rates/irs/pricer.rs
git commit -m "refactor(irs): use fixing helpers in compute_pv_raw"
```

---

### Task 3: Migrate IRS cashflow to use fixing helpers

**Files:**
- Modify: `finstack/valuations/src/instruments/rates/irs/cashflow.rs:157-165` (projected_overnight_rate)
- Modify: `finstack/valuations/src/instruments/rates/irs/cashflow.rs:478` (float_leg_schedule_with_curves_as_of)

- [ ] **Step 1: Replace inline error in projected_overnight_rate**

In `finstack/valuations/src/instruments/rates/irs/cashflow.rs`, replace lines 157-165:

```rust
    if obs_start < inputs.projection_base_date {
        let series = inputs.fixings.ok_or_else(|| {
            finstack_core::Error::Validation(format!(
                "Seasoned compounded swap requires RFR fixings for dates before as_of (missing series). \
                 Provide ScalarTimeSeries id='FIXING:{}' with business-day observations.",
                inputs.float.forward_curve_id.as_str()
            ))
        })?;
        return series.value_on(obs_start);
    }
```

with:

```rust
    if obs_start < inputs.projection_base_date {
        return finstack_core::market_data::fixings::require_fixing_value(
            inputs.fixings,
            inputs.float.forward_curve_id.as_str(),
            obs_start,
            inputs.projection_base_date,
        );
    }
```

- [ ] **Step 2: Replace inline format in float_leg_schedule_with_curves_as_of**

In the same file, replace line 478:

```rust
            let fixings_id = format!("FIXING:{}", float.forward_curve_id.as_str());
            let fixings = market.get_series(&fixings_id).ok();
```

with:

```rust
            let fixings = finstack_core::market_data::fixings::get_fixing_series(market, float.forward_curve_id.as_str()).ok();
```

- [ ] **Step 3: Run compounding and cashflow tests**

Run:

```bash
cargo nextest run -p finstack-valuations --lib --filter-expr 'test(irs::)' && cargo nextest run -p finstack-valuations --test test_compounding_accuracy
```

Expected: All pass.

- [ ] **Step 4: Commit**

```bash
git add finstack/valuations/src/instruments/rates/irs/cashflow.rs
git commit -m "refactor(irs): use fixing helpers in cashflow compounding"
```

---

### Task 4: Migrate shared swap_legs to use fixing helpers

**Files:**
- Modify: `finstack/valuations/src/instruments/common/pricing/swap_legs.rs:747-756`

- [ ] **Step 1: Replace inline error in pv_floating_leg**

In `finstack/valuations/src/instruments/common/pricing/swap_legs.rs`, replace lines 747-756:

```rust
        let index_rate = if reset_date < as_of {
            // Past reset: require historical fixing
            let series = fixings.ok_or_else(|| {
                finstack_core::Error::Validation(format!(
                    "Seasoned floating leg requires fixings for reset date {} (before as_of {}). \
                     Provide ScalarTimeSeries with historical index observations.",
                    reset_date, as_of
                ))
            })?;
            series.value_on_exact(reset_date)?
```

with:

```rust
        let index_rate = if reset_date < as_of {
            // Past reset: require historical fixing (exact date match for term resets)
            finstack_core::market_data::fixings::require_fixing_value_exact(
                fixings,
                "floating-leg",
                reset_date,
                as_of,
            )?
```

Note: This callsite doesn't have access to the `forward_curve_id` (it receives a generic `Option<&ScalarTimeSeries>`). Using `"floating-leg"` as the identifier is acceptable — the error message still clearly indicates the date and valuation context. The callers (IRS pricer, basis swap) resolve the series using the proper `FIXING:{id}` before passing it in.

- [ ] **Step 2: Run shared swap_legs tests**

Run:

```bash
cargo nextest run -p finstack-valuations --lib --filter-expr 'test(swap_legs::)'
```

Expected: All pass, including `pv_floating_leg_seasoned_requires_fixings` and `pv_floating_leg_seasoned_uses_fixings`. Note the error message assertions in the test at line 1262 check for `"fixings"` or `"Seasoned"` — both strings still appear in the new error message from `require_fixing_value_exact`.

- [ ] **Step 3: Commit**

```bash
git add finstack/valuations/src/instruments/common/pricing/swap_legs.rs
git commit -m "refactor(swap-legs): use fixing helpers in pv_floating_leg"
```

---

### Task 5: Migrate IRS metrics (par_rate, pv_float)

**Files:**
- Modify: `finstack/valuations/src/instruments/rates/irs/metrics/par_rate.rs:182-183`
- Modify: `finstack/valuations/src/instruments/rates/irs/metrics/pv_float.rs:55-56`

- [ ] **Step 1: Replace inline format in par_rate.rs**

In `finstack/valuations/src/instruments/rates/irs/metrics/par_rate.rs`, replace lines 182-183:

```rust
    let fixings_id = format!("FIXING:{}", irs.float.forward_curve_id.as_str());
    let fixings = ctx.curves.get_series(&fixings_id).ok();
```

with:

```rust
    let fixings = finstack_core::market_data::fixings::get_fixing_series(ctx.curves, irs.float.forward_curve_id.as_str()).ok();
```

- [ ] **Step 2: Replace inline format in pv_float.rs**

In `finstack/valuations/src/instruments/rates/irs/metrics/pv_float.rs`, replace lines 55-56:

```rust
        let fixings_id = format!("FIXING:{}", irs.float.forward_curve_id.as_str());
        let fixings = context.curves.get_series(&fixings_id).ok();
```

with:

```rust
        let fixings = finstack_core::market_data::fixings::get_fixing_series(context.curves, irs.float.forward_curve_id.as_str()).ok();
```

- [ ] **Step 3: Run IRS metric tests**

Run:

```bash
cargo nextest run -p finstack-valuations --lib --filter-expr 'test(irs::metrics::)' && cargo nextest run -p finstack-valuations --test test_swap_pricing
```

Expected: All pass.

- [ ] **Step 4: Commit**

```bash
git add finstack/valuations/src/instruments/rates/irs/metrics/par_rate.rs finstack/valuations/src/instruments/rates/irs/metrics/pv_float.rs
git commit -m "refactor(irs-metrics): use fixing helpers in par_rate and pv_float"
```

---

### Task 6: Migrate basis swap and cap/floor

**Files:**
- Modify: `finstack/valuations/src/instruments/rates/basis_swap/types.rs:461-462`
- Modify: `finstack/valuations/src/instruments/rates/cap_floor/types.rs:593-595`

- [ ] **Step 1: Replace inline format in basis_swap**

In `finstack/valuations/src/instruments/rates/basis_swap/types.rs`, replace lines 461-462:

```rust
        let fixings_id = format!("FIXING:{}", leg.forward_curve_id.as_str());
        let fixings = context.get_series(&fixings_id).ok();
```

with:

```rust
        let fixings = finstack_core::market_data::fixings::get_fixing_series(context, leg.forward_curve_id.as_str()).ok();
```

- [ ] **Step 2: Replace local helper in cap_floor**

In `finstack/valuations/src/instruments/rates/cap_floor/types.rs`, replace the local helper function at lines 593-595:

```rust
fn cap_floor_fixing_series_id(forward_curve_id: &CurveId) -> String {
    format!("FIXING:{}", forward_curve_id.as_str())
}
```

with:

```rust
fn cap_floor_fixing_series_id(forward_curve_id: &CurveId) -> String {
    finstack_core::market_data::fixings::fixing_series_id(forward_curve_id.as_str())
}
```

Alternatively, if the function is only called once, inline the call to `fixing_series_id()` at the callsite and remove the local wrapper entirely. Check usage first:

```bash
grep -n 'cap_floor_fixing_series_id' finstack/valuations/src/instruments/rates/cap_floor/types.rs
```

If used only once, delete the wrapper and replace the callsite directly.

- [ ] **Step 3: Run basis swap and cap/floor tests**

Run:

```bash
cargo nextest run -p finstack-valuations --lib --filter-expr 'test(basis_swap::)' && cargo nextest run -p finstack-valuations --test test_basis_swap && cargo nextest run -p finstack-valuations --lib --filter-expr 'test(cap_floor::)' && cargo nextest run -p finstack-valuations --test pricing
```

Expected: All pass.

- [ ] **Step 4: Commit**

```bash
git add finstack/valuations/src/instruments/rates/basis_swap/types.rs finstack/valuations/src/instruments/rates/cap_floor/types.rs
git commit -m "refactor(basis-swap,cap-floor): use fixing helpers"
```

---

### Task 7: Final integration test sweep

**Files:** None (test-only)

- [ ] **Step 1: Run full Rust test suite**

Run:

```bash
CARGO_INCREMENTAL=1 cargo nextest run --workspace --exclude finstack-py --features mc,test-utils --lib --test '*' --no-fail-fast
```

Expected: All tests pass. No regressions.

- [ ] **Step 2: Run clippy**

Run:

```bash
cargo clippy --workspace --exclude finstack-py --features mc,test-utils -- -D warnings
```

Expected: No warnings or errors.

- [ ] **Step 3: Run doc tests**

Run:

```bash
cargo test --workspace --exclude finstack-py --doc --features mc
```

Expected: All doc tests pass, including the new `fixing_series_id` example.

- [ ] **Step 4: Verify no remaining inline FIXING: format calls in production code**

Run:

```bash
grep -rn 'format!("FIXING:' finstack/valuations/src/ finstack/core/src/ --include='*.rs' | grep -v '#\[cfg(test)\]' | grep -v 'mod tests'
```

Expected: Only the canonical `fixing_series_id()` in `fixings.rs` should contain the pattern. Any remaining hits in production code need migration.

- [ ] **Step 5: Commit (if any fixups were needed)**

```bash
git add -A && git commit -m "chore: final cleanup after fixing helpers migration"
```
