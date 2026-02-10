# Error Type Naming Conventions

This document describes the intentional naming conventions for error types
across the Finstack crate hierarchy and the rationale behind them.

## Summary

Two valid patterns coexist. Both are intentional.

| Pattern | When to use | Example |
|---------|-------------|---------|
| **Plain `Error`** | Crate-level root error that is the *only* error callers interact with | `finstack_core::Error`, `finstack_scenarios::Error` |
| **`{Domain}Error`** | Domain-specific sub-errors, or root errors in crates whose name alone is ambiguous | `InputError`, `PricingError` |

## Per-Crate Inventory

| Crate | Root Error | Sub-Errors | Re-export |
|-------|-----------|------------|-----------|
| `finstack_core` | `Error` | `InputError` | `finstack_core::Error`, `finstack_core::InputError` |
| `finstack_valuations` | `error::Error` | `PricingError`, `CorrelationMatrixError`, `ValidationError` | `finstack_valuations::Error` |
| `finstack_portfolio` | `error::Error` | — | `finstack_portfolio::Error` |
| `finstack_scenarios` | `error::Error` | — | `finstack_scenarios::Error` |
| `finstack_statements` | `error::Error` | — | `finstack_statements::Error` |

## Design Rationale

### Why `finstack_core` uses plain `Error`

`finstack_core` is the foundational crate that every other crate depends on.
Its `Error` type is re-exported at crate root (`finstack_core::Error`) and
serves as the common currency for error propagation. The unqualified name is
idiomatic for a crate's primary error type (see `std::io::Error`,
`serde_json::Error`). The sub-error `InputError` uses a domain prefix because
it is *nested inside* `Error::Input(InputError)` and callers frequently import
both at the same time.

### Why `finstack_valuations` sub-errors use `{Domain}Error`

The valuations crate wraps multiple domain-specific error types:

```text
valuations::Error
├── Pricing(PricingError)
├── Correlation(CorrelationMatrixError)
└── WaterfallValidation(ValidationError)
```

Each sub-error has distinct match patterns and may be imported alongside
`finstack_core::Error`. The `{Domain}Error` prefix prevents name collisions
and makes imports self-documenting:

```rust
use finstack_core::Error;                    // core error
use finstack_valuations::PricingError;       // valuations pricing sub-error
```

The unified wrapper `error::Error` is re-exported at crate root as `Error`,
matching the standard convention. Consumers disambiguate via module path
(`finstack_valuations::Error` vs `finstack_core::Error`) just like every
other crate.

### Why `finstack_portfolio` uses `Error`

The portfolio crate follows the standard root error naming convention:
`finstack_portfolio::Error`. Callers disambiguate via module paths
(`finstack_portfolio::Error` vs `finstack_core::Error`) or by using an alias
in import blocks when needed.

### Why `finstack_scenarios` / `finstack_statements` use plain `Error`

These crates follow the same pattern as `finstack_core`: a single root error
enum living in `error::Error` with a `Result<T>` alias. Because they are
typically used in isolation, name collisions are rare. If a crate needs to be
imported alongside core, callers can qualify via module path
(`finstack_scenarios::Error` vs `finstack_core::Error`) or use an alias in
import blocks when needed.

## Guidelines for New Crates

1. **Default to plain `Error`** for the root error enum, following Rust
   ecosystem convention (`thiserror`, `anyhow`, `std::io`).

2. **Use `{Domain}Error`** when:
   - The error is a *sub-error* nested inside a parent `Error` enum.
   - The crate's error is commonly imported alongside `finstack_core::Error`.
   - Multiple error types from different crates appear in the same scope.

3. **Re-export as `Error`** (`pub use error::{Error, Result}`) at the crate
   root. Consumers disambiguate via module path when needed.

4. **Never name a sub-error plain `Error`** — this creates import ambiguity
   even within the same crate.

## See Also

- [`finstack_core::error`](../finstack/core/src/error/mod.rs) — Core error hierarchy
- [`finstack_valuations::error`](../finstack/valuations/src/error.rs) — Valuations unified error
- [`finstack_portfolio::error`](../finstack/portfolio/src/error.rs) — Portfolio error
- [`finstack_scenarios::error`](../finstack/scenarios/src/error.rs) — Scenarios error
- [Rust API Guidelines — Error naming](https://rust-lang.github.io/api-guidelines/naming.html)
