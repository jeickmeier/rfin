## Cashflow Module (core)

The `cashflow` module in `finstack-core` provides **foundational building blocks for dated cashflows and present value calculations**. It is deliberately **instrument‑agnostic**: instruments in the `valuations` crate build their payment schedules and pricing logic on top of these primitives.

- **Primitives**: `CashFlow`, `CFKind` and related validation
- **Discounting**: NPV against market discount curves via the `Discounting` trait
- **XIRR**: IRR and XIRR for investment analysis

This module focuses on **small, composable types** and **deterministic numerics**; instrument‑specific logic belongs in `valuations`, not here.

---

## Module Structure

- **`mod.rs`**
  - Public entrypoint for the cashflow module.
  - Re‑exports:
    - `primitives::{CashFlow, CFKind}`
    - `discounting::{npv, Discountable}`
    - `xirr::{InternalRateOfReturn}`
- **`primitives.rs`**
  - Defines:
    - `CFKind`: classification enum for cashflows (fixed coupon, fees, margin flows, principal, recovery, etc.).
    - `CashFlow`: single dated cashflow with `Date`, `Money`, `CFKind`, accrual factor, and optional rate.
  - Provides `CashFlow::validate()` for basic numeric and date sanity checks.
- **`discounting.rs`**
  - Curve‑based present value helpers:
    - `trait Discountable`: generic NPV interface for `AsRef<[(Date, Money)]>`.
    - `npv`: NPV with optional day count; uses the curve's day count when `None` (recommended for par-rate consistency).
    - `npv_amounts`: NPV for scalar (f64) cashflows using a flat discount rate.
  - Integrates with `market_data::traits::Discounting` and `dates::DayCount`.
- **`xirr.rs`**
  - **Internal Rate of Return**:
    - `InternalRateOfReturn` trait: unified interface for periodic and irregular flows.
    - `solve_rate_of_return`: shared numerical solver logic.

---

## Core Types and Traits

### `CashFlow`

`CashFlow` represents a **single dated monetary flow** with classification metadata:

- **Fields (key ones)**:
  - `date: Date` – payment or reset date.
  - `reset_date: Option<Date>` – index reset for floating coupons.
  - `amount: Money` – currency‑tagged amount (currency‑safe).
  - `kind: CFKind` – cashflow classification.
  - `accrual_factor: f64` – day‑count‑based accrual used to compute the cashflow.
  - `rate: Option<f64>` – effective annual rate for rate‑based flows, when known.
- **Validation**:
  - Non‑zero, finite `amount`.
  - Finite `accrual_factor` and `rate` (if present).
  - `reset_date <= date` when provided.

`CashFlow` is intentionally kept **small and POD‑like** (size‑bounded in tests) to support large schedules and vectorized operations in the `valuations` crate.

### `CFKind`

`CFKind` is a **non‑exhaustive enum** used to classify flows without imposing a holder/issuer sign convention:

- Coupons: `Fixed`, `FloatReset`
- Fees: `Fee`, `CommitmentFee`, `UsageFee`, `FacilityFee`
- Principal: `Notional`, `PIK`, `Amortization`, `PrePayment`, `RevolvingDraw`, `RevolvingRepayment`
- Credit events: `DefaultedNotional`, `Recovery`
- Schedule metadata: `Stub`
- Margin & collateral: `InitialMarginPost`, `InitialMarginReturn`, `VariationMarginReceive`,
  `VariationMarginPay`, `MarginInterest`, `CollateralSubstitutionIn`, `CollateralSubstitutionOut`

Instruments are responsible for mapping these kinds into their own view (e.g., positive/negative sign for holder vs issuer). **Do not bake view‑specific semantics into `CFKind`.**

### `Discountable`

The `Discountable` trait provides a unified way to **present‑value dated `Money` flows** against any discount curve implementing `Discounting`:

- Implemented for any `T: AsRef<[(Date, Money)]>`, including:
  - `&[(Date, Money)]`
  - `Vec<(Date, Money)>`
  - `SmallVec<(Date, Money)>` (via `AsRef`).
- Core method:
  - `fn npv(&self, disc: &dyn Discounting, base: Date, dc: DayCount) -> Result<Money>`

This lets instruments and portfolios reuse the same discounting core while retaining their own scheduling logic.

---

## Usage Examples

### Present Value with a Discount Curve

Use `npv` with `Some(day_count)` when you need an explicit day count, or `None` to use the curve's day count (recommended), or `Discountable::npv` for a generic container:

```rust
use finstack_core::cashflow::discounting::{npv, Discountable};
use finstack_core::currency::Currency;
use finstack_core::dates::DayCount;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use time::macros::date;

// Build a simple discount curve
let base = date!(2025 - 01 - 01);
let curve = DiscountCurve::builder("USD-OIS")
    .base_date(base)
    .day_count(DayCount::Act365F)
    .knots([(0.0, 1.0), (1.0, 0.95)])
    .build()?;

// Dated Money flows to discount
let flows = vec![(
    date!(2026 - 01 - 01),
    Money::new(1_000.0, Currency::USD),
)];

// 1) Static helper
let pv_explicit = npv(&curve, base, Some(DayCount::Act365F), &flows)?;

// 2) Via the Discountable trait
let pv_trait = flows.npv(&curve, base, None)?; // Uses curve's day count

assert!((pv_explicit.amount() - pv_trait.amount()).abs() < 1e-12);
# Ok::<(), finstack_core::Error>(())
```

To ensure **par-rate consistency** between metrics and NPV, use `npv(&curve, base, None, &flows)` so the curve's own `day_count` is used.

### NPV with a Flat Discount Rate

For **project or investment analysis** where a single discount rate is sufficient, use `npv_amounts`:

```rust
use finstack_core::cashflow::npv_amounts;
use finstack_core::dates::DayCount;
use time::macros::date;

let base = date!(2025 - 01 - 01);
let cashflows = vec![
    (base, -100_000.0),
    (date!(2026 - 01 - 01), 110_000.0),
];

let pv = npv_amounts(&cashflows, 0.05, Some(base), Some(DayCount::Act365F))?;
assert!(pv > 0.0); // positive NPV at 5% hurdle rate
# Ok::<(), finstack_core::Error>(())
```

This is independent of market curves and uses a scalar solver; it is not meant for instrument pricing against full term structures.

### IRR for Periodic Cashflows

When cashflows occur at **evenly spaced periods** (0, 1, 2, …), use the `InternalRateOfReturn` trait:

```rust
use finstack_core::cashflow::xirr::InternalRateOfReturn;

// Annual project: -100k now, 30k/year for 5 years
let amounts = vec![-100_000.0, 30_000.0, 30_000.0, 30_000.0, 30_000.0, 30_000.0];

let irr = amounts.irr(None)?;
assert!(irr > 0.10 && irr < 0.20); // ~15% annual IRR
# Ok::<(), finstack_core::Error>(())
```

`irr` uses a Newton solver with derivative and a small grid of seeds to improve robustness around challenging regions (e.g., near −100% or very high returns).

### XIRR for Irregular Cashflows

For **irregularly dated cashflows** (typical in private equity, real estate, or mutual funds), use the `InternalRateOfReturn` trait:

```rust
use finstack_core::cashflow::xirr::InternalRateOfReturn;
use finstack_core::dates::DayCount;
use time::macros::date;

// Irregular private investment schedule
let flows = vec![
    (date!(2023 - 01 - 15), -100_000.0),
    (date!(2023 - 06 - 30), -50_000.0),
    (date!(2024 - 03 - 15), 75_000.0),
    (date!(2024 - 12 - 31), 95_000.0),
];

// Excel‑compatible (Act/365F)
let irr_act365f = flows.irr(None)?;

// Alternate day count (e.g., Act/360 for money‑market style)
let irr_act360 = flows.as_slice().irr_with_daycount(DayCount::Act360, None)?;

assert!(irr_act365f != irr_act360);
# Ok::<(), finstack_core::Error>(())
```

Inputs may be unsorted; they are internally sorted and the earliest date is used as the base. A sign change (at least one negative and one positive value) is required.

---

## Error Handling and Invariants

- All public functions return `crate::Result<T>` using the shared `Error` type.
- Common error conditions:
  - `InputError::TooFewPoints` for empty or insufficient cashflow arrays.
  - `InputError::Invalid` for invalid sign patterns (e.g., no sign change for IRR/XIRR) or non‑finite values.
  - Day‑count calculation errors propagate from `dates::DayCount`.
- **Determinism**:
  - No randomness is used; solvers are deterministic for a given input and configuration.
  - Sorting behavior in XIRR is stable and defined (ascending by date).

When integrating from higher‑level crates, treat these errors as **input validation failures** or **configuration issues** rather than runtime surprises.

---

## Adding New Features

The `cashflow` module is **core infrastructure** for the rest of Finstack. When extending it, keep changes **small, deterministic, and instrument‑agnostic**.

### Extending `CFKind`

Add a new `CFKind` variant when you need a **new semantic classification** of cashflows that:

- Is expected to be reused across multiple instruments or products.
- Represents a distinct category for risk, accounting, or reporting (not just a view/sign change).

Checklist:

- Add the variant with clear doc comments that are **view‑agnostic**.
- Do **not** change or remove existing variants or their serde names (backwards compatibility).
- Add unit tests to `primitives.rs` that:
  - Construct a `CashFlow` with the new kind.
  - Call `validate()` to ensure invariants hold.

Instrument‑specific labels or views (e.g., “BorrowerInterestOutflow”) should be modeled in the `valuations` crate on top of `CFKind`, not here.

### New Cashflow Primitives or Fields

If you need to extend `CashFlow` itself:

- Keep the struct **compact**; check the size bound test and update it if there is a justified change.
- Document new fields thoroughly and update `CashFlow::validate()` to enforce invariants.
- Ensure serde (under the `serde` feature) remains stable:
  - Use defaults for new optional fields.
  - Avoid renaming existing serialized fields.

### New Discounting Helpers

New functionality that depends on discount curves should:

- Live in `discounting.rs` (not in `primitives`).
- Accept a `Discounting` implementation and explicit `Date` / `DayCount` arguments.
- Enforce **currency safety**:
  - All `Money` inputs must share the same currency.
  - FX conversions must occur outside this module via `FxProvider`.
- Use `signed_year_fraction` for year fractions and propagate `Result` from day‑count operations.
- Include:
  - Module‑level and function‑level docs with examples.
  - Unit tests for:
    - Empty/invalid input behavior.
    - Happy paths with simple discount curves (e.g., flat curve in tests).

### New Performance or XIRR Variants

If you need additional performance metrics or alternative IRR/XIRR flavors:

- Put **IRR/XIRR logic** in `xirr.rs`.
- Reuse solvers from `math::solver` / `math::solver_multi`; do not introduce ad‑hoc solvers.
- Follow the documentation standards:
  - Explain the financial model and assumptions.
  - Include formulas and references to textbooks or standards (GIPS, Excel, etc.).
  - Provide doctested examples.

---

## When to Use This Module vs. `valuations`

- **Use `core::cashflow` when**:
  - You need generic cashflow primitives, curve‑based NPV helpers, or IRR/XIRR calculations.
  - You are implementing new instruments or analytics in `valuations` and need reusable building blocks.
- **Use `valuations` when**:
  - You are constructing full instrument pricing, risk metrics, or portfolio aggregation.
  - You need instrument‑specific schedules, amortization logic, or embedded options.

Keeping this boundary clean ensures the `core` crate remains small, deterministic, and reusable across bindings (Python, WASM) and higher‑level analytics crates.
