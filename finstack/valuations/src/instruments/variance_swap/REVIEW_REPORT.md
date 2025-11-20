# Variance Swap Market Standards Review

## Overview

A detailed market standards review was conducted on the `variance_swap` module. The review focused on correctness of payoff mechanics, replication logic, annualization conventions, and consistency between pricing and risk metrics.

## Findings

### 1. Annualization Factor Inconsistency (Critical)
- **Issue**: The `RealizedVarianceCalculator` metric was using a hardcoded annualization factor of `365.0` for daily observations, while the instrument's `annualization_factor` method (used in pricing) correctly defaulted to `252.0` (standard business days) or used market scalars.
- **Impact**: This caused a discrepancy between the reported `RealizedVariance` metric and the value used inside the NPV calculation.
- **Fix**: Refactored `RealizedVarianceCalculator` to delegate to `VarianceSwap::partial_realized_variance`, ensuring a single source of truth for annualization logic.

### 2. Logic Duplication in Metrics
- **Issue**: `ExpectedVarianceCalculator` and `VegaCalculator` implemented simplified versions of variance estimation, ignoring the sophisticated replication logic (volatility surface integration) present in the main `VarianceSwap` struct.
- **Impact**: Metrics could diverge from the priced value, leading to inconsistent risk explanation.
- **Fix**: Updated metrics to call `VarianceSwap::remaining_forward_variance`, ensuring they benefit from the full replication model and fallbacks.

### 3. Forward Variance Replication
- **Status**: The implementation correctly uses the "log contract" replication method:
  $$ \text{Var} \approx \frac{2}{T} e^{rT} \sum \frac{\Delta K}{K^2} Q(K) - \frac{1}{T} \left(\frac{F}{K_0} - 1\right)^2 $$
- **Market Standard**: Matches standard discrete replication (Demeterfi et al.).
- **Note**: The current implementation assumes spot-starting replication. For forward-starting variance swaps (where `as_of < start_date`), the replication conceptually prices variance from `as_of` to `maturity` rather than `start` to `maturity`. Given the "Simple" scope, this is acceptable but noted for future enhancement.

### 4. Payoff Mechanics
- **Status**: Correct.
  $$ \text{Payoff} = N_{var} \times (\sigma^2_{realized} - K_{var}) $$
- **Notional**: The code uses Variance Notional ($N_{var}$). Users must convert Vega Notional ($N_{vega}$) using $N_{var} = N_{vega} / (2K)$.

## Fixes Applied

1.  **Unified Realized Variance**: `metrics/realized_variance.rs` now calls `VarianceSwap::partial_realized_variance`.
2.  **Unified Expected Variance**: `metrics/expected_variance.rs` now properly blends realized and forward variance using instrument methods.
3.  **Improved Vega**: `metrics/vega.rs` now uses the instrument's forward variance estimate (consistent with pricing) for the current volatility level, rather than a simplified lookup.

## Verification

- **Pricing**: Matches standard replication logic.
- **Metrics**: Now fully consistent with pricing logic.
- **Lints**: Clean.

## Future Improvements

- **Forward Start**: Explicit term structure deduction for forward-starting periods.
- **Dividends**: Use forward curves instead of spot + yield approximation for better equity forward estimation.

