# Market Standards Code Review: Portfolio Framework

**Review Date:** November 21, 2025
**Module:** `finstack/portfolio`
**Reviewer:** AI Assistant

## Executive Summary

The `portfolio` crate establishes a solid foundation for entity-based position management and cross-currency aggregation. The architecture correctly handles the separation of concerns between instrument pricing (native currency) and portfolio reporting (base currency) using explicit `FxMatrix` conversions.

However, a review against **market standards for pricing and risk** reveals significant gaps in metric coverage and functionality. While the structural elements (concurrency, determinism) are production-ready, the financial analytics layer is incomplete compared to standard industry platforms.

**Verdict:** The framework core is sound, but the risk analytics implementation is **Incomplete** for a general-purpose trading system. It currently functions primarily as a PV aggregator rather than a full risk management engine.

## Detailed Findings

### 1. Risk Metrics Completeness
**Status:** ⚠️ **Significant Gaps**

*   **Standard Greeks Missing:** The `standard_portfolio_metrics()` function only requests `Theta`, `Dv01`, and `Cs01`. Market standard portfolios *must* provide at least:
    *   **Equity/FX:** Delta, Gamma, Vega, Rho.
    *   **Rates:** PV01, IR01 (in addition to DV01).
*   **Bucketed Risk:** The system currently lacks support for aggregating bucketed metrics (vectors/maps).
    *   *Issue:* Summing scalar `f64` values works for parallel DV01, but `bucketed_dv01` (key-rate risk) is returned as a collection (e.g., map of tenor -> sensitivity).
    *   *Impact:* Without vector aggregation, granular curve risk management is impossible.
*   **Error Handling:** `value_single_position` silently swallows metric calculation errors ("if metrics fail, just get base value").
    *   *Market Standard:* Risk runs should report "Risk Errors" or partial failures explicitly, rather than silently returning zero risk.

### 2. Cross-Currency Aggregation
**Status:** ✅ **Market Standard**

*   **Methodology:** Prices in native currency, then converts to base using `FxMatrix`.
*   **Implementation:** Correctly handles `FxQuery` and error propagation if rates are missing.
*   **Consistency:** Using `FxMatrix` ensures that portfolio-level FX rates match those used in pricing (e.g., for quanto adjustments).

### 3. Attribution (P&L Explanation)
**Status:** ⚠️ **Implementation Issues**

*   **Methodology:** Supports `Parallel` and `Waterfall` decomposition.
*   **Metrics-Based Fallback:** The `MetricsBased` method (fast approximation using Greeks) is defined but falls back to `Parallel` (slow re-pricing) with a TODO. This defeats the purpose of having a fast intraday attribution mode.
*   **Date Handling:** `attribute_pnl_parallel` is called with `portfolio.as_of` for both T₀ and T₁ parameters in some paths. Accurate attribution requires distinct timestamps for the start and end of the period to capture market moves correctly.

### 4. Cashflow Management
**Status:** ❌ **Missing**

*   **Finding:** There is no facility to aggregate projected cashflows (e.g., a "Cashflow Ladder" report).
*   **Market Standard:** A standard portfolio view includes a report of expected future payments/receipts aggregated by currency and date (or bucket). This is essential for liquidity management.

### 5. Position Scaling
**Status:** ⚠️ **Ambiguous**

*   **Issue:** `value_single_position` scales value by `quantity` regardless of `PositionUnit`.
    *   `scaled_native = value_native * position.quantity`
*   **Risk:** If `PositionUnit::Notional` is used, and the instrument's `price` method returns the total PV of the notional, multiplying by `quantity` (if quantity = notional) results in `PV * Notional` (squared notional).
*   **Recommendation:** Explicitly define whether `Instrument::price()` returns "Unit Price" (per 1.0 notional) or "Trade Price" (for defined notional). Market standard is typically "Unit Price" (e.g., % of par for bonds, or per-share for equity), but "Total PV" is common for bespoke swaps. This contract must be strictly enforced.

## Code Quality & Architecture

*   **Concurrency:** Excellent use of `rayon` for parallel valuation. Separation of serial/parallel paths preserves determinism.
*   **Data Structures:** Usage of `IndexMap` ensures stable iteration order, which is critical for regression testing and deterministic reporting.
*   **Type Safety:** Strong use of newtypes (`EntityId`, `PositionId`) prevents ID swapping errors.

## Recommendations

1.  **Expand Standard Metrics:** Update `standard_portfolio_metrics` to include Delta, Gamma, Vega, and Rho.
2.  **Implement Vector Aggregation:** Add logic to `aggregate_metrics` to handle `IndexMap`/`Vec` metric types (e.g., summing `dv01_2y` across positions).
3.  **Strict Metric Error Handling:** Add a configuration flag (e.g., `strict_risk: bool`) to fail the run or report errors if requested metrics cannot be computed.
4.  **Clarify Unit Scaling:** Document the `Instrument::price` contract. If `price` returns Total PV, `Position` quantity should be 1.0 (or implicit). If `price` returns Unit PV, scaling is correct.
5.  **Fix Attribution Dates:** Ensure `attribute_portfolio_pnl` correctly accepts and propagates distinct `date_t0` and `date_t1`.

## Conclusion

The `portfolio` crate is well-engineered structurally but currently lacks the depth of analytics required for a "market standard" risk system. It requires focused development on **Greeks coverage**, **vector aggregation**, and **cashflow reporting** to meet industry expectations.

**Rating:** ⭐️⭐️⭐️ (Solid Core, Incomplete Features)

