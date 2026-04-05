# Finstack

Finstack is a high-performance financial computation library written in Rust with
first-class Python and WASM bindings. It provides pricing, risk analytics,
portfolio management, and financial statement modeling across rates, credit,
equity, FX, commodity, and structured products.

## At a Glance

- **71 instrument types** — bonds, swaps, CDS, options, exotics, structured credit, and more
- **11 Rust crates** — composable via feature flags; compile only what you need
- **243 Python API classes** — PEP 561 typed stubs, zero-copy where possible
- **11 Monte Carlo processes** — GBM, Heston, Hull-White, Rough Bergomi, LMM, and more
- **ISDA SIMM v2.5/v2.6** — initial margin calculation across all risk classes

## What You'll Find Here

| Section | Description |
|---------|-------------|
| [Getting Started](getting-started/README.md) | Install Finstack and run your first pricing in under 5 minutes |
| [Architecture](architecture/README.md) | Crate structure, design philosophy, and how the pieces fit together |
| [Cookbooks](cookbooks/README.md) | Step-by-step recipes: curve building, bond pricing, portfolio valuation |
| [Extending Finstack](extending/README.md) | Add new instruments, pricers, metrics, or bindings |
| [Conventions](conventions/README.md) | Naming, error handling, testing, and documentation standards |
| [Reference](reference/README.md) | Crate index, metric key catalog, market conventions |
| [Notebooks](notebooks/README.md) | Interactive Jupyter notebooks covering every major feature |

## API References

- [Rust API (rustdoc)](../api/rust/finstack/) — auto-generated from doc comments
- [Python API (mkdocs)](../api/python/) — auto-generated from `.pyi` type stubs

## Design Principles

- **Rust-first** — all business logic lives in Rust crates; bindings are thin
  wrappers for type conversion and ergonomic helpers.
- **Deterministic** — `Decimal` by default, `FxHashMap` for reproducible
  iteration, identical results in serial and parallel execution.
- **Currency-safe** — `Money` types enforce currency matching; cross-currency
  operations require explicit FX policies.
- **Composable** — feature-gated crates can be combined independently; you only
  pay for what you use.
- **No unsafe in bindings** — `#![forbid(unsafe_code)]` in binding crates;
  `#![deny(clippy::unwrap_used)]` everywhere.
