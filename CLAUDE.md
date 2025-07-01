# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

RFin is a high-performance financial computation library written in Rust with bindings for Python and WebAssembly. The project uses a workspace structure with three main crates:

- `core/` - Core Rust library (`rfin-core`) with financial primitives and date handling, designed to work in no_std environments
- `rfin-python/` - Python bindings using PyO3 and maturin
- `rfin-wasm/` - WebAssembly bindings using wasm-bindgen

## Essential Commands

### Building and Testing
```bash
# Build all crates
cargo build --workspace
make build

# Run all tests  
cargo test --workspace
make test

# Format code
cargo fmt --all
make fmt

# Lint code
cargo clippy --workspace --all-targets --all-features -- -D warnings
make lint
```

### Python Development
```bash
# Setup Python environment (creates .venv with uv)
make setup-python
source .venv/bin/activate
make python-dev

# Run Python tests and formatting
uv run pytest
uv run black .
uv run ruff check .
uv run mypy .
```

### WASM Development
```bash
# Build WASM package
make wasm-build
# Or manually:
cd rfin-wasm && wasm-pack build --target web
```

## Architecture

### Core Library (`rfin-core`)
- **no_std by default** - Enable `std` feature when needed
- **Feature flags**: `std`, `decimal128`, `holidays`, `serde`
- **Main modules**: `dates` (calendar, daycount, schedule), `primitives` (currency, money)
- **Data**: ISO 4217 currency data in `core/data/iso_4217.csv`

### Module Structure
- `dates/` - Date/time handling with calendar, daycount conventions, schedules
- `primitives/` - Financial primitives like currency and money types
- `macros.rs` - Internal macros for code generation

### Python Bindings
- Uses PyO3 for Rust-Python interop
- Maturin for building Python wheels
- Development workflow: `maturin develop --release` in `rfin-python/`

### WASM Bindings  
- Uses wasm-bindgen for JavaScript interop
- Optimized for small bundle size
- TypeScript definitions included

## Development Workflow

1. The project requires Rust 1.78+ and uses uv for Python package management
2. All code must pass `cargo fmt`, `cargo clippy`, and tests before commits
3. Python code uses black, ruff, and mypy for formatting and linting
4. The library is designed for financial computations with emphasis on performance and correctness