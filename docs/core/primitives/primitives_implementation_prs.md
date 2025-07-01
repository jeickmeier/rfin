# Implementation Road-map — *Primitives Crate*

This document enumerates the focused pull-requests that will take the **Primitives** Technical Design Document from design to production-ready code.  Each PR is self-contained, keeps `master` compiling, and remains below ~800 lines for easy review.

---

## PR #1 — Bootstrap the `primitives` crate -- DONE

**Goals**
* Create new workspace member `primitives` (no-std by default, MSRV 1.78).
* Add CI jobs to build with and without feature flags.
* Lay down scaffold for future modules (see TDD §3).

**Key changes**
1. Update root `Cargo.toml` to add workspace member.
2. Add `primitives/Cargo.toml` with features `std`, `decimal128`, `serde`.
3. Add `primitives/src/lib.rs` with `#![no_std]` and re-exports.
4. Extend CI (`cargo fmt`, `clippy`, `test`).

**Acceptance criteria**
* Workspace builds with `cargo check` for all feature combos.
* `cargo test -p primitives` passes (zero tests placeholder).
* CI green.

---

## PR #2 — Add core enum `Currency` -- DONE 

**Goals**
* Implement ISO-4217 enum with numeric discriminants.
* Provide `FromStr`, `Display`, and `serde` impls (gated by feature).
* Generate lookup table at build-time.

**Key changes**
1. `currency.rs` — enum + helpers (`minor_units`).
2. `macros.rs` — internal `impl_display_fromstr!`.
3. `build.rs` — parse `iso_4217.csv` into `const` table.
4. Unit tests: round-trip parse/format, size check, fuzz harness stub.

**Acceptance criteria**
* `assert_eq!(Currency::USD as u16, 840)`.
* `Currency::from_str("eur")? == Currency::EUR`.
* `size_of::<Currency>() == 2`.
* All tests pass with/without `std`, `serde`.

---

## PR #3 — Introduce `Money<F>` -- DONE

**Goals**
* Generic struct with default `f64`.
* Arithmetic ops with currency guard (`Add`, `Sub`, `Mul<f64>`, `Div<f64>`).
* Compile-time failure on mismatched currencies.
* Feature-gated `Decimal` support.

**Key changes**
1. `money.rs` — struct, trait impls, helper ctors.
2. `error.rs` — extend `Error::Input` variants.
3. `decimal128` feature support via `rust_decimal`.
4. Tests: arithmetic, overflow, serde snapshot.

**Acceptance criteria**
* `Money::new(100.0, USD) + Money::new(50.0, USD)` compiles.
* Adding EUR to USD fails to compile (`trybuild` test).
* `cargo bench` shows ≤5 ns add latency (release).

---

## PR #4 — Unified error type `primitives::Error`

**Goals**
* Create non-exhaustive `enum Error` with three categories.
* Implement `core::fmt::Display`, `std::error::Error` (behind `std`).
* Provide `alloc` fallback for `no_std + alloc`.

**Key changes**
1. `error.rs` — definition + feature gates.
2. Re-export in `lib.rs`.
3. Convert previous misuse in `Money` to new `Error`.
4. Add doctest examples.

**Acceptance criteria**
* `#![no_std]` build uses zero heap unless `alloc` enabled.
* `Error` implements `Clone + Debug + Eq`.
* Doctest compilation passes.

---

## PR #5 — Re-export day-count & frequency enums

**Goals**
* Introduce `DayCount`, `Frequency`, `BusDayConv` with exact `repr(u8)`.
* Derive `Serialize`/`Deserialize` behind `serde`.

**Key changes**
1. `date_key.rs` (or `date.rs`) — enum definitions.
2. Tests for discriminant stability.
3. Update docs.

**Acceptance criteria**
* `size_of::<DayCount>() == 1`.
* Serde round-trip for every variant.
* MSRV check passes.

---

## PR #6 — Implement `PeriodKey`

**Goals**
* Provide 12-byte cache key struct with custom `fxhash`.
* Implement `Copy`, `Eq`, `Hash`, `Ord`.

**Key changes**
1. Add `fxhash` dependency (`default-features = false`).
2. `date_key.rs` — struct + custom `Hash` impl.
3. `static_assertions` to validate size/alignment.
4. Criterion benchmark of hashing vs std.

**Acceptance criteria**
* `size_of::<PeriodKey>() == 12`.
* Hash throughput ≥5× std hash in benchmark.

---

## PR #7 — Add `Notional` & `AmortRule` stubs

**Goals**
* Define basic representations to break future cycles with `cashflow`.
* Builders will live in `cashflow`; here only storage types.

**Key changes**
1. `notional.rs` — enums/structs as per TDD.
2. Derive traits, add placeholder docs.
3. Unit tests for `Money` link.

**Acceptance criteria**
* Compiles without `cashflow` crate.
* `AmortRule::Linear` stores `Money`.

---

## PR #8 — Minimal `DiscountCurve` trait

**Goals**
* Provide trait signature; no implementation yet.
* Define feature gate for forward compatibility.

**Key changes**
1. `curve.rs` — trait, re-export `Date` once `dates` crate exists.
2. Document future `curves` integration.
3. Add compile-fail test to forbid blanket impl for foreign types.

**Acceptance criteria**
* Trait object-safety asserted.
* `cargo doc --all-features` shows public API.

---

## PR #9 — Serde versioning + Public-API audit → v0.1.0

**Goals**
* Add `#[serde(version = 1)]` to all public structs/enums.
* Run `cargo semver-checks` (CI) and freeze API.
* Tag release `v0.1.0` with CHANGELOG.

**Key changes**
1. Attribute additions across modules.
2. GitHub Action for `cargo semver-checks`.
3. `CHANGELOG.md` with migration notes.

**Acceptance criteria**
* `cargo publish --dry-run` succeeds.
* Release tag pushed after merge.

---

### Usage Tip
Create an umbrella issue "Implement primitives crate" and tick each PR as it merges to track progress. 