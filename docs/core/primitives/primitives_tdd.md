# Technical Design Document — **Primitives Core Types**

| **Doc ID** | **RF-PRIMITIVES-TDD-1.0-DRAFT** |
|------------|-----------------------------|
| **Status** | Draft for Review            |
| **Date**   | 29 June 2025                |
| **Authors**| Core Architecture Group     |

> This document consolidates **cross-cutting primitive types** that appear in multiple
> module-level designs (dates, calendar, cashflow, curves). Defining them in a
> dedicated `primitives` crate avoids duplication and guarantees single-source
> semantics across the code-base.

---

## 1 Motivation & Scope

The following types recur in at least two module designs:

| Group | Types / Traits | Referenced In |
|-------|----------------|---------------|
| **Foundation** | `Currency`, `Money`, `Error` | dates, cashflow, curves |
| **Date/Calendar** | `Date`, `DayCount`, `Frequency`, `BusDayConv`, `ScheduleKey` | dates, calendar, cashflow |
| **Calendar** | `CalCode`, `WeekendRule`, `HolidaySet`, `CompositeCalendar` | calendar, cashflow |
| **Finance** | `Notional`, `CFKind`, `AmortRule` | cashflow, instruments |
| **Curves** | `DiscountCurve` trait | cashflow, instruments, risk |

This TDD specifies a **`primitives` crate** (internal workspace member) housing
foundational enums, structs, and error handling primitives used across RustFin
layers **L0–L4**.

---

## 2 Goals & Non-Goals

### 2.1 Goals
1. Provide **single definitions** of ubiquitous primitives → prevent divergent
   implementations.
2. Keep the crate **no_std by default**; opt-in `std` feature for alloc-heavy
   components (e.g., `String` in error messages).
3. Guarantee **stable serialisation** (Serde rev-compatible) across language
   bindings (Rust/Python/WASM).
4. Remain **zero-unsafe** and compile on stable Rust (≥1.78).

### 2.2 Non-Goals
* High-level domain logic (those live in feature crates).
* Third-party currency conversion or locale formatting.

---

## 3 Module Layout
```
primitives/
  ├─ src/
  │   ├─ lib.rs
  │   ├─ currency.rs
  │   ├─ money.rs
  │   ├─ error.rs
  │   ├─ date_key.rs          // ScheduleKey / PeriodKey helpers
  │   └─ macros.rs            // internal derive helpers
  └─ Cargo.toml
```

---

## 4 Core Types

### 4.1 Currency
```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
#[repr(u16)]
pub enum Currency {
    USD = 840,
    EUR = 978,
    GBP = 826,
    JPY = 392,
    // … ISO-4217 numeric codes (2¹⁶-1 max)
}
```
* Numeric representation matches ISO-4217; enables compact storage and easy FFI.
* String parsing provided via `FromStr` impl (e.g., "USD").

> **Note on numeric precision** – The canonical floating-point alias `pub type F`
> is **defined once** in `primitives` (see `numeric.rs`). Down-stream crates
> must `use primitives::F` and must not redeclare the alias locally.

### 4.2 Money
```rust
pub struct Money<F = f64> {
    pub ccy: Currency,
    pub amount: F,
}
```
* Generic over numeric type (`f64` default, `Decimal` under `decimal128`).
* Implements arithmetic ops with currency guard.

### 4.3 Error
```rust
#[non_exhaustive]
#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    #[error("input validation: {0}")] Input(String),
    #[error("calendar data missing: {0}")] Calendar(String),
    #[error("internal bug: {0}")] Internal(String),
}
```
* Centralised error type shared by sub-crates via `primitives::Error`.
* Uses `alloc` `String` only if `std` or `alloc` feature enabled.

### 4.4 Day-Count & Frequency (re-exports)
These lightweight enums used by `dates` & `cashflow` are defined here to avoid cyclic deps.
```rust
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DayCount { Act360, Act365F, ThirtyE360 }

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Frequency { Daily = 1, Weekly = 7, Monthly = 12, Quarterly = 4, SemiAnnual = 2, Annual = 1 }
```

### 4.5 BusDayConv
```rust
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BusDayConv { Following, ModFollowing, Preceding, ModPreceding, None }
```

### 4.6 ScheduleKey / PeriodKey
Efficient key for LRU caches in `cashflow::accrual` (C-39).
```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct PeriodKey {
    pub start: i32, // epoch-days
    pub end:   i32,
    pub day_count: u8,
}
```
* Packs into 12 bytes; hash uses `fxhash` for speed.

### 4.7 Notional & AmortRule (basic definitions)
To avoid circularity with instruments, only basic representation lives here; builders reside in `cashflow`.
```rust
pub enum AmortRule {
    None,
    Linear { final_notional: Money },
    Step  { /* opaque pointer into cashflow */ },
}

pub struct Notional {
    pub initial: Money,
    pub amort: AmortRule,
}
```

### 4.8 DiscountCurve Trait (minimal)
```rust
pub trait DiscountCurve {
    fn df(&self, d: Date) -> f64;
    fn id(&self) -> CurveId;
}
```
* `Date` & `CurveId` re-exported from respective crates to avoid double defs.

### 4.9 Additional Design Details
| Topic | Design Notes |
|-------|--------------|
| **Currency metadata** | `impl Currency { pub const fn minor_units(self) -> u8 { … } }` returned values used by rounding helpers; lookup table generated at build-time from ISO-4217 CSV. |
| **Money arithmetic** | `Add/Sub` only allowed for identical currencies (compile-time check). `try_add(self, other, fx: &impl FxProvider)` lives in a future money/Fx module. Overflow checked in debug builds. |
| **Decimal ↔ f64 bridge** | `impl From<f64> for Money<f64>` is infallible; `TryFrom<Decimal>` for Money<f64>` with rounding‐mode param. Policy documented: bankers-round to 1e-12 default. |
| **PeriodKey hashing** | Uses `fxhash::FxHasher64`; not cryptographically secure but 5× faster than std hash. Collision risk negligible for random epoch-day pairs. |
| **Error categories** | Matches core TDD: `Input`, `Calendar`, `Internal`. `Error::Calendar` populated by calendar crate via `From<CalError>`. |
| **Derive macros** | `macros.rs` will house `impl_display_fromstr!` and `currency_enum!` proc-macros to generate Display/FromStr and currency tables. |
| **Thread-safety** | All primitives are `Copy + Send + Sync`. `Money<Decimal>` is `Clone + Send + Sync`; arithmetic functions are panic-free. |
| **Serde versioning** | All public types derive `Serialize/Deserialize` with `#[serde(version = 1)]`; bumping minor adds fields under `#[serde(default)]`. |
| **No-std / alloc** | Crate compiles with `#![no_std]`; enabling `std` adds `std::error::Error` impl for `Error`. `alloc` automatically pulled by `serde` or `decimal128` when needed. |
| **No CurveId re-export** | To avoid cycles, `CurveId` & `FactorKey` stay in curves crate; primitives only defines financial-agnostic types. |

---

## 5 Feature Flags
| Flag         | Purpose                                |
|--------------|----------------------------------------|
| `std`        | Enable `std` and heap allocations       |
| `decimal128` | Use `rust_decimal::Decimal` in `Money`  |
| `serde`      | Derive serialisation on all types       |

---

## 6 API Stability & Versioning
* Changes to numeric discriminants (**BREAKING**) require semver major bump.
* Adding new currencies or error variants is **additive** (non-breaking).
* Derive macros adhere to `rust-version = "1.78"` MSRV.

---

## 7 Testing Strategy
* Compile-time assertions on `repr` sizes via `static_assertions`.
* Round-trip Serde snapshots (`cargo insta`) for each type.
* Fuzz `Currency::from_str` with random 3-letter strings.

---

## 8 Timeline
* **v0.1.0** — Establish crate, add Currency, Money, Error.
* **v0.2.0** — Move DayCount, Frequency, BusDayConv from dates.
* **v0.3.0** — Introduce PeriodKey and Notional stubs.
* **v1.0.0** — Stabilise API alongside core v1.0 GA.

---

### One-line Summary
A dedicated `primitives` crate unifies foundational primitives like `Currency`, `Money`, `Error`, and day-count enums, ensuring coherent semantics and single-source maintenance across all RustFin modules. 