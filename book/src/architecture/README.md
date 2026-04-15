# Architecture

Finstack is organized as a Rust workspace with thin Python and WASM bindings.
The Rust crates own the canonical API design and business logic; bindings are
responsible for type conversion, registration, and host-language ergonomics.

## Crate Map

```text
finstack (umbrella crate with feature-gated re-exports)
├── finstack-core                 currencies, money, dates, calendars, market data, math, expressions
├── finstack-cashflows            schedule construction, accrual, and aggregation
├── finstack-analytics            return-series performance and risk analytics
├── finstack-correlation          copulas, factor models, recovery models
├── finstack-monte-carlo          simulation engine, processes, payoffs, pricers
├── finstack-margin               margin, collateral, SIMM, XVA primitives
├── finstack-statements           financial statement modeling and evaluation
├── finstack-statements-analytics higher-level statement analytics, templates, reporting
├── finstack-valuations           instruments, pricing, metrics, calibration, attribution
├── finstack-portfolio            portfolio construction, valuation, aggregation, optimization
└── finstack-scenarios            deterministic scenario composition and application

finstack-py                       Python bindings (PyO3)
finstack-wasm                     WebAssembly bindings (wasm-bindgen)
```

## Umbrella Features

The umbrella `finstack` crate exposes these domain features:

| Feature | Included crates |
|---|---|
| `core` | `finstack-core` |
| `analytics` | `finstack-core`, `finstack-analytics` |
| `correlation` | `finstack-core`, `finstack-correlation` |
| `margin` | `finstack-core`, `finstack-margin` |
| `monte_carlo` | `finstack-core`, `finstack-monte-carlo` |
| `statements` | `finstack-core`, `finstack-statements`, `finstack-statements-analytics` |
| `valuations` | `finstack-core`, `finstack-valuations` |
| `scenarios` | `finstack-core`, `finstack-statements`, `finstack-statements-analytics`, `finstack-scenarios` |
| `portfolio` | `finstack-core`, `finstack-statements`, `finstack-statements-analytics`, `finstack-valuations`, `finstack-portfolio` |
| `all` | all of the above |

`finstack-cashflows` remains a standalone crate used directly by
`finstack-valuations` rather than a separate umbrella feature.

## Design Principles

- **Rust is canonical**: public type and function design starts in Rust.
- **Bindings stay thin**: Python and WASM perform conversion and registration,
  not business logic.
- **Determinism matters**: ordering, numerics, and serialization aim for stable
  cross-run behavior.
- **Feature-gated composition**: consumers opt into only the workspace domains
  they need.

## Section Index

- [Core Primitives](core-primitives/README.md) — money, rates, dates,
  calendars, and shared foundational types
- [Market Data](market-data/README.md) — curves, surfaces, FX, and market
  context containers
- [Instruments](instruments/README.md) — valuation-facing instrument families
  and pricing interfaces
- [Risk](risk/README.md) — metrics, attribution, and risk-oriented workflows
- [Portfolio](portfolio/README.md) — entities, positions, aggregation, and
  portfolio analytics
- [Statements](statements/README.md) — statement models, evaluation, and
  higher-level statement analytics
- [Analytics](analytics/README.md) — return-series performance and risk
  analytics
- [Monte Carlo](monte-carlo/README.md) — simulation infrastructure and pricing
  workflows
- [Binding Layer](binding-layer/README.md) — Python and WASM binding patterns
