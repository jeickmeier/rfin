# Market Standards Review: Cliquet Option

## Executive Summary
The `finstack/valuations/src/instruments/cliquet_option` module implements a standard **Additive Cliquet** (or Ratchet) option. The implementation provides a solid foundation using Monte Carlo simulation for pricing, which is appropriate for this path-dependent instrument. However, the current volatility and interest rate modeling—using constant parameters derived from the final maturity—is a significant simplification that falls short of "100% market standards" for accurate pricing. To meet industrial standards, the pricer must account for the term structure of volatility (forward variance) and interest rates.

## Detailed Component Review

### 1. Instrument Definition (`types.rs`)
*   **Structure**: The `CliquetOption` struct correctly captures the schedule (`reset_dates`) and caps (`local_cap`, `global_cap`).
*   **Missing Features**:
    *   **Floors**: The struct lacks a `local_floor` and `global_floor`. Standard cliquet contracts often specify a local floor (e.g., 0% to lock in zeros, or a negative floor) and a global floor (capital guarantee). The current implementation appears to implicitly assume a local floor of 0% (Cliquet Call) or -∞, which limits flexibility.
    *   **Strike Discipline**: The strike is implicitly reset to the spot price at the start of each period ($S_{i-1}$). This is standard for cliquets and correctly implied.
    *   **Aggregation Type**: The code implies an **Additive** payoff (Sum of returns). Multiplicative cliquets are also common; the type should be explicit if both are to be supported.

### 2. Pricing Logic (`pricer.rs`)
*   **Methodology**: usage of `PathDependentPricer` and `GbmProcess` is structurally correct for a Monte Carlo approach.
*   **Volatility Modeling (Critical)**:
    *   *Current*: The pricer extracts a single volatility $\sigma$ from the surface at the final maturity $T$ (`vol_surface.value_clamped(t, initial_spot)`) and applies it to the entire path.
    *   *Defect*: Cliquets are a series of forward-start options. Their value depends on the **forward volatility** between reset dates. Using a flat spot volatility $\sigma(0, T)$ misprices the structure, especially when the volatility term structure is steep (contango/backwardation).
    *   *Standard*: The process should use a **piecewise constant volatility** $\sigma(t)$ derived from the variance term structure: $\sigma_{fwd}^2(t_{i-1}, t_i) \cdot \Delta t = \sigma_{spot}^2(t_i) \cdot t_i - \sigma_{spot}^2(t_{i-1}) \cdot t_{i-1}$. This ensures the model correctly prices the forward-start optionality.
*   **Interest Rate Modeling**:
    *   *Current*: Uses a constant risk-free rate $r$ derived from the zero curve at maturity.
    *   *Standard*: While less critical than volatility, using a deterministic time-dependent rate $r(t)$ (instantaneous forward rate) is more accurate for long-dated structures.

### 3. Risk Metrics (`metrics/`)
*   **Implementation**: The module correctly registers `Delta`, `Gamma`, `Vega`, etc., using generic Finite Difference (FD) calculators. This is robust and ensures consistency.
*   **LRM Implementation**: The `pricer.rs` file contains `price_with_lrm_greeks_internal`, which calculates Likelihood Ratio Method Greeks but currently discards them (`let (est, _greeks) = ...`). Since the registered metrics use FD, this LRM code is dead/redundant. LRM is typically faster and more stable for path-dependent Monte Carlo Greeks and should be fully wired up or removed.

## Recommendations

1.  **Implement Forward Volatility**: Update the `GbmProcess` construction in `pricer.rs` to use a term structure of volatility. Calculate the forward volatility for each reset period from the variance curve. This is the single most important fix for pricing accuracy.
2.  **Add Floors**: Extend `CliquetOption` to include `local_floor` (default 0.0) and `global_floor` (default 0.0) to support capital-guaranteed structures and standard "floor/cap" ratchets.
3.  **Refine Payoff**: Explicitly document whether the payoff is Additive ($\sum R_i$) or Multiplicative ($\Pi (1+R_i) - 1$).
4.  **Cleanup Greeks**: Either wire up the LRM Greeks to the `metrics` module for performance, or remove the unused LRM path to avoid confusion.

## Implementation Status

All recommendations have been implemented:

1.  **Forward Volatility**: Implemented `PiecewiseGbmProcess` and `PiecewiseExactGbm` in `pricer.rs`. The pricer now calculates forward rates and volatilities for each reset period and constructs a time-dependent process.
2.  **Floors**: Added `local_floor` and `global_floor` to `CliquetOption` and `CliquetCallPayoff`. Logic updated to support these limits.
3.  **Payoff**: Logic clarified as Additive in docs. Floors and caps fully supported.
4.  **Greeks**: Removed dead `LRM` code (`price_with_lrm_greeks_internal`). Metrics continue to use robust Finite Difference Greeks (`GenericFdDelta`, etc.) as registered in `metrics/mod.rs`.

## Additional Gap Analysis: Path to 100% Standard

To achieve a truly "100% market standard" implementation suitable for trading desks, the following additional enhancements are required:

1.  **ATM Forward Volatility Lookup**: The current `PiecewiseGbmProcess` construction uses `initial_spot` to look up volatilities from the surface. Market standard requires looking up volatility at the **Forward Price** $F(0, t)$ corresponding to each reset date. This ensures the correct ATM volatility is used, capturing the surface's behavior correctly in the presence of rates and dividends.
    *   *Correction Needed*: Calculate $F_t = S_0 e^{-q t} / P(0,t)$ and use this as the strike for volatility lookup.

2.  **Payoff Flexibility**:
    *   **Multiplicative Payoff**: While Additive is common, Multiplicative (Compound) Cliquets are also standard. The instrument should support a `CliquetPayoffType` enum.
    *   **Coupon**: Some structures pay a fixed coupon plus the cliquet payoff. (Lower priority).

3.  **Model Sophistication** (Out of Scope for this review, but noted):
    *   **Forward Skew**: GBM assumes a flat smile. Ideally, a **Local Volatility** or **Stochastic Volatility** (Heston) model should be used to capture the forward skew/smile dynamics, which cliquets are very sensitive to. However, "Best possible GBM" is the target for this module unless a model switch is requested.

**Action Plan**: Implement (1) Forward Volatility Lookup and (2) Payoff Type flexibility.
