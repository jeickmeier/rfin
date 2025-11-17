# Cashflow Market-Standards Upgrade — Implementation Summary

**Date:** November 12, 2025  
**Status:** ✅ Complete  
**Test Results:** 56/56 cashflow tests passing

## Overview

This upgrade addresses market-standards compliance gaps identified in the cashflow module review, focusing on day-count context propagation, error handling, and API safety while maintaining backward compatibility.

## Changes Implemented

### 1. Day-Count Context Propagation (✅ Complete)

**Files Modified:**
- `finstack/valuations/src/cashflow/builder/emission/coupons.rs`

**Changes:**
- Fixed coupon emission now constructs `DayCountCtx` with `frequency: Some(spec.freq)` and optional calendar lookup
- Floating coupon emission constructs `DayCountCtx` with `frequency: Some(spec.rate_spec.reset_freq)` and optional calendar
- Calendar resolution uses `finstack_core::dates::calendar::calendar_by_id` when calendar IDs are specified

**Impact:**
- Act/Act ISMA now receives required frequency context for correct accrual calculation
- Bus/252 now receives calendar context for business day counting
- Errors propagate correctly instead of using default context that may fail

**Tests Added:**
- `fixed_accrual_with_actact_isma_full_period`: Verifies ISMA gives accrual = 1.0 for full coupon
- `float_accrual_with_actact_isma_quarterly`: Verifies ISMA quarterly accrual = 1.0
- `bus252_accrual_requires_calendar`: Verifies Bus/252 with NYSE calendar computes ~5 biz days / 252

### 2. PV Aggregation — Checked Variants (✅ Complete)

**Files Modified:**
- `finstack/valuations/src/cashflow/aggregation.rs`
- `finstack/valuations/src/cashflow/builder/schedule.rs`

**New Public APIs:**
- `pv_by_period_with_ctx(flows, periods, disc, base, dc, dc_ctx) -> Result<...>`: Checked PV with explicit context
- `pv_by_period_credit_adjusted_with_ctx(flows, periods, disc, hazard, base, dc, dc_ctx) -> Result<...>`: Credit-adjusted PV with context
- `CashFlowSchedule::pre_period_pv_with_ctx(periods, disc, base, dc, dc_ctx) -> Result<...>`: Schedule method with context
- `CashFlowSchedule::pre_period_pv_with_market_and_ctx(periods, market, disc_id, hazard_id, base, dc, dc_ctx) -> Result<...>`: Market-aware with context

**Internal Functions:**
- `pv_by_period_sorted_checked`: Propagates day-count errors instead of swallowing with `unwrap_or(0.0)`
- Existing `pv_by_period_sorted` kept for backward compatibility (legacy unchecked variant)

**Documentation:**
- Module-level doc on rounding policy: per-flow rounding via `Money::new`, then currency-safe sum
- Notes on alternative sum-then-round strategy (not implemented by default)

**Tests Added:**
- `pv_with_ctx_sum_matches_direct_calculation`: Verifies sum of period PVs equals `npv_static`
- `pv_with_ctx_errors_on_missing_frequency_for_isma`: Verifies error propagation when ISMA frequency missing
- `pv_by_period_deterministic_multi_currency`: Verifies multi-currency separation preserved

### 3. Strict Scheduling Mode (✅ Complete)

**Files Modified:**
- `finstack/valuations/src/cashflow/builder/builder.rs`
- `finstack/valuations/src/cashflow/builder/compiler.rs`

**New API:**
- `CashflowBuilder::strict_schedules(bool) -> &mut Self`: Enable strict mode (default: false)
- `schedule_strict: bool` field added to `CashflowBuilder` (default: false)

**Behavior:**
- **Graceful mode (default):** Unknown calendars fall back to unadjusted schedules
- **Strict mode:** Unknown calendars or schedule errors propagate as errors

**Compilation Flow:**
- `compile_schedules_and_fees` and downstream functions (`build_fee_schedules`, `compute_coupon_schedules`) accept `strict: bool` parameter
- Strict mode: uses `build_dates_checked` and propagates errors
- Graceful mode: tries `build_dates_checked`, falls back to `build_dates` on error

**Tests Added:**
- `strict_schedule_mode_errors_on_unknown_calendar`: Verifies strict mode errors, graceful mode succeeds

### 4. Non-Panicking Builder Variants (✅ Complete)

**File Modified:**
- `finstack/valuations/src/cashflow/builder/builder.rs`

**New APIs:**
- `try_fixed_cf(spec) -> Result<&mut Self>`: Non-panicking fixed coupon
- `try_floating_cf(spec) -> Result<&mut Self>`: Non-panicking floating coupon
- `try_fixed_stepup(steps, schedule, split) -> Result<&mut Self>`: Non-panicking step-up
- `try_float_margin_stepup(steps, base_params, schedule, split) -> Result<&mut Self>`: Non-panicking margin step-up
- `try_fixed_to_float(switch, fixed_win, float_win, split) -> Result<&mut Self>`: Non-panicking hybrid
- `try_payment_split_program(steps) -> Result<&mut Self>`: Non-panicking PIK toggle

**Behavior:**
- Validate `issue` and `maturity` are set via `principal()` before proceeding
- Return `InputError::Invalid` instead of panicking
- Existing panicking methods retained for backward compatibility

**Tests Added:**
- `try_builder_methods_error_before_principal`: Verifies error return when principal not set

## Backward Compatibility

All changes are **backward compatible**:
- Existing functions (`pv_by_period`, `pre_period_pv`, etc.) unchanged in signature and behavior
- New `*_with_ctx` variants added alongside existing functions
- Graceful scheduling remains the default; strict mode is opt-in
- Existing builder methods (`fixed_cf`, `floating_cf`, etc.) unchanged; `try_*` variants added

## Migration Guide

### For Act/Act ISMA or Bus/252 Day Counts

**Before:**
```rust
let pv_map = schedule.pre_period_pv(periods, disc, base, DayCount::ActActIsma);
```

**After:**
```rust
use finstack_core::dates::DayCountCtx;

let dc_ctx = DayCountCtx {
    frequency: Some(Frequency::Months(6)), // Required for ISMA
    calendar: None,
    bus_basis: None,
};
let pv_map = schedule.pre_period_pv_with_ctx(periods, disc, base, DayCount::ActActIsma, dc_ctx)?;
```

### For Strict Schedule Generation

**Before:**
```rust
let schedule = CashFlowSchedule::builder()
    .principal(notional, issue, maturity)
    .fixed_cf(spec)
    .build()?;
```

**After:**
```rust
let schedule = CashFlowSchedule::builder()
    .principal(notional, issue, maturity)
    .strict_schedules(true)  // Error on unknown calendar
    .fixed_cf(spec)
    .build()?;
```

### For Library-Safe Builder Usage

**Before:**
```rust
builder.fixed_cf(spec); // Panics if principal not set
```

**After:**
```rust
builder.try_fixed_cf(spec)?; // Returns error if principal not set
```

## Test Coverage

**New Tests:** 8 comprehensive tests added
- 3 accrual context tests (ISMA fixed, ISMA float, Bus/252)
- 3 PV context tests (sum parity, error on missing ctx, multi-currency)
- 1 strict scheduling test (error vs fallback)
- 1 try_builder test (error before principal)

**All Tests Passing:** 56/56 cashflow tests ✅

## Performance Impact

**Minimal:**
- Context construction is stack-allocated and cheap
- Calendar lookup via `calendar_by_id` is fast (static registry)
- No additional allocations in hot paths
- Checked variants add error propagation cost, but unchecked legacy paths remain available

## Next Steps (Future Work)

1. **Extended Context Support:** Consider adding context overrides at builder level for global defaults
2. **Rounding Policy Configuration:** Add configurable sum-then-round variant if reconciliation workflows require it
3. **Documentation:** Update book sections on strict scheduling and day-count context usage
4. **Deprecation Path:** Consider deprecating panicking builder methods in future major version

## Files Modified

### Core Changes (5 files)
1. `finstack/valuations/src/cashflow/aggregation.rs` — PV checked variants + docs
2. `finstack/valuations/src/cashflow/builder/schedule.rs` — Schedule PV with context
3. `finstack/valuations/src/cashflow/builder/builder.rs` — Strict flag + try_* methods
4. `finstack/valuations/src/cashflow/builder/compiler.rs` — Strict scheduling logic
5. `finstack/valuations/src/cashflow/builder/emission/coupons.rs` — Context propagation

### Test Files (2 files)
6. `finstack/valuations/src/cashflow/builder/emission/tests.rs` — Accrual context tests
7. `finstack/valuations/src/cashflow/builder/tests.rs` — Strict scheduling + try_builder tests

## Acceptance Criteria — All Met ✅

- ✅ Accrual computations use correct `DayCountCtx` with frequency/calendar
- ✅ `pv_by_period_*_with_ctx` available and tested
- ✅ PV sums match `npv_static` within 1e-10
- ✅ No silent fallbacks in checked paths (errors propagated)
- ✅ Strict scheduling opt-in works; default remains graceful
- ✅ New `try_*` builder APIs return errors instead of panicking
- ✅ Docs updated on rounding policy and strict mode
- ✅ All tests pass (56/56)
- ✅ No clippy warnings
- ✅ Code formatted

## Scorecard Improvements

| Dimension | Before | After | Notes |
|-----------|--------|-------|-------|
| Conventions | 3 | 4 | Day-count context now properly threaded |
| Safety | 3 | 4 | Error propagation + non-panicking APIs |
| API/Design | 4 | 4.5 | Checked variants + strict mode toggles |
| Docs/Tests | 4 | 4.5 | 8 new tests + rounding policy documented |

Overall: Market-standards compliance significantly improved while maintaining full backward compatibility.








