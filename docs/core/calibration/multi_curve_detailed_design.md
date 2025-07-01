# Multi-Curve Solver – Detailed Design Document

_Last updated: 2025-06-29_

---

## 1 Market-Standard Multi-Curve Calibration Workflow

Since the 2008-09 collateralisation shift, dealers build **one discount curve per collateral currency** and **one forward-rate curve per floating-leg tenor**.  The market-standard workflow is therefore a *bootstrapped, sequential* calibration consisting of three stages:

| Stage | Curve(s) | Main instruments | Objective |
|-------|----------|------------------|-----------|
| ① **Discount (funding)** | OIS curve (one per collateral currency) | Overnight deposits, OIS swaps | Match par OIS prices |
| ② **Forward curves** | One curve per floating-leg tenor (1M, 3M, 6M, 12M, …) | FRA/IRS on that tenor **plus** tenor-basis swaps vs the discount curve | Match par prices given the discount curve |
| ③ **Consistency loop** *(optional)* | All curves | Re-price all calibration instruments with the latest set of curves and iterate until the largest mis-pricing < tolerance (≈ 0.01 bp) | Ensure self-consistency across tenors |

Stages ①–② reproduce the "standard" dealer spreadsheet, while the optional loop ③ polishes residual basis.

---

## 2 Solver Algorithms

Two complementary algorithms are implemented inside the `calibration::multi_curve` module.

### 2.1 Iterative Projection (Block Gauss–Seidel)

1. Hold every curve but one fixed.
2. Calibrate that curve (1-D bootstrap or Newton on its own nodes).
3. Move to the next curve.
4. Repeat until the maximum pricing error across all instruments drops below `outer_tol` (defaults to `1e-10` DF or `0.01 bp`).

*Pros* – extremely robust, easy to debug; each sub-problem is small.

*Cons* – only linear convergence; may require many outer iterations when the basis is wide.

### 2.2 Global Newton / Levenberg–Marquardt

1. Stack **all** curve nodes into a single vector `x`.
2. Price every calibration instrument to build the residual vector `f(x)`.
3. Compute or approximate the Jacobian `J = ∂f/∂x`.
4. Solve the linear system `J Δx = −f` using `nalgebra::DMatrix` / `DVector`.
5. Update `x ← x + Δx`, optionally with LM damping λ.
6. Iterate until `‖f‖∞ < resid_tol` **and** `‖Δx‖∞ < step_tol`.

*Pros* – quadratic convergence near the solution; scales to dozens of curves & thousands of instruments.

*Cons* – needs a good initial guess; Jacobian may be ill-conditioned, thus regularisation and damping are mandatory.

### 2.3 Hybrid Strategy (Default)

```rust
let mut curves = projection::solve(&market, &cfg.projection)?; // robust seed
if cfg.polish {
    curves = newton::solve_from(curves, &market, &cfg.newton)?;
}
```

* Use **projection** to obtain a stable starting point and as a fall-back when market quotes are inconsistent.
* Use a **global Newton/LM** step to "polish" the solution in a handful of iterations once a reasonable seed is available.

This mirrors modern open-source libraries such as *finmath*.

---

## 3 Module Layout

```text
crate::calibration
 ├── traits
 │   └── multi_curve_solver.rs   <-- defines `SolverConfig` & the high-level `solve` façade
 ├── projection.rs               <-- iterative block solver (Approach A)
 ├── newton.rs                   <-- global Newton / LM engine (Approach B)
 └── tests/                      <-- unit + regression tests (golden curves)
```

### 3.1 Shared State Vector

Each curve is exposed as:

```rust
pub struct Curve {
    pub nodes: Vec<Rate>,          // discount or forward rates
    pub interp: InterpKind,        // e.g. LogDf, LinearZero
}
```

All solvers share the same flat view of *every* node so that switching algorithms is transparent.

### 3.2 Jacobian Backend

`nalgebra::DMatrix` holds the sparse-but-rectangular Jacobian.  When the optional `parallel` feature is enabled the residual & Jacobian assembly is chunked with **Rayon**.

---

## 4 Convergence, Stability & Configuration

* **Outer tolerance** – maximum absolute pricing error (PV) across all instruments.
* **Inner tolerance** – maximum change in any curve node `Δx`.
* **Damping** – Tikhonov regularisation or LM λ schedule applied when `‖Δx‖` grows.
* **Fallback logic** – if the Newton/LM step fails to reduce the residual, the solver rolls back & halves λ.
* **Features** – `parallel` toggles Rayon; `decimal128` enables high-precision maths for tiny basis risks.

---

## 5 Testing Strategy

1. **Deterministic unit tests** with synthetic quotes – assert PV ≈ 0 after calibration.
2. **Regression tests** against historical fixing sets.
3. **Stress tests** with deliberately inconsistent basis to ensure graceful degradation (projection step falls back, Newton skipped).

---

## 6 Why This Matches Market Practice

The projection pass alone reproduces the sequential bootstrapping that remains the default on trading desks.  The Newton/LM overlay brings the speed and uniform risk attribution that quants require when curve sets explode in size.

---

## 7 Key References

* Fries, *Curves: A Primer* – multi-curve bootstrapping methodology (<https://www.finmath.net/topics/curvecalibration/>)
* Pallavicini & Tarenghi (2010), *Interest-Rate Modelling with Multiple Yield Curves* – HJM framework + bootstrapping algorithm (<https://arxiv.org/abs/1006.4767>)
* Quant.SE threads on sequential vs global approaches (<https://quant.stackexchange.com/>) 