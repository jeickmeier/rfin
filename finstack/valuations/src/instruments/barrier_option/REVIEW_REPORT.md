# Market Standards Review: Barrier Option

## Executive Summary

The `BarrierOption` implementation provides a solid foundation with support for standard barrier types (Up/Down, In/Out) and dual pricing engines (Analytical and Monte Carlo). The code structure follows the project's patterns with strong typing and trait integration.

However, the implementation **does not meet 100% market standards** due to a critical correctness bug in Monte Carlo pricing for Puts, the absence of Rebate support, and reliance on unstable Finite Difference Greeks for barrier instruments.

## 1. Correctness & Implementation Gaps

### 1.1 Critical: Monte Carlo Put Pricing Bug
**Severity: Critical**
The Monte Carlo pricer (`BarrierOptionMcPricer`) exclusively uses the `BarrierCall` payoff structure for all instruments.
- **Issue**: `BarrierCall` hardcodes the terminal payoff as `(S - K).max(0.0)`.
- **Impact**: `Put` options are incorrectly priced as `Call` options in the Monte Carlo engine.
- **Fix Required**: Implement `BarrierPut` payoff logic and dispatch based on `instrument.option_type`.

### 1.2 Missing Feature: Rebates
**Severity: High**
Standard barrier options often include a rebate mechanism:
- **Knock-Out**: A rebate paid if the barrier is hit (often paid immediately or at expiry).
- **Knock-In**: A rebate paid if the barrier is *not* hit (paid at expiry).
- **Current State**: The `BarrierOption` struct lacks a `rebate` field.
- **Impact**: Cannot price standard commercial barrier contracts that include rebates to cheapen the premium.

### 1.3 Pricing Discrepancy
**Severity: Medium**
- **Analytical (Reiner-Rubinstein)**: Assumes **continuous** monitoring.
- **Monte Carlo**: discrete monitoring (step-based).
- **Adjustment**: The `use_gobet_miri` flag allows MC to approximate continuous monitoring.
- **Gap**: There is no support for **explicitly discrete** monitoring in the Analytical engine (e.g., using Broadie-Glasserman-Kou continuity corrections to price a daily-close barrier using the continuous formula). The current implementation forces the user to choose between "Continuous Formula" or "Discrete MC with optional continuous adjustment".

## 2. Risk Metrics & Greeks

### 2.1 Finite Difference Instability
**Severity: High**
The implementation registers generic Finite Difference (FD) calculators for Delta, Gamma, and Vega.
- **Issue**: Barrier options have discontinuous payoffs. FD Greeks are numerically unstable near the barrier level (Gamma explodes).
- **Market Standard**:
    - Use **Analytical Greeks** (differentiated Reiner-Rubinstein formulas) whenever possible.
    - For MC, use **Likelihood Ratio Method (LRM)** or Malliavin calculus (Vibrational Smoothing) to stabilize Greeks near the barrier.
- **Current State**: `price_with_lrm_greeks_internal` exists in the MC pricer but is not wired up to the standard `MetricRegistry` (which uses `GenericFdDelta`, etc.).

### 2.2 Missing Greeks
- **Vanna/Volga**: Registered as generic FD. Like Gamma, these are unstable near the barrier. Analytical solutions exist and should be preferred.

## 3. Data Model & Conventions

### 3.1 Settlement
- **Gap**: No explicit settlement lag or type (Cash vs Physical). Implied Cash settlement at expiry. Standard for simple barriers, but worth explicit definition if physical delivery is needed.

### 3.2 Barrier Monitoring Window
- **Gap**: Barriers are often only active during specific windows (e.g., "American" barrier vs "Bermudan" barrier monitoring). The current model assumes the barrier is active for the entire life of the option.

## Recommendations

1.  **Fix MC Put Pricing**: Immediate priority. Create `BarrierPut` payoff or generalize `BarrierCall` to `BarrierVanilla`.
2.  **Add Rebates**: Add `rebate: Money` to `BarrierOption` and update pricing formulas (Rubinstein has terms for rebates).
3.  **Switch to Analytical Greeks**: Implement `delta`, `gamma`, `vega` in `BarrierOptionAnalyticalPricer` and register them instead of Generic FD.
4.  **Expose LRM for MC**: Wire `price_with_lrm_greeks` to the metric registry for MC-based Greek requests.
5.  **Discrete Correction**: Add support for Broadie-Glasserman-Kou adjustment in the Analytical pricer to price discretely monitored barriers without full MC.

## Scorecard

| Category | Status | Notes |
|----------|--------|-------|
| **Pricing Correctness (Analytical)** | ✅ Pass | Reiner-Rubinstein implemented correctly for Calls/Puts. |
| **Pricing Correctness (MC)** | ❌ Fail | **Puts priced as Calls.** |
| **Market Features** | ⚠️ Partial | Missing Rebates, Window Barriers. |
| **Risk (Greeks)** | ⚠️ Partial | Unstable FD Greeks used; Analytical Greeks missing. |
| **Code Quality** | ✅ Pass | Clean, idiomatic, strong typing. |

