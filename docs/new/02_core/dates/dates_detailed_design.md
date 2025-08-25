# Dates & Calendar Module – Detailed Design (v3 – built on the `time` crate)

> **Purpose**   Unifies the previous *dates* and *calendar* designs into **one cohesive module** that extends the [`time`](https://docs.rs/time) crate with finance-specific functionality: day-count conventions, business-day logic, holiday calendars, IMM helpers, and schedule generation.

---

## 1 Overview
We **reuse** the `time` crate for all general calendrical concerns and layer the following finance-domain capabilities on top:

1. **Day-count conventions** – ACT/360, ACT/365F, 30E/360, …
2. **Holiday calendars & business-day adjustment** – TARGET2, NYSE, LSE, custom composite calendars, weekend rules, business-day conventions.
3. **Schedule builder DSL** – coupon schedules with frequency, stub rules, EOM handling.
4. **IMM / Quarterly helpers** – third-Wednesday calculation, next IMM date.
5. **Convenience extension traits** – `is_weekend`, `quarter`, `add_business_days`, …

Everything is `no_std` by default, additive via feature flags, and strives for near-zero overhead over raw `time` operations.

---

## 2 Goals & Non-Goals
### 2.1 Goals
1. Re-use `time` for correctness/performance while hiding version churn behind our facade.
2. Provide full finance-grade date handling in **one crate** (no split between *dates* and *calendar*).
3. Stay `no_std`; gate optional pieces (`serde`, `std`, `holidays`, `ical`, `cli`).
4. Deterministic builds – holiday data embedded at compile-time.

### 2.2 Non-Goals
* Re-implementing basic calendar algorithms already in `time`.
* Time-zone database shipping (delegate to `time_tz` / `chrono-tz`).
* Runtime downloading of calendar data (build-time only for v1.0).

---

## 3 Public API Sketch
```rust
use rustfin::dates::{DateExt, DayCount, Schedule, HolidayCalendar, BusDayConv};
use rustfin::dates::cal::{CalCode, CompositeCalendar};
use time::macros::date;

// Quick helpers built on `time::Date`
let trade = date!(2025-06-27);
assert!(trade.is_weekend() == false);

// Day-count
let settle = date!(2025-07-01);
let yf = DayCount::Act365F.year_fraction(trade, settle)?;

// Holiday calendars
let target2 = CalCode::TARGET2.load()?;
assert!(!target2.is_business_day(trade));
let spot = target2.adjust(trade, BusDayConv::Following);

// Composite calendar (EUR settlement across TARGET2 & LSE)
let eur_gbp = CompositeCalendar::merge(&[CalCode::TARGET2.load()?, CalCode::LSE.load()?]);
let next_bd = eur_gbp.add_business_days(trade, 3);

// Schedule builder
let sched = Schedule::builder()
    .start(settle)
    .end(date!(2030-07-01))
    .frequency(Frequency::SemiAnnual)
    .business_calendar(&eur_gbp)
    .build()?;
```

---

## 4 Module Layout
```
src/dates/
  ├─ mod.rs              // public facade – re-exports from `time` + our API
  ├─ ext.rs              // DateExt / OffsetDateTimeExt helpers
  ├─ daycount.rs         // DayCount enum + algorithms
  ├─ schedule.rs         // ScheduleBuilder DSL
  ├─ frequency.rs        // Frequency, StubRule enums
  ├─ cal/                // ← merged former calendar module
  │   ├─ mod.rs          // re-exports CalCode, HolidaySet, etc.
  │   ├─ codes.rs        // CalCode enum
  │   ├─ holiday_set.rs  // HolidaySet struct & search algos
  │   ├─ weekend.rs      // WeekendRule enum / trait
  │   ├─ adjust.rs       // BusDayConv + adjust/add_business_days
  │   ├─ composite.rs    // CompositeCalendar impl
  │   ├─ imm.rs          // IMM helpers
  │   ├─ blob.rs         // runtime loader for compiled holiday blob
  │   └─ parser.rs       // iCalendar parser (feature `ical`)
  └─ tests/
```

---

## 5 Core Components
### 5.1 `DateExt` & `OffsetDateTimeExt`
Provide convenience methods such as `is_weekend`, `quarter`, `fiscal_year`, and `add_business_days` (delegates to calendar logic when calendar provided).

### 5.2 `DayCount` Enum
Implements industry-standard year-fraction algorithms with zero allocation.

### 5.3 Calendar Sub-module (`cal`)
* `CalCode` – stable `u16` identifiers for built-in calendars.
* `HolidaySet` – immutable holiday vector plus `WeekendRule`.
* `HolidayCalendar` trait – blanket-implemented for `HolidaySet` & `CompositeCalendar`.
* `CompositeCalendar` – on-stack merge of multiple calendars (`Union`/`Intersection`).
* `BusDayConv` – business-day conventions (`Following`, `ModFollowing`, …).
* `IMM` helpers – third-Wednesday calculations.

### 5.4 `ScheduleBuilder` & `Schedule`
Fluent DSL returning `SmallVec<[Date; 32]>` schedules with optional calendar adjustment.

---

## 6 Feature Flags
| Flag      | Purpose                                             | Default |
|-----------|-----------------------------------------------------|---------|
| `std`     | Enable `std::error::Error`, `Vec`, CLI, etc.        | ❌      |
| `serde`   | Derive (de)serialisation for enums & structs        | ❌      |
| `holidays`| Embed built-in calendars (TARGET2, NYSE, …)         | ✅      |
| `ical`    | Enable iCalendar parser & build-time JSON pipeline  | ❌      |
| `cli`     | Build `rustfin-cal-sync` tool (implies `std`)       | ❌      |

---

## 7 Build-Time Data Pipeline (when `ical` + `holidays`)
```mermaid
flowchart TD
    source[".ics files"] --> parser
    parser --> json[Clean JSON]
    json --> compile[build.rs: rustfin-cal-compiler]
    compile --> blob[calendars.bin]
    blob --> crate[include_bytes!("$OUT_DIR/calendars.bin")]
```

---

## 8 Algorithms & Performance Targets
| Function                              | Target   | Notes |
|---------------------------------------|----------|-------|
| `HolidaySet::is_business_day` 10 M    | < 25 ms  | branch-free weekend + SIMD gallop |
| `adjust` 1 M dates (Following)        | < 50 ms  | early exit on already BD |
| Schedule generation 10 k×20 periods   | < 5 ms   | uses `SmallVec` |
| Day-count `year_fraction`             | < 80 ns  | inline integer maths |

---

## 9 Testing Strategy
1. **Unit tests** for every algorithm branch.
2. **Property tests** (proptest) for calendar adjustment invariants.
3. **Golden-vector** comparisons vs Excel/QuantLib.
4. Reuse `time`'s test-suite for core date arithmetic.

---

## 10 Open Questions
1. Which additional calendars (CME, ICE, LME) ship in v0.1.0?
2. Should we expose helper macros (`5.days()`, `3.bd(&cal)`) for ergonomics?
3. How to handle `time` major-version upgrades without breaking semver?

---

## 11 Timeline & Versions
See merged implementation roadmap in `dates_implementation_prs.md`.  MVP **v0.1.0** targets: Extension traits, DayCount, TARGET2 calendar, business-day adjustment, Schedule builder.

---
*Last updated: 2025-07-01* 