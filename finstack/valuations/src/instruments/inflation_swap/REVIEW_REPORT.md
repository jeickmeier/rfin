# Zero Coupon Inflation Swap Code Review

## Overview

This review covers the implementation of `InflationSwap` (Zero Coupon Inflation Swap - ZCIS) in `finstack/valuations/src/instruments/inflation_swap`.

## Market Standards Compliance

The implementation correctly models a standard Zero Coupon Inflation Swap:
- **Payoff**: `Notional * (CPI(T) / BaseCPI - 1)` paid at maturity vs Fixed Rate accrued.
- **Inflation Index**: Supports standard indices (CPI-U, HICP, RPI) via `CurveId`.
- **Lag**: Supports standard inflation lags (e.g., 3M, 2M) via `InflationLag`.
- **Day Count**: Flexible day count for fixed leg accrual (`Act/Act`, `30/360`, etc.).
- **Base CPI**: The review identified that explicit `base_cpi` is required for booking existing trades where the reference index level is fixed. This field has been added.

## Pricing & Risk Logic

### 1. Net Present Value (NPV)
- **Fixed Leg**: $N \times ((1+K)^\tau - 1) \times DF(T)$. Correct for ZCIS (compounded fixed amount).
- **Inflation Leg**: $N \times (\frac{I(T)}{I(0)} - 1) \times DF(T)$. Correct.
- **Projection**: Uses `projected_index_ratio` which correctly applies lag to maturity date before querying the curve.

### 2. Risk Metrics
- **Par Rate (Breakeven)**: Solves for $K$ such that NPV=0. Implementation: $(\frac{I(T)}{I(0)})^{1/\tau} - 1$. Correct.
- **Inflation01**: Calculates sensitivity to 1bp change in the zero inflation rate.
  - Formula used: $N \times \text{Ratio} \times DF(T) \times T \times 1bp$.
  - This corresponds to the analytical derivative $\frac{dV}{dz} \approx V_{infl} \times T \times \Delta z$.
  - **Status**: Verified and refactored to ensure consistency with pricing logic.
- **InflationConvexity**: Calculates 2nd derivative via finite difference (numerical bumps). Correct implementation.

## Code Structure & Quality

- **Modularity**: Metrics are well-separated in `metrics/`.
- **Reusability**: Core logic for index projection was centralized in `types.rs` to prevent logic drift between pricing and risk metrics.
- **Safety**: Uses typed identifiers (`CurveId`, `InstrumentId`) and `Money` for currency safety.

## Changes Implemented

1.  **Explicit Base CPI**: Added `base_cpi: Option<f64>` to `InflationSwap` struct. This allows users to override the start date lookup, essential for matching trade tickets exactly.
2.  **Logic Centralization**: Exposed `projected_index_ratio` and `lagged_maturity_date` as `pub(crate)` helpers to be used by metrics.
3.  **Refactoring**: Updated `Inflation01Calculator` to use the centralized logic, eliminating code duplication and ensuring that pricing and risk see the same index projection.

## Conclusion

The implementation is robust, follows market standards for ZCIS, and integrates well with the `finstack` pricing engine. The addition of `base_cpi` ensures it can support both forward-starting and legacy trades accurately.

