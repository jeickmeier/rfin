# PIK Structural Credit Pricing Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add Merton/KMV structural credit model, endogenous hazard rates, dynamic recovery, and Monte Carlo toggle exercise to price PIK bonds with path-dependent credit risk.

**Architecture:** Four composable model modules under `instruments/common/models/credit/`, one MC pricing engine under `bond/pricing/`, and Python bindings mirroring the Rust module structure. All new Rust code lives behind the existing `mc` feature flag (which gates `nalgebra` and the MC module).

**Tech Stack:** Rust (finstack-valuations), PyO3 (Python bindings), existing `Pcg64Rng`/`RandomNumberGenerator`, `norm_cdf`/`standard_normal_inv_cdf`, `NewtonSolver`/`BrentSolver`, `HazardCurve::builder()`, `poisson_inverse_cdf`.

**Design doc:** `docs/plans/2026-03-01-pik-structural-credit-design.md`

---

## Task 1: Merton Model — Core Types and Distance-to-Default

**Files:**

- Create: `finstack/valuations/src/instruments/common/models/credit/mod.rs`
- Create: `finstack/valuations/src/instruments/common/models/credit/merton.rs`
- Modify: `finstack/valuations/src/instruments/common/models/mod.rs` (add `pub mod credit;`)

**Context:** The `models/` directory currently contains `closed_form/`, `correlation/`, `monte_carlo/` (behind `mc` feature), `trees/`, and `volatility/`. We add `credit/` at the same level. The Merton model does NOT require the `mc` feature for its analytical methods (DD, PD, `to_hazard_curve`); only `simulate_paths` will be feature-gated.

**Key imports to reuse:**

- `finstack_core::math::special_functions::{norm_cdf, standard_normal_inv_cdf}`
- `finstack_core::math::solver::{NewtonSolver, BrentSolver, Solver}`
- `finstack_core::math::random::{Pcg64Rng, RandomNumberGenerator, poisson_inverse_cdf}`
- `finstack_core::market_data::term_structures::HazardCurve`

### Step 1: Write failing tests for Merton DD and PD

Create the test file with known-value tests for distance-to-default and default probability. Use textbook values: V₀=100, B=80, σ=0.2, r=0.05, T=1 → DD=(ln(100/80)+(0.05-0.02)×1)/(0.2×1)=1.365, PD=N(-1.365)≈0.086.

```rust
// In merton.rs #[cfg(test)] mod tests

#[test]
fn dd_textbook_values() {
    let m = MertonModel::new(100.0, 0.20, 80.0, 0.05).unwrap();
    let dd = m.distance_to_default(1.0);
    assert!((dd - 1.365).abs() < 0.01, "DD={dd}");
}

#[test]
fn pd_textbook_values() {
    let m = MertonModel::new(100.0, 0.20, 80.0, 0.05).unwrap();
    let pd = m.default_probability(1.0);
    assert!((pd - 0.086).abs() < 0.01, "PD={pd}");
}

#[test]
fn zero_vol_means_no_default_when_solvent() {
    let m = MertonModel::new(100.0, 1e-10, 80.0, 0.05).unwrap();
    let pd = m.default_probability(1.0);
    assert!(pd < 1e-6, "Zero vol, solvent → PD≈0, got {pd}");
}

#[test]
fn pd_increases_with_vol() {
    let m_low = MertonModel::new(100.0, 0.10, 80.0, 0.05).unwrap();
    let m_high = MertonModel::new(100.0, 0.40, 80.0, 0.05).unwrap();
    assert!(m_high.default_probability(1.0) > m_low.default_probability(1.0));
}

#[test]
fn pd_increases_with_leverage() {
    let m_low = MertonModel::new(100.0, 0.20, 60.0, 0.05).unwrap();
    let m_high = MertonModel::new(100.0, 0.20, 95.0, 0.05).unwrap();
    assert!(m_high.default_probability(1.0) > m_low.default_probability(1.0));
}
```

### Step 2: Run tests to verify they fail

Run: `cargo test -p finstack-valuations --lib instruments::common::models::credit::merton -- --nocapture 2>&1 | head -20`

Expected: Compilation error (module doesn't exist yet).

### Step 3: Implement core types and DD/PD

Implement `MertonModel` struct with `AssetDynamics` and `BarrierType` enums. Implement `new()`, `distance_to_default()`, `default_probability()`, `implied_spread()`. For `BarrierType::Terminal`, use the classic Merton formula. For `BarrierType::FirstPassage`, use the Black-Cox closed-form:

```
PD_fp(T) = N(-DD) + (V/B)^(-2(r-q)/σ²) × N(-DD + 2(r-q)T/(σ√T))
```

Wire up the module: add `pub mod credit;` to `models/mod.rs`, create `credit/mod.rs` with `pub mod merton;`.

### Step 4: Run tests to verify they pass

Run: `cargo test -p finstack-valuations --lib instruments::common::models::credit::merton -- --nocapture`

Expected: All 5 tests pass.

### Step 5: Commit

```bash
git add finstack/valuations/src/instruments/common/models/credit/
git add finstack/valuations/src/instruments/common/models/mod.rs
git commit -m "feat(credit): add Merton structural model with DD and PD"
```

---

## Task 2: Merton Model — KMV Calibration and CDS Spread Calibration

**Files:**

- Modify: `finstack/valuations/src/instruments/common/models/credit/merton.rs`

**Context:** `from_equity()` solves a 2×2 nonlinear system. Use the existing `NewtonSolver` for the iterative solve (it supports `solve_with_derivative`). `from_cds_spread()` uses `BrentSolver` to find σ_V matching a target spread. `credit_grades()` uses the CreditGrades closed-form with barrier uncertainty.

### Step 1: Write failing tests for calibration

```rust
#[test]
fn from_equity_recovers_known_values() {
    // V=100, σ_V=0.20, B=80, r=0.05, T=1
    // Equity E = V·N(d1) - B·e^(-rT)·N(d2)
    // Compute E and σ_E from known asset params, then recover them
    let m_known = MertonModel::new(100.0, 0.20, 80.0, 0.05).unwrap();
    let (equity, equity_vol) = m_known.implied_equity(1.0);
    let m_calibrated = MertonModel::from_equity(equity, equity_vol, 80.0, 0.05, 1.0).unwrap();
    assert!((m_calibrated.asset_value() - 100.0).abs() < 0.5);
    assert!((m_calibrated.asset_vol() - 0.20).abs() < 0.01);
}

#[test]
fn from_cds_spread_roundtrips() {
    let m = MertonModel::new(100.0, 0.25, 80.0, 0.04).unwrap();
    let spread = m.implied_spread(5.0, 0.40);
    let m2 = MertonModel::from_cds_spread(spread * 10_000.0, 0.40, 80.0, 0.04, 5.0).unwrap();
    assert!((m2.asset_vol() - 0.25).abs() < 0.02);
}
```

### Step 2: Run tests, verify failure

Run: `cargo test -p finstack-valuations --lib instruments::common::models::credit::merton::tests::from_equity -- --nocapture`

Expected: Method not found.

### Step 3: Implement calibration methods

- `implied_equity(T) -> (equity_value, equity_vol)`: forward calculation from known asset params.
- `from_equity()`: iterative Newton solve. Start with V₀ = E + B, σ_V₀ = σ_E × E / V₀. Iterate until convergence.
- `from_cds_spread()`: Brent bracket [0.01, 2.0] for σ_V, target `implied_spread(σ_V) = target`.
- `credit_grades()`: CreditGrades construction with barrier uncertainty parameter.

### Step 4: Run tests, verify pass

Run: `cargo test -p finstack-valuations --lib instruments::common::models::credit::merton -- --nocapture`

Expected: All tests pass.

### Step 5: Commit

```bash
git add finstack/valuations/src/instruments/common/models/credit/merton.rs
git commit -m "feat(credit): add KMV, CDS spread, and CreditGrades calibration"
```

---

## Task 3: Merton Model — Hazard Curve Generation (Mode A)

**Files:**

- Modify: `finstack/valuations/src/instruments/common/models/credit/merton.rs`

**Context:** `to_hazard_curve()` converts the structural model into a `HazardCurve` compatible with all existing engines (HazardBondEngine, CDS pricer). This is Mode A from the design. It computes forward default probabilities at a tenor grid and backs out piecewise-constant hazard rates: `λ(t) = -ln(S(t+dt)/S(t))/dt`.

### Step 1: Write failing tests

```rust
#[test]
fn to_hazard_curve_survival_matches_pd() {
    let m = MertonModel::new(100.0, 0.25, 80.0, 0.04).unwrap();
    let base = Date::from_calendar_date(2026, Month::March, 1).unwrap();
    let hc = m.to_hazard_curve("TEST", base, &[1.0, 3.0, 5.0, 7.0, 10.0], 0.40).unwrap();
    // Survival at 5Y should match 1 - PD(5)
    let sp5 = hc.sp(5.0);
    let pd5 = m.default_probability(5.0);
    assert!((sp5 - (1.0 - pd5)).abs() < 0.02, "sp5={sp5}, 1-pd5={}", 1.0 - pd5);
}

#[test]
fn to_hazard_curve_plugs_into_hazard_bond_engine() {
    // Build a bond, price with generated hazard curve, verify PV < risk-free PV
    let m = MertonModel::new(100.0, 0.30, 80.0, 0.04).unwrap();
    let base = Date::from_calendar_date(2026, Month::March, 1).unwrap();
    let hc = m.to_hazard_curve("USD-CREDIT", base, &[1.0, 3.0, 5.0], 0.40).unwrap();
    // ... build bond, create MarketContext, price with HazardBondEngine
    // Assert: PV_hazard < PV_riskfree
}
```

### Step 2: Run tests, verify failure

### Step 3: Implement `to_hazard_curve`

- Compute `S(t) = 1 - PD(t)` at each tenor.
- Back out piecewise-constant λ: `λ_i = -ln(S(t_{i+1})/S(t_i)) / (t_{i+1} - t_i)`.
- Use `HazardCurve::builder(id).base_date(base).knots(knots).recovery_rate(recovery).build()`.

### Step 4: Run tests, verify pass

### Step 5: Commit

```bash
git commit -m "feat(credit): add Merton to_hazard_curve for Mode A integration"
```

---

## Task 4: Merton Model — Path Simulation (Mode B)

**Files:**

- Modify: `finstack/valuations/src/instruments/common/models/credit/merton.rs`

**Context:** This is feature-gated behind `#[cfg(feature = "mc")]`. Uses `Pcg64Rng` and `RandomNumberGenerator` trait. Supports GBM, jump-diffusion (using `poisson_inverse_cdf`), and antithetic variates.

### Step 1: Write failing tests

```rust
#[cfg(feature = "mc")]
#[test]
fn simulate_paths_deterministic_with_seed() {
    let m = MertonModel::new(100.0, 0.20, 80.0, 0.05).unwrap();
    let mut rng1 = Pcg64Rng::new(42);
    let mut rng2 = Pcg64Rng::new(42);
    let paths1 = m.simulate_paths(100, 60, 5.0, &mut rng1, false);
    let paths2 = m.simulate_paths(100, 60, 5.0, &mut rng2, false);
    assert_eq!(paths1.asset_values[0], paths2.asset_values[0]);
}

#[cfg(feature = "mc")]
#[test]
fn simulate_paths_gbm_mean_converges() {
    let m = MertonModel::new(100.0, 0.20, 80.0, 0.05).unwrap();
    let mut rng = Pcg64Rng::new(42);
    let paths = m.simulate_paths(50_000, 60, 5.0, &mut rng, true);
    let mean_terminal: f64 = paths.asset_values.iter()
        .map(|p| p.last().unwrap())
        .sum::<f64>() / paths.num_paths as f64;
    let expected = 100.0 * (0.05_f64 * 5.0).exp();
    assert!((mean_terminal - expected).abs() / expected < 0.02);
}

#[cfg(feature = "mc")]
#[test]
fn jump_diffusion_produces_fatter_tails() {
    let m_gbm = MertonModel::new(100.0, 0.20, 80.0, 0.05).unwrap();
    let m_jd = MertonModel::new_with_dynamics(
        100.0, 0.20, 80.0, 0.05, 0.0,
        BarrierType::FirstPassage { barrier_growth_rate: 0.05 },
        AssetDynamics::JumpDiffusion {
            jump_intensity: 0.5, jump_mean: -0.05, jump_vol: 0.10
        },
    ).unwrap();
    // JD should have more extreme outcomes
}
```

### Step 2: Run tests, verify failure

Run: `cargo test -p finstack-valuations --features mc --lib instruments::common::models::credit::merton -- simulate --nocapture`

### Step 3: Implement `simulate_paths`

GBM step: `V(t+dt) = V(t) × exp((r-q-σ²/2)dt + σ√dt × Z)`

Jump-diffusion: after GBM step, sample `n = poisson_inverse_cdf(λ_J × dt, rng.uniform())` jumps. For each jump: `V *= exp(μ_J - σ_J²/2 + σ_J × rng.normal(0,1))`.

Antithetic: for each path, also generate the path with `-Z` normals, average the two.

### Step 4: Run tests, verify pass

### Step 5: Commit

```bash
git commit -m "feat(credit): add Merton MC path simulation with GBM and jump-diffusion"
```

---

## Task 5: Endogenous Hazard Spec

**Files:**

- Create: `finstack/valuations/src/instruments/common/models/credit/endogenous_hazard.rs`
- Modify: `finstack/valuations/src/instruments/common/models/credit/mod.rs`

### Step 1: Write failing tests

```rust
#[test]
fn power_law_at_base_leverage_returns_base_hazard() {
    let spec = EndogenousHazardSpec::power_law(0.10, 1.5, 2.5);
    assert!((spec.hazard_at_leverage(1.5) - 0.10).abs() < 1e-10);
}

#[test]
fn power_law_increases_with_leverage() {
    let spec = EndogenousHazardSpec::power_law(0.10, 1.5, 2.5);
    let h_low = spec.hazard_at_leverage(1.5);
    let h_high = spec.hazard_at_leverage(2.0);
    assert!(h_high > h_low);
}

#[test]
fn exponential_at_base_returns_base() {
    let spec = EndogenousHazardSpec::exponential(0.10, 1.5, 5.0);
    assert!((spec.hazard_at_leverage(1.5) - 0.10).abs() < 1e-10);
}

#[test]
fn pik_accrual_increases_hazard() {
    let spec = EndogenousHazardSpec::power_law(0.10, 1.5, 2.5);
    let h_before = spec.hazard_after_pik_accrual(100.0, 100.0, 66.67); // L=1.5
    let h_after = spec.hazard_after_pik_accrual(100.0, 120.0, 66.67);  // L=1.8
    assert!(h_after > h_before);
}
```

### Step 2: Run tests, verify failure

### Step 3: Implement `EndogenousHazardSpec`

Implement struct with `LeverageHazardMap` enum (PowerLaw, Exponential, MertonImplied, Tabular). Each map variant computes `hazard_at_leverage(L)`. `hazard_after_pik_accrual` is a convenience: `L = accreted_notional / asset_value`, then delegates to `hazard_at_leverage`.

### Step 4: Run tests, verify pass

### Step 5: Commit

```bash
git commit -m "feat(credit): add endogenous hazard spec with power law and exponential maps"
```

---

## Task 6: Dynamic Recovery Spec

**Files:**

- Create: `finstack/valuations/src/instruments/common/models/credit/dynamic_recovery.rs`
- Modify: `finstack/valuations/src/instruments/common/models/credit/mod.rs`

### Step 1: Write failing tests

```rust
#[test]
fn constant_recovery_unchanged() {
    let spec = DynamicRecoverySpec::constant(0.40);
    assert!((spec.recovery_at_notional(150.0) - 0.40).abs() < 1e-10);
}

#[test]
fn inverse_linear_declines_with_notional() {
    let spec = DynamicRecoverySpec::inverse_linear(0.40, 100.0);
    let r_at_par = spec.recovery_at_notional(100.0);
    let r_at_150 = spec.recovery_at_notional(150.0);
    assert!((r_at_par - 0.40).abs() < 1e-10);
    assert!((r_at_150 - 0.40 * 100.0 / 150.0).abs() < 1e-10);
    assert!(r_at_150 < r_at_par);
}

#[test]
fn floored_inverse_respects_floor() {
    let spec = DynamicRecoverySpec::floored_inverse(0.40, 100.0, 0.15);
    let r_extreme = spec.recovery_at_notional(1000.0);
    assert!((r_extreme - 0.15).abs() < 1e-10, "Should be floored at 15%, got {r_extreme}");
}

#[test]
fn linear_decline_sensitivity() {
    let spec = DynamicRecoverySpec::linear_decline(0.40, 100.0, 0.5, 0.10);
    // At N=120: R = 0.40 × (1 - 0.5 × (1.2 - 1)) = 0.40 × 0.90 = 0.36
    let r = spec.recovery_at_notional(120.0);
    assert!((r - 0.36).abs() < 1e-6, "Got {r}");
}
```

### Step 2: Run tests, verify failure

### Step 3: Implement `DynamicRecoverySpec`

Implement struct with `RecoveryModel` enum (Constant, InverseLinear, InversePower, FlooredInverse, LinearDecline). Each computes `recovery_at_notional(N)`. All results clamped to `[0, base_recovery]`.

### Step 4: Run tests, verify pass

### Step 5: Commit

```bash
git commit -m "feat(credit): add dynamic recovery spec with leverage-dependent models"
```

---

## Task 7: Toggle Exercise Models

**Files:**

- Create: `finstack/valuations/src/instruments/common/models/credit/toggle_exercise.rs`
- Modify: `finstack/valuations/src/instruments/common/models/credit/mod.rs`

### Step 1: Write failing tests

```rust
#[test]
fn threshold_piks_above_threshold() {
    let model = ToggleExerciseModel::threshold(
        CreditStateVariable::HazardRate, 0.15, ThresholdDirection::Above
    );
    let mut rng = Pcg64Rng::new(42);
    let state_low = CreditState { hazard_rate: 0.10, ..Default::default() };
    let state_high = CreditState { hazard_rate: 0.20, ..Default::default() };
    assert!(!model.should_pik(&state_low, &mut rng));
    assert!(model.should_pik(&state_high, &mut rng));
}

#[test]
fn stochastic_toggle_probability_increases_with_hazard() {
    let model = ToggleExerciseModel::stochastic(
        CreditStateVariable::HazardRate, -3.0, 20.0
    );
    let mut rng = Pcg64Rng::new(42);
    // Run 10k samples at λ=0.10 and λ=0.20
    let count_low: usize = (0..10_000)
        .filter(|_| {
            let state = CreditState { hazard_rate: 0.10, ..Default::default() };
            model.should_pik(&state, &mut rng)
        }).count();
    let count_high: usize = (0..10_000)
        .filter(|_| {
            let state = CreditState { hazard_rate: 0.20, ..Default::default() };
            model.should_pik(&state, &mut rng)
        }).count();
    assert!(count_high > count_low, "Higher hazard should have more PIK elections");
}

#[test]
fn pik_fraction_returns_0_or_1_for_threshold() {
    let model = ToggleExerciseModel::threshold(
        CreditStateVariable::HazardRate, 0.15, ThresholdDirection::Above
    );
    let mut rng = Pcg64Rng::new(42);
    let state = CreditState { hazard_rate: 0.20, ..Default::default() };
    assert!((model.pik_fraction(&state, &mut rng) - 1.0).abs() < 1e-10);
}
```

### Step 2: Run tests, verify failure

### Step 3: Implement toggle models

- `CreditState` struct with `Default` derive.
- `ThresholdToggle`: simple comparison against threshold.
- `StochasticToggle`: `P(PIK) = 1 / (1 + exp(-(a + b×state)))`, sample `rng.uniform() < p`.
- `OptimalToggle`: stub with `todo!("nested MC")` for now — mark with `// TODO: Task 12` comment. Implement the threshold and stochastic models fully.
- `pik_fraction()`: delegates to `should_pik()` returning 0.0 or 1.0 for threshold/optimal, returns the probability itself for stochastic.

### Step 4: Run tests, verify pass

### Step 5: Commit

```bash
git commit -m "feat(credit): add toggle exercise models (threshold, stochastic)"
```

---

## Task 8: Monte Carlo PIK Pricing Engine — Config and Result Types

**Files:**

- Create: `finstack/valuations/src/instruments/fixed_income/bond/pricing/merton_mc_engine.rs`
- Modify: `finstack/valuations/src/instruments/fixed_income/bond/pricing/mod.rs`

**Context:** This entire module is behind `#[cfg(feature = "mc")]`. The engine struct, config, and result types go here. The simulation loop goes in Task 9.

### Step 1: Write the types

Define `MertonMcConfig`, `MertonMcResult`, `PathStatistics`, and the `MertonMcEngine` struct. Include builder methods on `MertonMcConfig` for ergonomic construction.

Add to `bond/pricing/mod.rs`:

```rust
/// Merton Monte Carlo engine for PIK bonds with structural credit risk
#[cfg(feature = "mc")]
pub mod merton_mc_engine;
```

### Step 2: Verify compilation

Run: `cargo check -p finstack-valuations --features mc`

### Step 3: Commit

```bash
git commit -m "feat(credit): add MertonMcEngine config and result types"
```

---

## Task 9: Monte Carlo PIK Pricing Engine — Simulation Loop

**Files:**

- Modify: `finstack/valuations/src/instruments/fixed_income/bond/pricing/merton_mc_engine.rs`

**Context:** This is the core simulation. It orchestrates Merton path simulation, endogenous hazard, dynamic recovery, and toggle exercise. Each path: evolve assets, check default (first-passage), at coupon dates compute credit state and toggle decision, accrete PIK or record cash flow, discount and aggregate.

### Step 1: Write failing integration tests

```rust
#[cfg(feature = "mc")]
#[test]
fn cash_bond_mc_price_near_hazard_engine() {
    // Cash-pay bond priced via MC should be close to HazardBondEngine price
    // when endogenous hazard is disabled (constant λ)
    // Tolerance: within 2% (MC noise)
}

#[cfg(feature = "mc")]
#[test]
fn pik_bond_higher_price_with_growing_notional() {
    // Full-PIK bond with recovery on accreted notional should have
    // higher PV than cash bond when using same coupon rate
    // (because recovery leg benefits from accreted notional)
}

#[cfg(feature = "mc")]
#[test]
fn endogenous_hazard_lowers_pik_price() {
    // PIK bond with endogenous hazard (λ increases with leverage)
    // should price lower than PIK without it
}

#[cfg(feature = "mc")]
#[test]
fn dynamic_recovery_lowers_pik_price() {
    // PIK bond with declining recovery should price lower
}

#[cfg(feature = "mc")]
#[test]
fn toggle_spread_between_cash_and_pik() {
    // Toggle bond price should be between cash and full-PIK
}

#[cfg(feature = "mc")]
#[test]
fn mc_is_deterministic_with_seed() {
    // Same seed → same result
}
```

### Step 2: Run tests, verify failure

Run: `cargo test -p finstack-valuations --features mc --lib instruments::fixed_income::bond::pricing::merton_mc_engine -- --nocapture`

### Step 3: Implement simulation loop

Follow the pseudocode from the design doc. Key implementation details:

- Build coupon schedule from `Bond`'s `CashflowSpec` or `custom_cashflows`.
- For each path, create `Pcg64Rng::new_with_stream(seed, path_idx as u64)` for independence.
- Track `outstanding_notional` per path (starts at `bond.notional`).
- At each time step: evolve V(t), check barrier, at coupon dates: compute state → toggle → accrete or record CF.
- Terminal: pay accreted notional if survived, recovery × notional if defaulted.
- Aggregate: mean PV across paths, compute EL/UL/ES.

### Step 4: Run tests, verify pass

### Step 5: Commit

```bash
git commit -m "feat(credit): implement MC PIK pricing simulation loop"
```

---

## Task 10: Bond Integration — `price_merton_mc` Method

**Files:**

- Modify: `finstack/valuations/src/instruments/fixed_income/bond/mod.rs`

**Context:** Add a public method on `Bond` that delegates to the MC engine. Feature-gated behind `mc`.

### Step 1: Write failing test

```rust
#[cfg(feature = "mc")]
#[test]
fn bond_price_merton_mc_api() {
    let bond = build_test_bond(issue, maturity);
    let merton = MertonModel::new(200.0, 0.25, 100.0, 0.04).unwrap();
    let config = MertonMcConfig::new(merton).num_paths(1000).seed(42);
    let market = MarketContext::new().insert_discount(disc);
    let result = bond.price_merton_mc(&config, &market, issue).unwrap();
    assert!(result.clean_price_pct > 0.0 && result.clean_price_pct < 200.0);
}
```

### Step 2: Run test, verify failure

### Step 3: Implement `price_merton_mc` on Bond

```rust
#[cfg(feature = "mc")]
impl Bond {
    pub fn price_merton_mc(
        &self,
        config: &MertonMcConfig,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<MertonMcResult> {
        MertonMcEngine::price(self, config, market, as_of)
    }
}
```

### Step 4: Run test, verify pass

### Step 5: Commit

```bash
git commit -m "feat(credit): add Bond::price_merton_mc integration method"
```

---

## Task 11: Python Bindings — Merton Model

**Files:**

- Create: `finstack-py/src/valuations/instruments/credit/mod.rs`
- Create: `finstack-py/src/valuations/instruments/credit/merton.rs`
- Modify: `finstack-py/src/valuations/instruments/mod.rs` (add `mod credit;`, register)
- Create: `finstack-py/finstack/valuations/instruments/credit/__init__.py`
- Create: `finstack-py/finstack/valuations/instruments/credit/merton.pyi`

**Context:** Follow the existing PyO3 pattern from `credit_derivatives/cds.rs`. Wrapper struct with `inner: RustMertonModel`, classmethods for construction, property methods for outputs.

### Step 1: Implement Python wrapper

```rust
#[pyclass(module = "finstack.valuations.instruments.credit", name = "MertonModel", frozen)]
pub struct PyMertonModel {
    pub(crate) inner: Arc<RustMertonModel>,
}

#[pymethods]
impl PyMertonModel {
    #[new]
    fn new(asset_value: f64, asset_vol: f64, debt_barrier: f64,
           risk_free_rate: f64, ...) -> PyResult<Self> { ... }

    #[classmethod]
    fn from_equity(...) -> PyResult<Self> { ... }

    #[classmethod]
    fn from_cds_spread(...) -> PyResult<Self> { ... }

    fn distance_to_default(&self, horizon: Option<f64>) -> f64 { ... }
    fn default_probability(&self, horizon: Option<f64>) -> f64 { ... }
    fn implied_spread(&self, horizon: f64, recovery: f64) -> f64 { ... }

    fn to_hazard_curve(&self, curve_id: &str, base_date: NaiveDate,
                       tenors: Option<Vec<f64>>, recovery: Option<f64>)
        -> PyResult<PyHazardCurve> { ... }
}
```

Also wrap `AssetDynamics`, `BarrierType` as simple pyclass enums with classmethods.

### Step 2: Write type stubs (.pyi)

### Step 3: Write Python test

```python
def test_merton_dd():
    m = MertonModel(asset_value=100, asset_vol=0.20, debt_barrier=80, risk_free_rate=0.05)
    dd = m.distance_to_default(1.0)
    assert abs(dd - 1.365) < 0.01

def test_from_equity_roundtrip():
    m = MertonModel.from_equity(equity_value=25.0, equity_vol=0.50,
                                 total_debt=80.0, risk_free_rate=0.05)
    assert m.distance_to_default(1.0) > 0

def test_to_hazard_curve():
    m = MertonModel(asset_value=100, asset_vol=0.25, debt_barrier=80, risk_free_rate=0.04)
    hc = m.to_hazard_curve("TEST", date(2026, 3, 1), recovery=0.40)
    assert hc is not None
```

### Step 4: Run Python tests

Run: `cd finstack-py && uv run pytest tests/ -k merton -v`

### Step 5: Commit

```bash
git commit -m "feat(py): add MertonModel Python bindings with type stubs"
```

---

## Task 12: Python Bindings — Credit Model Specs

**Files:**

- Create: `finstack-py/src/valuations/instruments/credit/endogenous_hazard.rs`
- Create: `finstack-py/src/valuations/instruments/credit/dynamic_recovery.rs`
- Create: `finstack-py/src/valuations/instruments/credit/toggle_exercise.rs`
- Create: `finstack-py/finstack/valuations/instruments/credit/endogenous_hazard.pyi`
- Create: `finstack-py/finstack/valuations/instruments/credit/dynamic_recovery.pyi`
- Create: `finstack-py/finstack/valuations/instruments/credit/toggle_exercise.pyi`
- Modify: `finstack-py/src/valuations/instruments/credit/mod.rs`

### Step 1: Implement wrappers

Follow the same PyO3 pattern. Each spec gets classmethods for its variants:

```python
# EndogenousHazardSpec
EndogenousHazardSpec.power_law(base_hazard=0.10, base_leverage=1.5, exponent=2.5)
EndogenousHazardSpec.exponential(base_hazard=0.10, base_leverage=1.5, sensitivity=5.0)

# DynamicRecoverySpec
DynamicRecoverySpec.constant(recovery=0.40)
DynamicRecoverySpec.floored_inverse(base_recovery=0.40, base_notional=100e6, floor=0.15)

# ToggleExerciseModel
ToggleExerciseModel.threshold(variable="hazard_rate", threshold=0.15)
ToggleExerciseModel.stochastic(variable="hazard_rate", intercept=-3.0, sensitivity=20.0)
```

### Step 2: Write type stubs

### Step 3: Write Python tests

### Step 4: Run tests

### Step 5: Commit

```bash
git commit -m "feat(py): add endogenous hazard, dynamic recovery, toggle exercise bindings"
```

---

## Task 13: Python Bindings — MC Config and Bond.price_merton_mc

**Files:**

- Create: `finstack-py/src/valuations/instruments/credit/mc_config.rs`
- Create: `finstack-py/finstack/valuations/instruments/credit/mc_config.pyi`
- Modify: `finstack-py/src/valuations/instruments/fixed_income/bond.rs` (add `price_merton_mc` method)
- Modify: `finstack-py/finstack/valuations/instruments/fixed_income/bond.pyi`

### Step 1: Implement `PyMertonMcConfig` and `PyMertonMcResult`

### Step 2: Add `price_merton_mc` to `PyBond`

```rust
#[pymethods]
impl PyBond {
    fn price_merton_mc(&self, config: &PyMertonMcConfig,
                        market: &PyMarketContext,
                        as_of: NaiveDate) -> PyResult<PyMertonMcResult> { ... }
}
```

### Step 3: Write end-to-end Python test

```python
def test_pik_spread_differential():
    """The primary use case: compare cash vs PIK vs toggle spreads."""
    merton = MertonModel.from_equity(
        equity_value=500e6, equity_vol=0.40,
        total_debt=800e6, risk_free_rate=0.04
    )
    endo = EndogenousHazardSpec.power_law(base_hazard=0.10, base_leverage=1.6, exponent=2.5)
    dyn_rec = DynamicRecoverySpec.floored_inverse(base_recovery=0.40, base_notional=100, floor=0.15)
    toggle = ToggleExerciseModel.threshold(variable="hazard_rate", threshold=0.15)

    config_base = MertonMcConfig(merton=merton, endogenous_hazard=endo,
                                  dynamic_recovery=dyn_rec, num_paths=5000, seed=42)
    config_toggle = MertonMcConfig(merton=merton, endogenous_hazard=endo,
                                    dynamic_recovery=dyn_rec, toggle_model=toggle,
                                    num_paths=5000, seed=42)

    cash_result = cash_bond.price_merton_mc(config_base, market, as_of)
    pik_result = pik_bond.price_merton_mc(config_base, market, as_of)
    toggle_result = toggle_bond.price_merton_mc(config_toggle, market, as_of)

    # PIK should price lower than cash (endogenous effects)
    assert pik_result.clean_price_pct < cash_result.clean_price_pct
    # Toggle should be between cash and PIK
    assert pik_result.clean_price_pct < toggle_result.clean_price_pct < cash_result.clean_price_pct
```

### Step 4: Run test

Run: `cd finstack-py && uv run pytest tests/ -k pik_spread -v`

### Step 5: Commit

```bash
git commit -m "feat(py): add MertonMcConfig bindings and Bond.price_merton_mc"
```

---

## Task 14: Optimal Toggle Exercise (Nested MC)

**Files:**

- Modify: `finstack/valuations/src/instruments/common/models/credit/toggle_exercise.rs`

**Context:** This replaces the `todo!()` stub from Task 7. The optimal toggle runs a small nested MC at each decision point to estimate equity value under cash vs PIK. Computationally expensive — use reduced path count (100-500) for the nested simulation.

### Step 1: Write failing test

```rust
#[cfg(feature = "mc")]
#[test]
fn optimal_toggle_prefers_pik_when_stressed() {
    // When the firm is near default (low DD), optimal exercise should prefer PIK
    // (preserving cash helps survival)
}
```

### Step 2: Implement nested MC

At each coupon date, estimate:
- `V_cash = E[max(V(T) - B(T), 0) | pay cash now]` (reduced future debt service)
- `V_pik = E[max(V(T) - B(T), 0) | PIK now]` (preserves cash, but higher barrier)

PIK if `V_pik > V_cash`.

### Step 3: Run tests

### Step 4: Commit

```bash
git commit -m "feat(credit): implement optimal toggle exercise with nested MC"
```

---

## Task 15: Convergence and Property Tests

**Files:**

- Create: `finstack/valuations/tests/instruments/bond/merton_mc_convergence.rs`

**Context:** Verify MC convergence, monotonicity properties, and boundary conditions.

### Step 1: Write property tests

```rust
#[test]
fn mc_converges_as_paths_increase() {
    // Price with 1k, 5k, 25k paths. Standard error should decrease as 1/√N.
}

#[test]
fn higher_asset_vol_increases_spread_differential() {
    // Higher σ_V → bigger difference between cash and PIK spread
}

#[test]
fn zero_pik_coupon_matches_zero_coupon_bond() {
    // Full-PIK bond with 0% coupon = zero-coupon bond
}

#[test]
fn no_endogenous_no_dynamic_recovery_matches_standard() {
    // MC with constant hazard and constant recovery ≈ HazardBondEngine result
}
```

### Step 2: Run all tests

Run: `cargo test -p finstack-valuations --features mc -- merton --nocapture`

### Step 3: Commit

```bash
git commit -m "test(credit): add convergence and property tests for Merton MC"
```

---

## Task 16: Python Parity Tests

**Files:**

- Create: `finstack-py/tests/parity/test_merton_mc_parity.py`

### Step 1: Write parity tests

```python
def test_merton_dd_parity():
    """Python DD matches Rust DD."""

def test_mc_price_parity():
    """Python MC result matches Rust MC result (same seed)."""

def test_mc_determinism():
    """Same seed in Python produces identical results."""
```

### Step 2: Run

Run: `cd finstack-py && uv run pytest tests/parity/test_merton_mc_parity.py -v`

### Step 3: Commit

```bash
git commit -m "test(py): add Merton MC Python parity tests"
```

---

## Summary: File Creation/Modification Map

### New files (Rust)

| File | Task |
|------|------|
| `finstack/valuations/src/instruments/common/models/credit/mod.rs` | 1 |
| `finstack/valuations/src/instruments/common/models/credit/merton.rs` | 1-4 |
| `finstack/valuations/src/instruments/common/models/credit/endogenous_hazard.rs` | 5 |
| `finstack/valuations/src/instruments/common/models/credit/dynamic_recovery.rs` | 6 |
| `finstack/valuations/src/instruments/common/models/credit/toggle_exercise.rs` | 7, 14 |
| `finstack/valuations/src/instruments/fixed_income/bond/pricing/merton_mc_engine.rs` | 8-9 |
| `finstack/valuations/tests/instruments/bond/merton_mc_convergence.rs` | 15 |

### New files (Python bindings)

| File | Task |
|------|------|
| `finstack-py/src/valuations/instruments/credit/mod.rs` | 11 |
| `finstack-py/src/valuations/instruments/credit/merton.rs` | 11 |
| `finstack-py/src/valuations/instruments/credit/endogenous_hazard.rs` | 12 |
| `finstack-py/src/valuations/instruments/credit/dynamic_recovery.rs` | 12 |
| `finstack-py/src/valuations/instruments/credit/toggle_exercise.rs` | 12 |
| `finstack-py/src/valuations/instruments/credit/mc_config.rs` | 13 |
| `finstack-py/finstack/valuations/instruments/credit/__init__.py` | 11 |
| `finstack-py/finstack/valuations/instruments/credit/merton.pyi` | 11 |
| `finstack-py/finstack/valuations/instruments/credit/endogenous_hazard.pyi` | 12 |
| `finstack-py/finstack/valuations/instruments/credit/dynamic_recovery.pyi` | 12 |
| `finstack-py/finstack/valuations/instruments/credit/toggle_exercise.pyi` | 12 |
| `finstack-py/finstack/valuations/instruments/credit/mc_config.pyi` | 13 |
| `finstack-py/tests/parity/test_merton_mc_parity.py` | 16 |

### Modified files

| File | Task | Change |
|------|------|--------|
| `finstack/valuations/src/instruments/common/models/mod.rs` | 1 | Add `pub mod credit;` |
| `finstack/valuations/src/instruments/fixed_income/bond/pricing/mod.rs` | 8 | Add `#[cfg(feature = "mc")] pub mod merton_mc_engine;` |
| `finstack/valuations/src/instruments/fixed_income/bond/mod.rs` | 10 | Add `price_merton_mc` method |
| `finstack-py/src/valuations/instruments/mod.rs` | 11 | Add `mod credit;`, register |
| `finstack-py/src/valuations/instruments/fixed_income/bond.rs` | 13 | Add `price_merton_mc` to PyBond |
| `finstack-py/finstack/valuations/instruments/fixed_income/bond.pyi` | 13 | Add stub |

### Dependency map

```
Task 1 (Merton core)
  ├── Task 2 (calibration) → Task 3 (hazard curve)
  └── Task 4 (path simulation)
Task 5 (endogenous hazard) ─── independent
Task 6 (dynamic recovery) ──── independent
Task 7 (toggle exercise) ───── independent
Tasks 1-7 → Task 8 (engine types) → Task 9 (simulation loop) → Task 10 (bond integration)
Tasks 1-10 → Task 11 (Py merton) → Task 12 (Py specs) → Task 13 (Py MC config)
Task 7 → Task 14 (optimal toggle)
Tasks 1-10 → Task 15 (convergence tests)
Tasks 11-13 → Task 16 (python parity)
```

Tasks 5, 6, 7 are independent of each other and can be parallelized.
Tasks 11, 12 depend on their Rust counterparts being complete.
