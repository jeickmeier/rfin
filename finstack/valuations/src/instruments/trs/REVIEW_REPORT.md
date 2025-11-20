# TRS Market Standards Review

## Executive Summary

The Total Return Swap (TRS) implementation provides a solid structural foundation with clear separation of Equity and Fixed Income Index variants. However, the pricing logic relies on simplified heuristic models that deviate from market standards for accurate valuation, particularly for "Total Return" semantics and Fixed Income pricing.

## 1. Implementation Status

### 1.1 Structural Quality
- **✅ Type Safety**: Good use of strong types (`TrsSide`, `InstrumentId`, `Money`).
- **✅ Leg Separation**: Clear distinction between `FinancingLeg` and `TotalReturnLeg`.
- **✅ Variant Handling**: Distinct structs for `EquityTotalReturnSwap` and `FIIndexTotalReturnSwap` avoids "god object" anti-pattern.
- **✅ Metric Integration**: Metrics are correctly registered and dispatched.

### 1.2 Market Standard Gaps

#### A. Equity Total Return Semantics
The current `EquityReturnModel` calculates return as the percentage change in **Forward Price**:
```rust
// current implementation
let fwd_start = initial_level * df_start.recip() * (-div_yield * t_start).exp();
let fwd_end = initial_level * df_end.recip() * (-div_yield * t_end).exp();
// Result is approx (r - q) * dt
```
- **Issue**: This effectively models a **Price Return Swap**, not a **Total Return Swap**.
- **Market Standard**: A Total Return Swap payer pays `(Price_End - Price_Start) + Dividends`. In a risk-neutral framework, the expected value of this leg is equivalent to the risk-free rate ($r$), not ($r-q$).
- **Impact**: The current valuation underprices the Total Return leg by the dividend yield amount (approx $q \times Notional$). The receiver is not compensated for the dividends they are owed.

#### B. Fixed Income Index Pricing
The `FiIndexReturnModel` uses a "Carry + Roll" heuristic:
```rust
// current implementation
let carry_return = self.index_yield * yf;
let roll_return = self.duration * (-0.0001 * yf); // simplified roll-down
```
- **Issue**: This is a P&L attribution approximation, not a PV pricing model. Using physical yield (`index_yield`) for valuation in a risk-neutral context allows for arbitrage (if Yield > RiskFree, NPV > 0 at inception).
- **Market Standard**:
    - **Bond TRS**: Pricing should be based on the Forward Price of the underlying bond(s), projected using the Repo curve or specific bond forward curve.
    - **Index TRS**: Should ideally project the Index Value using a forward model consistent with the funding cost, or explicitly model the basket. If using a generic index, the forward price should be $S_0 e^{(r - c)T}$ where $c$ is the income yield, similar to Equity but with coupons.
- **Impact**: Valuation is highly sensitive to the "Yield" input and does not respect the no-arbitrage principle relative to the financing leg.

#### C. Dividend Handling
- **Missing**: Explicit handling of Dividend Reinvestment vs. Cash Payment.
- **Missing**: Tax treatment (Gross vs. Net dividends). Market standard often involves a percentage of the dividend being passed through (e.g., 100% or 85%).

#### D. Financing Leg
- **Minor**: The current implementation uses `fwd.rate_period(t_start, t_end)`. Market standard usually specifies a Fixing Date (e.g., `t_start - 2 days`) and the rate is fixed for the period. Using the forward rate over the period is mathematically close for OIS, but distinct for Term rates.

## 2. Recommendations

### Critical Fixes (for Accuracy)
1.  **Equity Model**: Update `EquityReturnModel` to reflect "Total Return" semantics.
    -   *Option A (Implicit)*: If it's Total Return, the projected return in risk-neutral measure should be the Risk-Free Rate (plus/minus repo spread).
    -   *Option B (Explicit)*: Add a `dividend_treatment` field. If `TotalReturn`, add `div_yield * dt` to the return.
2.  **FI Model**: Replace the "Yield + Roll" heuristic with a Forward Price based model.
    -   Forward Price $F_T = P_0 / P(0,T) \times \text{Correction}$.
    -   Or simpler: $F_T = P_0 e^{(r - \text{income\_yield})T}$.

### Enhancements (for Completeness)
1.  **Dividend Tax**: Add `dividend_tax_pct` field to allow Net vs Gross returns.
2.  **Resets**: Ensure `initial_level` handling correctly supports resetting notionals if required (though current constant notional is fine for standard bullets).
3.  **Currency**: Add support for Quanto adjustments if Underlying Currency != Settlement Currency.

## 3. Conclusion
The implementation is structurally sound but mathematically "light". It implements a P&L approximation rather than a rigorous derivative pricing model. To be "100% market standard for accurate pricing", the return projection logic must be updated to be arbitrage-free and correctly account for dividend/coupon flows.

