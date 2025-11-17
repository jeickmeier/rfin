# Term Loan Market-Standards Implementation Summary

## Overview

This document summarizes the market-standards-driven improvements implemented for the `TermLoan` instrument module following the comprehensive code review. All changes align the term loan cashflow engine, metrics, and conventions with industry best practices and the shared `CashFlowSchedule` infrastructure.

## Implemented Changes

### 1. Notional & Sign Convention Alignment ✅

**Location:** `cashflows.rs`, `types.rs`

**Changes:**
- Funding legs (draws) now encoded as **negative** `CFKind::Notional` flows (cash out from lender).
- Redemptions encoded as **positive** `CFKind::Notional` flows (cash in to lender).
- `Notional.initial` set to `0.0` for funding-leg modelling (consistent with CashFlowSchedule helpers).
- Amortization and PIK flows remain positive (as per CashFlowSchedule conventions).

**Impact:**
- `outstanding_by_date_including_notional()` now produces correct principal paths.
- Holder-view and internal-view conventions are now clearly separated and documented.

### 2. Holder-View CashflowProvider ✅

**Location:** `types.rs::impl CashflowProvider for TermLoan`

**Changes:**
- `build_schedule` now filters the full `CashFlowSchedule` to return only holder inflows:
  - Coupons (`CFKind::Fixed`, `Stub`, `FloatReset`)
  - Amortization (positive principal repayments)
  - Positive notional redemptions
- Excludes funding legs (negative notional) and PIK capitalization.
- Matches the documented holder-view convention used by `Bond`.

**Impact:**
- Consistent portfolio analytics across bonds and term loans.
- Clear separation between internal cashflow engine and holder-facing schedules.

### 3. Time-Dependent DDTL Outstanding ✅

**Location:** `cashflows.rs`

**Changes:**
- Introduced `PrincipalEvent` struct to track all principal-affecting events (draws, sweeps, amortization, PIK).
- Implemented `compute_outstanding_at(events, target_date, as_of, currency)` helper for time-consistent outstanding calculation.
- Refactored coupon/fee loop to:
  - Compute outstanding at each period start using `compute_outstanding_at`.
  - Respect DDTL draw timing (draws only affect outstanding after their date).
  - Apply `as_of` correctly (historical path up to `as_of`, then projected forward).
- PIK and amortization flows now immediately recorded as principal events for downstream periods.

**Impact:**
- Interest and fees now computed on correct time-weighted outstanding balances.
- DDTL facilities with staged draws produce accurate PVs and all-in rates.
- `as_of` semantics properly separate historical vs projected cashflows.

### 4. Schedule Convention Wiring ✅

**Location:** `cashflows.rs`

**Changes:**
- `ScheduleBuilder` now respects `loan.bdc` and `loan.calendar_id` via `adjust_with_id`.
- `CashFlowSchedule.day_count` now set to `loan.day_count` (was hard-coded to `Act360`).
- Coupon dates properly adjusted for business days and holidays when calendar is specified.

**Impact:**
- Consistent day-count and business-day conventions across schedule, accrual, and discounting.
- Term loans now respect market conventions for payment date adjustments.

### 5. Yield Metrics Refactor ✅

**Location:** `metrics/ytm.rs`, `metrics/ytc.rs`, `metrics/ytw.rs`, `metrics/ytn.rs`

**Changes:**
- **YTM:** Uses holder-view schedule from `build_schedule` (excludes funding legs).
- **YTC/YTW:** Compute outstanding at call/exercise dates using `outstanding_by_date_including_notional()`.
- **YtN:** Same pattern with synthetic horizons (as_of + N years, clamped to maturity).
- All yield metrics now use holder-view cashflows and corrected outstanding paths.
- Added explicit documentation for IRR solver usage and day-count consistency.

**Impact:**
- YTM/YTC/YTW/YtN now produce market-standard yields for callable and amortizing loans.
- Call redemptions correctly based on outstanding principal (not zero or negative balances).

### 6. All-In Rate Improvement ✅

**Location:** `metrics/all_in_rate.rs`

**Changes:**
- Replaced bespoke outstanding tracking with calls to `generate_cashflows` and `outstanding_by_date_including_notional()`.
- Uses time-dependent outstanding for both numerator (cash interest + fees) and denominator (time-weighted outstanding).
- Clearly documented as **cash-cost** all-in rate (excludes PIK from numerator).
- Respects BDC/calendar for coupon date generation.

**Impact:**
- All-in rate now reflects true time-weighted borrower cost for amortizing and DDTL loans.
- Consistent with corrected outstanding path used by other metrics.

### 7. Discount Margin Documentation ✅

**Location:** `metrics/discount_margin.rs`

**Changes:**
- Added explicit **fidelity level** documentation:
  - Moderate-fidelity approximation (simplified outstanding, direct DM addition).
  - Suitable for plain floating loans; may deviate for complex structures.
  - Notes possibility of higher-fidelity implementation using full cashflow re-generation.

**Impact:**
- Users understand the approximation level and know when to exercise caution.
- Sets expectation for future enhancement without breaking existing code.

### 8. Spec Documentation Updates ✅

**Location:** `spec.rs`

**Changes:**
- Updated `TermLoanSpec` example to use correct `RateSpec::Floating(FloatingRateSpec { ... })` syntax.
- Marked `OidEirSpec` and `PikSpec` as **experimental** with clear notes that they're not fully wired.
- Removed references to obsolete `RateSpec::FloatingSpread` variant.

**Impact:**
- Documentation matches actual implementation.
- Users won't be misled by unused or experimental types.

### 9. Golden Tests Added ✅

**Location:** `tests/instruments/term_loan/term_loan_tests.rs`, `tests/instruments.rs`

**Changes:**
- Added `term_loan` module to main test runner (`instruments.rs`).
- Created `term_loan/mod.rs` test module structure.
- Added two new tests:
  - `term_loan_golden_pv_and_metrics`: Validates PV, YTM, DV01 for simple bullet loan.
  - `term_loan_amortizing_outstanding_path`: Verifies outstanding path decreases correctly with amortization.
- Existing tests updated to work with new conventions (all 4 tests pass).

**Impact:**
- Regression protection for term loan PV, yields, and outstanding calculations.
- Validates holder-view convention (all flows positive).
- Confirms outstanding path monotonicity for amortizing loans.

## Conventions Summary

### Sign Conventions

- **Funding legs (draws):** Negative `CFKind::Notional` (cash out from lender)
- **Redemptions:** Positive `CFKind::Notional` (cash in to lender)
- **Amortization:** Positive `CFKind::Amortization` (economically reduces outstanding)
- **PIK:** Positive `CFKind::PIK` (economically increases outstanding)
- **Interest/Fees:** Positive amounts (inflows to lender)

### View Conventions

- **Internal Engine View:** Full `CashFlowSchedule` with all flows including funding legs, PIK, fees.
- **Holder View:** Filtered schedule via `CashflowProvider::build_schedule` with only contractual inflows (coupons, amortization, positive redemptions).

### Outstanding Calculation

- Uses `outstanding_by_date_including_notional()` from `CashFlowSchedule`.
- Time-dependent: respects draw timing, amortization, PIK, and sweeps.
- `as_of`-aware: historical path up to `as_of`, projected forward thereafter.

### Schedule Generation

- Respects `bdc`, `calendar_id`, `day_count`, `pay_freq`, and `stub` from instrument spec.
- Uses `ScheduleBuilder::adjust_with_id` for business-day adjustments.
- `CashFlowSchedule.day_count` matches instrument's day-count convention.

### Yield Metrics

- YTM/YTC/YTW/YtN use holder-view flows + initial price leg.
- Call redemptions based on `outstanding_by_date_including_notional()` at exercise dates.
- All yields solved via same IRR engine with instrument's day-count.

### CS01

- Uses generic `GenericParallelCs01` and `GenericBucketedCs01` calculators.
- Requires `HasCreditCurve` trait (term loans use `discount_curve_id` as credit curve).
- Units: PV change for 1bp parallel or key-rate bump.

## Testing Summary

All 4 term loan tests pass:
- ✅ `term_loan_fixed_with_draws_and_fees`: DDTL with draws, OID, commitment/usage fees
- ✅ `term_loan_pik_toggle_and_cash_sweep`: Covenant-driven PIK and cash sweeps
- ✅ `term_loan_golden_pv_and_metrics`: PV, YTM, DV01 golden test for bullet loan
- ✅ `term_loan_amortizing_outstanding_path`: Outstanding path correctness for amortizing loan

Full lint/test suite:
- ✅ `make lint` passes (no clippy warnings)
- ✅ `cargo test -p finstack-valuations --lib` passes (447 tests)
- ✅ Term loan integration tests pass (4/4)

## Remaining Work (Future Enhancements)

### OID/EIR Amortization
- `OidEirSpec` defined but not wired into cashflow engine.
- Would require EIR solver and amortization leg emission.
- Currently marked as experimental.

### PikSpec
- Defined but not used; PIK behavior controlled via `CouponType` and `PikToggle` in `CovenantSpec`.
- Consider removing or wiring into a more sophisticated PIK capitalization model.

### Higher-Fidelity Discount Margin
- Current implementation is moderate-fidelity (simplified outstanding, direct rate adjustment).
- Consider implementing version that re-runs full `generate_cashflows` with adjusted spread.

### More Comprehensive Golden Tests
- Add tests with floating rates, caps/floors, and complex DDTL scenarios.
- Add tests comparing term loan to equivalent bond for PV/yield parity.
- Add tests for YTC/YTW with multiple call dates and different call prices.

## Acceptance Criteria Met ✅

All acceptance criteria from the market-standards review have been satisfied:

- ✅ Notional semantics aligned with `CashFlowSchedule` conventions
- ✅ Holder-view convention implemented and documented
- ✅ DDTL outstanding is time-dependent and respects draw timing
- ✅ `as_of` handling correct for valuation and cashflow filtering
- ✅ BDC, calendar, and day-count properly wired into schedule generation
- ✅ Yield metrics use corrected outstanding paths and holder-view flows
- ✅ All-in rate uses time-dependent outstanding
- ✅ Discount margin fidelity level documented
- ✅ Spec examples updated to match actual code
- ✅ Experimental types clearly marked
- ✅ Golden tests added and passing
- ✅ Full lint and test suite passes

## API Stability

All changes are **backward compatible** at the type level:
- `TermLoan` struct unchanged
- Spec types unchanged
- Metric registration unchanged

**Behavioral changes:**
- PV may differ slightly due to corrected outstanding timing and sign conventions.
- Yields (YTM/YTC/YTW/YtN) may differ for loans with calls or amortization.
- All-in rate may differ for DDTL or amortizing loans.

Users should re-validate golden values and regression tests after upgrading.

