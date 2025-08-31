# Enhanced Loan Simulation Methodology

## Overview

This document describes the enhanced valuation methodology implemented for loan instruments with undrawn commitments, specifically `DelayedDrawTermLoan` (DDTL) and `RevolvingCreditFacility` (Revolver).

## Problem with Previous Approach

The original implementation used simplified approximations for valuing future draws:
- Static interest calculation using remaining years × rate
- No proper handling of floating rates or PIK capitalization  
- Missing utilization fee tier modeling
- No comprehensive Expected Exposure calculation

## Enhanced Methodology

### 1. Forward Simulation Model

**Event-Driven Timeline**: Instead of fixed time steps, the simulation builds an event grid containing:
- Expected draw and repayment dates from `ExpectedFundingCurve`
- Interest payment dates based on facility frequency
- Commitment fee payment dates  
- Commitment expiry and final maturity dates

**State Evolution**: The simulation tracks the expected drawn balance `E[B(t)]` over time:
```
E[B(t_i)] = clamp(E[B(t_{i-1})] + Σ(D_k × p_k) - Σ(R_m × q_m), 0, Commitment)
```

### 2. Comprehensive Cash Flow Valuation

#### A. Interest Payments
- **Fixed Rate**: `Interest = Outstanding × Rate × τ` with step-up support
- **Floating Rate**: Uses forward curve with proper reset lag and gearing:
  ```
  Rate = [ForwardRate(t_fix, t_pay) + Spread/10000] × Gearing
  Interest = Outstanding × Rate × τ
  ```
- **PIK Handling**: PIK portion capitalizes into outstanding balance

#### B. Fee Calculations
- **Commitment Fees**: `Fee = UndrawnAvg × FeeRate × τ` (only until commitment expiry)
- **Utilization Fees**: `Fee = DrawnBalance × UtilizationRate(utilization) × τ`
- **Mid-Point Averaging**: Uses `0.5 × (Start + End)` balances for period averages

#### C. Principal Flows  
- Draw events create negative cash flows (funding outflows)
- Repayment events create positive cash flows
- Amortization schedules generate principal redemption flows

### 3. Expected Exposure Metric

**Definition**: `EE(t)` = Expected drawn balance at future date `t`

**Calculation**: Direct output from balance simulation:
```
EE(t) = DrawnBalance(as_of) + Σ_{as_of < t_i ≤ t} (D_i × p_i)
```

**Applications**:
- Credit risk management
- Regulatory capital calculations  
- Portfolio exposure monitoring

### 4. Monte Carlo Enhancement

**Purpose**: Capture non-linear utilization fee tier effects accurately

**Method**: 
- Run 1,000+ simulation paths with stochastic draw/repay events
- Each event occurs with its specified probability (Bernoulli trials)
- Average utilization fees across all paths to get `E[UtilizationFee]`

**Benefits**: 
- Handles step function in fee schedules
- More accurate than expected-path approximation for tiered fees

## Implementation Architecture

### Core Components

1. **`simulation.rs`**: Shared simulation engine
   - `LoanSimulator`: Main simulation coordinator
   - `LoanFacility`: Trait implemented by DDTL and Revolver
   - `SimulationConfig`: Configuration for deterministic vs Monte Carlo

2. **`metrics.rs`**: Loan-specific metrics
   - `ExpectedExposureCalculator`: Standard 1-year EE calculation
   - `ExpectedExposureMCCalculator`: Monte Carlo enhanced EE
   - `UtilizationCalculator`, `CommitmentFeePvCalculator`, etc.

3. **Enhanced Priceable**: Both DDTL and Revolver now use:
   ```rust
   fn value(&self, curves: &CurveSet, as_of: Date) -> Result<Money> {
       let simulator = LoanSimulator::new();
       let result = simulator.simulate(self, curves, as_of)?;
       Ok(result.total_pv)
   }
   ```

### Mathematical Formulation

**Total PV** = PV(Existing Balance) + PV(Future Events)

Where:
```
PV(Future Events) = Σ_i [
    - D_i × p_i × DF(t_i)                    // Draw cash flows
    + R_i × q_i × DF(t_i)                    // Repayment cash flows  
    + PV(Incremental Interest from D_i)      // Interest on new draws
    + PV(Incremental Principal from D_i)     // Principal redemption
    + PV(Fee Changes from D_i)               // Commitment/utilization fee impacts
]
```

## Key Benefits

1. **Accuracy**: Proper forward rate projections and PIK capitalization
2. **Completeness**: All cash flow types modeled consistently  
3. **Flexibility**: Supports complex interest specs (fixed, floating, PIK, toggle)
4. **Risk Management**: Expected Exposure metrics for credit monitoring
5. **Performance**: Deterministic by default, Monte Carlo when needed
6. **Consistency**: Uses same discount functions as other instruments [[memory:7450000]]

## Usage Examples

### DDTL with Expected Draws
```rust
let ddtl = DelayedDrawTermLoan::new(...)
    .with_expected_funding_curve(funding_curve)
    .with_commitment_fee(0.0050);

let result = ddtl.price_with_metrics(&curves, as_of, &[
    MetricId::custom("expected_exposure_1y"),
    MetricId::custom("commitment_fee_pv"),
])?;
```

### Revolver with Utilization Tiers
```rust
let revolver = RevolvingCreditFacility::new(...)
    .with_utilization_fees(tier_schedule)
    .with_expected_funding_curve(seasonal_pattern);

// Use Monte Carlo for tier accuracy
let mc_result = revolver.price_with_metrics(&curves, as_of, &[
    MetricId::custom("expected_exposure_mc_1y"),
])?;
```

## Validation and Testing

- All existing loan tests continue to pass
- New simulation tests verify deterministic behavior
- Example files demonstrate the enhanced methodology
- Comprehensive metric coverage for risk management

This enhancement brings loan valuation to institutional standards while maintaining the project's determinism and currency-safety principles.
