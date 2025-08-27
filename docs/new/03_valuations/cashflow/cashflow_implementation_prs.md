# Implementation Road-map — *CashFlow Module*

This roadmap translates the **CashFlow** Detailed Design into a series of small, review-friendly pull requests (PRs).  Each PR is intended to remain below ~800 lines of change, compiles independently, and keeps `master` green.

**Feature matrix flags**: `default`, `decimal128`, `parallel`, **`index`** (enables floating-leg builder).

---

## PR #1 — Bootstrap the `cashflow` crate

**Goals**
* Add new workspace member `cashflow` (`#![no_std]` default, MSRV 1.78).
* Establish CI matrix covering feature combos (`default`, `decimal128`, `parallel`, `index`).
* Create basic folder structure per design §4.

**Key changes**
1. Update root `Cargo.toml` workspace to include `cashflow`.
2. `cashflow/Cargo.toml` with features: `decimal128`, `parallel` (rayon optional).
3. `cashflow/src/lib.rs` facade re-exporting empty `mod` stubs (`cashflow`, `leg`, `npv`, etc.).
4. GitHub Actions: `cargo fmt`, `clippy`, `test` across feature matrix.

**Acceptance criteria**
* `cargo check` passes for all feature permutations.
* Zero clippy warnings (`-D warnings`).
* CI workflow green.

---

## PR #2 — Core enum `CFKind` & struct `CashFlow`

**Goals**
* Implement `CFKind` (`non_exhaustive`) with variants Fixed, FloatReset, Notional, Fee, Stub.
* Define `CashFlow` struct (`date`, `reset_date`, `amount: Money`, `kind`, `accrual_factor`).
* Provide basic `new` constructor + size assertions (≤ 48 bytes with `f64`).

**Key changes**
1. `cashflow.rs` — struct + trait derives (`Clone`, `Debug`, `PartialEq`).
2. Re-export `Money` from `primitives` crate (feature-gated Decimal).
3. Unit tests: size_of, round-trip clone, default Display via Debug.

**Acceptance criteria**
* `size_of::<CashFlow>() ≤ 48` bytes (`f64` path).
* `CashFlow::fixed_cf(date, amount)` compiles and stores fields.

---

## PR #3 — Factory helpers & principal / fee flows

**Goals**
* Add associated fns `CashFlow::principal_exchange` and `CashFlow::fee`.
* Implement validation (non-zero amount, date in valid range).
* Introduce `Error::Input("cashflow")` mapping.

**Key changes**
1. Extend `cashflow.rs` with factory fns.
2. Add `error.rs` (or reuse primitives error) wiring.
3. Tests for negative amount rejection.

**Acceptance criteria**
* `CashFlow::principal_exchange(date, amount)` returns flow with `CFKind::Notional`.
* Added `CFKind::PIK` and `CFKind::Amortization`; helpers: `pik_cf`, `amort_cf`.
* Invalid inputs return `Error::Input`.

---

## PR #4 — Fixed-leg builder & NPV helpers (v0.1 MVP)

**Goals**
* Implement `CashFlowLeg` struct + `fixed_rate()` builder API.
* Add `Discountable` trait + `npv` helper that consumes a stub `DiscountCurve` trait copy from `curves` crate.
* Provide criterion bench for leg NPV loop.

**Key changes**
1. `leg.rs` — builder pattern (`CashFlowLegBuilder`).
2. `npv.rs` — `Discountable` + `npv_portfolio` helpers.
3. Integration tests: generate semi-annual schedule via `dates::Schedule` and compute NPV with mock curve (df = 1).

**Acceptance criteria**
* `fixed_leg.npv(&FlatCurve::new(1.0))` equals sum of cash amounts.
* Workspace publishes `v0.1.0` tag after merge (CHANGELOG stub).

**Note**: Until PR #7 introduces `CFKind::Stub`, any irregular periods detected by the schedule builder are temporarily tagged as `CFKind::Fixed`; the later PR will retro-label them.

---

## PR #5 — `accrued` helper

**Goals**
* Expose `CashFlowLeg::accrued(val_date)` util.

**Key changes**
1. Extend `cashflow.rs` to store period `accrual_factor`.
2. Bench: accrue 1 M coupons within target thresholds.

**Acceptance criteria**
* Accrued interest monotone increasing between coupon dates (property test).

---

## PR #6 — Notional & amortisation schedules (C-38)

**Goals**
* Introduce `Notional` struct & `AmortRule` enum (None, Linear, Step).
* Implement `CashFlowLeg::apply_amortisation` modifying flows post-build.

**Key changes**
1. `notional.rs` — definitions & helpers (`par`, `linear_to`).
2. Extend `leg.rs` builder to accept `Notional` and call amort logic.
3. Tests: linear amort reduces principal to zero; step schedule sums ≤ initial notional.

**Acceptance criteria**
* `apply_amortisation` passes invariants; invalid rules error out.
* Bench amortising 10 k flows < 1 ms.

---

## PR #7 — Stub period detection & support (C-40)

**Goals**
* Detect irregular first/last periods in schedule builder.
* Mark corresponding flows with `CFKind::Stub` and store proper accrual fraction.

**Key changes**
1. Add `stub.rs` util; integrate into `leg.rs` generation loop.
2. Unit tests using short-front & long-back stub schedules.
3. Doc examples.

**Acceptance criteria**
* `flow.kind == CFKind::Stub` for irregular periods.
* Accrual factor equals actual/actual day-count for stub segment.

---

## PR #8 — Floating-rate leg builder (depends on index curves)

**Goals**
* Implement `CashFlowLeg::floating_rate()` builder supporting spread, gearing, reset lag.
* Generate `FloatReset` flows and coupon flows with `reset_date` populated.
* Feature-gated behind `index` until curves crate available.

**Key changes**
1. Extend `leg.rs` with floating builder.
2. Add `index` feature flag that adds dependency on `curves` crate for `ForwardIndexCurve` trait.
3. Integration tests with mock index curve (rate = 0).

**Acceptance criteria**
* PV of floating leg equals fixed leg when index rate matches fixed coupon (sanity test).
* Crate compiles without `index` flag.

---

## PR #9 — Parallel NPV (`parallel` feature)

**Goals**
* Enable Rayon `par_iter()` implementation for `npv_portfolio` when `parallel` feature enabled.
* Preserve single-thread performance when disabled.

**Key changes**
1. Conditionally include Rayon dep.
2. Benchmark: 2 M flows PV < 8 ms on 16-core machine.
3. CI job runs bench in allow-failure mode.

**Acceptance criteria**
* `cargo test --features parallel` passes and produces identical PV to scalar path.
* Single-thread path unchanged (< 1 % slowdown).

---

## PR #10 — Documentation & public-API audit → v0.3.0

**Goals**
* Finalise rustdoc examples, module-level docs, and README usage.
* Run `cargo public-api` & semver-checks; bump version to v0.3.0.

**Key changes**
1. Add `CHANGELOG.md` entries for 0.2 & 0.3.
2. `release.yml` GitHub workflow to tag release and publish crates.
3. Enforce `#![deny(missing_docs)]` in crate root.

**Acceptance criteria**
* `cargo publish --dry-run` succeeds for `cashflow` crate.
* Docs build with `-D warnings`.
* Tag `v0.3.0` pushed & CI green.

---

### Usage Tip
Create an umbrella issue "Implement cashflow module" and tick each PR as it merges to track progress. 