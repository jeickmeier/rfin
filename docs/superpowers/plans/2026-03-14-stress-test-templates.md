# Stress Test Template Library — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a template registry with named/tagged presets and a factory API for historical stress scenarios (GFC 2008, COVID 2020, 2022 rate shock, SVB 2023, LTCM 1998), producing standard `ScenarioSpec` via parameterized builders.

**Architecture:** A new `templates/` submodule within `finstack-scenarios`. Templates are plain factory functions returning `ScenarioSpecBuilder`. An optional `TemplateRegistry` indexes them for programmatic discovery. The entire feature is additive — no changes to the existing engine, specs, or adapters. Uses `IndexMap` (not `HashMap`) consistent with the rest of the crate.

**Tech Stack:** Rust, serde, indexmap, time

**Spec Reference:** `docs/superpowers/specs/2026-03-14-stress-test-templates-design.md`

---

## Task 1: TemplateMetadata and Enums

**Files:**
- Create: `finstack/scenarios/src/templates/mod.rs`
- Create: `finstack/scenarios/src/templates/metadata.rs`
- Modify: `finstack/scenarios/src/lib.rs` — add `pub mod templates;`

**Context:** `TemplateMetadata` is a pure data struct describing a template. `Severity` and `AssetClass` are simple enums. The crate uses `#[serde(deny_unknown_fields)]` on all spec types, `time::Date` for dates, and derives `Debug, Clone, Serialize, Deserialize` on all public types.

- [ ] **Step 1: Write failing tests for TemplateMetadata**

Create `finstack/scenarios/src/templates/metadata.rs` with the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    #[test]
    fn test_metadata_construction() {
        let meta = TemplateMetadata {
            id: "gfc_2008".into(),
            name: "Global Financial Crisis 2008".into(),
            description: "Lehman collapse scenario".into(),
            event_date: date!(2008 - 09 - 15),
            asset_classes: vec![AssetClass::Rates, AssetClass::Credit],
            tags: vec!["systemic".into(), "credit".into()],
            severity: Severity::Severe,
            components: vec!["gfc_2008_rates".into(), "gfc_2008_credit".into()],
        };
        assert_eq!(meta.id, "gfc_2008");
        assert_eq!(meta.severity, Severity::Severe);
        assert_eq!(meta.asset_classes.len(), 2);
    }

    #[test]
    fn test_metadata_serde_roundtrip() {
        let meta = TemplateMetadata {
            id: "test".into(),
            name: "Test".into(),
            description: "A test template".into(),
            event_date: date!(2020 - 03 - 16),
            asset_classes: vec![AssetClass::Equity],
            tags: vec!["test".into()],
            severity: Severity::Mild,
            components: vec![],
        };
        let json = serde_json::to_string(&meta).expect("serialize");
        let deser: TemplateMetadata = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(meta.id, deser.id);
        assert_eq!(meta.severity, deser.severity);
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Mild < Severity::Moderate);
        assert!(Severity::Moderate < Severity::Severe);
    }

    #[test]
    fn test_asset_class_display() {
        // Ensure all variants exist and are distinct
        let classes = vec![
            AssetClass::Rates,
            AssetClass::Credit,
            AssetClass::Equity,
            AssetClass::FX,
            AssetClass::Volatility,
            AssetClass::Commodity,
        ];
        assert_eq!(classes.len(), 6);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p finstack-scenarios --lib templates::metadata::tests`
Expected: FAIL — module doesn't exist yet

- [ ] **Step 3: Implement TemplateMetadata, Severity, AssetClass**

In `finstack/scenarios/src/templates/metadata.rs`:

```rust
//! Template metadata types for the stress test template library.

use serde::{Deserialize, Serialize};

/// Severity classification for stress scenarios.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Mild stress (e.g., modest curve reshaping).
    Mild,
    /// Moderate stress (e.g., sector-specific event).
    Moderate,
    /// Severe stress (e.g., systemic crisis).
    Severe,
}

/// Asset class categories affected by a stress template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetClass {
    /// Interest rates and fixed income.
    Rates,
    /// Credit spreads and default risk.
    Credit,
    /// Equity prices and dividends.
    Equity,
    /// Foreign exchange rates.
    FX,
    /// Implied and realized volatility.
    Volatility,
    /// Commodity prices.
    Commodity,
}

/// Metadata describing a stress test template.
///
/// Used by the [`TemplateRegistry`](super::TemplateRegistry) for discovery and filtering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMetadata {
    /// Stable identifier (e.g., `"gfc_2008"`).
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Description of the historical event and what this template models.
    pub description: String,
    /// Primary date of the historical event.
    pub event_date: time::Date,
    /// Asset classes affected by this template.
    pub asset_classes: Vec<AssetClass>,
    /// Freeform tags for filtering (e.g., `["systemic", "credit", "liquidity"]`).
    pub tags: Vec<String>,
    /// Severity classification.
    pub severity: Severity,
    /// IDs of composable sub-component templates.
    pub components: Vec<String>,
}
```

Create `finstack/scenarios/src/templates/mod.rs`:

```rust
//! Historical stress test templates and registry.
//!
//! Provides pre-built [`ScenarioSpec`](crate::ScenarioSpec) templates for major
//! financial crises, a parameterized [`ScenarioSpecBuilder`] for customization,
//! and an optional [`TemplateRegistry`] for programmatic discovery.

mod metadata;

pub use metadata::{AssetClass, Severity, TemplateMetadata};
```

Add to `finstack/scenarios/src/lib.rs` after the `pub mod utils;` line:

```rust
/// Historical stress test templates and registry.
pub mod templates;
```

And add to the re-exports block:

```rust
pub use templates::{AssetClass, Severity, TemplateMetadata};
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p finstack-scenarios --lib templates::metadata::tests`
Expected: PASS (all 4 tests)

- [ ] **Step 5: Commit**

```bash
git add finstack/scenarios/src/templates/metadata.rs finstack/scenarios/src/templates/mod.rs finstack/scenarios/src/lib.rs
git commit -m "feat(scenarios): add TemplateMetadata, Severity, and AssetClass types"
```

---

## Task 2: ScenarioSpecBuilder

**Files:**
- Create: `finstack/scenarios/src/templates/builder.rs`
- Modify: `finstack/scenarios/src/templates/mod.rs` — add `mod builder; pub use builder::ScenarioSpecBuilder;`
- Modify: `finstack/scenarios/src/lib.rs` — add `ScenarioSpecBuilder` to re-exports

**Context:** The builder stores operations with conventional curve/equity IDs. `build()` resolves overrides by walking operations and substituting IDs, then validates the resulting `ScenarioSpec`. The crate uses `indexmap::IndexMap` everywhere (never `HashMap`). The crate forbids `unwrap_used` and `expect_used` — use `ok_or_else` or `?` instead. Use `crate::error::{Error, Result}` for error handling. `ScenarioEngine::compose()` takes `Vec<ScenarioSpec>` and returns `ScenarioSpec`.

- [ ] **Step 1: Write failing tests for ScenarioSpecBuilder**

Create `finstack/scenarios/src/templates/builder.rs` with test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CurveKind, OperationSpec};
    use finstack_core::currency::Currency;

    #[test]
    fn test_builder_basic_construction() {
        let builder = ScenarioSpecBuilder::new("test_scenario")
            .name("Test Scenario")
            .description("A test scenario")
            .priority(5);

        let spec = builder.build().expect("should build");
        assert_eq!(spec.id, "test_scenario");
        assert_eq!(spec.name.as_deref(), Some("Test Scenario"));
        assert_eq!(spec.description.as_deref(), Some("A test scenario"));
        assert_eq!(spec.priority, 5);
        assert!(spec.operations.is_empty());
    }

    #[test]
    fn test_builder_with_operations() {
        let spec = ScenarioSpecBuilder::new("rates")
            .with_operation(OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Discount,
                curve_id: "USD-SOFR".into(),
                bp: 100.0,
            })
            .with_operation(OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Forward,
                curve_id: "EUR-ESTR".into(),
                bp: -50.0,
            })
            .build()
            .expect("should build");

        assert_eq!(spec.operations.len(), 2);
    }

    #[test]
    fn test_builder_curve_override() {
        let spec = ScenarioSpecBuilder::new("test")
            .with_operation(OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Discount,
                curve_id: "USD-SOFR".into(),
                bp: 100.0,
            })
            .override_curve("USD-SOFR", "MY_CUSTOM_SOFR")
            .build()
            .expect("should build");

        match &spec.operations[0] {
            OperationSpec::CurveParallelBp { curve_id, .. } => {
                assert_eq!(curve_id, "MY_CUSTOM_SOFR");
            }
            _ => panic!("unexpected operation type"),
        }
    }

    #[test]
    fn test_builder_equity_override() {
        let spec = ScenarioSpecBuilder::new("test")
            .with_operation(OperationSpec::EquityPricePct {
                ids: vec!["SPX".into(), "NDX".into()],
                pct: -20.0,
            })
            .override_equity("SPX", "MY_SPX_INDEX")
            .build()
            .expect("should build");

        match &spec.operations[0] {
            OperationSpec::EquityPricePct { ids, .. } => {
                assert!(ids.contains(&"MY_SPX_INDEX".to_string()));
                assert!(ids.contains(&"NDX".to_string()));
                assert!(!ids.contains(&"SPX".to_string()));
            }
            _ => panic!("unexpected operation type"),
        }
    }

    #[test]
    fn test_builder_fx_override() {
        let spec = ScenarioSpecBuilder::new("test")
            .with_operation(OperationSpec::MarketFxPct {
                base: Currency::EUR,
                quote: Currency::USD,
                pct: -10.0,
            })
            .override_fx(
                (Currency::EUR, Currency::USD),
                (Currency::GBP, Currency::USD),
            )
            .build()
            .expect("should build");

        match &spec.operations[0] {
            OperationSpec::MarketFxPct { base, quote, .. } => {
                assert_eq!(*base, Currency::GBP);
                assert_eq!(*quote, Currency::USD);
            }
            _ => panic!("unexpected operation type"),
        }
    }

    #[test]
    fn test_builder_compose() {
        let builder1 = ScenarioSpecBuilder::new("rates")
            .priority(0)
            .with_operation(OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Discount,
                curve_id: "USD-SOFR".into(),
                bp: 100.0,
            });

        let builder2 = ScenarioSpecBuilder::new("equity")
            .priority(1)
            .with_operation(OperationSpec::EquityPricePct {
                ids: vec!["SPX".into()],
                pct: -20.0,
            });

        let composed = ScenarioSpecBuilder::compose(vec![builder1, builder2]);
        let spec = composed.build().expect("should build");

        // compose delegates to ScenarioEngine::compose, which concatenates by priority
        assert_eq!(spec.operations.len(), 2);
    }

    #[test]
    fn test_builder_vol_surface_override() {
        let spec = ScenarioSpecBuilder::new("test")
            .with_operation(OperationSpec::VolSurfaceParallelPct {
                surface_kind: crate::VolSurfaceKind::Equity,
                surface_id: "SPX_VOL".into(),
                pct: 50.0,
            })
            .override_curve("SPX_VOL", "MY_VOL_SURFACE")
            .build()
            .expect("should build");

        match &spec.operations[0] {
            OperationSpec::VolSurfaceParallelPct { surface_id, .. } => {
                assert_eq!(surface_id, "MY_VOL_SURFACE");
            }
            _ => panic!("unexpected operation type"),
        }
    }

    #[test]
    fn test_builder_validation_empty_id() {
        let result = ScenarioSpecBuilder::new("").build();
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_with_operations_batch() {
        let ops = vec![
            OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Discount,
                curve_id: "A".into(),
                bp: 10.0,
            },
            OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Discount,
                curve_id: "B".into(),
                bp: 20.0,
            },
        ];

        let spec = ScenarioSpecBuilder::new("test")
            .with_operations(ops)
            .build()
            .expect("should build");

        assert_eq!(spec.operations.len(), 2);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p finstack-scenarios --lib templates::builder::tests`
Expected: FAIL — `ScenarioSpecBuilder` not defined

- [ ] **Step 3: Implement ScenarioSpecBuilder**

In `finstack/scenarios/src/templates/builder.rs`:

```rust
//! Parameterized builder for constructing [`ScenarioSpec`](crate::ScenarioSpec) from templates.

use crate::engine::ScenarioEngine;
use crate::spec::{OperationSpec, ScenarioSpec};
use finstack_core::currency::Currency;
use indexmap::IndexMap;

/// A builder for constructing [`ScenarioSpec`] with parameterized overrides.
///
/// Templates return builders pre-configured with conventional curve/equity/FX IDs.
/// Users can override these IDs to match their market data before calling [`build()`](Self::build).
///
/// # Examples
///
/// ```rust
/// use finstack_scenarios::templates::ScenarioSpecBuilder;
/// use finstack_scenarios::{OperationSpec, CurveKind};
///
/// let spec = ScenarioSpecBuilder::new("my_scenario")
///     .name("My Scenario")
///     .with_operation(OperationSpec::CurveParallelBp {
///         curve_kind: CurveKind::Discount,
///         curve_id: "USD-SOFR".into(),
///         bp: 100.0,
///     })
///     .override_curve("USD-SOFR", "MY_SOFR_CURVE")
///     .build()
///     .expect("valid scenario");
/// ```
#[derive(Debug, Clone)]
pub struct ScenarioSpecBuilder {
    id: String,
    name: Option<String>,
    description: Option<String>,
    operations: Vec<OperationSpec>,
    priority: i32,
    curve_overrides: IndexMap<String, String>,
    equity_overrides: IndexMap<String, String>,
    fx_overrides: Vec<((Currency, Currency), (Currency, Currency))>,
}

impl ScenarioSpecBuilder {
    /// Create a new builder with the given scenario ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: None,
            description: None,
            operations: Vec::new(),
            priority: 0,
            curve_overrides: IndexMap::new(),
            equity_overrides: IndexMap::new(),
            fx_overrides: Vec::new(),
        }
    }

    /// Set the display name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set the priority for composition ordering (lower = runs first).
    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Add a single operation.
    pub fn with_operation(mut self, op: OperationSpec) -> Self {
        self.operations.push(op);
        self
    }

    /// Add multiple operations.
    pub fn with_operations(mut self, ops: Vec<OperationSpec>) -> Self {
        self.operations.extend(ops);
        self
    }

    /// Override a curve or surface ID. Applies to `CurveParallelBp`, `CurveNodeBp`,
    /// `VolSurfaceParallelPct`, and `VolSurfaceBucketPct` operations.
    pub fn override_curve(mut self, default_id: &str, user_id: &str) -> Self {
        self.curve_overrides
            .insert(default_id.to_string(), user_id.to_string());
        self
    }

    /// Override an equity identifier. Applies to `EquityPricePct` operations.
    pub fn override_equity(mut self, default_id: &str, user_id: &str) -> Self {
        self.equity_overrides
            .insert(default_id.to_string(), user_id.to_string());
        self
    }

    /// Override an FX pair. Applies to `MarketFxPct` operations.
    pub fn override_fx(
        mut self,
        default: (Currency, Currency),
        user: (Currency, Currency),
    ) -> Self {
        self.fx_overrides.push((default, user));
        self
    }

    /// Compose multiple builders into a single builder.
    ///
    /// Delegates to [`ScenarioEngine::compose()`] which sorts by priority
    /// and concatenates operations additively.
    pub fn compose(builders: Vec<ScenarioSpecBuilder>) -> Self {
        let specs: Vec<ScenarioSpec> = builders
            .into_iter()
            .map(|b| ScenarioSpec {
                id: b.id,
                name: b.name,
                description: b.description,
                operations: b.operations,
                priority: b.priority,
            })
            .collect();

        let engine = ScenarioEngine::new();
        let composed = engine.compose(specs);

        Self {
            id: composed.id,
            name: composed.name,
            description: composed.description,
            operations: composed.operations,
            priority: composed.priority,
            curve_overrides: IndexMap::new(),
            equity_overrides: IndexMap::new(),
            fx_overrides: Vec::new(),
        }
    }

    /// Build the final [`ScenarioSpec`], resolving all overrides and validating.
    pub fn build(mut self) -> crate::Result<ScenarioSpec> {
        self.resolve_overrides();

        let spec = ScenarioSpec {
            id: self.id,
            name: self.name,
            description: self.description,
            operations: self.operations,
            priority: self.priority,
        };

        spec.validate()?;
        Ok(spec)
    }

    /// Walk all operations and substitute overridden IDs.
    fn resolve_overrides(&mut self) {
        for op in &mut self.operations {
            match op {
                OperationSpec::CurveParallelBp { curve_id, .. }
                | OperationSpec::CurveNodeBp { curve_id, .. } => {
                    if let Some(replacement) = self.curve_overrides.get(curve_id.as_str()) {
                        *curve_id = replacement.clone();
                    }
                }
                OperationSpec::VolSurfaceParallelPct { surface_id, .. }
                | OperationSpec::VolSurfaceBucketPct { surface_id, .. } => {
                    if let Some(replacement) = self.curve_overrides.get(surface_id.as_str()) {
                        *surface_id = replacement.clone();
                    }
                }
                OperationSpec::EquityPricePct { ids, .. } => {
                    for id in ids.iter_mut() {
                        if let Some(replacement) = self.equity_overrides.get(id.as_str()) {
                            *id = replacement.clone();
                        }
                    }
                }
                OperationSpec::MarketFxPct { base, quote, .. } => {
                    for ((def_base, def_quote), (usr_base, usr_quote)) in &self.fx_overrides {
                        if base == def_base && quote == def_quote {
                            *base = *usr_base;
                            *quote = *usr_quote;
                            break;
                        }
                    }
                }
                OperationSpec::BaseCorrParallelPts { surface_id, .. }
                | OperationSpec::BaseCorrBucketPts { surface_id, .. } => {
                    if let Some(replacement) = self.curve_overrides.get(surface_id.as_str()) {
                        *surface_id = replacement.clone();
                    }
                }
                // Operations without overridable IDs
                _ => {}
            }
        }
    }
}
```

Update `finstack/scenarios/src/templates/mod.rs` to include:

```rust
mod builder;
pub use builder::ScenarioSpecBuilder;
```

Add `ScenarioSpecBuilder` to re-exports in `finstack/scenarios/src/lib.rs`:

```rust
pub use templates::{AssetClass, ScenarioSpecBuilder, Severity, TemplateMetadata};
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p finstack-scenarios --lib templates::builder::tests`
Expected: PASS (all 9 tests)

- [ ] **Step 5: Commit**

```bash
git add finstack/scenarios/src/templates/builder.rs finstack/scenarios/src/templates/mod.rs finstack/scenarios/src/lib.rs
git commit -m "feat(scenarios): add ScenarioSpecBuilder with parameterized overrides"
```

---

## Task 3: TemplateRegistry

**Files:**
- Create: `finstack/scenarios/src/templates/registry.rs`
- Modify: `finstack/scenarios/src/templates/mod.rs` — add `mod registry; pub use registry::TemplateRegistry;`
- Modify: `finstack/scenarios/src/lib.rs` — add `TemplateRegistry` to re-exports

**Context:** The registry holds factory closures (`Box<dyn Fn() -> ScenarioSpecBuilder + Send + Sync>`) so each `.build()` call produces a fresh builder. Uses `IndexMap` for deterministic iteration. The `Default` impl will be wired up in Task 8 after all templates are implemented — for now, `Default` returns an empty registry. The crate has `#[warn(clippy::new_without_default)]` so if we provide `new()` we must also impl `Default`.

- [ ] **Step 1: Write failing tests for TemplateRegistry**

Create `finstack/scenarios/src/templates/registry.rs` with test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::templates::{AssetClass, ScenarioSpecBuilder, Severity, TemplateMetadata};
    use crate::{CurveKind, OperationSpec};
    use time::macros::date;

    fn sample_metadata() -> TemplateMetadata {
        TemplateMetadata {
            id: "test_scenario".into(),
            name: "Test Scenario".into(),
            description: "A test".into(),
            event_date: date!(2020 - 03 - 16),
            asset_classes: vec![AssetClass::Rates, AssetClass::Credit],
            tags: vec!["systemic".into(), "test".into()],
            severity: Severity::Severe,
            components: vec![],
        }
    }

    fn sample_builder() -> ScenarioSpecBuilder {
        ScenarioSpecBuilder::new("test_scenario").with_operation(
            OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Discount,
                curve_id: "USD-SOFR".into(),
                bp: 100.0,
            },
        )
    }

    #[test]
    fn test_registry_register_and_get() {
        let mut registry = TemplateRegistry::new();
        registry.register(sample_metadata(), || sample_builder());

        let entry = registry.get("test_scenario");
        assert!(entry.is_some());
        assert_eq!(entry.expect("entry").metadata().id, "test_scenario");
    }

    #[test]
    fn test_registry_get_missing() {
        let registry = TemplateRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_registry_list() {
        let mut registry = TemplateRegistry::new();
        registry.register(sample_metadata(), || sample_builder());

        let list = registry.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "test_scenario");
    }

    #[test]
    fn test_registry_filter_by_tag() {
        let mut registry = TemplateRegistry::new();
        registry.register(sample_metadata(), || sample_builder());

        let mut meta2 = sample_metadata();
        meta2.id = "other".into();
        meta2.tags = vec!["other".into()];
        registry.register(meta2, || ScenarioSpecBuilder::new("other"));

        let systemic = registry.filter_by_tag("systemic");
        assert_eq!(systemic.len(), 1);
        assert_eq!(systemic[0].id, "test_scenario");
    }

    #[test]
    fn test_registry_filter_by_asset_class() {
        let mut registry = TemplateRegistry::new();
        registry.register(sample_metadata(), || sample_builder());

        let rates = registry.filter_by_asset_class(AssetClass::Rates);
        assert_eq!(rates.len(), 1);

        let commodity = registry.filter_by_asset_class(AssetClass::Commodity);
        assert!(commodity.is_empty());
    }

    #[test]
    fn test_registry_filter_by_severity() {
        let mut registry = TemplateRegistry::new();
        registry.register(sample_metadata(), || sample_builder());

        let severe = registry.filter_by_severity(Severity::Severe);
        assert_eq!(severe.len(), 1);

        let mild = registry.filter_by_severity(Severity::Mild);
        assert!(mild.is_empty());
    }

    #[test]
    fn test_registry_build_produces_fresh_builder() {
        let mut registry = TemplateRegistry::new();
        registry.register(sample_metadata(), || sample_builder());

        let entry = registry.get("test_scenario").expect("entry exists");

        // Two calls produce independent builders
        let spec1 = entry.builder().override_curve("USD-SOFR", "A").build().expect("build");
        let spec2 = entry.builder().override_curve("USD-SOFR", "B").build().expect("build");

        match (&spec1.operations[0], &spec2.operations[0]) {
            (
                OperationSpec::CurveParallelBp { curve_id: id1, .. },
                OperationSpec::CurveParallelBp { curve_id: id2, .. },
            ) => {
                assert_eq!(id1, "A");
                assert_eq!(id2, "B");
            }
            _ => panic!("unexpected operation types"),
        }
    }

    #[test]
    fn test_registry_register_with_components() {
        let mut registry = TemplateRegistry::new();
        let mut meta = sample_metadata();
        meta.components = vec!["test_scenario_rates".into()];

        registry.register_with_components(
            meta,
            || sample_builder(),
            vec![(
                "test_scenario_rates".into(),
                Box::new(|| ScenarioSpecBuilder::new("test_scenario_rates")),
            )],
        );

        let entry = registry.get("test_scenario").expect("entry exists");
        let component = entry.component("test_scenario_rates");
        assert!(component.is_some());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p finstack-scenarios --lib templates::registry::tests`
Expected: FAIL — `TemplateRegistry` not defined

- [ ] **Step 3: Implement TemplateRegistry**

In `finstack/scenarios/src/templates/registry.rs`:

```rust
//! Dynamic template registry for discovering and instantiating stress templates.

use super::builder::ScenarioSpecBuilder;
use super::metadata::TemplateMetadata;
use super::metadata::{AssetClass, Severity};
use indexmap::IndexMap;

/// A registered template entry containing metadata and a factory closure.
pub struct RegisteredTemplate {
    metadata: TemplateMetadata,
    factory: Box<dyn Fn() -> ScenarioSpecBuilder + Send + Sync>,
    components: IndexMap<String, Box<dyn Fn() -> ScenarioSpecBuilder + Send + Sync>>,
}

impl RegisteredTemplate {
    /// Returns the template metadata.
    pub fn metadata(&self) -> &TemplateMetadata {
        &self.metadata
    }

    /// Returns a fresh builder from the factory closure.
    pub fn builder(&self) -> ScenarioSpecBuilder {
        (self.factory)()
    }

    /// Returns a fresh builder for a named sub-component.
    pub fn component(&self, id: &str) -> Option<ScenarioSpecBuilder> {
        self.components.get(id).map(|f| f())
    }

    /// Lists available component IDs.
    pub fn component_ids(&self) -> Vec<&str> {
        self.components.keys().map(|s| s.as_str()).collect()
    }
}

/// A dynamic registry of stress test templates.
///
/// Supports both built-in historical templates and user-registered custom templates.
/// Use [`Default::default()`] to get a registry pre-loaded with all built-in templates,
/// or [`TemplateRegistry::new()`] for an empty registry.
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_scenarios::templates::{TemplateRegistry, AssetClass};
///
/// let registry = TemplateRegistry::default();
/// let credit_scenarios = registry.filter_by_tag("credit");
/// ```
pub struct TemplateRegistry {
    entries: IndexMap<String, RegisteredTemplate>,
}

impl Default for TemplateRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        super::register_builtins(&mut registry);
        registry
    }
}

impl TemplateRegistry {
    /// Create an empty registry with no templates.
    pub fn new() -> Self {
        Self {
            entries: IndexMap::new(),
        }
    }

    /// Register a template with metadata and a factory closure.
    pub fn register(
        &mut self,
        metadata: TemplateMetadata,
        factory: impl Fn() -> ScenarioSpecBuilder + Send + Sync + 'static,
    ) -> &mut Self {
        let id = metadata.id.clone();
        self.entries.insert(
            id,
            RegisteredTemplate {
                metadata,
                factory: Box::new(factory),
                components: IndexMap::new(),
            },
        );
        self
    }

    /// Register a template with sub-components.
    pub fn register_with_components(
        &mut self,
        metadata: TemplateMetadata,
        factory: impl Fn() -> ScenarioSpecBuilder + Send + Sync + 'static,
        components: Vec<(String, Box<dyn Fn() -> ScenarioSpecBuilder + Send + Sync>)>,
    ) -> &mut Self {
        let id = metadata.id.clone();
        let component_map: IndexMap<String, Box<dyn Fn() -> ScenarioSpecBuilder + Send + Sync>> =
            components.into_iter().collect();
        self.entries.insert(
            id,
            RegisteredTemplate {
                metadata,
                factory: Box::new(factory),
                components: component_map,
            },
        );
        self
    }

    /// Look up a registered template by ID.
    pub fn get(&self, id: &str) -> Option<&RegisteredTemplate> {
        self.entries.get(id)
    }

    /// List metadata for all registered templates.
    pub fn list(&self) -> Vec<&TemplateMetadata> {
        self.entries.values().map(|e| &e.metadata).collect()
    }

    /// Filter templates by tag (case-sensitive exact match).
    pub fn filter_by_tag(&self, tag: &str) -> Vec<&TemplateMetadata> {
        self.entries
            .values()
            .filter(|e| e.metadata.tags.iter().any(|t| t == tag))
            .map(|e| &e.metadata)
            .collect()
    }

    /// Filter templates by asset class.
    pub fn filter_by_asset_class(&self, ac: AssetClass) -> Vec<&TemplateMetadata> {
        self.entries
            .values()
            .filter(|e| e.metadata.asset_classes.contains(&ac))
            .map(|e| &e.metadata)
            .collect()
    }

    /// Filter templates by severity.
    pub fn filter_by_severity(&self, severity: Severity) -> Vec<&TemplateMetadata> {
        self.entries
            .values()
            .filter(|e| e.metadata.severity == severity)
            .map(|e| &e.metadata)
            .collect()
    }
}
```

Update `finstack/scenarios/src/templates/mod.rs` to include:

```rust
mod registry;
pub use registry::TemplateRegistry;
```

Also add a placeholder `register_builtins` function to `mod.rs` that will be populated later:

```rust
/// Register all built-in historical templates.
fn register_builtins(_registry: &mut TemplateRegistry) {
    // Will be populated as template modules are added
}
```

Add `TemplateRegistry` to re-exports in `finstack/scenarios/src/lib.rs`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p finstack-scenarios --lib templates::registry::tests`
Expected: PASS (all 8 tests)

- [ ] **Step 5: Commit**

```bash
git add finstack/scenarios/src/templates/registry.rs finstack/scenarios/src/templates/mod.rs finstack/scenarios/src/lib.rs
git commit -m "feat(scenarios): add TemplateRegistry with tag/asset-class/severity filtering"
```

---

## Task 4: GFC 2008 Template

**Files:**
- Create: `finstack/scenarios/src/templates/gfc_2008.rs`
- Modify: `finstack/scenarios/src/templates/mod.rs` — add module and wire into `register_builtins`

**Context:** GFC 2008 (Lehman collapse, Sep 2008 – Mar 2009). Approximate shocks: Rates -200bp (flight to quality), bear steepener +100bp 2s10s; Credit +300bp IG, +800bp HY; Equity -50% SPX, -55% broad; Vol +40pts; FX EUR/USD -10%. Use `CurveKind::Discount` for rate shocks, `CurveKind::ParCDS` for credit spread shocks. Default curve IDs follow the convention `"USD-SOFR"`, `"USD-HY"`, `"USD-IG"`, etc. Use `VolSurfaceKind::Equity` for equity vol.

- [ ] **Step 1: Write failing tests**

Create `finstack/scenarios/src/templates/gfc_2008.rs` with tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::OperationSpec;

    #[test]
    fn test_gfc_2008_full_builds() {
        let spec = gfc_2008().build().expect("should build");
        assert_eq!(spec.id, "gfc_2008");
        assert!(spec.name.is_some());
        assert!(!spec.operations.is_empty());
    }

    #[test]
    fn test_gfc_2008_metadata() {
        let meta = metadata();
        assert_eq!(meta.id, "gfc_2008");
        assert_eq!(meta.severity, crate::templates::Severity::Severe);
        assert_eq!(meta.components.len(), 5);
    }

    #[test]
    fn test_gfc_2008_rates_builds() {
        let spec = gfc_2008_rates().build().expect("should build");
        assert!(!spec.operations.is_empty());
        // All operations should be curve-related
        for op in &spec.operations {
            match op {
                OperationSpec::CurveParallelBp { .. } | OperationSpec::CurveNodeBp { .. } => {}
                other => panic!("unexpected operation in rates component: {other:?}"),
            }
        }
    }

    #[test]
    fn test_gfc_2008_credit_builds() {
        let spec = gfc_2008_credit().build().expect("should build");
        assert!(!spec.operations.is_empty());
    }

    #[test]
    fn test_gfc_2008_equity_builds() {
        let spec = gfc_2008_equity().build().expect("should build");
        assert!(!spec.operations.is_empty());
    }

    #[test]
    fn test_gfc_2008_vol_builds() {
        let spec = gfc_2008_vol().build().expect("should build");
        assert!(!spec.operations.is_empty());
    }

    #[test]
    fn test_gfc_2008_fx_builds() {
        let spec = gfc_2008_fx().build().expect("should build");
        assert!(!spec.operations.is_empty());
    }

    #[test]
    fn test_gfc_2008_composable() {
        // Compose only rates + credit
        let spec = crate::templates::ScenarioSpecBuilder::compose(vec![
            gfc_2008_rates(),
            gfc_2008_credit(),
        ])
        .build()
        .expect("should build");

        // Should have operations from both components
        let rates_spec = gfc_2008_rates().build().expect("build");
        let credit_spec = gfc_2008_credit().build().expect("build");
        assert_eq!(
            spec.operations.len(),
            rates_spec.operations.len() + credit_spec.operations.len()
        );
    }

    #[test]
    fn test_gfc_2008_with_overrides() {
        let spec = gfc_2008()
            .override_curve("USD-SOFR", "MY_SOFR")
            .override_equity("SPX", "MY_SPX")
            .build()
            .expect("should build");

        // Verify the override was applied — at least one curve should use MY_SOFR
        let has_override = spec.operations.iter().any(|op| match op {
            OperationSpec::CurveParallelBp { curve_id, .. } => curve_id == "MY_SOFR",
            _ => false,
        });
        assert!(has_override, "curve override should be applied");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p finstack-scenarios --lib templates::gfc_2008::tests`
Expected: FAIL

- [ ] **Step 3: Implement GFC 2008 template**

In `finstack/scenarios/src/templates/gfc_2008.rs`:

```rust
//! Global Financial Crisis 2008 stress template.
//!
//! Models the Lehman collapse period (Sep 2008 – Mar 2009) across rates,
//! credit, equity, volatility, and FX.

use super::builder::ScenarioSpecBuilder;
use super::metadata::{AssetClass, Severity, TemplateMetadata};
use crate::spec::{CurveKind, OperationSpec, VolSurfaceKind};
use finstack_core::currency::Currency;
use time::macros::date;

/// Metadata for the GFC 2008 template.
pub fn metadata() -> TemplateMetadata {
    TemplateMetadata {
        id: "gfc_2008".into(),
        name: "Global Financial Crisis 2008".into(),
        description: "Lehman Brothers collapse and ensuing global credit crisis. \
            Severe flight-to-quality in rates, massive credit spread widening, \
            equity crash, and volatility spike."
            .into(),
        event_date: date!(2008 - 09 - 15),
        asset_classes: vec![
            AssetClass::Rates,
            AssetClass::Credit,
            AssetClass::Equity,
            AssetClass::Volatility,
            AssetClass::FX,
        ],
        tags: vec![
            "systemic".into(),
            "credit".into(),
            "liquidity".into(),
            "historical".into(),
        ],
        severity: Severity::Severe,
        components: vec![
            "gfc_2008_rates".into(),
            "gfc_2008_credit".into(),
            "gfc_2008_equity".into(),
            "gfc_2008_vol".into(),
            "gfc_2008_fx".into(),
        ],
    }
}

/// Full GFC 2008 composite scenario.
pub fn gfc_2008() -> ScenarioSpecBuilder {
    ScenarioSpecBuilder::compose(vec![
        gfc_2008_rates(),
        gfc_2008_credit(),
        gfc_2008_equity(),
        gfc_2008_vol(),
        gfc_2008_fx(),
    ])
    .name("Global Financial Crisis 2008")
    .description("Lehman collapse — rates rally, credit blowout, equity crash, vol spike")
}

/// Rates component: flight-to-quality rally with bear steepening.
pub fn gfc_2008_rates() -> ScenarioSpecBuilder {
    ScenarioSpecBuilder::new("gfc_2008_rates")
        .name("GFC 2008 — Rates")
        .priority(0)
        .with_operations(vec![
            // Parallel rally: -200bp across discount curve
            OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Discount,
                curve_id: "USD-SOFR".into(),
                bp: -200.0,
            },
            // Bear steepener: short end rallies more, long end sells off
            // Net effect: 2s10s widens ~100bp
            OperationSpec::CurveNodeBp {
                curve_kind: CurveKind::Discount,
                curve_id: "USD-SOFR".into(),
                nodes: vec![
                    ("2Y".into(), -50.0),  // Additional rally at 2Y
                    ("5Y".into(), 0.0),    // Neutral at 5Y
                    ("10Y".into(), 50.0),  // Sell-off at 10Y
                    ("30Y".into(), 75.0),  // More sell-off at 30Y
                ],
                match_mode: crate::TenorMatchMode::Interpolate,
            },
        ])
}

/// Credit component: severe spread widening across IG and HY.
pub fn gfc_2008_credit() -> ScenarioSpecBuilder {
    ScenarioSpecBuilder::new("gfc_2008_credit")
        .name("GFC 2008 — Credit")
        .priority(1)
        .with_operations(vec![
            // IG spreads: +300bp
            OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::ParCDS,
                curve_id: "USD-IG".into(),
                bp: 300.0,
            },
            // HY spreads: +800bp
            OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::ParCDS,
                curve_id: "USD-HY".into(),
                bp: 800.0,
            },
        ])
}

/// Equity component: broad equity crash.
pub fn gfc_2008_equity() -> ScenarioSpecBuilder {
    ScenarioSpecBuilder::new("gfc_2008_equity")
        .name("GFC 2008 — Equity")
        .priority(2)
        .with_operations(vec![
            OperationSpec::EquityPricePct {
                ids: vec!["SPX".into()],
                pct: -50.0,
            },
            OperationSpec::EquityPricePct {
                ids: vec!["RTY".into()],
                pct: -55.0,
            },
        ])
}

/// Volatility component: massive equity vol spike.
pub fn gfc_2008_vol() -> ScenarioSpecBuilder {
    ScenarioSpecBuilder::new("gfc_2008_vol")
        .name("GFC 2008 — Volatility")
        .priority(3)
        .with_operation(OperationSpec::VolSurfaceParallelPct {
            surface_kind: VolSurfaceKind::Equity,
            surface_id: "SPX_VOL".into(),
            pct: 200.0, // ~40 vol points from ~20 base = +200%
        })
}

/// FX component: USD strengthening (flight to safety).
pub fn gfc_2008_fx() -> ScenarioSpecBuilder {
    ScenarioSpecBuilder::new("gfc_2008_fx")
        .name("GFC 2008 — FX")
        .priority(4)
        .with_operations(vec![
            OperationSpec::MarketFxPct {
                base: Currency::EUR,
                quote: Currency::USD,
                pct: -10.0, // EUR weakens 10% vs USD
            },
            OperationSpec::MarketFxPct {
                base: Currency::GBP,
                quote: Currency::USD,
                pct: -25.0, // GBP weakens 25% vs USD
            },
        ])
}
```

Wire into `finstack/scenarios/src/templates/mod.rs`:

```rust
pub mod gfc_2008;
```

Update `register_builtins` in `mod.rs`:

```rust
fn register_builtins(registry: &mut TemplateRegistry) {
    registry.register_with_components(
        gfc_2008::metadata(),
        gfc_2008::gfc_2008,
        vec![
            ("gfc_2008_rates".into(), Box::new(gfc_2008::gfc_2008_rates)),
            ("gfc_2008_credit".into(), Box::new(gfc_2008::gfc_2008_credit)),
            ("gfc_2008_equity".into(), Box::new(gfc_2008::gfc_2008_equity)),
            ("gfc_2008_vol".into(), Box::new(gfc_2008::gfc_2008_vol)),
            ("gfc_2008_fx".into(), Box::new(gfc_2008::gfc_2008_fx)),
        ],
    );
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p finstack-scenarios --lib templates::gfc_2008::tests`
Expected: PASS (all 9 tests)

- [ ] **Step 5: Commit**

```bash
git add finstack/scenarios/src/templates/gfc_2008.rs finstack/scenarios/src/templates/mod.rs
git commit -m "feat(scenarios): add GFC 2008 stress template with 5 composable components"
```

---

## Task 5: COVID 2020 Template

**Files:**
- Create: `finstack/scenarios/src/templates/covid_2020.rs`
- Modify: `finstack/scenarios/src/templates/mod.rs` — add module and wire into `register_builtins`

**Context:** COVID-19 March 2020 liquidity crisis. Rates -150bp bull flattener; Credit +200bp IG, +600bp HY; Equity -34% SPX; Vol +250% (VIX from ~20 to ~82); FX DXY +5%, EM FX -10%. Follow the exact same pattern as `gfc_2008.rs`.

- [ ] **Step 1: Write failing tests**

Create `finstack/scenarios/src/templates/covid_2020.rs` following the same test pattern as GFC 2008 (test full build, metadata, each component, composability, overrides).

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p finstack-scenarios --lib templates::covid_2020::tests`
Expected: FAIL

- [ ] **Step 3: Implement COVID 2020 template**

Operations:
- **Rates:** `CurveParallelBp { Discount, "USD-SOFR", -150.0 }` + `CurveNodeBp` for bull flattener (front end drops more: 2Y -30bp, 10Y +10bp, 30Y +20bp)
- **Credit:** `CurveParallelBp { ParCDS, "USD-IG", 200.0 }` + `CurveParallelBp { ParCDS, "USD-HY", 600.0 }`
- **Equity:** `EquityPricePct { ["SPX"], -34.0 }`
- **Vol:** `VolSurfaceParallelPct { Equity, "SPX_VOL", 250.0 }` (~VIX tripled)
- **FX:** `MarketFxPct { EUR, USD, -5.0 }` + `MarketFxPct { BRL, USD, -15.0 }`

Wire into `register_builtins` in `mod.rs`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p finstack-scenarios --lib templates::covid_2020::tests`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add finstack/scenarios/src/templates/covid_2020.rs finstack/scenarios/src/templates/mod.rs
git commit -m "feat(scenarios): add COVID 2020 stress template"
```

---

## Task 6: Rate Shock 2022 Template

**Files:**
- Create: `finstack/scenarios/src/templates/rate_shock_2022.rs`
- Modify: `finstack/scenarios/src/templates/mod.rs`

**Context:** 2022 Fed hiking cycle. Unique because rates RISE (unlike GFC/COVID). Rates +300bp; Credit +100bp IG, +200bp HY; Equity -25% SPX, -33% NDX; Vol +10pts (~50% increase); FX DXY +15%, EUR/USD -15%.

- [ ] **Step 1: Write failing tests**

Same test pattern: full build, metadata, components, composability.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p finstack-scenarios --lib templates::rate_shock_2022::tests`

- [ ] **Step 3: Implement Rate Shock 2022 template**

Operations:
- **Rates:** `CurveParallelBp { Discount, "USD-SOFR", 300.0 }` (positive — rates up)
- **Credit:** `CurveParallelBp { ParCDS, "USD-IG", 100.0 }` + `CurveParallelBp { ParCDS, "USD-HY", 200.0 }`
- **Equity:** `EquityPricePct { ["SPX"], -25.0 }` + `EquityPricePct { ["NDX"], -33.0 }`
- **Vol:** `VolSurfaceParallelPct { Equity, "SPX_VOL", 50.0 }`
- **FX:** `MarketFxPct { EUR, USD, -15.0 }`

Wire into `register_builtins`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p finstack-scenarios --lib templates::rate_shock_2022::tests`

- [ ] **Step 5: Commit**

```bash
git add finstack/scenarios/src/templates/rate_shock_2022.rs finstack/scenarios/src/templates/mod.rs
git commit -m "feat(scenarios): add 2022 rate shock stress template"
```

---

## Task 7: SVB 2023 Template

**Files:**
- Create: `finstack/scenarios/src/templates/svb_2023.rs`
- Modify: `finstack/scenarios/src/templates/mod.rs`

**Context:** SVB / regional banking crisis, March 2023. Unique because it uses `InstrumentSpreadBpByAttr` for sector-specific shocks. Rates: front-end rally -100bp with steepener; Credit: +150bp regional banks (attribute-based); Equity: -20% KRE, -5% SPX; Vol: +15pts; FX: minimal. Uses `IndexMap` from the `indexmap` crate for attribute maps (not `HashMap`).

- [ ] **Step 1: Write failing tests**

Same test pattern. Additional test: verify credit component uses `InstrumentSpreadBpByAttr` (not just `CurveParallelBp`).

```rust
#[test]
fn test_svb_2023_credit_uses_attribute_filter() {
    let spec = svb_2023_credit().build().expect("should build");
    let has_attr_shock = spec.operations.iter().any(|op| {
        matches!(op, OperationSpec::InstrumentSpreadBpByAttr { .. })
    });
    assert!(has_attr_shock, "SVB credit should use attribute-based spread shocks");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p finstack-scenarios --lib templates::svb_2023::tests`

- [ ] **Step 3: Implement SVB 2023 template**

Operations:
- **Rates:** `CurveParallelBp { Discount, "USD-SOFR", -100.0 }` + `CurveNodeBp` steepener (2Y -50bp, 10Y +25bp)
- **Credit:** `InstrumentSpreadBpByAttr { attrs: {"sector": "regional_banks"}, bp: 150.0 }` — uses `IndexMap` via `indexmap::indexmap!` macro or manual construction
- **Equity:** `EquityPricePct { ["KRE"], -20.0 }` + `EquityPricePct { ["SPX"], -5.0 }`
- **Vol:** `VolSurfaceParallelPct { Equity, "SPX_VOL", 75.0 }`
- **FX:** Minimal — single `MarketFxPct { EUR, USD, -1.0 }`

Wire into `register_builtins`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p finstack-scenarios --lib templates::svb_2023::tests`

- [ ] **Step 5: Commit**

```bash
git add finstack/scenarios/src/templates/svb_2023.rs finstack/scenarios/src/templates/mod.rs
git commit -m "feat(scenarios): add SVB 2023 stress template with attribute-based shocks"
```

---

## Task 8: LTCM 1998 Template

**Files:**
- Create: `finstack/scenarios/src/templates/ltcm_1998.rs`
- Modify: `finstack/scenarios/src/templates/mod.rs`

**Context:** LTCM / Russian default, August 1998. Flight to quality in UST, EM spread blowout, equity sell-off, vol spike, EM FX collapse. Uses `CurveKind::ParCDS` for credit, separate EM sovereign curve.

- [ ] **Step 1: Write failing tests**

Same test pattern as previous templates.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p finstack-scenarios --lib templates::ltcm_1998::tests`

- [ ] **Step 3: Implement LTCM 1998 template**

Operations:
- **Rates:** `CurveParallelBp { Discount, "USD-SOFR", -75.0 }` + `CurveNodeBp` steepener (2Y -25bp, 30Y +50bp) + `CurveParallelBp { ParCDS, "EM-SOVEREIGN", 200.0 }` (EM sovereign spread)
- **Credit:** `CurveParallelBp { ParCDS, "USD-IG", 100.0 }` + `CurveParallelBp { ParCDS, "EM-CORPORATE", 200.0 }`
- **Equity:** `EquityPricePct { ["SPX"], -20.0 }`
- **Vol:** `VolSurfaceParallelPct { Equity, "SPX_VOL", 100.0 }`
- **FX:** `MarketFxPct { BRL, USD, -30.0 }` + `MarketFxPct { RUB, USD, -50.0 }` — check if `Currency::BRL` and `Currency::RUB` exist; if not, use `Currency::from_str()` or skip those pairs and use available currencies

Wire into `register_builtins`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p finstack-scenarios --lib templates::ltcm_1998::tests`

- [ ] **Step 5: Commit**

```bash
git add finstack/scenarios/src/templates/ltcm_1998.rs finstack/scenarios/src/templates/mod.rs
git commit -m "feat(scenarios): add LTCM 1998 stress template"
```

---

## Task 9: Registry Default Integration + Cross-Template Tests

**Files:**
- Create: `finstack/scenarios/tests/templates/mod.rs`
- Create: `finstack/scenarios/tests/templates/registry_test.rs`
- Create: `finstack/scenarios/tests/templates/cross_template_test.rs`
- Modify: `finstack/scenarios/tests/mod.rs` — add `mod templates;`

**Context:** Integration tests that verify the full registry works end-to-end: `TemplateRegistry::default()` returns all 5 templates, filtering works, cross-event composition works, serde round-tripping works.

- [ ] **Step 1: Write registry integration tests**

Create `finstack/scenarios/tests/templates/registry_test.rs`:

```rust
//! Integration tests for the template registry.

use finstack_scenarios::templates::{AssetClass, Severity, TemplateRegistry};

#[test]
fn test_default_registry_contains_all_builtins() {
    let registry = TemplateRegistry::default();
    let templates = registry.list();
    assert_eq!(templates.len(), 5, "should have 5 built-in templates");

    let ids: Vec<&str> = templates.iter().map(|t| t.id.as_str()).collect();
    assert!(ids.contains(&"gfc_2008"));
    assert!(ids.contains(&"covid_2020"));
    assert!(ids.contains(&"rate_shock_2022"));
    assert!(ids.contains(&"svb_2023"));
    assert!(ids.contains(&"ltcm_1998"));
}

#[test]
fn test_default_registry_filter_by_severity() {
    let registry = TemplateRegistry::default();
    let severe = registry.filter_by_severity(Severity::Severe);
    // GFC and COVID are severe
    assert!(severe.len() >= 2);
}

#[test]
fn test_default_registry_filter_by_tag() {
    let registry = TemplateRegistry::default();
    let systemic = registry.filter_by_tag("systemic");
    assert!(!systemic.is_empty());
}

#[test]
fn test_default_registry_filter_by_asset_class() {
    let registry = TemplateRegistry::default();
    let rates = registry.filter_by_asset_class(AssetClass::Rates);
    // All 5 templates affect rates
    assert_eq!(rates.len(), 5);
}

#[test]
fn test_registry_get_and_build() {
    let registry = TemplateRegistry::default();
    let entry = registry.get("gfc_2008").expect("GFC template should exist");
    let spec = entry.builder().build().expect("should build");
    assert_eq!(spec.id, "gfc_2008");
    assert!(!spec.operations.is_empty());
}

#[test]
fn test_registry_component_access() {
    let registry = TemplateRegistry::default();
    let entry = registry.get("gfc_2008").expect("GFC template should exist");

    let rates = entry.component("gfc_2008_rates");
    assert!(rates.is_some(), "should have rates component");

    let spec = rates.expect("rates").build().expect("should build");
    assert!(!spec.operations.is_empty());
}

#[test]
fn test_registry_custom_template() {
    use finstack_scenarios::templates::{ScenarioSpecBuilder, TemplateMetadata};
    use time::macros::date;

    let mut registry = TemplateRegistry::default();
    registry.register(
        TemplateMetadata {
            id: "custom".into(),
            name: "Custom Scenario".into(),
            description: "A custom scenario".into(),
            event_date: date!(2025 - 01 - 01),
            asset_classes: vec![AssetClass::Rates],
            tags: vec!["custom".into()],
            severity: Severity::Mild,
            components: vec![],
        },
        || ScenarioSpecBuilder::new("custom"),
    );

    assert_eq!(registry.list().len(), 6); // 5 built-in + 1 custom
    assert!(registry.get("custom").is_some());
}
```

- [ ] **Step 2: Write cross-template composition tests**

Create `finstack/scenarios/tests/templates/cross_template_test.rs`:

```rust
//! Tests for composing operations across different historical templates.

use finstack_scenarios::templates::{self, ScenarioSpecBuilder};

#[test]
fn test_cross_event_composition() {
    // GFC credit + 2022 rate shock
    let spec = ScenarioSpecBuilder::compose(vec![
        templates::gfc_2008::gfc_2008_credit(),
        templates::rate_shock_2022::rate_shock_2022_rates(),
    ])
    .build()
    .expect("should build");

    let credit_ops = templates::gfc_2008::gfc_2008_credit()
        .build()
        .expect("build")
        .operations
        .len();
    let rates_ops = templates::rate_shock_2022::rate_shock_2022_rates()
        .build()
        .expect("build")
        .operations
        .len();

    assert_eq!(spec.operations.len(), credit_ops + rates_ops);
}

#[test]
fn test_serde_roundtrip_all_templates() {
    let registry = finstack_scenarios::templates::TemplateRegistry::default();
    for meta in registry.list() {
        let entry = registry.get(&meta.id).expect("should exist");
        let spec = entry.builder().build().expect("should build");

        let json = serde_json::to_string(&spec).expect("serialize");
        let deser: finstack_scenarios::ScenarioSpec =
            serde_json::from_str(&json).expect("deserialize");

        assert_eq!(spec.id, deser.id);
        assert_eq!(spec.operations.len(), deser.operations.len());
    }
}

#[test]
fn test_all_templates_pass_validation() {
    let registry = finstack_scenarios::templates::TemplateRegistry::default();
    for meta in registry.list() {
        let entry = registry.get(&meta.id).expect("should exist");
        // build() calls validate() internally
        let result = entry.builder().build();
        assert!(
            result.is_ok(),
            "template {} should validate: {:?}",
            meta.id,
            result.err()
        );
    }
}
```

Create `finstack/scenarios/tests/templates/mod.rs`:

```rust
mod cross_template_test;
mod registry_test;
```

Add to `finstack/scenarios/tests/mod.rs`:

```rust
mod templates;
```

- [ ] **Step 3: Run all template tests**

Run: `cargo test -p finstack-scenarios templates`
Expected: PASS (all tests)

- [ ] **Step 4: Commit**

```bash
git add finstack/scenarios/tests/templates/ finstack/scenarios/tests/mod.rs
git commit -m "test(scenarios): add integration tests for template registry and cross-template composition"
```

---

## Task 10: Final Cleanup and Full Test Suite

**Files:**
- Modify: `finstack/scenarios/src/lib.rs` — verify all re-exports are correct
- Modify: `finstack/scenarios/src/templates/mod.rs` — verify all modules wired

**Context:** Final pass to ensure everything compiles cleanly (`cargo clippy`), all tests pass, and documentation builds.

- [ ] **Step 1: Run clippy**

Run: `cargo clippy -p finstack-scenarios -- -D warnings`
Expected: PASS (no warnings)

- [ ] **Step 2: Run full test suite**

Run: `cargo test -p finstack-scenarios`
Expected: PASS (all existing + new tests)

- [ ] **Step 3: Run doc tests**

Run: `cargo test -p finstack-scenarios --doc`
Expected: PASS

- [ ] **Step 4: Verify docs build**

Run: `cargo doc -p finstack-scenarios --no-deps`
Expected: PASS

- [ ] **Step 5: Final commit if any cleanup was needed**

```bash
git add -A finstack/scenarios/
git commit -m "chore(scenarios): stress template library cleanup and final verification"
```
