# Extract `finstack-margin` Crate Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract margin and XVA code from `finstack-valuations` into a standalone `finstack-margin` crate with clean dependency boundaries via trait inversion.

**Architecture:** New `finstack-margin` crate depends only on `finstack-core` (and optionally `finstack-monte-carlo` behind `mc` feature). The `Marginable` trait becomes standalone (no `Instrument` supertrait). Concrete `Marginable` impls for each instrument stay in `finstack-valuations` as a bridge. `finstack-valuations` re-exports `finstack-margin` for backward compatibility.

**Tech Stack:** Rust workspace crate, serde, finstack-core, finstack-monte-carlo (optional)

**Spec:** `docs/superpowers/specs/2026-03-16-extract-finstack-margin-crate-design.md`

---

## Chunk 1: Crate Scaffold and Types

### Task 1: Create crate scaffold and register in workspace

**Files:**

- Create: `finstack/margin/Cargo.toml`
- Create: `finstack/margin/src/lib.rs`
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Create `finstack/margin/Cargo.toml`**

```toml
[package]
name = "finstack-margin"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true

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

[dev-dependencies]
time = { workspace = true, features = ["macros"] }
rust_decimal_macros = { workspace = true }

[features]
default = []
mc = ["dep:nalgebra", "dep:finstack-monte-carlo", "finstack-monte-carlo/advanced"]
```

- [ ] **Step 2: Create minimal `finstack/margin/src/lib.rs`**

```rust
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![doc(test(attr(allow(clippy::expect_used))))]

//! Margin, collateral, and XVA (valuation adjustments) framework.
//!
//! This crate provides industry-standard margining, collateral management,
//! and XVA calculations independent of specific instrument implementations.
```

- [ ] **Step 3: Register in workspace `Cargo.toml`**

Add `"finstack/margin"` to both `members` and `default-members` arrays in the workspace root `Cargo.toml`.

- [ ] **Step 4: Verify scaffold compiles**

Run: `cargo check -p finstack-margin`
Expected: compiles with no errors

- [ ] **Step 5: Commit**

```bash
git add finstack/margin/Cargo.toml finstack/margin/src/lib.rs Cargo.toml Cargo.lock
git commit -m "feat: scaffold finstack-margin crate and register in workspace"
```

---

### Task 2: Move margin type definitions

**Files:**

- Copy: `finstack/valuations/src/margin/types/*.rs` → `finstack/margin/src/types/*.rs`
- Copy: `finstack/valuations/src/margin/constants.rs` → `finstack/margin/src/constants.rs`
- Modify: `finstack/margin/src/lib.rs` (add modules)

These files depend only on `finstack_core` types (`Currency`, `Money`, `Date`, `CurveId`, `HashMap`, `Error`, `Result`, `FinstackConfig`). The move requires updating `use crate::` paths.

- [ ] **Step 1: Copy the types directory**

Copy all 9 files from `finstack/valuations/src/margin/types/` to `finstack/margin/src/types/`:

- `mod.rs`
- `call.rs`
- `collateral.rs`
- `csa.rs`
- `enums.rs`
- `netting.rs`
- `otc.rs`
- `simm_types.rs`
- `thresholds.rs`

- [ ] **Step 2: Copy constants.rs**

Copy `finstack/valuations/src/margin/constants.rs` to `finstack/margin/src/constants.rs`.

- [ ] **Step 3: Update import paths in copied files**

In all copied files, replace:

- `use crate::margin::types::` → `use crate::types::`
- `use crate::margin::constants` → `use crate::constants`
- Any `finstack_valuations::margin::` in doc comments → `finstack_margin::`

The types files use only `finstack_core::` imports (which don't change) and intra-margin references (which become `crate::` references).

- [ ] **Step 4: Add modules to `lib.rs`**

Add to `finstack/margin/src/lib.rs`:

```rust
pub mod constants;
pub mod types;

// Re-export main types for convenience
pub use types::{
    ClearingStatus, CollateralAssetClass, CollateralEligibility, ConcentrationBreach,
    CsaSpec, EligibleCollateralSchedule, ImMethodology, ImParameters, InstrumentMarginResult,
    MarginCall, MarginCallTiming, MarginCallType, MarginTenor, MaturityConstraints,
    NettingSetId, OtcMarginSpec, SimmCreditSector, SimmRiskClass, SimmSensitivities,
    VmParameters,
};
```

- [ ] **Step 5: Verify compiles**

Run: `cargo check -p finstack-margin`
Expected: compiles with no errors

- [ ] **Step 6: Commit**

```bash
git add finstack/margin/src/
git commit -m "feat(margin): move margin type definitions and constants to finstack-margin"
```

---

### Task 3: Move `RepoMarginSpec` to finstack-margin

**Files:**

- Copy: `finstack/valuations/src/instruments/rates/repo/margin/spec.rs` → `finstack/margin/src/types/repo_margin.rs`
- Modify: `finstack/margin/src/types/mod.rs` (add module + re-exports)
- Modify: `finstack/margin/src/lib.rs` (add re-exports)

- [ ] **Step 1: Copy `spec.rs` to `repo_margin.rs`**

Copy `finstack/valuations/src/instruments/rates/repo/margin/spec.rs` to `finstack/margin/src/types/repo_margin.rs`.

- [ ] **Step 2: Update imports in `repo_margin.rs`**

Replace:

- `use crate::margin::types::{EligibleCollateralSchedule, MarginTenor};` → `use crate::types::{EligibleCollateralSchedule, MarginTenor};`
- `finstack_valuations::` in doc comments → `finstack_margin::`

- [ ] **Step 3: Add to `types/mod.rs`**

Add module declaration and re-exports:

```rust
pub mod repo_margin;
pub use repo_margin::{RepoMarginSpec, RepoMarginType};
```

- [ ] **Step 4: Add to `lib.rs` re-exports**

Add `RepoMarginSpec, RepoMarginType` to the `pub use types::` block.

- [ ] **Step 5: Verify compiles**

Run: `cargo check -p finstack-margin`
Expected: compiles with no errors

- [ ] **Step 6: Commit**

```bash
git add finstack/margin/src/types/
git commit -m "feat(margin): move RepoMarginSpec to finstack-margin types"
```

---

### Task 4: Create standalone `Marginable` trait

**Files:**

- Create: `finstack/margin/src/traits.rs`
- Modify: `finstack/margin/src/lib.rs`

- [ ] **Step 1: Create `traits.rs` with standalone `Marginable`**

```rust
//! Traits for marginable instruments.
//!
//! Defines the common interface for instruments that support margin calculations,
//! enabling uniform margin metric calculation and portfolio aggregation.

use crate::types::repo_margin::RepoMarginSpec;
use crate::types::OtcMarginSpec;
use crate::types::{NettingSetId, SimmSensitivities};
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

/// Trait for instruments that support margin calculations.
///
/// This is a standalone trait (no `Instrument` supertrait) so that
/// `finstack-margin` has no dependency on `finstack-valuations`.
/// Concrete implementations live in `finstack-valuations` as a bridge layer.
pub trait Marginable: Send + Sync {
    /// Get the instrument's unique identifier.
    fn id(&self) -> &str;

    /// Get the OTC margin specification for this instrument.
    ///
    /// Returns `None` if the instrument has no OTC margin requirements configured.
    fn margin_spec(&self) -> Option<&OtcMarginSpec>;

    /// Get the repo margin specification (for repos only).
    ///
    /// Default implementation returns `None`. Override for repo instruments.
    fn repo_margin_spec(&self) -> Option<&RepoMarginSpec> {
        None
    }

    /// Get the netting set identifier for margin aggregation.
    ///
    /// Instruments in the same netting set can offset each other.
    /// Returns `None` if the instrument is not part of a netting set.
    fn netting_set_id(&self) -> Option<NettingSetId>;

    /// Calculate SIMM sensitivities for this instrument.
    ///
    /// Returns the risk sensitivities needed for ISDA SIMM calculation.
    fn simm_sensitivities(&self, market: &MarketContext, as_of: Date) -> Result<SimmSensitivities>;

    /// Get the current mark-to-market value for VM calculation.
    ///
    /// This is typically the NPV of the instrument.
    fn mtm_for_vm(&self, market: &MarketContext, as_of: Date) -> Result<Money>;

    /// Check if margin is applicable for this instrument.
    fn has_margin(&self) -> bool {
        self.margin_spec().is_some() || self.repo_margin_spec().is_some()
    }
}
```

- [ ] **Step 2: Add to `lib.rs`**

```rust
pub mod traits;
pub use traits::Marginable;
```

- [ ] **Step 3: Verify compiles**

Run: `cargo check -p finstack-margin`
Expected: compiles with no errors

- [ ] **Step 4: Commit**

```bash
git add finstack/margin/src/traits.rs finstack/margin/src/lib.rs
git commit -m "feat(margin): add standalone Marginable trait without Instrument supertrait"
```

---

## Chunk 2: Calculators, Registry, Config, and Metrics

### Task 5: Move margin calculators

**Files:**

- Copy: `finstack/valuations/src/margin/calculators/` → `finstack/margin/src/calculators/`
- Modify: All copied calculator files (update imports)

Key change: `ImCalculator::calculate` signature changes from `&dyn Instrument` to `&dyn Marginable`.

- [ ] **Step 1: Copy entire calculators directory**

Copy `finstack/valuations/src/margin/calculators/` to `finstack/margin/src/calculators/`:

- `mod.rs`
- `traits.rs` (contains `ImCalculator`, `ImResult`)
- `vm.rs` (contains `VmCalculator`, `VmResult`)
- `im/mod.rs`
- `im/simm.rs`
- `im/schedule.rs`
- `im/clearing.rs`
- `im/haircut.rs`
- `im/internal.rs`

- [ ] **Step 2: Update `calculators/traits.rs` — key trait change**

Replace:

```rust
use crate::instruments::common_impl::traits::Instrument;
```

with:

```rust
use crate::traits::Marginable;
```

In the `ImCalculator` trait, change:

```rust
fn calculate(
    &self,
    instrument: &dyn Instrument,
    context: &MarketContext,
    as_of: Date,
) -> Result<ImResult>;
```

to:

```rust
fn calculate(
    &self,
    instrument: &dyn Marginable,
    context: &MarketContext,
    as_of: Date,
) -> Result<ImResult>;
```

Update the doc example to use `Marginable` instead of `Instrument`.

- [ ] **Step 3: Update all IM calculator implementations**

In each of `simm.rs`, `schedule.rs`, `clearing.rs`, `haircut.rs`, `internal.rs`:

- Replace `use crate::instruments::common_impl::traits::Instrument;` → `use crate::traits::Marginable;`
- Replace `instrument: &dyn Instrument` → `instrument: &dyn Marginable` in `calculate()` signatures
- Replace `use crate::margin::` → `use crate::` for internal references
- Replace `use super::super::` → `use crate::` where appropriate
- Update doc comments: `finstack_valuations::` → `finstack_margin::`

Note: The IM calculators primarily use `simm_sensitivities()`, `margin_spec()`, and `id()` — all available on `Marginable`. Some calculators use `instrument.value()` — these calls should use `instrument.mtm_for_vm()` instead (semantically equivalent for margin purposes).

- [ ] **Step 4: Update `calculators/vm.rs`**

This file uses only `finstack_core` types (no `Instrument` dependency). Update:

- `use crate::margin::types::` → `use crate::types::`
- Doc comments: `finstack_valuations::margin::` → `finstack_margin::`

- [ ] **Step 5: Add to `lib.rs`**

```rust
pub mod calculators;

pub use calculators::im::schedule::ScheduleAssetClass;
pub use calculators::im::simm::SimmVersion;
pub use calculators::{
    CcpMarginInputSource, CcpMethodology, ClearingHouseImCalculator, HaircutImCalculator,
    ImCalculator, ImResult, InternalModelImCalculator, InternalModelInputSource,
    ScheduleImCalculator, SimmCalculator, VmCalculator, VmResult,
};
```

- [ ] **Step 6: Verify compiles**

Run: `cargo check -p finstack-margin`
Expected: compiles with no errors

- [ ] **Step 7: Run tests**

Run: `cargo test -p finstack-margin`
Expected: all tests pass (unit tests embedded in calculator files)

- [ ] **Step 8: Commit**

```bash
git add finstack/margin/src/calculators/ finstack/margin/src/lib.rs
git commit -m "feat(margin): move margin calculators with Marginable trait interface"
```

---

### Task 6: Move registry, config, and data files

**Files:**

- Copy: `finstack/valuations/src/margin/registry/` → `finstack/margin/src/registry/`
- Copy: `finstack/valuations/src/margin/config.rs` → `finstack/margin/src/config.rs`
- Copy: `finstack/valuations/data/margin/` → `finstack/margin/data/margin/`
- Copy: `finstack/valuations/schemas/margin/` → `finstack/margin/schemas/margin/`

- [ ] **Step 1: Copy registry directory**

Copy all 4 files: `mod.rs`, `embedded.rs`, `merge.rs`, `wire.rs`.

- [ ] **Step 2: Copy config.rs**

- [ ] **Step 3: Copy data and schema directories**

Copy `finstack/valuations/data/margin/` (all 5 JSON files) to `finstack/margin/data/margin/`.
Copy `finstack/valuations/schemas/margin/` to `finstack/margin/schemas/margin/`.

- [ ] **Step 4: Update import paths in registry and config**

In all copied files:

- `use crate::margin::` → `use crate::`
- `finstack_valuations::` in doc comments → `finstack_margin::`

In `registry/embedded.rs`, update `CARGO_MANIFEST_DIR` paths — these should now resolve to `finstack/margin/` automatically since `CARGO_MANIFEST_DIR` is per-crate.

- [ ] **Step 5: Add to `lib.rs`**

```rust
pub mod config;
pub mod registry;
```

- [ ] **Step 6: Verify compiles and tests pass**

Run: `cargo check -p finstack-margin && cargo test -p finstack-margin`

- [ ] **Step 7: Commit**

```bash
git add finstack/margin/src/registry/ finstack/margin/src/config.rs finstack/margin/data/ finstack/margin/schemas/
git commit -m "feat(margin): move registry, config, and data files to finstack-margin"
```

---

### Task 7: Move margin metrics

**Files:**

- Copy: `finstack/valuations/src/margin/metrics/` → `finstack/margin/src/metrics/`

- [ ] **Step 1: Copy metrics directory**

Copy `mod.rs` and `instrument.rs`.

- [ ] **Step 2: Update imports**

In `metrics/instrument.rs`:

- Replace `use crate::instruments::common_impl::traits::Instrument` references — the metrics module uses `Marginable` trait methods. Replace with `use crate::traits::Marginable;`
- Replace `use crate::margin::` → `use crate::`
- Update doc comments

In `metrics/mod.rs`:

- Replace `use crate::margin::` → `use crate::`
- Update doc comments

- [ ] **Step 3: Add to `lib.rs`**

```rust
pub mod metrics;
```

- [ ] **Step 4: Verify compiles and tests pass**

Run: `cargo check -p finstack-margin && cargo test -p finstack-margin`

- [ ] **Step 5: Commit**

```bash
git add finstack/margin/src/metrics/ finstack/margin/src/lib.rs
git commit -m "feat(margin): move margin metrics to finstack-margin"
```

---

## Chunk 3: XVA Module

### Task 8: Create `Valuable` trait and move XVA

**Files:**

- Create: `finstack/margin/src/xva/traits.rs`
- Copy: `finstack/valuations/src/xva/` → `finstack/margin/src/xva/`

- [ ] **Step 1: Copy XVA directory**

Copy all 4 files: `mod.rs`, `cva.rs`, `exposure.rs`, `netting.rs`, `types.rs`.

- [ ] **Step 2: Create `xva/traits.rs` with `Valuable` trait**

```rust
//! Traits for XVA-compatible instruments.

use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;
use std::sync::Arc;

/// Minimal trait for instruments used in XVA exposure calculations.
///
/// XVA exposure only needs to value instruments at future dates.
/// This decouples XVA from the full `Instrument` trait in `finstack-valuations`.
pub trait Valuable: Send + Sync {
    /// Get the instrument's unique identifier.
    fn id(&self) -> &str;

    /// Calculate the instrument's present value.
    fn value(&self, market: &MarketContext, as_of: Date) -> Result<Money>;
}
```

- [ ] **Step 3: Update `xva/mod.rs`**

Add `pub mod traits;` and re-export `Valuable`.
Update doc comments: `finstack_valuations::xva::` → `finstack_margin::xva::`.

- [ ] **Step 4: Update `xva/exposure.rs`**

Key changes:

- Replace `use crate::instruments::Instrument;` → `use crate::xva::traits::Valuable;`
- Replace all `&[Arc<dyn Instrument>]` → `&[Arc<dyn Valuable>]`
- Replace `&dyn Instrument` → `&dyn Valuable` in helper functions
- The test `StaticInstrument` mock: replace `impl Instrument for StaticInstrument` with `impl Valuable for StaticInstrument` — this is simpler since `Valuable` only requires `id()` and `value()`.
- Replace `use crate::instruments::Attributes` in tests → remove (not needed for `Valuable`)
- Update `#[cfg(feature = "mc")]` imports: `use finstack_monte_carlo::` stays the same

- [ ] **Step 5: Update `xva/cva.rs`**

- Replace `finstack_valuations::xva::` → `finstack_margin::xva::` in doc comments
- Internal types are all within XVA, so `use super::types::` stays as-is

- [ ] **Step 6: Update `xva/netting.rs`**

- Replace `finstack_valuations::xva::` → `finstack_margin::xva::` in doc comments

- [ ] **Step 7: Update `xva/types.rs`**

- Replace `finstack_valuations::xva::` → `finstack_margin::xva::` in doc comments
- `#[cfg(feature = "mc")]` types stay the same

- [ ] **Step 8: Add to `lib.rs`**

```rust
pub mod xva;
```

- [ ] **Step 9: Verify compiles and tests pass**

Run: `cargo check -p finstack-margin && cargo test -p finstack-margin`

For MC-gated code: `cargo check -p finstack-margin --features mc`

- [ ] **Step 10: Commit**

```bash
git add finstack/margin/src/xva/ finstack/margin/src/lib.rs
git commit -m "feat(margin): move XVA module with Valuable trait to finstack-margin"
```

---

## Chunk 4: Wire Up Re-exports and Update Downstream

### Task 9: Add `finstack-margin` dependency to `finstack-valuations` and create re-export layer

**Files:**

- Modify: `finstack/valuations/Cargo.toml`
- Rewrite: `finstack/valuations/src/margin/mod.rs` (thin re-export)
- Rewrite: `finstack/valuations/src/xva/mod.rs` (thin re-export)
- Keep: `finstack/valuations/src/margin/impls.rs` (bridge layer)
- Keep: `finstack/valuations/src/margin/traits.rs` (bridge: re-export + Instrument supertrait alias)

- [ ] **Step 1: Add dependency to `finstack/valuations/Cargo.toml`**

Add to `[dependencies]`:

```toml
finstack-margin = { path = "../margin" }
```

And for mc feature, add to `[features]`:

```toml
mc = ["dep:nalgebra", "finstack-monte-carlo/advanced", "finstack-margin/mc"]
```

- [ ] **Step 2: Rewrite `finstack/valuations/src/margin/mod.rs`**

Replace the entire file with a thin re-export layer that preserves backward compatibility:

```rust
//! Margin and collateral management for financial instruments.
//!
//! This module re-exports types from `finstack-margin` and provides
//! `Marginable` implementations for specific instrument types.

// Re-export everything from finstack-margin
pub use finstack_margin::calculators;
pub use finstack_margin::config;
pub use finstack_margin::constants;
pub use finstack_margin::metrics;
pub use finstack_margin::registry;
pub use finstack_margin::types;

// Re-export all top-level types for backward compatibility
pub use finstack_margin::{
    CcpMarginInputSource, CcpMethodology, ClearingHouseImCalculator, ClearingStatus,
    CollateralAssetClass, CollateralEligibility, ConcentrationBreach, CsaSpec,
    EligibleCollateralSchedule, HaircutImCalculator, ImCalculator, ImMethodology, ImParameters,
    ImResult, InstrumentMarginResult, InternalModelImCalculator, InternalModelInputSource,
    MarginCall, MarginCallTiming, MarginCallType, MarginTenor, Marginable, MaturityConstraints,
    NettingSetId, OtcMarginSpec, RepoMarginSpec, RepoMarginType, ScheduleAssetClass,
    ScheduleImCalculator, SimmCalculator, SimmCreditSector, SimmRiskClass, SimmSensitivities,
    SimmVersion, VmCalculator, VmParameters, VmResult,
};

// Bridge: Marginable implementations for concrete instruments
mod impls;

// Bridge: re-export traits module for backward compat with
// `use finstack_valuations::margin::traits::Marginable`
pub mod traits {
    //! Re-exports from `finstack-margin` traits.
    pub use finstack_margin::traits::Marginable;
    pub use finstack_margin::types::{
        InstrumentMarginResult, NettingSetId, SimmCreditSector, SimmRiskClass, SimmSensitivities,
    };
}
```

- [ ] **Step 3: Update `finstack/valuations/src/margin/impls.rs`**

Update imports to use `finstack_margin` types:

- Replace `use crate::margin::constants` → `use finstack_margin::constants`
- Replace `use crate::margin::traits::{Marginable, NettingSetId, SimmSensitivities}` → `use finstack_margin::{Marginable, NettingSetId, SimmSensitivities}`
- Replace `use crate::margin::types::{ClearingStatus, OtcMarginSpec}` → `use finstack_margin::{ClearingStatus, OtcMarginSpec}`
- Keep: `use crate::instruments::*` imports (these are the bridge to valuations)

The `Marginable` trait no longer has `Instrument` as a supertrait, so `impl Marginable for InterestRateSwap` just needs to add `fn id(&self) -> &str { self.id.as_str() }` (or equivalent) since `id()` is now part of `Marginable` directly.

- [ ] **Step 4: Rewrite `finstack/valuations/src/xva/mod.rs`**

Replace with thin re-export:

```rust
//! XVA (Valuation Adjustments) framework.
//!
//! Re-exports from `finstack-margin` XVA module.

pub use finstack_margin::xva::*;

// Bridge: blanket Valuable impl for Instrument
mod bridge;
```

- [ ] **Step 5: Create `finstack/valuations/src/xva/bridge.rs`**

```rust
//! Bridge between `Instrument` and `Valuable` traits.

use crate::instruments::Instrument;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_margin::xva::traits::Valuable;

impl<T: Instrument> Valuable for T {
    fn id(&self) -> &str {
        Instrument::id(self)
    }

    fn value(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<Money> {
        Instrument::value(self, market, as_of)
    }
}
```

- [ ] **Step 6: Delete moved source files from `finstack-valuations`**

Delete (they are now re-exported from `finstack-margin`):

- `finstack/valuations/src/margin/types/` (entire directory)
- `finstack/valuations/src/margin/constants.rs`
- `finstack/valuations/src/margin/config.rs`
- `finstack/valuations/src/margin/calculators/` (entire directory)
- `finstack/valuations/src/margin/metrics/` (entire directory)
- `finstack/valuations/src/margin/registry/` (entire directory)
- `finstack/valuations/src/xva/cva.rs`
- `finstack/valuations/src/xva/exposure.rs`
- `finstack/valuations/src/xva/netting.rs`
- `finstack/valuations/src/xva/types.rs`
- `finstack/valuations/data/margin/` (entire directory)
- `finstack/valuations/schemas/margin/` (entire directory)

Keep:

- `finstack/valuations/src/margin/mod.rs` (re-export layer)
- `finstack/valuations/src/margin/impls.rs` (bridge)
- `finstack/valuations/src/margin/traits.rs` → delete this file, it's replaced by the `traits` submodule in the new `mod.rs`
- `finstack/valuations/src/xva/mod.rs` (re-export layer)
- `finstack/valuations/src/xva/bridge.rs` (blanket impl)

- [ ] **Step 7: Update `RepoMarginSpec` import in Repo instrument**

In `finstack/valuations/src/instruments/rates/repo/margin/spec.rs` — this file is now empty / a re-export since the type moved. Replace its contents with:

```rust
//! Re-exports from `finstack-margin`.
pub use finstack_margin::{RepoMarginSpec, RepoMarginType};
```

Or update `finstack/valuations/src/instruments/rates/repo/mod.rs` to import from `finstack_margin` directly.

- [ ] **Step 8: Verify full workspace compiles**

Run: `cargo check --workspace`
Expected: compiles with no errors

- [ ] **Step 9: Run all tests**

Run: `cargo test --workspace`
Expected: all tests pass

- [ ] **Step 10: Commit**

```bash
git add -A finstack/valuations/ finstack/margin/
git commit -m "refactor: wire finstack-valuations to re-export from finstack-margin"
```

---

### Task 10: Update `finstack-portfolio` imports

**Files:**

- Modify: `finstack/portfolio/Cargo.toml`
- Modify: `finstack/portfolio/src/margin/netting_set.rs`
- Modify: `finstack/portfolio/src/margin/results.rs`
- Modify: `finstack/portfolio/src/margin/aggregator.rs`

- [ ] **Step 1: Add `finstack-margin` dependency**

Add to `finstack/portfolio/Cargo.toml` `[dependencies]`:

```toml
finstack-margin = { path = "../margin" }
```

- [ ] **Step 2: Update imports in portfolio margin files**

In `netting_set.rs`, `results.rs`, `aggregator.rs`, replace:

- `use finstack_valuations::margin::` → `use finstack_margin::`

These will still compile with the old imports (due to re-exports), but direct imports are cleaner for the stated goal of independent consumability.

- [ ] **Step 3: Verify compiles and tests**

Run: `cargo check -p finstack-portfolio && cargo test -p finstack-portfolio`

- [ ] **Step 4: Commit**

```bash
git add finstack/portfolio/
git commit -m "refactor: update finstack-portfolio to import directly from finstack-margin"
```

---

### Task 11: Update umbrella crate and bindings

**Files:**

- Modify: `finstack/Cargo.toml`
- Modify: `finstack/src/lib.rs` (if margin/xva are re-exported)
- Modify: `finstack-py/Cargo.toml`
- Modify: `finstack-py/src/valuations/margin/mod.rs`
- Modify: `finstack-py/src/valuations/xva/mod.rs`
- Modify: `finstack-py/src/portfolio/margin.rs`
- Modify: `finstack-wasm/Cargo.toml`
- Modify: `finstack-wasm/src/valuations/margin/*.rs`
- Modify: `finstack-wasm/src/portfolio/margin.rs`

- [ ] **Step 1: Update umbrella crate**

Add optional `finstack-margin` dependency to `finstack/Cargo.toml` and add a `margin` feature. Update re-exports in `finstack/src/lib.rs` if the umbrella crate re-exports margin types.

- [ ] **Step 2: Update Python bindings**

Add `finstack-margin` to `finstack-py/Cargo.toml` dependencies.

In binding files, the imports `use finstack_valuations::margin::` will still work via re-exports. Optionally update to `use finstack_margin::` for clarity, but this is not blocking.

- [ ] **Step 3: Update WASM bindings**

Same approach as Python bindings. Add `finstack-margin` to `finstack-wasm/Cargo.toml`. Update imports if desired.

- [ ] **Step 4: Verify full workspace**

Run: `cargo check --workspace --all-features && cargo test --workspace`

- [ ] **Step 5: Commit**

```bash
git add finstack/Cargo.toml finstack/src/ finstack-py/ finstack-wasm/ Cargo.lock
git commit -m "refactor: update umbrella crate and bindings for finstack-margin"
```

---

## Chunk 5: Cleanup and Verification

### Task 12: Final cleanup and verification

**Files:**

- Possibly modify: various README.md files with stale import paths
- Modify: `finstack/valuations/src/margin/README.md` (if exists, update paths)

- [ ] **Step 1: Search for stale `finstack_valuations::margin` references**

Run: `grep -r "finstack_valuations::margin" --include="*.rs" finstack/margin/`

Expected: no hits (all should be `crate::` or `finstack_margin::`)

- [ ] **Step 2: Search for stale `crate::instruments` references in finstack-margin**

Run: `grep -r "crate::instruments" --include="*.rs" finstack/margin/`

Expected: no hits (finstack-margin has no knowledge of instruments)

- [ ] **Step 3: Verify independent compilation**

Run: `cargo check -p finstack-margin --no-default-features`
Run: `cargo check -p finstack-margin --features mc`

Expected: both compile

- [ ] **Step 4: Run clippy on new crate**

Run: `cargo clippy -p finstack-margin -- -D warnings`

Expected: no warnings

- [ ] **Step 5: Run full workspace test suite**

Run: `cargo test --workspace`

Expected: all tests pass

- [ ] **Step 6: Verify no stale data files remain**

Confirm `finstack/valuations/data/margin/` has been removed and `finstack/margin/data/margin/` contains all 5 JSON files.

- [ ] **Step 7: Final commit**

```bash
git add -A
git commit -m "chore: cleanup stale references after finstack-margin extraction"
```
