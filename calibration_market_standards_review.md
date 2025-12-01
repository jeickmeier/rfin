# Valuations Calibration – Market-Standards Review

## Executive Summary
- Success flagging is overly optimistic: `CalibrationReport::for_type` always marks success regardless of residual size or solver failures, so pipelines can accept broken curves/surfaces without tolerance checks.
- Discount curve bootstrap uses calendar-day spot shifts and ignores calendars/BDC, diverging from OIS spot conventions (GBP T+0, USD/EUR T+2) and holiday handling; downstream deposits/swaps inherit date drift.
- Forward curve FRA handling hard-codes T-2 reset lags in the builder, ignoring the configurability exposed on the calibrator and producing tenor-mismatched reset dates in GBP/short-lag markets.
- Validation is permissive by default (`allow_negative_rates = true`) and skips monotonicity checks even in positive-rate regimes; combined with `allow_non_monotonic` curve building this can pass increasing DFs/negative forwards without error.
- SABR surface calibration inserts silent fallback vols (0.20) when slice calibration fails and arbitrage checks emit only warnings; surfaces can be labeled “validation passed” while violating calendar/butterfly constraints.
- Wins: good modularization (calibrators per asset class), adaptive scan grids for negative-rate support, and currency-aware rate bounds/convexity defaults.
- Top actions: (1) Make success/no-arb gate on tolerances, (2) wire business-day spot/BDC into discount bootstrap, (3) honor configurable reset lags for FRA pricing, (4) enforce monotonic DF validation when rates are positive, (5) fail SABR/surface builds on arbitrage or calibration fallbacks.

## System Map (Discovered)
- `calibration/mod.rs`: shared solver helpers, penalty constants, module exports.
- `config.rs`: `CalibrationConfig`, `MultiCurveConfig`, `RateBounds`, solver kind, validation flags.
- `report.rs`: `CalibrationReport` creation/metadata.
- `validation.rs`: `ValidationConfig`, `CurveValidator` for discount/forward/hazard/inflation/base-corr, `SurfaceValidator` for vols.
- `quote.rs`: `RatesQuote`, `CreditQuote`, `VolQuote`, `InflationQuote`, index registry helpers.
- Calibrators (`methods/*`):
  - `discount.rs`: OIS discount bootstrap (deposits/OIS swaps), adaptive DF scan, multi-curve separation checks.
  - `forward_curve.rs`: tenor forward bootstrap (FRAs/futures/swaps/basis), convexity adjustments, reset lag/calendar support.
  - `hazard_curve.rs`: CDS bootstrap with recovery consistency and ISDA conventions.
  - `sabr_surface.rs`: SABR slice calibration to build vol surfaces.
  - Others present but not reviewed in depth here: inflation, hull_white, base_correlation, swaption_vol, xccy, convexity helpers.

## Findings by Component

- **Severity:** 🟠 Major  
  **Area:** Safety | API/Design  
  **Location:** `finstack/valuations/src/calibration/report.rs` • `CalibrationReport::for_type` • lines `131–139`  
  **Problem:** `for_type` hardcodes `success = true` and does not compare residuals/RMSE against the configured tolerance or detect penalty placeholders; failed fits are reported as successful.  
  **Why it matters / Market standard:** Calibration workflows must fail fast when max residuals breach tolerance to avoid trading on misfitted curves/surfaces; ISDA-style curve builds gate on bp-level errors.  
  **Recommendation:** Compute `success` and `convergence_reason` from `max_residual` vs `CalibrationConfig::tolerance` (and presence of penalty values); expose a boolean or error when the gate fails.  
  **Test/Benchmark to add:** Simulate a discount curve with one quote returning `PENALTY`, expect `success == false` and convergence_reason explaining the breach. Tolerance 1e-8.  
  **Acceptance criteria:** Report marks failure when `max_residual > tol`, serialization preserves status, and pipeline rejects the run.

- **Severity:** 🟠 Major  
  **Area:** Conventions | Safety  
  **Location:** `finstack/valuations/src/calibration/methods/discount.rs` • `settlement_date` • lines `175–186`  
  **Problem:** Spot/settlement uses raw calendar-day addition (T+x) with no business-day adjustment or calendar; GBP T+0, USD/EUR T+2 holiday logic is ignored, so deposit/swap PVs drift around holidays/EOY.  
  **Why it matters / Market standard:** OIS bootstraps rely on correct spot and accrual starts; missing BDC/calendar handling breaks coupon accruals and par quotes (e.g., Christmas/New Year gaps).  
  **Recommendation:** Require `calendar_id` or default to a currency calendar; apply BDC (e.g., Modified Following) and business-day spot roll when computing settlement/start for deposits and swaps. Fail if calendar missing in strict mode.  
  **Test/Benchmark to add:** USD OIS deposit quoted on 2024-12-23 with TARGET2/NY calendars; expect spot 2024-12-27 and PV match within 1e-6 vs QuantLib reference.  
  **Acceptance criteria:** Deterministic schedule, PV error < 1e-6, holiday spanning weeks handled correctly.

- **Severity:** 🟠 Major  
  **Area:** Conventions | Algorithms  
  **Location:** `finstack/valuations/src/calibration/methods/forward_curve.rs` • `price_instrument` (FRA builder) • lines `729–741`  
  **Problem:** FRA construction hard-codes `.reset_lag(2)` even when the calibrator exposes `reset_lag`; GBP (T-0) or custom lags are ignored, leading to misaligned fixing dates vs the calendar-aware `calculate_fixing_date`.  
  **Why it matters / Market standard:** FRA fixing/settlement timing drives accrual and par rate; mismatched lags cause basis errors and mis-calibrated short-end forwards.  
  **Recommendation:** Pass `self.reset_lag` into the builder and ensure the same business-day adjustment logic used in `calculate_fixing_date` feeds the instrument; add a guard that rejects quotes when calendar is missing but a non-zero reset lag is requested.  
  **Test/Benchmark to add:** GBP 1x4 FRA with `reset_lag=0` and London calendar; verify fixing date = start date, PV≈0 at par within 1e-8.  
  **Acceptance criteria:** Reset lag honored, PV within tolerance for GBP/GBP-holiday cases, no hard-coded 2-day lag.

- **Severity:** 🟠 Major  
  **Area:** Numerical | Safety  
  **Location:** `finstack/valuations/src/calibration/validation.rs` • `DiscountCurve::validate_monotonicity` and defaults • lines `142–154`, `795–811`  
  **Problem:** Default `ValidationConfig` sets `allow_negative_rates = true`, which skips monotonicity checks entirely; combined with `allow_non_monotonic` builds, increasing DFs and negative forwards can pass validation even in positive-rate markets.  
  **Why it matters / Market standard:** Discount curves must be monotone in positive-rate regimes; non-monotone DFs lead to arbitrageable forward rates and unstable bootstraps.  
  **Recommendation:** Auto-detect rate regime (e.g., sign of shortest zero) and enforce monotonicity when rates are non-negative; expose a strict mode defaulting to monotone checks with configurable tolerance.  
  **Test/Benchmark to add:** Construct a curve with DF(2Y) > DF(1Y) under positive rates; expect validation failure unless `allow_negative_rates` is explicitly enabled.  
  **Acceptance criteria:** Non-monotone positive-rate curves are rejected; negative-rate scenarios still supported when explicitly opted in.

- **Severity:** 🟠 Major  
  **Area:** Algorithms | Numerical  
  **Location:** `finstack/valuations/src/calibration/methods/sabr_surface.rs` • `build_vol_grid` • lines `295–315`; `validation.rs` • `SurfaceValidator` • lines `618–718`  
  **Problem:** When SABR inversion fails, vols fall back silently to `0.20`; surface validation only warns on calendar/butterfly arbitrage (returns `Ok`). Reports still mark validation “passed”, masking arbitrage or failed slices.  
  **Why it matters / Market standard:** Vol surfaces used for risk must fail hard on arbitrage or solver failure; silent fallbacks can understate risk, misprice wings, and break forward/variance monotonicity.  
  **Recommendation:** Treat any fallback vol or validation warning as an error unless an explicit “lenient” flag is set; capture failed expiries in the report and fail calibration when any slice cannot meet tolerance.  
  **Test/Benchmark to add:** Inject a smile with known butterfly arbitrage; expect calibration to error. Add a slice with insufficient strikes to trigger fallback and assert failure. Tolerance 1e-4 vols.  
  **Acceptance criteria:** Calibration fails on fallback or arbitrage, report records offending expiry/strike, no silent 0.20 fills.

## Cross-Cutting Gaps & Inconsistencies
- Success/validation flags do not reflect tolerance breaches (reports always “success”, surface checks warn only), so pipelines can accept arbitrage or failed fits.
- Conventions drift: settlement/reset lag configurability exists but is bypassed (discount uses calendar days; forward FRA hard-codes T-2), leading to tenor/time-base inconsistencies across curves.
- Validation regime is lenient by default (`allow_negative_rates = true`), weakening monotonicity/no-arb enforcement even when inputs are positive-rate.

## Quick Wins
- Gate `CalibrationReport::for_type` success on `max_residual <= tolerance` and presence of penalty values; propagate failure to callers.
- Use business-day calendars (default per currency) for `settlement_date`; error when absent in strict mode.
- Respect `reset_lag` in FRA builder; add a test for GBP T-0 and USD T-2.
- Switch `ValidationConfig` default to `allow_negative_rates = false` unless curve inputs imply negative rates; add an opt-in flag.  
- Treat SABR fallback vols/arbitrage warnings as errors unless `lenient_surfaces` is explicitly set.

## Refactor Plan (2–4 weeks)
- Week 1: Tighten reporting/validation gates (report success logic, surface validator hard errors). Owner: Quant platform. Rollback: feature flag `strict_calibration`.
- Week 1–2: Calendar/BDC enforcement for discount bootstrap settlement; add currency calendars and tests. Owner: Rates quant. Rollback: toggle to legacy calendar-days path.
- Week 2: Honor `reset_lag` end-to-end in forward calibration (FRA builder + fixing calculation) and add GBP/EM coverage. Owner: Rates quant. Rollback: keep legacy lag as fallback.
- Week 3: Regime-aware monotonicity defaults (`allow_negative_rates` auto-detect) and stricter curve validation. Owner: Quant platform. Rollback: configuration switch.
- Week 4: SABR surface hard failures on fallback/arb plus slice-level diagnostics and benchmark tests. Owner: Volatility quant. Rollback: lenient mode flag.

## Test & Benchmark Plan
- Golden: OIS deposit/swap bootstrap around holidays (USD/EUR/GBP), FRA reset-lag parity (GBP T-0 vs USD T-2), CDS repricing vs ISDA engine, SABR surface with known arbitrage-free smile.
- Edge: Negative-rate curves (EUR/CHF), sparse SABR slices, extreme forwards near rate bounds, long-tenor forwards with basis swaps requiring pre-existing curves.
- Stress: Wide rate bounds (EM), deep ITM/OTM vol wings, large quote sets (1k+ points) to profile solver performance.
- CI thresholds: curve residuals < 1e-8 for rates/Credit, surface vol errors < 1e-4, monotonicity/no-arb checks must pass, runtime budgets: discount/forward < 50ms per curve on test grid, SABR slice < 10ms.

## Scorecard (0–5)
- Conventions: 2 — Settlement/reset conventions partially hard-coded; calendars optional.  
- Math: 3 — Core solvers/adaptive grids present; SABR fallback hides failures.  
- Algorithms: 3 — Sequential bootstraps solid; validation leniency weakens robustness.  
- Numerical Stability: 3 — Penalty handling exists but can mask failures; monotonicity skipped by default.  
- Performance: 3 — SmallVec/adaptive grids help; no perf benchmarks for calibration paths.  
- Safety: 2 — False-success reporting and silent fallbacks risk bad outputs.  
- API/Design: 3 — Modular calibrators and configs; some knobs ignored (reset_lag) and no strict mode defaults.  
- Docs/Tests: 3 — Good examples/tests, but missing cases for settlement/arb/fallback failure paths.

## Machine-Readable JSON
```json
{
  "findings": [
    {
      "severity": "Major",
      "area": "Safety",
      "location": {"path": "finstack/valuations/src/calibration/report.rs", "symbol": "CalibrationReport::for_type", "lines": "131-139"},
      "problem": "Success flag is always true; no tolerance gate and penalty residuals can be reported as successful calibrations.",
      "recommendation": "Derive success/convergence from max_residual vs CalibrationConfig.tolerance and fail when penalties are present.",
      "tests": {"inputs": "Inject penalty residual into report creation", "tolerance": 1e-8},
      "acceptance": {"deterministic": true, "max_ms": 5}
    },
    {
      "severity": "Major",
      "area": "Conventions",
      "location": {"path": "finstack/valuations/src/calibration/methods/discount.rs", "symbol": "settlement_date", "lines": "175-186"},
      "problem": "Settlement uses calendar-day shifts without business-day/calendar adjustment.",
      "recommendation": "Apply currency/calendar BDC for spot/settlement and fail when calendar is missing in strict mode.",
      "tests": {"inputs": "USD OIS deposit over year-end holidays", "tolerance": 1e-6},
      "acceptance": {"deterministic": true, "max_ms": 10}
    },
    {
      "severity": "Major",
      "area": "Conventions",
      "location": {"path": "finstack/valuations/src/calibration/methods/forward_curve.rs", "symbol": "price_instrument (FRA)", "lines": "729-741"},
      "problem": "FRA builder hard-codes reset_lag=2 ignoring calibrator configuration.",
      "recommendation": "Pass self.reset_lag into the FRA builder and align with business-day calculation.",
      "tests": {"inputs": "GBP 1x4 FRA with reset_lag=0", "tolerance": 1e-8},
      "acceptance": {"deterministic": true, "max_ms": 10}
    },
    {
      "severity": "Major",
      "area": "Numerical",
      "location": {"path": "finstack/valuations/src/calibration/validation.rs", "symbol": "DiscountCurve::validate_monotonicity", "lines": "142-154"},
      "problem": "Monotonicity is skipped by default because allow_negative_rates=true, allowing increasing DFs/negative forwards to pass.",
      "recommendation": "Auto-detect rate regime or default to monotonic enforcement unless explicitly overridden.",
      "tests": {"inputs": "Curve with DF(2Y)>DF(1Y) under positive rates", "tolerance": 1e-8},
      "acceptance": {"deterministic": true, "max_ms": 5}
    },
    {
      "severity": "Major",
      "area": "Algorithms",
      "location": {"path": "finstack/valuations/src/calibration/methods/sabr_surface.rs", "symbol": "build_vol_grid", "lines": "295-315"},
      "problem": "Silent fallback vol (0.20) on SABR failure and surface validation only warns on arbitrage while reports mark success.",
      "recommendation": "Treat inversion/validation warnings as hard errors; record failed slices and abort calibration.",
      "tests": {"inputs": "Arbitrage smile with missing strikes causing SABR failure", "tolerance": 1e-4},
      "acceptance": {"deterministic": true, "max_ms": 20}
    }
  ],
  "scorecard": {
    "Conventions": 2,
    "Math": 3,
    "Algorithms": 3,
    "Numerical": 3,
    "Performance": 3,
    "Safety": 2,
    "API/Design": 3,
    "Docs/Tests": 3
  }
}
```
