# Market Standards Audit & Implementation Report

**Date:** November 21, 2025
**Module:** `finstack/portfolio`
**Status:** ✅ 100% Compliant

## Executive Summary

Following a detailed market standards review, the `portfolio` crate has been upgraded to fully meet industry requirements for pricing, risk management, and cross-currency attribution. 

This audit confirmed that the initial gaps identified in `simple-market-review.md` were addressed, and **identified three additional critical issues** which have now been fixed.

## 1. Resolution of Initial Findings

| Area | Finding | Status | Resolution |
|------|---------|--------|------------|
| **Risk Metrics** | Missing Greeks/Rates metrics | ✅ Fixed | `standard_portfolio_metrics` now includes Theta, Dv01, Cs01, PV01, IR01, Delta, Gamma, Vega, Rho. |
| **Aggregation** | Bucketed risk not aggregated | ✅ Fixed | `aggregate_metrics` supports composite keys (e.g., `bucketed_dv01::2y`) via `is_summable`. |
| **Error Handling** | Silent failures | ✅ Fixed | Implemented `PortfolioValuationOptions { strict_risk: bool }` to enforce complete risk runs. |
| **Cashflows** | No cashflow ladder | ✅ Fixed | Added `cashflows.rs` with `aggregate_cashflows` and currency-preserving ladders. |
| **Scaling** | Ambiguous price/quantity contract | ✅ Fixed | Documented strict contract: `quantity` scales `Instrument::price()`. |
| **Attribution** | Date handling issues | ✅ Fixed | `attribute_portfolio_pnl` now correctly propagates distinct T₀ and T₁ dates. |

## 2. Additional Findings & Fixes (New)

During the detailed code review, three additional critical deviations from market standards were identified and resolved:

### 2.1. Attribution Quantity Scaling (Critical Bug)
*   **Finding:** The attribution engine calculated P&L per unit but failed to multiply by `position.quantity` when aggregating to the portfolio level. This resulted in massive under-reporting of P&L for positions with quantity != 1.0.
*   **Fix:** 
    *   Added `scale(factor: f64)` method to `PnlAttribution` in the valuations crate.
    *   Updated `attribute_portfolio_pnl` to strictly scale all attribution factors by position quantity before aggregation.

### 2.2. FX Principal Revaluation (Critical Gap)
*   **Finding:** The cross-currency attribution logic only converted the *P&L flow* to base currency. It failed to account for the **FX revaluation of the opening principal** (i.e., the value of the asset at T₀ changing in base currency terms purely due to FX moves).
*   **Fix:**
    *   Updated `attribute_portfolio_pnl` to fetch/calculate `Value_Native(T0)`.
    *   Implemented standard dual-term FX attribution:
        1.  Translation of P&L: `Pnl_Native * (FX_T1 - FX_T0)`
        2.  Revaluation of Principal: `Value_Native(T0) * (FX_T1 - FX_T0)`
    *   Both terms are now captured in `fx_translation_pnl` and `total_pnl`.

### 2.3. Invalid Metric Aggregation (Standard Violation)
*   **Finding:** The `is_summable` logic incorrectly included `duration_mac`, `duration_mod`, and `spread_duration`. Summing duration across positions is mathematically invalid (requires weighted averaging).
*   **Fix:** Removed duration metrics from `SUMMABLE_METRICS` in `metrics.rs`.

## Conclusion

The `finstack/portfolio` crate is now robust and aligns with standard market practices for a multi-currency risk and attribution system. The implementation explicitly handles the complexities of FX revaluation and ensures strict consistency between position-level pricing and portfolio-level reporting.

