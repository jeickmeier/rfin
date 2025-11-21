# Market Standards Review: FX Barrier Option

## Executive Summary
The current implementation of `FxBarrierOption` is **incomplete and violates fundamental market standards** for FX option pricing. It cannot be used for accurate pricing or risk management in its current state.

The most critical failure is the **incorrect handling of foreign interest rates**. The pricing logic assumes the foreign interest rate is either equal to the domestic rate (Analytic) or zero (Monte Carlo), which yields incorrect prices for all currency pairs with interest rate differentials (i.e., effectively all real-world pairs).

## Critical Issues

### 1. Missing Foreign Discount Curve
**Severity: Critical**
*   **Issue**: The `FxBarrierOption` struct lacks a field for the foreign discount curve (e.g., `foreign_discount_curve_id`).
*   **Impact**: It is impossible to determine the foreign risk-free rate ($r_f$), which is a required input for the Garman-Kohlhagen model (the standard for FX options).
*   **Comparison**: The standard `FxOption` implementation correctly includes both `domestic_discount_curve_id` and `foreign_discount_curve_id`.

### 2. Incorrect Interest Rate Logic (Analytic Pricer)
**Severity: Critical**
*   **Location**: `pricer.rs` lines 293-294
    ```rust
    // For FX, q = r_for (foreign rate, simplified to r_dom for now)
    let r_for = r_dom; // Simplified: would fetch from separate curve in production
    ```
*   **Impact**: This simplification assumes Interest Rate Parity holds with zero differential ($r_d = r_f$). This is factually incorrect and results in significant pricing errors (often > 10% of premium).
*   **Standard**: Must use $r_f$ sourced from the foreign discount curve.

### 3. Incorrect Drift in Monte Carlo (MC Pricer)
**Severity: Critical**
*   **Location**: `pricer.rs` line 100
    ```rust
    let q = 0.0; // Foreign rate handled via quanto adjustment if needed
    ```
*   **Impact**: In FX Black-Scholes/Garman-Kohlhagen, the drift of the spot process under the risk-neutral measure is $r_d - r_f$. Setting $q=0$ implies $r_f=0$.
*   **Standard**: The drift term must be $r_d - r_f$.

### 4. Incomplete/Confusing Quanto Logic
**Severity: Major**
*   **Issue**: The struct includes a `correlation` field and the MC pricer attempts a "quanto adjustment" with hardcoded placeholders (`sigma * 0.12`).
*   **Impact**: 
    1.  Standard FX Barrier options do not need "quanto" adjustments unless they are explicitly Quanto Barriers (settled in a third currency or fixed exchange rate).
    2.  The implementation is hardcoded and incorrect even for a Quanto.
*   **Recommendation**: Remove `correlation` and quanto logic if this is a standard FX Barrier. If a Quanto Barrier is intended, it should be a separate instrument or a clearly defined optional feature with proper inputs (foreign volatility, correlation surface).

### 5. Volatility Handling
**Severity: Moderate**
*   **Issue**: Uses a single clamped volatility from the surface at strike/expiry.
*   **Standard**: FX Barrier options are highly sensitive to the volatility smile/skew (Vanna/Volga effects). While a flat vol is "correct" for a pure Black-Scholes implementation, market standard usually involves Vanna-Volga adjustments or Local Volatility models to account for the smile, especially since the barrier might be far from the strike.
*   **Recommendation**: At minimum, ensure the volatility lookup is correct (ATM vs. Strike). Ideally, implement Vanna-Volga adjustments (though this might be out of scope for a "simple" review, it's worth noting).

## Code Quality & Consistency

1.  **ID Consistency**: `FxBarrierOption` uses `fx_spot_id: String` for the spot rate. `FxOption` typically uses the `FxMatrix` implicitly or explicit currency pairs. Using a raw `String` for spot ID is acceptable but less robust than `Currency` pair lookups or typed IDs.
2.  **Attributes**: The `Attributes` struct usage is good.
3.  **Metrics**: Metrics (`Delta`, `Vega`) are implemented using finite differences, which is standard and robust, assuming the underlying pricer is correct. Since the pricer is incorrect, the metrics are currently incorrect.

## Remediation Plan (Completed)

1.  **Update Struct**: Added `foreign_discount_curve_id` to `FxBarrierOption`. **(Done)**
2.  **Update Analytic Pricer**:
    *   Fetch `foreign_discount_curve`. **(Done)**
    *   Calculate $r_f$ from the foreign curve. **(Done)**
    *   Pass correct $r_f$ to `barrier_call_continuous` / `barrier_put_continuous`. **(Done)**
3.  **Update MC Pricer**:
    *   Calculate $r_f$. **(Done)**
    *   Set drift $q = r_f$. **(Done)**
    *   Removed incorrect "quanto adjustment". **(Done)**
4.  **Remove Correlation**: Removed `correlation` field. **(Done)**

The implementation now correctly handles interest rate differentials and foreign discounting, complying with standard Garman-Kohlhagen / Reiner-Rubinstein mechanics.

