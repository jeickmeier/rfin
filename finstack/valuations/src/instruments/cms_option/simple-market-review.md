# CMS Option - Market Standards Review

## Executive Summary

The `CmsOption` module is currently in a **skeletal state** and is **not functioning**. The pricing logic is explicitly unimplemented (`todo!` / error returning), and the data structures lack critical fields required for market-standard valuation.

**Current Status**: ❌ **Non-Functional / 0% Market Standard**

To achieve market standards, the module requires a complete implementation of a pricing engine (preferably Static Replication for vanilla options) and significant updates to the instrument definition to support standard payment conventions.

## Critical Issues

### 1. Missing Pricing Implementation
**Severity: Critical**

The `pricer.rs` module contains a placeholder `price_internal` function that explicitly returns an error:
`"CMS Option pricing not yet implemented (Hull-White model required)"`

**Market Standard**:
-   **Vanilla CMS Caps/Floors**: typically priced using **Static Replication** (Hagan) over a portfolio of swaptions, or using a closed-form convexity adjustment (e.g., Hagan approximated) on top of a Black/SABR forward rate.
-   **Hull-White**: is generally reserved for path-dependent CMS products (e.g., Bermudan CMS, CMS Spread Options with complex features). For standard CMS Cap/Floors, Hull-White calibration is computationally expensive and often less accurate for matching the vanilla skew than replication.

**Recommendation**:
Implement **Hagan's Static Replication** or **Convexity Adjusted Black** as the primary pricer. This aligns with standard Swaption Volatility Cube inputs.

### 2. Instrument Definition Gaps
**Severity: High**

The `CmsOption` struct lacks several key fields:
-   **Payment Dates**: CMS payments often occur in arrears or with a specific lag. The current struct has `fixing_dates` but no `payment_dates`. Assuming `payment_date = fixing_date` is incorrect for most CMS contracts.
-   **Swap Index Definition**: `cms_tenor` (f64) is insufficient to fully define the underlying swap rate. A CMS rate depends on the conventions of the underlying swap (e.g., 3M vs 6M floating leg, fixed leg frequency, day counts). A `SwapIndexId` or explicit `SwapConventions` struct is needed.
-   **Cap/Floor Strips**: The structure assumes a vector of fixing dates and accrual fractions, which implies a strip of caplets/floorlets. This is good, but without explicit payment dates for each caplet, the discounting will be wrong.

### 3. Market Data Requirements
**Severity: Medium**

The struct allows for an optional `vol_surface_id`.
-   **Requirement**: CMS pricing heavily relies on the **volatility skew/smile**. A flat volatility (single curve) is insufficient because the convexity adjustment is a function of the volatility of the forward swap rate.
-   **Validation**: The pricer must enforce that a valid **Swaption Volatility Cube** (or Surface with Smile) is provided.

## Detailed Component Review

### `types.rs`
-   **Good**: Uses `Attributes`, `PricingOverrides`, and standard `Money` types.
-   **Bad**: `cms_tenor` is a raw `f64`. Should be strictly typed or tied to an index definition.
-   **Missing**: `payment_dates`.

### `pricer.rs`
-   **Status**: Placeholder.
-   **Issue**: Calls for Hull-White, which might be over-engineering for vanilla CMS.

### `metrics/`
-   **Vega**: Implemented using finite difference, which is good, but depends on a working `npv`.
-   **ConvexityAdjustmentRisk**: Placeholder. This is a sophisticated metric that is very relevant for CMS, so its presence (even as a stub) shows good intent.

## Action Plan

To bring this module to 100% market standards:

1.  **Update `CmsOption` Struct**:
    *   Add `payment_dates: Vec<Date>`.
    *   Add `swap_index_id` or similar to define the underlying rate conventions.
2.  **Implement Convexity Adjustment**:
    *   Implement the **Hagan Approximation** for CMS adjustments using the SABR model or a replication integral.
    *   This requires a `SwaptionVolSurface` that supports smile lookups (Strike vs Vol).
3.  **Implement `npv`**:
    *   Loop over each caplet/floorlet.
    *   Calculate Forward Swap Rate ($S_0$).
    *   Calculate Convexity Adjustment ($CA$).
    *   Adjusted Rate $S_{adj} = S_0 + CA$.
    *   Price using Black/Bachelier on $S_{adj}$ with the option strike.
    *   Discount to today.
4.  **Update Tests**:
    *   Add parity tests against a known CMS benchmark or textbook example (e.g., Hull or Brigo & Mercurio).

## Conclusion
The current implementation is a placeholder. It requires significant development to become usable.

