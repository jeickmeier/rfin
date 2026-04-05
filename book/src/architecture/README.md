# Architecture

Finstack is organized as a Rust workspace with feature-gated crates. All
business logic lives in Rust; Python and WASM bindings are thin wrappers that
handle type conversion, error mapping, and language-specific ergonomics.

## Crate Map

```text
finstack (umbrella — re-exports behind feature flags)
├── finstack-core           Currency, money, dates, calendars, market data, math
├── finstack-cashflows      Cashflow schedule construction and projection
├── finstack-analytics      Expression engine, computed metrics
├── finstack-correlation    Copula, factor, and recovery models
├── finstack-monte-carlo    Monte Carlo simulation engine
├── finstack-margin         Margin, collateral, XVA primitives
├── finstack-valuations     Instrument pricing, calibration, risk analytics
├── finstack-portfolio      Portfolio valuation, grouping, optimization
├── finstack-statements     Financial statement modeling, waterfalls
├── finstack-statements-analytics  Covenant monitoring, statement analytics
└── finstack-scenarios      Scenario modeling, stress testing

finstack-py                 Python bindings (PyO3)
finstack-wasm               WASM bindings (wasm-bindgen)
```

## Feature Flags

The umbrella `finstack` crate exposes these features:

| Feature | Crates Included |
|---------|----------------|
| `core` | finstack-core |
| `analytics` | finstack-analytics |
| `margin` | finstack-margin |
| `valuations` | finstack-valuations, finstack-cashflows, finstack-correlation, finstack-monte-carlo |
| `portfolio` | finstack-portfolio |
| `statements` | finstack-statements, finstack-statements-analytics |
| `scenarios` | finstack-scenarios |
| `all` | Everything above |

## Design Philosophy

- **Determinism** — `Decimal` arithmetic by default; `FxHashMap` for reproducible
  iteration order; identical results across serial and parallel execution.
- **Currency safety** — `Money` types enforce currency matching; cross-currency
  operations require explicit FX policies.
- **Composition over inheritance** — traits and feature flags; you only pay for
  what you compile.
- **Logic stays in Rust** — binding crates do type conversion and error mapping
  only; no business logic in Python or WASM.

## Section Index

- [Core Primitives](core-primitives/README.md) — Currency, money, dates, calendars, configuration
- [Market Data](market-data/README.md) — Curves, surfaces, FX rates
- [Instruments](instruments/README.md) — Instrument types, pricer registry
- [Risk](risk/README.md) — Metrics, attribution, scenarios
- [Portfolio](portfolio/README.md) — Entities, positions, aggregation
- [Statements](statements/README.md) — Waterfalls, covenants, forecasting
- [Analytics](analytics/README.md) — Expression engine
- [Monte Carlo](monte-carlo/README.md) — Simulation engine
- [Binding Layer](binding-layer/README.md) — Python and WASM binding patterns
