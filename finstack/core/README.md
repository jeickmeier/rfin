# finstack-core

`finstack-core` is the foundational crate for the Finstack workspace. It owns
the shared types and utilities that higher-level crates build on: currencies,
money, rates, dates, calendars, market data containers, cashflow primitives,
math helpers, configuration, and the expression engine.

## What This Crate Covers

- **Types and money**: currencies, money amounts, rates, basis points,
  percentages, credit ratings, and other shared typed wrappers.
- **Dates and calendars**: business-day conventions, holiday calendars, day
  counts, tenors, period identifiers, fiscal period helpers, and schedule
  utilities.
- **Market data**: discount, forward, hazard, inflation, and base-correlation
  curves, volatility surfaces, FX matrices, time series, and market contexts.
- **Cashflow primitives**: shared dated cashflow types used across schedules
  and pricing crates.
- **Math and numerics**: interpolation, solvers, integration, statistics,
  matrix helpers, and compensated summation.
- **Expression engine**: AST-based expression evaluation and lowering utilities
  used by statement-oriented workflows.
- **Configuration**: rounding policies and shared runtime settings.

## Module Map

The main crate-local documentation lives in the module READMEs under `src/`:

- `src/README.md`
- `src/dates/README.md`
- `src/market_data/README.md`
- `src/math/README.md`
- `src/cashflow/README.md`
- `src/expr/README.md`
- `src/money/README.md`
- `src/types/README.md`

## Cargo Features

`finstack-core` currently defines no crate-local Cargo features. Shared
capabilities such as serde wire formats, tracing calls, and golden-test helpers
are compiled as part of the crate.

## Typical Usage

Depend on the crate directly:

```toml
[dependencies]
finstack-core = { path = "../finstack/core" }
```

Or consume it through the umbrella crate:

```toml
[dependencies]
finstack = { path = "../finstack" }
```

## Where It Fits

Reach for `finstack-core` when you need shared primitives and deterministic
building blocks. Reach for adjacent crates when you need more specialized
behavior:

- `finstack-cashflows` for schedule construction and accrual workflows.
- `finstack-valuations` for pricing, metrics, calibration, and attribution.
- `finstack-statements` for financial statement modeling and evaluation.
- `finstack-analytics` for return-series performance and risk analytics.

## Verification

```bash
cargo fmt -p finstack-core
cargo clippy -p finstack-core --all-targets -- -D warnings
cargo test -p finstack-core
RUSTDOCFLAGS="-D warnings" cargo doc -p finstack-core --no-deps
cargo test -p finstack-core --doc
```

## License

MIT OR Apache-2.0
