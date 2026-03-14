# Carry Decomposition Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Decompose carry into coupon income, pull-to-par, roll-down (including slide), and optional funding cost — implemented as first-class metrics that attribution consumes.

**Architecture:** New `CarryDecompositionCalculator` in the metrics pipeline computes all components in one pass (two reprices: flat-curve for pull-to-par, actual-curve for roll-down), flattens into individual `MetricId` scalars. Attribution methods read these metrics into an expanded `CarryDetail` struct. An optional `funding_curve_id()` trait method on `Instrument` enables funding cost computation.

**Tech Stack:** Rust, finstack_core, finstack_valuations (metrics + attribution modules)

**Spec:** `docs/superpowers/specs/2026-03-14-carry-decomposition-design.md`

---

## Chunk 1: Metrics Foundation

### Task 1: Add New MetricId Constants

**Files:**
- Modify: `finstack/valuations/src/metrics/core/ids.rs`

- [ ] **Step 1: Add the five new MetricId constants**

In `finstack/valuations/src/metrics/core/ids.rs`, add after the `ThetaDecay` constant (around line 134):

```rust
    /// Total carry decomposition (coupon_income + pull_to_par + roll_down - funding_cost).
    pub const CarryTotal: Self = Self(Cow::Borrowed("carry_total"));

    /// Coupon/interest income received during the carry horizon.
    pub const CouponIncome: Self = Self(Cow::Borrowed("coupon_income"));

    /// PV convergence toward par (time effect at flat yield, isolates amortization).
    pub const PullToPar: Self = Self(Cow::Borrowed("pull_to_par"));

    /// Curve shape benefit from aging along a sloped curve (includes slide).
    pub const RollDown: Self = Self(Cow::Borrowed("roll_down"));

    /// Cost of financing the position (dirty_price × funding_rate × dcf).
    pub const FundingCost: Self = Self(Cow::Borrowed("funding_cost"));
```

- [ ] **Step 2: Add new constants to ALL_STANDARD array**

Find the `ALL_STANDARD` array (around line 841) and add the five new entries after the `ThetaDecay` entry:

```rust
        MetricId::CarryTotal,
        MetricId::CouponIncome,
        MetricId::PullToPar,
        MetricId::RollDown,
        MetricId::FundingCost,
```

- [ ] **Step 3: Verify compilation**

Note: `from_str` uses a `metric_lookup()` hashmap auto-built from `ALL_STANDARD`, so adding to the array in Step 2 automatically enables string parsing. No match arms to update.

Run: `cargo check -p finstack-valuations 2>&1 | head -20`
Expected: Compiles successfully (new constants are just `const` declarations, no consumers yet).

- [ ] **Step 4: Commit**

```bash
git add finstack/valuations/src/metrics/core/ids.rs
git commit -m "feat(metrics): add CarryTotal, CouponIncome, PullToPar, RollDown, FundingCost MetricId constants"
```

---

### Task 2: Add `funding_curve_id()` to Instrument Trait

**Files:**
- Modify: `finstack/valuations/src/instruments/common/traits.rs`

- [ ] **Step 1: Add the default method to the Instrument trait**

In `finstack/valuations/src/instruments/common/traits.rs`, add inside the `Instrument` trait, after the `dividend_schedule_id()` method (around line 1160):

```rust
    /// Optional funding curve for carry cost computation.
    ///
    /// Returns the CurveId of the funding/repo curve used to finance
    /// this position. Used by `CarryDecompositionCalculator` to compute
    /// funding cost as `dirty_price × funding_rate × day_count_fraction`.
    ///
    /// Returns `None` for unfunded positions (default).
    fn funding_curve_id(&self) -> Option<finstack_core::types::CurveId> {
        None
    }
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p finstack-valuations 2>&1 | head -20`
Expected: Compiles — default method, no existing impls need updating.

- [ ] **Step 3: Commit**

```bash
git add finstack/valuations/src/instruments/common/traits.rs
git commit -m "feat(instruments): add funding_curve_id() default method to Instrument trait"
```

---

### Task 3: Add `funding_curve_id` Field to Bond and Override

**Files:**
- Modify: `finstack/valuations/src/instruments/fixed_income/bond/types.rs`

- [ ] **Step 1: Add the optional field to the Bond struct**

In the `Bond` struct (around line 49), add after the `credit_curve_id` field (line 65):

```rust
    /// Optional funding/repo curve for carry cost computation.
    /// When present, `CarryDecompositionCalculator` computes funding cost.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[builder(default)]
    pub funding_curve_id: Option<CurveId>,
```

Also add the field to the `BondHelper` deserialization struct (around line 101) with the same serde attributes:

```rust
    #[serde(default)]
    funding_curve_id: Option<CurveId>,
```

And wire it through in the `From<BondHelper>` conversion.

- [ ] **Step 2: Override `funding_curve_id()` in the Instrument impl**

In the `impl Instrument for Bond` block (around line 1282), add after the `expiry()` method:

```rust
    fn funding_curve_id(&self) -> Option<finstack_core::types::CurveId> {
        self.funding_curve_id.clone()
    }
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p finstack-valuations 2>&1 | head -20`
Expected: Compiles. The new field is `Option` with `#[serde(default)]` and `#[builder(default)]` so all existing construction paths continue to work.

- [ ] **Step 4: Commit**

```bash
git add finstack/valuations/src/instruments/fixed_income/bond/types.rs
git commit -m "feat(bond): add optional funding_curve_id field for carry cost computation"
```

---

### Task 4: Create CarryDecompositionCalculator

**Files:**
- Create: `finstack/valuations/src/metrics/sensitivities/carry_decomposition.rs`
- Modify: `finstack/valuations/src/metrics/sensitivities/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `finstack/valuations/src/metrics/sensitivities/carry_decomposition.rs` with the test module first:

```rust
//! Carry decomposition calculator.
//!
//! Computes a full carry breakdown: coupon income, pull-to-par, roll-down
//! (including slide), and optional funding cost. All components are stored
//! as individual scalars in `MetricContext::computed` for consumption by
//! attribution and screening tools.
//!
//! # Algorithm
//!
//! 1. Collect cashflows in `(as_of, as_of + horizon]` → `CouponIncome`
//! 2. Reprice at horizon with flat curve at current YTM → `PullToPar`
//! 3. Reprice at horizon with actual T0 curve → total PV change
//! 4. `RollDown = total_pv_change - PullToPar`
//! 5. Optional: `FundingCost = dirty_price × funding_rate × dcf`
//! 6. `CarryTotal = CouponIncome + PullToPar + RollDown - FundingCost`

use crate::instruments::common_impl::traits::Instrument;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

/// Computes carry decomposition and stores all components in `context.computed`.
///
/// Registered under `MetricId::CarryTotal`. Sub-components are retrieved via
/// `CarryComponentLookup`.
pub struct CarryDecompositionCalculator;

impl Default for CarryDecompositionCalculator {
    fn default() -> Self {
        Self
    }
}

/// Lookup calculator for carry sub-components stored by [`CarryDecompositionCalculator`].
///
/// Returns a value previously inserted into [`MetricContext::computed`] by the
/// decomposition calculator, avoiding redundant re-computation.
pub(crate) struct CarryComponentLookup(pub MetricId);

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::config::FinstackConfig;
    use finstack_core::currency::Currency;
    use finstack_core::money::Money;

    #[test]
    fn test_zero_coupon_bond_flat_curve_no_roll_down() {
        // A zero-coupon bond on a flat curve should have:
        // - coupon_income = 0
        // - pull_to_par > 0 (discount bond converges to par)
        // - roll_down ≈ 0 (flat curve, no shape benefit)
        // This test will fail until the calculator is implemented.
        assert!(false, "CarryDecompositionCalculator not yet implemented");
    }
}
```

- [ ] **Step 2: Add module declaration**

In `finstack/valuations/src/metrics/sensitivities/mod.rs`, add:

```rust
pub(crate) mod carry_decomposition;
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p finstack-valuations test_zero_coupon_bond_flat_curve_no_roll_down 2>&1 | tail -5`
Expected: FAIL with "CarryDecompositionCalculator not yet implemented"

- [ ] **Step 4: Implement the MetricCalculator for CarryDecompositionCalculator**

Replace the placeholder in `carry_decomposition.rs` with the full implementation. Key logic:

```rust
impl MetricCalculator for CarryDecompositionCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        use crate::metrics::sensitivities::theta::{calculate_theta_date, collect_cashflows_in_period};

        let period_str = context
            .pricing_overrides
            .as_ref()
            .and_then(|po| po.scenario.theta_period.as_deref())
            .unwrap_or("1D");

        let expiry_date = context.instrument.expiry();
        let rolled_date = calculate_theta_date(context.as_of, period_str, expiry_date)?;

        if rolled_date <= context.as_of {
            context.computed.insert(MetricId::CouponIncome, 0.0);
            context.computed.insert(MetricId::PullToPar, 0.0);
            context.computed.insert(MetricId::RollDown, 0.0);
            context.computed.insert(MetricId::FundingCost, 0.0);
            return Ok(0.0);
        }

        let base_pv = context.instrument.value(&context.curves, context.as_of)?.amount();
        let base_ccy = context.base_value.currency();

        // 1. Coupon income: cashflows in (as_of, rolled_date]
        let coupon_income = collect_cashflows_in_period(
            context.instrument.as_ref(),
            &context.curves,
            context.as_of,
            rolled_date,
            base_ccy,
        )?;

        // 2. Pull-to-par: reprice at rolled_date with flat curve at current YTM
        let pull_to_par = if let Some(&ytm) = context.computed.get(&MetricId::Ytm) {
            let flat_market = build_flat_curve_market(
                &context.curves,
                context.instrument.as_ref(),
                ytm,
                context.as_of,
            )?;
            let flat_pv = context.instrument.value(&flat_market, rolled_date)?.amount();
            flat_pv - base_pv
        } else {
            // No YTM available (swaps, options) — pull-to-par = 0
            0.0
        };

        // 3. Total PV change at rolled_date with actual curves
        let curved_pv = context.instrument.value(&context.curves, rolled_date)?.amount();
        let total_pv_change = curved_pv - base_pv;

        // 4. Roll-down = total PV change - pull-to-par
        let roll_down = total_pv_change - pull_to_par;

        // 5. Funding cost (optional)
        let funding_cost = compute_funding_cost(context, rolled_date)?;

        // 6. CarryTotal
        let carry_total = coupon_income + pull_to_par + roll_down - funding_cost;

        context.computed.insert(MetricId::CouponIncome, coupon_income);
        context.computed.insert(MetricId::PullToPar, pull_to_par);
        context.computed.insert(MetricId::RollDown, roll_down);
        context.computed.insert(MetricId::FundingCost, funding_cost);

        Ok(carry_total)
    }

    fn dependencies(&self) -> &[MetricId] {
        // Depend on Ytm so it's computed first (needed for flat curve construction).
        // If Ytm isn't applicable, its absence is handled gracefully.
        static DEPS: &[MetricId] = &[MetricId::Ytm];
        DEPS
    }
}
```

The helper functions `build_flat_curve_market` and `compute_funding_cost` should be private functions in the same file:

**`build_flat_curve_market`** — Constructs a `MarketContext` with a flat discount curve at the given YTM rate:

```rust
fn build_flat_curve_market(
    original_market: &finstack_core::market_data::context::MarketContext,
    instrument: &dyn Instrument,
    ytm: f64,
    base_date: finstack_core::dates::Date,
) -> Result<finstack_core::market_data::context::MarketContext> {
    let deps = instrument.market_dependencies()?;
    let discount_curve_id = &deps.curve_dependencies().discount_curves[0];

    // Build flat discount curve: DF(t) = exp(-ytm * t) for standard tenors
    let mut knots = Vec::new();
    knots.push((0.0, 1.0));
    for &tenor in finstack_core::market_data::diff::STANDARD_TENORS {
        let discount = (-ytm * tenor).exp();
        knots.push((tenor, discount));
    }

    let flat_curve = finstack_core::market_data::term_structures::DiscountCurve::builder(
        discount_curve_id.as_str(),
    )
    .base_date(base_date)
    .knots(knots)
    .interp(finstack_core::math::interp::InterpStyle::Linear)
    .build()?;

    // Clone original market, replace the discount curve with the flat one
    Ok(original_market.clone().insert(flat_curve))
}
```

**`compute_funding_cost`** — Checks `instrument.funding_curve_id()`, looks up the rate from the market, computes `base_pv × annual_rate × dcf`:

```rust
fn compute_funding_cost(
    context: &MetricContext,
    rolled_date: finstack_core::dates::Date,
) -> Result<f64> {
    let Some(funding_curve_id) = context.instrument.funding_curve_id() else {
        return Ok(0.0);
    };

    let funding_curve = context.curves.get_discount_curve(funding_curve_id.as_str())?;
    let tenor_years = (rolled_date - context.as_of).whole_days() as f64 / 365.0;
    let df = funding_curve.discount_factor(tenor_years);
    // Extract annual rate from discount factor: rate = -ln(df) / t
    let annual_rate = if tenor_years > 0.0 { -df.ln() / tenor_years } else { 0.0 };

    let dirty_price = context.base_value.amount();
    let dcf = tenor_years; // Simple Act/365 for funding cost

    Ok(dirty_price * annual_rate * dcf)
}
```

**Important**: The `collect_cashflows_in_period` function in `theta.rs` is currently private. It needs to be made `pub(crate)` so `carry_decomposition.rs` can use it. Similarly, `calculate_theta_date` is already `pub`.

- [ ] **Step 5: Make `collect_cashflows_in_period` pub(crate) in theta.rs**

In `finstack/valuations/src/metrics/sensitivities/theta.rs`, change line 325:

```rust
// From:
fn collect_cashflows_in_period(
// To:
pub(crate) fn collect_cashflows_in_period(
```

- [ ] **Step 6: Implement CarryComponentLookup**

```rust
impl MetricCalculator for CarryComponentLookup {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        context.computed.get(&self.0).copied().ok_or_else(|| {
            finstack_core::InputError::NotFound {
                id: format!("metric:{}", self.0),
            }
            .into()
        })
    }

    fn dependencies(&self) -> &[MetricId] {
        static DEPS: &[MetricId] = &[MetricId::CarryTotal];
        DEPS
    }
}
```

- [ ] **Step 7: Replace failing test with real tests**

Replace the placeholder test with comprehensive tests:

1. **Zero-coupon bond on flat curve**: coupon_income = 0, pull_to_par > 0, roll_down ≈ 0
2. **Fixed-coupon par bond on flat curve**: coupon_income > 0, pull_to_par ≈ 0, roll_down ≈ 0
3. **Premium bond on flat curve**: coupon_income > 0, pull_to_par < 0
4. **Discount bond on steep curve**: pull_to_par > 0, roll_down > 0
5. **Component sum identity**: coupon_income + pull_to_par + roll_down - funding_cost ≈ carry_total
6. **Bond with funding cost**: verify funding_cost = dirty_price × rate × dcf
7. **No funding curve**: verify funding_cost = 0

Use the existing test infrastructure from `theta.rs` tests — construct `MetricContext` with a `TestInstrument` or real `Bond` using `Bond::fixed()`, flat/steep `DiscountCurve`, and call `calculator.calculate(&mut context)`.

- [ ] **Step 8: Run tests**

Run: `cargo test -p finstack-valuations carry_decomposition 2>&1 | tail -20`
Expected: All tests pass.

- [ ] **Step 9: Commit**

```bash
git add finstack/valuations/src/metrics/sensitivities/carry_decomposition.rs \
       finstack/valuations/src/metrics/sensitivities/mod.rs \
       finstack/valuations/src/metrics/sensitivities/theta.rs
git commit -m "feat(metrics): implement CarryDecompositionCalculator with coupon income, pull-to-par, roll-down, funding cost"
```

---

### Task 5: Register Carry Metrics in Standard Registry

**Files:**
- Modify: `finstack/valuations/src/metrics/mod.rs` (re-export)
- Modify: `finstack/valuations/src/metrics/core/registry.rs` (registration)

- [ ] **Step 1: Add re-exports in metrics/mod.rs**

In `finstack/valuations/src/metrics/mod.rs`, add after the existing theta re-exports (around line 231):

```rust
pub(crate) use sensitivities::carry_decomposition::{
    CarryDecompositionCalculator, CarryComponentLookup,
};
```

- [ ] **Step 2: Register the five metrics in register_universal_metrics**

In `finstack/valuations/src/metrics/core/registry.rs`, inside `register_universal_metrics` (after the ThetaDecay registration around line 456), add:

```rust
    registry.register_metric(
        MetricId::CarryTotal,
        std::sync::Arc::new(CarryDecompositionCalculator),
        &[],
    );
    registry.register_metric(
        MetricId::CouponIncome,
        std::sync::Arc::new(CarryComponentLookup(MetricId::CouponIncome)),
        &[],
    );
    registry.register_metric(
        MetricId::PullToPar,
        std::sync::Arc::new(CarryComponentLookup(MetricId::PullToPar)),
        &[],
    );
    registry.register_metric(
        MetricId::RollDown,
        std::sync::Arc::new(CarryComponentLookup(MetricId::RollDown)),
        &[],
    );
    registry.register_metric(
        MetricId::FundingCost,
        std::sync::Arc::new(CarryComponentLookup(MetricId::FundingCost)),
        &[],
    );
```

- [ ] **Step 3: Verify compilation and tests**

Run: `cargo test -p finstack-valuations carry_decomposition 2>&1 | tail -10`
Expected: All carry decomposition tests pass.

Run: `cargo test -p finstack-valuations test_metrics 2>&1 | tail -10`
Expected: Existing metric tests still pass.

- [ ] **Step 4: Commit**

```bash
git add finstack/valuations/src/metrics/mod.rs \
       finstack/valuations/src/metrics/core/registry.rs
git commit -m "feat(metrics): register carry decomposition metrics in standard registry"
```

---

## Chunk 2: Attribution Integration

### Task 6: Expand CarryDetail Struct

**Files:**
- Modify: `finstack/valuations/src/attribution/types.rs`

- [ ] **Step 1: Expand the CarryDetail struct**

In `finstack/valuations/src/attribution/types.rs`, replace the `CarryDetail` struct (lines 376-410):

```rust
/// Detailed carry decomposition.
///
/// When available, breaks carry into sub-components:
/// - **coupon_income**: Net cashflows (coupons, interest) received during the period
/// - **pull_to_par**: PV convergence toward par (time effect at flat yield)
/// - **roll_down**: Curve shape benefit from aging along a sloped curve (includes slide)
/// - **funding_cost**: Cost of financing the position
/// - **theta**: Legacy field — total pre-funding carry (coupon_income + pull_to_par + roll_down)
///
/// # Reference
///
/// Bloomberg PORT decomposes carry into Carry (coupon/funding), Curve Roll-Down,
/// and Shift as distinct P&L components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarryDetail {
    /// Total carry P&L (sum of all components).
    pub total: Money,

    /// Coupon/interest income received during the period.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coupon_income: Option<Money>,

    /// PV convergence toward par (time effect at flat yield).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pull_to_par: Option<Money>,

    /// Curve shape benefit from aging along a sloped curve (includes slide).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roll_down: Option<Money>,

    /// Cost of financing the position (negative = cost).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub funding_cost: Option<Money>,

    /// Legacy theta field — retained for backward compatibility.
    /// Equal to coupon_income + pull_to_par + roll_down (total pre-funding).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theta: Option<Money>,
}
```

- [ ] **Step 2: Update scale() for CarryDetail**

In the `scale()` method (around line 578), replace the carry_detail scaling:

```rust
        if let Some(d) = &mut self.carry_detail {
            d.total *= factor;
            scale_money_opt(&mut d.coupon_income, factor);
            scale_money_opt(&mut d.pull_to_par, factor);
            scale_money_opt(&mut d.roll_down, factor);
            scale_money_opt(&mut d.funding_cost, factor);
            scale_money_opt(&mut d.theta, factor);
        }
```

- [ ] **Step 3: Update explain_impl() for CarryDetail**

In the `explain_impl()` method (around line 838), replace the carry_detail display:

```rust
            if let Some(ref detail) = self.carry_detail {
                if let Some(ref ci) = detail.coupon_income {
                    lines.push(format!("  │   ├─ Coupon Income: {}", ci));
                }
                if let Some(ref ptp) = detail.pull_to_par {
                    lines.push(format!("  │   ├─ Pull-to-Par: {}", ptp));
                }
                if let Some(ref rd) = detail.roll_down {
                    lines.push(format!("  │   ├─ Roll-Down: {}", rd));
                }
                if let Some(ref fc) = detail.funding_cost {
                    lines.push(format!("  │   ├─ Funding Cost: {}", fc));
                }
                if let Some(ref theta) = detail.theta {
                    lines.push(format!("  │   └─ Theta (legacy): {}", theta));
                }
            }
```

- [ ] **Step 4: Fix all compilation errors from struct field changes**

The `CarryDetail` struct is constructed in several places. Search for `CarryDetail {` across the codebase and update each construction site:

- `metrics_based.rs` (around line 323): Update to use new field names
- Any other files constructing `CarryDetail`

For `metrics_based.rs`, the existing construction becomes:

```rust
        attribution.carry_detail = Some(CarryDetail {
            total: attribution.carry,
            coupon_income: None,
            pull_to_par: None,
            roll_down: None,
            funding_cost: None,
            theta: Some(attribution.carry),
        });
```

- [ ] **Step 5: Verify compilation**

Run: `cargo check -p finstack-valuations 2>&1 | head -20`
Expected: Compiles successfully.

- [ ] **Step 6: Run existing attribution tests**

Run: `cargo test -p finstack-valuations attribution 2>&1 | tail -20`
Expected: All existing tests pass — we changed the struct but preserved all semantics.

- [ ] **Step 7: Commit**

```bash
git add finstack/valuations/src/attribution/types.rs \
       finstack/valuations/src/attribution/metrics_based.rs
git commit -m "feat(attribution): expand CarryDetail with coupon_income, pull_to_par, roll_down, funding_cost"
```

---

### Task 7: Wire Carry Metrics into Metrics-Based Attribution

**Files:**
- Modify: `finstack/valuations/src/attribution/metrics_based.rs`

- [ ] **Step 1: Write a failing test for carry decomposition in metrics-based attribution**

Add a test to the existing `mod tests` in `metrics_based.rs`:

```rust
    #[test]
    fn test_metrics_based_carry_decomposition() {
        // When carry decomposition metrics are present in val_t0.measures,
        // attribution should populate all CarryDetail fields.
        let as_of_t0 = date!(2025 - 01 - 15);
        let as_of_t1 = date!(2025 - 01 - 16);
        let meta = finstack_core::config::results_meta(&FinstackConfig::default());

        let instrument: Arc<dyn Instrument> = Arc::new(TestInstrument::new(
            "TEST-CARRY-DECOMP",
            Money::new(100_000.0, Currency::USD),
        ));

        let mut measures_t0 = IndexMap::new();
        measures_t0.insert(MetricId::Theta, -5.0);
        measures_t0.insert(MetricId::CarryTotal, -4.5);
        measures_t0.insert(MetricId::CouponIncome, 13.7);
        measures_t0.insert(MetricId::PullToPar, -8.2);
        measures_t0.insert(MetricId::RollDown, -10.0);
        measures_t0.insert(MetricId::FundingCost, 0.0);

        let val_t0 = ValuationResult::stamped_with_meta(
            "TEST-CARRY-DECOMP", as_of_t0,
            Money::new(100_000.0, Currency::USD), meta.clone(),
        ).with_measures(measures_t0);
        let val_t1 = ValuationResult::stamped_with_meta(
            "TEST-CARRY-DECOMP", as_of_t1,
            Money::new(99_995.5, Currency::USD), meta,
        );

        let attribution = attribute_pnl_metrics_based(
            &instrument, &MarketContext::new(), &MarketContext::new(),
            &val_t0, &val_t1, as_of_t0, as_of_t1,
        ).expect("metrics-based attribution should succeed");

        let detail = attribution.carry_detail.expect("carry_detail should be populated");
        assert!(detail.coupon_income.is_some());
        assert!(detail.pull_to_par.is_some());
        assert!(detail.roll_down.is_some());
        assert!(detail.funding_cost.is_some());
        assert!((detail.coupon_income.unwrap().amount() - 13.7).abs() < 1e-9);
        assert!((detail.pull_to_par.unwrap().amount() - (-8.2)).abs() < 1e-9);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p finstack-valuations test_metrics_based_carry_decomposition 2>&1 | tail -10`
Expected: FAIL — carry_detail doesn't have the new fields populated yet.

- [ ] **Step 3: Update carry section in attribute_pnl_metrics_based_impl**

In `metrics_based.rs`, update the carry attribution section (around line 310-328). After computing carry from Theta, check for carry decomposition metrics:

```rust
    // 1. Carry attribution
    if let Some(theta) = val_t0.measures.get(MetricId::Theta.as_str()) {
        let carry_amount = theta * time_period_days;
        attribution.carry = Money::new(carry_amount, val_t1.value.currency());

        // Check for full carry decomposition metrics
        let ccy = val_t1.value.currency();
        let has_decomposition = val_t0.measures.contains_key(MetricId::CarryTotal.as_str());

        if has_decomposition {
            // Use carry_total from decomposition as the authoritative carry value
            if let Some(&carry_total) = val_t0.measures.get(MetricId::CarryTotal.as_str()) {
                attribution.carry = Money::new(carry_total * time_period_days, ccy);
            }

            let get_scaled = |id: &MetricId| -> Option<Money> {
                val_t0.measures.get(id.as_str())
                    .map(|&v| Money::new(v * time_period_days, ccy))
            };

            attribution.carry_detail = Some(CarryDetail {
                total: attribution.carry,
                coupon_income: get_scaled(&MetricId::CouponIncome),
                pull_to_par: get_scaled(&MetricId::PullToPar),
                roll_down: get_scaled(&MetricId::RollDown),
                funding_cost: get_scaled(&MetricId::FundingCost),
                theta: Some(Money::new(carry_amount, ccy)),
            });
        } else {
            // Fallback: only theta available
            attribution.carry_detail = Some(CarryDetail {
                total: attribution.carry,
                coupon_income: None,
                pull_to_par: None,
                roll_down: None,
                funding_cost: None,
                theta: Some(attribution.carry),
            });
        }
    }
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p finstack-valuations test_metrics_based_carry 2>&1 | tail -10`
Expected: Both the new and existing carry tests pass.

- [ ] **Step 5: Commit**

```bash
git add finstack/valuations/src/attribution/metrics_based.rs
git commit -m "feat(attribution): wire carry decomposition metrics into metrics-based attribution"
```

---

### Task 8: Wire Carry Metrics into Parallel Attribution

**Files:**
- Modify: `finstack/valuations/src/attribution/parallel.rs`

- [ ] **Step 1: Add carry decomposition to parallel attribution**

In `parallel.rs`, after the carry computation (around line 209 where `attribution.carry` is set), add inline decomposition using the same `collect_cashflows_in_period` + reprice approach:

```rust
    attribution.carry = compute_pnl(val_t0, val_carry, val_t1.currency(), market_t1, as_of_t1)?;

    // Attempt carry decomposition inline (requires cashflow provider + YTM)
    {
        use crate::metrics::sensitivities::theta::collect_cashflows_in_period;

        let ccy = val_t1.currency();
        let base_pv = val_t0.amount();

        // Coupon income
        let coupon_income = if instrument.as_cashflow_provider().is_some() {
            collect_cashflows_in_period(
                instrument.as_ref(), &market_frozen, as_of_t0, as_of_t1, ccy,
            ).ok()
        } else {
            None
        };

        // Pull-to-par: reprice at T1 with flat curve at YTM
        // Only attempt if YTM was pre-computed (requires val_t0 to be provided)
        // For parallel attribution, we compute it via cashflow subtraction:
        // pull_to_par = total_carry - coupon_income - roll_down
        // Since we don't have YTM readily, set pull_to_par + roll_down as combined
        // Users wanting the full decomposition should use CarryTotal metric.

        if let Some(ci) = coupon_income {
            let pv_change = attribution.carry.amount() - ci;
            attribution.carry_detail = Some(CarryDetail {
                total: attribution.carry,
                coupon_income: Some(Money::new(ci, ccy)),
                pull_to_par: None, // Requires YTM; use CarryTotal metric for full decomposition
                roll_down: None,   // Combined with pull_to_par
                funding_cost: None,
                theta: Some(Money::new(pv_change + ci, ccy)),
            });
        }
    }
```

Note: Parallel attribution can separate coupon income (via cashflows) from PV change, but cannot fully separate pull-to-par from roll-down without the YTM-based flat curve reprice. The full decomposition requires the `CarryTotal` metric. This is documented in the code.

- [ ] **Step 2: Run existing parallel attribution tests**

Run: `cargo test -p finstack-valuations attribution 2>&1 | tail -20`
Expected: All tests pass.

- [ ] **Step 3: Commit**

```bash
git add finstack/valuations/src/attribution/parallel.rs
git commit -m "feat(attribution): add partial carry decomposition to parallel attribution"
```

---

### Task 9: Wire Carry Metrics into Waterfall Attribution

**Files:**
- Modify: `finstack/valuations/src/attribution/waterfall.rs`

- [ ] **Step 1: Add carry decomposition to waterfall attribution**

In `waterfall.rs`, in the match arm where carry factor P&L is recorded (around line 250):

```rust
            AttributionFactor::Carry => {
                attribution.carry = factor_pnl;

                // Same partial decomposition as parallel: extract coupon income
                use crate::metrics::sensitivities::theta::collect_cashflows_in_period;

                let ccy = factor_pnl.currency();
                if let Ok(ci) = collect_cashflows_in_period(
                    self.current_instrument.as_ref(),
                    &self.current_market,
                    self.as_of_t0,
                    self.current_date,
                    ccy,
                ) {
                    attribution.carry_detail = Some(CarryDetail {
                        total: factor_pnl,
                        coupon_income: Some(Money::new(ci, ccy)),
                        pull_to_par: None,
                        roll_down: None,
                        funding_cost: None,
                        theta: Some(Money::new(factor_pnl.amount(), ccy)),
                    });
                }
            }
```

- [ ] **Step 2: Run waterfall attribution tests**

Run: `cargo test -p finstack-valuations waterfall 2>&1 | tail -20`
Expected: All tests pass.

- [ ] **Step 3: Commit**

```bash
git add finstack/valuations/src/attribution/waterfall.rs
git commit -m "feat(attribution): add partial carry decomposition to waterfall attribution"
```

---

### Task 10: Integration Tests

**Files:**
- Create or modify: `finstack/valuations/tests/attribution/bond_attribution.rs` (or appropriate test file)

- [ ] **Step 1: Write integration test for full carry decomposition via metrics-based attribution**

Add a test that constructs a real `Bond`, populates a `MarketContext` with curves, computes valuations with carry metrics, and runs metrics-based attribution:

```rust
#[test]
fn test_bond_carry_decomposition_full() {
    // Setup: 5% coupon bond, 5Y maturity, priced at par on a steep curve
    // Run: compute val_t0 with CarryTotal metric, then attribute
    // Assert: all CarryDetail fields populated, component sum ≈ total
}
```

- [ ] **Step 2: Write backward compatibility test**

```rust
#[test]
fn test_carry_detail_backward_compat() {
    // Setup: same bond, but compute val_t0 with only Theta metric (no CarryTotal)
    // Run: metrics-based attribution
    // Assert: carry_detail has theta populated, coupon_income/pull_to_par/roll_down are None
}
```

- [ ] **Step 3: Run all attribution tests**

Run: `cargo test -p finstack-valuations attribution 2>&1 | tail -20`
Expected: All tests pass including new integration tests.

- [ ] **Step 4: Run full test suite**

Run: `cargo test -p finstack-valuations 2>&1 | tail -20`
Expected: All tests pass. No regressions.

- [ ] **Step 5: Commit**

```bash
git add finstack/valuations/tests/
git commit -m "test(attribution): add integration tests for carry decomposition"
```

---

## Chunk 3: Final Verification

### Task 11: Full Build and Test Verification

- [ ] **Step 1: Run full workspace build**

Run: `cargo build --workspace 2>&1 | tail -20`
Expected: Clean build, no warnings related to carry decomposition.

- [ ] **Step 2: Run full workspace tests**

Run: `cargo test --workspace 2>&1 | tail -30`
Expected: All tests pass.

- [ ] **Step 3: Run clippy**

Run: `cargo clippy --workspace 2>&1 | tail -20`
Expected: No new warnings.

- [ ] **Step 4: Verify no golden file changes**

Run: `git diff --name-only`
Expected: Only files we explicitly modified. No unexpected changes to golden test files.
