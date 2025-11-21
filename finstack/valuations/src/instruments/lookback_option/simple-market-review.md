# Market Standards Review: Lookback Option

## Overall Assessment
The current implementation provides a foundational framework for Lookback Options but **fails critical market standards regarding "seasoned" options**. By assuming the lookback period starts at the valuation date (`spot_extremum = spot`), the pricer ignores any realized minimum or maximum asset prices observed prior to the valuation date. This renders the model incorrect for any option priced after its inception date ($t > 0$).

Additionally, the implementation relies purely on continuous monitoring formulas (Goldman-Sosin-Gatto), whereas market standard lookback options typically specify discrete monitoring frequencies (e.g., daily, weekly). This leads to pricing biases (typically overvaluing the option).

## Critical Issues (Must Fix)

### 1. Missing Historical Extremum (Seasoning)
**Severity: Critical**
*   **Issue**: The code hardcodes `let spot_extremum = spot;` in `pricer.rs`.
*   **Impact**: This is only correct at $t=0$. For any $t > 0$, the payoff depends on the realized minimum (for Floating Call / Fixed Put) or maximum (for Floating Put / Fixed Call) observed since inception.
*   **Correction**:
    *   Add `observed_min: Option<Money>` and `observed_max: Option<Money>` to the `LookbackOption` struct.
    *   Update the pricer to use:
        *   `current_min = min(observed_min, current_spot)`
        *   `current_max = max(observed_max, current_spot)`
    *   Pass these correct extrema into the closed-form formulas.

### 2. Metrics Availability
**Severity: High**
*   **Issue**: In `metrics/mod.rs`, the `register_lookback_option_metrics` function and module imports are guarded by `#[cfg(feature = "mc")]`.
*   **Impact**: Users employing the analytical pricer (default) in a build without the Monte Carlo feature enabled will have **no Greeks** (Delta, Gamma, Vega, etc.) registered.
*   **Correction**: Remove the `#[cfg(feature = "mc")]` guard from the registration function and the `metrics` module itself. The generic FD metrics and `Rho`/`Vega` implementations do not inherently require Monte Carlo.

## Market Standard Gaps

### 3. Continuous vs. Discrete Monitoring
**Severity: Medium**
*   **Issue**: The analytic pricer uses continuous monitoring formulas. Real-world contracts usually define a fixing schedule (e.g., Daily Close).
*   **Impact**: Continuous monitoring assumes the asset can reach more extreme values than is possible with discrete observations, leading to an **overestimation of the option value** (premium).
*   **Correction**:
    *   Add a `monitoring_frequency` field (e.g., `Continuous`, `Daily`, `Weekly`).
    *   Implement the **Broadie-Glasserman-Kou continuity correction**: Adjust the strike or extremum by a factor $\exp(\pm \beta \sigma \sqrt{dt})$ where $\beta \approx 0.5826$.

## Code Quality & Minor Issues

### 4. Fixed Strike Payoff Convention
*   **Observation**: The code implements Fixed Strike Lookback Call as using `spot_extremum` (Max).
    *   Standard Payoff: $\max(S_{max} - K, 0)$.
    *   Current Code: Checks `OptionType::Call` -> `fixed_strike_lookback_call`.
    *   Ensure the underlying `fixed_strike_lookback_call` implements the standard $\max(S_{max} - K, 0)$ and not just $S_{max} - K$ (though for lookbacks $S_{max} \ge S_T$, so if $S_{max} > K$ it is usually in the money, but technically $S_{max}$ could be $< K$ if the option is deep out of the money and just started).
*   **Recommendation**: Verify `fixed_strike_lookback_call` handles the $S_{max} < K$ case correctly (payoff 0).

### 5. Strike Logic
*   **Issue**: `strike` is `Option<Money>`.
*   **Refinement**: The builder/constructor should enforce that `strike` is `Some` for `FixedStrike` and `None` (or ignored) for `FloatingStrike`. Currently, it panics or errors at pricing time (`expect("Strike should be Some...")`). A validation step in the `build()` or `new()` method is preferred.

## Implementation Plan

1.  **Update `LookbackOption` Struct**: Add `observed_min` and `observed_max` fields.
2.  **Update Pricer**: Use observed values to determine the effective `spot_extremum` passed to the analytic formulas.
3.  **Fix Metrics Config**: Enable metrics registration for non-MC builds.
4.  **Add Continuity Correction** (Optional but recommended): Add `monitoring` field and adjustment factor logic.

