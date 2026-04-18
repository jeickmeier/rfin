# Finstack

Finstack is a Rust-first quantitative finance workspace with Python and
WebAssembly bindings. The repository is organized around reusable financial
primitives, pricing and risk engines, statement modeling, deterministic
scenario tooling, portfolio analytics, and thin binding layers that keep the
business logic in Rust.

## Workspace Layout

```text
rfin/
├── finstack/
│   ├── Cargo.toml                 # `finstack` umbrella crate
│   ├── src/                       # Feature-gated re-exports
│   ├── core/                      # Dates, money, market data, math, expressions
│   ├── cashflows/                 # Schedule construction and cashflow aggregation
│   ├── analytics/                 # Return-series performance and risk analytics
│   ├── correlation/               # Copulas, factor models, recovery models
│   ├── monte_carlo/               # Simulation engine, processes, payoffs, pricers
│   ├── margin/                    # Margin, collateral, and XVA primitives
│   ├── statements/                # Financial statement modeling and evaluation
│   ├── statements-analytics/      # Higher-level statement analytics and reporting
│   ├── valuations/                # Instruments, pricing, metrics, calibration
│   ├── portfolio/                 # Portfolio valuation, grouping, optimization
│   └── scenarios/                 # Scenario composition and application
├── finstack-py/                   # PyO3 bindings packaged as `finstack`
├── finstack-wasm/                 # wasm-bindgen bindings packaged as `finstack-wasm`
├── docs/                          # Shared references and project documentation
├── book/                          # mdBook documentation
├── pyproject.toml                 # Python packaging and tooling
├── Cargo.toml                     # Workspace manifest
└── Makefile                       # Common build, test, and packaging commands
```

## Library Map

- `finstack-core`: currencies, money, rates, dates, calendars, market data,
  cashflow primitives, math utilities, and the expression engine.
- `finstack-cashflows`: schedule construction, accrual logic, and
  currency-preserving cashflow aggregation for bonds, loans, swaps, and
  structured products.
- `finstack-analytics`: return-series performance analytics, drawdown analysis,
  tail risk, benchmark-relative metrics, and rolling statistics.
- `finstack-monte-carlo`: generic Monte Carlo engine, stochastic processes,
  discretizations, payoffs, variance reduction, and result types.
- `finstack-margin`: CSA and repo margin specs, VM/IM engines, SIMM helpers,
  collateral eligibility, and XVA primitives.
- `finstack-statements`: period-based financial statement modeling,
  forecasting, formula evaluation, and extension hooks.
- `finstack-statements-analytics`: higher-level analysis on top of
  `finstack-statements`, including scenarios, variance tooling, templates,
  reporting, and covenant-oriented workflows.
- `finstack-valuations`: instrument coverage across rates, credit, equity, FX,
  structured products, and private markets, plus pricing, metrics,
  attribution, covenants, and calibration.
- `finstack-portfolio`: entity and position containers, aggregation, grouping,
  selective repricing, factor decomposition, optimization, and scenario-aware
  workflows.
- `finstack-scenarios`: deterministic scenario composition, market-data and
  statement shocks, instrument shocks, and time roll-forward workflows.

## Umbrella Crate

The top-level Rust crate is `finstack`, which re-exports every sub-crate so
downstream consumers reach the full API through a single dependency.

```toml
[dependencies]
finstack = { path = "finstack" }
```

Two pass-through features remain to gate heavy compile-time costs:

- `mc` — enables `finstack-monte-carlo`/`margin`/`valuations` Monte Carlo paths
  (pulls in `nalgebra`).
- `dataframes` — enables the `polars`-based DataFrame surfaces in
  `finstack-statements`/`finstack-portfolio`.

`finstack-cashflows` is a standalone workspace crate and a direct dependency of
`finstack-valuations`.

## Python Bindings

`finstack-py` builds the Python package named `finstack`. The binding tree
mirrors the Rust domain layout and currently exposes these top-level
subpackages:

- `analytics`
- `core`
- `correlation`
- `margin`
- `monte_carlo`
- `portfolio`
- `scenarios`
- `statements`
- `statements_analytics`
- `valuations`

The Python examples and notebook curriculum live under `finstack-py/examples/`.
The notebook runner is `finstack-py/examples/run_all_notebooks.py`.

## WASM Bindings

`finstack-wasm` builds the `finstack-wasm` package for browser and Node.js
consumers. It exposes namespaced modules that mirror the Rust workspace:

- `core`
- `analytics`
- `correlation`
- `margin`
- `monte_carlo`
- `portfolio`
- `scenarios`
- `statements`
- `statements_analytics`
- `valuations`

The package facade lives in `finstack-wasm/index.js`, TypeScript declarations
live in `finstack-wasm/index.d.ts`, and the namespace shims live in
`finstack-wasm/exports/`.

## Development Setup

### Prerequisites

- [Rust 1.90+](https://rustup.rs/)
- [Python 3.12+](https://www.python.org/)
- [uv](https://github.com/astral-sh/uv)
- [Node.js](https://nodejs.org/) for the WASM package
- [wasm-pack](https://rustwasm.github.io/wasm-pack/) for WASM builds

### Quick Start

```bash
git clone https://github.com/rustfin/rfin.git
cd rfin

make build
make test
make python-dev
make wasm-pkg
```

### Common Commands

| Command | Purpose |
|---|---|
| `make build` | Build the Rust workspace excluding binding crates |
| `make test` | Run Rust, Python, and WASM tests |
| `make fmt` | Format Rust, Python, and WASM code |
| `make lint` | Run the fast lint pass across Rust, Python, and WASM |
| `make lint-full` | Run the slower full lint pass including bindings |
| `make python-dev` | Build the Python extension (dev profile, fast compile) |
| `make python-dev-release` | Build the Python extension in release mode (slow compile, faster runtime) |
| `make python-dev-debug` | Alias for `make python-dev` |
| `make wasm-pkg` | Build the web and node WASM packages |
| `make test-rust` | Run Rust tests with `cargo nextest` |
| `make test-python` | Run Python tests |
| `make test-wasm` | Run WASM package tests |
| `make coverage` | Run coverage for the supported components |
| `make doc` | Generate Rust documentation |

## Documentation

- `docs/` for shared references, standards, and design notes.
- `book/src/architecture/README.md` for the workspace architecture map.
- `finstack-py/examples/README.md` for the Python notebook curriculum.
- `book/src/notebooks/README.md` for notebook navigation from the mdBook side.

## License

MIT OR Apache-2.0
