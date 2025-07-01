# Dates Module – Detailed Design Document

## 1 Overview
The **dates** module provides a foundational API for representing, manipulating, and validating calendar dates within the RustFin codebase. It is designed to be:

* 📦 **Self-contained** – no external crates beyond the standard library and the de-facto chrono crate (behind an optional `chrono` feature flag).
* 🦀 **Idiomatic Rust** – strong typing, zero-cost abstractions, trait-based design, exhaustive `enum` usage, and `Result`/`Error` handling.
* ⚖️ **Precise & Correct** – compliant with ISO-8601 and proleptic Gregorian calendar rules.
* 🚦 **High-performance** – constexpr-style `const fn` where possible and zero runtime allocations for core types.

## 2 Goals & Non-Goals
### 2.1 Goals
1. Represent a calendar date (year, month, day) as an *infallible* value type.
2. Provide ergonomic construction/parsing from common formats: ISO-8601 (`YYYY-MM-DD`), epoch days, and chrono interop.
3. Support arithmetic: addition/subtraction of days, weeks, months, years with overflow safety.
4. Offer utilities for quarters, fiscal years, weekends/holidays (via strategy trait).
5. Facilitate comparison, hashing, (de)serialization (serde), and formatting (Display / Debug).
6. Remain `no_std`-compatible **by default**; enable chrono features under the `std`/`chrono` feature flag.

### 2.2 Non-Goals
* Time-of-day and timezone handling – covered by a future *time* module.
* Astronomical calendars (Julian, Buddhist, etc.).
* Localization / i18n.

## 3 High-Level API Sketch
```rust
use rustfin::dates::{Date, DateParseError};

// Construction
let d = Date::new(2025, 6, 29)?;           // ISO constructor
let e = "2025-06-29".parse::<Date>()?;      // FromStr
let f = Date::from_epoch_days(19_500);      // proleptic days since 1970-01-01

// Arithmetic (checked)
let next = d + 1.day();                     // AddDays trait ext
let prev_q = d - 1.quarter();               // AddQuarter ext

// Queries
assert!(d.is_weekend());
assert_eq!(d.quarter(), 2);

// Formatting
println!("Report: {d:%Y-%m-%d}");
```

## 4 Module Organization
```
src/dates/
  ├─ mod.rs          // public facade
  ├─ date.rs         // core Date struct & impls
  ├─ arithmetic.rs   // Add/Sub traits impls
  ├─ fmt.rs          // Display / Debug / formatting helpers
  ├─ parse.rs        // FromStr, serde features
  ├─ holiday.rs      // Strategy trait & built-ins (optional)
  └─ tests.rs        // criterion benchmarks & unit tests
```

## 5 Core Types
### 5.1 `Date`
```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Date {
    days_since_epoch: i32, // proleptic days since 1970-01-01 (fits ±5_873_241 years)
}
```
* `const fn` constructors enable compile-time dates.
* Internal representation is opaque; all user-facing APIs validate ranges.

### 5.2 Error Types
```rust
#[non_exhaustive]
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DateError {
    #[error("year {0} out of range (0000-9999)")]
    InvalidYear(i32),
    #[error("month {0} out of range (1-12)")]
    InvalidMonth(u8),
    #[error("day {day} invalid for {year}-{month}")]
    InvalidDay { year: i32, month: u8, day: u8 },
}
```
Using a `non_exhaustive` enum ensures forward-compatibility.

## 6 Traits & Generic Extensions
1. `AddDays`, `AddWeeks`, `AddMonths`, `AddYears` – mirror `std::time::Duration` but type-safe.
2. `HolidayCalendar` – strategy trait; users implement `is_holiday(&self, date: Date) -> bool`.
3. `QuarterExt` – extension trait providing `quarter()` and `quarter_start()` helpers.

## 7 Parsing & Formatting
* Implement `FromStr` + `TryFrom<&str>` for infallible compile-time parsing via `const fn` where possible.
* Support `serde::{Serialize, Deserialize}` behind the `serde` feature.
* Implement `Display` with ISO-8601 as the canonical format.

## 8 Feature Flags
* `default = []` (no_std)
* `std` – enables `std` and `chrono` interop.
* `serde` – enables serde derive.
* `holidays` – ships a default `Europe::Target` calendar.

## 9 Performance Considerations
* `i32` storage is 4 bytes vs 12 bytes for `(y,m,d)` triple.
* All arithmetic operations compile to a handful of integer ops.
* Critical hot-paths annotated with `#[inline(always)]`.

## 10 Testing Strategy
* Property-based tests via `proptest` for round-trip conversions.
* Exhaustive leap-year tests for years 1900-2400.
* Fuzz parsers with AFL-rs (CI nightly job).
* Criterion benchmarks vs chrono's `NaiveDate`.

## 11 Open Questions
1. Should we expose the internal epoch-days representation publicly? (Leaning **no**.)
2. Do we support Gregorian reform discontinuity (1582-10-05 → 1582-10-14)?
3. Which holiday calendars to bundle under `holidays` feature?

## 12 Timeline
* **v0.1.0** – MVP: `Date`, parsing, arithmetic, serde.
* **v0.2.0** – Schedule builder, day-count, holiday strategy, quarters, performance tuning.
* **v1.0.0** – Stable API freeze after ≥ 6 months of production use.

---
*Last updated: 2025-06-29*

### 6a Schedule & Day-Count Utilities  {#schedule-daycount}

#### Schedule Builder (C-05c)
```rust
let sched = Schedule::builder()
    .start(Date::new(2025, 1, 15)?)
    .end(Date::new(2030, 1, 15)?)
    .frequency(Frequency::SemiAnnual)
    .stub(StubRule::ShortFront)          // optional
    .business_calendar(cal)              // TARGET2 calendar
    .business_convention(BusDayConv::ModFollowing)
    .build()?;                           // returns Schedule { dates: SmallVec<[Date; 32]> }
```
* **Algorithm**: start at `start`, iterate by `frequency.add_to(date)`; detect front/back stub when last period exceeds `end`.  
* Business-day adjustment performed via `calendar.adjust(date, conv)`.
* `Schedule::iter()` yields `&[Date]` slice; zero allocations for ≤32 dates via `smallvec`.

#### Day-Count & Year-Fraction (C-05a)
**Note**  `DayCount`, `Frequency`, and `BusDayConv` enums are **defined once** in the `primitives` crate and re-exported here via `use primitives::{DayCount, Frequency, BusDayConv};`. The dates module never redeclares these enums.

`dates::daycount::year_fraction(dc, d1, d2) -> f64` implements:
* ACT/360, ACT/365F – exact day count over denominator.  
* 30E/360 – European 30/360 algorithm.  
* Function is `#[inline]` and panic-free; returns `Error::Input` on `d2 < d1` unless `allow_negative` flag set.

#### Frequency Helpers (C-05b)
```rust
impl Frequency {
    pub fn add_to(self, d: Date, n: i32) -> Date { /* … */ }
    pub fn periods_per_year(self) -> u8 { /* … */ }
}
```
* Backed by compile-time match; uses `Date::add_months` internally.

#### Weekday & ISO Week
```rust
pub enum Weekday { Mon, Tue, Wed, Thu, Fri, Sat, Sun }
impl Date { pub fn weekday(self) -> Weekday { /* O(1) */ } }
```
* Weekday calculation uses `((days_since_epoch + 4) % 7)` trick.
* `iso_week()` returns `(year, week_no)` used by calendar weekend rules.

#### Gregorian Conversion & Validation
* Utilises Howard Hinnant's civil algorithm in a `const fn` to translate `(y,m,d)` ↔ epoch-days without heap or floating-point.
* Validation catches month/day out-of-range and Gregorian gap (configurable feature flag `gregorian_gap`).

#### Business-Day Adjustment Convenience
```rust
pub fn adjust(date: Date, conv: BusDayConv, cal: &impl HolidayCalendar) -> Date {
    cal.adjust(date, conv)
}
```
* Lightweight re-export calling into calendar module for ergonomics.

#### Serialisation & Versioning (mirrors curves)
* `#[serde(tag = "type", version = 1)]` on `Date` to allow future internal-representation changes.

#### Error Handling
* `DateError` maps to workspace `Error::Input("date")` via `From` impl so callers can bubble up unified error.

#### Performance – Schedule Generation
* Generating 10 k schedules (20 periods each) in < 5 ms single-thread (benchmarked in CI).  
* Iterator avoids heap by re-using local `SmallVec` and returns slices.

---

## 11 Open Questions  (renumber unchanged)
1. Should we expose the internal epoch-days representation publicly? (Leaning **no**.)
2. Do we support Gregorian reform discontinuity (1582-10-05 → 1582-10-14)?
3. Which holiday calendars to bundle under `holidays` feature?

## 12 Timeline
* **v0.1.0** – MVP: `Date`, parsing, arithmetic, serde.
* **v0.2.0** – Schedule builder, day-count, holiday strategy, quarters, performance tuning.
* **v1.0.0** – Stable API freeze after ≥ 6 months of production use.

---
*Last updated: 2025-06-29* 