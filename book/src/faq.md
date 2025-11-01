# Frequently Asked Questions

## General

### Why Decimal instead of float?

Financial calculations require exact precision. Floating-point arithmetic introduces rounding errors that compound over many operations:

```rust
// Floating point error
let x = 0.1f64 + 0.2f64;
assert_ne!(x, 0.3f64);  // ❌ Not equal!

// Decimal is exact
let x = Decimal::from_str("0.1")? + Decimal::from_str("0.2")?;
assert_eq!(x, Decimal::from_str("0.3")?);  // ✅ Equal!
```

### Why enforce currency safety?

Mixing currencies without explicit conversion is a common source of errors in financial systems. By enforcing currency safety at compile time, Finstack prevents:

- Accidentally adding USD and EUR amounts
- Forgetting to apply FX conversions
- Silent currency mismatch bugs

### What is determinism and why does it matter?

Determinism means identical inputs always produce identical outputs, regardless of:
- Serial vs parallel execution
- Platform or architecture
- Timing or thread scheduling

This is critical for:
- Regulatory compliance
- Reproducible research
- Auditable calculations
- Golden test suites

## Performance

### Is Decimal slower than float?

Yes, `Decimal` arithmetic is slower than `f64` operations. However:

1. Correctness matters more than raw speed for financial calculations
2. Finstack optimizes hot paths (vectorization, caching)
3. Most pricing operations are I/O or algorithm-bound, not arithmetic-bound

For performance-critical paths, Finstack provides:
- Vectorized operations via Polars
- Optional parallel execution
- Efficient caching

### Can I use parallelism?

Yes! Finstack supports optional parallel execution via Rayon. Importantly:

- Parallel results are **identical** to serial results (in Decimal mode)
- Feature flag: `features = ["parallel"]`
- Heavy operations release the Python GIL

### How fast is Finstack compared to X?

Finstack prioritizes **correctness and determinism** over raw speed. That said:

- Competitive with QuantLib for single-instrument pricing
- Faster than pandas-based Python solutions (due to Rust core)
- Polars integration provides excellent DataFrame performance

## Python Bindings

### Do I need to know Rust?

No! The Python bindings provide a complete, idiomatic Python API. You can use Finstack without ever writing Rust code.

### Are Python wheels available?

Yes, pre-built wheels are available for:
- macOS (Intel and Apple Silicon)
- Linux (x86_64)
- Windows (x86_64)

Install via pip:
```bash
pip install finstack
```

### Can I use Finstack in Jupyter notebooks?

Yes! Finstack works great in Jupyter. The Python bindings include Pydantic models that display nicely in notebooks.

### Does Finstack work with pandas?

Finstack uses **Polars** for DataFrame operations, not pandas. However, you can easily convert:

```python
# Polars → pandas
df_pandas = polars_df.to_pandas()

# pandas → Polars
polars_df = pl.from_pandas(df_pandas)
```

## WebAssembly

### Can I use Finstack in the browser?

Yes! The WASM bindings work in modern browsers and Node.js environments.

### What's the bundle size?

The minified WASM bundle is approximately:
- Core: ~200 KB (gzipped)
- Full library: ~800 KB (gzipped)

Use feature flags to reduce bundle size by including only what you need.

### Does WASM have feature parity with Rust?

Nearly complete parity for core functionality. Some advanced features (I/O, external providers) may have limited support in WASM due to platform constraints.

## Data & Integration

### Can Finstack read Bloomberg data?

Yes, via the `io` crate's Bloomberg provider integration (requires Bloomberg API credentials).

### Does Finstack support databases?

Yes, the `io` crate includes:
- SQLite support (embedded)
- PostgreSQL support (via ORM)
- Migrations for schema management

### Can I export to Excel?

Yes, Finstack can export to:
- CSV (builtin)
- Parquet (builtin)
- Excel (via Polars `write_excel`)

## Pricing & Instruments

### What instruments are supported?

Current support includes:

**Fixed Income:**
- Bonds (fixed, floating, zero-coupon)
- Interest rate swaps
- Caps, floors, swaptions

**Credit:**
- CDS, CDS indices

**Equity Derivatives:**
- Vanilla options
- Barrier options, Asian options
- Autocallables

**Structured Products:**
- ABS, RMBS, CMBS, CLO

**Private Markets:**
- Private equity funds
- Real estate investments

See the [Valuations](./valuations/overview.md) section for details.

### How do I price custom instruments?

Implement the `Pricer` trait:

```rust
impl Pricer for MyCustomPricer {
    fn price(&self, ctx: &MarketContext) -> Result<ValuationResult> {
        // Your pricing logic
    }
}
```

See [Custom Instruments](./valuations/custom-instruments.md) for a complete guide.

### Does Finstack support Monte Carlo?

Yes! Monte Carlo features are available with the `mc` feature flag:

```toml
finstack = { version = "0.1", features = ["mc"] }
```

Includes GBM and Heston stochastic processes.

## Development

### How do I contribute?

See the [Contributing Guide](./developer/contributing.md) for details.

### What's the testing strategy?

Finstack uses multiple test layers:
- Unit tests
- Property-based tests (via proptest)
- Golden tests (snapshot testing)
- Parity tests (Rust ↔ Python ↔ WASM)

### How do I report a bug?

Open an issue on GitHub with:
- Minimal reproducible example
- Expected vs actual behavior
- Version information (`cargo --version`, `python --version`)

### Where can I get help?

- **GitHub Discussions**: Ask questions, share ideas
- **GitHub Issues**: Report bugs, request features
- **Documentation**: Start here!

---

*Have a question not listed here? Open a discussion on GitHub!*
