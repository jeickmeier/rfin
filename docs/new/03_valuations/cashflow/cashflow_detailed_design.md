# CashFlow Module – Detailed Design Document

## 1 Overview
The **cashflow** module represents monetary payments (and related metadata) and provides valuation helpers that rely on the previously defined `dates` and `calendar` modules, plus curve traits from the forthcoming `curves` module. It implements the PRD/TDD capabilities C-01, C-02, C-38, C-39, C-40 and anticipates extensions required by C-22.

### Layer Context
```
L0  math/error
L1  dates  ← calendar
L2  cashflow   ◀─── this design
L3  curves
```

## 2 Goals & Non-Goals
### 2.1 Goals
1. Provide a **`CashFlow` struct** (date, amount, kind) with zero-cost arithmetic and serialization.
2. Offer `npv` helpers that discount using an abstract `Curve` trait (**C-02**).
3. Support **notional/amortisation schedules** and step tables (**C-38**).
4. Compute day-count accrual factors deterministically (**C-39**).
5. Model **stub cash-flows** (irregular first/last periods) (**C-40**).
6. Facilitate future extensions for bond/private-credit variants (**C-22**).

### 2.2 Non-Goals
* Instrument pricing logic (lives in instrument layer).
* Currency & FX handling (provided by `money` sub-module later).
* Accrual conventions beyond day-count and business-day adjustments.

## 3 High-Level API Sketch
```rust
use rustfin::cashflow::{CashFlow, CashFlowLeg, CFKind, npv_portfolio};
use rustfin::dates::{Date, Schedule};
use rustfin::calendar::CalCode;
use rustfin::curves::{DiscountCurve, CurveId};

// Fixed-leg generation via helper builder
let schedule = Schedule::builder()
    .start(Date::new(2025, 1, 15)?)
    .end(Date::new(2030, 1, 15)?)
    .frequency(Frequency::SemiAnnual)
    .business_calendar(CalCode::TARGET2.load()?)
    .build()?;                                 // generates business-day-adjusted period dates

let fixed_leg = CashFlowLeg::fixed_rate(
    Notional::par(1_000_000, Currency::EUR),
    0.025,                             // 2.5 % fixed coupon
    schedule,
    DayCount::Act365F,
);

let pv = fixed_leg.npv(&curve);

// Portfolio NPV
let total = npv_portfolio(&[fixed_leg, float_leg], &curve);
```

## 4 Module Layout
```
src/cashflow/
  ├─ mod.rs             // facade
  ├─ cashflow.rs        // CashFlow struct & impls
  ├─ leg.rs             // CashFlowLeg builder + amort schedule (C-38, C-40)
  ├─ stub.rs            // Stub period support
  ├─ npv.rs             // Discount helpers (C-02)
  ├─ notional.rs        // Notional type & amortisation rules
  ├─ stub.rs            // Stub period support
  ├─ tests.rs
```

## 5 Core Types & Traits
### 5.1 `CFKind`
```rust
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CFKind {
    Fixed,          // fixed-rate coupon
    FloatReset,     // floating reset (index-linked accrual)
    Notional,       // principal exchange
    Fee,            // fee / upfront
    Stub,           // irregular stub (C-40)
    // upcoming: PIK, StepUp, Amort, etc. (C-22)
}
```

### 5.2 `CashFlow`
```rust
#[derive(Clone, Debug, PartialEq)]
pub struct CashFlow {
    pub date: Date,
    /// For floating-rate legs this stores the index reset (fixing) date;
    /// `None` for fixed coupons or principal exchanges.
    pub reset_date: Option<Date>,
    pub amount: Money,                       // amount incl. currency
    pub kind: CFKind,
    accrual_factor: f64,                     // period year fraction
}
```
* `Money` is thin wrapper `(Currency, F)`, where `F` is `f64` or `Decimal`.
* `accrual_factor` computed via `DayCount::year_fraction(start, end)` (C-39).

### 5.3 `CashFlowLeg`
Aggregates ordered `Vec<CashFlow>` plus leg metadata:
```rust
pub struct CashFlowLeg {
    pub flows: Vec<CashFlow>,
    notional: Notional,
    day_count: DayCount,
    pay_calendar: &'static HolidaySet,        // settlement calendar
}
```

### 5.4 `Notional` & Amortisation (C-38)
```rust
pub enum AmortRule {
    None,
    Linear { final_notional: Money },
    Step { schedule: Vec<(Date, Money)> },
}

pub struct Notional {
    initial: Money,
    amort: AmortRule,
}
```
* `Notional::par(amount, ccy)` shortcut sets `amort = None`.
* `CashFlowLeg::apply_amortisation` mutates flows post generation.

### 5.5 `DayCount`, `Schedule`, `Frequency`
* Re-exported from `dates` module. `Schedule` builder relies on calendar adjust helpers.
* Additional functionality to be added in `dates` (if not present yet) to cover schedule generation.

### 5.6 `Discountable` Trait (C-02)
```rust
pub trait Discountable {
    type PVOutput;
    fn npv<C: DiscountCurve>(&self, curve: &C) -> Self::PVOutput;
}

impl Discountable for CashFlowLeg { /* … */ }
impl Discountable for [CashFlow] { /* … */ }
```

### 5.7 Floating-Rate Leg Builder
```rust
let float_leg = CashFlowLeg::floating_rate()
    .notional(Notional::par(1_000_000, Currency::USD))
    .index("USD-LIB3M")                 // ties into ForwardIndexCurve
    .spread_bp(12.5)                     // 12.5 bp over index
    .gearing(1.0)
    .reset_lag(2)                        // business days
    .schedule(schedule)                  // from dates::Schedule builder
    .day_count(DayCount::Act360)
    .build()?;
```
* Supports **in-advance** (`reset_lag >= 0`) and **in-arrears** (`reset_lag < 0`) fixing.  
* On `build` it adds a `FloatReset` cash-flow at each reset date and corresponding coupon `CashFlow` at payment date.  
* `CashFlow::reset_date` is populated for coupons to facilitate index look-up.

### 5.8 Fee & Principal Helpers
```rust
CashFlow::principal_exchange(date, amount)
CashFlow::fee(date, amount)
```
* Provided as associated fns returning a pre-initialised `CashFlow` with `CFKind::Notional` or `CFKind::Fee`.

### 5.9 Accrued-Interest Utilities
* `CashFlowLeg::accrued(Date)` returns accrued interest up to (but excluding) the pricing date using the applicable day-count factor.  
* Uses last coupon period where `reset_date < val_date`.

### 5.10 Additional Shared Details
| Topic | Notes |
|-------|-------|
| **Thread-Safety** | `CashFlow` is `Copy + Send + Sync` (32 bytes). `CashFlowLeg` wraps flows in `Arc<[CashFlow]>`; cloning the leg is an atomic pointer copy. |
| **Serialisation** | All structs derive Serde with `#[serde(tag = "type", version = 1)]`. Changes bump version; old reader kept for N-1 release. |
| **Error Handling** | Builder fails with `Error::Input` for negative notional, empty schedule, or unordered flows. `apply_amortisation` validates sum of amort steps ≤ initial notional. |
| **Memory Model** | For large portfolios flows stored in `Vec<CashFlow>`; legs optionally compress identical accrual factors into `Arc<[f64]>` lookup table. |
| **Performance Target** | Portfolio of 2 million flows PV in < 50 ms single-thread; with `parallel` feature, < 8 ms on 16 cores. |
| **Builder Ergonomics** | `CashFlowLegBuilder` consumes `Schedule`; immutable after `build` ensuring thread-safe sharing. |
| **DV01 Helpers** | Implemented in **risk** module, not here (per project decision).

## 6 Algorithms
1. **Schedule Generation** (dates module): iterate periods based on frequency; business-day adjust via calendar and `BusDayConv`.
2. **Accrual Factor**: compute `year_fraction(start, end, day_count)` directly via `DayCount`.
3. **NPV**: Present value = `Σ amount * curve.df(date)`; inlined loop; SIMD reduction when `parallel` feature is on.
4. **Amortisation Processing**: after base leg generation, traverse flows and mutate notional per amort rule.
5. **Stub Periods**: builder detects irregular first/last period and marks flow with `CFKind::Stub`; accrual uses actual period length.

## 7 Feature Flags
* `default = []`
* `decimal128` – high-precision amounts (`rust_decimal::Decimal`).
* `parallel` – enable Rayon `par_iter()` in NPV helpers.

## 8 Integration with Other Modules
* **dates**: relies on `Date`, `Schedule`, `DayCount`, `Frequency`, and `BusDayConv`.
* **calendar**: requires `HolidaySet` for schedule adjustment and settlement.
* **curves**: consumes `DiscountCurve` trait which provides `df(Date)` and internal interpolation.
* **error**: propagates via crate-wide `Error` enum.

## 9 Testing Strategy
* Unit tests: fixed, floating, amort, stub, fee, principal exchange.
* Property tests: amort schedule idempotence; accrued interest monotone ↑ between coupons.
* Criterion benches: NPV 2 M flows < 50 ms (f64, single-thread); parallel bench < 8 ms.

## 10 Open Questions
_Relocated / Resolved_
1. **Index reset dates** are now stored on `CashFlow::reset_date` (see § 5.2).
2. **Currency conversion** to be addressed in a dedicated money/FX utility module.
3. **DV01 & exposures** will be delivered by the upcoming **risk** module, not within cashflow.

## 11 Timeline
* **v0.1.0** – Core structs, fixed leg builder, NPV.
* **v0.2.0** – Amortisation.
* **v0.3.0** – Stub support, float leg (depends on index curves).
* **v1.0.0** – API freeze after integration with instruments layer.

---
*Last updated: 2025-06-29* 