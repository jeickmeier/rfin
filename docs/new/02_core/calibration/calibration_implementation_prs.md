# Implementation Road-map — *Calibration Module*

This document decomposes the **Calibration** Detailed Design into focused pull-requests. Each PR is self-contained (≤ ~800 LoC), compiles independently, and keeps `master` green.

---

## PR #1 — Bootstrap the `calibration` crate

**Goals**
* Add workspace member `calibration` (`#![no_std]` default, MSRV 1.78).
* Establish CI matrix for features (`default`, `parallel`, `decimal128`, `sabr`).
* Create folder layout as per design §4.

**Key changes**
1. Update root `Cargo.toml` workspace.
2. `calibration/Cargo.toml` with feature flags & deps (`nalgebra`, optional `rayon`, `sabr_rs`).
3. `calibration/src/lib.rs` facade with empty module stubs (`solver`, `bootstrap`, `sabr`, `tree`, etc.).
4. GitHub Actions: `cargo fmt`, `clippy`, `test` across feature matrix.

**Acceptance criteria**
* `cargo check` passes for all feature combos.
* Zero clippy warnings (`-D warnings`).
* CI workflow green.

---

## PR #2 — Solver trait & root-finding algorithms

**Goals**
* Implement `Solver` trait with `solve(&self, f, guess)` signature.
* Provide Newton, Brent, Bisection, Secant structs with common interface.

**Key changes**
1. `solver.rs` — trait + generic implementations.
2. Unit tests: solve `x^2 – 2 = 0`; ensure convergence within tol.
3. Bench: Newton vs Brent for typical DF root (≤ 2× diff).

**Acceptance criteria**
* All solvers reach root `√2` within 1e-12 in ≤ 20 iters.
* Trait object safe (`dyn Solver`).

---

## PR #3 — `Bootstrappable` trait and yield-curve bootstrapper (C-46)

**Goals**
* Define `Bootstrappable` trait with async `calibrate` fn.
* Implement piecewise log-DF bootstrap in `bootstrap/yield.rs`.
* Support overnight depo, term depo, swaps; quotes modelled in simple struct.

**Key changes**
1. `bootstrap/yield.rs` with sequential knot solve.
2. Sample `YieldQuote` enum + helpers.
3. Integration test: 10 OIS quotes produce DF curve with PV≈0.

**Acceptance criteria**
* Max abs PV error < 1e-10 after calibration.
* Runtime < 3 ms for 100 quotes (bench).

---

## PR #4 — Hazard-curve bootstrapper (C-43)

**Goals**
* Implement `bootstrap/hazard.rs` using piecewise flat λ methodology.
* Quotes: CDS par spreads.

**Key changes**
1. Structs `CdsQuote` and hazard bootstrap logic.
2. Tests against analytic exponential survival.
3. Property tests: survival probability monotone ↓.

**Acceptance criteria**
* PV error < 1e-8 bp for sample CDS.
* Bench 100 CDS quotes < 4 ms.

---

## PR #5 — Inflation-curve calibration

**Goals**
* Add `bootstrap/inflation.rs` for CPI level curve.
* Support ZC-swap quotes and CPI fixings.

**Key changes**
1. `InflationQuote` structs.
2. Linear-in-log CPI solver.
3. Unit tests: synthetic swap PV ≈ 0.

**Acceptance criteria**
* Calibration converges in ≤ 15 iterations per knot.
* JSON serde round-trip (behind `serde` flag).

---

## PR #6 — Vol-surface grid fit & SABR calibration (C-47)

**Goals**
* Implement `surface.rs` grid vol fit from cap/floor strips.
* Add `sabr.rs` fitting of (α, ν, ρ) per expiry/tenor pair when `sabr` feature enabled.

**Key changes**
1. Bilinear grid interpolation helper.
2. Least-squares SABR fit using `newton` solver.
3. Tests: reproduce input vols within 0.1 bp.

**Acceptance criteria**
* Surface query `vol(exp,strk)` within 1 e-4 of input grid.
* Crate compiles without `sabr` flag.

---

## PR #7 — Multi-curve solver infrastructure (C-34)

**Goals**
* Add `multi_curve` module with iterative projection solver.
* Integrate with previously built bootstrappers.

**Key changes**
1. `multi_curve/mod.rs`, `projection.rs`, config struct.
2. Unit test: bootstrap OIS + 3-M LIBOR curves self-consistent < 0.01 bp.
3. Bench: two-curve set 100 quotes < 10 ms.

**Acceptance criteria**
* Outer loop converges in ≤ 6 passes for sample market.
* Feature-gated Rayon parallel assembly (`parallel`).

---

## PR #8 — Global Newton / LM polish

**Goals**
* Implement `newton.rs` global solver using `nalgebra` matrices.
* Hook optional polish step in multi-curve `SolverConfig`.

**Key changes**
1. Jacobian assembly with automatic differencing.
2. LM damping schedule.
3. Regression test: PV residual < 1e-12 after polish.

**Acceptance criteria**
* Newton step reduces max error by ≥ 100× relative to projection seed.
* Fallback to projection when Jacobian ill-conditioned.

---

## PR #9 — Tree calibration helpers (equity, rate, credit)

**Goals**
* Add `tree_equity`, `tree_rate`, `tree_credit` calibration helpers.
* Provide generic lattice calibration trait used by risk engines.

**Key changes**
1. Implement Cox-Ross-Rubinstein & Jarrow-Rudd calibrations.
2. Ho-Lee tree short-rate calibration.
3. Credit intensity lattice via Duffie-Singleton.

**Acceptance criteria**
* Model option PVs match market within 0.5 bp.
* Bench equity tree (256 steps) calibration < 5 ms.

---

## PR #10 — Stress-test mode (C-50) & parallel feature

**Goals**
* Implement `stress.rs` re-pricing with shocked quotes (no root-find).
* Integrate Rayon parallel loops when `parallel` feature enabled.

**Key changes**
1. `stress.rs` util functions.
2. CLI example (optional) showing 25 bp shock scenario.
3. Bench: stress-reprice 10 curves × 1 k quotes < 5 ms.

**Acceptance criteria**
* Results identical to full bootstrap when shock=0.
* Parallel path speedup ≥ 3× on 8 cores.

---

## PR #11 — Documentation & public-API audit → v0.3.0

**Goals**
* Final rustdoc examples, CHANGELOG updates, API freeze for `v0.3.0`.
* Ensure semver-check passes across all public items.

**Key changes**
1. Enforce `#![deny(missing_docs)]`.
2. Release workflow tagging.
3. Serde version tags on all structs.

**Acceptance criteria**
* `cargo publish --dry-run` succeeds.
* Docs build with `-D warnings`.
* Tag `v0.3.0` pushed & CI green.

---

### Usage Tip
Open an umbrella issue "Implement calibration module" and tick each PR once merged to track progress. 