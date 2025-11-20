# Basis Swap Code Review & Market Standards Analysis

## Overview
Review of the `basis_swap` implementation against market standards for interest rate derivatives.

## Findings

### 1. Struct Definitions (`BasisSwapLeg`)
- **Missing Payment Lag**: Standard OIS and some basis swaps have a payment delay (e.g., 2 business days after period end). The current implementation assumes payment exactly on `period_end`.
- **Missing Reset Lag**: Standard floating legs have a reset/fixing lag (e.g., 2 days before start). While `FloatLegSpec` in common parameters has `reset_lag_days`, `BasisSwapLeg` is missing it.
- **Calendar**: `BasisSwap` has a `calendar_id`, but legs might rely on different calendars in cross-currency scenarios (though this struct implies single currency). For single currency, shared calendar is acceptable.

### 2. Pricing Logic (`pv_float_leg`)
- **Discounting**: Correctly discounts from payment date to valuation date.
- **Forward Rate**: Relies on `fwd.rate_period(t_start, t_end)`. This assumes the curve handles fixing conventions. Explicit fixing date calculation is market standard to determine if the rate is fixed (historical) or projected.
- **Payment Date**: Currently uses `period_end`. Should be `period_end + payment_lag`.
- **Accrued Interest**: Calculates "Dirty PV" (including accrued). This is standard for pricing engines unless "Clean Price" is requested.

### 3. Metrics
- **Par Spread**: Correctly implemented as the spread that zeroes the NPV.
- **PV**: Wraps `pv_float_leg`.

## Recommendations
1.  **Update `BasisSwapLeg`**: Add `payment_lag_days` and `reset_lag_days`.
2.  **Update Pricing Logic**:
    - Apply `payment_lag_days` to determine the actual payment date.
    - Use `reset_lag_days` to compute the fixing date (even if only for reporting or future historical lookup).
3.  **Update Usages**: Fix struct initialization in `calibration` and tests.

## Implementation Plan
The following changes will be applied to bring the code to 100% market standards:
- Modify `finstack/valuations/src/instruments/common/parameters/legs.rs`.
- Update `finstack/valuations/src/instruments/basis_swap/types.rs` to use lags.
- Fix breaking changes in `calibration` and `metrics`.

