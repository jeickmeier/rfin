# Quanto Option - Market Standards Review

## Overview
This document reviews the `QuantoOption` instrument implementation against market standards for pricing and risk management.

**Status**: ⚠️ **CRITICAL ISSUES FOUND**

The current implementation contains significant deviations from market standards that affect pricing correctness, particularly regarding interest rate handling and drift adjustments.

## Critical Findings

### 1. Incorrect Interest Rate Handling (Major Correctness Issue)
In `pricer.rs`, the `collect_quanto_inputs` function explicitly sets the foreign risk-free rate equal to the domestic risk-free rate:

```rust
// pricer.rs:229
let r_for = r_dom; // Simplified: assume same for now
```

**Impact**:
- **Pricing Error**: This is mathematically incorrect for Quanto options. The defining characteristic of a Quanto option is the drift adjustment which depends on the differential between domestic and foreign rates, as well as the correlation correction.
- **Drift Formula**: The correct risk-neutral drift for the underlying asset in the domestic measure is:
  $$ \mu = r_f - q - \rho \sigma_S \sigma_{FX} $$
  where $r_f$ is the *foreign* risk-free rate. By forcing $r_f = r_d$, the pricer ignores the interest rate differential, leading to incorrect forward prices and option values.

**Recommendation**:
- Add `foreign_discount_curve_id` to the `QuantoOption` struct.
- Retrieve the foreign rate $r_f$ from this curve instead of using $r_d$.

### 2. Monte Carlo Drift Calculation Error
In `pricer.rs` (MC implementation), the drift calculation appears to be erroneous:

```rust
// pricer.rs:113
let adjusted_drift = r_for - q - quanto_adjustment + r_for;
```

**Impact**:
- **Double Counting**: The term `r_for` is added twice.
- **Formula Mismatch**: The standard drift term for the geometric Brownian motion simulation should be:
  $$ \mu_{drift} = r_f - q - \rho \sigma_S \sigma_{FX} $$
  (assuming `quanto_adjustment` calculates $\rho \sigma_S \sigma_{FX}$).
  The current code effectively computes $2 r_f - q - \dots$, which is incorrect.

**Recommendation**:
- Fix the formula to `r_for - q - quanto_adjustment`.

### 3. Hardcoded FX Volatility
The code falls back to a hardcoded value if FX volatility is missing:

```rust
// pricer.rs:100
0.12 // Default FX vol if not provided
```

**Impact**:
- **Model Risk**: Using a magic number (12%) silently introduces significant pricing error if the user forgets to supply an FX vol surface.
- **Standards**: Financial libraries should typically fail or return an error if required market data is missing, rather than assuming a specific volatility level.

**Recommendation**:
- Remove the default value. Return an error if `fx_vol_id` is provided but the surface is missing, or if it is required for the calculation (which it is for Quantos).

### 4. Missing Foreign Rate Sensitivity (Foreign Rho)
Due to the aliasing of $r_f$ and $r_d$ in the pricer, the `RhoCalculator` currently measures the sensitivity of the price to a parallel shift in *both* rates (assuming the same curve is used).

**Impact**:
- **Risk Management**: Users cannot distinguish between sensitivity to domestic rates (Rho) and foreign rates (Foreign Rho / Phi).
- **Hedging**: Proper hedging requires isolating these risks.

**Recommendation**:
- Once separate curves are supported, implement `ForeignRhoCalculator`.

## Minor Issues & Improvements

### 1. Input Validation
- **Correlation**: There is no validation that `correlation` is within $[-1, 1]$. Values outside this range will cause numerical issues or valid but nonsensical results.
- **Time to Maturity**: `t <= 0.0` checks return 0.0 value. Standard behavior should be to check if it's expired (0 value) or if it's today (payoff value).

### 2. Struct Definition
- The `QuantoOption` struct lacks a field for the `foreign_discount_curve_id`. While it has `foreign_currency`, it doesn't store the curve ID required to fetch $r_f$.

## Proposed Remediation Plan

1.  **Update `QuantoOption` Struct**:
    - Add `pub foreign_discount_curve_id: CurveId`.
    - Update `example()` builder to populate this field.

2.  **Fix Pricer Logic**:
    - In `collect_quanto_inputs`, fetch `r_for` using `foreign_discount_curve_id`.
    - Remove the hardcoded `0.12` FX vol default; return error if missing.
    - Correct the MC drift formula to remove the double addition of `r_for`.

3.  **Enhance Metrics**:
    - Add `ForeignRhoCalculator` (or `PhiCalculator`) to measure sensitivity to `foreign_discount_curve_id`.
    - Update `RhoCalculator` to only bump `discount_curve_id` (domestic).

4.  **Validation**:
    - Add checks for `correlation` bounds in the builder or a `validate()` method.

## Market Standard Formulas

**Analytical Price (Call)**:
$$ V = N \cdot e^{-r_d T} [ F^* N(d_1) - K N(d_2) ] $$

Where:
- $F^* = S \cdot e^{(r_f - q - \rho \sigma_S \sigma_{FX})T}$ (Forward price adjusted for quanto drift)
- $d_1 = \frac{\ln(F^*/K) + 0.5 \sigma_S^2 T}{\sigma_S \sqrt{T}}$
- $d_2 = d_1 - \sigma_S \sqrt{T}$

**Note**: The current implementation uses `quanto_call` from `closed_form`. We should ensure that function correctly implements this logic, accepting distinct $r_d$ and $r_f$.

