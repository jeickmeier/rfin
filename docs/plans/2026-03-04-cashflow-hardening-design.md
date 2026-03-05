# Cashflow Module Hardening Design

**Date:** 2026-03-04
**Scope:** Critical + Major + Moderate recommendations from quant library review
**Module:** `finstack/valuations/src/cashflow/`

## Motivation

A detailed quant library review of the cashflow module identified 11 actionable items across three severity tiers. This design addresses all of them to bring the module to production-grade for running a multi-asset book.

## Changes

### 1. Floating Rate Fallback Policy (Critical)

**Problem:** `emit_float_coupons_on()` silently falls back to spread-only rate when forward curve lookup fails. This produces materially wrong cashflows without any error.

**Design:**

Add `FloatingRateFallback` enum to `specs/coupon.rs`:

```rust
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FloatingRateFallback {
    #[default]
    Error,
    SpreadOnly,
    FixedRate(Decimal),
}
```

Add `#[serde(default)] pub fallback: FloatingRateFallback` to `FloatingRateSpec`.

In `emission/coupons.rs`, replace the `warn!` + `project_fallback_rate()` path with:
- `Error` → return `Err(...)` with descriptive message including curve ID and reset date
- `SpreadOnly` → current behavior (use spread as total rate), emit `warn!`
- `FixedRate(r)` → use `r` as the index rate, emit `info!`

**Files:** `specs/coupon.rs`, `emission/coupons.rs`

### 2. Kahan Summation for PV Aggregation (Major)

**Problem:** Naive f64 accumulation in PV aggregation produces O(N*eps) floating-point drift on large portfolios.

**Design:**

Add `KahanAccumulator` struct to `aggregation.rs`:

```rust
#[derive(Default, Clone, Copy)]
struct KahanAccumulator {
    sum: f64,
    compensation: f64,
}

impl KahanAccumulator {
    fn add(&mut self, value: f64) {
        let y = value - self.compensation;
        let t = self.sum + y;
        self.compensation = (t - self.sum) - y;
        self.sum = t;
    }

    fn total(&self) -> f64 {
        self.sum + self.compensation
    }
}
```

Use in `pv_by_period_generic()` inner loop: accumulate f64 amounts with Kahan, construct `Money` from final total. Also use in `aggregate_by_period_sorted()` and `aggregate_cashflows_precise_checked()`.

**Files:** `aggregation.rs`

### 3. Accrued-on-Default Support (Moderate)

**Problem:** Default events don't include accrued but unpaid interest. ISDA standard for CDS requires accrued-on-default.

**Design:**

Add to `DefaultEvent`:

```rust
pub accrued_on_default: Option<f64>,  // Pre-computed accrued interest amount
```

Add `AccruedOnDefault` variant to `CFKind` (enum is already `#[non_exhaustive]`).

In `emit_default_on()`, when `event.accrued_on_default` is `Some(amt)` and `amt > 0.0`, emit an additional `CashFlow` with `kind: CFKind::AccruedOnDefault` on the default date. The accrued amount is computed by the caller (builder orchestrator) using `accrued_interest_amount()` before calling `emit_default_on()`.

**Files:** `specs/default.rs`, `emission/credit.rs`, `finstack/core/src/cashflow/primitives.rs`

### 4. Overnight Compounding Wiring (Major)

**Problem:** `OvernightCompoundingMethod` is configured on `FloatingRateSpec` but ignored during cashflow emission. `compute_overnight_rate()` exists in `rate_helpers.rs` but isn't called.

**Design:**

In `emit_float_coupons_on()`, after computing the reset date and resolving the forward curve:

1. If `spec.rate_spec.overnight_compounding.is_some()`:
   - Extract daily rates from the forward curve for the accrual period [accrual_start, accrual_end)
   - Call `compute_overnight_rate()` with the appropriate method variant
   - Use the resulting compounded rate as the index rate (before gearing/spread/caps)
2. If the forward curve doesn't support daily rate extraction (no `daily_rates()` method), fall back to `project_floating_rate()` with a `warn!` log noting that overnight compounding was requested but term rate was used.

The forward curve trait already exposes `rate(t)` which can be sampled daily. We compute daily rates by sampling the curve at each business day in the accrual period.

**Files:** `emission/coupons.rs`, `rate_helpers.rs` (minor: ensure public visibility of overnight functions)

### 5. Test Suite Hardening (Critical + Moderate)

**New tests to add:**

#### 5a. Floating rate golden values (Critical)

- SOFR + 200bp, quarterly, Act/360, $1M notional
- Verify each coupon = `$1M * (SOFR_forward + 0.02) * year_fraction`
- Use flat forward curve at 4.5%

#### 5b. Cap/floor tests (Moderate)

- Index floor at 0%, verify negative index clamped
- Index cap at 5%, verify excess clamped
- All-in cap at 7%, verify total rate clamped after spread

#### 5c. Negative rate test (Moderate)

- EUR EURIBOR at -0.40% + 300bp spread
- Verify positive coupon (2.60%)
- Test with floor at 0% → verify coupon = spread only (3.00%)

#### 5d. Bus/252 golden values (Moderate)

- Known date range with Brazilian holidays
- Verify year fraction = business_days / 252

#### 5e. Cross-currency test (Moderate)

- Build USD schedule, verify all flows are USD
- Attempt cross-currency PV aggregation, verify error

#### 5f. CPR validation test (Moderate)

- Verify `cpr_to_smm(-0.05)` returns error
- Verify `cpr_to_smm(0.0)` returns 0.0
- Verify `cpr_to_smm(1.5)` clamps correctly

**Files:** `finstack/valuations/tests/cashflows/` (new test modules)

### 6. Commitment Fee Time-Weighting (Moderate)

**Problem:** Undrawn balance computed at single point in time, not time-weighted average.

**Design:**

Add to `FeeSpec::PeriodicBps`:

```rust
#[serde(default)]
pub accrual_basis: FeeAccrualBasis,

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FeeAccrualBasis {
    #[default]
    PointInTime,
    TimeWeightedAverage,
}
```

In `emit_fees_on()`, when `FeeAccrualBasis::TimeWeightedAverage`, compute the time-weighted average outstanding over the accrual period using the outstanding balance path. This requires passing the outstanding history (or a reference to the schedule's `outstanding_by_date()` result) into `emit_fees_on()`.

**Files:** `specs/fees.rs`, `emission/fees.rs`, `builder.rs` (pass outstanding history)

### 7. WAL Method (Moderate)

**Design:**

Add to `CashFlowSchedule`:

```rust
pub fn weighted_average_life(&self, as_of: Date) -> f64 {
    let mut principal_time_sum = 0.0;
    let mut principal_total = 0.0;
    for cf in &self.flows {
        if matches!(cf.kind, CFKind::Amortization | CFKind::Notional | CFKind::PrePayment)
            && cf.date > as_of
            && cf.amount.amount() > 0.0
        {
            let t = self.day_count.year_fraction(as_of, cf.date, Default::default())
                .unwrap_or(0.0);
            principal_time_sum += cf.amount.amount() * t;
            principal_total += cf.amount.amount();
        }
    }
    if principal_total > 0.0 { principal_time_sum / principal_total } else { 0.0 }
}
```

**Files:** `builder/schedule.rs`

### 8. PIK Metadata (Moderate)

**Problem:** PIK flows lose rate and accrual_factor from parent coupon.

**Design:**

Change `add_pik_flow_if_nonzero()` signature to accept `rate: Option<f64>` and `accrual_factor: f64`, pass them through to the `CashFlow`.

In `emit_fixed_coupons_on()` and `emit_float_coupons_on()`, pass the computed rate and year fraction to `add_pik_flow_if_nonzero()`.

**Files:** `emission/helpers.rs`, `emission/coupons.rs`

### 9. CPR Validation (Moderate)

**Design:**

In `cpr_to_smm()`, add:

```rust
if cpr < 0.0 {
    return Err(InputError::Invalid.into());
}
```

**Files:** `builder/credit_rates.rs`

## Non-Goals

- Inflation-linked coupon emission (strategic gap, separate design needed)
- Make-whole call premium calculation
- Mortgage-style declining balance amortization
- P&L attribution infrastructure
- Phased recovery modeling

## Testing Strategy

Each change includes unit tests co-located with the implementation. The test suite hardening (Section 5) provides integration-level golden value tests. All existing tests must continue to pass (backward compatibility via `#[serde(default)]` on new fields).

## Migration

All new fields use `#[serde(default)]` with safe defaults:
- `FloatingRateFallback` defaults to `Error` (stricter than before — may surface previously-hidden curve lookup failures)
- `FeeAccrualBasis` defaults to `PointInTime` (preserves current behavior)
- `DefaultEvent::accrued_on_default` defaults to `None` (preserves current behavior)
