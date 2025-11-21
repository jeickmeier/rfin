# Asian Option Market Standards Review

## Overview
The `asian_option` module implements Asian options with support for:
- **Averaging Types:** Arithmetic and Geometric.
- **Pricing Models:**
  - Analytical (Closed-form for Geometric).
  - Semi-Analytical (Turnbull-Wakeman for Arithmetic).
  - Monte Carlo (Control Variate support).
- **Metrics:** Standard Greeks via finite difference (Rho, Vega) and LRM Greeks in MC.

## Critical Findings

### 1. Lack of Seasoning Support (Critical)
The current implementation assumes all options are **forward-starting**. It ignores the possibility that the valuation date (`as_of`) falls within the averaging period.
- **Issue:** The `AsianOption` struct lacks fields to store `past_fixings` or `accumulated_average`.
- **Consequence:** For a seasoned option, the pricer ignores realized fixings, treating the option as if the averaging period starts at `as_of`. This leads to incorrect pricing (wrong effective strike/average).
- **Market Standard:** Instruments must support "seasoned" state by accepting realized fixing history or accumulated values.

### 2. Incorrect Expiry Handling
- **Issue:** If `time_to_maturity <= 0`, the pricers return the intrinsic value based on the *current spot price*.
- **Consequence:** An expired Asian option should settle based on the *final average*, not the spot price at expiry.
- **Market Standard:** Expired options should return the payoff based on the realized average.

### 3. Floating Strike Support (Missing Feature)
- **Issue:** Only Fixed Strike (Average Rate) Asian options are supported.
- **Market Standard:** Full standard often includes Floating Strike (Average Strike) options (`Payoff = Spot - Average`).

### 4. Monte Carlo Efficiency
- **Issue:** The Control Variate implementation in `price_internal` re-simulates paths for the control (Geometric) payoff instead of reusing the path for both payoffs.
- **Consequence:** Inefficient resource usage (double the path generation work).

## Recommendations
1.  **Update `AsianOption` Struct:** Add `past_fixings` (map of Date -> Price) or `current_average` + `fixing_count`.
2.  **Update Pricers:**
    -   **Analytical:** Calculate `adjusted_strike` based on realized fixings for seasoned options.
    -   **Monte Carlo:** Initialize payoff structs with pre-accumulated sums/products.
3.  **Refactor MC Payoffs:** Update `AsianCall` / `AsianPut` to accept initial state (`accumulated_sum`, `count`).
4.  **Fix Expiry Logic:** Use realized average for expired options.

## Remediation Plan
1.  Modify `AsianOption` to include `past_fixings`.
2.  Update `collect_asian_inputs` to handle seasoned options (adjusting $T$, $N$, and effective Strike).
3.  Update `AsianCall`/`AsianPut` payoff models to accept `initial_sum` and `initial_count`.
4.  Update `AsianOptionMcPricer` to pass realized values to payoff models.

