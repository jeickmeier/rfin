---
description: Repository Information Overview
alwaysApply: true
---

# Finstack Repository Information

## Summary

Finstack is a deterministic financial computation engine written in Rust with first-class bindings for Python and WebAssembly. The repository is a multi-project workspace emphasizing accounting-grade correctness (Decimal numerics), currency-safety, stable wire formats, and performance for financial modeling, valuations, scenarios, and portfolio analysis.

## Repository Structure

```
.
├── finstack/                 # Rust workspace meta-crate and examples
│   ├── core/                 # Primitives: types, money/fx, time, expressions
│   ├── statements/           # Financial statement modeling and evaluation
│   ├── valuations/           # Instrument pricing, cashflows, risk metrics
│   ├── scenarios/            # Deterministic DSL for scenario analysis
│   ├── portfolio/            # Portfolio aggregation and position management
│   ├── io/                   # CSV/Parquet/Arrow interoperability
│   └── valuations/macros/    # Procedural macros for valuations crate
├── finstack-py/             # Python bindings (Maturin + PyO3)
├── finstack-wasm/           # WebAssembly bindings (wasm-pack)
├── packages/finstack-ui/    # React/TypeScript UI component library
├── Cargo.toml               # Rust workspace manifest
├── pyproject.toml           # Python project configuration
├── Makefile                 # Build orchestration
└── book/                    # mdBook documentation
```

### Main Repository Components

- **finstack (meta-crate)**: Re-exports finstack subcrates via feature flags (core, statements, valuations, scenarios, portfolio, io).
- **finstack-py**: Python bindings exposing financial computation APIs via PyO3; Pydantic v2 models mirror serde shapes.
- **finstack-wasm**: WebAssembly bindings for browser/Node.js; JSON IO parity with serde.
- **packages/finstack-ui**: React component library (Radix UI, Recharts, TailwindCSS) with WASM worker bootstrap; private package.

## Projects

### Finstack Core (Rust Workspace)

**Workspace File**: `Cargo.toml`

#### Language & Runtime

**Language**: Rust
**Version**: 1.90+ (MSRV)
**Edition**: 2021
**Build System**: Cargo (workspace resolver v2)
**Profiles**:
- `dev`: Fast compilation, full debug info
- `test`: Reduced debug, incremental
- `release`: Optimized for speed (thin LTO, 3x optimization)
- `release-size`: Optimized for size (WASM deployments)
- `bench`: Profiling-enabled optimized builds

#### Dependencies

**Key Dependencies**:
- `rust_decimal` (1.39): Accounting-grade decimal arithmetic
- `polars` (re-exported from core): DataFrames for time-series analysis
- `time` (0.3): Date/time handling with ISO-8601 support
- `serde`/`serde_json` (1.0): Serialization with strict serde names
- `rayon` (1.11): Parallel iteration (optional, enabled by default)
- `thiserror` (2.0): Error handling
- `statrs` (0.18): Statistical distributions

**Build Dependencies**:
- `strum`/`strum_macros` (0.27): Enum introspection

**Features**:
- `core` (default): Core types and utilities
- `statements`: Financial statement modeling
- `valuations`: Pricing and risk analytics
- `scenarios`: Scenario DSL and composition
- `portfolio`: Position and book aggregation
- `io`: CSV/Parquet/Arrow interop
- `parallel`: Rayon-based parallelism
- `dataframes`: Polars DataFrame exports

#### Build & Installation

```bash
# Build all crates
make build

# Build optimized without debug info
make build-prod

# Run Rust examples (categorized)
make examples

# Build individual crates
cargo build -p finstack-core
cargo build -p finstack-statements
cargo build -p finstack-valuations

# With features
cargo build --all-features
```

#### Testing

**Framework**: cargo nextest (parallel test runner), criterion (benchmarks)
**Test Location**: `finstack/*/tests/` and `**/src/lib.rs` (#[test] modules)
**Naming Convention**: `*_tests.rs` (integration tests), `test_` functions
**Configuration**: Workspace-level lints deny panics, unwrap, expect; enforce numerical safety

**Run Commands**:

```bash
# Fast unit and integration tests
make test-rust

# Include slow tests (tagged with #[ignore])
make test-rust-slow

# Documentation tests
make test-rust-doc

# Benchmarks (criterion)
make bench-perf

# Flamegraph profiling (MC pricing)
make bench-flamegraph

# Coverage
make coverage-html
```

#### Linting

```bash
# Clippy with strict rules (panics, unwrap, float comparisons)
make lint-rust

# Auto-fix
make lint-rust-fix
```

---

### Python Bindings (finstack-py)

**Configuration File**: `pyproject.toml`

#### Language & Runtime

**Language**: Python
**Version**: 3.12+
**Build System**: Maturin (PyO3 bridge)
**Package Manager**: uv (fast Python package management)

#### Dependencies

**Main Dependencies**:
- `numpy` (2.3.5+): Numeric arrays
- `pandas` (2.3.3+): Data manipulation
- `polars` (1.35.2+): DataFrames (primary for time-series)
- `pydantic` (2.12.5+): Data validation with v2 models
- `pyarrow` (22.0.0+): Arrow/Parquet interop
- `matplotlib` (3.10.7+): Visualization

**Development Dependencies**:
- pytest, pytest-benchmark: Testing and performance
- mypy: Type checking
- ruff: Linting/formatting
- flake8, flake8-cognitive-complexity: Code quality

#### Build & Installation

```bash
# Setup Python environment
make setup-python
source .venv/bin/activate

# Build Python bindings (release-perf profile)
make python-dev

# Development install with maturin
cd finstack-py && maturin develop --profile release-perf

# Rebuild after Rust changes
make python-dev
```

#### Testing

**Framework**: pytest with Pydantic v2 model validation
**Test Location**: `finstack-py/tests/`
**Naming Convention**: `test_*.py`, `*_test.py`
**Configuration**: pytest.ini options in pyproject.toml
**Markers**: `perf`, `security`, `integration`, `slow`

**Run Command**:

```bash
make test-python
```

---

### WebAssembly Bindings (finstack-wasm)

**Configuration File**: `finstack-wasm/package.json`

#### Language & Runtime

**Language**: TypeScript/JavaScript (bindings), Rust (core)
**Version**: wasm-pack (latest)
**Build System**: wasm-pack (web/nodejs targets)
**Package Manager**: npm
**Node.js**: 18+

#### Dependencies

**Main**:
- Generated WASM bindings via wasm-bindgen
- `comlink` (4.4.2): Worker communication

**Development**:
- TypeScript (5.9.3)
- ESLint + Prettier: Code quality
- No test framework in package.json (tests via wasm-pack test)

#### Build & Installation

```bash
# Build for web
make wasm-build
cd finstack-wasm && wasm-pack build --target web

# Build for Node.js
wasm-pack build --target nodejs

# Size-optimized build
wasm-pack build --target web --profile release-size

# Run examples
make wasm-examples-dev
```

#### Testing

**Framework**: wasm-pack test (headless Chrome via wasm-bindgen-test)
**Configuration**: Tests in Rust crate with `#[wasm_bindgen_test]`
**Run Command**:

```bash
make test-wasm
```

---

### React UI Library (packages/finstack-ui)

**Configuration File**: `packages/finstack-ui/package.json`

#### Language & Runtime

**Language**: TypeScript/React
**Version**: React 19.0.0+, Node.js 18+
**Build System**: Vite
**Package Manager**: npm
**Type**: Private monorepo package (`"private": true`)

#### Dependencies

**Main**:
- React 19.0.0, react-dom: UI framework
- Radix UI: Accessible primitives (label, select, slot)
- TanStack: Table and virtual scrolling
- Tailwind CSS: Utility CSS with animate plugin
- Recharts (2.14.1): Data visualization
- Zod (3.23.8): Schema validation
- Zustand (5.0.0-beta): State management
- comlink (4.4.2): WASM worker communication

**Development**:
- Vite (5.4.10): Build tool
- Vitest (2.1.4): Test runner with coverage
- ESLint + TypeScript: Code quality
- Prettier: Code formatting

#### Build & Installation

```bash
# Build UI library
cd packages/finstack-ui && npm run build

# Development server
npm run dev

# Type definitions export
vite build --emptyOutDir
```

#### Testing

**Framework**: Vitest
**Test Location**: Tests colocated with source (TBD)
**Run Commands**:

```bash
# Run tests
make test-ui

# Coverage
make test-ui-coverage
```

---

## Shared Build & Lint Commands

```bash
# Format all code
make fmt-rust
make fmt-python
make fmt-wasm
make fmt-ui

# Lint all code
make lint
  # or per-language: lint-rust, lint-python, lint-wasm, lint-ui

# Full test code
make test
  # or per-language: test-rust, test-python, test-wasm, test-ui

# Clean build artifacts
make clean

# Generate documentation
make doc              # Rust rustdoc
make book-serve       # mdBook with live reload
```

## Key Technical Decisions

- **Decimal Default**: All financial computations use `rust_decimal` for exactness.
- **Currency Safety**: Cross-currency arithmetic requires explicit FX conversions; policies stamped in results.
- **Serde Stability**: Strict field names, `unknown_fields = "deny"` for inbound types.
- **Parallelism**: Opt-in via `parallel` feature; serial mode ensures reproducibility.
- **DataFrame Standard**: Polars re-exported from core as canonical time-series surface.
- **No Unsafe**: Workspace deny unsafe in lints.
