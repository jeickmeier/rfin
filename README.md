# Finstack

Finstack is a Rust-first quantitative finance workspace with Python and WebAssembly bindings.
The repository combines reusable financial primitives, pricing and risk engines, statement
modeling, scenario analysis, portfolio tooling, and binding layers that keep the core logic in
Rust.

## Workspace Layout

```text
rfin/
├── finstack/                    # Rust workspace and umbrella crate
│   ├── core/                    # Dates, calendars, currencies, market data, math
│   ├── cashflows/               # Cashflow schedule construction and projection
│   ├── correlation/             # Copula, factor, and recovery models
│   ├── monte_carlo/             # Monte Carlo simulation engine
│   ├── analytics/               # Shared analytics utilities
│   ├── margin/                  # Margin, collateral, and XVA primitives
│   ├── statements/              # Financial statement modeling and forecasting
│   ├── valuations/              # Instrument pricing, calibration, and risk
│   ├── portfolio/               # Portfolio valuation, grouping, and optimization
│   ├── scenarios/               # Scenario modeling and stress testing
│   └── src/                     # `finstack` umbrella crate re-exports
├── finstack-py/                 # Python bindings built with PyO3 + maturin
├── finstack-wasm/               # WebAssembly bindings built with wasm-bindgen
├── docs/                        # Project references and documentation standards
├── scripts/                     # Audits and developer automation
├── pyproject.toml               # Python tooling and dependency configuration
├── Cargo.toml                   # Workspace configuration and shared profiles
└── Makefile                     # Common build, test, lint, and packaging commands
```

## What The Project Covers

The current workspace is organized around a few major areas:

- `finstack-core` for shared financial primitives, market data containers, interpolation, solvers,
  and numerical utilities.
- `finstack-cashflows`, `finstack-correlation`, and `finstack-monte-carlo` for specialized
  modeling engines used by higher-level crates.
- `finstack-valuations` for instruments, pricers, calibration, and risk analytics across rates,
  credit, equity, FX, structured products, and more.
- `finstack-statements` for financial statement modeling, formulas, forecasting, and extensions.
- `finstack-scenarios` and `finstack-portfolio` for stress testing, portfolio aggregation, and
  multi-position workflows.
- `finstack-py` and `finstack-wasm` for Python and TypeScript/browser consumers.

## Rust Workspace

The top-level Rust crate is `finstack`, an umbrella crate that re-exports the major sub-crates
behind feature flags so downstream users can opt into only the parts they need.

```toml
[dependencies]
finstack = { path = "finstack", features = ["valuations", "portfolio", "scenarios"] }
```

Available umbrella features include:

- `core`
- `analytics`
- `margin`
- `statements`
- `valuations`
- `portfolio`
- `scenarios`
- `all`

Default workspace builds exclude the Python and WASM binding crates, which keeps the normal Rust
edit-test loop fast.

## Python Bindings

`finstack-py` exposes the Rust functionality through PyO3 bindings and is built with `maturin`.
The Python package is configured from the repository root `pyproject.toml`, uses `uv` for package
management, and includes manually maintained `.pyi` stubs plus example scripts and notebooks.

Useful paths:

- `finstack-py/finstack/` for the package and type stubs
- `finstack-py/tests/` for Python and parity tests
- `finstack-py/examples/scripts/` for runnable examples
- `finstack-py/examples/notebooks/` for notebooks
- `finstack-py/docs/` for Python-focused documentation

## WASM Bindings

`finstack-wasm` provides browser and Node.js bindings via `wasm-bindgen` and `wasm-pack`. The
package emits TypeScript definitions and also includes an example app workflow for local
development.

Useful paths:

- `finstack-wasm/src/` for bindings
- `finstack-wasm/examples/` for the example app
- `finstack-wasm/package.json` for JS tooling commands

## Development Setup

### Prerequisites

- [Rust 1.90+](https://rustup.rs/)
- [Python 3.12+](https://www.python.org/)
- [uv](https://github.com/astral-sh/uv)
- [Node.js](https://nodejs.org/) for WASM tooling
- [wasm-pack](https://rustwasm.github.io/wasm-pack/) for WASM builds

### Quick Start

```bash
git clone https://github.com/rustfin/rfin.git
cd rfin

# Build the default Rust workspace
make build

# Run Rust, Python, and WASM tests
make test

# Build Python bindings for local development
make python-dev

# Build the WASM packages
make wasm-pkg
```

## Common Commands

Run `make help` for the full command list. These are the targets used most often:

| Command | Purpose |
|---|---|
| `make build` | Build the Rust workspace excluding `finstack-py` and `finstack-wasm` |
| `make test` | Run Rust, Python, and WASM tests |
| `make test-rust` | Run Rust tests with `cargo nextest` |
| `make test-python` | Run Python tests with `pytest` |
| `make test-wasm` | Run WASM tests |
| `make fmt` | Format Rust, Python, and WASM code |
| `make lint` | Run the fast lint pass across Rust, Python, and WASM |
| `make lint-full` | Run the slower full Rust lint pass including bindings and all features |
| `make python-dev` | Create the Python environment and build bindings in release mode |
| `make python-dev-debug` | Build Python bindings in debug mode for a faster compile loop |
| `make wasm-examples-dev` | Build WASM and start the example app |
| `make coverage` | Run Rust and Python coverage reports |
| `make wheel-local` | Build a wheel for the current platform |
| `make wheel-all` | Build wheels for all locally available Python interpreters |
| `make audit` | Run Rust and Python security audits |

## Code Quality

The workspace is set up for a strict development workflow:

- Rust formatting via `cargo fmt`
- Rust linting via `cargo clippy`
- Python formatting and linting via `ruff`
- Python typing via `ty` and `pyright`
- Python security checks via `bandit`
- WASM and frontend formatting via `prettier`
- WASM and frontend linting via `eslint`

For day-to-day development, `make fmt`, `make lint`, and `make test` are the main entry points.

## Examples And Documentation

The repo already includes several documentation surfaces:

- Root-level standards and references in `docs/`
- Python package docs in `finstack-py/docs/`
- Python example scripts in `finstack-py/examples/scripts/`
- Python notebooks in `finstack-py/examples/notebooks/`
- Audit and parity tooling in `scripts/audits/`

## License

MIT OR Apache-2.0
