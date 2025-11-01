# Introduction

Welcome to the **Finstack** documentation!

## What is Finstack?

Finstack is a deterministic, cross-platform financial computation engine built in Rust with first-class Python and WebAssembly bindings. It provides accounting-grade correctness, currency-safety, and predictable performance for:

- **Financial Statement Modeling** - Declarative statement graphs with deterministic evaluation
- **Instrument Valuations** - Pricing, risk metrics, and analytics across asset classes
- **Scenario Analysis** - Deterministic stress testing and what-if analysis
- **Portfolio Analytics** - Multi-instrument aggregation with explicit FX policies

## Key Features

### 🎯 Determinism

All computations use `Decimal` numerics by default, ensuring that serial and parallel runs produce identical results. This is critical for:
- Regulatory compliance
- Audit trails
- Reproducible research
- Golden test suites

### 💰 Currency Safety

Finstack enforces currency safety at the type level:
- No implicit cross-currency arithmetic
- Explicit FX conversions via `FxProvider` interfaces
- All conversion policies are stamped in results metadata
- Full traceability of currency transformations

### 📊 Stable Schemas

All data structures use strict serde field names, making them suitable for:
- Long-lived data pipelines
- Golden tests and regression suites
- Cross-language serialization (Rust ↔ Python ↔ WASM)
- API versioning

### ⚡ Performance

While prioritizing correctness, Finstack achieves excellent performance through:
- Vectorized execution via Polars
- Optional Rayon parallelism (preserving Decimal determinism)
- Efficient caching strategies
- Zero-copy interop where possible

## Architecture Overview

Finstack is organized as a Rust workspace with multiple specialized crates:

```
finstack/
├── core/          → Currency, dates, math, market data, expressions
├── statements/    → Financial statement modeling & evaluation
├── valuations/    → Instrument pricing, risk, and analytics
├── scenarios/     → Deterministic scenario DSL
├── portfolio/     → Position tracking & aggregation
└── io/            → Data I/O (CSV, Parquet, databases)

Bindings:
├── finstack-py/   → Python bindings (PyO3 + Pydantic)
└── finstack-wasm/ → WebAssembly bindings
```

## Philosophy

1. **Correctness first** - Decimal by default, strict validation
2. **Performance second** - Optimize without changing Decimal results
3. **Ergonomic APIs** - Easy things should be easy, hard things possible
4. **Documentation** - Every public API documented with examples
5. **Testing** - Unit, property, golden, and parity tests

## Documentation Structure

Finstack has two types of documentation:

- **📖 This Book** - Guides, tutorials, and conceptual explanations
- **📚 API Reference** - Generated from code docstrings ([run `make doc`](about:blank))

## Getting Started

Ready to dive in? Check out the [Installation](./getting-started/installation.md) guide and [Quick Start](./getting-started/quick-start.md) tutorial.

For specific use cases, jump to:
- [Statements](./statements/overview.md) - Financial modeling
- [Valuations](./valuations/overview.md) - Instrument pricing
- [Scenarios](./scenarios/overview.md) - Stress testing
- [Portfolio](./portfolio/overview.md) - Multi-instrument analytics

## Community & Support

- **GitHub**: [github.com/yourusername/finstack](https://github.com/yourusername/finstack)
- **Issues**: Report bugs and request features
- **Discussions**: Ask questions and share ideas
- **Contributing**: See the [Developer Guide](./developer/contributing.md)

---

*Built with ❤️ using Rust*

