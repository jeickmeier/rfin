# Builder Entry-Point Conventions

This document describes the three intentional builder entry-point patterns
used across the Finstack crate hierarchy and the rationale behind each.

## Summary

Three builder patterns coexist. All three are intentional and serve
different construction needs.

| Pattern | Entry Point | When to Use | Examples |
|---------|-------------|-------------|----------|
| **A — Derive-macro builder** | `Type::builder()` | Types with many required fields where a derive macro generates the builder | `Bond`, `Swap`, `FRA`, `Equity`, `Basket` |
| **B — Keyed associated builder** | `Type::builder("id")` | Types with a single natural key and mostly optional fields (term structures) | `DiscountCurve`, `ForwardCurve`, `HazardCurve` |
| **C — Standalone builder** | `TypeBuilder::new(…)` | Complex multi-step builders with custom logic or type-state | `PortfolioBuilder`, `ModelBuilder`, `WaterfallBuilder` |

## Pattern A — `Type::builder()` (derive-macro)

### Mechanism

The `#[derive(FinancialBuilder)]` proc-macro generates a companion builder
struct and a no-arg `Type::builder()` associated function. Every field is
set through named setter methods, and `build()` validates required fields
at runtime.

### When to use

- The type has **many required fields** of distinct types (notional,
  dates, rates, curve IDs, etc.).
- A derive macro can mechanically generate all setters.
- ID is just one of many required fields — no single field is
  "more natural" as a constructor argument.

### Canonical examples

```rust
// Bond — finstack/valuations/src/instruments/fixed_income/bond/types.rs
let bond = Bond::builder()
    .id("SETTLE_T0".into())
    .notional(Money::new(1000.0, Currency::USD))
    .issue(issue_date)
    .maturity(maturity_date)
    .cashflow_spec(CashflowSpec::fixed(0.05, Tenor::semi_annual(), DayCount::Act365F))
    .discount_curve_id("USD-OIS".into())
    .build()?;

// FRA — finstack/valuations/src/instruments/rates/fra/types.rs
let fra = ForwardRateAgreement::builder()
    .id(InstrumentId::new("FRA-3X6-USD"))
    .notional(Money::new(10_000_000.0, Currency::USD))
    .start_date(date!(2024 - 04 - 03))
    .end_date(date!(2024 - 07 - 03))
    .fixed_rate(0.045)
    .build()?;

// Basket — finstack/valuations/src/instruments/exotics/basket/types.rs
let basket = Basket::builder()
    .id(InstrumentId::new("BASKET-60-40"))
    .constituents(constituents)
    .expense_ratio(0.0025)
    .currency(Currency::USD)
    .discount_curve_id(CurveId::new("USD-OIS"))
    .build()?;
```

### Defined in

- Proc-macro: [`finstack/valuations/macros/src/financial_builder.rs`](../finstack/valuations/macros/src/financial_builder.rs) (line 338)

---

## Pattern B — `Type::builder("id")` (keyed associated builder)

### Mechanism

A hand-written `builder(id)` associated function takes the type's natural
key as its only required argument and returns a builder struct pre-seeded
with that key. Remaining fields are set through chainable setters, and
`build()` performs validation.

### When to use

- The type has a **single natural key** (typically a `CurveId`).
- Most other fields have sensible defaults or are truly optional.
- The key is required in every construction — passing it up front
  eliminates a class of "missing ID" errors and communicates intent.

### Canonical examples

```rust
// DiscountCurve — finstack/core/src/market_data/term_structures/discount_curve.rs
let curve = DiscountCurve::builder("USD-OIS")
    .base_date(base)
    .knots([(0.0, 1.0), (5.0, 0.9)])
    .interp(InterpStyle::Linear)
    .build()?;

// ForwardCurve — finstack/core/src/market_data/term_structures/forward_curve.rs
// (takes id + tenor_years as required positional args)
let fwd = ForwardCurve::builder("USD-SOFR-3M", 0.25)
    .base_date(base)
    .knots([(0.0, 0.05), (5.0, 0.05)])
    .interp(InterpStyle::Linear)
    .build()?;

// HazardCurve — finstack/core/src/market_data/term_structures/hazard_curve.rs
let haz = HazardCurve::builder("USD-CREDIT")
    .base_date(base)
    .recovery_rate(0.40)
    .knots([(0.0, 0.01), (5.0, 0.015), (10.0, 0.02)])
    .build()?;
```

### Other types using this pattern

| Type | Required args | File |
|------|--------------|------|
| `DiscountCurve` | `id` | `finstack/core/src/market_data/term_structures/discount_curve.rs` |
| `ForwardCurve` | `id`, `tenor_years` | `finstack/core/src/market_data/term_structures/forward_curve.rs` |
| `HazardCurve` | `id` | `finstack/core/src/market_data/term_structures/hazard_curve.rs` |
| `InflationCurve` | `id` | `finstack/core/src/market_data/term_structures/inflation.rs` |
| `PriceCurve` | `id` | `finstack/core/src/market_data/term_structures/price_curve.rs` |
| `VolatilityIndexCurve` | `id` | `finstack/core/src/market_data/term_structures/vol_index_curve.rs` |
| `BaseCorrelationCurve` | `id` | `finstack/core/src/market_data/term_structures/base_correlation.rs` |

---

## Pattern C — `TypeBuilder::new(…)` (standalone builder)

### Mechanism

A standalone builder struct (`TypeBuilder`) lives alongside or near the
target type. `TypeBuilder::new(…)` is its constructor — typically taking
a single required argument (an ID or a currency). The builder exposes
domain-specific methods beyond simple setters (e.g. `add_tier()`,
`periods()`, `.value()`), and `build()` performs multi-step validation.

### When to use

- Construction involves **complex multi-step logic** that a derive macro
  cannot generate.
- The builder exposes domain-specific verbs (not just property setters).
- The builder may accumulate heterogeneous items (positions, tiers,
  time-series values).
- Type-state or ordering constraints may exist between steps.

### Canonical examples

```rust
// PortfolioBuilder — finstack/portfolio/src/builder.rs
let portfolio = PortfolioBuilder::new("MY_FUND")
    .base_ccy(Currency::USD)
    .as_of(as_of)
    .entity(Entity::new("ACME_CORP"))
    .position(position)
    .build()?;

// ModelBuilder — finstack/statements/src/builder/model_builder.rs
let model = ModelBuilder::new("test")
    .periods("2025Q1..Q4", None)?
    .value("revenue", &[100.0, 110.0, 120.0, 130.0])
    .build()?;

// WaterfallBuilder — finstack/valuations/src/instruments/fixed_income/structured_credit/types/waterfall.rs
let waterfall = WaterfallBuilder::new(Currency::USD)
    .add_tier(
        WaterfallTier::new("senior", 1, PaymentType::Interest)
            .add_recipient(Recipient::new("ClassA", 0.6)),
    )
    .add_tier(
        WaterfallTier::new("residual", 2, PaymentType::Residual)
            .add_recipient(Recipient::new("Equity", 1.0)),
    )
    .build()?;
```

---

## Design Rationale

### Why not unify into one pattern?

Each pattern addresses a distinct construction shape:

| Concern | Pattern A | Pattern B | Pattern C |
|---------|-----------|-----------|-----------|
| Number of required fields | Many (5–10+) | 1–2 key fields | 1 key + complex steps |
| Field types | Uniform setters | Uniform setters + defaults | Domain-specific verbs |
| Code generation | Derive macro | Hand-written | Hand-written |
| Validation complexity | Field presence checks | Field presence + curve consistency | Multi-step structural validation |

Forcing all types into a single entry point would either:

- **Remove the natural key** from Pattern B, making curve construction
  error-prone (easy to forget the ID).
- **Require a derive macro** for Pattern C, losing the domain-specific
  verbs and multi-step validation.
- **Add a required key arg** to Pattern A, providing no benefit since
  instruments already have many equally-important required fields.

### Decision record

The three-pattern approach was reviewed and found to be **defensible and
intentional**. The patterns map cleanly to construction complexity:

```
Simple key + defaults → Pattern B (Type::builder("id"))
Many required fields  → Pattern A (Type::builder() + derive macro)
Complex multi-step    → Pattern C (TypeBuilder::new(…))
```

## Guidelines for New Types

1. **Default to Pattern A** for new instrument types. Add
   `#[derive(FinancialBuilder)]` and let the macro generate the builder.

2. **Use Pattern B** for new term structures or any type with a single
   natural key and mostly-optional configuration. Keep the `builder(id)`
   method on the type itself.

3. **Use Pattern C** for new builders that need domain-specific verbs,
   accumulation methods, or multi-step validation. Name the builder
   `{Type}Builder` and provide `TypeBuilder::new(…)`.

4. **Never mix patterns** on the same type — pick one entry point and
   stick with it.

5. **Always end with `.build()`** returning `Result<T>`. All three
   patterns follow this convention.

## See Also

- [`FinancialBuilder` derive macro](../finstack/valuations/macros/src/financial_builder.rs) — Pattern A implementation
- [`DiscountCurve::builder`](../finstack/core/src/market_data/term_structures/discount_curve.rs) — Pattern B example
- [`PortfolioBuilder`](../finstack/portfolio/src/builder.rs) — Pattern C example
- [CONVENTIONS_ERROR_NAMING.md](./CONVENTIONS_ERROR_NAMING.md) — Companion conventions doc
