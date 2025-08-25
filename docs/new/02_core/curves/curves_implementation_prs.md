# Implementation Road-map — *Curves Module*
> **Note (2025-07-12):** The `InterpPolicy` enum described in early PRs has
> since been **removed**.  Builders now accept concrete `Interpolator`
> variants directly via helper methods (`linear_df()`, `log_df()`, etc.).  The
> historical PR outlines below are kept for context but no longer represent
> the current API.

This document splits the **Curves** Detailed Design into manageable pull-requests.  Each PR is self-contained, ≤~800 LoC, compiles independently, and keeps `master` green.

---

## PR #1 — Bootstrap the `curves` crate

**Goals**
* Add workspace member `curves` (`#![no_std]` default, MSRV 1.78).
* Configure CI matrix for features (`default`, `decimal128`, `serde`, `parallel`, `sabr`, `index`).
* Lay out folder structure per design §4.

**Key changes**
1. Update root `Cargo.toml` & workspace.
2. `curves/Cargo.toml` with feature flags.
3. `curves/src/lib.rs` facade with empty `mod` stubs.
4. GitHub Actions running `fmt`, `clippy`, `test` across feature matrix.
5. Add `primitives` crate to `curves/Cargo.toml` `[dependencies]` so that the `F` alias re-export in PR #2 resolves.

**Acceptance criteria**
* `cargo check` succeeds for all feature combos.
* Zero clippy warnings (`-D warnings`).
* CI workflow green.

---

## PR #2 — Numeric precision layer & `CurveId`

**Goals**
* Re-export the numeric alias `F` from the `primitives` crate (no local definition).
* Implement `CurveId` new-type and simple `FactorKey` taxonomy stub.
* Add minimal `Error` enum (`InterpOutOfBounds`, `Input`).

**Key changes**
1. `id.rs` — `CurveId`, derives, helpers.
2. Update `lib.rs` to `pub use primitives::F;` so downstream modules can reference the alias.
3. Unit tests: `CurveId("USD-OIS") == CurveId("USD-OIS")`.

**Acceptance criteria**
* Compile without `decimal128`.
* `size_of::<CurveId>() == 8` on 64-bit (pointer size).

---

## PR #3 — Core traits & `InterpPolicy`

**Goals**
* Implement traits `Curve`, `DiscountCurve`, `Surface` with default methods.
* Add `InterpPolicy` enum (LinearDf, LogDf, MonotoneConvex, CubicHermite, FlatFwd).
* Provide `Interpolator` trait interface.

**Key changes**
1. `traits.rs` — definitions + docs.
2. `interp.rs` — enum + `Box<dyn Interpolator>` plumbing.
3. Unit tests for default methods (`zero`, `fwd`).

**Acceptance criteria**
* `DummyCurve::df` returns constant; `zero` matches formula.
* Docs build without warnings.

---

## PR #4 — Interpolator infrastructure & `YieldCurve` (Linear / Log DF)

**Goals**
* Implement `LinearDf` & `LogDf` interpolators.
* Create `YieldCurve` builder supporting those policies.
* Provide `df`, `zero`, `fwd` functions with branch-free binary search.

**Key changes**
1. `interp/linear.rs`, `interp/log.rs` implementations.
2. `yield_curve.rs` struct + builder.
3. Criterion micro-bench: 10 M `df` calls ≤15 ms.

**Acceptance criteria**
* Builder rejects unsorted knots (`Error::Input`).
* Bench target met on scalar-`F` (f64) path.
* Round-trip DF↔zero tests within 1 bp.

---

## PR #5 — Advanced interpolators & `InterpPolicy` variants

**Goals**
* Add `MonotoneConvex`, `CubicHermite`, `FlatFwd` interpolators.
* Expose policy selection in `YieldCurve::builder`.

**Key changes**
1. New modules in `interp/` with slope pre-compute.
2. Property tests: monotonicity preserved where applicable.
3. Bench compare MC vs Linear (≤1.3× slowdown).

**Acceptance criteria**
* All policies selectable via builder.
* Unit tests cover edge cases (duplicate knots error).

---

## PR #6 — `HazardCurve` implementation (Credit)

**Goals**
* Add `HazardCurve` struct with survival / default prob helpers.
* Support piecewise-constant hazards; reuse interpolator infra for piecewise-linear.

**Key changes**
1. `hazard_curve.rs` implementation + builder.
2. Tests with analytic exponential hazard comparison.
3. Docs showcasing CDS default probability calc.

**Acceptance criteria**
* `sp(t)` monotone decreasing; `default_prob(t1,t2) ≥ 0`.
* Bench 1 M `sp` calls < 20 ms on the scalar-`F` (f64) build.

---

## PR #7 — `InflationCurve` (real & breakeven)

**Goals**
* Introduce `InflationCurve` with `cpi(t)` and `inflation_rate(t1,t2)`.
* Builder chooses level vs log-return representation.

**Key changes**
1. `inflation.rs` struct + builder.
2. Unit tests vs constant CPI scenario.
3. Serde derives behind `serde` feature.

**Acceptance criteria**
* Round-trip serialize/deserialize JSON equals original.
* `inflation_rate` matches finite-difference of `cpi` within 1 bp.

---

## PR #8 — `CurveSet` multi-curve container & registry

**Goals**
* Implement `CurveSet` with discount, forward, hazard, inflation maps.
* Provide getters (`discount(id)`, `forward(id)`, …) returning `Result`.
* Support collateral map.

**Key changes**
1. `multicurve.rs` struct + methods.
2. HashMap from `hashbrown` (no-std compatibility).
3. Integration test: assemble set, fetch curves, compute PV on sample leg.

**Acceptance criteria**
* `CurveSet::discount("USD-OIS")?` returns reference.
* `Clone` of `CurveSet` is O(1) (Arc pointer copy).

---

## PR #9 — 2-D `VolSurface` grid & SABR feature

**Goals**
* Add `VolSurface` (expiry × strike) with bilinear interpolation.
* Gated `sabr` feature adds analytic SABR representation.

**Key changes**
1. `vol_surface.rs` struct + builder.
2. `interp/bilinear.rs` helper or simple 2-D search.
3. Tests against flat-vol surface; SABR path unit tests when enabled.

**Acceptance criteria**
* `value(exp,strike)` within 1e-8 of flat vol in test case.
* Crate compiles with and without `sabr` feature.

---

## PR #10 — Parallel evaluation & serde integration

**Goals**
* Add `parallel` feature: Rayon `par_iter()` over knot arrays for bulk `df`.
* Derive `Serialize/Deserialize` on all public structs when `serde` flag set.

**Key changes**
1. Conditional Rayon dep & `cfg(feature="parallel")` loops.
2. `serde` derives added; versioned representation.
3. Bench: 10 M `df` calls parallel < 3 ms on 16-core.

**Acceptance criteria**
* `cargo test --features "serde parallel"` passes.
* Scalar path perf unchanged (<1 % diff).

---

## PR #11 — Documentation & public-API audit → v0.3.0

**Goals**
* Final rustdoc examples, module-level docs, API freeze for `v0.3.0`.
* Run `cargo public-api` & semver checks.

**Key changes**
1. Update CHANGELOG (v0.2 & v0.3 sections).
2. Release workflow updates tags.
3. Enforce `#![deny(missing_docs)]`.

**Acceptance criteria**
* `cargo publish --dry-run` succeeds.
* Docs build with `-D warnings`.
* Tag `v0.3.0` pushed & CI green.

---

### Usage Tip
Open an umbrella issue "Implement curves module" and tick each PR box as it merges to visualise progress. 