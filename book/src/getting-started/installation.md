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
finstack = { version = "0.1", features = ["all"] }
```

Or pick individual feature flags to compile only what you need:

| Feature | What It Includes |
|---------|-----------------|
| `core` | Currency, money, dates, calendars, market data, math |
| `analytics` | Expression engine, computed metrics |
| `margin` | ISDA SIMM, variation margin |
| `valuations` | Instruments, pricing, calibration, risk (includes cashflows, correlation, monte-carlo) |
| `portfolio` | Portfolio valuation, grouping |
| `statements` | Financial statement modeling, waterfalls, covenants |
| `scenarios` | Scenario engine, stress testing |
| `all` | Everything above |

### Minimum Example

```toml
[dependencies]
finstack = { version = "0.1", features = ["core", "valuations"] }
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
