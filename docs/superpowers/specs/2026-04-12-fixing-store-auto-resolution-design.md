# Fixing Store Auto-Resolution Design

## Problem

Seasoned floating-rate instruments (swaps with observation/reset dates before `as_of`) require historical fixing rates. The codebase already supports this via `ScalarTimeSeries` stored in `MarketContext` with the convention `FIXING:{forward_curve_id}`, but:

1. The `FIXING:` prefix convention is scattered as inline `format!()` calls across multiple files
2. Error messages when fixings are missing vary in quality and detail across code paths
3. There is no shared utility that new instruments (FRAs, caps/floors, term loans, CLOs) can reuse for fixing resolution

## Goals

- Centralize the `FIXING:{curve_id}` convention into a single utility module
- Provide shared helper functions that any instrument pricer can use for fixing lookups
- Produce clear, actionable error messages when required fixings are missing
- Minimal code churn: no new types on `MarketContext`, no changes to storage mechanism

## Non-Goals

- No new dedicated `FixingStore` type on `MarketContext` -- fixings remain as `ScalarTimeSeries` in the existing `series` map
- No upfront validation pass -- errors occur inline during pricing
- No fixing interpolation changes -- `value_on_exact()` remains the default for term fixings, `value_on()` for overnight
- No Python/WASM binding changes -- the helpers are Rust-internal; bindings already pass fixings through `MarketContext`

## Approach: Inline Detection with Shared Helper

### New Module: `finstack/core/src/market_data/fixings.rs`

A small utility module providing:

#### Constants and ID Construction

```rust
/// Canonical prefix for fixing series stored in MarketContext.
pub const FIXING_PREFIX: &str = "FIXING:";

/// Build the canonical series ID for a given forward curve / rate index.
///
/// Example: `fixing_series_id("USD-SOFR")` returns `"FIXING:USD-SOFR"`.
pub fn fixing_series_id(forward_curve_id: &str) -> String {
    format!("{}{}", FIXING_PREFIX, forward_curve_id)
}
```

#### Series Resolution

```rust
/// Look up the fixing series for a rate index in MarketContext.
///
/// Returns a clear error when the series is missing, directing the user
/// to provide the expected ScalarTimeSeries.
pub fn get_fixing_series<'a>(
    context: &'a MarketContext,
    forward_curve_id: &str,
) -> Result<&'a ScalarTimeSeries>
```

Error when missing:
> "No fixing series found for index '{id}'. Seasoned instruments require a ScalarTimeSeries with id 'FIXING:{id}' containing historical observations for dates before the valuation date."

#### Value Resolution (for callers that already hold `Option<&ScalarTimeSeries>`)

```rust
/// Require a fixing value from an already-resolved optional series.
///
/// Use when the caller has already looked up the series (e.g., via
/// `get_fixing_series()` or `context.get_series(...).ok()`) and needs
/// to retrieve a specific date's value with a clear error on failure.
///
/// Uses `value_on()` (step interpolation / LOCF) by default, which is
/// appropriate for overnight RFR fixings in the compounded path.
/// Callers needing exact-date matching (e.g., term rate fixings in the
/// simple float path) should use `require_fixing_value_exact()` instead.
pub fn require_fixing_value(
    series: Option<&ScalarTimeSeries>,
    forward_curve_id: &str,
    date: Date,
    as_of: Date,
) -> Result<f64>

/// Same as `require_fixing_value` but uses `value_on_exact()` --
/// fails if no observation exists for the exact requested date.
/// Appropriate for term rate fixings (e.g., 3M LIBOR resets).
pub fn require_fixing_value_exact(
    series: Option<&ScalarTimeSeries>,
    forward_curve_id: &str,
    date: Date,
    as_of: Date,
) -> Result<f64>
```

Error when series is None:
> "Seasoned instrument requires fixings for index '{id}' on {date} (valuation date: {as_of}). Provide a ScalarTimeSeries with id 'FIXING:{id}'."

Error when date is missing in series: delegates to the underlying `ScalarTimeSeries` lookup method, wrapping with index context.

### Integration Points

#### 1. IRS Compounded Path (`cashflow.rs`)

`projected_overnight_rate()` already errors when fixings are `None` for past observation dates. Replace the inline `ok_or_else` block (lines 157-165) with a call to `require_fixing_value()` (LOCF / step interpolation, appropriate for daily overnight fixings).

The function signature stays the same -- it still receives `OvernightProjectionInputs` with `fixings: Option<&ScalarTimeSeries>`.

#### 2. IRS Simple Float Path (`swap_legs.rs`)

`pv_floating_leg()` already checks `reset_date < as_of` and requires fixings (lines 747-756). Replace the inline error construction with `require_fixing_value_exact()` (exact-date matching, appropriate for term rate fixings like 3M resets).

The function signature stays the same -- it still receives `fixings: Option<&ScalarTimeSeries>`.

#### 3. Callsite ID Construction (`pricer.rs`, `cashflow.rs`)

Replace `format!("FIXING:{}", curve_id)` with `fixing_series_id(curve_id)` at:
- `pricer.rs:355` in `compute_pv_raw()`
- `cashflow.rs:478` in `float_leg_schedule_with_curves_as_of()`

### Error Message Template

All fixing-related errors follow a consistent pattern:

- **Missing series:** "No fixing series found for index '{id}'. Seasoned instruments require a ScalarTimeSeries with id 'FIXING:{id}' containing historical observations for dates before the valuation date ({as_of})."
- **Missing date:** "Missing fixing for '{id}' on {date} (valuation date: {as_of}). The fixing series exists but does not contain an observation for this date."
- **Series exists but value lookup fails:** Wraps the underlying `ScalarTimeSeries` error with index context.

### New Fixing Support for Additional Instruments

Three floating-rate instrument types currently lack historical fixing support and will be updated:

#### Revolving Credit (`cashflow_engine.rs`)
The cashflow engine's `generate_period_cashflows()` always projects floating rates from the forward curve via `project_floating_rate_with_curve()`. For seasoned facilities with reset dates before `as_of`, the engine will check fixings first. The `CashflowEngine` struct receives `valuation_date` and `fixings` fields.

#### Structured Credit (`utils/rate_helpers.rs`)
`tranche_all_in_rate()`, `asset_all_in_rate()`, and their `try_` variants always project from forward curves. They will receive an `as_of` parameter and use `require_fixing_value_exact()` for dates before `as_of`. The infallible wrappers gracefully degrade to forward projection when fixings are missing.

#### Term Loan (`pricing/discounting.rs`)
Term loans use the cashflow builder which doesn't have fixing support. The pricer will post-process generated cashflows: after `generate_cashflows()` but before discounting, replace rates for `FloatReset` flows with `reset_date < as_of` using historical fixings.

### File Changes Summary

| File | Change |
|------|--------|
| `finstack/core/src/market_data/fixings.rs` | **New.** `FIXING_PREFIX`, `fixing_series_id()`, `get_fixing_series()`, `require_fixing_value()`, `require_fixing_value_exact()` |
| `finstack/core/src/market_data/mod.rs` | Add `pub mod fixings;` re-export |
| `finstack/valuations/.../irs/pricer.rs` | Use `fixing_series_id()` instead of inline format |
| `finstack/valuations/.../irs/cashflow.rs` | Use `require_fixing_value()` in `projected_overnight_rate()`, `fixing_series_id()` in callsite |
| `finstack/valuations/.../pricing/swap_legs.rs` | Use `require_fixing_value_exact()` in `pv_floating_leg()` |
| `finstack/valuations/.../irs/metrics/par_rate.rs` | Use `get_fixing_series()` |
| `finstack/valuations/.../irs/metrics/pv_float.rs` | Use `get_fixing_series()` |
| `finstack/valuations/.../basis_swap/types.rs` | Use `get_fixing_series()` |
| `finstack/valuations/.../cap_floor/types.rs` | Use `fixing_series_id()` |
| `finstack/valuations/.../revolving_credit/cashflow_engine.rs` | **New fixing support.** Add `valuation_date` + `fixings` to engine, check `reset_d < as_of` |
| `finstack/valuations/.../structured_credit/utils/rate_helpers.rs` | **New fixing support.** Add `as_of` param, use fixings for past dates |
| `finstack/valuations/.../term_loan/pricing/discounting.rs` | **New fixing support.** Post-process cashflows to replace forward projections with fixings |

### How Other Instruments Use This

Any instrument pricer follows the same pattern:

1. At the pricing entry point, resolve fixings: `let fixings = get_fixing_series(context, curve_id).ok();`
2. When encountering a reset/observation date before `as_of`: `let rate = require_fixing_value(fixings, curve_id, date, as_of)?;`

No trait or interface to implement -- just call the functions.
