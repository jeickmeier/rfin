# Implementation Road-map — *Dates Module*

This document enumerates the focused pull-requests that will convert the **Dates** Detailed Design Document into production-ready code.  Each PR is self-contained, keeps `master` compiling, and remains below ~800 LoC for easier review.

---

## PR #1 — Bootstrap the `dates` crate -- DONE

**Goals**
* Add new workspace member `dates` (`#![no_std]` by default, MSRV 1.78).
* Wire up CI jobs (build matrix for `default`, `std`, `serde`, `holidays`).
* Create initial file/folder layout described in TDD §4.

**Key changes**
1. Update root `Cargo.toml` to include `dates` crate.
2. `dates/Cargo.toml` with feature flags: `std`, `serde`, `holidays`.
3. `dates/src/lib.rs` facade re-exporting sub-modules (all `mod` stubs).
4. GitHub Actions: `cargo fmt`, `clippy`, `test` across feature combos.

**Acceptance criteria**
* `cargo check` succeeds for all feature permutations.
* Zero warnings under `cargo clippy --all-targets --all-features`.
* CI pipeline green.

---

## PR #2 — Core `Date` struct

**Goals**
* Implement opaque `Date` type backed by `days_since_epoch: i32`.
* Provide `const fn new_unchecked` (private) and basic getters (`year`, `month`, `day`).
* Derive `Copy`, `Clone`, `Eq`, `Ord`, `Hash`, `Default`.

**Key changes**
1. `date.rs` — struct definition + trait impls.
2. Add basic unit tests (`size_of::<Date>() == 4`).
3. Export `Date` in `lib.rs`.

**Acceptance criteria**
* Building with `no_std` succeeds.
* `Date` is 4 bytes and `Copy`.
* Tests pass on stable & nightly.

---

## PR #3 — Validation & public constructors

**Goals**
* Implement safe, public `Date::new(y, m, d)` with full Gregorian validation.
* Add `Date::from_epoch_days(i32)` & `Date::epoch()` helpers.
* Introduce `DateError` enum (`non_exhaustive`).

**Key changes**
1. `date.rs` — validation logic using Howard Hinnant algorithm.
2. `error.rs` — `DateError` + `core::fmt::Display` & (behind `std`) `std::error::Error`.
3. Property-based tests for leap years (1900–2400).

**Acceptance criteria**
* `Date::new(2024, 2, 29)` is `Ok`, whereas `Date::new(2023, 2, 29)` is `Err`.
* All validation is `const fn` where possible (build-time assert example compiled).
* 100% branch coverage on constructor path.

---

## PR #4 — ISO-8601 parsing & formatting

**Goals**
* Implement `FromStr` and `TryFrom<&str>` for `Date`.
* Add `Display` impl producing canonical `YYYY-MM-DD`.
* Provide compile-time parsing via `const fn` when literal string supplied.

**Key changes**
1. `parse.rs` — fast, allocation-free parser.
2. `fmt.rs` — `Display` + helper to write into `[u8; 10]`.
3. Round-trip tests + fuzz harness stub.

**Acceptance criteria**
* `"2025-06-29".parse::<Date>()?` equals `Date::new(2025,6,29)?`.
* `format!("{date}")` returns `2025-06-29`.
* Parser rejects malformed inputs under 100 ns (criterion bench).

---

## PR #5 — Arithmetic traits

**Goals**
* Supply `AddDays`, `AddWeeks`, `AddMonths`, `AddYears` traits with checked arithmetic.
* Overload `+` / `-` for day operations via newtype `Days`.
* Ensure overflow panics in debug but wraps safely in release (cfg guard).
* Use the `DayCount`, `Frequency`, and `BusDayConv` enums **re-exported from the primitives crate** (no duplicate definitions).

**Key changes**
1. `arithmetic.rs` — trait definitions + impls.
2. Extend `date.rs` with helper `const fn is_leap_year`.
3. Tests: adding 1 day over month ends, subtracting across leap year.

**Acceptance criteria**
* `Date::new(2025,12,31)? + 1.day()` == `Date::new(2026,1,1)?`.
* No heap allocations in arithmetic paths (verified via `alloc_counter`).
* Benchmarks show ≤ 30 ns for +1 day.

---

## PR #6 — Weekday & ISO week utilities

**Goals**
* Implement `Weekday` enum and `Date::weekday()` O(1) function.
* Provide `is_weekend()`, `quarter()`, `quarter_start()` helpers.
* Add `iso_week()` returning `(year, week_no)`.

**Key changes**
1. `weekday.rs` — enum + methods.
2. `date.rs` — new query fns wired into core.
3. Unit tests for every weekday across 1970-01-01 → 2030-12-31 sample.

**Acceptance criteria**
* `Date::epoch().weekday() == Weekday::Thu`.
* `Date::new(2025,6,29)?.quarter() == 2`.
* 100% line coverage on new module.

---

## PR #7 — `serde` support *(feature-gated)*

**Goals**
* Conditionally derive `Serialize` / `Deserialize` for `Date`, `Weekday`, error types.
* Use versioned representation (`#[serde(tag = "type", version = 1)]`).
* Provide example in docs.

**Key changes**
1. Update `Cargo.toml` features & optional `serde` dep.
2. `serde.rs` or inline attrs in modules.
3. Serde round-trip tests for JSON & bincode.

**Acceptance criteria**
* `serde_json::to_string(&Date::epoch())? == "\"1970-01-01\""`.
* Crate compiles without `serde` flag.
* Semver compatibility tests added to CI.

---

## PR #8 — `chrono` & `std` interop *(std feature)*

**Goals**
* Implement `From/Into` conversions to `chrono::NaiveDate`.
* Add `TryFrom<Date>` for `time::PrimitiveDateTime` when `std`.
* Provide quick-start example in docs.

**Key changes**
1. `interop.rs` — conversions under `cfg(feature = "std")`.
2. Integration tests linking both crates.
3. Update README badges for `std` feature.

**Acceptance criteria**
* `chrono::NaiveDate::from(date) == ...` round-trip passes.
* MSRV enforced after enabling `std`.

---

## PR #9 — Holiday calendar strategy trait *(holidays feature)*

**Goals**
* Define `trait HolidayCalendar { fn is_holiday(&self, Date) -> bool }`.
* Provide built-in `Target2` implementation behind `holidays`.
* Expose `adjust(date, conv, cal)` convenience function.

**Key changes**
1. `holiday.rs` — trait + `Target2` struct.
2. `bus_day.rs` — `BusDayConv` enum + adjustment logic.
3. Parametric tests for 2025 TARGET2 holidays.

**Acceptance criteria**
* `cal.is_holiday(Date::new(2025,1,1)?)` is `true`.
* `adjust` respects `ModFollowing` rule across month end.
* Feature compiles out completely when `holidays` disabled (<5 bytes diff in .rlib).

---

## PR #10 — Schedule builder DSL

**Goals**
* Implement `ScheduleBuilder` fluent API (see TDD §6a).
* Support stub rules, frequency, business-day adjustment.
* Return `SmallVec<[Date; 32]>` internally to avoid heap.

**Key changes**
1. `schedule.rs` — builder + `Schedule` struct.
2. Add `Frequency`, `StubRule` enums in new `frequency.rs`.
3. Integration tests generating semi-annual schedule between 2025-2030.

**Acceptance criteria**
* `Schedule::len()` returns 11 for example in design doc.
* Generating 10 k schedules (20 periods each) < 5 ms single-thread (bench).
* Public API compiles without `holidays` feature (calendar parameter optional).

---

## PR #11 — Day-count & year-fraction utilities

**Goals**
* Provide `daycount::year_fraction(dc, d1, d2)` implementing ACT/360, ACT/365F, 30E/360.
* Return `Result<f64, DateError>` when `d2 < d1` unless flag.
* Inline zero-panic algorithms.

**Key changes**
1. `daycount.rs` — algorithms.
2. Extend `Frequency` impl with `periods_per_year()`.
3. Unit tests using known ISDA gold cases.

**Acceptance criteria**
* `year_fraction(DayCount::Act360, d, d+360d) == 1.0` ±1e-9.
* `cargo test --all-features` green.

---

## PR #12 — Performance, fuzzing & benchmarks

**Goals**
* Add Criterion benches comparing `Date` vs `chrono::NaiveDate` for construction, parsing, arithmetic.
* Integrate AFL-rs fuzz targets for parser and arithmetic overflow cases.
* Gate nightly-only jobs in CI.

**Key changes**
1. `benches/` folder with benchmarks.
2. `fuzz/` folder with corpus seeds.
3. GitHub Workflow `bench.yml` + `fuzz.yml` (nightly, allow-failure).

**Acceptance criteria**
* Benchmarks demonstrate ≥3× speedup over chrono parsing.
* Fuzzing runs 60 s in CI without crash.

---

## PR #13 — Documentation, public-API audit → v0.1.0

**Goals**
* Complete rustdoc examples & module-level docs.
* Run `cargo public-api` and semver-checks; freeze API.
* Prepare CHANGELOG and tag `v0.1.0`.

**Key changes**
1. Add README badges & usage example.
2. `CHANGELOG.md` with migration notes.
3. GitHub Release draft via `release.yml` action.

**Acceptance criteria**
* `cargo publish --dry-run` succeeds for all crates.
* Documentation builds without warnings (`RUSTDOCFLAGS='-D warnings'`).
* Tag `v0.1.0` pushed after merge.

---

### Usage Tip
Create an umbrella issue "Implement dates module" and tick each PR as it merges to track progress. 