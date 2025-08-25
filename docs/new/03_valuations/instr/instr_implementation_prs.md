# Implementation Road-map — *Instruments Crate*

This document decomposes the **Instruments** design into a sequenced series of focused pull-requests.  Each PR keeps `master` compiling, remains reviewable (≤ 800 LOC changed), and adds tests + docs.  The numbering purposefully follows the order in which lower-level functionality is required by downstream instruments.

---

## PR #1 — Bootstrap `instr` Crate & Common Scaffolding

**Goals**
* Add new workspace member `instr` (no-std by default, MSRV 1.78).
* Create module tree & feature flags (`parallel`, `serde`, `private_credit`).
* Re-export `Priced`, `CurveProvider`, and error types.

**Key changes**
1. Update root `Cargo.toml` workspace.
2. Add `instr/Cargo.toml` with dependencies on `primitives`, `cashflow`, `curves`.
3. `lib.rs` with `#![no_std]`, feature gates, and `mod common` placeholder.
4. CI jobs (`clippy`, `fmt`, feature matrix).

**Acceptance criteria**
* `cargo check` passes for all feature combos.
* Empty test suite runs & CI green.

## PR #1b — ValuationContext plumbing

**Goals**
* Introduce `ValuationContext` struct and blanket `impl CurveProvider for ValuationContext` so that downstream instrument PRs compile.
* Re-export the struct from `primitives` temporarily until a dedicated crate for shared runtime objects is added.

**Key changes**
1. Add new module `context.rs` inside `primitives` containing the struct definition shown in the core TDD §2.6.
2. Implement `CurveProvider` for `&ValuationContext` (simple delegation).
3. Update workspace `Cargo.toml` dependency graph (`instr`, `risk`, `calibration`) to use the new module.

**Acceptance criteria**
* `cargo check` continues to succeed for all feature combos after the change.
* Dummy unit test can construct a `ValuationContext` with stub `MarketDataProvider` and call `discount()` via the blanket impl.

---

## PR #2 — Common Helpers & Builders (from *common_features.md*)

**Goals**
* Implement shared enums/structs (`Side`, `CallSchedule`, etc.).
* Add blanket `pv_slice` / `pv_slice_parallel` impl.
* Provide base `Builder` typestate pattern.

**Key changes**
1. `common.rs` with helper types + Serde derives.
2. `builder.rs` generic typestate scaffolding + macro to reduce boilerplate.
3. Unit tests for validation helper & typestate transitions.

**Acceptance criteria**
* `pv_slice_parallel` benchmarks show ≥ 5× speed-up on 8-core machine when `parallel` flag.
* All helpers `Send + Sync` and zero unsafe.

---

## PR #3 — Implement `SpotAsset` (C-09)

**Goals**
* Introduce `SpotAsset` struct, fluent builder, and analytic NPV.

**Key changes**
1. `spot.rs` with struct + `Priced` impl.
2. Add tests: PV sign, currency guard, Serde round-trip.
3. Criterion bench: 1 M SpotAsset PVs < 1 ms.

**Acceptance criteria**
* `SpotAsset::pv` returns analytical result within 1e-9 rel. tol vs formula.
* Builder fails to compile on missing mandatory fields (`trybuild`).

---

## PR #4 — Money-Market Contracts (Deposit, FRA, Future) (C-10/C-11)

**Goals**
* Add `Deposit`, `FRA`, `Future` structs with shared accrual helper.
* Provide carry/rolldown analytics.

**Key changes**
1. `mm.rs` with enum `MoneyMarketInstr` + individual structs.
2. Accrual helper in `mm_common.rs`.
3. Tests: golden PV vs QuantLib; rate-PV monotonicity property.

**Acceptance criteria**
* `cargo bench` deposit PV < 15 ns.
* Forward DV01 matches finite-diff within 0.1 bp.

---

## PR #5 — Interest-Rate Swaps (C-23)

**Goals**
* Implement `Swap` with fixed & float `CashFlowLeg` support.
* Add par-rate solver & carry/rolldown analytics.

**Key changes**
1. `swap.rs` struct + `Priced` impl.
2. `swap_analytics.rs` extension trait.
3. Integration test: 1 k random swaps PV parity fixed-vs-float.

**Acceptance criteria**
* Par-rate root-finder converges < 6 iterations avg.
* Bench: price 10 k swaps < 25 ms single-thread.

---

## PR #6 — Caps & Floors (C-24)

**Goals**
* Add `CapFloor` struct (cap or floor) priced via Black model.
* Provide vega & implied-vol helpers.

**Key changes**
1. `capfloor.rs` with generator from floating leg.
2. Link to vol2D surface in `curves` via `vol_surface_id`.
3. Tests: cap-floor parity, implied-vol accuracy.

**Acceptance criteria**
* Parity error < 1e-8.
* Implied vol root--find < 1e-4 precision.

---

## PR #7 — Swaptions (C-24 continuation)

**Goals**
* Introduce `Swaption` struct with Black-76 pricing.
* Support SABR vol lookup from 3-D surface.

**Key changes**
1. `swaption.rs` + `swaption_surface.rs` access layer.
2. Vega / delta analytic functions.
3. Tests vs Bloomberg examples.

**Acceptance criteria**
* Delta difference vs finite diff < 0.5 bp notional.

---

## PR #8 — Bonds Framework (C-25)

**Goals**
* Add `Bond` enum variants: Fixed, Floating, Callable.
* Implement coupon generation & clean/dirty price helpers.

**Key changes**
1. `bond.rs` + leg builders.
2. `bond_analytics.rs` with YTM, duration, convexity.
3. Tests: UST, corporate, callable muni golden prices.

**Acceptance criteria**
* `yield_to_maturity` converges < 1e-8.
* Clean – dirty price diff equals accrued within 1e-10.

---

## PR #9 — Inflation-Linked Bonds (C-26)

**Goals**
* Implement `InflBond` adjusting coupons and principal by CPI.

**Key changes**
1. `infl_bond.rs` struct + CPI lookup helper.
2. Tests: US TIPS PV vs Treasury data.

**Acceptance criteria**
* CPI interpolation accuracy < 0.01 index points.

---

## PR #10 — Inflation Swaps (C-27)

**Goals**
* Add `InflationSwap` supporting ZC & YY variants.

**Key changes**
1. `infl_swap.rs` with enum & valuation.
2. Tests: PV zero at par coupon.

**Acceptance criteria**
* ZC vs YY parity within 0.1 bp.

---

## PR #11 — XCCY Basis Swaps (C-28)

**Goals**
* Introduce `XccyBasisSwap` with FX forward integration.

**Key changes**
1. `xccy_basis.rs` struct; FX curves link.
2. Tests: CIP parity scenario.

**Acceptance criteria**
* Parity error < 1e-7 of notional.

---

## PR #12 — FX Forwards & Options (C-29)

**Goals**
* Implement `FxForward` and `FxOption` (Garman–Kohlhagen).

**Key changes**
1. `fx.rs` module with instrument structs.
2. Vol surface 2-D lookup & delta conventions.
3. Tests vs market examples.

**Acceptance criteria**
* ATM option PV within 0.1 % vs Bloomberg.

---

## PR #13 — Credit Default Swaps (C-30)

**Goals**
* Add `Cds` struct with premium & protection legs + PV01/CS01 analytics.

**Key changes**
1. `cds.rs` with hazard curve integration.
2. Tests: ISDA CDS standard cases.

**Acceptance criteria**
* PV error < 0.5 bp on test cases.

---

## PR #14 — Equity Options (C-31)

**Goals**
* Implement `EquityOption` supporting European & American styles.

**Key changes**
1. `equity_option.rs` + CRR binomial lattice helper.
2. Tests: parity & early exercise boundary.

**Acceptance criteria**
* American–Euro PV difference matches Hull table within 1 cent.

---

## PR #15 — Convertible Bonds (C-32)

**Goals**
* Add `ConvertibleBond` combining bond + embedded equity call option.

**Key changes**
1. `conv_bond.rs` struct & iterative solver.
2. Tests: yield convergence example.

**Acceptance criteria**
* Solver residual < 1e-8.

---

## PR #16 — Repos / Securities-Financing (C-51)

**Goals**
* Implement `RepoTrade` with haircut and margining schedule placeholders.

**Key changes**
1. `repo.rs` PV logic & effective rate helper.
2. Tests: repo rate formula.

**Acceptance criteria**
* Effective rate matches analytic derivation within 1e-10.

---

## PR #17 — Commodity Futures & Forwards (C-52)

**Goals**
* Add `CommodityFuture` & `CommodityForward` structs.

**Key changes**
1. `commodity.rs` with storage / convenience-yield curves.
2. Tests: WTI CL futures curve replication.

**Acceptance criteria**
* Implied convenience-yield error < 0.5 bp.

---

## PR #18 — Bermudan Swaps (C-55)

**Goals**
* Introduce `BermudanSwap` data structure and placeholder pay-off schedule.

**Key changes**
1. `berm_swap.rs` struct + Serde.
2. Tests: schedule validation & serialization.

**Acceptance criteria**
* Schedule ordering lint passes; round-trip size stable.

---

## PR #19 — Documentation & public-API audit → v0.3.0

**Goals**
* Finish rustdoc, run `cargo public-api`, tag `v0.3.0`.

**Acceptance criteria**
* `cargo publish --dry-run` succeeds; docs `-D warnings` clean.