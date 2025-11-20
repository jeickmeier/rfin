# Term Loan Code Review

## Overview
This document represents a detailed market standards code review of the `term_loan` module located in `finstack/valuations/src/instruments/term_loan`. The review focuses on implementation correctness, market standard compliance, and architectural soundness.

## Summary
The `term_loan` module is a high-quality, comprehensive implementation of institutional term loans. It supports complex features such as Delayed-Draw Term Loans (DDTL), Payment-In-Kind (PIK), and covenant-driven events. The code is well-structured, type-safe, and follows the project's design patterns.

**Overall Status**: ✅ **Market Standard Compliant** (Corrections Applied)

---

## Detailed Findings

### 1. Cash Flow Generation & Features
**Status**: ✅ **Robust**

*   **DDTL (Delayed Draw)**: The implementation correctly models funding legs (draws) as negative notional flows and redemptions/amortization as positive flows.
    *   *Update*: Logic for commitment fees has been refined to correctly distinguish between Term Loan behavior (Limit - Cumulative Draws) and Revolver behavior (Limit - Outstanding).
*   **PIK (Payment-In-Kind)**: The handling of PIK is excellent.
    *   PIK interest is calculated and capitalized (added to outstanding principal).
    *   PIK "flows" are generated with `CFKind::PIK` but are correctly excluded from PV calculations in `pricing.rs`.
*   **Amortization**: Supports Bullet, Linear, and Custom schedules.
*   **Interest**: Correctly handles Fixed and Floating rates with floors, caps, and gearing.

### 2. Pricing & Valuation
**Status**: ✅ **Correct**

*   **Discounting**: Uses deterministic discounting of cash flows.
*   **Curve Handling**: Correctly resolves forward curves for floating rates and discount curves for PV.
    *   *Update*: Added `credit_curve_id` to `TermLoanSpec` to allow separation of discounting (OIS) and credit risk (Hazard/CS01).
*   **PIK Treatment**: PIK flows are excluded from discounting, which is the correct market standard.

### 3. Metrics
**Status**: ✅ **Comprehensive**

*   **Yield Metrics**: Implements YTM, YTC, YTW, and term yields.
*   **Spread Metrics**: Discount Margin (DM) and All-In Rate implemented correctly.
*   **Risk Metrics**: Supports DV01 (Interest Rate Risk) and CS01 (Credit Spread Risk).

### 4. Market Conventions & Logic

#### ✅ DDTL "Undrawn" Calculation
**Correction Applied**: `cashflows.rs` and `all_in_rate.rs` now calculate "Undrawn" amount based on the specific fee base:
- `Undrawn`: Uses `Limit - Cumulative Draws` (Standard Term Loan).
- `CommitmentMinusOutstanding`: Uses `Limit - Outstanding` (Revolver style).

#### ✅ Credit Curve Identity
**Correction Applied**: Added `credit_curve_id` field to `TermLoanSpec` and `TermLoan`. This allows distinct curves for discounting and credit sensitivity (CS01).

#### ✅ Cross-Module Dependency
**Correction Applied**: Removed dependency on `private_markets_fund::metrics::calculate_irr`. `ytm.rs` and `irr_helpers.rs` now use `finstack_core::cashflow::xirr::xirr_with_daycount` directly, ensuring cleaner architecture and using core primitives.

#### ⚠️ Amortization Basis
**Observation**: `AmortizationSpec::PercentPerPeriod` uses `loan.notional_limit.amount() * pct`.
**Recommendation**: Ensure this aligns with the intended "Amortization Rate" definition. If the rate is meant to be "% of Initial Balance", it is correct. If meant to be "% of Current Balance" (declining balance), it would need adjustment.

---

## Conclusion
The `term_loan` module is **production-ready** and meets high market standards. The critical issues regarding DDTL logic, credit curve separation, and architectural dependencies have been resolved.

**Grade**: A+
