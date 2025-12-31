## Finstack Core `src/` Overview

The `finstack-core` crate provides the low‑level, deterministic building blocks for the Finstack ecosystem:
currencies and money, date and calendar utilities, numerical methods, expression evaluation, and
market‑data term structures. Everything under `src/` is designed to be:

- **Deterministic**: serial and parallel runs produce the same results.
- **Currency‑safe**: no implicit FX; all cross‑currency math is explicit and auditable.
- **Serde‑stable**: public types have well‑defined, versioned wire formats.

If you are writing pricing, risk, or reporting logic in other Finstack crates, you will almost always
depend on types and traits defined here.

## Directory Structure

Top‑level modules in `finstack/core/src`:

- **`lib.rs`**: Crate entry point, crate‑level docs, and public module declarations.
- **`config.rs`**: Numeric mode, rounding policy, `FinstackConfig`, and `ResultsMeta`/`RoundingContext`.
- **`currency.rs`**: ISO‑4217 currency enum and metadata; integrates with generated tables under `generated/`.
- **`money/`**: Currency‑tagged `Money` type, rounding helpers, FX matrix/provider implementations, and
  conversion policies. See `money/README.md` for details.
- **`dates/`**: Date/time facade over the `time` crate, plus calendars, business‑day logic, day‑count
  conventions, IMM helpers, tenors, schedules, and period builders. See `dates/README.md` for details.
- **`market_data/`**: Term structures (discount/forward/hazard/inflation/credit index/base correlation),
  scalar time series, inflation indices, dividend and bump utilities, and `MarketContext` for
  aggregating market data. See `market_data/README.md`.
- **`math/`**: Interpolation framework, root‑finding solvers, integration, statistics, random numbers,
  summation utilities, and basic linear algebra. See `math/README.md`.
- **`expr/`**: Expression engine (AST, planner, evaluator) with scalar and Polars‑lowered execution paths.
  See `expr/README.md`.
- **`cashflow/`**: Cashflow primitives, discounting helpers, XIRR/IRR, and performance utilities.
  See `cashflow/README.md`.
- **`types/`**: Newtype identifiers (`CurveId`, `InstrumentId`, etc.), rate types, ratings, and shared
  scalar types. See `types/README.md`.
- **`volatility.rs`**: Volatility conventions and conversion helpers.
- **`error.rs`**: Unified error type (`Error`) and input/validation error variants; re‑exported as
  `finstack_core::Error`.
- **`explain.rs`**: Explainability infrastructure for tracing and annotating computations.
- **`generated/`**: Code generated at build time (currencies, calendars, Chinese New Year tables).

Most submodules have their own `README.md` with deeper explanations and design notes.

## Using `finstack-core`

### As a dependency

In an external crate, add `finstack-core` as a dependency (check crates.io or this workspace’s
`Cargo.toml` for the current version):

```toml
[dependencies]
finstack-core = "x.y.z" # replace with the latest published version
```

Inside this workspace, other crates depend on the local `finstack-core` via the workspace `Cargo.toml`.

### Example: currency‑safe money

Import the types you need explicitly:

```rust
use finstack_core::currency::Currency;
use finstack_core::money::Money;

fn main() -> finstack_core::Result<()> {
    // Work with strongly typed currencies
    let eur = Currency::EUR;

    // Construct monetary amounts (stored as scaled integers internally)
    let subtotal = Money::new(49.50, eur);
    let tax      = Money::new(9.90, eur);

    // Checked arithmetic refuses to mix currencies
    let total = (subtotal + tax)?;
    assert_eq!(format!("{}", total), "EUR 59.40");

    Ok(())
}
```

### Example: dates, calendars, and day‑count

Date helpers wrap the `time` crate and provide business‑day logic and standard conventions:

```rust
use finstack_core::dates::{create_date, DayCount, DayCountCtx};
use time::Month;

fn main() -> finstack_core::Result<()> {
    let start = create_date(2025, Month::January, 1)?;
    let end   = create_date(2026, Month::January, 1)?;

    // Actual/Actual (ISDA) year fraction
    let yf = DayCount::ActAct
        .year_fraction(start, end, DayCountCtx::default())?;

    assert!((yf - 1.0).abs() < 1e-9);
    Ok(())
}
```

For business‑day conventions and holiday calendars, see the examples in `dates/README.md`.

### Example: discount curves and present value

`market_data::term_structures` provides bootstrapped discount curves and related primitives:

```rust
use finstack_core::dates::create_date;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use time::Month;

fn main() -> finstack_core::Result<()> {
    let base_date = create_date(2025, Month::January, 1)?;

    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (5.0, 0.9)])
        .set_interp(InterpStyle::MonotoneConvex)
        .build()?;

    let df_3y = curve.df(3.0);
    assert!(df_3y < 1.0);

    Ok(())
}
```

For richer examples (bootstrapping from instruments, hazard curves, inflation, etc.), see
`market_data/README.md` and the `finstack/core/tests/` directory.

## Adding New Features to `core/src`

When extending `finstack-core`, prefer evolving existing modules over adding ad‑hoc utilities.
The high‑level process is:

- **1. Choose the right module**
  - **Domain primitives** (IDs, rates, ratings): extend `types/`.
  - **Money/FX**: extend `money/` (`types.rs`, `fx.rs`, `fx/providers.rs`, `rounding.rs`).
  - **Dates/calendars/day‑count**: extend `dates/` and its submodules.
  - **Term structures and market observables**: extend `market_data/`.
  - **Numerical methods**: extend `math/`.
  - **Expressions**: extend `expr/`.
  - **Cashflows**: extend `cashflow/` primitives and helpers.

- **2. Design with core invariants in mind**
  - **Determinism**: serial ≡ parallel; avoid time‑dependent behavior and randomness.
  - **Currency‑safety**: never perform implicit FX; require an `FxProvider`/`FxMatrix` and document
    `FxConversionPolicy`.
  - **Serde stability**: for new public types, either derive `Serialize`/`Deserialize` behind the
    `serde` feature or introduce `*State`/`*Spec` DTOs with `to_state`/`from_state` helpers.
  - **Type safety**: prefer newtype IDs from `types::id` (`CurveId`, `InstrumentId`, etc.) over raw
    `String`.

- **3. Wire the new code**
  - Add a new module or file under the appropriate directory.
  - Expose it via the parent `mod.rs` and, if appropriate, through `lib.rs`.
  - Ensure any new public APIs have clear, concise docs and at least one example.

- **4. Test thoroughly**
  - Add unit tests close to the implementation (`mod tests { … }`).
  - Add integration or golden tests under `finstack/core/tests/` where behavior spans modules
    (e.g., new day‑count conventions, new term structure types, or new FX policies).
  - For serialization, add round‑trip tests and keep field names stable.

### Common Extension Patterns

- **New calendar**: add a JSON definition under `finstack/core/data/calendars/`, let `build.rs`
  regenerate `generated` code, and add tests under `core/tests/dates/`.
- **New day‑count convention**: add a variant to `dates::DayCount`, implement its logic in
  `dates/daycount.rs`, and add unit and integration tests.
- **New interpolation method**: add an implementation under `math/interp/`, extend `InterpStyle`,
  and ensure tests cover knot handling, extrapolation, and edge cases.
- **New term structure**: add a module under `market_data/term_structures/` with a concrete type,
  trait implementations (`TermStructure`, `Discounting`/`Forward`/`Survival`, etc.), a serializable
  state type, and integration with `MarketContext`.
- **New FX provider or policy**: extend `money/fx.rs` or `money/fx/providers.rs`, keep caching
  bounded and deterministic, and ensure applied policies are visible in results metadata.

## Further Reading

- **Module‑level READMEs**: `cashflow/README.md`, `dates/README.md`, `expr/README.md`,
  `market_data/README.md`, `math/README.md`, `money/README.md`, `types/README.md`.
- **Workspace book**: high‑level architecture, crate responsibilities, and design philosophy live
  under `book/src/`.
- **Tests and examples**: see `finstack/core/tests/` and `finstack/examples/core/` for concrete
  end‑to‑end usage patterns.
