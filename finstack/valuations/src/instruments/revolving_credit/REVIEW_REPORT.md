# Market Standards Code Review: Revolving Credit Facility

## Overview

This review covers the `finstack/valuations/src/instruments/revolving_credit` module, assessing its compliance with market standards for pricing and risk metrics of Revolving Credit Facilities (RCFs).

## Summary of Findings

The implementation is **sophisticated and well-structured**, featuring:
-   Unified Deterministic and Stochastic cashflow generation.
-   Comprehensive fee structures (Upfront, Commitment, Usage, Facility).
-   Flexible utilization modeling (Deterministic events or Stochastic 3-factor MC).
-   Correct accrual logic (intra-period slicing) for deterministic events.

However, a **critical deviation from market standards** was found in the pricing logic regarding Credit Risk and Recovery, which leads to systematic **underpricing** of the instrument.

### Critical Issues

#### 1. Incomplete Credit Risk Pricing (Missing Recovery Leg)
**Severity: High**
-   **Observation**: The pricer calculates a "Survival Probability" based on a hazard rate $h$, where $h$ is derived from the credit spread $s$ and recovery rate $R$ as $h = s / (1 - R)$. The cashflows are then discounted by $e^{-(r+h)T}$.
-   **Issue**: This formula ($PV = \sum CF \cdot DF \cdot Survival$) represents the value of cashflows *contingent on survival* (i.e., assuming 0% recovery upon default). It **ignores the Recovery Value** (the amount recovered if default occurs).
-   **Impact**: For a standard loan with Spread $s$ and Recovery $R > 0$, this method yields a price significantly **below Par**.
    -   *Example*: A loan paying RiskFree + 200bps, with 40% Recovery.
    -   *Standard Price*: ~100.00% (Par).
    -   *Current Implementation*: ~98.44%. (Discounting effectively at $r + 200/(1-0.4) = r + 333bps$).
-   **Recommendation**:
    -   **Rigorous Fix**: Implement the Recovery Leg: $PV_{recovery} = \sum (RecoveryRate \times Notional \times DF(t) \times ProbDefault(t))$.
    -   **Pragmatic Fix (Standard for Loans)**: If the intention is to match "Spread Discounting" where the spread compensates for expected loss, the discount rate should be $r + s$. This is mathematically equivalent to setting the effective hazard rate $h = s$ and assuming $R=0$ for the PV calculation, or strictly $PV = \text{Cashflows} / (r+s)$.

### Minor Issues

#### 2. Floating Rate Projection (Reset Lag)
**Severity: Low/Medium**
-   **Observation**: The `CashflowEngine` projects floating rates using `project_floating_rate_with_curve` at the `reset_date` (start of accrual period).
-   **Issue**: It ignores the `reset_lag_days` field from `FloatingRateSpec`. Many modern indices (e.g., SOFR Lookback, Term SOFR with lag) require fixing $N$ days prior to the period start.
-   **Recommendation**: Apply the `reset_lag_days` offset to determining the *fixing date* passed to the forward curve.

#### 3. Unused Code
**Severity: Low**
-   **Observation**: `pricer/components.rs` contains a `FeeCalculator` struct that is unused (the engine uses `RevolvingCreditFees` from `types.rs`).
-   **Recommendation**: Remove unused code to avoid confusion.

## Detailed Validation

### Cashflow Engine
-   **Accruals**: Correct. Uses intra-period slicing to accurately accrue interest on changing balances.
-   **Fees**: Correct. Implements Commitment (on undrawn), Usage (on drawn), and Facility (on total) fees correctly with tiering support.
-   **Day Count**: Correct. Applied consistently.

### Monte Carlo / Stochastic
-   **Path Generation**: Standard 3-factor model (OU Utilization, HW1F Rate, CIR/Anchored Spread). Logic appears sound.
-   **Consistency**: Stochastic pricing matches deterministic pricing (when vol=0) due to the unified engine.

### Risk Metrics
-   **DV01/CS01**: Derived via generic bumping. Accurate insofar as the base pricing logic is accurate (but suffers from the underpricing bias mentioned above).

## Conclusion

The `revolving_credit` module is structurally sound but requires a **correction to the pricing formula** to align with market standards for loans with non-zero recovery rates. Without this fix, the model systematically undervalues the credit risk component.

