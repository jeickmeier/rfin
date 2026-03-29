# Yield Curve Bootstrapping Extensions Design

**Date:** 2026-03-29
**Status:** Draft
**Approach:** Extend In Place (Approach A)

## Overview

Extend the existing curve calibration framework with five features, ordered by priority:

1. Cross-currency basis curve bootstrap
2. Dual-curve stripping (sequential OIS-first)
3. Nelson-Siegel / Nelson-Siegel-Svensson parametric curves
4. Futures convexity adjustment (HW1F analytical)
5. Turn-of-year effects

Each feature adds to existing infrastructure (new targets, curve types, utility functions) without architectural changes. Features are independently shippable.

## Existing Infrastructure Summary

| Component | Location |
|-----------|----------|
| Calibration framework | `finstack/valuations/src/calibration/` |
| `BootstrapTarget` / `GlobalSolveTarget` traits | `calibration/solver/traits.rs` |
| Sequential bootstrapper (Brent) | `calibration/solver/bootstrap.rs` |
| Global optimizer (LM) | `calibration/solver/global.rs` |
| Calibration schema (`StepParams` enum) | `calibration/api/schema.rs` |
| Interpolation strategies | `finstack/core/src/math/interp/` |
| Term structures (Discount, Forward, Hazard, Inflation, ...) | `finstack/core/src/market_data/term_structures/` |
| Curve storage macro | `finstack/core/src/market_data/context/curve_storage.rs` |
| Market context | `finstack/core/src/market_data/context/` |
| Hull-White 1F | `calibration/hull_white.rs` |
| XCCY swap instrument | `instruments/rates/xccy_swap/` |
| Basis swap instrument | `instruments/rates/basis_swap/` |
| IR futures instrument | `instruments/rates/ir_future/` |

## Feature 1: Cross-Currency Basis Curve Bootstrap

### Purpose

Derive a foreign currency discount curve from a domestic OIS curve, FX spot, and FX forwards or XCCY basis swap quotes. Expose the implied basis spread curve as an analytics byproduct.

### New Types (core crate)

**`BasisSpreadCurve`** in `finstack/core/src/market_data/term_structures/basis_spread_curve.rs`:

- Stores `(time, spread_bps)` knots with an interpolator
- Implements `TermStructure`
- Provides `spread(t) -> f64` (continuously compounded spread in rate units)
- Builder pattern: `BasisSpreadCurve::builder("USD-EUR-BASIS").base_date(...).knots(...).build()`
- Added to `CurveStorage` enum via the `for_each_context_curve!` macro
- Added to `MarketContext` accessors (`get_basis_spread`, `insert_basis_spread`)
- Serde support via `RawBasisSpreadCurve`

### New Calibration Target (valuations crate)

**`XccyBasisTarget`** in `finstack/valuations/src/calibration/targets/xccy_basis.rs`:

- Implements `BootstrapTarget<Quote = RateQuote, Curve = DiscountCurve>`
- Primary output: FX-implied foreign `DiscountCurve`
- Constructor inputs:
  - `domestic_discount_curve: Arc<DiscountCurve>` (pre-calibrated)
  - `fx_spot: f64`
  - `base_date: Date`
  - `currency: Currency`
  - Interpolation/extrapolation config

**Bootstrap logic per quote:**

For FX forwards:
```
DF_foreign(T) = DF_domestic(T) * FX_fwd(T) / FX_spot
```
The solver finds the foreign DF value at each knot that reprices the FX forward.

For XCCY basis swaps: prices the swap using the trial foreign DF curve and the known domestic OIS curve, solving for the foreign DF knot that makes PV = 0.

**Post-calibration basis spread extraction:**

After the foreign discount curve is calibrated, extract basis spreads:
```
spread(T) = z_foreign(T) - z_domestic(T)
```
Build a `BasisSpreadCurve` from these knots and insert into the market context.

### New Schema Variant

```rust
StepParams::XccyBasis(XccyBasisParams)
```

**`XccyBasisParams`** fields:
- `curve_id: CurveId` -- the foreign discount curve being built
- `currency: Currency` -- foreign currency
- `base_date: Date`
- `domestic_discount_id: CurveId` -- pre-calibrated domestic OIS curve
- `fx_spot: f64` -- spot FX rate (domestic per foreign)
- `basis_spread_curve_id: Option<CurveId>` -- optional ID for the byproduct spread curve
- `method: CalibrationMethod` (default: sequential bootstrap)
- `interpolation: InterpStyle`
- `extrapolation: ExtrapolationPolicy`
- `conventions: RatesStepConventions`

### Supported Instruments

- FX forward points (or outright forward rates)
- XCCY basis swaps (reuses existing `XccySwap` instrument)

## Feature 2: Dual-Curve Stripping (Sequential OIS-First)

### Purpose

Calibrate OIS discount curve first, then strip a SOFR tenor forward curve using the pre-calibrated OIS for discounting. This is already largely supported by the sequential `CalibrationPlan`.

### Changes Required

**1. Plan-level dependency validation** in calibration engine (`step_runtime.rs` or engine entry point):

- Before execution, scan all steps and verify that any `discount_curve_id` or `domestic_discount_id` reference points to a step that appears earlier in the plan (or exists in `initial_market`)
- Emit a clear error: `"Step '{id}' references curve '{ref}' which is not produced by any prior step or initial_market"`

**2. Forward curve target verification:**

- Add an integration test confirming that `ForwardCurveTarget` correctly handles tenor mismatch (e.g., 3M SOFR forwards discounted on OIS) via the existing `discount_curve_id` field
- Verify that the forward curve residuals use the OIS curve for discounting and the forward curve for projection

**3. Documentation:**

- Add a canonical two-step calibration plan example to the schema docs showing:
  1. Step 1: `kind: "discount"`, calibrate `USD-OIS` from OIS swaps
  2. Step 2: `kind: "forward"`, calibrate `USD-SOFR-3M` from SOFR swaps, `discount_curve_id: "USD-OIS"`

### No New Types or Schema Variants

Existing infrastructure handles this. Work is validation, testing, and documentation.

## Feature 3: Nelson-Siegel / Nelson-Siegel-Svensson Parametric Curves

### Purpose

Provide parametric curve fitting as an alternative to knot-based bootstrap. Supports two use cases:
1. Direct calibration from instruments (global optimization over 4 or 6 parameters)
2. Fit-to-curve post-processing (fit NS/NSS to an already-bootstrapped curve)

### New Types (core crate)

**`NelsonSiegelModel`** enum in `finstack/core/src/market_data/term_structures/parametric_curve.rs`:

```rust
pub enum NelsonSiegelModel {
    NS {
        beta0: f64,  // long-term rate level
        beta1: f64,  // short-term component
        beta2: f64,  // medium-term hump
        tau: f64,    // decay factor (> 0)
    },
    NSS {
        beta0: f64,
        beta1: f64,
        beta2: f64,
        beta3: f64,  // second hump
        tau1: f64,   // first decay (> 0)
        tau2: f64,   // second decay (> 0, != tau1)
    },
}
```

Zero rate formulas:
```
NS:  z(t) = beta0
           + beta1 * ((1 - exp(-t/tau)) / (t/tau))
           + beta2 * ((1 - exp(-t/tau)) / (t/tau) - exp(-t/tau))

NSS: z(t) = NS terms
           + beta3 * ((1 - exp(-t/tau2)) / (t/tau2) - exp(-t/tau2))
```

Discount factor: `DF(t) = exp(-z(t) * t)`

**`ParametricCurve`** struct:
- Stores: `id: CurveId`, `base_date: Date`, `currency: Currency`, `model: NelsonSiegelModel`, `day_count: DayCount`
- Implements `Discounting` trait: `df(t)` computes via the parametric formula above
- Implements `TermStructure` trait: `id()` returns the curve ID
- Builder pattern: `ParametricCurve::builder("USD-NS").model(...).base_date(...).currency(...).build()`
- Validation at build time: `tau > 0`, `tau1 > 0`, `tau2 > 0`, `tau1 != tau2` for NSS
- Added to `CurveStorage` enum via `for_each_context_curve!` macro
- Added to `MarketContext` accessors (`get_parametric`, `insert_parametric`)
- Serde support via `RawParametricCurve`

**Additional methods on `ParametricCurve`:**
- `zero_rate(t) -> f64` -- the NS/NSS formula
- `forward_rate(t) -> f64` -- analytical derivative (closed-form)
- `params() -> &NelsonSiegelModel` -- access the fitted parameters

### New Calibration Target (valuations crate)

**`ParametricCurveTarget`** in `finstack/valuations/src/calibration/targets/parametric.rs`:

Implements `GlobalSolveTarget<Quote = RateQuote, Curve = ParametricCurve>`:

- `build_time_grid_and_guesses`: returns dummy times `[0.0, 1.0, ..., N-1]` (one per parameter) and initial guesses. Default guesses: `beta0 = 0.03, beta1 = -0.02, beta2 = 0.01, tau = 1.5` for NS; add `beta3 = 0.01, tau2 = 5.0` for NSS.
- `build_curve_from_params`: constructs `ParametricCurve` from the parameter vector
- `calculate_residuals`: prices each instrument against the parametric curve, returns model-vs-market residuals
- `lower_bounds`: `[None, None, None, 0.01]` for NS (tau > 0); `[None, None, None, None, 0.01, 0.01]` for NSS
- `upper_bounds`: `[None, None, None, 30.0]` for NS; `[None, None, None, None, 30.0, 30.0]` for NSS
- Analytical Jacobian: NS/NSS partials w.r.t. each parameter are closed-form. Implement `jacobian()` and return `supports_efficient_jacobian() = true`.

**GlobalSolveTarget reuse note:** The existing trait uses `times` and `params` as parallel vectors. For NS/NSS, the "times" vector carries dummy indices (0, 1, 2, ...) and the solver only uses `params.len()` to determine dimensionality. This is a pragmatic reuse -- the dummy times are internal to the target and not exposed to callers.

### Fit-to-Curve Utility (core crate)

**`fit_nelson_siegel`** in `finstack/core/src/math/fitting.rs`:

```rust
pub fn fit_nelson_siegel(
    zero_rates: &[(f64, f64)],  // (time, zero_rate) sample points
    variant: NsVariant,          // NS or NSS
    initial_guess: Option<NelsonSiegelModel>,
) -> Result<NelsonSiegelModel>
```

- Accepts sampled zero rates (from any curve via `DiscountCurve::zero_rate()`)
- Fits NS or NSS parameters via `LevenbergMarquardtSolver`
- Convenience wrapper: `DiscountCurve::fit_parametric(variant, sample_maturities) -> Result<ParametricCurve>`

### New Schema Variant

```rust
StepParams::Parametric(ParametricCurveParams)
```

**`ParametricCurveParams`** fields:
- `curve_id: CurveId`
- `currency: Currency`
- `base_date: Date`
- `model: NsVariant` -- enum: `"ns"` or `"nss"`
- `initial_params: Option<NelsonSiegelInitialGuess>` -- optional overrides
- `discount_curve_id: Option<CurveId>` -- for multi-curve instrument pricing

## Feature 4: Futures Convexity Adjustment (HW1F Analytical)

### Purpose

Compute model-based convexity adjustments for IR futures using calibrated Hull-White 1F parameters, and apply them during discount curve bootstrapping.

### New Utility Function

**`hw1f_convexity_adjustment`** in `finstack/valuations/src/calibration/hull_white.rs` (alongside existing HW1F code):

```rust
pub fn hw1f_convexity_adjustment(
    kappa: f64,     // mean reversion speed
    sigma: f64,     // short rate volatility
    t_settle: f64,  // futures settlement time
    t_start: f64,   // underlying rate period start
    t_end: f64,     // underlying rate period end
) -> f64
```

Formula:
```
B(t1, t2) = (1 - exp(-kappa * (t2 - t1))) / kappa
convexity = 0.5 * sigma^2 * B(t_settle, t_end) * (B(t_settle, t_end) - B(t_settle, t_start))
```

The `B` function is already implemented in `hull_white.rs`. The convexity adjustment is additive to the forward rate implied from the futures price:
```
adjusted_forward = futures_implied_rate - convexity_adjustment
```

### Integration into Discount Curve Calibration

**`DiscountCurveTargetParams`** -- add optional field:
```rust
pub hw_params: Option<HullWhiteParams>
```

When present and the current quote is a futures instrument, the target applies the HW1F convexity adjustment before computing the residual.

**`DiscountCurveParams` schema** -- add optional field:
```rust
pub hull_white_curve_id: Option<CurveId>
```

If set, the calibration engine looks up the HW1F result from the market context (stored by a prior `HullWhite` calibration step) and passes `(kappa, sigma)` to the target.

### Calibration Plan Pattern

Three-step recipe for convexity-adjusted curves:

1. **Bootstrap preliminary OIS** from deposits + swaps (no futures, or unadjusted futures)
2. **Calibrate HW1F** from swaptions (needs the OIS curve from step 1)
3. **Re-bootstrap OIS** with the full instrument set including futures, now with HW1F convexity adjustment from step 2

### No New Types or Traits

One function, two optional fields on existing structs.

## Feature 5: Turn-of-Year Effects

### Purpose

Account for seasonal funding spikes around year-end in overnight rate curves. TOY spreads are treated as known inputs, not calibrated parameters.

### New Config Types (valuations crate)

**`ToyAdjustment`** and **`ToyWindow`** in `finstack/valuations/src/calibration/config.rs`:

```rust
pub struct ToyAdjustment {
    pub windows: Vec<ToyWindow>,
}

pub struct ToyWindow {
    pub start_month: u8,  // 1-12
    pub start_day: u8,    // 1-31
    pub end_month: u8,    // 1-12
    pub end_day: u8,      // 1-31
    pub spread_bps: f64,  // additive spread in basis points
}
```

Example: `{ start_month: 12, start_day: 15, end_month: 1, end_day: 15, spread_bps: 5.0 }` adds 5bp to overnight rates across the year-end window.

### Integration

**`DiscountCurveParams` schema** -- add optional field:
```rust
pub toy_adjustment: Option<ToyAdjustment>
```

**Calibration behavior:**

During residual calculation in `DiscountCurveTarget`, when a TOY adjustment is configured:
- For any instrument whose accrual period overlaps a TOY window, the target adds the TOY spread to the model forward rate before computing the residual
- The bootstrapper solves for the "clean" curve -- TOY effects are subtracted from market rates, not embedded in the discount factors
- This means the calibrated curve is smooth and TOY-free

**TOY spreads are NOT baked into the output discount factors.** Rationale: embedding jumps in the DF curve creates non-smooth forwards that break derivative sensitivities and interpolation assumptions.

**Utility function** for consumers who need TOY-adjusted forwards:

```rust
pub fn apply_toy_adjustment(
    forward_rate: f64,
    t1: f64,
    t2: f64,
    base_date: Date,
    toy: &ToyAdjustment,
) -> f64
```

Returns the forward rate with TOY spread added if `[t1, t2]` overlaps any TOY window.

### No New Curve Types

One config struct, one optional schema field, one utility function.

## Summary of Changes by Crate

### `finstack/core` (core crate)

| Change | Location |
|--------|----------|
| New `BasisSpreadCurve` type | `market_data/term_structures/basis_spread_curve.rs` |
| New `ParametricCurve` + `NelsonSiegelModel` types | `market_data/term_structures/parametric_curve.rs` |
| Add both to `CurveStorage` enum | `market_data/context/curve_storage.rs` |
| Add both to `MarketContext` accessors | `market_data/context/mod.rs` |
| Serde support for both | `market_data/context/state_serde.rs` |
| `fit_nelson_siegel` utility | `math/fitting.rs` |

### `finstack/valuations` (valuations crate)

| Change | Location |
|--------|----------|
| New `XccyBasisTarget` | `calibration/targets/xccy_basis.rs` |
| New `ParametricCurveTarget` | `calibration/targets/parametric.rs` |
| `hw1f_convexity_adjustment` function | `calibration/hull_white.rs` |
| `ToyAdjustment` / `ToyWindow` config types | `calibration/config.rs` |
| `StepParams::XccyBasis` variant | `calibration/api/schema.rs` |
| `StepParams::Parametric` variant | `calibration/api/schema.rs` |
| Optional `hull_white_curve_id` on `DiscountCurveParams` | `calibration/api/schema.rs` |
| Optional `toy_adjustment` on `DiscountCurveParams` | `calibration/api/schema.rs` |
| Optional `hw_params` on `DiscountCurveTargetParams` | `calibration/targets/discount.rs` |
| Plan-level dependency validation | `calibration/step_runtime.rs` |
| `apply_toy_adjustment` utility | `calibration/targets/util.rs` |

### Tests

| Test | Purpose |
|------|---------|
| XCCY basis bootstrap from FX forwards | Round-trip: FX forwards -> foreign DF -> basis spread |
| XCCY basis bootstrap from XCCY swaps | PV = 0 verification with known domestic OIS |
| Dual-curve two-step plan | OIS first, then SOFR 3M forward, verify discounting correctness |
| NS fit-to-curve round-trip | Bootstrap curve -> fit NS -> compare zero rates within tolerance |
| NSS direct calibration from swaps | Global solve -> verify instrument repricing |
| NS analytical Jacobian vs finite diff | Correctness check for efficient Jacobian |
| HW1F convexity adjustment formula | Known-value test against Andersen-Piterbarg reference |
| Convexity-adjusted bootstrap | Three-step plan with futures, verify adjustment applied |
| TOY adjustment residual effect | Verify instruments spanning year-end use adjusted rates |
| TOY clean curve output | Verify output DF curve is smooth (no jumps at year-end) |
| Plan dependency validation | Verify clear error on misordered steps |
