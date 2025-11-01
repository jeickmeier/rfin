# Portfolio Overview

The portfolio crate provides entity-based position tracking and aggregation.

## Key Features

- **Multi-instrument positions** across books and entities
- **Currency-safe aggregation** with explicit FX policies
- **Scenario integration** for stress testing
- **DataFrame exports** for analysis

## Quick Example

Here's a complete working example from the codebase:

```rust
{{#include ../../../finstack/examples/portfolio/portfolio_example.rs}}
```

Run this example yourself:

```bash
cargo run --example portfolio_example
```

## Core Concepts

See the following sections for details:
- [Entities & Positions](./entities-positions.md)
- [Aggregation](./aggregation.md)
- [FX Policies](./fx-policies.md)
- [DataFrame Exports](./dataframes.md)

## API Reference

For detailed API documentation, run:

```bash
make doc
```

Then navigate to `finstack::portfolio` in the generated documentation.
