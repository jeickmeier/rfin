# Finstack

A high-performance quantitative finance library written in Rust with bindings for Python (PyO3) and WebAssembly (wasm-bindgen).

## Project Structure

```
rfin/
├── finstack/                  # Rust library (workspace)
│   ├── core/                  # Dates, calendars, currencies, curves, surfaces, math
│   ├── statements/            # Financial statement modeling, cashflows, DSL
│   ├── valuations/            # Instrument pricing, risk, calibration, Monte Carlo
│   ├── portfolio/             # Portfolio valuation, attribution, optimization
│   └── scenarios/             # Scenario modeling and stress testing
├── finstack-py/               # Python bindings (PyO3 / Maturin)
├── finstack-wasm/             # WebAssembly bindings (wasm-bindgen / wasm-pack)
├── docs/                      # Design documents and specs
└── scripts/                   # Developer tooling scripts
```

## Instrument Coverage

50+ instruments across seven asset classes:

| Asset Class | Instruments |
|---|---|
| **Fixed Income** | Bond, Inflation-Linked Bond, Convertible, Term Loan, Revolving Credit, Structured Credit, Agency MBS, Agency CMO, Bond Future, Dollar Roll, TBA |
| **Rates** | IRS, Basis Swap, Swaption, Cap/Floor, Deposit, FRA, Repo, Inflation Swap, Inflation Cap/Floor, Range Accrual, Cross-Currency Swap, CMS Option, IR Future |
| **Credit** | CDS, CDS Index, CDS Tranche, CDS Option |
| **Equity** | Equity Option, Variance Swap, Equity TRS, Equity Index Future, Vol Index Future/Option, Autocallable, Cliquet, DCF, Real Estate, Private Markets Fund |
| **FX** | Spot, Forward, FX Swap, Vanilla Option, Barrier, Digital, Touch, FX Variance Swap, NDF, Quanto |
| **Commodity** | Forward, Swap, Option, Asian Option |
| **Exotics** | Asian Option, Barrier Option, Lookback Option, Basket Option |

### Pricing Models

- **Closed-form** -- Black-Scholes, SABR, Heston, Asian (Turnbull-Wakeman), Barrier, Lookback, Quanto, implied vol solvers
- **Trees** -- Binomial, Trinomial, Hull-White, short-rate, two-factor (rates/credit)
- **Monte Carlo** -- GBM, Heston, CIR, Jump Diffusion, Bates, Schwartz-Smith; variance reduction (antithetic, control variate, moment matching); LSM for early exercise
- **Volatility** -- Black vol, Normal vol, SABR, Local vol, Heston

## Tech Stack

| Layer | Technology |
|---|---|
| Core library | Rust 1.90, edition 2021 |
| Python bindings | PyO3, Maturin, Python >= 3.12 |
| WASM bindings | wasm-bindgen, wasm-pack, TypeScript definitions |
| Build/CI | GitHub Actions, cargo-nextest, pre-commit, cargo-deny |
| Package mgmt | uv (Python), npm (WASM/JS) |
| Code quality | clippy (strict), ruff, prettier, ty, pyright, bandit |

## Development Setup

### Prerequisites

- [Rust 1.90+](https://rustup.rs/) (pinned via `rust-toolchain.toml`)
- [Python 3.12+](https://www.python.org/)
- [uv](https://github.com/astral-sh/uv) (Python package manager)
- [Node.js 20+](https://nodejs.org/) (for WASM examples)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/) (for WASM builds)

### Quick Start

```bash
git clone https://github.com/rustfin/rfin.git
cd rfin

# Build core Rust crates
cargo build

# Run all Rust tests
make test-rust

# Set up Python environment + build bindings
make python-dev

# Run Python tests
make test-python
```

### Makefile Targets

Run `make help` for the full list. Key targets:

| Target | Description |
|---|---|
| `make build` | Build all core Rust crates |
| `make test` | Run all tests (Rust + Python + WASM) |
| `make test-rust` | Rust tests via cargo-nextest |
| `make test-python` | Python tests via pytest |
| `make test-wasm` | WASM tests via wasm-pack |
| `make fmt` | Format all code (Rust, Python, JS) |
| `make lint` | Lint core Rust crates (fast) |
| `make lint-full` | Lint everything including bindings and all features |
| `make ci-test` | Run the full CI pipeline locally |
| `make coverage` | Code coverage (cargo-llvm-cov) |
| `make python-dev` | Set up Python env and build bindings |
| `make wasm-examples-dev` | Build WASM and launch Vite dev server |

## Build Profiles

| Profile | Use Case | Command |
|---|---|---|
| `dev` | Fast compilation, full debug info | `cargo build` |
| `release` | Speed-optimized (thin LTO, 16 CGUs) | `cargo build --release` |
| `release-size` | Size-optimized for WASM (`opt-level = "z"`, full LTO) | `cargo build --profile release-size` |
| `release-perf` | Max speed (thin LTO, 8 CGUs) | `cargo build --profile release-perf` |
| `bench` | Benchmarking with profiling symbols | `cargo bench` |

## Python Development

```bash
# Install uv
curl -LsSf https://astral.sh/uv/install.sh | sh

# One-command setup (creates venv, installs deps, builds bindings)
make python-dev

# Or manual setup
uv venv && source .venv/bin/activate
uv pip install -e ".[dev]"
cd finstack-py && maturin develop --release

# Run tests
uv run pytest

# Format and lint
uv run ruff format .
uv run ruff check .
```

### Python Package

The `finstack` Python package includes:

- Full `.pyi` type stubs and `py.typed` marker
- Pydantic v2 integration
- Polars DataFrame exports
- Example scripts and Jupyter notebooks in `finstack-py/examples/`

## WASM Development

```bash
# Install wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build for web and Node.js
make wasm-pkg

# Run the React example app with live reload
make wasm-examples-dev
```

The WASM package ships with auto-generated TypeScript definitions.

## Feature Flags

| Feature | Default | Description | Size |
|---|---|---|---|
| `serde` | Yes | Serialization (JSON, CBOR, MessagePack). Required for bindings. | +150 KB |
| `parallel` | Yes | Multi-threaded computation via Rayon. 2-10x speedup on multi-core. | +200 KB |
| `dataframes` | No | Polars DataFrame exports, CSV/Parquet/Arrow interop. | +2-3 MB |
| `stochastic` | No | Monte Carlo and stochastic models (reserved, planned for 0.5.x). | +100 KB |

```toml
# Default (serde + parallel)
finstack = "0.4"

# With DataFrames
finstack = { version = "0.4", features = ["dataframes"] }

# Minimal build
finstack = { version = "0.4", default-features = false, features = ["serde"] }
```

## CI/CD

### Build Workflow (on push/PR to `master`)

- **Lint** -- pre-commit hooks (rustfmt, clippy, ruff, prettier, markdownlint)
- **Test Rust** -- cargo-nextest with `mc` and `test-utils` features
- **Test Python** -- uv + maturin develop + pytest
- **Test WASM** -- wasm-pack test
- **Supply chain** -- cargo-deny
- **Semver checks** -- cargo-semver-checks on `finstack-core` (PRs only)

### Release Workflow (on version tag `v*`)

- Builds Python wheels for Linux (x64/arm64), macOS (arm64), Windows (x64) targeting Python 3.12, 3.13, 3.14
- Builds WASM packages (web + Node.js targets)
- Publishes artifacts to GitHub Releases

## Code Quality

The workspace enforces strict Clippy lints including:

- **No panics** -- `unwrap`, `expect`, `panic!`, and `unreachable!` are denied in library code
- **Numerical soundness** -- lossy float literals denied, float comparisons warned
- **Modern Rust** -- manual pattern lints enforced (`let-else`, `ok_or`, etc.)

## Code Coverage

```bash
make coverage           # Quick summary
make coverage-html      # Detailed HTML report in target/llvm-cov/
make coverage-lcov      # LCOV report for CI integration
```

Coverage analysis runs on core Rust crates only (Python and WASM bindings are excluded).

## Packaging & Distribution

```bash
make wheel-all          # Build wheels for all local Python versions
make wheel-local        # Build wheel for current platform
make wheel-docker       # Build manylinux wheel via Docker
make wasm-pkg           # Build WASM packages (web + node)
```

## License

MIT OR Apache-2.0

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
