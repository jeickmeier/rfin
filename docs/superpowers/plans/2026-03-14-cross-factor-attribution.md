# Cross-Factor Attribution Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add explicit cross-factor interaction terms (rates×credit, rates×vol, spot×vol, etc.) as reusable metrics, then integrate them into P&L attribution to reduce parallel-method residual from 5-15% to <2% for multi-asset books.

**Architecture:** New `CrossGamma{F1}{F2}` `MetricId` constants backed by `MetricCalculator` implementations that use the hybrid compute strategy: analytical where available (e.g., existing `Vanna`), finite-difference via `central_mixed` otherwise, reusing bumped-market states from single-factor computations where possible. A new `cross_factor_pnl` field on `PnlAttribution` with `CrossFactorDetail` breakdown keeps single-factor fields pure first-order while making cross-terms explicit. Parallel×Parallel granularity now, but `Structured2D` ready for future bucketed×bucketed.

**Tech Stack:** Rust, finstack_valuations metrics framework, finstack_core market data bumps

---

## File Structure

### New Files

| File | Responsibility |
|------|---------------|
| `valuations/src/metrics/sensitivities/cross_factor.rs` | `CrossFactorPair` enum, generic `CrossFactorCalculator` struct, all `MetricCalculator` impls for cross-gamma metrics |
| `valuations/tests/cross_factor_metrics_tests.rs` | Integration tests for cross-factor metrics (standalone, outside attribution) |
| `valuations/tests/cross_factor_attribution_tests.rs` | Integration tests for cross-factor attribution (parallel + metrics-based) |

### Modified Files

| File | Changes |
|------|---------|
| `valuations/src/metrics/core/ids.rs` | Add `CrossGammaRatesCredit`, `CrossGammaRatesVol`, `CrossGammaSpotVol`, `CrossGammaSpotCredit`, `CrossGammaFxVol`, `CrossGammaFxRates` constants + add to `ALL_STANDARD` |
| `valuations/src/metrics/sensitivities/mod.rs` | Add `pub(crate) mod cross_factor;` |
| `valuations/src/metrics/core/finite_difference.rs` | Remove `#[cfg(any(feature = "mc", test))]` gate from `central_mixed` so it's always available |
| `valuations/src/attribution/types.rs` | Add `cross_factor_pnl: Money` field, `cross_factor_detail: Option<CrossFactorDetail>`, `CrossFactorDetail` struct, update `compute_residual`, `scale`, `explain`, `validate_currencies` |
| `valuations/src/attribution/parallel.rs` | Add cross-factor computation step between single-factor isolation and residual |
| `valuations/src/attribution/metrics_based.rs` | Use cross-factor metrics in P&L approximation |
| `valuations/src/attribution/mod.rs` | Re-export `CrossFactorDetail` |
| `valuations/src/metrics/mod.rs` | Re-export cross-factor types if needed |

---

## Chunk 1: Cross-Factor MetricId Variants + `central_mixed` Ungating

### Task 1: Ungate `central_mixed` from feature flag

**Files:**
- Modify: `valuations/src/metrics/core/finite_difference.rs:189`

Currently `central_mixed` is gated behind `#[cfg(any(feature = "mc", test))]`. Cross-factor calculators need it unconditionally.

- [ ] **Step 1: Write a test that uses `central_mixed` without the `mc` feature**

In `valuations/src/metrics/core/finite_difference.rs`, the existing test `central_mixed_rejects_nonpositive_or_invalid_hk` already exercises it under `#[cfg(test)]`. We need to verify it compiles without `mc`. Add a simple correctness test:

```rust
#[test]
fn central_mixed_computes_known_cross_derivative() {
    // f(x,y) = x*y  =>  ∂²f/∂x∂y = 1.0
    let h = 0.01;
    let k = 0.01;
    let result = central_mixed(
        || Ok(( 1.0 + h) * (1.0 + k)),   // f(+h, +k)
        || Ok(( 1.0 + h) * (1.0 - k)),   // f(+h, -k)
        || Ok(( 1.0 - h) * (1.0 + k)),   // f(-h, +k)
        || Ok(( 1.0 - h) * (1.0 - k)),   // f(-h, -k)
        h,
        k,
    )
    .expect("should succeed");
    assert!((result - 1.0).abs() < 1e-10, "cross derivative of x*y should be 1.0, got {result}");
}
```

- [ ] **Step 2: Run test to verify it passes (it should, since tests have the cfg gate)**

Run: `cargo test -p finstack_valuations central_mixed_computes_known`
Expected: PASS

- [ ] **Step 3: Remove the `cfg` gate from `central_mixed`**

> **Note:** The unit test above passes even before ungating (since `#[cfg(test)]` is always true in test builds). The ungating is validated by the fact that `cross_factor.rs` (Task 3) imports `central_mixed` without `cfg(test)` — if the gate weren't removed, the production build would fail. The unit test serves as a correctness check, not a gate-removal proof.

In `valuations/src/metrics/core/finite_difference.rs`, change line 190:

```rust
// BEFORE:
#[cfg(any(feature = "mc", test))]
pub fn central_mixed<...>(

// AFTER:
pub fn central_mixed<...>(
```

- [ ] **Step 4: Run all tests to verify nothing breaks**

Run: `cargo test -p finstack_valuations`
Expected: PASS (all existing tests still pass)

- [ ] **Step 5: Commit**

```bash
git add valuations/src/metrics/core/finite_difference.rs
git commit -m "feat(metrics): ungate central_mixed for cross-factor use"
```

---

### Task 2: Add CrossGamma MetricId constants

**Files:**
- Modify: `valuations/src/metrics/core/ids.rs`

- [ ] **Step 1: Write a test for the new MetricId constants**

Add to the existing test module in `ids.rs`:

```rust
#[test]
fn test_cross_gamma_metric_ids_exist_and_parse() {
    let pairs = [
        (MetricId::CrossGammaRatesCredit, "cross_gamma_rates_credit"),
        (MetricId::CrossGammaRatesVol, "cross_gamma_rates_vol"),
        (MetricId::CrossGammaSpotVol, "cross_gamma_spot_vol"),
        (MetricId::CrossGammaSpotCredit, "cross_gamma_spot_credit"),
        (MetricId::CrossGammaFxVol, "cross_gamma_fx_vol"),
        (MetricId::CrossGammaFxRates, "cross_gamma_fx_rates"),
    ];
    for (id, expected_str) in &pairs {
        assert_eq!(id.as_str(), *expected_str);
        let parsed = MetricId::parse_strict(expected_str).unwrap();
        assert_eq!(&parsed, id);
        assert!(!parsed.is_custom());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p finstack_valuations test_cross_gamma_metric_ids_exist`
Expected: FAIL — `CrossGammaRatesCredit` does not exist

- [ ] **Step 3: Add the MetricId constants**

In `ids.rs`, after the existing `IrCrossGamma` constant (around line 451), add a new section:

```rust
// ========================================================================
// Cross-Factor Gamma Metrics
// ========================================================================

/// Cross-gamma between interest rates and credit spreads.
///
/// Mixed second derivative: ∂²V / (∂r × ∂s).
/// Measures how DV01 changes when credit spreads move (and vice versa).
/// Parallel×parallel granularity; bucketed variants planned for future.
pub const CrossGammaRatesCredit: Self = Self(Cow::Borrowed("cross_gamma_rates_credit"));

/// Cross-gamma between interest rates and implied volatility.
///
/// Mixed second derivative: ∂²V / (∂r × ∂σ).
/// Captures rate-vol interaction (e.g., callable bonds, swaptions).
pub const CrossGammaRatesVol: Self = Self(Cow::Borrowed("cross_gamma_rates_vol"));

/// Cross-gamma between spot price and implied volatility (Vanna-like).
///
/// Mixed second derivative: ∂²V / (∂S × ∂σ).
/// Equivalent to Vanna but named consistently with the cross-factor framework.
/// For instruments that already compute Vanna analytically, this metric
/// delegates to the analytical value when available.
pub const CrossGammaSpotVol: Self = Self(Cow::Borrowed("cross_gamma_spot_vol"));

/// Cross-gamma between spot price and credit spreads.
///
/// Mixed second derivative: ∂²V / (∂S × ∂s).
/// Important for convertible bonds and credit-equity hybrid products.
pub const CrossGammaSpotCredit: Self = Self(Cow::Borrowed("cross_gamma_spot_credit"));

/// Cross-gamma between FX rates and implied volatility.
///
/// Mixed second derivative: ∂²V / (∂FX × ∂σ).
/// Captures FX-vol interaction for quanto and cross-currency options.
pub const CrossGammaFxVol: Self = Self(Cow::Borrowed("cross_gamma_fx_vol"));

/// Cross-gamma between FX rates and interest rates.
///
/// Mixed second derivative: ∂²V / (∂FX × ∂r).
/// Important for cross-currency swaps and FX forwards with rate sensitivity.
pub const CrossGammaFxRates: Self = Self(Cow::Borrowed("cross_gamma_fx_rates"));
```

Then add all six to the `ALL_STANDARD` array (after `IrCrossGamma`):

```rust
MetricId::CrossGammaRatesCredit,
MetricId::CrossGammaRatesVol,
MetricId::CrossGammaSpotVol,
MetricId::CrossGammaSpotCredit,
MetricId::CrossGammaFxVol,
MetricId::CrossGammaFxRates,
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p finstack_valuations test_cross_gamma_metric_ids_exist`
Expected: PASS

- [ ] **Step 5: Run full test suite**

Run: `cargo test -p finstack_valuations`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add valuations/src/metrics/core/ids.rs
git commit -m "feat(metrics): add CrossGamma MetricId constants for 6 factor pairs"
```

---

## Chunk 2: Cross-Factor Metric Calculators

### Task 3: Create `CrossFactorPair` enum and `CrossFactorCalculator`

**Files:**
- Create: `valuations/src/metrics/sensitivities/cross_factor.rs`
- Modify: `valuations/src/metrics/sensitivities/mod.rs`

This is the core reusable module. It defines which factor pairs exist and how to compute their cross-gamma via bump-and-reprice using `central_mixed`.

- [ ] **Step 1: Add the module declaration**

In `valuations/src/metrics/sensitivities/mod.rs`, add:

```rust
pub(crate) mod cross_factor;
```

- [ ] **Step 2: Write the failing test for CrossGammaRatesCredit**

Create `valuations/src/metrics/sensitivities/cross_factor.rs` with the test first:

```rust
//! Cross-factor gamma calculators.
//!
//! Provides reusable `MetricCalculator` implementations for mixed second
//! derivatives (∂²V/∂f₁∂f₂) between pairs of risk factors. These capture
//! interaction effects that single-factor sensitivities miss.
//!
//! # Supported Pairs
//!
//! | MetricId | Factor 1 | Factor 2 | Bump 1 | Bump 2 |
//! |----------|----------|----------|--------|--------|
//! | `CrossGammaRatesCredit` | IR parallel | Credit parallel | 1bp | 1bp |
//! | `CrossGammaRatesVol` | IR parallel | Vol surface | 1bp | 1 vol pt |
//! | `CrossGammaSpotVol` | Spot price | Vol surface | 1% | 1 vol pt |
//! | `CrossGammaSpotCredit` | Spot price | Credit parallel | 1% | 1bp |
//! | `CrossGammaFxVol` | FX rate | Vol surface | 1% | 1 vol pt |
//! | `CrossGammaFxRates` | FX rate | IR parallel | 1% | 1bp |
//!
//! # Granularity
//!
//! Currently parallel×parallel only. The architecture supports future
//! bucketed×parallel and bucketed×bucketed via `Structured2D` storage,
//! controlled by a granularity enum on the calculator.
//!
//! # Computation Strategy
//!
//! Uses `central_mixed` from `finite_difference.rs`:
//! ```text
//! ∂²V/∂f₁∂f₂ = [V(+h,+k) - V(+h,-k) - V(-h,+k) + V(-h,-k)] / (4hk)
//! ```
//!
//! Where bumped markets are constructed by composing single-factor bump
//! helpers (`bump_discount_curve_parallel`, `bump_surface_vol_absolute`,
//! `bump_scalar_price`).

use crate::instruments::common_impl::traits::Instrument;
use crate::metrics::core::finite_difference::{bump_scalar_price, bump_surface_vol_absolute};
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::currency::Currency;
use finstack_core::market_data::bumps::{BumpSpec, MarketBump};
use finstack_core::market_data::context::MarketContext;
use finstack_core::types::CurveId;
use finstack_core::Result;
use std::sync::Arc;

// NOTE: We inline the central-difference formula in CrossFactorCalculator::calculate
// rather than using `central_mixed` because the four repricings happen outside a
// closure context (we need mutable bumper composition that doesn't compose well
// with the closure-based API). The formula is identical:
//   (v_pp - v_pm - v_mp + v_mm) / (4 * h * k)

/// Identifies which two risk factors are crossed.
///
/// Each variant maps to a specific pair of bump operations.
/// Parallel×parallel granularity is the default; future variants
/// (e.g., `BucketedRatesCredit`) can extend this without breaking
/// existing code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CrossFactorPair {
    /// IR parallel × Credit parallel
    RatesCredit,
    /// IR parallel × Vol surface parallel
    RatesVol,
    /// Spot price × Vol surface parallel
    SpotVol,
    /// Spot price × Credit parallel
    SpotCredit,
    /// FX rate × Vol surface parallel
    FxVol,
    /// FX rate × IR parallel
    FxRates,
}

impl CrossFactorPair {
    /// All supported pairs, for iteration during attribution.
    pub const ALL: &'static [CrossFactorPair] = &[
        CrossFactorPair::RatesCredit,
        CrossFactorPair::RatesVol,
        CrossFactorPair::SpotVol,
        CrossFactorPair::SpotCredit,
        CrossFactorPair::FxVol,
        CrossFactorPair::FxRates,
    ];

    /// Returns the `MetricId` for this cross-factor pair.
    pub fn metric_id(&self) -> crate::metrics::MetricId {
        use crate::metrics::MetricId;
        match self {
            CrossFactorPair::RatesCredit => MetricId::CrossGammaRatesCredit,
            CrossFactorPair::RatesVol => MetricId::CrossGammaRatesVol,
            CrossFactorPair::SpotVol => MetricId::CrossGammaSpotVol,
            CrossFactorPair::SpotCredit => MetricId::CrossGammaSpotCredit,
            CrossFactorPair::FxVol => MetricId::CrossGammaFxVol,
            CrossFactorPair::FxRates => MetricId::CrossGammaFxRates,
        }
    }

    /// Human-readable label for display/logging.
    pub fn label(&self) -> &'static str {
        match self {
            CrossFactorPair::RatesCredit => "Rates×Credit",
            CrossFactorPair::RatesVol => "Rates×Vol",
            CrossFactorPair::SpotVol => "Spot×Vol",
            CrossFactorPair::SpotCredit => "Spot×Credit",
            CrossFactorPair::FxVol => "FX×Vol",
            CrossFactorPair::FxRates => "FX×Rates",
        }
    }
}

// ============================================================================
// Bump Helpers — factor-specific market bumping
// ============================================================================

/// Trait for applying a single-factor bump in either direction.
///
/// Each factor type (rates, credit, vol, spot, fx) implements this
/// to produce a bumped `MarketContext`. The cross-factor calculator
/// composes two `FactorBumper` implementations to build the four
/// corner markets needed for `central_mixed`.
pub(crate) trait FactorBumper: Send + Sync {
    /// Apply a bump of the given signed magnitude to the market.
    ///
    /// `direction` is +1.0 or -1.0, multiplied by the bump size.
    fn bump_market(&self, market: &MarketContext, direction: f64) -> Result<MarketContext>;

    /// The absolute bump size (always positive) used for normalization.
    fn bump_size(&self) -> f64;

    /// Whether this bumper can operate on the given market context.
    ///
    /// Returns false if the required market data (curves, surfaces, scalars)
    /// is missing. Used to skip irrelevant cross-factor pairs.
    fn is_applicable(&self, market: &MarketContext) -> bool;
}

/// Bumps all discount curves in parallel by `bump_bp` basis points.
pub(crate) struct RatesParallelBumper {
    pub curve_ids: Vec<CurveId>,
    pub bump_bp: f64,
}

impl FactorBumper for RatesParallelBumper {
    fn bump_market(&self, market: &MarketContext, direction: f64) -> Result<MarketContext> {
        let bumps: Vec<MarketBump> = self
            .curve_ids
            .iter()
            .map(|id| MarketBump::Curve {
                id: id.clone(),
                spec: BumpSpec::parallel_bp(self.bump_bp * direction),
            })
            .collect();
        market.bump(bumps)
    }

    fn bump_size(&self) -> f64 {
        self.bump_bp * 1e-4 // convert bp to decimal for normalization
    }

    fn is_applicable(&self, market: &MarketContext) -> bool {
        self.curve_ids
            .iter()
            .any(|id| market.get_discount(id.as_str()).is_ok())
    }
}

/// Bumps all hazard curves in parallel by `bump_bp` basis points.
pub(crate) struct CreditParallelBumper {
    pub curve_ids: Vec<CurveId>,
    pub bump_bp: f64,
}

impl FactorBumper for CreditParallelBumper {
    fn bump_market(&self, market: &MarketContext, direction: f64) -> Result<MarketContext> {
        let bumps: Vec<MarketBump> = self
            .curve_ids
            .iter()
            .map(|id| MarketBump::Curve {
                id: id.clone(),
                spec: BumpSpec::parallel_bp(self.bump_bp * direction),
            })
            .collect();
        market.bump(bumps)
    }

    fn bump_size(&self) -> f64 {
        self.bump_bp * 1e-4
    }

    fn is_applicable(&self, market: &MarketContext) -> bool {
        self.curve_ids
            .iter()
            .any(|id| market.get_hazard(id.as_str()).is_ok())
    }
}

/// Bumps a vol surface by `bump_abs` vol points (additive parallel).
pub(crate) struct VolParallelBumper {
    pub surface_id: String,
    pub bump_abs: f64,
}

impl FactorBumper for VolParallelBumper {
    fn bump_market(&self, market: &MarketContext, direction: f64) -> Result<MarketContext> {
        bump_surface_vol_absolute(market, &self.surface_id, self.bump_abs * direction)
    }

    fn bump_size(&self) -> f64 {
        self.bump_abs
    }

    fn is_applicable(&self, market: &MarketContext) -> bool {
        market.get_surface(&self.surface_id).is_ok()
    }
}

/// Bumps a scalar price by `bump_pct` (relative, e.g., 0.01 = 1%).
pub(crate) struct SpotBumper {
    pub price_id: String,
    pub bump_pct: f64,
}

impl FactorBumper for SpotBumper {
    fn bump_market(&self, market: &MarketContext, direction: f64) -> Result<MarketContext> {
        bump_scalar_price(market, &self.price_id, self.bump_pct * direction)
    }

    fn bump_size(&self) -> f64 {
        // For spot, the absolute bump depends on the current spot level.
        // We return the percentage bump; the calculator will resolve the
        // absolute amount at compute time from the market context.
        self.bump_pct
    }

    fn is_applicable(&self, market: &MarketContext) -> bool {
        market.get_price(&self.price_id).is_ok()
    }
}

/// Bumps FX rate by `bump_pct` (relative, e.g., 1.0 = 1%).
///
/// FX bumps are applied via `MarketBump::FxPct` which wraps the
/// underlying FX matrix with a `BumpedFxProvider`. This is the same
/// mechanism used by the scenario engine for FX shocks.
pub(crate) struct FxBumper {
    pub base: Currency,
    pub quote: Currency,
    pub bump_pct: f64,
}

impl FactorBumper for FxBumper {
    fn bump_market(&self, market: &MarketContext, direction: f64) -> Result<MarketContext> {
        market.bump([MarketBump::FxPct {
            base: self.base,
            quote: self.quote,
            pct: self.bump_pct * direction,
        }])
    }

    fn bump_size(&self) -> f64 {
        self.bump_pct / 100.0 // convert percentage to decimal for normalization
    }

    fn is_applicable(&self, market: &MarketContext) -> bool {
        market.fx().is_some()
    }
}

// ============================================================================
// Generic Cross-Factor Calculator
// ============================================================================

/// Computes the cross-gamma for an arbitrary pair of factors.
///
/// Uses `central_mixed` with four-corner repricing:
/// ```text
/// ∂²V/∂f₁∂f₂ = [V(+h,+k) - V(+h,-k) - V(-h,+k) + V(-h,-k)] / (4hk)
/// ```
///
/// The calculator takes two `FactorBumper` trait objects and composes them
/// to build the four bumped market states. If either bumper is not applicable
/// (missing market data), the calculator returns 0.0 — the cross-term is
/// irrelevant for that instrument/market combination.
pub struct CrossFactorCalculator {
    /// Identifies this pair for logging/debugging.
    pub pair: CrossFactorPair,
    /// Bumper for the first factor.
    pub bumper_a: Arc<dyn FactorBumper>,
    /// Bumper for the second factor.
    pub bumper_b: Arc<dyn FactorBumper>,
}

impl MetricCalculator for CrossFactorCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let market = &context.curves;
        let instrument = &context.instrument;
        let as_of = context.as_of;

        // Skip if either factor's market data is missing
        if !self.bumper_a.is_applicable(market) || !self.bumper_b.is_applicable(market) {
            return Ok(0.0);
        }

        let h = self.bumper_a.bump_size();
        let k = self.bumper_b.bump_size();

        // Build four corner markets: (a+, b+), (a+, b-), (a-, b+), (a-, b-)
        let market_ap = self.bumper_a.bump_market(market, 1.0)?;
        let market_am = self.bumper_a.bump_market(market, -1.0)?;

        let market_ap_bp = self.bumper_b.bump_market(&market_ap, 1.0)?;
        let market_ap_bm = self.bumper_b.bump_market(&market_ap, -1.0)?;
        let market_am_bp = self.bumper_b.bump_market(&market_am, 1.0)?;
        let market_am_bm = self.bumper_b.bump_market(&market_am, -1.0)?;

        let market_ap_bp = Arc::new(market_ap_bp);
        let market_ap_bm = Arc::new(market_ap_bm);
        let market_am_bp = Arc::new(market_am_bp);
        let market_am_bm = Arc::new(market_am_bm);

        // Reprice at four corners
        let inst = Arc::clone(instrument);
        let v_pp = inst.value(&market_ap_bp, as_of)?;
        let v_pm = inst.value(&market_ap_bm, as_of)?;
        let v_mp = inst.value(&market_am_bp, as_of)?;
        let v_mm = inst.value(&market_am_bm, as_of)?;

        // Central mixed difference
        let cross_gamma =
            (v_pp.amount() - v_pm.amount() - v_mp.amount() + v_mm.amount()) / (4.0 * h * k);

        Ok(cross_gamma)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn cross_factor_pair_metric_id_roundtrip() {
        for pair in CrossFactorPair::ALL {
            let id = pair.metric_id();
            assert!(!id.is_custom(), "CrossFactorPair::{:?} metric should be standard", pair);
        }
    }

    #[test]
    fn cross_factor_pair_labels_are_nonempty() {
        for pair in CrossFactorPair::ALL {
            assert!(!pair.label().is_empty());
        }
    }
}
```

- [ ] **Step 3: Run the test to verify it compiles and passes**

Run: `cargo test -p finstack_valuations cross_factor_pair`
Expected: PASS (the basic enum tests should pass)

- [ ] **Step 4: Commit**

```bash
git add valuations/src/metrics/sensitivities/cross_factor.rs valuations/src/metrics/sensitivities/mod.rs
git commit -m "feat(metrics): add CrossFactorPair enum and CrossFactorCalculator"
```

---

### Task 4: Write integration test for CrossGammaRatesCredit with a real instrument

**Files:**
- Create: `valuations/tests/cross_factor_metrics_tests.rs`

This test exercises the full pipeline. Since `sensitivities` is `pub(crate)`, these tests must import via public re-exports.

- [ ] **Step 0: Add public re-exports for cross-factor types**

In `valuations/src/metrics/mod.rs`, add re-exports for the types integration tests need:

```rust
pub use sensitivities::cross_factor::{CrossFactorPair, CrossFactorCalculator};
```

This makes the types available as `finstack_valuations::metrics::CrossFactorPair` etc.

The bumper types (`RatesParallelBumper`, `CreditParallelBumper`, etc.) remain `pub(crate)` — they are implementation details. Integration tests exercise cross-factor metrics via the `MetricRegistry` (compute by `MetricId`), not by constructing calculators directly.

- [ ] **Step 1: Write the integration test**

```rust
//! Integration tests for cross-factor gamma metrics.
//!
//! Tests exercise the cross-factor calculators through the MetricRegistry
//! public API, computing metrics by MetricId.

use finstack_core::currency::Currency;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_valuations::metrics::{MetricContext, MetricId, MetricRegistry};
use std::sync::Arc;
use time::macros::date;

/// Helper: build a simple discount curve.
fn make_discount_curve(id: &str, base_date: time::Date) -> DiscountCurve {
    DiscountCurve::builder(id)
        .base_date(base_date)
        .knots(vec![(0.0, 1.0), (1.0, 0.97), (5.0, 0.85), (10.0, 0.72)])
        .interp(InterpStyle::Linear)
        .build()
        .expect("test curve should build")
}

#[test]
fn cross_gamma_metric_id_is_registered_and_computable() {
    // Verify CrossGammaRatesCredit can be computed via the registry
    // for instruments that have both rate and credit sensitivity.
    //
    // This test will be populated after Task 8 (registry wiring).
    // For now, verify the MetricId exists and is standard:
    let id = MetricId::CrossGammaRatesCredit;
    assert!(!id.is_custom());
    assert_eq!(id.as_str(), "cross_gamma_rates_credit");
}

#[test]
fn cross_factor_pair_covers_all_metric_ids() {
    use finstack_valuations::metrics::CrossFactorPair;
    // Every pair maps to a standard MetricId
    for pair in CrossFactorPair::ALL {
        let id = pair.metric_id();
        let parsed = MetricId::parse_strict(id.as_str()).unwrap();
        assert_eq!(parsed, id);
    }
}
```

- [ ] **Step 2: Run test to verify it compiles**

Run: `cargo test -p finstack_valuations --test cross_factor_metrics_tests`
Expected: PASS (placeholder test)

- [ ] **Step 3: Commit**

```bash
git add valuations/tests/cross_factor_metrics_tests.rs
git commit -m "test(metrics): add integration test scaffold for cross-factor metrics"
```

---

## Chunk 3: Attribution Types + Integration

### Task 5: Add `cross_factor_pnl` and `CrossFactorDetail` to `PnlAttribution`

**Files:**
- Modify: `valuations/src/attribution/types.rs`
- Modify: `valuations/src/attribution/mod.rs`

- [ ] **Step 1: Write tests for the new fields**

Add to the existing test module in `types.rs` (or create a new test file):

```rust
#[test]
fn test_cross_factor_detail_serde_roundtrip() {
    let detail = CrossFactorDetail {
        total: Money::new(500.0, Currency::USD),
        by_pair: {
            let mut map = IndexMap::new();
            map.insert("Rates×Credit".to_string(), Money::new(300.0, Currency::USD));
            map.insert("Spot×Vol".to_string(), Money::new(200.0, Currency::USD));
            map
        },
    };
    let json = serde_json::to_string(&detail).unwrap();
    let parsed: CrossFactorDetail = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.total.amount(), 500.0);
    assert_eq!(parsed.by_pair.len(), 2);
}

#[test]
fn test_compute_residual_includes_cross_factor() {
    let total = Money::new(1000.0, Currency::USD);
    let mut attr = PnlAttribution::new(
        total,
        "TEST",
        date!(2025-01-01),
        date!(2025-01-02),
        AttributionMethod::Parallel,
    );
    // Set some single-factor P&L
    attr.rates_curves_pnl = Money::new(600.0, Currency::USD);
    attr.credit_curves_pnl = Money::new(200.0, Currency::USD);
    // Set cross-factor P&L
    attr.cross_factor_pnl = Money::new(150.0, Currency::USD);

    attr.compute_residual().unwrap();

    // Residual should be: 1000 - 600 - 200 - 150 = 50
    assert!((attr.residual.amount() - 50.0).abs() < 1e-10);
}
```

- [ ] **Step 2: Run test to verify it fails (fields don't exist yet)**

Run: `cargo test -p finstack_valuations test_cross_factor_detail`
Expected: FAIL — `CrossFactorDetail` not found

- [ ] **Step 3: Add `CrossFactorDetail` struct and new fields**

In `valuations/src/attribution/types.rs`, add the struct:

```rust
/// Detailed attribution for cross-factor interaction terms.
///
/// Breaks down the `cross_factor_pnl` by factor pair, showing how much
/// P&L is attributable to each interaction (e.g., rates×credit, spot×vol).
///
/// # Design
///
/// This is a dedicated bucket separate from single-factor fields. This
/// preserves backward compatibility (existing fields remain pure first-order)
/// and makes it easy to verify that residual is truly unexplained.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossFactorDetail {
    /// Total cross-factor P&L (sum of all pair contributions).
    pub total: Money,

    /// P&L by factor pair label (e.g., "Rates×Credit" → $300).
    ///
    /// Keys use `CrossFactorPair::label()` format for human readability.
    /// The map is ordered by insertion (IndexMap) for deterministic output.
    #[serde(default)]
    pub by_pair: IndexMap<String, Money>,
}
```

Add two new fields to `PnlAttribution`:

```rust
/// Cross-factor interaction P&L (rates×credit, spot×vol, etc.).
///
/// Captures second-order mixed-partial effects that would otherwise
/// appear in residual. Single-factor fields remain pure first-order.
pub cross_factor_pnl: Money,

/// Detailed cross-factor attribution by pair.
pub cross_factor_detail: Option<CrossFactorDetail>,
```

Update `PnlAttribution::new()` to initialize:

```rust
cross_factor_pnl: zero,
cross_factor_detail: None,
```

Update `compute_residual()` to subtract `cross_factor_pnl`:

```rust
attributed_sum = add_factor(
    attributed_sum,
    self.cross_factor_pnl,
    "cross-factor P&L",
    &mut self.meta.notes,
)?;
```

Update `scale()` to scale the new fields:

```rust
self.cross_factor_pnl *= factor;
if let Some(d) = &mut self.cross_factor_detail {
    d.total *= factor;
    scale_money_map(&mut d.by_pair, factor);
}
```

Update `validate_currencies()` to check cross_factor_pnl:

```rust
("cross_factor", self.cross_factor_pnl.currency()),
```

Update `explain_impl()` to show cross-factor detail:

```rust
if show(&self.cross_factor_pnl) {
    lines.push(format!(
        "  ├─ Cross-Factor: {}",
        fmt(&self.cross_factor_pnl, &self.total_pnl)
    ));
    if let Some(ref detail) = self.cross_factor_detail {
        for (pair_label, pnl) in &detail.by_pair {
            lines.push(format!("  │   ├─ {}: {}", pair_label, pnl));
        }
    }
}
```

- [ ] **Step 4: Update mod.rs re-exports**

In `valuations/src/attribution/mod.rs`, add `CrossFactorDetail` to the re-export:

```rust
pub use types::{
    AttributionFactor, AttributionInput, AttributionMeta, AttributionMethod, CarryDetail,
    CorrelationsAttribution, CreditCurvesAttribution, CrossFactorDetail, FxAttribution,
    InflationCurvesAttribution, ModelParamsAttribution, PnlAttribution, RatesCurvesAttribution,
    ScalarsAttribution, VolAttribution,
};
```

- [ ] **Step 5: Run tests to verify everything passes**

Run: `cargo test -p finstack_valuations`
Expected: PASS (all existing tests still pass, new test passes)

- [ ] **Step 6: Commit**

```bash
git add valuations/src/attribution/types.rs valuations/src/attribution/mod.rs
git commit -m "feat(attribution): add cross_factor_pnl field and CrossFactorDetail struct"
```

---

### Task 6: Integrate cross-factor computation into parallel attribution

**Files:**
- Modify: `valuations/src/attribution/parallel.rs`

- [ ] **Step 1: Restructure single-factor steps to capture PVs for cross-factor reuse**

The existing code creates `val_with_t0_rates` and `val_with_t0_credit` inside conditional blocks. These need to be available in the cross-factor section. Restructure by storing them as `Option<Money>`:

At the top of `attribute_pnl_parallel_impl`, after `val_t1`, add:

```rust
// Capture single-factor PVs for cross-factor reuse
let mut pv_with_t0_rates: Option<Money> = None;
let mut pv_with_t0_credit: Option<Money> = None;
let mut pv_with_t0_vol: Option<Money> = None;
```

Then in each single-factor step, after repricing, store the value. For example in Step 3 (rates), after `let val_with_t0_rates = ...`:

```rust
pv_with_t0_rates = Some(val_with_t0_rates);
```

And in Step 4 (credit):

```rust
pv_with_t0_credit = Some(val_with_t0_credit);
```

And in Step 8 (vol):

```rust
pv_with_t0_vol = Some(val_with_t0_vol);
```

- [ ] **Step 2: Write a test that verifies cross-factor terms reduce residual**

Add to the test module in `parallel.rs`. This test uses a synthetic instrument with known rate + credit sensitivity:

```rust
#[test]
fn test_cross_factor_pnl_field_populated_for_multi_factor() {
    // Build a market with both discount and hazard curves at T0 and T1
    let as_of_t0 = date!(2025 - 01 - 15);
    let as_of_t1 = date!(2025 - 01 - 16);

    let curve_t0 = DiscountCurve::builder("USD-OIS")
        .base_date(as_of_t0)
        .knots(vec![(0.0, 1.0), (1.0, 0.98)])
        .interp(InterpStyle::Linear)
        .build()
        .expect("test curve");

    let curve_t1 = DiscountCurve::builder("USD-OIS")
        .base_date(as_of_t1)
        .knots(vec![(0.0, 1.0), (1.0, 0.97)])
        .interp(InterpStyle::Linear)
        .build()
        .expect("test curve");

    let market_t0 = MarketContext::new().insert(curve_t0);
    let market_t1 = MarketContext::new().insert(curve_t1);
    let config = FinstackConfig::default();

    // For now: verify the cross_factor_pnl field exists and is zero
    // for a single-factor instrument (no credit curves → no cross-term).
    // Full multi-factor test requires a risky bond instrument.
    let instrument = Arc::new(TestInstrument::new(
        "TEST-SINGLE-FACTOR",
        Money::new(1000.0, Currency::USD),
    ));

    let attribution = attribute_pnl_parallel(
        &(instrument as Arc<dyn Instrument>),
        &market_t0,
        &market_t1,
        as_of_t0,
        as_of_t1,
        &config,
        None,
    ).unwrap();

    // Single-factor: no cross-factor terms expected
    assert_eq!(attribution.cross_factor_pnl.amount(), 0.0);
    assert!(attribution.cross_factor_detail.is_none());
}
```

- [ ] **Step 3: Add cross-factor computation step to parallel attribution**

In `attribute_pnl_parallel_impl`, after Step 10 (market scalars) and before Step 11 (compute residual), add:

```rust
// Step 10b: Cross-factor interaction terms
//
// For each applicable factor pair, compute the interaction P&L using
// the discrete cross-term identity:
//   cross_AB = V(A₁,B₁) - V(A₁,B₀) - V(A₀,B₁) + V(A₀,B₀)
//
// where:
//   V(A₁,B₁) = val_t1                    (already computed)
//   V(A₁,B₀) = val_with_t0_B             (from single-factor step)
//   V(A₀,B₁) = val_with_t0_A             (from single-factor step)
//   V(A₀,B₀) = new repricing (1 per pair, both factors at T₀)
//
// IMPORTANT: This uses pv_with_t0_rates, pv_with_t0_credit, pv_with_t0_vol
// captured as Option<Money> during single-factor steps above.
{
    use indexmap::IndexMap;

    let mut cross_total = Money::new(0.0, val_t1.currency());
    let mut cross_by_pair: IndexMap<String, Money> = IndexMap::new();

    // Rates × Credit
    if let (Some(pv_rates_t0), Some(pv_credit_t0)) =
        (pv_with_t0_rates, pv_with_t0_credit)
    {
        // Build market with BOTH rates and credit at T₀
        let market_both_t0 = MarketSnapshot::restore_market(
            &market_with_t0_rates,
            &credit_snapshot,
            CurveRestoreFlags::CREDIT,
        );
        let pv_both_t0 = reprice_instrument(instrument, &market_both_t0, as_of_t1)?;
        num_repricings += 1;

        // cross = V(t1) - V(rates_t0) - V(credit_t0) + V(both_t0)
        // Reformulated using PV directly (all at T₁ date, T₁ FX):
        let cross_amount = val_t1.amount() - pv_rates_t0.amount()
            - pv_credit_t0.amount() + pv_both_t0.amount();
        let cross_pnl = Money::new(cross_amount, val_t1.currency());

        if cross_pnl.amount().abs() > 1e-10 {
            cross_by_pair.insert("Rates×Credit".to_string(), cross_pnl);
            cross_total = cross_total.checked_add(cross_pnl)?;
        }
    }

    // TODO: Add remaining pairs (Rates×Vol, Spot×Vol, etc.) following same pattern.
    // Each requires: 1) checking both factors have data, 2) building combined-T0 market,
    // 3) repricing, 4) computing cross-term via differencing.

    if !cross_by_pair.is_empty() {
        attribution.cross_factor_pnl = cross_total;
        attribution.cross_factor_detail = Some(CrossFactorDetail {
            total: cross_total,
            by_pair: cross_by_pair,
        });
    }
}
```

> **Note to implementer:** The exact formulation depends on careful accounting of which single-factor values are already computed. The key identity is:
>
> `cross_AB = V(A₁,B₁) - V(A₁,B₀) - V(A₀,B₁) + V(A₀,B₀)`
>
> where `V(A₁,B₁) = val_t1`, `V(A₁,B₀) = val_with_t0_B`, `V(A₀,B₁) = val_with_t0_A`.
> The only new repricing needed is `V(A₀,B₀)` — restoring both factors to T₀.
> This costs 1 extra repricing per pair, reusing already-computed single-factor values.

- [ ] **Step 3: Run tests**

Run: `cargo test -p finstack_valuations parallel`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add valuations/src/attribution/parallel.rs
git commit -m "feat(attribution): integrate cross-factor terms into parallel attribution"
```

---

### Task 7: Integrate cross-factor metrics into metrics-based attribution

**Files:**
- Modify: `valuations/src/attribution/metrics_based.rs`

- [ ] **Step 1: Extract market shift values into named variables for cross-factor reuse**

The existing metrics-based attribution computes rate shifts, credit shifts, vol shifts, and spot shifts inline within scoped blocks. To reuse them in cross-factor terms, extract them into named `Option<f64>` variables at function scope. Add these **before** the single-factor approximation blocks:

```rust
// Pre-compute market shifts for cross-factor reuse.
// These are populated during single-factor attribution and consumed
// by the cross-factor section at the end.
let mut avg_rate_shift_bp: Option<f64> = None;   // average rate shift in bp
let mut avg_credit_shift_bp: Option<f64> = None;  // average credit shift in bp
let mut avg_vol_shift_abs: Option<f64> = None;     // average vol shift in absolute terms
let mut avg_spot_shift_pct: Option<f64> = None;    // average spot shift in percentage
```

Then, within the existing rate attribution block, after computing `avg_shift`:

```rust
avg_rate_shift_bp = Some(avg_shift);  // already in bp units
```

Similarly for credit, vol, and spot blocks. This is a refactoring step — no behavioral change.

- [ ] **Step 2: Add cross-factor P&L approximation using pre-computed metrics**

After the single-factor approximations, add:

```rust
// Cross-factor terms using pre-computed cross-gamma metrics
//
// For each pair where both the cross-gamma metric and both market moves
// are available:
//   PnL_cross = CrossGamma × Δf₁ × Δf₂
//
// This mirrors how Vanna is already handled (Vanna × Δspot × Δσ) but
// generalizes to all supported pairs.
//
// UNIT CONTRACT:
// - CrossGammaRatesCredit: $ / (bp × bp) — multiply by Δrate_bp × Δspread_bp
// - CrossGammaRatesVol: $ / (bp × vol_point) — multiply by Δrate_bp × Δσ
// - CrossGammaSpotVol: $ / (% × vol_point) — multiply by Δspot_pct × Δσ

let mut cross_total = 0.0;
let mut cross_by_pair: IndexMap<String, Money> = IndexMap::new();

// Rates×Credit cross-term
if let Some(&cross_gamma) = val_t0.measures.get(MetricId::CrossGammaRatesCredit.as_str()) {
    if cross_gamma.abs() > 1e-16 {
        if let (Some(rate_shift), Some(credit_shift)) =
            (avg_rate_shift_bp, avg_credit_shift_bp)
        {
            let pnl = cross_gamma * rate_shift * credit_shift;
            cross_total += pnl;
            cross_by_pair.insert(
                "Rates×Credit".to_string(),
                Money::new(pnl, val_t1.value.currency()),
            );
        }
    }
}

// ... similar blocks for CrossGammaRatesVol, CrossGammaSpotVol, etc.
// Each follows the same pattern: check metric exists, check both shifts
// exist, compute pnl = cross_gamma × shift_a × shift_b.

if !cross_by_pair.is_empty() {
    attribution.cross_factor_pnl = Money::new(cross_total, val_t1.value.currency());
    attribution.cross_factor_detail = Some(CrossFactorDetail {
        total: Money::new(cross_total, val_t1.value.currency()),
        by_pair: cross_by_pair,
    });
}
```

- [ ] **Step 2: Update `required_metrics()` for MetricsBased method**

In `types.rs`, add cross-gamma metrics to the `MetricsBased` required metrics:

```rust
AttributionMethod::MetricsBased => vec![
    // ... existing first-order and second-order metrics ...
    // Cross-factor metrics
    MetricId::CrossGammaRatesCredit,
    MetricId::CrossGammaRatesVol,
    MetricId::CrossGammaSpotVol,
    MetricId::CrossGammaSpotCredit,
    MetricId::CrossGammaFxVol,
    MetricId::CrossGammaFxRates,
],
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p finstack_valuations metrics_based`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add valuations/src/attribution/metrics_based.rs valuations/src/attribution/types.rs
git commit -m "feat(attribution): integrate cross-factor metrics into metrics-based attribution"
```

---

## Chunk 4: Registry Wiring + Full Integration Tests

### Task 8: Runtime ID resolution + Registry wiring

**Files:**
- Modify: `valuations/src/metrics/sensitivities/cross_factor.rs` (add `InstrumentAwareBumper`)
- Modify: instrument-specific metric registration files (found via `grep -r "register_metric" valuations/src/instruments/`)

The key design problem: bumper IDs (e.g., which curve to bump, which vol surface) depend on the specific instrument being valued — they can't be hardcoded at registration time. The existing pattern (see `GenericFdVanna`) resolves IDs from the instrument via trait methods like `instrument.equity_dependencies()`.

- [ ] **Step 1: Add instrument-aware bumper factory pattern**

Instead of pre-configuring bumpers with hardcoded IDs, make `CrossFactorCalculator` resolve IDs at calculate-time from `MetricContext`. Add a `BumperFactory` trait:

```rust
/// Factory that creates a `FactorBumper` from the instrument context.
///
/// This allows cross-factor calculators to be registered once per
/// instrument type, with IDs resolved at calculation time from the
/// instrument's market dependencies.
pub(crate) trait BumperFactory: Send + Sync {
    /// Create a bumper for this factor, or None if the instrument
    /// doesn't expose the required dependency (e.g., no vol surface).
    fn create(&self, context: &MetricContext) -> Result<Option<Box<dyn FactorBumper>>>;
}
```

Then refactor `CrossFactorCalculator` to use factories:

```rust
pub struct CrossFactorCalculator {
    pub pair: CrossFactorPair,
    pub factory_a: Arc<dyn BumperFactory>,
    pub factory_b: Arc<dyn BumperFactory>,
}

impl MetricCalculator for CrossFactorCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        // Create bumpers from instrument context
        let bumper_a = match self.factory_a.create(context)? {
            Some(b) => b,
            None => return Ok(0.0), // factor not applicable
        };
        let bumper_b = match self.factory_b.create(context)? {
            Some(b) => b,
            None => return Ok(0.0),
        };
        // ... rest of four-corner repricing logic unchanged ...
    }
}
```

Example factory for rates (uses `market_dependencies()` to resolve curve IDs at runtime, matching the pattern in `GenericDfEndCalculator`):

```rust
/// Creates a RatesParallelBumper using the instrument's discount curve IDs.
pub(crate) struct RatesBumperFactory {
    pub bump_bp: f64,
}

impl BumperFactory for RatesBumperFactory {
    fn create(&self, context: &MetricContext) -> Result<Option<Box<dyn FactorBumper>>> {
        let deps = context.instrument.market_dependencies()?;
        let curve_ids = deps.curves.discount_curves.to_vec();
        if curve_ids.is_empty() {
            return Ok(None);
        }
        Ok(Some(Box::new(RatesParallelBumper {
            curve_ids,
            bump_bp: self.bump_bp,
        })))
    }
}

/// Creates a CreditParallelBumper using the instrument's hazard curve IDs.
pub(crate) struct CreditBumperFactory {
    pub bump_bp: f64,
}

impl BumperFactory for CreditBumperFactory {
    fn create(&self, context: &MetricContext) -> Result<Option<Box<dyn FactorBumper>>> {
        let deps = context.instrument.market_dependencies()?;
        let curve_ids = deps.curves.credit_curves.to_vec();
        if curve_ids.is_empty() {
            return Ok(None);
        }
        Ok(Some(Box::new(CreditParallelBumper {
            curve_ids,
            bump_bp: self.bump_bp,
        })))
    }
}
```

- [ ] **Step 2: Find registry setup files**

Run: `grep -r "register_metric\|register_metrics!" valuations/src/instruments/ --include="*.rs" -l`

This will show which files contain metric registrations per instrument type.

- [ ] **Step 3: Register cross-factor calculators for multi-factor instruments**

For each instrument type with multiple factor sensitivities, add registrations. Example for bonds (which have both rate and credit sensitivity):

```rust
// In the bond's metric registration file:
use crate::metrics::sensitivities::cross_factor::*;

registry.register_metric(
    MetricId::CrossGammaRatesCredit,
    Arc::new(CrossFactorCalculator {
        pair: CrossFactorPair::RatesCredit,
        factory_a: Arc::new(RatesBumperFactory { bump_bp: 1.0 }),
        factory_b: Arc::new(CreditBumperFactory { bump_bp: 1.0 }),
    }),
    &[InstrumentType::Bond],
);
```

Registration matrix (which pairs to register per instrument type):

| Instrument Type | Cross-Factor Pairs |
|---|---|
| Bond | RatesCredit |
| EquityOption | SpotVol, SpotCredit (if credit-linked) |
| FxOption | FxVol, FxRates |
| IRS | (none initially — single-factor) |
| CrossCurrencySwap | FxRates |
| ConvertibleBond | SpotVol, SpotCredit, RatesCredit |

- [ ] **Step 4: Run all tests**

Run: `cargo test -p finstack_valuations`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add valuations/src/metrics/sensitivities/cross_factor.rs valuations/src/instruments/
git commit -m "feat(metrics): register cross-factor calculators with runtime ID resolution"
```

---

### Task 9: Write end-to-end integration test for cross-factor attribution

**Files:**
- Create: `valuations/tests/cross_factor_attribution_tests.rs`

- [ ] **Step 1: Write integration test with a multi-factor instrument**

```rust
//! End-to-end integration tests for cross-factor P&L attribution.

use finstack_core::config::FinstackConfig;
use finstack_core::currency::Currency;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_valuations::attribution::{attribute_pnl_parallel, PnlAttribution};
use finstack_valuations::instruments::Instrument;
use finstack_valuations::instruments::rates::deposit::Deposit;
use std::sync::Arc;
use time::macros::date;

#[test]
fn single_factor_instrument_has_zero_cross_factor() {
    // A pure rate instrument (deposit) should have zero cross-factor P&L
    // since there's only one factor at play.
    let as_of_t0 = date!(2025 - 01 - 15);
    let as_of_t1 = date!(2025 - 01 - 16);

    let curve_t0 = DiscountCurve::builder("USD-OIS")
        .base_date(as_of_t0)
        .knots(vec![(0.0, 1.0), (1.0, 0.98)])
        .interp(InterpStyle::Linear)
        .build()
        .expect("curve");

    let curve_t1 = DiscountCurve::builder("USD-OIS")
        .base_date(as_of_t1)
        .knots(vec![(0.0, 1.0), (1.0, 0.97)])
        .interp(InterpStyle::Linear)
        .build()
        .expect("curve");

    let market_t0 = MarketContext::new().insert(curve_t0);
    let market_t1 = MarketContext::new().insert(curve_t1);

    let deposit = Arc::new(
        Deposit::builder()
            .id("DEP-1Y".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .start_date(as_of_t0)
            .maturity(date!(2026 - 01 - 15))
            .day_count(finstack_core::dates::DayCount::Act360)
            .discount_curve_id("USD-OIS".into())
            .build()
            .expect("deposit"),
    ) as Arc<dyn Instrument>;

    let config = FinstackConfig::default();
    let attr = attribute_pnl_parallel(
        &deposit, &market_t0, &market_t1, as_of_t0, as_of_t1, &config, None,
    ).unwrap();

    // Single-factor: no cross-factor terms
    assert_eq!(attr.cross_factor_pnl.amount(), 0.0,
        "Deposit (single-factor) should have zero cross-factor P&L");
    assert!(attr.cross_factor_detail.is_none(),
        "Deposit should have no cross-factor detail");
}

#[test]
fn cross_factor_detail_keys_use_standard_labels() {
    // Verify CrossFactorDetail by_pair keys use "Rates×Credit" format
    use finstack_valuations::attribution::CrossFactorDetail;
    use indexmap::IndexMap;

    let detail = CrossFactorDetail {
        total: Money::new(100.0, Currency::USD),
        by_pair: {
            let mut m = IndexMap::new();
            m.insert("Rates×Credit".to_string(), Money::new(100.0, Currency::USD));
            m
        },
    };

    assert!(detail.by_pair.contains_key("Rates×Credit"));
    assert_eq!(detail.by_pair.len(), 1);
}

// NOTE: The full multi-factor test (parallel_attribution_with_cross_factors_reduces_residual)
// requires a risky bond instrument with both rate AND credit curve sensitivity.
// This test should be added once a suitable multi-factor test instrument is available.
// The test should:
// 1. Build a risky bond with discount + hazard curves
// 2. Shift both curves between T0 and T1
// 3. Run parallel attribution
// 4. Assert: cross_factor_pnl != 0 (rates×credit interaction exists)
// 5. Assert: residual_pct < 2.0% (cross-terms captured the interaction)
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p finstack_valuations --test cross_factor_attribution_tests`
Expected: PASS (placeholder tests pass)

- [ ] **Step 3: Commit**

```bash
git add valuations/tests/cross_factor_attribution_tests.rs
git commit -m "test(attribution): add integration test scaffold for cross-factor attribution"
```

---

### Task 10: Update Taylor attribution to include cross-factor terms

**Files:**
- Modify: `valuations/src/attribution/taylor.rs`

The Taylor method already computes sensitivities via bump-and-reprice. Adding cross-factor terms follows the same pattern: compute `CrossGamma × Δf₁ × Δf₂` for each pair.

- [ ] **Step 1: Write failing test for Taylor cross-factor config**

```rust
#[test]
fn taylor_config_include_cross_factor_defaults_false() {
    let config = TaylorAttributionConfig::default();
    assert!(!config.include_cross_factor);
}

#[test]
fn taylor_config_cross_factor_deserializes() {
    let json = r#"{"include_gamma": true, "include_cross_factor": true}"#;
    let config: TaylorAttributionConfig = serde_json::from_str(json).unwrap();
    assert!(config.include_cross_factor);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p finstack_valuations taylor_config_include_cross`
Expected: FAIL — `include_cross_factor` field doesn't exist

- [ ] **Step 3: Add cross-factor config to `TaylorAttributionConfig`**

```rust
/// Include cross-factor interaction terms.
///
/// When true, computes mixed second derivatives for applicable factor
/// pairs and includes them in the explained P&L.
#[serde(default)]
pub include_cross_factor: bool,
```

Also update the `Default` impl to include:

```rust
include_cross_factor: false,
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p finstack_valuations taylor_config_include_cross`
Expected: PASS

- [ ] **Step 5: Add cross-factor computation loop to Taylor attribution**

After the single-factor sensitivity loop, add a cross-factor section that:
1. Guards on `config.include_cross_factor`
2. Iterates over `CrossFactorPair::ALL`
3. For each pair where both market moves are available
4. Computes the cross-gamma via four-corner repricing (reusing bumped markets from single-factor steps — the rate-bumped and credit-bumped markets are already constructed)
5. Adds `cross_gamma × Δf₁ × Δf₂` to `total_explained` and creates a `TaylorFactorResult` with `factor_name: pair.label()`

- [ ] **Step 6: Run tests**

Run: `cargo test -p finstack_valuations taylor`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add valuations/src/attribution/taylor.rs
git commit -m "feat(attribution): add cross-factor support to Taylor attribution"
```

---

## Summary

| Task | What | New Repricings | Files |
|------|------|---------------|-------|
| 1 | Ungate `central_mixed` | 0 | `finite_difference.rs` |
| 2 | Add 6 `CrossGamma*` MetricId constants | 0 | `ids.rs` |
| 3 | `CrossFactorPair` enum + `CrossFactorCalculator` | 0 | `cross_factor.rs`, `sensitivities/mod.rs` |
| 4 | Integration test scaffold for metrics | 0 | `tests/cross_factor_metrics_tests.rs` |
| 5 | `cross_factor_pnl` + `CrossFactorDetail` on PnlAttribution | 0 | `types.rs`, `mod.rs` |
| 6 | Wire into parallel attribution | +1 per pair | `parallel.rs` |
| 7 | Wire into metrics-based attribution | 0 | `metrics_based.rs`, `types.rs` |
| 8 | Runtime ID resolution + registry wiring | 0 | `cross_factor.rs`, instrument metric files |
| 9 | End-to-end integration tests | varies | `tests/cross_factor_attribution_tests.rs` |
| 10 | Wire into Taylor attribution | +4 per pair | `taylor.rs` |

**Incremental repricing cost for parallel attribution:** 1 extra repricing per cross-factor pair (only the "both at T₀" corner is new — single-factor steps already compute the other three values). For 3 active pairs in a multi-asset book, that's 3 extra repricings on top of the existing ~10.

**Backward compatibility:** Zero risk. `cross_factor_pnl` defaults to zero, `cross_factor_detail` defaults to `None`. Existing single-factor fields and residual computation are unchanged. The new fields only populate when cross-factor data exists.
