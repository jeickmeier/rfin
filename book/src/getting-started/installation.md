# Installation

This guide covers installation of Finstack for Rust, Python, and WebAssembly.

## Prerequisites

- **Rust**: 1.75 or later (see [rust-lang.org](https://www.rust-lang.org/))
- **Python**: 3.9+ (for Python bindings)
- **Node.js**: 18+ (for WebAssembly examples)

## Rust

Add Finstack to your `Cargo.toml`:

```toml
[dependencies]
finstack = { version = "0.1", features = ["full"] }
```

### Feature Flags

Finstack uses feature flags to control which components are compiled:

- `full` - All features enabled
- `core` - Core primitives only
- `statements` - Financial statement modeling
- `valuations` - Instrument pricing and risk
- `scenarios` - Scenario analysis
- `portfolio` - Portfolio analytics
- `io` - Data I/O (CSV, Parquet, databases)
- `mc` - Monte Carlo simulation support

Example for a minimal setup:

```toml
[dependencies]
finstack = { version = "0.1", features = ["core", "valuations"] }
```

## Python

### Using pip

```bash
pip install finstack
```

### Using uv (recommended)

```bash
uv pip install finstack
```

### From source

```bash
git clone https://github.com/yourusername/finstack.git
cd finstack/finstack-py
uv venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate
maturin develop --release
```

## WebAssembly

### NPM

```bash
npm install finstack-wasm
```

### Yarn

```bash
yarn add finstack-wasm
```

### From source

```bash
git clone https://github.com/yourusername/finstack.git
cd finstack/finstack-wasm
wasm-pack build --target web
```

## Verification

### Rust

Create a simple test program:

```rust
use finstack::prelude::*;

fn main() -> Result<()> {
    let amount = Amount::from_str("100.00 USD")?;
    println!("Amount: {}", amount);
    Ok(())
}
```

Run with:

```bash
cargo run
```

### Python

Test your installation:

```python
from finstack import Amount

amount = Amount.from_str("100.00 USD")
print(f"Amount: {amount}")
```

### WebAssembly

```typescript
import init, { Amount } from 'finstack-wasm';

await init();
const amount = Amount.from_str("100.00 USD");
console.log(`Amount: ${amount}`);
```

## Next Steps

Continue to the [Quick Start](./quick-start.md) guide to start using Finstack.
