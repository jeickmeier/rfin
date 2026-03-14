# Carry Decomposition Design Spec

## Overview

Enrich the existing `CarryDetail` within P&L attribution to decompose carry into four sub-components: coupon income, pull-to-par, roll-down (including slide), and optional funding cost. Implement these as first-class metrics so they are reusable beyond attribution (e.g., future FI RV desk carry screener).

## Approach

Extend the existing `GenericThetaDecomposed` pattern — a composite `MetricCalculator` that computes all components in a single pass (one additional reprice for pull-to-par isolation), then flattens results into individual `MetricId` entries in the measures map. Attribution methods consume the metrics; they don't produce them.

## New MetricId Variants

| MetricId | Key string | Description |
|----------|-----------|-------------|
| `CarryTotal` | `"carry_total"` | Sum: coupon_income + pull_to_par + roll_down - funding_cost |
| `CouponIncome` | `"coupon_income"` | Net cashflows (coupons, interest) received during carry horizon |
| `PullToPar` | `"pull_to_par"` | PV change from discounting at flat yield (convergence to par) |
| `RollDown` | `"roll_down"` | Curve shape benefit from aging along sloped curve (includes slide) |
| `FundingCost` | `"funding_cost"` | Cost of financing: dirty_price x funding_rate x dcf |

Existing `ThetaCarry`, `ThetaRollDown`, `ThetaDecay` remain unchanged for backward compatibility.

## CarryDecompositionCalculator

New file: `finstack/valuations/src/metrics/sensitivities/carry_decomposition.rs`

### Algorithm

1. **Collect cashflows** in `(as_of, as_of + horizon]` via `CashflowProvider` → `CouponIncome`
2. **Reprice at horizon date with flat curve** at instrument's current YTM → PV_flat. `PullToPar = PV_flat - base_PV`
3. **Reprice at horizon date with actual T0 curve** → PV_curved. `total_pv_change = PV_curved - base_PV`
4. **RollDown = total_pv_change - PullToPar** (residual captures pure curve shape benefit)
5. **Funding cost**: if `instrument.funding_curve_id()` is `Some`, look up rate and compute `dirty_price x rate x dcf`
6. **CarryTotal = CouponIncome + PullToPar + RollDown - FundingCost**

All values stored via `context.computed.insert(MetricId::X, value)`.

### Dependencies

Depends on `MetricId::Ytm` for constructing the flat curve used in pull-to-par isolation. For instruments without YTM (swaps, options), pull-to-par = 0 and roll-down = total PV change.

### Lookup Pattern

`CarryComponentLookup(MetricId)` — analogous to existing `ThetaComponentLookup`. Registered for each sub-component, depends on `CarryTotal` to ensure the composite runs first.

## Expanded CarryDetail Struct

```rust
pub struct CarryDetail {
    /// Total carry P&L (sum of all components).
    pub total: Money,

    /// Coupon/interest income received during the period.
    pub coupon_income: Option<Money>,

    /// PV convergence toward par (time effect at flat yield).
    pub pull_to_par: Option<Money>,

    /// Curve shape benefit from aging along a sloped curve (includes slide).
    pub roll_down: Option<Money>,

    /// Cost of financing the position (negative = cost).
    pub funding_cost: Option<Money>,

    /// Legacy theta field — retained for backward compatibility.
    /// Equal to coupon_income + pull_to_par + roll_down (total pre-funding).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theta: Option<Money>,
}
```

All new fields use `skip_serializing_if = "Option::is_none"`. The `scale()` and `explain()` methods on `PnlAttribution` are updated to handle new fields.

## Attribution Integration

Attribution methods are consumers of carry metrics, not producers.

**Parallel & Waterfall**: If the instrument's `ValuationResult` at T0 contains carry metrics (user requested `CarryTotal`), populate `CarryDetail` fields from the measures map. Otherwise, fall back to current behavior (total only, detail fields None). No additional repricing in attribution itself.

**Metrics-based**: If `CouponIncome`, `PullToPar`, `RollDown`, `FundingCost` are in the measures map, populate all fields. If only `Theta` is available, use current behavior.

## Instrument Funding Support

New default method on `Instrument` trait:

```rust
fn funding_curve_id(&self) -> Option<CurveId> {
    None
}
```

Overridden by `Bond` and `Repo` to return their funding curve. Other instruments can opt in later.

`CarryDecompositionCalculator` calls `instrument.funding_curve_id()`:
- `Some(curve_id)` → look up funding curve, extract rate for carry horizon tenor, compute `dirty_price x rate x dcf`
- `None` → `FundingCost` not emitted

## Registration

In `metrics/core/registry.rs`:

```
CarryTotal    → CarryDecompositionCalculator
CouponIncome  → CarryComponentLookup(CouponIncome)
PullToPar     → CarryComponentLookup(PullToPar)
RollDown      → CarryComponentLookup(RollDown)
FundingCost   → CarryComponentLookup(FundingCost)
```

## Modified Files

| File | Change |
|------|--------|
| `metrics/core/ids.rs` | Add 5 new `MetricId` constants, add to `ALL_STANDARD` |
| `metrics/sensitivities/mod.rs` | Add `pub mod carry_decomposition;` |
| `metrics/sensitivities/carry_decomposition.rs` | **New** — calculator + lookup |
| `metrics/core/registry.rs` | Register the 5 new metrics |
| `attribution/types.rs` | Expand `CarryDetail` struct |
| `attribution/parallel.rs` | Read carry metrics into `CarryDetail` when available |
| `attribution/waterfall.rs` | Same |
| `attribution/metrics_based.rs` | Same |
| `instruments/common/traits.rs` | Add `fn funding_curve_id()` default method |
| `instruments/rates/bond.rs` | Override `funding_curve_id()` |

## Testing

### Unit tests (carry_decomposition.rs)

1. Par bond — coupon income offsets pull-to-par, roll-down ≈ 0 on flat curve
2. Premium bond on flat curve — negative pull-to-par
3. Discount bond on steep curve — positive pull-to-par and roll-down
4. Zero coupon bond — coupon_income = 0
5. Funding cost — bond with funding_curve_id, verify formula
6. No funding — no funding_curve_id, FundingCost not emitted
7. Component sum — coupon_income + pull_to_par + roll_down - funding_cost ≈ carry_total

### Integration tests (tests/attribution/)

8. Bond attribution with carry decomposition — CarryDetail fully populated
9. Backward compatibility — without carry metrics, old behavior preserved

No golden file changes expected.
