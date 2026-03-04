# Generic Bond Pricer Metrics

**Date:** 2026-03-03
**Status:** Implemented

## Problem

Bond pricers return inconsistent metric sets:

- **Discount pricer**: PV only from `price_dyn`; full metrics via `Instrument::price_with_metrics`
- **Hazard pricer**: PV only; no metrics path
- **OAS pricer**: PV + `oas_bp`; no standard metrics
- **MC pricer**: PV + structural measures + hand-rolled cash-equivalent z-spread/YTM (only 2 of ~15 available metrics)

Additionally, spread metrics must be on a cash-equivalent basis for cross-structure comparison (PIK z-spread solved against PIK cashflows produces misleadingly low numbers).

## Design

### 1. `PricerRegistry::price_with_metrics`

Chains `price_dyn` (model PV) into `build_with_metrics_dyn` (standard metrics pipeline). For non-discounting models, metrics are computed on the instrument's `metrics_equivalent()` so that spread metrics use normalized (cash-equivalent) cashflows.

```rust
pub fn price_with_metrics(&self, instrument, model, market, as_of, metrics, cfg) {
    let base_result = self.price_with_registry(instrument, model, market, as_of, cfg)?;

    let metrics_inst = if model == ModelKey::Discounting {
        instrument.clone_box()
    } else {
        instrument.metrics_equivalent()
    };

    let mut enriched = build_with_metrics_dyn(
        Arc::from(metrics_inst),
        Arc::new(market.clone()),
        as_of,
        base_result.value,   // model PV as base_value
        metrics, ...
    )?;

    // Model-specific measures from price_dyn take priority
    for (k, v) in base_result.measures {
        enriched.measures.insert(k, v);
    }
    Ok(enriched)
}
```

### 2. `Instrument::metrics_equivalent()`

New method on the `Instrument` trait with a default that returns `self.clone_box()`. Bond overrides to normalize PIK coupon type to Cash and clear MC model config, so that z-spread/YTM are solved against standard cash-pay cashflows.

```rust
// Instrument trait — default impl
fn metrics_equivalent(&self) -> Box<dyn Instrument> {
    self.clone_box()
}

// Bond override
fn metrics_equivalent(&self) -> Box<dyn Instrument> {
    let mut clone = self.clone();
    // Normalize PIK → Cash for comparable spread metrics
    if let CashflowSpec::Fixed(ref mut spec) = clone.cashflow_spec {
        spec.coupon_type = CouponType::Cash;
    }
    clone.pricing_overrides.model_config.merton_mc_config = None;
    Box::new(clone)
}
```

### 3. MC pricer cleanup

Removed `cash_equivalent_bond`, `compute_ceq_metrics`, and hand-rolled z-spread/YTM from `price_dyn`. Standard metrics flow through the generic pipeline.

### 4. Python binding

Lifted the "discounting only" restriction. Routes through the registry's `price_with_metrics`.

## Why This Works

For non-discounting models, `metrics_equivalent()` returns a version of the instrument with normalized cashflows. The model PV flows as `base_value` into the metrics pipeline. Metric calculators solve "what z-spread / YTM / duration over the discount curve matches this model price?" against the normalized cashflows.

- **Hazard**: z-spread on cash cashflows at credit-adjusted price
- **OAS/Tree**: z-spread on cash cashflows at option-adjusted price
- **MC/PIK**: z-spread on cash cashflows at MC price (cash-equivalent z-spread)

For discounting, `clone_box()` is used (no normalization) — the instrument's own cashflows are the correct basis.

## Files Changed

| File | Change |
|------|--------|
| `valuations/src/instruments/common/traits.rs` | Add `metrics_equivalent()` to `Instrument` trait |
| `valuations/src/instruments/fixed_income/bond/types.rs` | Override to normalize PIK → Cash |
| `valuations/src/pricer.rs` | Add `price_with_metrics` to `PricerRegistry` using `metrics_equivalent()` |
| `valuations/src/instruments/fixed_income/bond/pricing/pricer/merton_mc.rs` | Remove hand-rolled metrics |
| `finstack-py/src/valuations/pricer.rs` | Lift model restriction, route through registry |

## What Does NOT Change

- `PricingOptions`: untouched
- `price_with_options` / `Instrument::price_with_metrics`: untouched
- `price_dyn` on hazard, OAS, and discount pricers: untouched
- Metric calculators: untouched
- `build_with_metrics_dyn`: untouched
- `PricerRegistry::price_with_registry` (PV-only path): untouched
