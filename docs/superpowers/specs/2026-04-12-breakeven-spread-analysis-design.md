# Breakeven Parameter Analysis

**Date**: 2026-04-12
**Status**: Draft
**Scope**: Generic breakeven analytic answering "how much can parameter P move before I lose money over horizon H?"

## Problem

Carry decomposition computes the P&L from holding a position under static curves. The natural follow-up question -- how much can a valuation parameter (spread, yield, vol, correlation) move against you before that carry is wiped out -- is currently not a first-class metric. Users must manually divide carry by sensitivity outside the framework.

## Design

### Approach: Override-Driven Single MetricId

One `MetricId::Breakeven` registered universally. The target parameter and solve mode are specified at query time via `BreakevenConfig` on `MetricPricingOverrides`. The calculator reads config from `context.metric_overrides`, resolves the appropriate sensitivity metric, and computes the result.

This follows the existing pattern where `theta_period` on `MetricPricingOverrides` controls carry/theta horizon at query time.

### Core Data Model

```rust
/// Which valuation parameter to solve the breakeven for.
pub enum BreakevenTarget {
    ZSpread,
    Ytm,
    ImpliedVol,
    BaseCorrelation,
    OAS,
}

/// Linear (first-order) or iterative (full-reprice root-find).
pub enum BreakevenMode {
    Linear,
    Iterative,
}

impl Default for BreakevenMode {
    fn default() -> Self {
        Self::Linear
    }
}

/// Configuration for the breakeven calculator.
pub struct BreakevenConfig {
    pub target: BreakevenTarget,
    pub mode: BreakevenMode,
}
```

`BreakevenConfig` is added to `MetricPricingOverrides`:

```rust
pub struct MetricPricingOverrides {
    pub bump_config: BumpConfig,
    pub mc_seed_scenario: Option<String>,
    pub theta_period: Option<String>,
    pub breakeven_config: Option<BreakevenConfig>,  // NEW
}
```

### Sensitivity Mapping

`BreakevenTarget` maps to a sensitivity `MetricId` that the calculator reads from `context.computed`:

| Target            | Sensitivity MetricId  |
|-------------------|-----------------------|
| `ZSpread`         | `Cs01`                |
| `OAS`             | `Cs01`                |
| `Ytm`             | `Dv01`                |
| `ImpliedVol`      | `Vega`                |
| `BaseCorrelation` | `Correlation01`       |

This mapping lives as a method on `BreakevenTarget`:

```rust
impl BreakevenTarget {
    pub fn sensitivity_metric(&self) -> MetricId { ... }
}
```

### Linear Mode

Formula: `breakeven = -(carry_total) / sensitivity`

Where:
- `carry_total` = `CarryTotal` from `context.computed` (coupon income + pull-to-par + roll-down - funding cost over the theta horizon)
- `sensitivity` = the mapped sensitivity metric value from `context.computed`

Sign convention: positive means spreads/yields/vol can widen/rise by that amount before P&L hits zero. Negative means carry is negative and the parameter must move in the position's favor to break even.

Units: same as the sensitivity bump (typically 1bp for CS01/DV01, percentage points for vol).

### Iterative Mode

Uses root-finding to solve for the exact parameter shift that zeroes out total return, accounting for convexity.

**Objective function**: find delta such that `carry_total + PV_change(delta) = 0`

Where `PV_change(delta) = PV(bumped_market, rolled_date) - PV(base_market, rolled_date)`:
- `rolled_date` = horizon date from `theta_period` (same as carry decomposition)
- `bumped_market` = market context with the target parameter shifted by `delta`

**Bump mechanism per target** (reuses the same bump path each sensitivity metric already uses):
- `ZSpread` / `OAS`: uses the same z-spread bump as `BondCs01Calculator` -- adds delta to the z-spread and re-discounts cashflows
- `Ytm`: uses the same parallel rate bump as `Dv01Calculator` -- shifts the discount curve by delta
- `ImpliedVol`: uses the same vol bump as `VegaCalculator` -- parallel-shifts the vol surface by delta
- `BaseCorrelation`: uses the same correlation bump as `Correlation01Calculator` -- shifts correlation by delta

The key principle: the iterative mode bumps the parameter the same way the corresponding sensitivity metric does, just with a variable-size bump instead of a fixed 1bp bump.

**Root-finder**: Brent's method.
- Prerequisite: compute the linear estimate first. If the linear estimate fails (zero sensitivity), return error without entering the root-finder.
- Initial bracket: `[linear_estimate * 0.5, linear_estimate * 2.0]` using the linear approximation as seed. If the linear estimate is negative (parameter must move in your favor), the bracket flips accordingly.
- Tolerance: 1e-8 (sub-basis-point)
- Max iterations: 50
- Failure behavior: return error if no convergence (don't return garbage)

Repricing uses `context.reprice_money()` which handles pricer dispatch, so iterative mode works for any instrument with a registered pricer.

### Dependencies

Static dependency: `CarryTotal` only.

The sensitivity metric (CS01, DV01, Vega, etc.) is NOT declared as a static dependency because `dependencies()` cannot read `MetricContext` to know which target is configured. Instead:

- The caller must request the sensitivity metric alongside `Breakeven` (e.g., `[MetricId::Cs01, MetricId::Breakeven]`)
- If the sensitivity is missing from `context.computed`, `calculate()` returns a clear error: "Breakeven requires {metric} to be computed first"

This matches the `CarryComponentLookup` pattern where sub-components depend on `CarryTotal` being computed first.

### Horizon

Inherited from carry decomposition via `theta_period` on `MetricPricingOverrides`. The breakeven horizon is the same period used for carry accrual. No separate horizon parameter.

Default: `"1D"` (matching carry decomposition default). For the typical use case ("how much can spreads widen over 6 months?"), set `theta_period: "6M"`.

### MetricId and Registration

New constant:

```rust
pub const Breakeven: Self = Self(Cow::Borrowed("breakeven"));
```

Added to the carry/theta `MetricGroup`.

Registered universally in `register_universal_metrics()`:

```rust
registry.register_metric(
    MetricId::Breakeven,
    Arc::new(BreakevenCalculator),
    &[],  // applies to all instruments
);
```

The calculator fails gracefully if the required sensitivity isn't available for a given instrument type (e.g., requesting vol breakeven on a bond with no vol surface returns an error).

### File Location

New file: `valuations/src/metrics/sensitivities/breakeven.rs`

Alongside `carry_decomposition.rs` and `theta.rs` in the sensitivities module.

Contents:
- `BreakevenTarget` enum
- `BreakevenMode` enum
- `BreakevenConfig` struct
- `BreakevenCalculator` struct + `MetricCalculator` impl
- Sensitivity mapping method
- Iterative solve helper (bump + root-find)

### Python Bindings

`BreakevenConfig` exposed via PyO3 in `finstack-py`. Added to `PricingOptions`:

```python
result = bond.price_with_metrics(
    market, as_of,
    metrics=["carry_total", "cs01", "breakeven"],
    options=PricingOptions(
        theta_period="6M",
        breakeven_config=BreakevenConfig(
            target="z_spread",
            mode="linear",
        ),
    ),
)
breakeven_bps = result.measures["breakeven"]
```

`BreakevenTarget` and `BreakevenMode` are string-mapped enums (`"z_spread"`, `"ytm"`, `"implied_vol"`, `"base_correlation"`, `"oas"` / `"linear"`, `"iterative"`).

## Testing

- **Linear mode, bond**: compute breakeven for z_spread at 6M horizon. Verify `breakeven = -(carry_total) / cs01` within floating-point tolerance.
- **Linear mode, sign**: negative carry should produce negative breakeven (parameter must move in your favor).
- **Linear mode, zero sensitivity**: sensitivity with absolute value below 1e-12 should return an error, not infinity/NaN.
- **Iterative mode, bond**: verify convergence and that the result is close to (but not identical to) linear for small carry. For larger carry, iterative should diverge from linear due to convexity.
- **Iterative mode, convergence failure**: instrument with near-zero sensitivity should return error.
- **Missing sensitivity**: requesting `Breakeven` without the required sensitivity metric in the compute list should return a clear error.
- **Missing config**: requesting `Breakeven` without `breakeven_config` set should return a clear error.
- **Multiple instrument types**: verify the universal registration works for bonds, IRS, CDS, options (where applicable).
- **Horizon inheritance**: verify breakeven uses the same `theta_period` as carry decomposition.

## Non-Goals

- Multiple breakeven targets per compute call (caller can loop).
- Breakeven surface (breakeven across multiple horizons in one call).
- Second-order linear approximation (carry / sensitivity + convexity adjustment). The iterative mode covers the convexity case exactly.
