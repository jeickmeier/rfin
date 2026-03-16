# Extract `finstack-margin` Crate

**Date:** 2026-03-16
**Status:** Approved
**Motivation:** Compile-time isolation (A) + independent consumability (B)

## Summary

Extract margin and XVA code from `finstack-valuations` into a new `finstack-margin` crate using trait inversion. The `Marginable` trait becomes standalone (no `Instrument` supertrait), and concrete implementations stay in `finstack-valuations` as a bridge layer.

## Current State

| Location | Files | Lines |
|---|---|---|
| `finstack/valuations/src/margin/` | 29 | ~8,400 |
| `finstack/valuations/src/xva/` | 5 | ~3,450 |
| `finstack/portfolio/src/margin/` | 4 | ~965 |
| **Total** | **38** | **~12,800** |

Portfolio-level margin code stays in `finstack-portfolio` (decided during brainstorming).

## Crate Structure

```
finstack/margin/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── traits.rs          # Marginable trait (standalone, no Instrument supertrait)
│   ├── constants.rs
│   ├── config.rs
│   ├── types/             # 9 type files (CSA, collateral, thresholds, enums, etc.)
│   ├── calculators/       # VM calculator + IM calculators (SIMM, Schedule, CCP, Haircut, Internal)
│   ├── metrics/           # Margin-specific metrics
│   ├── registry/          # Configuration registries + embedded JSON defaults
│   └── xva/               # CVA, DVA, FVA, exposure profiles, netting
└── data/
    └── margin/            # simm.v1.json, schedule_im.v1.json, ccp_methodologies.v1.json,
                           # collateral_schedules.v1.json, defaults.v1.json
```

## Dependency Graph

```
finstack-core <── finstack-margin <── finstack-valuations
                        ^                      |
                        |              (Marginable impls + Valuable blanket impl)
                        └──────────────────────┘

finstack-margin also depends on:
  - finstack-monte-carlo (XVA stochastic exposure, behind `mc` feature)
```

`finstack-portfolio` adds a direct `finstack-margin` dependency for margin types; keeps `finstack-valuations` for instrument concerns.

## Key Design Decisions

### 1. Trait Inversion — `Marginable` Becomes Standalone

**Before:** `Marginable: Instrument` (inherits full pricing/greeks/cashflow machinery)

**After:** `Marginable` is a standalone trait in `finstack-margin` with only the methods margin actually needs:

```rust
// finstack-margin/src/traits.rs
pub trait Marginable: Send + Sync {
    fn id(&self) -> &str;
    fn margin_spec(&self) -> Option<&OtcMarginSpec>;
    fn repo_margin_spec(&self) -> Option<&RepoMarginSpec>;
    fn netting_set_id(&self) -> Option<NettingSetId>;
    fn simm_sensitivities(&self, market: &MarketContext, as_of: Date) -> Result<SimmSensitivities>;
    fn mtm_for_vm(&self, market: &MarketContext, as_of: Date) -> Result<Money>;
    fn has_margin(&self) -> bool { /* default */ }
}
```

`RepoMarginSpec` (currently at `instruments/rates/repo/margin/spec.rs`) moves into `finstack-margin/src/types/repo_margin.rs`. It already depends on margin types (`EligibleCollateralSchedule`, `MarginTenor`), so it belongs with margin. The `Repo` struct in `finstack-valuations` will reference the type via `finstack_margin::RepoMarginSpec`.

### 2. `ImCalculator` Takes `&dyn Marginable`

**Before:** `fn calculate(&self, instrument: &dyn Instrument, ...) -> Result<ImResult>`

**After:** `fn calculate(&self, instrument: &dyn Marginable, ...) -> Result<ImResult>`

IM calculators only need SIMM sensitivities and margin specs — not pricing or greeks.

### 3. XVA Exposure — New `Valuable` Trait

XVA exposure currently takes `&[Arc<dyn Instrument>]` but only calls `.value()` and `.id()`. A minimal trait replaces this:

```rust
// finstack-margin/src/xva/traits.rs
pub trait Valuable: Send + Sync {
    fn id(&self) -> &str;
    fn value(&self, market: &MarketContext, as_of: Date) -> Result<Money>;
}
```

In `finstack-valuations`, a blanket impl bridges the gap:

```rust
impl<T: Instrument> finstack_margin::xva::Valuable for T { ... }
```

### 4. What Stays in `finstack-valuations`

- `impls.rs` — the 6 `Marginable` implementations (IRS, CDS, CDSIndex, EquityTRS, FITRS, Repo). These need instrument internals (pricers, struct fields).
- Blanket `Valuable` impl for `Instrument`
- Re-exports of `finstack_margin` types for backward compatibility

### 5. Backward Compatibility

`finstack-valuations` re-exports `finstack_margin` under its existing modules:

```rust
// finstack-valuations/src/margin/mod.rs (becomes thin re-export layer)
pub use finstack_margin::*;
mod impls;  // Bridge implementations stay here
```

Downstream code using `finstack_valuations::margin::*` and `finstack_valuations::xva::*` continues to work unchanged. Code that only needs margin/XVA types can depend on `finstack-margin` directly.

### 6. Feature Flags

```toml
# finstack-margin/Cargo.toml
[features]
default = []
mc = ["dep:nalgebra", "dep:finstack-monte-carlo", "finstack-monte-carlo/advanced"]  # Stochastic exposure
```

## Dependencies for `finstack-margin`

```toml
[dependencies]
finstack-core = { path = "../core" }
finstack-monte-carlo = { path = "../monte_carlo", default-features = false, optional = true }
serde = { workspace = true }
serde_json = { workspace = true, features = ["raw_value"] }
time = { workspace = true }
rust_decimal = { workspace = true }
tracing = { version = "0.1", default-features = false, features = ["attributes"] }
thiserror = { workspace = true }
strum = { workspace = true, features = ["derive"] }
nalgebra = { version = "0.34", optional = true }
```

## Files That Move

### From `finstack/valuations/src/margin/` → `finstack/margin/src/`

All files **except** `impls.rs`:
- `mod.rs` → `lib.rs` (rewritten as crate root)
- `traits.rs` (modified: remove `Instrument` supertrait)
- `constants.rs`
- `config.rs`
- `types/` (entire directory)
- `calculators/` (entire directory, `ImCalculator` updated to `&dyn Marginable`)
- `metrics/` (entire directory)
- `registry/` (entire directory)

### From `finstack/valuations/src/xva/` → `finstack/margin/src/xva/`

All files:
- `mod.rs`
- `cva.rs`
- `exposure.rs` (modified: `Instrument` → `Valuable` trait)
- `netting.rs`
- `types.rs`

### From `finstack/valuations/src/instruments/rates/repo/margin/spec.rs` → `finstack/margin/src/types/repo_margin.rs`

- `RepoMarginSpec`, `RepoMarginType` (margin types currently embedded in repo instrument module)

### From `finstack/valuations/data/margin/` → `finstack/margin/data/margin/`

- `simm.v1.json`
- `schedule_im.v1.json`
- `ccp_methodologies.v1.json`
- `collateral_schedules.v1.json`
- `defaults.v1.json`

### From `finstack/valuations/schemas/margin/` → `finstack/margin/schemas/margin/`

- `1/margin.schema.json`

## What Changes in `finstack-valuations`

1. `margin/mod.rs` becomes a thin re-export + `impls.rs`
2. `xva/mod.rs` becomes a thin re-export
3. `Cargo.toml` adds `finstack-margin` dependency
4. Removes direct margin/xva source files (replaced by re-exports)

## What Changes in `finstack-portfolio`

1. `Cargo.toml` adds `finstack-margin` dependency
2. `src/margin/` imports update from `finstack_valuations::margin::*` to `finstack_margin::*`

## What Changes in `finstack` (umbrella crate)

1. `Cargo.toml` adds optional `finstack-margin` dependency
2. Feature flags updated to include `margin` feature
3. Re-exports added

## What Changes in Bindings (finstack-py, finstack-wasm)

1. Import paths update to use `finstack_margin::*` where applicable
2. `Cargo.toml` adds `finstack-margin` dependency

## Workspace Changes

1. `Cargo.toml` workspace members adds `"finstack/margin"`
2. `Cargo.toml` default-members adds `"finstack/margin"`

## Test Strategy

- **Unit tests inside moved files** (e.g., `vm.rs`, `simm.rs`, `cva.rs`): Move with their source files. Update `use crate::` paths to reflect new crate root.
- **XVA exposure tests**: The `StaticInstrument` test mock currently implements `Instrument`. It must be updated to implement `Valuable` instead. This is a simplification — `Valuable` requires only `id()` and `value()`.
- **Integration tests** in `finstack/valuations/tests/instruments/*/margin.rs`: These test `Marginable` impls on concrete instruments. They stay in `finstack-valuations` since the impls stay there.
- **Portfolio margin tests** in `finstack/portfolio/tests/margin_aggregation.rs`: Stay in `finstack-portfolio`, update imports.
- **CI verification**: `cargo check --all-features` and `cargo check --no-default-features` must pass for `finstack-margin` independently. The `mc` feature gate must compile in isolation.

## Migration Ordering

1. **Create crate scaffold** — `Cargo.toml`, `lib.rs`, workspace registration
2. **Move types and traits** — `types/`, `traits.rs`, `constants.rs`, `config.rs`, `RepoMarginSpec`
3. **Move calculators and registry** — `calculators/`, `registry/`, data files
4. **Move metrics** — `metrics/`
5. **Move XVA** — `xva/`, introduce `Valuable` trait
6. **Wire up re-exports** — `finstack-valuations` re-exports, `impls.rs` stays as bridge
7. **Update downstream** — `finstack-portfolio`, `finstack` umbrella, bindings
8. **Verify** — full workspace `cargo test`, `cargo clippy`, CI checks
