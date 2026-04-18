# Installation

## Python

Python 3.12+ is required. Install from PyPI or build from source:

```bash
# Using uv (recommended)
uv pip install finstack

# Or with pip
pip install finstack
```

### Build from Source

Building from source requires the Rust toolchain (1.82+):

```bash
git clone https://github.com/your-org/finstack.git
cd finstack

# Development build (debug, fast compile)
make python-dev-debug

# Release build (optimized, slower compile, required for portfolio valuation)
make python-dev
```

The release profile is recommended for anything beyond simple instrument
pricing — debug builds are too slow for portfolio-level computation.

### Verify Installation

```python
import finstack
print(finstack.__version__)
```

## Rust

Add Finstack to your `Cargo.toml`:

```toml
[dependencies]
finstack = { version = "0.1" }
```

The umbrella crate re-exports every sub-crate (core, analytics, margin,
valuations, portfolio, statements, statements_analytics, scenarios,
monte_carlo, correlation) unconditionally.

Two pass-through features are available to opt in to heavier compile-time
costs:

| Feature | Effect |
|---------|--------|
| `mc` | Enables Monte Carlo pricers across `monte_carlo`, `margin`, `valuations` (pulls in `nalgebra`). |
| `dataframes` | Enables the `polars`-based DataFrame surfaces in `statements` and `portfolio`. |

```toml
[dependencies]
finstack = { version = "0.1", features = ["mc", "dataframes"] }
```

## WASM / TypeScript

```bash
npm install @finstack/wasm
```

> **Note:** WASM bindings are under active development. See the
> [WASM Bindings architecture page](../architecture/binding-layer/wasm-bindings.md)
> for current status.

## Development Setup

For contributors building the full workspace:

```bash
# Prerequisites
# - Rust 1.82+ (via rustup)
# - Python 3.12+ (via pyenv or system)
# - uv (Python package manager)
# - Node.js 18+ (for WASM)

# Clone and set up
git clone https://github.com/your-org/finstack.git
cd finstack

# Install Python dev dependencies
uv sync --group dev

# Build Python bindings (release)
make python-dev

# Run full test suite
make test

# Format and lint
make fmt
make lint
```
