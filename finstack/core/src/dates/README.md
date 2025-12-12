## Dates Module (core)

The `dates` module in `finstack-core` provides **time and calendar primitives** for all other crates. It wraps the `time` crate and adds finance‑specific behavior:

- **Re‑exports** of `time` types (`Date`, `OffsetDateTime`, `PrimitiveDateTime`)
- **Holiday calendars** and business‑day conventions
- **Day‑count conventions** for accrual and discounting
- **Schedule generation** for coupons, cashflows, and CDS IMM ladders
- **Tenors and period systems** (quarterly/monthly/weekly/fiscal)
- **IMM and option expiry helpers**
- **Rate conversion utilities** between simple / periodic / continuous compounding

Everything is accessible via `finstack_core::dates`, and is designed to be:

- **Deterministic**: No randomness; date math is pure and repeatable
- **Panic‑free in public APIs**: functions return `crate::Result<T>` for invalid input
- **Wire‑stable**: public enums and DTOs are serde‑ready under the `serde` feature

---

## Module Structure

- **`mod.rs`**
  - Public entrypoint for the dates module.
  - Re‑exports:
    - `time::{Date, OffsetDateTime, PrimitiveDateTime}`
    - Extension traits: `DateExt`, `OffsetDateTimeExt`
    - Day‑count types: `DayCount`, `DayCountCtx`, `DayCountCtxState`, `Thirty360Convention`
    - Rate utilities: `rate_conversions::*`
    - Calendars and business days:
      - `HolidayCalendar`, `BusinessDayConvention`, `adjust`, `available_calendars`
      - `CompositeCalendar`, `CalendarRegistry`
    - Schedule types: `Frequency`, `Schedule`, `ScheduleBuilder`, `ScheduleSpec`, `StubKind`
    - Tenor and IMM helpers: `Tenor`, `TenorUnit`, `next_imm`, `next_cds_date`, `third_wednesday`, etc.
    - Period system: `Period`, `PeriodId`, `PeriodKind`, `PeriodPlan`, `FiscalConfig`,
      `build_periods`, `build_fiscal_periods`
    - Safe constructor `create_date(year, month, day) -> Result<Date>`
- **`date_extensions.rs`**
  - Extension traits for `Date` and `OffsetDateTime`:
    - Calendar helpers: `is_weekend`, `is_business_day`, `end_of_month`, `next_imm`
    - Fiscal helpers: `quarter`, `fiscal_year(config: FiscalConfig)`
    - Arithmetic: `add_months`, `add_weekdays`, `add_business_days`
    - Analytics: `months_until(other)`
    - `BusinessDayIter` iterator over business days in `[start, end)`.
- **`calendar/`**
  - Holiday calendar system:
    - `business_days.rs`: `HolidayCalendar` trait, `BusinessDayConvention`, `adjust`, `seek_business_day`
    - `rule.rs`: rule DSL for holiday definitions (fixed dates, nth weekday, Easter offsets, lunar rules, etc.)
    - `generated.rs` + `algo.rs`: build‑time compiled bitsets for 1970–2150 and shared helpers
    - `composite.rs`: `CompositeCalendar` for unions of calendars
    - `types.rs`: metadata types (`CalendarMetadata`, IDs, etc.)
    - `registry.rs`: `CalendarRegistry` and global registry, calendar lookup by ID
- **`daycount.rs`**
  - Industry‑standard day‑count conventions:
    - `DayCount` enum with variants:
      - `Act360`, `Act365F`, `Act365L`
      - `Thirty360`, `ThirtyE360`
      - `ActAct`, `ActActIsma`
      - `Bus252`
    - `DayCountCtx` / `DayCountCtxState` to supply calendars and coupon frequency (for `Bus252`, `ActActIsma`).
    - Core API:
      - `DayCount::year_fraction(start, end, ctx) -> Result<f64>`
    - Helpers for 30/360, Act/Act (ISDA + ISMA), Act/365L, and business‑day counting.
- **`schedule_iter.rs`**
  - Schedule generation engine:
    - `Frequency` (months or days, with helpers like `monthly`, `quarterly`, `weekly`, etc.)
    - `StubKind` (None, ShortFront/Back, LongFront/Back)
    - `Schedule` (vector of monotonically increasing `Date`s)
    - `ScheduleBuilder`:
      - Frequency configuration
      - Stub rules
      - End‑of‑month convention
      - Business‑day adjustment via calendar or calendar ID
      - CDS IMM mode (`cds_imm`)
      - Graceful fallback mode (empty schedule on error)
    - `ScheduleSpec` (serde DTO) → reconstructs a `ScheduleBuilder` at runtime.
- **`tenor.rs`**
  - Market‑standard tenor handling:
    - `TenorUnit` (Days, Weeks, Months, Years)
    - `Tenor { count, unit }` with:
      - Parsing from strings (`"3M"`, `"6M"`, `"1Y"`)
      - Conversions:
        - `to_years_simple()` – simple approximation
        - `to_years_with_context(as_of, calendar, bdc, day_count)`
        - `add_to_date(date, calendar, bdc)`
      - Constructors: `overnight`, `one_week`, `one_month`, `three_months`, `six_months`, `one_year`
- **`periods.rs`**
  - Financial reporting period system for statements and time‑series:
    - `PeriodKind`: `Quarterly`, `Monthly`, `Weekly`, `SemiAnnual`, `Annual`
    - `PeriodId`: typed IDs like `2025Q1`, `2025M03`, `2025W10`
      - Ordering by actual calendar spans (even across mixed kinds)
      - Display/parse and serde support
    - `FiscalConfig`: description of fiscal year start (month/day) with presets (US federal, UK, Japan, etc.)
    - `Period`: `(id, start, end, is_actual)` with inclusive/exclusive bounds
    - `PeriodPlan { periods: Vec<Period> }` with iteration helpers
    - Builders:
      - `build_periods("2025Q1..Q4", Some("2025Q2"))`
      - `build_periods("2024M11..2025M02", None)`
      - `build_fiscal_periods("2025Q1..Q4", FiscalConfig::us_federal(), Some("2025Q2"))`
- **`imm.rs`**
  - IMM and option expiry helpers:
    - `third_wednesday(month, year)`
    - `next_imm(date)` – next IMM (3rd Wed of Mar/Jun/Sep/Dec) strictly after `date`
    - `next_cds_date(date)` – next CDS roll (20‑Mar/Jun/Sep/Dec)
    - `imm_option_expiry(month, year)` – Friday before third Wednesday
    - `third_friday(month, year)`
    - `next_imm_option_expiry(date)` – next IMM option expiry (quarterly)
    - `next_equity_option_expiry(date)` – next equity option expiry (3rd Friday monthly)
- **`rate_conversions.rs`**
  - Interest‑rate compounding conversions:
    - Simple ↔ periodic: `simple_to_periodic`, `periodic_to_simple`
    - Periodic ↔ continuous: `periodic_to_continuous`, `continuous_to_periodic`
    - Simple ↔ continuous: `simple_to_continuous`, `continuous_to_simple`
  - All functions return `Result<f64>` and validate inputs (non‑negative year fractions, positive frequencies, etc.).

---

## Core Types and Traits

### `Date`, `OffsetDateTime` and `DateExt` / `OffsetDateTimeExt`

The module re‑exports `time::Date` and `time::OffsetDateTime` as the canonical date types, and augments them via extension traits:

- **Calendar awareness**:
  - `is_weekend() -> bool`
  - `is_business_day(&C) -> bool` (using a `HolidayCalendar`)
  - `end_of_month() -> Date`
  - `next_imm() -> Date`
- **Fiscal logic**:
  - `quarter() -> u8`
  - `fiscal_year(FiscalConfig) -> i32`
- **Arithmetic**:
  - `add_months(months: i32) -> Date`
  - `add_weekdays(n: i32) -> Date` (weekends only)
  - `add_business_days(n: i32, &C) -> Result<Date>` (weekends + holidays)
  - `months_until(other) -> u32`
- **Iterator**:
  - `BusinessDayIter<'a, C>` over business days in `[start, end)`.

Use the **safe constructor**:

```rust
use finstack_core::dates::{create_date, Date};
use time::Month;

let d = create_date(2025, Month::January, 15)?;        // Ok
let invalid = create_date(2025, Month::February, 30);  // Err(InputError::InvalidDate { .. })
```

### `HolidayCalendar` and `BusinessDayConvention`

Calendars are read‑only, deterministic objects backed by generated bitsets plus runtime rules. The core trait is:

- `HolidayCalendar` (in `calendar::business_days`):
  - `is_holiday(date: Date) -> bool`
  - `is_business_day(date: Date) -> bool`

Business‑day adjustments use:

- `BusinessDayConvention` enum (`Following`, `ModifiedFollowing`, `Preceding`, etc.)
- `adjust(date, convention, &cal) -> Result<Date>`
- `available_calendars() -> impl Iterator<Item = &str>` for discovery

Lookup is done via `CalendarRegistry`:

```rust
use finstack_core::dates::{Date, BusinessDayConvention, adjust};
use finstack_core::dates::calendar::registry::CalendarRegistry;
use time::Month;

let base = Date::from_calendar_date(2025, Month::December, 25)?;
let nyse = CalendarRegistry::global()
    .resolve_str("nyse")
    .ok_or("missing NYSE calendar")?;

let adj = adjust(base, BusinessDayConvention::Following, nyse)?;
```

For multi‑market instruments, use `CompositeCalendar` to union calendars.

### `DayCount` and `DayCountCtx`

`DayCount` encodes market day‑count conventions. The main API:

```rust
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use time::Month;

let start = Date::from_calendar_date(2025, Month::January, 1)?;
let end = Date::from_calendar_date(2026, Month::January, 1)?;

let yf = DayCount::ActAct.year_fraction(start, end, DayCountCtx::default())?;
```

Context carries:

- `calendar: Option<&dyn HolidayCalendar>` for `Bus252`
- `frequency: Option<Frequency>` for `ActActIsma`
- `bus_basis: Option<u16>` for custom `Bus/N` denominators

`DayCountCtxState` is a serde DTO that can be serialized (e.g. to JSON) and re‑hydrated using a `CalendarRegistry`.

### `Frequency`, `ScheduleBuilder`, and `Schedule`

`Frequency` is a **payment frequency**:

- Month‑based: `Frequency::annual()`, `semi_annual()`, `quarterly()`, `monthly()`, etc.
- Day‑based: `Frequency::weekly()`, `biweekly()`, `daily()`

Schedules are built via `ScheduleBuilder`:

```rust
use finstack_core::dates::{ScheduleBuilder, Frequency, BusinessDayConvention};
use finstack_core::dates::calendar::registry::CalendarRegistry;
use time::{Date, Month};

let start = Date::from_calendar_date(2025, Month::January, 15)?;
let end = Date::from_calendar_date(2025, Month::December, 15)?;
let nyse = CalendarRegistry::global()
    .resolve_str("nyse")
    .ok_or("nyse not found")?;

let schedule = ScheduleBuilder::new(start, end)
    .frequency(Frequency::quarterly())
    .stub_rule(finstack_core::dates::StubKind::ShortBack)
    .end_of_month(false)
    .adjust_with(BusinessDayConvention::ModifiedFollowing, nyse)
    .build()?;

for d in schedule.into_iter() {
    println!("{d}");
}
```

Key invariants:

- Dates are strictly increasing and deduplicated after EOM and adjustment.
- `build()` returns `Result<Schedule>`; invalid ranges (`start > end`) yield `Error::Input(InputError::InvalidDateRange)`.
- With `.graceful_fallback(true)`, `build()` returns an **empty** schedule with a `ScheduleWarning::GracefulFallback` warning instead of an error. Always check `schedule.has_warnings()` to detect suppressed errors and avoid silent PV=0 scenarios.

`ScheduleSpec` is a serde‑capable spec you can persist and later call `.build()` on to rebuild schedules.

### `Tenor` and `TenorUnit`

Tenors encapsulate relative time periods with finance semantics:

```rust
use finstack_core::dates::{Tenor, TenorUnit, Date, DayCount, DayCountCtx, BusinessDayConvention};
use finstack_core::dates::calendar::TARGET2;
use time::Month;

let as_of = Date::from_calendar_date(2025, Month::January, 31)?;
let tenor = Tenor::parse("1M")?;

// Date math with EOM + business‑day adjustment
let end = tenor.add_to_date(as_of, Some(&TARGET2), BusinessDayConvention::ModifiedFollowing)?;

// Calendar‑aware year fraction
let yf = tenor.to_years_with_context(
    as_of,
    Some(&TARGET2),
    BusinessDayConvention::ModifiedFollowing,
    DayCount::ActAct,
)?;
```

Use `Tenor::from_years(years, DayCount)` when converting continuous year fractions back into market‑style tenors.

### Periods and Fiscal Configurations

The period system is designed for statements and forecasting:

- Use `PeriodId` to represent a single period (`2025Q1`, `2025M03`, `2025W10`).
- Use `build_periods` to expand range expressions:

```rust
use finstack_core::dates::{build_periods, Period};

let plan = build_periods("2025Q1..Q3", Some("2025Q2"))?;
for p in plan.periods {
    println!("{}: {}..{} (actual={})", p.id, p.start, p.end, p.is_actual);
}
```

For fiscal years, use `FiscalConfig` and `build_fiscal_periods`:

```rust
use finstack_core::dates::{FiscalConfig, build_fiscal_periods};

let config = FiscalConfig::us_federal(); // Oct 1 start
let fiscal = build_fiscal_periods("2025Q1..Q4", config, Some("2025Q2"))?;
```

Range strings may mix absolute and relative right‑hand sides (`"2024M11..2025M02"`, `"2025Q1..Q4"`). Mixed frequencies in the same year are ordered by actual calendar spans.

### IMM and Expiry Helpers

Use IMM utilities instead of ad‑hoc “third Wednesday” logic:

```rust
use finstack_core::dates::{third_wednesday, next_imm, next_cds_date, next_imm_option_expiry};
use time::{Date, Month};

let imm_march = third_wednesday(Month::March, 2025);
let next_futures_roll = next_imm(Date::from_calendar_date(2025, Month::March, 20)?);
let next_cds_roll = next_cds_date(Date::from_calendar_date(2025, Month::March, 10)?);
let next_imm_option = next_imm_option_expiry(Date::from_calendar_date(2025, Month::March, 15)?);
```

For equity options, use `next_equity_option_expiry(date)` (3rd Friday of each month).

### Rate Conversion Utilities

The `rate_conversions` module normalizes interest rates across quoting conventions:

```rust
use finstack_core::dates::rate_conversions::{
    simple_to_periodic, periodic_to_continuous, continuous_to_simple,
};

// Money‑market simple rate → swap (semi‑annual) → continuous → back to simple
let simple = 0.035;
let yf = 0.25;                // 3M
let periodic = simple_to_periodic(simple, yf, 2)?; // 2 coupons/year
let continuous = periodic_to_continuous(periodic, 2)?;
let simple_back = continuous_to_simple(continuous, yf)?;
```

All functions:

- Validate inputs (e.g., `periods_per_year > 0`, non‑negative year fractions)
- Preserve precision under realistic rates (round‑trip tests)
- Support negative rates, which are common in modern markets

---

## Usage Examples

### Business‑Day Adjustments

```rust
use finstack_core::dates::{Date, DateExt, BusinessDayConvention, adjust};
use finstack_core::dates::calendar::TARGET2;
use time::Month;

let cal = TARGET2;
let trade_date = Date::from_calendar_date(2025, Month::June, 27)?; // Friday

// 3 business days forward via extension
let settlement = trade_date.add_business_days(3, &cal)?;

// Equivalent explicit adjustment from “raw” date
let raw_settlement = trade_date + time::Duration::days(3);
let adjusted = adjust(raw_settlement, BusinessDayConvention::Following, &cal)?;
```

### Reporting Period Plans

```rust
use finstack_core::dates::{build_periods, build_fiscal_periods, FiscalConfig};

// Calendar quarters with actual/forecast split
let plan = build_periods("2025Q1..Q4", Some("2025Q2"))?;
assert_eq!(plan.periods.len(), 4);
assert!(plan.periods[0].is_actual);
assert!(!plan.periods[3].is_actual);

// US federal fiscal quarters (FY2025)
let cfg = FiscalConfig::us_federal();
let fiscal = build_fiscal_periods("2025Q1..Q4", cfg, None)?;
```

### Curve‑Compatible Day Counts

You should use `DayCount` consistently between curves and cashflow accruals:

```rust
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use time::Month;

let start = Date::from_calendar_date(2025, Month::January, 1)?;
let end   = Date::from_calendar_date(2025, Month::July, 1)?;

let dc = DayCount::Act360;
let yf = dc.year_fraction(start, end, DayCountCtx::default())?;
```

For `Bus252`, provide a calendar:

```rust
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::dates::calendar::TARGET2;
use time::Month;

let start = Date::from_calendar_date(2025, Month::January, 2)?;
let end   = Date::from_calendar_date(2025, Month::January, 9)?;

let ctx = DayCountCtx { calendar: Some(&TARGET2), frequency: None, bus_basis: None };
let yf = DayCount::Bus252.year_fraction(start, end, ctx)?;
```

---

## Adding New Features

The `dates` module is **core infrastructure** shared by curves, cashflows, statements, scenarios, and portfolio analytics. When extending it:

- Keep changes **small and deterministic**
- Avoid panics in public APIs; return `crate::Result<T>`
- Preserve **serde stability** for any existing public types
- Prefer **reusing** existing helpers (`DateExt`, `ScheduleBuilder`, `DayCount`, calendars) over adding ad‑hoc date logic

### New Calendar or Calendar Rule

- Add a JSON file under `finstack/core/data/calendars/` following existing examples (e.g. `nyse.json`, `target2.json`).
- Use rule types from `calendar::rule::Rule` (fixed dates, nth weekday, Easter offsets, Chinese New Year, Japanese equinoxes, etc.).
- Run `cargo build` to regenerate compiled calendars under `dates::calendar::generated`.
- Add tests under `finstack/core/tests/dates/` to validate:
  - Known holidays
  - Weekend behavior
  - Calendar lookup via `CalendarRegistry::global()` and `available_calendars()`.

### New Day‑Count Convention

- Extend `dates/daycount.rs`:
  - Add a variant to `DayCount` with clear **doc comments** and financial references.
  - Implement logic in `DayCount::year_fraction` (and `DayCount::days` for tests if applicable).
  - Use `DayCountCtx` (and `DayCountCtxState` under `serde`) for any needed context:
    - Calendars (`Bus/N`‑style)
    - Coupon frequency (coupon‑aware conventions)
  - Add unit tests that cover:
    - Equal dates and inverted ranges (error)
    - Leap years and edge cases
    - Calendar‑aware behavior where relevant
- Avoid changing existing semantics or serialized names; add‑only is the norm.

### New Schedule Features

- Extend `schedule_iter.rs`:
  - For new frequency styles, add variants to `Frequency` and map them through the internal `Step`.
  - For new stub logic, consider whether it can be expressed via existing `StubKind`; if not, add a new variant and implement it in `BuilderInternal`.
  - Keep `Schedule` invariant: strictly increasing, deduplicated dates.
- Add tests that:
  - Compare schedules with and without EOM
  - Validate stub behaviors (short/long, front/back)
  - Check interaction with calendars (`adjust_with` / `adjust_with_id`)

### New Period Formats or Fiscal Behavior

- Extend `PeriodKind` and `PeriodId` only if there is a clear, reusable frequency not already covered.
- Update:
  - Range parsing (`parse_id`, `parse_range`)
  - `PeriodCalendar` implementations (`Gregorian` and `FiscalCalendar`)
  - Ordering (`Ord`/`PartialOrd` for `PeriodId`)
- Add tests for:
  - Parsing and enumeration
  - Mixed‑frequency ordering and contiguity
  - Fiscal ranges with different `FiscalConfig` presets
- Preserve existing string formats and serde behavior for `PeriodId` (they are part of the public wire format).

### New Tenor Behavior

- Prefer using `Tenor` instead of introducing new ad‑hoc `(count, unit)` types.
- If adding features (e.g., new parsing forms, special units):
  - Extend `TenorUnit` and `Tenor::parse` with careful input validation.
  - Provide doc‑tested examples and references to market conventions (e.g., OIS, money‑market futures).
  - Add tests for:
    - Parsing valid/invalid strings
    - Year‑fraction behavior in combination with `DayCount`.

### New Rate Conversion Helpers

- Keep all compounding logic in `rate_conversions.rs`.
- Follow patterns already used:
  - Validate arguments early (`periods_per_year > 0`, finite values, no negative discount factors).
  - Prefer **mathematically stable** forms (`ln`, `exp`) and add tests for high/low rates and high compounding frequencies.
- Add tests that:
  - Round‑trip across conversion pairs (simple ↔ periodic, periodic ↔ continuous, simple ↔ continuous).
  - Verify behavior for zero and negative rates.

---

## When to Use This Module vs. Higher‑Level Crates

- **Use `core::dates` when**:
  - You need calendars, day‑counts, schedules, tenors, or reporting periods.
  - You are implementing new curves, instruments, or statement models and need date primitives.
- **Use higher‑level crates (`valuations`, `statements`, `scenarios`, `portfolio`) when**:
  - You are building full instrument pricing, scenario generation, or portfolio aggregation.

Keeping this separation clean ensures the `core` crate remains **small, deterministic, and reusable** across Rust, Python, and WASM bindings.



