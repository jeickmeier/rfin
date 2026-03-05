# Calibration Module Improvements — Design Document

**Date:** 2026-03-04
**Scope:** Critical + Major + Moderate recommendations from quant library review
**Module:** `finstack/valuations/src/calibration/` + supporting math/market modules

---

## Overview

This design addresses 10 prioritised findings from a practitioner-level review of the
calibration module. Items span numerical robustness (HW1F solver), missing market
conventions (futures convexity, inflation seasonality), API completeness (HW1F/SVI
wiring), risk infrastructure (vega bumps), and code-quality fixes.

---

## Item 1 — Critical: HW1F Brent Fallback for r* Newton Solver

**File:** `hull_white.rs`

**Problem:** `find_critical_rate()` uses Newton-Raphson to locate the critical rate r*
in Jamshidian decomposition. Newton can diverge for extreme κ/σ combinations or deep
OTM swaptions. No fallback exists.

**Design:**
- After Newton exhausts `max_iter` or detects divergence (step size growing), fall
  back to Brent root-finding.
- Bracket: `[fixed_rate - 500bp, fixed_rate + 500bp]` (generous, covers all
  practical scenarios).
- Use existing `finstack_core::math::solver::brent_solve()`.
- Emit a log warning when fallback is triggered.

**Acceptance:** Existing HW1F tests continue to pass; add edge-case test with extreme
κ=5.0, σ=0.03 that triggers the fallback.

---

## Item 2 — Major: Apply Futures Convexity Adjustment

**File:** `targets/discount.rs` (lines ~675-678)

**Problem:** `RateQuote::Futures` already carries `convexity_adjustment: Option<f64>`
but the bootstrap target converts `price → rate = (100-price)/100` without applying it.

**Design:**

```
forward_rate = futures_rate - convexity_adjustment.unwrap_or(0.0)
```

- If `vol_surface_id` is `Some` but `convexity_adjustment` is `None`, log a warning
  suggesting the user pre-compute the adjustment.
- Do NOT auto-compute from vol surface — that is a separate feature.

**Acceptance:** Unit test with known convexity adjustment verifies the corrected DF.
Existing tests (adjustment=None) must remain unchanged.

---

## Item 3 — Major: Wire HW1F into Plan-Driven API

**Files:** `api/schema.rs`, `step_runtime.rs`, possibly `prepared.rs`

**Problem:** HW1F calibration logic exists but cannot be invoked via the
`CalibrationEnvelope` JSON API.

**Design:**
1. Add to `api/schema.rs`:

   ```rust
   pub struct HullWhiteParams {
       pub curve_id: String,
       pub currency: Currency,
       pub base_date: NaiveDate,
       pub swaption_quotes: Vec<SwaptionQuoteRef>,
       pub initial_kappa: Option<f64>,
       pub initial_sigma: Option<f64>,
       pub solver_config: Option<HullWhiteSolverConfig>,
   }
   ```

2. Add `StepParams::HullWhite(HullWhiteParams)` variant (9th variant).
3. Dispatch in `step_runtime.rs::execute_params()` — call existing
   `calibrate_hull_white()`, store result in `MarketContext`.

**Acceptance:** Round-trip test: JSON envelope → calibration → retrieve HW params.

---

## Item 4 — Major: Wire SVI into Plan-Driven API

**Files:** `api/schema.rs`, `step_runtime.rs`

**Problem:** SVI vol parameterisation exists in `core/math/volatility/svi.rs` but has
no `StepParams` entry.

**Design:**
1. Add `SviSurfaceParams` struct mirroring `VolSurfaceParams` but specifying SVI
   fitting.
2. Add `StepParams::SviSurface(SviSurfaceParams)` variant (10th variant).
3. Dispatch calibrates SVI per-expiry, builds `VolSurface`.

**Acceptance:** JSON envelope → SVI calibration → retrieve vol surface. Verify
against known SVI parameters.

---

## Item 5 — Moderate: Vega Bump Infrastructure

**Files:** `bumps/mod.rs` (re-export), new `bumps/vol.rs`

**Problem:** `BumpRequest` only supports rate bumps (Parallel, Tenors). No vega risk.

**Design:**

```rust
pub enum VolBumpRequest {
    Parallel(f64),                         // flat absolute vol shift
    ByExpiry(Vec<(f64, f64)>),            // (expiry_years, vol_shift)
    ByExpiryStrike(Vec<(f64, f64, f64)>), // (expiry, strike, vol_shift)
}
```

- `bump_vol_surface()` clones surface, applies shifts to implied vols, optionally
  re-fits SABR/SVI.
- Keep `BumpRequest` unchanged for rates.

**Acceptance:** Unit test: parallel 1bp vega bump produces correct shifted surface.

---

## Item 6 — Moderate: Inflation Seasonality

**Files:** `api/schema.rs`, `targets/inflation.rs`

**Problem:** No seasonality support. EUR HICP and GBP RPI exhibit strong monthly
patterns.

**Design:**

```rust
pub struct SeasonalFactors {
    pub monthly_adjustments: [f64; 12],  // Jan=0..Dec=11, sum ≈ 0
}
```

- Add `seasonal_factors: Option<SeasonalFactors>` to `InflationCurveParams`.
- Workflow: deseasonalise CPI observations → fit smooth ZC curve →
  reseasonalise output.

**Acceptance:** Synthetic test with known seasonal pattern round-trips correctly.

---

## Item 7 — Moderate: Fix Synthetic Bump Currency Detection

**File:** `bumps/rates.rs` (lines 128-138)

**Problem:** Currency detected by `id_str.contains("USD")` — fragile, wrong for
non-standard naming.

**Design:**
- Add `currency: Option<Currency>` parameter to `bump_discount_curve_synthetic()`.
- Callers pass currency from calibration step params.
- Fall back to string heuristic only if `None`, with log warning.

**Acceptance:** Test with curve ID "INTERNAL_CORP_01" + explicit currency = USD
produces correct bumped DF bounds.

---

## Item 8 — Moderate: Parameter Bounds in Global LM Solver

**Files:** `solver/traits.rs`, `solver/global.rs`, target implementations

**Problem:** `GlobalFitOptimizer` enforces bounds via penalty surface only. True
box constraints improve convergence.

**Design:**
- Add to `GlobalSolveTarget` trait:

  ```rust
  fn lower_bounds(&self) -> Option<Vec<f64>> { None }
  fn upper_bounds(&self) -> Option<Vec<f64>> { None }
  ```

- In LM iteration, clamp parameter vector to bounds after step computation
  (projected LM).
- Implement bounds for discount (DF > 0), hazard (λ > 0), SABR (α > 0, ρ ∈
  [-1,1], ν > 0).

**Acceptance:** Existing global solve tests pass. New test with tight bounds
verifies clamping.

---

## Item 9 — Moderate: SABR Density Monitoring

**File:** `core/math/volatility/sabr.rs`

**Problem:** Hagan formula can produce negative implied densities (butterfly
arbitrage) for extreme parameters. No diagnostic warning.

**Design:**
- Add `check_density(strikes: &[f64], forward: f64) -> Vec<DensityWarning>`.
- Compute `d²C/dK²` at a strike grid; flag negative values.
- Call after calibration; emit warnings. Do NOT fail calibration.

**Acceptance:** Synthetic test with extreme ν triggers warning.

---

## Item 10 — Moderate: Use hazard_rate(t) in Validation

**File:** `validation/curves.rs`

**Problem:** Validation approximates hazard rates via FD:
`-(ln(S(t+dt))-ln(S(t)))/dt` when `HazardCurve::hazard_rate(t)` already exists
(line 277).

**Design:** Replace FD block with direct `curve.hazard_rate(t)` call. One-line fix.

**Acceptance:** Existing validation tests pass with identical results (modulo FD
approximation error — results should be *better*).

---

## Dependency Graph

```
Independent (parallelisable):
  [1] HW1F Brent fallback
  [2] Futures convexity adjustment
  [5] Vega bump infrastructure
  [6] Inflation seasonality
  [7] Fix currency detection
  [9] SABR density monitoring
  [10] Hazard rate validation fix

Sequential:
  [1] → [3] Wire HW1F into API (needs working HW1F)
  [4] Wire SVI into API (independent)
  [8] Parameter bounds (independent, test with [2])
```

## Risk Notes

- **Item 3/4** (API wiring) may require `MarketContext` extension if no generic
  storage exists for HW params. Investigate during implementation.
- **Item 8** (LM bounds) — projected LM can slow convergence near active bounds.
  Monitor benchmark times.
- **Item 6** (seasonality) — deseasonalisation methodology varies by market. Start
  with additive monthly factors; multiplicative can follow.
