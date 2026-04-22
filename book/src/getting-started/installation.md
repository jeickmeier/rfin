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

# Development build (dev profile, fast compile)
mise run python-build

# Release build (optimized, slower compile; use for portfolio-scale work)
mise run python-build -- --release
```

The release profile is recommended for heavy portfolio-level computation;
the dev profile is preferred for day-to-day development and faster rebuilds.

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

One pass-through feature is available to opt in to heavier compile-time cost:

| Feature | Effect |
|---------|--------|
| `mc` | Enables Monte Carlo pricers across `monte_carlo`, `margin`, `valuations` (pulls in `nalgebra`). |

```toml
[dependencies]
finstack = { version = "0.1", features = ["mc"] }
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

# Build Python bindings (dev profile; use `mise run python-build -- --release` for optimized extension)
mise run python-build

# Run full test suite
mise run all-test

# Format and lint
mise run all-fmt
mise run all-lint
```
