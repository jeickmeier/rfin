# Global Configuration — Rounding & Scale Policy

Status: Draft (implementation-ready)
Last updated: 2025-01-25

## Purpose

Provide a workspace-wide configuration system with a deterministic rounding/scale policy for `rust_decimal` that applies at ingest (inputs/deserialization/builders) and at output (serialization/export). This ensures CSV/JSON interop stability across hosts and bindings. Active policy is stamped into `ResultsMeta.rounding` for auditability and reproducibility.

## Core API (in `finstack-core::config`)

```rust
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum RoundingMode { Bankers, AwayFromZero, TowardZero, Floor, Ceil }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurrencyScalePolicy {
    pub default_scale: u32,                 // e.g., 2 for USD, 0 for JPY
    pub overrides: indexmap::IndexMap<Currency, u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoundingPolicy {
    pub mode: RoundingMode,
    pub ingest_scale: CurrencyScalePolicy,
    pub output_scale: CurrencyScalePolicy,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoundingContext {
    pub mode: RoundingMode,
    pub ingest_scale_by_ccy: indexmap::IndexMap<Currency, u32>,
    pub output_scale_by_ccy: indexmap::IndexMap<Currency, u32>,
    pub version: u32,
}

#[derive(Clone, Debug)]
pub struct FinstackConfig { pub rounding: RoundingPolicy }

pub fn config() -> std::sync::Arc<FinstackConfig>;
pub fn with_temp_config<T>(cfg: FinstackConfig, f: impl FnOnce() -> T) -> T;
```

Implementation notes:
- Global storage via `once_cell::sync::OnceCell<std::sync::Arc<FinstackConfig>>` with an initialization function called by the meta-crate/host. Safe concurrent reads thereafter.
- `with_temp_config` uses thread-local or scoped guard to swap active config for the duration of a closure (testing/utilities); not for production mutation.

## Defaults

- `mode = RoundingMode::Bankers` (half-to-even)
- `default_scale = 2`
- ISO-4217 overrides example: `JPY=0`, `KWD=3`, `BHD=3`, `TND=3`
- Version starts at `1` and increments on schema/semantic changes.

## Application Points

- Ingest: deserializers, CSV/JSON loaders, builder methods that accept external numeric inputs normalize `Decimal` using `ingest_scale` per currency when the currency is known; otherwise `default_scale`.
- Output: serializers and export utilities (CSV/JSON/Arrow, display helpers) apply `output_scale` per currency. Results include `ResultsMeta.rounding`.
- Unitless scalars: use `default_scale` unless explicitly overridden by the caller/context.

## Results Meta

All top-level result envelopes include the active `RoundingContext`:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResultsMeta {
    pub numeric_mode: NumericMode,
    pub parallel: bool,
    pub seed: u64,
    pub model_currency: Option<Currency>,
    pub rounding: crate::config::RoundingContext,
}
```

## Cross-Crate Responsibilities

- Statements: Outputs in `Results.meta.rounding` reflect the policy used when evaluating/serializing node values.
- Valuations: `ValuationResult.meta.rounding` reflects the policy used for amounts/measures and any collapsed model/base-currency outputs.
- Portfolio: `PortfolioResults.meta.rounding` reflects the portfolio-wide policy used during aggregation and export.

## Testing & Acceptance

- Property tests: stable string serialization for a fixed `RoundingContext` across platforms/bindings.
- Golden tests: include `rounding` in meta; replays must match byte-for-byte.
- Negative tests: verify different `RoundingMode` or scales produce expected deltas.

## Future Extensions

- Locale-aware formatting profiles (names, separators) separate from rounding policy.
- Per-field scale overrides (e.g., percents/bps) with explicit type-level carriers.
- Time-based policy versioning for migrations.


