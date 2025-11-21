# Autocallable Market Standards Review & Remediation

## Overview

This review assesses the `Autocallable` instrument implementation against market standards for pricing accuracy, risk metric coverage, and functional completeness.
**Status:** Critical correctness bugs have been fixed. Feature gaps remain (see below).

## Critical Fixes Implemented

### 1. Path-Dependent Event Logic (Fixed)
- **Issue**: `AutocallablePayoff::on_event` was incorrectly iterating through all observation dates at every step, leading to potential false triggers.
- **Fix**: Logic updated to strictly check `(state.time - obs_date).abs() < epsilon`.
- **Fix**: Time grid construction in `AutocallableMcPricer` updated to explicitly include observation dates, ensuring the simulation visits exact event times.

### 2. Discounting of Early Cashflows (Fixed)
- **Issue**: The engine applied a single maturity-based discount factor to all cashflows. Early redemptions were being discounted from Maturity instead of the Call Date.
- **Fix**: `AutocallablePayoff` now receives a vector of `df_ratios` (DF(Call) / DF(Mat)). It applies this ratio to the payoff value, ensuring that when the engine multiplies by DF(Mat), the net result is $Payoff \times DF(Call)$.

### 3. Time Grid Alignment (Fixed)
- **Issue**: The generic `PathDependentPricer` used a uniform grid which likely missed observation dates.
- **Fix**: Added `price_with_grid` to `PathDependentPricer` and updated `AutocallableMcPricer` to construct a custom grid merging uniform steps with exact observation dates.

## Remaining Market Standard Gaps (Completeness)

### 1. Missing "Phoenix" Coupon Barrier
The module documentation claims "Coupon barrier: Conditional coupons if S > Lower Barrier".
- **Gap**: The `Autocallable` struct lacks a `coupon_barriers` field. It only supports coupons paid upon *autocall*.
- **Standard**: Standard "Phoenix" autocallables pay a coupon if $S > CouponBarrier$ even if not autocalled.
- **Current State**: The implementation is a "Capital-at-Risk Reverse Convertible with Autocall". The documentation regarding "Coupon Barrier" is currently misleading relative to the implementation.

### 2. Missing Memory Coupons
- **Gap**: No support for "Memory" coupons (paying missed previous coupons if a barrier is hit later).
- **Standard**: Very common feature.

### 3. Barrier Monitoring Frequency
- **Observation**: `KnockInPut` final payoff uses `min_spot_observed` (Continuous/American barrier).
- **Standard**:
    - **Single Stock**: Often American (Continuous) or Daily Close.
    - **Index**: Often European (at Maturity only).
- **Current State**: Hardcoded to Continuous. This is acceptable for many single-stock structures but restrictive for indices.

## Recommendations for Future Work

1.  **Add Phoenix Features**: Update `Autocallable` struct to include `coupon_barriers: Vec<f64>` and `memory_coupons: bool`.
2.  **Barrier Observation Type**: Add an enum `BarrierType { Continuous, Daily, European }` to control `min_spot_observed` tracking logic.
3.  **Discrete Dividends**: Switch to discrete dividend modeling for single-name equity underlyings when supported by the engine.

## Conclusion
The implementation is now **mathematically correct** for the supported features (Autocall w/ fixed coupon, Reverse Convertible payoff). The critical bugs affecting pricing accuracy (timing & discounting) have been resolved. Future work should focus on adding standard features like Phoenix coupons and configurable barrier types.
