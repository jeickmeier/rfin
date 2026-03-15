# Rust Codebase Simplification — Remaining Findings

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Consolidate duplicated Black-Scholes/Black-76 computations, extract shared Carr-Madan variance replication, add parallel VaR scenarios, and audit dead-code annotations.

**Architecture:** Five independent workstreams — each can be implemented and merged independently. Ordered by impact: (1) d1/d2 consolidation eliminates 16 hand-rolled formula sites, (2) BS price consolidation eliminates 5 reimplementations, (3) Carr-Madan extraction deduplicates ~60 lines between equity/FX variance swaps, (4) parallel VaR adds rayon support to scenario evaluation, (5) dead-code audit removes unnecessary `#![allow(dead_code)]` annotations.

**Dependencies:** Task 4 (BS price consolidation) depends on Task 1 (d1/d2 consolidation). All other chunks are independent.

**Important: Module path aliasing.** The `instruments/` directory uses a path alias: `#[path = "common/mod.rs"] pub(crate) mod common_impl;` in `instruments/mod.rs`. All internal crate imports use `common_impl`, NOT `common`. The public `mod common` is an inline re-export facade. Always use `crate::instruments::common_impl::...` for internal imports.

**Tech Stack:** Rust, `finstack_core`, `finstack_valuations`, `rayon` (existing optional feature)

---

## Chunk 1: d1/d2 Consolidation (Tasks 1–3)

The canonical functions are:
- `d1_d2(spot, strike, r, sigma, t, q) -> (f64, f64)` — BSM form
- `d1_d2_black76(forward, strike, sigma, t) -> (f64, f64)` — Black-76 form

Both live in `finstack/valuations/src/instruments/common/models/volatility/black.rs`.

### Task 1: Replace straightforward BSM d1/d2 sites in valuations crate

Each site computes `((spot / strike).ln() + (r - q + 0.5 * vol * vol) * t) / (vol * sqrt_t)` inline. Replace with a call to `d1_d2(spot, strike, r, sigma, t, q)`.

**Files to modify:**
- `finstack/valuations/src/instruments/common/models/closed_form/barrier.rs:234` — `vanilla_bs` helper
- `finstack/valuations/src/instruments/common/models/closed_form/asian.rs:632,650` — `vanilla_call_bs`, `vanilla_put_bs`
- `finstack/valuations/src/instruments/common/models/closed_form/heston.rs:421` — `black_scholes_call` fallback
- `finstack/valuations/src/instruments/common/models/monte_carlo/variance_reduction/control_variate.rs:66,91` — `black_scholes_call`, `black_scholes_put`
- `finstack/valuations/src/instruments/common/models/trees/binomial_tree.rs:162` — Leisen-Reimer d1/d2
- `finstack/valuations/src/instruments/common/models/volatility/sabr.rs:1590,1606` — `bs_call_vega`, `bs_call_price`
- `finstack/valuations/src/instruments/equity/variance_swap/pricer.rs:373` — inline d1 in Carr-Madan loop
- `finstack/valuations/src/instruments/fx/fx_barrier_option/vanna_volga.rs:73,86,100` — `bs_vega`, `bs_vanna`, `bs_volga`

**Import to add:** `use crate::instruments::common_impl::models::volatility::black::d1_d2;`

- [ ] **Step 1: Add import and replace d1/d2 in barrier.rs `vanilla_bs`**

In `barrier.rs:234`, replace:

```rust
let d1 = ((spot / strike).ln() + (rate - div_yield + 0.5 * vol * vol) * time) / (vol * sqrt_t);
let d2 = d1 - vol * sqrt_t;
```

with:

```rust
let (d1, d2) = d1_d2(spot, strike, rate, vol, time, div_yield);
```

Check if `sqrt_t` is used elsewhere in the same function before removing it — several sites (e.g., variance_swap pricer) use `sqrt_t` for other purposes beyond d1/d2. Only remove if the compiler warns about unused bindings.

- [ ] **Step 2: Replace d1/d2 in asian.rs `vanilla_call_bs` and `vanilla_put_bs`**

Same pattern at lines 632 and 650. Add import, replace inline formulas with `d1_d2(spot, strike, rate, vol, time, div_yield)`.

- [ ] **Step 3: Replace d1/d2 in heston.rs `black_scholes_call`**

At line 421: `let (d1, d2) = d1_d2(spot, strike, r, vol, time, q);`

- [ ] **Step 4: Replace d1/d2 in control_variate.rs**

At lines 66 and 91. Both functions compute the same formula.

- [ ] **Step 5: Replace d1/d2 in binomial_tree.rs Leisen-Reimer**

At line 162: `let (d1, d2) = d1_d2(spot, strike, r, sigma, t, q);`

- [ ] **Step 6: Investigate and replace d1/d2 in sabr.rs `bs_call_vega` and `bs_call_price`**

At lines 1590 and 1606. **WARNING:** The param is named `forward` but the formula uses BSM `r, q` drift. The call site at `sabr.rs:1559-1560` passes `self.forward` (a true forward price from `SABRSmile`), then applies BSM drift `r - q` on top — which is mathematically suspicious (double-counting the forward adjustment). Before replacing, verify:
1. Read the call sites to confirm whether `forward` is spot or actual forward
2. If it IS forward, the existing formula may be a latent bug — replacing with `d1_d2` would preserve the (potentially incorrect) behavior
3. If uncertain, skip this site and add it to the "blocked" table in Task 3

- [ ] **Step 7: Replace d1/d2 in variance_swap/pricer.rs**

At line 373 inside the Carr-Madan loop. Uses `spot, k, r, vol, t, q`.

- [ ] **Step 8: Replace d1/d2 in vanna_volga.rs**

Three functions at lines 73, 86, 100. Use `d1_d2(spot, strike, r_d, vol, t, r_f)` — the foreign rate maps to the dividend yield `q`.

- [ ] **Step 9: Run tests**

```bash
cargo test --lib -p finstack-valuations -- barrier asian heston control_variate binomial sabr variance_swap vanna_volga 2>&1 | tail -5
```

Expected: all pass

- [ ] **Step 10: Commit**

```bash
git add finstack/valuations/src/instruments/
git commit -m "refactor: replace 16 hand-rolled d1/d2 with canonical d1_d2()"
```

### Task 2: Replace Black-76 d1/d2 sites

**Files to modify:**
- `finstack/valuations/src/instruments/rates/range_accrual/pricer.rs:395` — Black-76 in closure
- `finstack/core/src/market_data/surfaces/fx_delta_vol_surface.rs:207` — only d1 needed

**Import:** `use crate::instruments::common_impl::models::volatility::black::{d1_d2_black76, d1_black76};`

- [ ] **Step 1: Replace in range_accrual/pricer.rs**

At line 395 inside the `black_call` closure:

```rust
let (d1, d2) = d1_d2_black76(forward, k, vol, t_obs);
```

- [ ] **Step 2: Replace in fx_delta_vol_surface.rs**

At line 207 — this is in `finstack_core`. The canonical `d1_d2_black76` also lives in `finstack_valuations`, so it **cannot** be imported here. Instead, add a `d1_black76` function to `finstack_core::math::volatility::pricing` (which already has `black_call`, `black_put`, `black_vega` etc.) and call that.

Check if `finstack_core` already has an inline d1_black76:

```bash
grep -n "d1_black76\|fn d1(" finstack/core/src/math/volatility/pricing.rs
```

If not, add one (simple 5-line function). Then use it from `fx_delta_vol_surface.rs`.

- [ ] **Step 3: Run tests**

```bash
cargo test -p finstack-valuations -- range_accrual 2>&1 | tail -3
cargo test -p finstack -- fx_delta_vol 2>&1 | tail -3
```

- [ ] **Step 4: Commit**

### Task 3: Skip sites with blockers (document rationale)

These sites should NOT be replaced — document why in a code comment at each site:

| Site | Reason |
|------|--------|
| `core/src/math/volatility/implied.rs:212` | `ln_fk` pre-hoisted for Newton loop performance |
| `core/src/math/volatility/sabr.rs:388` | In `finstack_core`, cannot import from valuations |
| `core/src/math/volatility/heston.rs:593` | In `finstack_core`, cannot import from valuations |
| `closed_form/barrier.rs:322` | Merton barrier x,x1,y,y1 terms — not d1/d2 |
| `closed_form/quanto.rs:115,166` | Quanto-adjusted drift doesn't map cleanly to `r, q` params |
| `commodity_asian_option/pricer.rs:219` | Pre-computed adjusted variance, not decomposable into sigma,t |

- [ ] **Step 1: Add brief `// Uses hand-rolled d1 (see docs/superpowers/plans/)` comment at each site**

No code change — only a comment explaining why these are intentionally not consolidated.

- [ ] **Step 2: Commit**

---

## Chunk 2: BS Price Function Consolidation (Tasks 4–5)

The canonical function is `bs_price(spot, strike, r, q, sigma, t, option_type) -> f64` in `finstack/valuations/src/instruments/common/models/closed_form/vanilla.rs:159`.

### Task 4: Replace hand-rolled BS price functions

**Files to modify:**
- `finstack/valuations/src/instruments/common/models/closed_form/heston.rs:413` — `black_scholes_call`
- `finstack/valuations/src/instruments/common/models/closed_form/asian.rs:622,640` — `vanilla_call_bs`, `vanilla_put_bs`
- `finstack/valuations/src/instruments/common/models/monte_carlo/variance_reduction/control_variate.rs:53,78` — `black_scholes_call`, `black_scholes_put`

Each of these computes d1/d2 then applies the standard BS formula. After Task 1 consolidates d1/d2, these functions become thin wrappers that could be replaced by `bs_price()`.

- [ ] **Step 1: Replace `black_scholes_call` in heston.rs**

Replace the entire function body at line 413 with:

```rust
fn black_scholes_call(spot: f64, strike: f64, time: f64, r: f64, q: f64, vol: f64) -> f64 {
    use crate::instruments::common_impl::models::closed_form::vanilla::bs_price;
    use crate::instruments::common_impl::parameters::market::OptionType;
    bs_price(spot, strike, r, q, vol, time, OptionType::Call)
}
```

Note: check if `OptionType` is already imported at the top of heston.rs. If so, remove the local `use`.

- [ ] **Step 2: Replace `vanilla_call_bs` and `vanilla_put_bs` in asian.rs**

Same pattern — delegate to `bs_price`.

- [ ] **Step 3: Replace `black_scholes_call` and `black_scholes_put` in control_variate.rs**

These are `pub` functions — check if anything outside the module calls them. If so, keep the functions but replace the body. If not, consider making them private or removing in favor of direct `bs_price` calls at the call sites.

- [ ] **Step 4: Replace `black_call_undiscounted` in core SABR**

`finstack/core/src/math/volatility/sabr.rs:382` — this IS in core and can use `black_call` from `core/src/math/volatility/pricing.rs`:

```rust
fn black_call_undiscounted(forward: f64, strike: f64, expiry: f64, vol: f64) -> f64 {
    crate::math::volatility::pricing::black_call(forward, strike, vol, expiry)
}
```

Or just inline the call at the call site and remove the function.

- [ ] **Step 5: Run tests**

```bash
cargo test --lib -p finstack-valuations -- heston asian control_variate 2>&1 | tail -5
cargo test --lib -p finstack -- sabr 2>&1 | tail -5
```

- [ ] **Step 6: Commit**

### Task 5: Document remaining BS price sites that cannot be consolidated

These are NOT straightforward replacements:

| Function | File | Reason |
|----------|------|--------|
| `bs_call_vega` (SABR) | `sabr.rs:1584` | No canonical vega without 0.01 scaling factor |
| `bs_vega/vanna/volga` (Vanna-Volga) | `vanna_volga.rs:68,81,95` | Second-order Greeks have no canonical equivalent |
| `bs_call_fallback` (core Heston) | `core/heston.rs:573` | In `finstack_core`, cannot import from valuations |
| `black_call/put/vega/delta/gamma` (core pricing) | `core/pricing.rs` | These ARE the canonical core-level implementations |

- [ ] **Step 1: No code changes — this task is documentation only**

---

## Chunk 3: Carr-Madan Variance Replication Extraction (Task 6)

### Task 6: Extract shared Carr-Madan discrete replication integral

The equity variance swap (`equity/variance_swap/pricer.rs:350-409`) and FX variance swap (`fx/fx_variance_swap/pricer.rs:301-356`) share ~30 lines of identical replication logic. The differences are only:
1. How the forward is computed (equity: `spot * ((r-q)*t).exp()`, FX: `spot * ((r_d-r_f)*t).exp()`)
2. How BS prices are obtained (equity: inline, FX: calls `bs_price`)
3. Which rate appears in the variance formula (equity: `r`, FX: `r_d`)

**Files:**
- Create: `finstack/valuations/src/instruments/common/pricing/variance_replication.rs`
- Modify: `finstack/valuations/src/instruments/common/pricing/mod.rs` — add `pub mod variance_replication;` (this is the filesystem path; internally it's accessed as `common_impl::pricing::variance_replication`)
- Modify: `finstack/valuations/src/instruments/equity/variance_swap/pricer.rs`
- Modify: `finstack/valuations/src/instruments/fx/fx_variance_swap/pricer.rs`

- [ ] **Step 1: Design the shared interface**

Read both call sites to determine the minimal closure interface:
- Equity (`equity/variance_swap/pricer.rs:372-383`): computes `bs_price(spot, k, r, q, vol, t, opt)` inline
- FX (`fx/fx_variance_swap/pricer.rs:323-324`): calls `bs_price(spot, k, r_d, r_f, vol, t, opt)`

Both ultimately need `(strike, vol, option_type) -> price`. All other params (`spot`, `r`, `q`, `t`) are constant within the loop and can be captured by the closure. The `dk` (numerical differentiation width) is NOT a pricing parameter — it's used outside `bs_price_fn` in the formula `sum += (dk / (k*k)) * qk`.

Chosen signature:

```rust
bs_price_fn: impl Fn(f64, f64, OptionType) -> f64,  // (strike, vol, option_type) -> price
```

- [ ] **Step 2: Write the failing test**

Create a unit test in the new module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::common_impl::models::closed_form::vanilla::bs_price;
    use crate::instruments::common_impl::parameters::market::OptionType;

    #[test]
    fn test_carr_madan_atm_flat_vol() {
        // With flat vol and many strikes, variance ≈ vol^2
        let vol = 0.20;
        let t = 1.0;
        let fwd = 100.0;
        let r = 0.05;
        let strikes: Vec<f64> = (50..=150).map(|k| k as f64).collect();
        let vol_fn = |_t: f64, _k: f64| vol;
        // Capture spot, r, q, t in closure — only strike, vol, opt vary
        let spot = fwd; // at-the-money, no dividends
        let bs_fn = |k: f64, v: f64, opt: OptionType| -> f64 {
            bs_price(spot, k, r, 0.0, v, t, opt)
        };
        let result = carr_madan_forward_variance(&strikes, fwd, r, t, vol_fn, bs_fn);
        assert!(result.is_some());
        let variance = result.unwrap();
        assert!((variance - vol * vol).abs() < 0.01, "Expected ~{}, got {}", vol * vol, variance);
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cargo test -p finstack-valuations -- variance_replication 2>&1
```

Expected: compilation error (function doesn't exist)

- [ ] **Step 4: Implement the shared function**

```rust
use crate::instruments::common_impl::parameters::market::OptionType;

/// Carr-Madan discrete variance replication integral.
///
/// Computes forward variance from a discrete strike grid using the
/// log-contract replication approach (Carr & Madan, 1998).
///
/// `vol_fn(t, k)` returns implied vol at time `t` and strike `k`.
/// `bs_price_fn(strike, vol, option_type)` returns the undiscounted option price.
/// All other parameters (spot, rates, etc.) should be captured by the closures.
///
/// Returns `None` if the result is non-finite or non-positive (falls
/// back to ATM vol^2 at the call site).
pub fn carr_madan_forward_variance(
    strikes: &[f64],
    forward: f64,
    risk_free_rate: f64,
    time_to_expiry: f64,
    vol_fn: impl Fn(f64, f64) -> f64,
    bs_price_fn: impl Fn(f64, f64, OptionType) -> f64,
) -> Option<f64> {
    if strikes.len() < 3 || !forward.is_finite() || forward <= 0.0 {
        return None;
    }

    // Find ATM strike index
    let k0_idx = strikes
        .iter()
        .position(|&k| k > forward)
        .unwrap_or(strikes.len() - 1)
        .saturating_sub(1);
    let k0 = strikes[k0_idx].max(1e-12);

    let mut sum = 0.0;
    for (i, &k) in strikes.iter().enumerate() {
        // Numerical derivative dk (forward/backward/central differences)
        let dk = if i == 0 {
            strikes[1] - strikes[0]
        } else if i == strikes.len() - 1 {
            strikes[i] - strikes[i - 1]
        } else {
            (strikes[i + 1] - strikes[i - 1]) * 0.5
        };

        let vol = vol_fn(time_to_expiry, k).max(1e-8);

        // Option type selection per Carr-Madan: puts below forward, calls above
        let qk = if i == k0_idx {
            // ATM: average of put and call
            0.5 * (bs_price_fn(k, vol, OptionType::Put)
                + bs_price_fn(k, vol, OptionType::Call))
        } else if k < forward {
            bs_price_fn(k, vol, OptionType::Put)
        } else {
            bs_price_fn(k, vol, OptionType::Call)
        };

        sum += (dk / (k * k)) * qk;
    }

    let variance = (2.0 * (risk_free_rate * time_to_expiry).exp() / time_to_expiry) * sum
        - (1.0 / time_to_expiry) * ((forward / k0 - 1.0).powi(2));

    if variance.is_finite() && variance > 0.0 {
        Some(variance)
    } else {
        None
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

- [ ] **Step 5: Update equity variance swap pricer to use shared function**

- [ ] **Step 6: Update FX variance swap pricer to use shared function**

- [ ] **Step 7: Run both test suites**

```bash
cargo test -p finstack-valuations -- variance_swap 2>&1 | tail -5
```

- [ ] **Step 8: Commit**

---

## Chunk 4: Parallel VaR Scenarios (Task 7)

### Task 7: Add rayon-parallel VaR full revaluation

**Files:**
- Modify: `finstack/valuations/src/metrics/risk/var_calculator.rs:269-341`

The existing `aggregate_scenario_pnls` function iterates scenarios sequentially. Each scenario is independent (creates its own `MarketContext` via `scenario.apply(base_market)`). The crate already has `parallel` as a default feature with rayon.

The existing patterns to follow are in:
- `finstack/valuations/src/pricer/registry.rs:330` — `#[cfg(feature = "parallel")]` batch pricing
- `finstack/valuations/src/instruments/common/models/monte_carlo/engine.rs` — parallel path simulation

- [ ] **Step 1: Read the existing `aggregate_scenario_pnls` function**

Understand the current signature:

```rust
fn aggregate_scenario_pnls<F>(
    history: &MarketHistory,
    base_market: &MarketContext,
    mut scenario_pnl: F,
) -> Result<Vec<f64>>
where
    F: FnMut(&MarketContext) -> Result<f64>,
```

- [ ] **Step 2: Investigate `MarketHistory` iteration and trait bounds**

`MarketHistory` does NOT implement `IntoParallelRefIterator`. Check its internal structure:

```bash
grep -A 5 "struct MarketHistory" finstack/valuations/src/metrics/risk/market_history.rs
```

The parallel version will need to access the internal `Vec<MarketScenario>` directly (e.g., `history.scenarios().par_iter()`) or collect via `.iter().collect::<Vec<_>>().par_iter()`.

Also: the current closure is `FnMut`. The parallel version requires `Fn + Send + Sync`. The closure captures `instrument_refs: Vec<&I>` which contains references — this requires `I: Sync` to be `Send` across threads. Add this bound to the parallel path.

- [ ] **Step 3: Add parallel version**

Add a `#[cfg(feature = "parallel")]` block. Use the internal scenarios slice directly:

```rust
#[cfg(feature = "parallel")]
fn aggregate_scenario_pnls_par<F>(
    scenarios: &[MarketScenario],
    base_market: &MarketContext,
    scenario_pnl: F,
) -> Result<Vec<f64>>
where
    F: Fn(&MarketContext) -> Result<f64> + Send + Sync,
{
    use rayon::prelude::*;
    scenarios
        .par_iter()
        .map(|scenario| {
            let scenario_market = scenario.apply(base_market)?;
            scenario_pnl(&scenario_market)
        })
        .collect()
}
```

The caller needs to extract the scenarios slice from `MarketHistory` and pass it in. Update the `calculate_var_full_revaluation` function's trait bounds to include `I: Sync` when the `parallel` feature is enabled.

- [ ] **Step 3: Update `calculate_var_full_revaluation` to use parallel version**

Gate with `#[cfg(feature = "parallel")]` / `#[cfg(not(feature = "parallel"))]`.

- [ ] **Step 4: Run tests**

```bash
cargo test -p finstack-valuations -- var 2>&1 | tail -5
```

- [ ] **Step 5: Commit**

---

## Chunk 5: Dead Code Annotation Audit (Tasks 8–9)

### Task 8: Remove unnecessary `#![allow(dead_code)]` from validated files

The following files have `#![allow(dead_code)]` but all public items are actively used via re-exports and Python bindings:

- `finstack/valuations/src/calibration/validation/curves.rs`
- `finstack/valuations/src/calibration/validation/points.rs`
- `finstack/valuations/src/calibration/validation/surfaces.rs`
- `finstack/valuations/src/margin/registry/wire.rs` (serde deserialization targets — may need per-struct `#[allow(dead_code)]` instead of blanket file-level)

- [ ] **Step 1: Remove `#![allow(dead_code)]` from curves.rs, points.rs, surfaces.rs**

- [ ] **Step 2: Build to check for warnings**

```bash
cargo check -p finstack-valuations 2>&1 | grep "warning.*dead_code\|warning.*unused"
```

If new warnings appear, add targeted `#[allow(dead_code)]` on specific items rather than the blanket file-level annotation.

- [ ] **Step 3: Handle wire.rs**

For `wire.rs`, serde `Deserialize` accesses fields via generated code that the compiler can't see. Replace the blanket `#![allow(dead_code)]` with per-struct annotations:

```rust
#[allow(dead_code)] // Fields accessed via serde Deserialize
#[derive(Debug, Deserialize)]
pub struct ScheduleImRecord { ... }
```

- [ ] **Step 4: Build and test**

```bash
cargo check -p finstack-valuations 2>&1 | grep warning | head -10
cargo test -p finstack-valuations -- calibration margin 2>&1 | tail -5
```

- [ ] **Step 5: Commit**

### Task 9: Audit remaining `#![allow(dead_code)]` files (investigative)

The remaining ~37 files with `#![allow(dead_code)]` need individual investigation. This is a larger audit task.

**Approach:**
1. For each file, check if items are re-exported in `mod.rs`/`lib.rs`
2. Check if items are used in `finstack-py/` bindings
3. If items are genuinely unused, remove them (not just the annotation)
4. If items are used only via re-export for external crates, replace blanket annotation with targeted `#[allow(dead_code)]` on specific items

- [ ] **Step 1: Generate list of all affected files**

```bash
grep -rl '#!\[allow(dead_code)\]' finstack/valuations/src/ finstack/core/src/ finstack/portfolio/src/ finstack/scenarios/src/ finstack/statements/src/ | grep -v target | grep -v test | sort
```

- [ ] **Step 2: For each file, check usage**

For each file, run:

```bash
# Check if items are imported elsewhere
grep -r "use.*<module_name>" finstack/ finstack-py/ --include="*.rs" | grep -v target
```

- [ ] **Step 3: Remove blanket annotations where safe, add targeted ones where needed**

- [ ] **Step 4: Build and verify no regressions**

- [ ] **Step 5: Commit**
