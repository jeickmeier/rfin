# Implementation Road-map — *Dates & Calendar Module* (v3 – built on the `time` crate)

This replaces the original 13-PR roadmap. Because we now leverage the `time` crate for all core calendrical functionality, only **five** focused pull-requests are required to deliver the finance-specific extensions.  Each PR is self-contained, keeps `master` compiling, and should stay below ~600 LoC for ease of review.

---

## PR #1 — Crate Bootstrap & `time` Re-export  ✅

**Goals**
* Add new workspace member `dates` which depends on `time = "0.3"`.
* Re-export key `time` types (`Date`, `OffsetDateTime`, `PrimitiveDateTime`) behind our facade.
* Wire up CI (build matrix for `default`, `std`, `serde`, `holidays` features).

**Key changes**
1. Update root `Cargo.toml` to include `dates` crate.
2. `dates/Cargo.toml` with feature flags mirroring `time` (`std`, `serde`).
3. `dates/src/lib.rs` re-exports & stub sub-modules.
4. GitHub Actions: `cargo fmt`, `clippy`, `test` across feature combos.

**Acceptance criteria**
* `cargo check` succeeds for all feature permutations.
* Zero warnings under `cargo clippy`.
* CI pipeline green.

---

## PR #2 — Extension Traits (`ext.rs`)  ✅

**Goals**
* Introduce `DateExt` and `OffsetDateTimeExt` providing convenience methods:
  * `is_weekend`, `quarter`, `fiscal_year`, `add_business_days`, …
* Implement these as `#[inline]` wrappers around `time` APIs.
* Unit tests for every added method.

**Acceptance criteria**
* `Date::is_weekend()` matches expected results for 1970-01-01→2030-12-31 sample.
* All methods are `no_std` compatible.

---

## PR #3 — Day-count Conventions (`daycount.rs`)  ✅

**Goals**
* Provide `DayCount` enum with `days()` and `year_fraction()` functions supporting: ACT/360, ACT/365F, 30/360, 30E/360, ACT/ACT
* Reference implementations validated against ISDA golden cases.
* No heap allocations; panic-free API returning `Result<f64, Error>`.

**Acceptance criteria**
* `year_fraction(DayCount::Act360, d, d+360d) == 1.0 ±1e-9`.
* 100 % branch coverage on algorithms.

---

## PR #4 — Holiday Calendars & Business-Day Adjustment (`calendar.rs`)   ✅

**Goals**
* Define `trait HolidayCalendar { fn is_holiday(&self, Date) -> bool }`.
* Add built-in `Target2` calendar behind `holidays` feature.
* Implement `BusDayConv` enum + `adjust(date, conv, cal)` helper.

**Acceptance criteria**
* `Target2.is_holiday(date!(2025-01-01))` is `true`.
* `adjust` passes month-end roll tests across conventions.
* Feature compiles out completely when `holidays` disabled (<5 B diff in `.rlib`).

---

## PR #5 — Schedule Builder DSL (`schedule.rs`)   ✅

**Goals**
* Implement `ScheduleBuilder` fluent API producing `SmallVec<[Date; 32]>` schedules.
* Support frequencies, stub rules, optional business-day adjustment.
* Extend `Frequency` enum with `periods_per_year()` helper.

**Acceptance criteria**
* Generating semi-annual schedule 2025-01-15→2030-01-15 returns 11 dates.
* Generating 10k schedules (20 periods each) < 5 ms single-thread (bench).


---

## PR #6 — Composite Calendars (`cal/composite.rs`)   ✅

**Goals**
* Provide `CompositeCalendar` with merge modes `Union` (default) & `Intersection`.
* Blanket-implement `HolidayCalendar` for composites.
* Useful for cross-currency swaps or bonds that might reference two calendars.

**Acceptance criteria**
* `CompositeCalendar::merge(&[TARGET2, LSE])` union vs intersection unit tests pass.

---

## PR #7 — IMM / Quarterly Helpers (`imm.rs`)

**Goals**
* Add `third_wednesday(month, year)` and `next_imm(date)` helpers.
* Add `next_cds_date(date)` helpers
* Re-export via crate root for downstream derivatives pricing.

**Acceptance criteria**
* `third_wednesday(3, 2025)` == `2025-03-19`.
* `next_imm(date!(2025-03-20))` == `2025-06-18`.
* `next_cds_date(date!(2025-03-10))` == `2025-03-20`

---

## PR #8 — Documentation & Release v0.1.0

**Goals**
* Finalise rustdoc, README badges.

**Acceptance criteria**
* `cargo publish --dry-run` succeeds.
* Docs build with `-D warnings`.

---
