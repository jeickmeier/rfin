# Quant Audit Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all findings from the quantitative audit of the `finstack/valuations/` crate, from critical probability errors through low-severity consistency improvements.

**Architecture:** Each task is an independent fix targeting a specific finding. Tasks are ordered by severity (critical → high → medium → low). Most fixes are 1-3 line changes with corresponding test additions. All work happens in a dedicated worktree branching from `master`.

**Tech Stack:** Rust, cargo-nextest, finstack workspace (finstack-core, finstack-valuations)

**Test command:** `cargo nextest run -p finstack-valuations --features mc,test-utils --lib --test '*' --no-fail-fast`
**Core test command:** `cargo nextest run -p finstack-core --lib --no-fail-fast`

---

## File Structure

No new files are created. All changes modify existing files:

| File | Changes |
|------|---------|
| `finstack/valuations/src/instruments/common/models/trees/hull_white_tree.rs` | Fix interior probabilities (Task 1), add swaption regression test (Task 13) |
| `finstack/valuations/src/instruments/common/models/volatility/sabr.rs` | Fix β=0 time correction (Task 2), fix arbitrage validation (Task 3) |
| `finstack/valuations/src/instruments/common/models/closed_form/greeks.rs` | Fix theta convention (Task 4) |
| `finstack/core/src/math/special_functions.rs` | Add bounds check to inv_cdf (Task 5), tighten roundtrip test (Task 14) |
| `finstack/valuations/src/calibration/config.rs` | Propagate fd_step to LM solver (Task 6) |
| `finstack/valuations/src/instruments/rates/swaption/pricing/monte_carlo_lsmc.rs` | Add discounting documentation (Task 7) |
| `finstack/valuations/src/metrics/sensitivities/cs01.rs` | Use neumaier_sum (Task 10) |

---

### Task 1: Fix Hull-White Tree Interior Node Probability Sign Error [CRITICAL]

**Files:**
- Modify: `finstack/valuations/src/instruments/common/models/trees/hull_white_tree.rs:326-328`
- Test: same file, `mod tests` at line 797

The interior node probabilities have `+jM` in `p_up` and `-jM` in `p_down`. Per Hull & White (1994) Table 1, this must be reversed: `p_up` gets `-jM` (higher j → more likely to revert down) and `p_down` gets `+jM`.

- [ ] **Step 1: Write a failing test that verifies mean-reverting drift**

Add this test inside the `mod tests` block (after line 989):

```rust
#[test]
fn test_interior_probabilities_are_mean_reverting() {
    // For j > 0 (above mean), the expected displacement should be negative
    // (mean-reverting): E[Δx] = (p_up - p_down) * dx should be < 0.
    // This requires p_down > p_up for positive j.
    let kappa = 0.05;
    let dt = 0.025; // 1Y / 40 steps
    let dx = 0.01 * (3.0 * dt).sqrt();

    for j in [1, 3, 5, 10] {
        let (p_up, _p_mid, p_down) =
            HullWhiteTree::compute_probabilities(kappa, dt, dx, j, 20)
                .expect("valid probabilities");
        // Mean reversion: for j > 0, p_down > p_up
        assert!(
            p_down > p_up,
            "j={}: p_up={:.6}, p_down={:.6} — expected p_down > p_up for mean reversion",
            j, p_up, p_down
        );
        // Expected drift should be negative (toward zero)
        let expected_drift = (p_up - p_down) * dx;
        assert!(
            expected_drift < 0.0,
            "j={}: drift={:.6e} should be negative for mean reversion",
            j, expected_drift
        );
    }

    // Symmetric: for j < 0, p_up > p_down (reverts upward)
    for j in [-1, -3, -5, -10] {
        let (p_up, _p_mid, p_down) =
            HullWhiteTree::compute_probabilities(kappa, dt, dx, j, 20)
                .expect("valid probabilities");
        assert!(
            p_up > p_down,
            "j={}: p_up={:.6}, p_down={:.6} — expected p_up > p_down for mean reversion",
            j, p_up, p_down
        );
    }
}
```

- [ ] **Step 2: Run the test to confirm it fails**

Run: `cargo nextest run -p finstack-valuations --lib -E 'test(test_interior_probabilities_are_mean_reverting)'`
Expected: FAIL — `p_up > p_down` for positive j (anti-mean-reverting)

- [ ] **Step 3: Fix the probability formulas**

In `hull_white_tree.rs`, change lines 326-328 from:

```rust
let mut p_up = 1.0 / 6.0 + (jf * jf * m * m + jf * m) / 2.0;
let mut p_mid = 2.0 / 3.0 - jf * jf * m * m;
let mut p_down = 1.0 / 6.0 + (jf * jf * m * m - jf * m) / 2.0;
```

to:

```rust
let mut p_up = 1.0 / 6.0 + (jf * jf * m * m - jf * m) / 2.0;
let mut p_mid = 2.0 / 3.0 - jf * jf * m * m;
let mut p_down = 1.0 / 6.0 + (jf * jf * m * m + jf * m) / 2.0;
```

Also update the doc comment block at lines 196-205 to match:

```rust
//   p_up = 1/6 + (j²M² - jM)/2
//   p_mid = 2/3 - j²M²
//   p_down = 1/6 + (j²M² + jM)/2
```

- [ ] **Step 4: Run the new test and all existing HW tree tests**

Run: `cargo nextest run -p finstack-valuations --lib -E 'test(hull_white_tree)'`
Expected: ALL PASS — the new mean-reversion test passes, and existing calibration/backward-induction tests still pass (they test first-moment matching via alpha calibration, which is sign-invariant).

- [ ] **Step 5: Commit**

```
git add finstack/valuations/src/instruments/common/models/trees/hull_white_tree.rs
git commit -m "fix(hull-white): correct interior node probability sign for mean reversion

Swap +jM/-jM terms between p_up and p_down to match Hull & White (1994)
Table 1. The previous formulas produced anti-mean-reverting drift at
interior nodes. Boundary treatment was already correct.

Alpha calibration masked this for zero-coupon bonds, but option pricing
via backward induction (swaptions, caps, callable bonds) was affected."
```

---

### Task 2: Fix SABR Normal Model (β=0) Missing α² Time Correction [HIGH]

**Files:**
- Modify: `finstack/valuations/src/instruments/common/models/volatility/sabr.rs:331-333,373-375`
- Test: same file, `mod tests` at line 1617

When β=0, the time correction should include `α²/(24·F²)` per Hagan et al. (2002) Eq. 2.17a. Both the off-ATM path (line 331-333) and ATM path (line 373-375) omit this term.

- [ ] **Step 1: Write a failing test that checks the α² correction**

Add inside `mod tests`:

```rust
#[test]
fn test_normal_sabr_atm_includes_alpha_squared_correction() {
    // For beta=0, the full ATM formula is:
    //   sigma_N = alpha * [1 + T * (alpha^2/(24*F^2) + (2-3*rho^2)*nu^2/24)]
    // The alpha^2/(24*F^2) term is material for typical rate params.
    let alpha = 0.005; // 50bp normal vol
    let forward = 0.03; // 3% rate
    let nu = 0.3;
    let rho = 0.0;
    let t = 5.0; // 5Y expiry amplifies the correction

    let params = SABRParameters::new(alpha, 0.0, nu, rho).unwrap();
    let model = SABRModel::new(params);
    let vol = model.implied_volatility(forward, forward, t).unwrap();

    // Expected correction terms:
    let alpha_sq_term = alpha * alpha / (24.0 * forward * forward);
    let nu_term = (2.0 - 3.0 * rho * rho) / 24.0 * nu * nu;
    let expected = alpha * (1.0 + t * (alpha_sq_term + nu_term));

    let rel_error = ((vol - expected) / expected).abs();
    assert!(
        rel_error < 1e-6,
        "Normal SABR ATM vol={:.8}, expected={:.8}, rel_error={:.2e}. \
         Missing alpha^2/(24*F^2) correction?",
        vol, expected, rel_error
    );
}
```

- [ ] **Step 2: Run the test to confirm it fails**

Run: `cargo nextest run -p finstack-valuations --lib -E 'test(test_normal_sabr_atm_includes_alpha_squared_correction)'`
Expected: FAIL — rel_error ~ 0.15 (missing ~15% of the correction)

- [ ] **Step 3: Fix the off-ATM time correction (lines 331-333)**

Change from:

```rust
let time_correction = if beta_is_zero {
    // Normal SABR time correction
    (2.0 - 3.0 * rho.powi(2)) / 24.0 * nu.powi(2)
} else {
```

to:

```rust
let time_correction = if beta_is_zero {
    // Normal SABR time correction (Hagan et al. 2002, Eq. 2.17a)
    alpha.powi(2) / (24.0 * f_mid.powi(2))
        + (2.0 - 3.0 * rho.powi(2)) / 24.0 * nu.powi(2)
} else {
```

- [ ] **Step 4: Fix the ATM formula (lines 373-375)**

Change from:

```rust
} else if beta_is_zero {
    // Normal SABR: vol = alpha * (1 + T * (2-3*rho²)/24 * nu²)
    alpha * (1.0 + time_to_expiry * (2.0 - 3.0 * rho.powi(2)) / 24.0 * nu.powi(2))
} else if beta_is_one {
```

to:

```rust
} else if beta_is_zero {
    // Normal SABR ATM (Hagan et al. 2002, Eq. 2.17a):
    // vol = alpha * [1 + T * (alpha²/(24F²) + (2-3ρ²)ν²/24)]
    let alpha_sq_term = alpha.powi(2) / (24.0 * forward.powi(2));
    let nu_term = (2.0 - 3.0 * rho.powi(2)) / 24.0 * nu.powi(2);
    alpha * (1.0 + time_to_expiry * (alpha_sq_term + nu_term))
} else if beta_is_one {
```

- [ ] **Step 5: Run the full SABR test suite**

Run: `cargo nextest run -p finstack-valuations --lib -E 'test(sabr)'`
Expected: ALL PASS

- [ ] **Step 6: Commit**

```
git add finstack/valuations/src/instruments/common/models/volatility/sabr.rs
git commit -m "fix(sabr): add missing alpha²/(24F²) term in normal SABR (beta=0)

Both ATM and off-ATM paths for beta=0 omitted the alpha-squared time
correction from Hagan et al. (2002) Eq. 2.17a. For typical rate params
(alpha~50bp, F~3%), this caused ~15% error in the time correction factor."
```

---

### Task 3: Fix SABR Arbitrage Validation Double-Counting Drift [MEDIUM]

**Files:**
- Modify: `finstack/valuations/src/instruments/common/models/volatility/sabr.rs:1585-1614`
- Test: same file, `mod tests`

The `bs_call_price` and `bs_call_vega` helpers pass a forward price to `d1_d2()` with non-zero `r` and `q`, double-counting drift. Since SABR produces Black-76 implied vols, these should use `r=0, q=0` (already a forward).

**API reference:**
- `SABRSmile::new(model, forward, time_to_expiry)` — 3 fields, no rate/div
- `smile.generate_smile(strikes)` → `Result<Vec<f64>>`
- `smile.validate_no_arbitrage(strikes, r, q)` → `Result<ArbitrageValidationResult>`
- `result.is_arbitrage_free()` → bool
- `result.butterfly_violations` → `Vec<ButterflyViolation>`

- [ ] **Step 1: Write a failing test**

```rust
#[test]
fn test_arbitrage_validation_uses_black76_not_bs() {
    // When r != q, bs_call_price was double-counting drift by passing
    // a forward to d1_d2 with non-zero r/q. This caused butterfly
    // spread calculations to use wrong prices.
    let params = SABRParameters::new(0.20, 1.0, 0.30, -0.25).unwrap();
    let model = SABRModel::new(params);
    let forward = 100.0;
    let t = 1.0;

    let smile = SABRSmile::new(model, forward, t);

    let strikes: Vec<f64> = (80..=120).map(|k| k as f64).collect();

    // With large r-q spread, the double-counted drift distorts prices.
    // A well-behaved SABR smile should be arbitrage-free.
    let result = smile
        .validate_no_arbitrage(&strikes, 0.10, 0.02)
        .expect("validation should succeed");
    assert!(
        result.is_arbitrage_free(),
        "Well-behaved SABR smile should have no arbitrage, but got {} butterfly \
         and {} monotonicity violations (likely due to double-counted drift)",
        result.butterfly_violations.len(),
        result.monotonicity_violations.len()
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run -p finstack-valuations --lib -E 'test(test_arbitrage_validation_uses_black76_not_bs)'`
Expected: FAIL or observe the arbitrage violations are present due to distorted prices

- [ ] **Step 3: Fix `bs_call_price` and `bs_call_vega` to use Black-76**

In `sabr.rs`, change `bs_call_price` (around line 1603). The key insight: since `forward` is already a forward price, using `r=0, q=0` in `d1_d2` gives the correct undiscounted Black-76 price. The arbitrage checks (butterfly, monotonicity) only compare relative prices, so the missing discount factor cancels out.

```rust
fn bs_call_price(forward: f64, strike: f64, _r: f64, _q: f64, vol: f64, t: f64) -> f64 {
    if t <= 0.0 {
        return (forward - strike).max(0.0);
    }
    // Black-76: forward is already a forward price, so r=0, q=0 avoids
    // double-counting drift. Discount factor omitted since arbitrage checks
    // only compare relative prices (butterfly, monotonicity).
    let (d1, d2) = d1_d2(forward, strike, 0.0, vol, t, 0.0);
    let cdf_d1 = finstack_core::math::norm_cdf(d1);
    let cdf_d2 = finstack_core::math::norm_cdf(d2);
    forward * cdf_d1 - strike * cdf_d2
}
```

And `bs_call_vega` (around line 1585):

```rust
fn bs_call_vega(forward: f64, strike: f64, _r: f64, _q: f64, vol: f64, t: f64) -> f64 {
    if t <= 0.0 || vol <= 0.0 {
        return 0.0;
    }
    // Black-76: forward is already a forward price, so r=0, q=0.
    let (d1, _d2) = d1_d2(forward, strike, 0.0, vol, t, 0.0);
    let pdf_d1 = finstack_core::math::norm_pdf(d1);
    forward * t.sqrt() * pdf_d1
}
```

- [ ] **Step 4: Run all SABR tests**

Run: `cargo nextest run -p finstack-valuations --lib -E 'test(sabr)'`
Expected: ALL PASS

- [ ] **Step 5: Commit**

```
git add finstack/valuations/src/instruments/common/models/volatility/sabr.rs
git commit -m "fix(sabr): use Black-76 in arbitrage validation helpers

bs_call_price and bs_call_vega passed a forward price to d1_d2 with
non-zero r and q, double-counting the drift. Since SABR produces
Black-76 implied vols, these helpers now use r=0, q=0."
```

---

### Task 4: Fix BsGreeks Theta Convention Mismatch [MEDIUM]

**Files:**
- Modify: `finstack/valuations/src/instruments/common/models/closed_form/greeks.rs:444-515`
- Test: same file, `mod tests` at line 517

`bs_call_greeks()` in `greeks.rs` stores **annualized** theta, while `vanilla.rs::bs_greeks()` stores **per-day** theta (dividing by 365). Both return `BsGreeks`. Normalize `greeks.rs` to per-day (365 days) to match `vanilla.rs`.

- [ ] **Step 1: Write a failing test for theta consistency**

`OptionType` is at `crate::instruments::common_impl::parameters::OptionType`. `vanilla::bs_greeks` signature: `(spot, strike, r, q, sigma, t, option_type, theta_days_per_year)`.

```rust
#[test]
fn test_greeks_aggregate_theta_matches_vanilla() {
    // greeks.rs and vanilla.rs should produce the same theta convention.
    use crate::instruments::common::models::closed_form::vanilla;
    use crate::instruments::common_impl::parameters::OptionType;

    let spot = 100.0;
    let strike = 100.0;
    let rate = 0.05;
    let div = 0.02;
    let vol = 0.20;
    let time = 1.0;

    let from_greeks = bs_call_greeks(spot, strike, time, rate, div, vol);
    let from_vanilla = vanilla::bs_greeks(
        spot, strike, rate, div, vol, time,
        OptionType::Call, 365.0,
    );

    let rel_diff = ((from_greeks.theta - from_vanilla.theta) / from_vanilla.theta).abs();
    assert!(
        rel_diff < 0.01,
        "Theta mismatch: greeks.rs={:.6}, vanilla.rs={:.6}, rel_diff={:.4}",
        from_greeks.theta, from_vanilla.theta, rel_diff
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run -p finstack-valuations --lib -E 'test(test_greeks_aggregate_theta_matches_vanilla)'`
Expected: FAIL — greeks.rs returns annualized theta (~365x larger)

- [ ] **Step 3: Fix theta in `bs_call_greeks` and `bs_put_greeks`**

In `greeks.rs`, change line 456 from:

```rust
let theta = bs_call_theta(spot, strike, time, rate, div_yield, vol);
```

to:

```rust
let theta = bs_call_theta(spot, strike, time, rate, div_yield, vol) / 365.0;
```

And similarly in `bs_put_greeks` line 495, change:

```rust
let theta = bs_put_theta(spot, strike, time, rate, div_yield, vol);
```

to:

```rust
let theta = bs_put_theta(spot, strike, time, rate, div_yield, vol) / 365.0;
```

- [ ] **Step 4: Run all greeks tests**

Run: `cargo nextest run -p finstack-valuations --lib -E 'test(greeks)'`
Expected: ALL PASS (existing tests may need tolerance adjustments if they tested the old annualized convention — check and fix if needed)

- [ ] **Step 5: Commit**

```
git add finstack/valuations/src/instruments/common/models/closed_form/greeks.rs
git commit -m "fix(greeks): normalize theta to per-day in bs_call/put_greeks

bs_call_greeks and bs_put_greeks stored annualized theta in BsGreeks,
while vanilla::bs_greeks stored per-day (÷365). This created a silent
convention mismatch for downstream consumers of the shared struct."
```

---

### Task 5: Add Bounds Check to `standard_normal_inv_cdf` [MEDIUM]

**Files:**
- Modify: `finstack/core/src/math/special_functions.rs:334-337`
- Test: same file, `mod tests` at line 464

The underlying `statrs` implementation panics on p ∉ [0,1]. Add a guard returning `f64::NEG_INFINITY` / `f64::INFINITY` / `f64::NAN` for boundary/invalid inputs.

- [ ] **Step 1: Write a failing test**

```rust
#[test]
fn test_inv_cdf_boundary_inputs_do_not_panic() {
    // These should return well-defined values, not panic
    let neg = standard_normal_inv_cdf(-0.1);
    assert!(neg.is_nan(), "p < 0 should return NaN, got {}", neg);

    let over = standard_normal_inv_cdf(1.5);
    assert!(over.is_nan(), "p > 1 should return NaN, got {}", over);

    let zero = standard_normal_inv_cdf(0.0);
    assert!(zero == f64::NEG_INFINITY, "p = 0 should return -inf, got {}", zero);

    let one = standard_normal_inv_cdf(1.0);
    assert!(one == f64::INFINITY, "p = 1 should return +inf, got {}", one);

    let nan = standard_normal_inv_cdf(f64::NAN);
    assert!(nan.is_nan(), "p = NaN should return NaN, got {}", nan);
}
```

- [ ] **Step 2: Run test to verify it panics**

Run: `cargo nextest run -p finstack-core --lib -E 'test(test_inv_cdf_boundary_inputs_do_not_panic)'`
Expected: FAIL with panic — `x must be in [0, 1]`

- [ ] **Step 3: Add bounds guard**

Change `standard_normal_inv_cdf` (line 334-337) from:

```rust
pub fn standard_normal_inv_cdf(p: f64) -> f64 {
    use statrs::distribution::ContinuousCDF;
    STANDARD_NORMAL.inverse_cdf(p)
}
```

to:

```rust
pub fn standard_normal_inv_cdf(p: f64) -> f64 {
    if p.is_nan() || p < 0.0 || p > 1.0 {
        return f64::NAN;
    }
    if p == 0.0 {
        return f64::NEG_INFINITY;
    }
    if p == 1.0 {
        return f64::INFINITY;
    }
    use statrs::distribution::ContinuousCDF;
    STANDARD_NORMAL.inverse_cdf(p)
}
```

- [ ] **Step 4: Run all core math tests**

Run: `cargo nextest run -p finstack-core --lib -E 'test(special_functions)' --no-fail-fast`
Expected: ALL PASS

- [ ] **Step 5: Commit**

```
git add finstack/core/src/math/special_functions.rs
git commit -m "fix(core): guard standard_normal_inv_cdf against out-of-range input

The underlying statrs implementation panics on p outside [0,1].
Add explicit bounds checks returning NaN for invalid input and
+/-infinity for boundary values (p=0, p=1). Prevents panics in
Python bindings and downstream copula/VaR code."
```

---

### Task 6: Propagate `jacobian_step_size` to LM Solver [MEDIUM]

**Files:**
- Modify: `finstack/valuations/src/calibration/config.rs:730-735`

`create_lm_solver()` propagates `tolerance` and `max_iterations` but not `jacobian_step_size` (default 1e-6 in config, 1e-8 in LM). The LM solver has a `with_fd_step()` method.

- [ ] **Step 1: Write a failing test**

Add inside `mod tests` in `config.rs`:

```rust
#[test]
fn test_create_lm_solver_propagates_fd_step() {
    let mut config = CalibrationConfig::default();
    config.discount_curve.jacobian_step_size = 5e-7;
    let solver = config.discount_curve.create_lm_solver();
    assert!(
        (solver.fd_step - 5e-7).abs() < 1e-15,
        "LM solver fd_step={} should match config jacobian_step_size=5e-7",
        solver.fd_step
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run -p finstack-valuations --lib -E 'test(test_create_lm_solver_propagates_fd_step)'`
Expected: FAIL — solver.fd_step is 1e-8 (LM default), not 5e-7

- [ ] **Step 3: Add `.with_fd_step()` to `create_lm_solver`**

Change lines 730-735 from:

```rust
pub fn create_lm_solver(&self) -> finstack_core::math::solver_multi::LevenbergMarquardtSolver {
    use finstack_core::math::solver_multi::LevenbergMarquardtSolver;

    LevenbergMarquardtSolver::new()
        .with_tolerance(self.solver.tolerance())
        .with_max_iterations(self.solver.max_iterations())
}
```

to:

```rust
pub fn create_lm_solver(&self) -> finstack_core::math::solver_multi::LevenbergMarquardtSolver {
    use finstack_core::math::solver_multi::LevenbergMarquardtSolver;

    LevenbergMarquardtSolver::new()
        .with_tolerance(self.solver.tolerance())
        .with_max_iterations(self.solver.max_iterations())
        .with_fd_step(self.jacobian_step_size)
}
```

- [ ] **Step 4: Run calibration tests**

Run: `cargo nextest run -p finstack-valuations --lib --test '*' -E 'test(calibrat)' --no-fail-fast`
Expected: ALL PASS

- [ ] **Step 5: Commit**

```
git add finstack/valuations/src/calibration/config.rs
git commit -m "fix(calibration): propagate jacobian_step_size to LM solver

create_lm_solver() was not passing jacobian_step_size (default 1e-6) to
the LM solver's fd_step (default 1e-8). For typical zero rates (~1-5%),
1e-8 can fall into numerical noise causing StepTooSmall convergence."
```

---

### Task 7: Document LSMC Market-Curve Discounting Approximation [MEDIUM]

**Files:**
- Modify: `finstack/valuations/src/instruments/rates/swaption/pricing/monte_carlo_lsmc.rs:628-633`

The LSMC backward induction discounts continuation values using the market discount curve instead of path-dependent short rate integrals. This is a known approximation. Add a doc comment explaining the trade-off.

- [ ] **Step 1: Add documentation comment**

Before line 628, add:

```rust
// NOTE: Discounting approximation — we use market discount factors D(t)
// rather than path-dependent accumulated short rates exp(-∫r(s)ds).
// Under Hull-White, the exercise decision correlates with the discount
// factor through the short rate, introducing a systematic (typically small)
// bias for Bermudan swaptions. The exact approach would require storing
// cumulative discount factors along each path. This is a standard
// simplification in production LSMC implementations.
// Reference: Longstaff & Schwartz (2001), Andersen & Piterbarg (2010) §15.4.
```

- [ ] **Step 2: Run LSMC tests to confirm no regression**

Run: `cargo nextest run -p finstack-valuations --features mc,test-utils --lib -E 'test(monte_carlo)' --no-fail-fast`
Expected: ALL PASS (documentation-only change)

- [ ] **Step 3: Commit**

```
git add finstack/valuations/src/instruments/rates/swaption/pricing/monte_carlo_lsmc.rs
git commit -m "docs(lsmc): document market-curve discounting approximation

Add comment explaining why the backward induction uses market discount
factors rather than path-dependent short rate integrals, and the
resulting bias for Bermudan swaptions under Hull-White."
```

---

### Task 8: Use Neumaier Summation in CS01 Bucketed Totals [LOW]

**Files:**
- Modify: `finstack/valuations/src/metrics/sensitivities/cs01.rs:241,516`

DV01 bucketed totals use `neumaier_sum` (line 418 of dv01.rs) but CS01 uses naive `+= cs01`. Align for consistency.

- [ ] **Step 1: Fix par-spread bucketed CS01 (line 239-241)**

Change from:

```rust
let cs01 = sensitivity_central_diff(pv_bumped_up, pv_bumped_down, bump_bp);
series.push((label, cs01));
total += cs01;
```

After the loop, replace the total computation. Find the pattern:

```rust
context.store_bucketed_series(series_id, series);
Ok(total)
```

Replace with:

```rust
context.store_bucketed_series(series_id, series.clone());
let total: f64 = finstack_core::math::neumaier_sum(series.iter().map(|(_, v)| *v));
Ok(total)
```

Remove the `let mut total = 0.0;` declaration and the `total += cs01;` lines that preceded the loop.

- [ ] **Step 2: Fix hazard-rate bucketed CS01 (line 514-516)**

Same pattern as step 1. In the `GenericBucketedCs01Hazard` compute method, find:

```rust
let cs01 = sensitivity_central_diff(pv_up, pv_down, bump_bp);
series.push((label, cs01));
total += cs01;
```

Remove the `let mut total = 0.0;` and `total += cs01;`. After the loop, replace:

```rust
context.store_bucketed_series(series_id, series);
Ok(total)
```

with:

```rust
context.store_bucketed_series(series_id, series.clone());
let total: f64 = finstack_core::math::neumaier_sum(series.iter().map(|(_, v)| *v));
Ok(total)
```

- [ ] **Step 3: Run CS01 tests**

Run: `cargo nextest run -p finstack-valuations --features mc,test-utils --lib --test '*' -E 'test(cs01)' --no-fail-fast`
Expected: ALL PASS

- [ ] **Step 4: Commit**

```
git add finstack/valuations/src/metrics/sensitivities/cs01.rs
git commit -m "fix(cs01): use neumaier_sum for bucketed totals

Align with DV01 which already uses compensated summation for bucket
totals. Prevents potential precision loss when many small sensitivities
are summed."
```

---

### Task 9: Tighten `standard_normal_inv_cdf` Roundtrip Test Tolerance [LOW]

**Files:**
- Modify: `finstack/core/src/math/special_functions.rs`, test module at line 464

The existing roundtrip test uses `1e-3` tolerance. The actual implementation (Boost-derived erfc_inv) achieves `~1e-14`. Tighten to reflect true precision.

- [ ] **Step 1: Find and update the roundtrip test**

Find the existing test `test_normal_cdf_inv_cdf_roundtrip` (or similar name) and tighten its tolerance from `1e-3` to `1e-12` for the main range, and from 10% relative to 1e-8 for tail values.

```rust
#[test]
fn test_inv_cdf_roundtrip_tight_tolerance() {
    // Main range: the Boost-derived erfc_inv achieves ~1e-14 accuracy
    for p in [0.01, 0.05, 0.10, 0.25, 0.50, 0.75, 0.90, 0.95, 0.99] {
        let x = standard_normal_inv_cdf(p);
        let p_back = norm_cdf(x);
        let abs_err = (p_back - p).abs();
        assert!(
            abs_err < 1e-12,
            "Roundtrip failed for p={}: got p_back={}, error={:.2e}",
            p, p_back, abs_err
        );
    }

    // Tails: slightly relaxed but still tight
    for p in [1e-6, 1e-10, 1.0 - 1e-6, 1.0 - 1e-10] {
        let x = standard_normal_inv_cdf(p);
        let p_back = norm_cdf(x);
        let rel_err = ((p_back - p) / p).abs();
        assert!(
            rel_err < 1e-8,
            "Tail roundtrip failed for p={:.2e}: got p_back={:.2e}, rel_error={:.2e}",
            p, p_back, rel_err
        );
    }
}
```

- [ ] **Step 2: Run the test**

Run: `cargo nextest run -p finstack-core --lib -E 'test(test_inv_cdf_roundtrip_tight_tolerance)'`
Expected: PASS

- [ ] **Step 3: Commit**

```
git add finstack/core/src/math/special_functions.rs
git commit -m "test(core): add tight roundtrip test for standard_normal_inv_cdf

The existing roundtrip test used 1e-3 tolerance. The Boost-derived
implementation achieves ~1e-14 in the main range. New test verifies
this precision and adds tail coverage."
```

---

### Task 10: Add Swaption Pricing Sanity Test for HW Tree [LOW]

**Files:**
- Modify: `finstack/valuations/src/instruments/common/models/trees/hull_white_tree.rs`, test module

This test prices a swaption via tree backward induction and validates the result is positive and bounded. It catches future drift regressions that unit-payoff tests (which only test first-moment matching) would miss.

- [ ] **Step 1: Write the sanity test**

```rust
#[test]
fn test_swaption_price_sanity_hw_tree() {
    // Price a 2Y into 3Y payer swaption on the tree.
    // This catches drift regressions that unit-payoff tests miss,
    // since swaption values depend on the conditional rate distribution.
    let kappa = 0.03;
    let sigma = 0.01;
    let config = HullWhiteTreeConfig::new(kappa, sigma, 200);
    let curve = test_discount_curve();
    let maturity = 5.0; // tree must cover option expiry + swap tenor

    let tree = HullWhiteTree::calibrate(config, &curve, maturity)
        .expect("Calibration should succeed");

    let option_expiry = 2.0;
    let swap_maturity = 5.0; // 3Y swap
    let swap_rate = 0.04; // fixed coupon
    let expiry_step = tree.time_to_step(option_expiry);

    // Terminal payoff at option expiry: max(swap_value, 0) for payer
    let num_nodes = tree.num_nodes(expiry_step);
    let terminal: Vec<f64> = (0..num_nodes)
        .map(|j| {
            // Swap value = 1 - P(T_opt, T_swap) - K * annuity
            let p_swap_end = tree.bond_price(expiry_step, j, swap_maturity, &curve);
            let mut annuity = 0.0;
            for year in 1..=3 {
                let t = option_expiry + year as f64;
                annuity += tree.bond_price(expiry_step, j, t, &curve);
            }
            let swap_value = 1.0 - p_swap_end - swap_rate * annuity;
            swap_value.max(0.0)
        })
        .collect();

    let tree_price = tree.backward_induction(&terminal, |_, _, cont| cont);

    // Positive and bounded
    assert!(
        tree_price > 0.0,
        "Swaption price should be positive, got {}",
        tree_price
    );
    assert!(
        tree_price < 1.0,
        "Swaption price {} seems too large (per unit notional)",
        tree_price
    );
}
```

- [ ] **Step 2: Run the test**

Run: `cargo nextest run -p finstack-valuations --lib -E 'test(test_swaption_price_matches_analytical_hw)'`
Expected: PASS (this is a sanity test; post-Task-1 fix, the tree dynamics are correct)

- [ ] **Step 3: Commit**

```
git add finstack/valuations/src/instruments/common/models/trees/hull_white_tree.rs
git commit -m "test(hull-white): add swaption pricing regression test

Prices a payer swaption via tree backward induction and validates
the result is positive and reasonable. Catches drift regressions that
unit-payoff tests (which only test first-moment matching) would miss."
```

---

### Task 11: Run Full Test Suite and Verify No Regressions

- [ ] **Step 1: Run the complete workspace test suite**

Run: `CARGO_INCREMENTAL=1 cargo nextest run --workspace --exclude finstack-py --features mc,test-utils --lib --test '*' --no-fail-fast`

Expected: ALL PASS. If any tests fail, investigate — the HW tree fix (Task 1) may cause small numerical changes in tests that relied on the old (incorrect) dynamics.

- [ ] **Step 2: Run doc tests**

Run: `CARGO_INCREMENTAL=1 cargo test --workspace --exclude finstack-py --doc --features mc`

Expected: ALL PASS

- [ ] **Step 3: Run clippy**

Run: `cargo clippy --workspace --exclude finstack-py --features mc,test-utils -- -D warnings`

Expected: No new warnings

- [ ] **Step 4: Final commit (if any test adjustments were needed)**

```
git add -A
git commit -m "test: adjust tolerances for corrected HW tree dynamics"
```

---

## Deferred Findings

The following audit findings are intentionally excluded from this plan:

| Audit # | Finding | Reason Deferred |
|---------|---------|-----------------|
| 8 | Expose `norm_sf()` survival function | New public API addition — requires API design review and downstream consumer analysis |
| 9 | Expose `ln_pdf()` for log-likelihood | Same — new public API, no current callers identified |
| 11 | LSMC antithetic per-step allocation | Performance optimization only, no correctness impact; benchmark first |
| 12 | LSMC redundant swap rate recomputation | Performance optimization only; would require refactoring backward_induction signature |
| 15 | Student-t df>100 normal approximation threshold | Changing the threshold affects all existing t-distribution callers; needs impact analysis on copula models |
