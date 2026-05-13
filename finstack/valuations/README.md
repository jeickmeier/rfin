# Finstack Valuations

`finstack-valuations` is the pricing and risk engine for the Finstack
workspace. It combines broad instrument coverage with deterministic valuation,
cashflow-aware risk metrics, calibration, attribution, covenant tooling, and
schema-friendly result types.

## What This Crate Owns

The crate is organized around a few major subsystems:

- `src/instruments/`: instrument definitions, builders, pricing hooks, JSON
  loading, and instrument-specific docs.
- `src/metrics/`: metric identifiers, registries, dependency-aware calculators,
  scalar metrics, and bucketed sensitivities.
- `src/calibration/`: calibration plans, solvers, targets, validation, and
  bump helpers for market structures.
- `src/attribution/`: parallel, waterfall, and metrics-based P&L attribution.
- `src/covenants/`: covenant types, engines, schedules, and forward-looking
  covenant workflows.
- `src/results/`: valuation result envelopes and export helpers.
- `schemas/`: generated JSON Schema artifacts for external API contracts.

## Instrument Coverage

The instrument library spans 40+ instrument types across the main asset-class
families:

- **Fixed income**: bonds, convertibles, inflation-linked bonds, term loans,
  revolving credit facilities.
- **Rates**: deposits, swaps, basis swaps, inflation swaps, FRAs, swaptions,
  rate futures, CMS options, repos.
- **Credit**: single-name CDS, CDS indices, CDS tranches, CDS options, and
  structured credit.
- **Equity and exotics**: spot, vanilla options, Asian, barrier, lookback,
  autocallable, cliquet, range accrual, variance swaps.
- **FX**: spot, swaps, vanilla options, barrier options, quanto options.
- **Cross-asset and private markets**: total return swaps, basket instruments,
  private-markets funds.

## Ecosystem Dependencies

`finstack-valuations` builds on other workspace crates:

- `finstack-core` for dates, money, market data, and math primitives.
- `finstack-cashflows` for schedule construction, accrual, and aggregation.
- `finstack-margin` for margin and XVA integrations.
- `finstack-monte-carlo` for optional advanced simulation workflows.

Credit correlation, copula, factor-model, and stochastic-recovery tooling used
by structured-credit and CDS-tranche workflows lives inside this crate, in the
[`correlation`](src/correlation) submodule.

This crate is also surfaced through:

- the umbrella `finstack` crate via the `valuations` feature
- `finstack-py`
- `finstack-wasm`

## Feature Flags

| Feature | Default | Purpose |
|---|---|---|
| `parallel` | yes | Enables rayon-backed parallel execution in supported valuation paths. |
| `mc` | no | Enables advanced Monte Carlo workflows via `finstack-monte-carlo` and margin/XVA MC support. |
| `slow` | no | Enables slower long-running tests and benchmarks. |
| `ts_export` | no | Enables TypeScript export support for selected schema-oriented workflows. |

## Typical Dependency Setup

Depend on the crate directly:

```toml
[dependencies]
finstack-valuations = { path = "../finstack/valuations", features = ["mc"] }
```

Or consume it through the umbrella crate:

```toml
[dependencies]
finstack = { path = "../finstack", features = ["valuations"] }
```

## Additional Module Docs

For deeper coverage, use the crate-local READMEs:

- `src/instruments/README.md`
- `src/metrics/README.md`
- `src/calibration/README.md`
- `src/attribution/README.md`
- `src/covenants/README.md`
- `src/results/README.md`
- `schemas/README.md`

## Verification

```bash
cargo fmt -p finstack-valuations
cargo clippy -p finstack-valuations --all-targets --all-features -- -D warnings
cargo test -p finstack-valuations --all-features
```

## License

MIT OR Apache-2.0
