# Finstack (Rust)

A high-performance financial computation library written in Rust with bindings for Python and WebAssembly.

## Project Structure

- `core/` - Core Rust library with financial functionality (crate name `finstack-core`)
- `finstack-py/` - Python bindings using PyO3 (crate name `finstack-py`)
- `finstack-wasm/` - WebAssembly bindings using wasm-bindgen (crate name `finstack-wasm`)
- `finstack/` - Meta-crate re-exporting subcrates via features
- `examples/` - Example usage for different bindings
- `docs/` - Technical documentation and design documents

## Development Setup

### Prerequisites

- Rust 1.78+ (install via [rustup](https://rustup.rs/))
- Python 3.8+ (for Python bindings)
- Node.js (for WASM development)
- [uv](https://github.com/astral-sh/uv) (for Python package management)

### Quick Start

1. Clone the repository:
```bash
git clone https://github.com/rustfin/rfin.git
cd rfin
```

2. Build the core library:
```bash
cargo build
```

3. Run tests:
```bash
cargo test
```

## Python Development with uv

We use `uv` for fast Python package management and virtual environment handling.

### Install uv

```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
```

### Setup Python Environment

```bash
# Option 1: Run the setup script (recommended)
./scripts/setup-python.sh

# Option 2: Manual setup
uv venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate
uv pip install maturin pytest pytest-benchmark black mypy ruff ipython jupyter
cd rfin-python && python -m maturin develop --release

# Option 3: Using Make
make python-dev
```

### Run Python Example

```bash
# With activated venv
python examples/python_example.py

# Or directly with uv
uv run python examples/python_example.py
```

### Python Development Workflow

```bash
# Install development dependencies
uv pip install -e ".[dev]"

# Run tests
uv run pytest

# Format code
uv run black .
uv run ruff check .

# Type checking
uv run mypy .
```

## WASM Development

### Build WASM Package

```bash
# Install wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build for web
cd finstack-wasm
wasm-pack build --target web

# Build for Node.js
wasm-pack build --target nodejs
```

### Run WASM Example

1. Build the WASM package (see above)
2. Serve the example with a local web server:
```bash
python -m http.server 8000
# Or use any other static file server
```
3. Open http://localhost:8000/examples/wasm/primitives_wasm_example.html

## Features

- **Core Library** (`finstack-core`):
  - `std` - Standard library support (always on)
  - `decimal128` - High-precision decimal support
  - `serde` - Serialization support
  - `holidays` - Holiday calendar functionality

- **Python Bindings** (`finstack-py`):
  - Inherits features from core
  - Provides Pythonic API

- **WASM Bindings** (`finstack-wasm`):
  - Optimized for web browsers
  - Small bundle size
  - TypeScript definitions

## CI/CD

The project uses GitHub Actions for continuous integration:

- Code formatting (`cargo fmt`)
- Linting (`cargo clippy`)
- Testing (`cargo test`)
- Testing across multiple platforms and Rust versions
- Python bindings testing (Python 3.8, 3.11, 3.12)
- WASM build verification

## Code Coverage

The project includes comprehensive code coverage tools using `cargo-llvm-cov`:

```bash
# Quick coverage summary
make coverage

# Generate detailed HTML report
make coverage-html

# Generate LCOV report for CI
make coverage-lcov
```

Coverage reports are generated in `target/llvm-cov/` and provide detailed insights into test coverage across the core Rust crates. The Python and WASM bindings are intentionally excluded from coverage analysis as they don't contain Rust business logic.

For more details, see [docs/COVERAGE.md](docs/COVERAGE.md).

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.