# Calendar Module – Detailed Design Document

## 1 Overview
The **calendar** module provides holiday–weekend awareness and business-day adjustment services to the RustFin ecosystem. It sits directly on top of the `dates` module (see `dates_detailed_design.md`) and fulfils the TDD/PRD capabilities C-06, C-06a, C-06b, C-35, C-36, and C-37.

Key design pillars:

* 🦀 **Idiomatic Rust** – zero-`unsafe`, trait-oriented, and `no_std` compatible by default.
* ⚡ **Zero-cost Abstractions** – on-stack look-ups, SIMD-friendly date search, and minimal allocations.
* 📦 **Data-Driven** – holiday sets compiled into a binary asset during `build.rs`, enabling deterministic builds.
* 🔌 **Composable** – works seamlessly with `Date`, `DayCount`, and `Schedule` types; downstream crates can inject custom calendars easily.

## 2 Goals & Non-Goals
### 2.1 Goals
1. Represent immutable holiday sets for a given financial centre or exchange.
2. Provide weekend rules (Sat/Sun by default) configurable per calendar.
3. Offer fast **business-day queries & adjustments** (`is_business_day`, `next/prev_business_day`, `add_business_days`).
4. Support **composite calendars** (union/intersection) for multi-center schedules.
5. Supply a **CLI + build-script toolchain** to parse iCalendar (`*.ics`) sources into a binary blob embedded at compile time.
6. Include out-of-box calendars: `USD::NYSE`, `EUR::TARGET2`, `GBP::LSE`, `CME`, `ICE`, `LME`.
7. Integrate with the trait `HolidayCalendar` already sketched in the dates design.

### 2.2 Non-Goals
* Time-zone support (handled in a future *time* module).
* Dynamic calendar downloads at runtime – build-time only for v1.0.

## 3 High-Level API Sketch
```rust
use rustfin::dates::Date;
use rustfin::calendar::{HolidayCalendar, CalCode, CompositeCalendar, BusDayConv};

let cal = CalCode::TARGET2.load()?;          // Built-in calendar
assert!(!cal.is_business_day(Date::new(2025, 1, 1)?));

let spot = cal.adjust(Date::new(2025, 7, 5)?, BusDayConv::Following);
assert_eq!(spot, Date::new(2025, 7, 7)?);

// Composite (EUR settlement across TARGET2 & LSE)
let eur_gbp = CompositeCalendar::merge(&[CalCode::TARGET2.load()?, CalCode::LSE.load()?]);

let next_bd = eur_gbp.add_business_days(Date::new(2025, 4, 2)?, 3);
```

## 4 Module Layout
```
src/calendar/
  ├─ mod.rs           // facade re-exporting public types & functions
  ├─ codes.rs         // CalCode enum (static mapping of string ↔ id)
  ├─ holiday_set.rs   // HolidaySet struct & search algo
  ├─ composite.rs     // CompositeCalendar impl
  ├─ weekend.rs       // WeekendRule enum / trait
  ├─ adjust.rs        // Business-day conventions & helpers (C-35)
  ├─ imm.rs           // IMM / quarterly date helpers (C-36)
  ├─ parser.rs        // iCalendar parser (feature `ical`)
  ├─ build.rs         // binary-blob generator (executed at crate build)
  ├─ cli/             // `rustfin-cal-sync` tool (opt-in)
  └─ tests.rs         // unit & integration tests
```

## 5 Core Types & Data Structures
### 5.1 `CalCode`
```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(u16)]
pub enum CalCode {
    TARGET2,
    NYSE,
    LSE,
    CME,
    ICE,
    LME,
    // … extendable up to 65 535 codes
}
```
* Stable numeric discriminant permits compact serialization and FFI (u16).
* `impl CalCode { pub fn load(self) -> Result<&'static HolidaySet, CalError>; }`

### 5.2 `HolidaySet`
```rust
pub struct HolidaySet {
    year_span: RangeInclusive<i32>,       // min_year..=max_year inclusive
    holidays: &'static [i32],             // sorted days_since_epoch (packed)
    weekend_rule: WeekendRule,
}
```
* `holidays` stored as monotonic slice of `i32`, enabling binary search or SIMD galloping.
* Fit for embedded systems due to `&'static` reference into `.rodata`.

### 5.3 `CompositeCalendar<'a>`
```rust
pub struct CompositeCalendar<'a> {
    components: SmallVec<[&'a HolidaySet; 4]>,
    merge_mode: MergeMode,                // Union (default) or Intersection
}
```
* Merges holidays/ weekends on-the-fly; no allocations if ≤ 4 components.

### 5.4 `WeekendRule`
Enum of common patterns (`SatSun`, `FriSat`, `SunThu`) with `const fn is_weekend(Date) -> bool`.

### 5.5 `BusDayConv`
Business-day convention enum fulfilling C-35:
```rust
pub enum BusDayConv { Following, ModFollowing, Preceding, ModPreceding, None }
```

### 5.6 Error Types
Re-uses crate-wide `Error` enum; variants `InvalidCalCode`, `DateOutOfRange`, `ParseError`.

### 5.7 Additional Implementation Details

| Topic | Notes |
|-------|-------|
| **WeekendRule Patterns** | Built-ins: `SatSun`, `FriSat`, `SunThu`. Users can construct `WeekendRule::Custom(BitMask)` where bit 0 = Mon … bit 6 = Sun. The `is_weekend` check is single `&` on the mask (compile-time `const fn`). |
| **MergeMode Variants** | `Union`, `Intersection`, `NAND`, `Xor` (exclusive). `NAND`/`Xor` gated behind `advanced_merge` feature; algorithms short-circuit via component `is_business_day` checks. |
| **CalCode Registry** | Naming convention `<CCY>::<VENUE>` (e.g., `EUR::TARGET2`, `USD::CME`). New codes added via PR with JSON manifest; CI lints for uniqueness. |
| **HolidaySet Blob Format** | Header: `b"RFCLDR\0"` magic, `u8 version`, `u16 code`, `u16 year_span`, `u32 count`; body: *delta-encoded* `i32` days. Version bump retains reader for N-1 release. |
| **Thread-Safety & Sharing** | `HolidaySet` is `&'static HolidaySet` (`Sync`), `CompositeCalendar` stores `&'static` refs; cloning is pointer copy. |
| **Error Mapping** | Local `CalError` (`InvalidCalCode`, `MissingBlob`, `DateOutOfRange`) implements `From<CalError> for primitives::Error` mapping into `Error::Calendar`. |
| **Historical Rule Changes** | Strategy: multiple `HolidaySet` per `CalCode` keyed by `year_span`; loader picks the first covering requested date. |
| **Memory Footprint** | Target: six built-in calendars occupy ≤ 150 kB `.rodata`; delta encoding + gzip opt-in for WASM builds. |
| **Build Hash** | `build.rs` computes SHA-256 of concatenated source JSON; stored in blob header; CI fails if mismatch. |
| **Daylight-Saving Note** | Calendars are **date-based**; DST shifts don't impact holiday logic. |

## 6 Algorithms
1. **`is_business_day`**: `!weekend_rule.is_weekend(d) && !holidays.binary_search(&d.days_since_epoch()).is_ok()`.
2. **`adjust`**: iterative loop using `is_business_day`; worst-case 7 checks.
3. **`add_business_days`**: branching skip using pre-computed weekday delta; uses unchecked `Date::add_days_unchecked` internally for speed but wrapped in debug builds.
4. **IMM helpers (C-36)**: formulaic calculation (`third Wednesday` rule) avoids table look-up.

## 7 Feature Flags
* `default = []` (no_std, no calendars compiled)
* `std` – enables `Vec`, `Box`, and CLI tool.
* `ical` – pulls [`ical`](https://crates.io/crates/ical) crate for parsing *.ics* files.
* `holidays` – embeds official RustFin holiday blob (enabled by workspace default).

## 8 Build-Time Data Pipeline
```mermaid
flowchart TD
    source[".ics files (public sources)"] --> parser
    parser --> json[Clean JSON]
    json --> compile[build.rs: rustfin-cal-compiler]
    compile --> blob[calendars.bin (bincode)]
    blob --> crate[include_bytes!("$OUT_DIR/calendars.bin")] 
```
* The CLI (`rustfin-cal-sync pull <code>`) fetches latest *.ics* files.
* `build.rs` converts JSON dates → epoch-day vector, groups by CalCode.
* Final binary blob hashed; mismatch triggers full re-build.

## 9 Interoperability with `dates` Module
* Relies on `Date` type's **epoch-day** internal representation.
* Implements the shared `HolidayCalendar` trait defined in `dates` (section 6 of its design):
  ```rust
  impl HolidayCalendar for HolidaySet { /* … */ }
  impl<'a> HolidayCalendar for CompositeCalendar<'a> { /* … */ }
  ```
* `WeekendRule` delegates `is_weekend` to `Date::weekday()` method (planned)
* `dates::Schedule::generate` will take `&dyn HolidayCalendar` parameter to enforce business-day generation.

## 10 Performance Benchmarks
| Scenario                               | Target  | Technique |
|----------------------------------------|---------|-----------|
| `is_business_day` 10 M calls           | < 25 ms | branch-free weekend check + SIMD gallop |
| Adjust 1 M dates (Following)           | < 50 ms | early exit when already BD |
| Build composite (3 sets) & 100 k calls | < 80 ms | on-stack merge, memoised component slice |

Benchmarks run under `cargo criterion` and compared against `chrono-holidays` reference.

## 11 Testing Strategy
* **Golden-vector** fixtures cross-checked against Excel/QuantLib.
* **Property tests** (proptest) on adjustment invariants.
* **Round-trip**: parse iCalendar → HolidaySet → export → diff original.
* CI ensures calendar blob reproducibility via SHA-256.

## 12 Open Questions
1. Do we store historical rule changes (e.g., pre-1999 TARGET vs TARGET2)?
2. Should `CompositeCalendar` support runtime NAND/XOR merge modes?
3. Strategy for daylight-saving related holiday shifts (out-of-scope v1.0?).

## 13 Timeline
* **v0.1.0** – Core `HolidaySet`, `CalCode`, weekend rules, business-day adjust.
* **v0.2.0** – Composite calendars, iCalendar parser, CLI sync.
* **v0.3.0** – Additional exchange calendars and IMM helpers.
* **v1.0.0** – API freeze post production soak.

---
*Last updated: 2025-06-29* 

### 7a CLI Tool – `rustfin-cal-sync`
* `pull <CODE>` – download / update iCalendar and regenerate JSON.
* `diff <CODE> --since YYYY` – show added/removed holidays.
* `validate --all` – lint JSON against weekday consistency.
* `json` flag outputs machine-readable diff for CI.
* Non-interactive by default; `--yes` auto-confirms overwrites.

_(Subsequent sections retain numbering.)_ 