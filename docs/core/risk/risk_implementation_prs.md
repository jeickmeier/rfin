# Implementation Road-map — *Risk Metrics Module*

This roadmap converts the **Risk Metrics** Detailed Design into focused pull-requests (PRs). Each PR is self-contained (≤ ~800 LoC), compiles independently, and keeps `master` green.

---

## PR #1 — Bootstrap the `risk` crate

**Goals**
* Add workspace member `risk` (`#![no_std]` default, MSRV 1.78).
* Configure CI matrix for feature flags (`default`, `parallel`, `analytic_only`, `aad_only`, `serde`).
* Lay down folder structure per design §5.

**Key changes**
1. Update root `Cargo.toml` workspace list.
2. `risk/Cargo.toml` with feature flags + deps (`rayon`, `dashmap`, `bumpalo`).
3. `risk/src/lib.rs` facade re-exporting empty module stubs (`engine`, `factor`, etc.).
4. GitHub Actions running `fmt`, `clippy`, `test` across feature matrix.

**Acceptance criteria**
* `cargo check` passes for all feature combos.
* Zero clippy warnings (`-D warnings`).
* CI pipeline green.

---

## PR #2 — Core enums & structs (`Order`, `FDStencil`, `RiskFactor`)

**Goals**
* Implement `Order`, `FDStencil` enums **and the `BucketId` type alias**.
* Add `RiskFactor` taxonomy with rate, vol, FX, inflation, credit, equity variants.
* Provide helper constructors & hashing utils.

**Key changes**
1. `factor.rs` — enum definitions, `pub type BucketId = u8;`, impl `Hash`, `Eq`.
2. Unit tests: hashing uniqueness, equality semantics.

**Acceptance criteria**
* `HashSet<RiskFactor>` deduplicates identical factors.
* `size_of::<RiskFactor>()` ≤ 32 bytes (f64 path).

---

## PR #3 — `RiskReport` struct & serialization

**Goals**
* Define `RiskReport` with sparse parallel-vector layout and cross-gamma store.
* Derive Serde when `serde` flag enabled; ensure versioned representation.

**Key changes**
1. `report.rs` — struct, `Default` impl (pv=0).
2. Property test: serialization → deserialization yields equality.
3. Docs example building a mini report.

**Acceptance criteria**
* JSON round-trip equals original under `serde` feature.
* `RiskReport::factors.len()` always equals `delta.len()`, etc. (assert in ctor).

---

## PR #4 — `RiskEngine` trait & `RiskMode` dispatcher

**Goals**
* Add `RiskEngine` trait with generic `compute` method.
* Implement enum `RiskMode` (Analytic, Adjoint{order}, FiniteDiff{bump, stencil}) and blanket trait impl dispatching to concrete engines.

**Key changes**
1. `engine.rs` — trait, enum, default bump settings **with `#[cfg]` guards for `analytic_only` / `aad_only` flags**.
2. Unit test uses dummy `Priced` instrument returning const PV to verify dispatch path.

**Acceptance criteria**
* `RiskMode::Analytic.compute(..)` calls analytic engine (mocked).
* Compilation succeeds under `analytic_only` (excludes AAD & FD modules).

---

## PR #5 — Analytic engine scaffold & first instruments (v0.1.0)

**Goals**
* Implement `analytic.rs` with analytic Greeks for SpotAsset (Instruments PR #3) and Deposit (Instruments PR #4) so they serve as first reference instruments.
* Provide helper traits `AnalyticGreeks` per instrument.

**Key changes**
1. `analytic.rs` implementation using closed-form formulas.
2. Integration tests comparing to finite-difference baseline within 1 bp.
3. Prepare CHANGELOG and tag `v0.1.0`.

**Acceptance criteria**
* Analytic Δ for spot equals 1.0 within 1e-12.
* `cargo publish --dry-run` succeeds for risk crate.

---

## PR #6 — Finite-difference engine (`finite_diff.rs`)

**Goals**
* Add fallback FD engine supporting one-sided, two-sided, four-point stencils.
* Configurable absolute vs relative bump via settings.

**Key changes**
1. `finite_diff.rs` algorithm using parallel bumps when `parallel` flag.
2. Bench: FD Δ on 10 k swaps with 2-sided stencil < 120 ms (16 cores).
3. Property tests ensure Δ sign flips with underlying price.

**Acceptance criteria**
* FD γ approximates analytic γ for spot asset within 1e-4.
* Crate compiles with `analytic_only` flag disabled (guard).

---

## PR #7 — Adjoint (AAD) engine & bump cache (C-59)

**Goals**
* Implement `adjoint.rs` reverse-sweep engine using `bumpalo` arena.
* Add `cache.rs` bump-seed cache built on `dashmap` with LRU clock.

**Key changes**
1. Tape representation structs; `reverse()` sweep.
2. `BumpCache` with configurable capacity (env var override).
3. Tests: cache reuse hit-rate ≥95 % on repeat scenario.

**Acceptance criteria**
* AAD Δ matches FD Δ within 0.5 bp for vanilla swaption.
* Memory leak test passes (heap after 1 k runs stable).

---

## PR #8 — Key-rate DV01 & Vega bucket generators (C-57)

**Goals**
* Add `bucket.rs` utilities to construct tenor buckets for curves & vol surfaces.
* Provide helpers to map risk factors to bucket index.

**Key changes**
1. Functions `generate_key_rates(&curve, tenors)` and `vol_buckets(..)`.
2. Unit tests: generated buckets match expected grid.

**Acceptance criteria**
* 11-point govvie grid produced by default.
* Bucket list monotone increasing maturities.

---

## PR #9 — Scenario re-valuation helpers (C-60)

**Goals**
* Implement `scenario.rs` with `MarketSnapshot` trait and shocking utilities (`shift_curve`, `shift_surface`).
* Provide `RiskEngine::scenario` convenience wrapper reusing bump cache.

**Key changes**
1. Snapshot structs capturing curves & FX spots.
2. Examples in docs applying +25 bp parallel shift.
3. Integration tests ensure PV shift sign is correct.

**Acceptance criteria**
* Scenario PV equals base PV when shift = 0.
* Cache reused across scenarios (metrics log).

---

## PR #10 — Portfolio aggregation & cross-gamma support

**Goals**
* Add `aggregate.rs` functions merging `RiskReport`s across trades, summing buckets and currencies.
* **Persist, store, and aggregate** sparse upper-triangular cross-gamma matrices.

**Key changes**
1. `aggregate.rs` reduce by `RiskFactor` hashing.
2. Test: cross-gamma symmetry enforced.
3. Bench: aggregate 50 k trade reports < 20 ms.

**Acceptance criteria**
* Aggregated DV01 equals sum of individual DV01s.
* Cross-gamma duplicates merged correctly.

---

## PR #11 — Documentation & public-API audit → v0.3.0

**Goals**
* Complete rustdoc examples, enforce `#![deny(missing_docs)]`, ensure semver-check passes.
* Update CHANGELOG and tag `v0.3.0`.

**Key changes**
1. Add README usage guide & feature matrix table.
2. Release workflow for risk crate.
3. Serde versioning attributes on structs.

**Acceptance criteria**
* `cargo publish --dry-run` succeeds.
* Docs build with `-D warnings`.
* Tag `v0.3.0` pushed & CI green.

---

### Usage Tip
Create an umbrella issue "Implement risk metrics module" and tick each PR box as it merges to monitor progress. 