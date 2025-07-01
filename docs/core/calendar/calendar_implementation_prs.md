# Implementation Road-map — *Calendar Module*

This document breaks down the **Calendar** Detailed Design into a sequence of focused pull-requests (PRs).  Each PR is self-contained, keeps `master` compiling, and aims to stay below ~800 lines to simplify code review.

---

## PR #1 — Bootstrap the `calendar` crate

**Goals**
* Add new workspace member `calendar` (defaults to `no_std`, MSRV 1.78).
* Establish CI matrix for feature permutations (`default`, `std`, `holidays`, `ical`).
* Lay out initial folder structure as per design §4.

**Key changes**
1. Update workspace `Cargo.toml` to include `calendar` crate.
2. `calendar/Cargo.toml` with feature flags: `std`, `ical`, `holidays`.
3. `calendar/src/lib.rs` facade re-exporting placeholder `mod` stubs (`codes`, `holiday_set`, `weekend`, etc.).
4. GitHub Actions: `cargo fmt`, `clippy`, `test` across feature matrix.

**Acceptance criteria**
* `cargo check` passes for all feature combinations.
* Zero clippy warnings (`-D warnings`).
* CI pipeline green.

---

## PR #2 — Core Enums `CalCode` & `WeekendRule`

**Goals**
* Implement `CalCode` (`repr(u16)`) with built-in codes `TARGET2`, `NYSE`, `LSE`, `CME`, `ICE`, `LME`.
* Provide `WeekendRule` enum + `const fn is_weekend(Date)`; include patterns `SatSun`, `FriSat`, `SunThu`, `Custom(BitMask)`.
* Expose `CalCode::load()` placeholder returning `Result<&'static HolidaySet, CalError>`.

**Key changes**
1. `codes.rs` — define `CalCode`, add docs, derive common traits.
2. `weekend.rs` — enum, bit-mask logic, unit tests.
3. Export in `lib.rs`.

**Acceptance criteria**
* `size_of::<CalCode>() == 2`.
* `WeekendRule::SatSun.is_weekend(d)` returns true for Sat/Sun sample dates.
* Doctest examples compile.

---

## PR #3 — Error Handling & CalError Enum

**Goals**
* Create non-exhaustive `CalError` (`InvalidCalCode`, `MissingBlob`, `DateOutOfRange`, `ParseError`).
* Implement `core::fmt::Display`, and `std::error::Error` when `std`.
* Provide `From<CalError> for primitives::Error` mapping to `Error::Calendar`.

**Key changes**
1. `error.rs` — enum + impls.
2. Update previous code to use new error type.
3. Unit tests for `Display` strings.

**Acceptance criteria**
* Builds with and without `std`.
* `assert_eq!(format!("{}", CalError::InvalidCalCode), "invalid calendar code")`.

---

## PR #4 — `HolidaySet` Data Structure & `is_business_day`

**Goals**
* Define `HolidaySet` struct storing `year_span`, `&'static [i32] holidays`, and `WeekendRule`.
* Implement methods `is_business_day`, `is_holiday`, `year_span()`.
* Provide initial dummy data for `TARGET2` (1970–1971) for testing.

**Key changes**
1. `holiday_set.rs` — struct definition + logic.
2. Add `HolidayCalendar` trait import from `dates` crate and implement for `HolidaySet`.
3. Unit tests: weekend vs holiday behaviour.

**Acceptance criteria**
* `!set.is_business_day(2025-01-01)` for dummy TARGET2 data.
* Binary search branch-free implementation passes criterion micro-bench ( < 50 ns per call ).

---

## PR #5 — Binary Blob Loader (`build.rs` + `blob.rs`)

**Goals**
* Implement `build.rs` that embeds `calendars.bin` via `include_bytes!` when `holidays` feature enabled.
* Provide runtime loader mapping `CalCode` → `&'static HolidaySet`.
* Validate blob header magic & version on load.

**Key changes**
1. `build.rs` in crate root generating minimal binary for TARGET2 & NYSE sample years.
2. `blob.rs` — `BlobIndex` struct and parse logic.
3. Extend `CalCode::load()` to use blob.

**Acceptance criteria**
* `CalCode::TARGET2.load()?` returns `HolidaySet` covering 2025.
* Mismatch version triggers `CalError::MissingBlob`.
* `cargo test --features holidays` green.

---

## PR #6 — Business-Day Conventions (`BusDayConv`) & `adjust`

**Goals**
* Add `BusDayConv` enum (`Following`, `ModFollowing`, `Preceding`, `ModPreceding`, `None`).
* Implement `HolidaySet::adjust(date, conv)` and blanket impl for `HolidayCalendar` objects.

**Key changes**
1. `adjust.rs` — algorithms (≤ 7 date checks worst-case).
2. Extend unit tests covering all conventions across weekend & holiday scenarios.
3. Update docs with examples.

**Acceptance criteria**
* `adjust(2025-07-05, Following)` returns `2025-07-07` for TARGET2 sample.
* No heap allocation in hot path (verified via `alloc_counter`).
* 100% branch coverage of convention switch.

---

## PR #7 — `add_business_days` & Date Arithmetic Helpers

**Goals**
* Provide `add_business_days(date, n)` supporting ±i32 days.
* Optimise using weekday pre-computation; panic-free.

**Key changes**
1. Extend `adjust.rs` or new `arith.rs` with algorithm.
2. Criterion benchmarks vs naive loop.
3. Property tests to ensure symmetry (`add_business_days(d, x) -> add_business_days(d, -x)` inverse).

**Acceptance criteria**
* Adding 10 k business days < 150 µs (bench target).
* All property tests pass.

---

## PR #8 — `CompositeCalendar` Implementation

**Goals**
* Implement `CompositeCalendar<'a>` with merge modes `Union` (default) & `Intersection`.
* Support up to 4 component calendars on stack via `SmallVec`.
* Implement `HolidayCalendar` for composite.

**Key changes**
1. `composite.rs` — struct + logic.
2. Unit tests: composite TARGET2+LSE union vs intersection.
3. Benchmark overhead vs single calendar (<2×).

**Acceptance criteria**
* `CompositeCalendar::merge(&[TARGET2, LSE]).is_business_day(d)` matches expected truth table.
* `size_of::<CompositeCalendar>()` ≤ 64 bytes for ≤4 components.

---

## PR #9 — IMM & Quarterly Helpers

**Goals**
* Add `imm.rs` module providing `third_wednesday(month, year)` calc and `next_imm(date)` helper.
* Re-export via crate root.

**Key changes**
1. `imm.rs` algorithm (no look-up tables).
2. Unit tests comparing against CME 2024-2026 IMM dates.
3. Doc examples.

**Acceptance criteria**
* `third_wednesday(3, 2025)` == `2025-03-19`.
* `next_imm(Date::new(2025,3,20)?)` == `2025-06-18`.

---

## PR #10 — iCalendar Parser *(`ical` feature)*

**Goals**
* Integrate `ical` crate to parse `*.ics` files into JSON manifest during CLI/build.
* Support DTSTART, RRULE, EXDATE parsing.

**Key changes**
1. `parser.rs` — iCalendar ingest and normalisation.
2. Extend `build.rs` to call parser when `ical` enabled.
3. Fuzz harness for parser (AFL-rs).

**Acceptance criteria**
* Parsing official ECB TARGET2 *.ics* yields expected 2025 holiday count.
* Fuzz target runs 30 s without panic.

---

## PR #11 — CLI Tool `rustfin-cal-sync` *(std feature)*

**Goals**
* Introduce CLI in `calendar/cli/` (independent crate if needed).
* Commands: `pull <CODE>`, `diff`, `validate`, `json` per design §7a.
* Non-interactive by default; `--yes` auto-confirms.

**Key changes**
1. Use `clap` + `reqwest` (behind `std`) to fetch remote *.ics*.
2. Wire into parser & build-script JSON generation.
3. GitHub Action workflow to run `cal-sync validate` in CI.

**Acceptance criteria**
* `cargo run --bin rustfin-cal-sync -- pull TARGET2 --yes` exits 0.
* `validate --all` passes on repo copy.
* CLI excluded from `no_std` build.

---

## PR #12 — Documentation, public-API audit → v0.1.0

**Goals**
* Finalise rustdoc examples, module-level docs, README badges.
* Run `cargo public-api` & semver checks.
* Tag release `v0.1.0` with CHANGELOG & release notes.

**Key changes**
1. Add `CHANGELOG.md` and migration section.
2. `release.yml` GitHub workflow to draft release notes.
3. Audit feature-flag coverage; ensure `#![deny(missing_docs)]` passes.

**Acceptance criteria**
* `cargo publish --dry-run` succeeds for calendar crate.
* Docs build with `-D warnings`.
* Tag pushed & CI green.

---

### Usage Tip
Create an umbrella issue "Implement calendar module" and reference each PR.  Tick them off as they merge to visualise progress. 