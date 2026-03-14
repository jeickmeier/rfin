# Stress Test Template Library — Design Specification

## Overview

A historical stress template library providing pre-built `ScenarioSpec` templates for major financial crises, with a parameterized builder API and optional dynamic registry for discoverability. The entire feature is additive — no changes to the existing scenario engine, specs, or portfolio pipeline.

## Architecture: Module Functions + Separate Registry (Approach 2)

Templates are plain factory functions in a `templates` submodule. An optional `TemplateRegistry` indexes them for programmatic discovery. Users can call functions directly or go through the registry — both paths produce `ScenarioSpec` via `ScenarioSpecBuilder`.

```
┌─────────────────────────────────────────────────────┐
│                  User Code                          │
│                                                     │
│  templates::gfc_2008()          registry.get("...")  │
│       │                              │               │
│       ▼                              ▼               │
│  ScenarioSpecBuilder ◄──── TemplateRegistry          │
│       │                     (optional, dynamic)      │
│       │ .override_curve()                            │
│       │ .build()                                     │
│       ▼                                              │
│  ScenarioSpec  ──► ScenarioEngine::apply()           │
│                ──► portfolio::apply_and_revalue()     │
│                ──► serde_json::to_string()           │
└─────────────────────────────────────────────────────┘
```

## Component 1: ScenarioSpecBuilder

The core primitive. Every template factory returns one.

### Structure

```rust
pub struct ScenarioSpecBuilder {
    id: String,
    name: Option<String>,
    description: Option<String>,
    operations: Vec<OperationSpec>,
    priority: i32,
    curve_overrides: HashMap<String, String>,    // default_id → user_id
    equity_overrides: HashMap<String, String>,   // default_id → user_id
    fx_overrides: HashMap<(Currency, Currency), (Currency, Currency)>,
}
```

### API

```rust
impl ScenarioSpecBuilder {
    pub fn new(id: impl Into<String>) -> Self;

    // Metadata
    pub fn name(mut self, name: impl Into<String>) -> Self;
    pub fn description(mut self, desc: impl Into<String>) -> Self;
    pub fn priority(mut self, priority: i32) -> Self;

    // Override parameterization
    pub fn override_curve(mut self, default_id: &str, user_id: &str) -> Self;
    pub fn override_equity(mut self, default_id: &str, user_id: &str) -> Self;
    pub fn override_fx(mut self, default: (Currency, Currency), user: (Currency, Currency)) -> Self;

    // Operations
    pub fn with_operation(mut self, op: OperationSpec) -> Self;
    pub fn with_operations(mut self, ops: Vec<OperationSpec>) -> Self;

    // Composition
    pub fn compose(builders: Vec<ScenarioSpecBuilder>) -> Self;

    // Terminal — resolves overrides, validates, returns ScenarioSpec
    pub fn build(self) -> Result<ScenarioSpec>;
}
```

### Override Resolution

`build()` walks all operations and substitutes IDs:
- `CurveParallelBp { curve_id }` → check `curve_overrides`
- `CurveNodeBp { curve_id }` → check `curve_overrides`
- `EquityPricePct { ids }` → check `equity_overrides` for each ID
- `MarketFxPct { base, quote }` → check `fx_overrides`
- `VolSurfaceParallelPct { surface_id }` → check `curve_overrides` (same namespace)

After substitution, calls `ScenarioSpec::validate()`.

## Component 2: Template Modules

### File Layout

```
finstack/scenarios/src/
├── templates/
│   ├── mod.rs              // Re-exports, TemplateMetadata, AssetClass, Severity
│   ├── builder.rs          // ScenarioSpecBuilder implementation
│   ├── registry.rs         // TemplateRegistry implementation
│   ├── gfc_2008.rs         // Global Financial Crisis
│   ├── covid_2020.rs       // COVID-19 March 2020
│   ├── rate_shock_2022.rs  // 2022 Fed hiking cycle
│   ├── svb_2023.rs         // SVB / regional banking crisis
│   └── ltcm_1998.rs        // LTCM / Russian default
```

### Per-Template Pattern

Each file exposes:

```rust
pub fn metadata() -> TemplateMetadata;

// Full composite — internally composes the blocks below
pub fn gfc_2008() -> ScenarioSpecBuilder;

// Composable sub-components by asset class
pub fn gfc_2008_rates() -> ScenarioSpecBuilder;
pub fn gfc_2008_credit() -> ScenarioSpecBuilder;
pub fn gfc_2008_equity() -> ScenarioSpecBuilder;
pub fn gfc_2008_vol() -> ScenarioSpecBuilder;
pub fn gfc_2008_fx() -> ScenarioSpecBuilder;
```

### Historical Shock Magnitudes (Defaults)

#### GFC 2008 (Lehman collapse, Sep 2008 – Mar 2009)

- **Rates:** -200bp parallel (flight to quality), bear steepener +100bp 2s10s
- **Credit:** +300bp IG spreads, +800bp HY spreads via `CurveParallelBp` on ParCDS curves
- **Equity:** -50% SPX, -55% broad equity
- **Volatility:** +40 vol points on equity vol surfaces
- **FX:** EUR/USD -10%, USD strengthening broadly

#### COVID 2020 (March 2020 liquidity crisis)

- **Rates:** -150bp parallel, bull flattener
- **Credit:** +200bp IG, +600bp HY
- **Equity:** -34% SPX
- **Volatility:** +50 vol points (VIX to 82)
- **FX:** DXY +5%, EM FX -10%

#### Rate Shock 2022 (Fed hiking cycle)

- **Rates:** +300bp parallel shift
- **Credit:** +100bp IG, +200bp HY
- **Equity:** -25% SPX, -33% NDX
- **Volatility:** +10 vol points
- **FX:** DXY +15%, EUR/USD -15%

#### SVB 2023 (Regional banking crisis, March 2023)

- **Rates:** -100bp front end, steepener (flight to safety in short duration)
- **Credit:** +150bp regional bank spreads via `InstrumentSpreadBpByAttr`
- **Equity:** -20% KRE (regional banks), -5% SPX
- **Volatility:** +15 vol points
- **FX:** minimal

#### LTCM 1998 (Russian default / LTCM unwind)

- **Rates:** -75bp UST (flight to quality), steepener, +200bp EM sovereign
- **Credit:** +200bp EM spreads, +100bp corporate
- **Equity:** -20% broad equity
- **Volatility:** +20 vol points
- **FX:** EM FX -15% (RUB, BRL, etc.)

## Component 3: TemplateMetadata

```rust
pub struct TemplateMetadata {
    pub id: String,                        // "gfc_2008"
    pub name: String,                      // "Global Financial Crisis 2008"
    pub description: String,               // Narrative of the event
    pub event_date: time::Date,            // Primary event date (e.g., 2008-09-15)
    pub asset_classes: Vec<AssetClass>,    // Affected asset classes
    pub tags: Vec<String>,                 // Freeform tags for filtering
    pub severity: Severity,               // Severity classification
    pub components: Vec<String>,           // Sub-component IDs
}

pub enum Severity { Mild, Moderate, Severe }

pub enum AssetClass { Rates, Credit, Equity, FX, Volatility, Commodity }
```

## Component 4: TemplateRegistry

### Structure

```rust
pub struct TemplateRegistry {
    entries: HashMap<String, RegisteredTemplate>,
}

struct RegisteredTemplate {
    metadata: TemplateMetadata,
    factory: Box<dyn Fn() -> ScenarioSpecBuilder + Send + Sync>,
    components: HashMap<String, Box<dyn Fn() -> ScenarioSpecBuilder + Send + Sync>>,
}
```

### API

```rust
impl TemplateRegistry {
    /// Pre-loaded with all built-in historical templates
    pub fn default() -> Self;

    /// Empty registry
    pub fn new() -> Self;

    /// Register a custom template
    pub fn register(
        &mut self,
        metadata: TemplateMetadata,
        factory: impl Fn() -> ScenarioSpecBuilder + Send + Sync + 'static,
    ) -> &mut Self;

    /// Register with sub-components
    pub fn register_with_components(
        &mut self,
        metadata: TemplateMetadata,
        factory: impl Fn() -> ScenarioSpecBuilder + Send + Sync + 'static,
        components: Vec<(String, Box<dyn Fn() -> ScenarioSpecBuilder + Send + Sync>)>,
    ) -> &mut Self;

    /// Lookup by ID
    pub fn get(&self, id: &str) -> Option<&RegisteredTemplate>;

    /// List all template metadata
    pub fn list(&self) -> Vec<&TemplateMetadata>;

    /// Filter by tag
    pub fn filter_by_tag(&self, tag: &str) -> Vec<&TemplateMetadata>;

    /// Filter by asset class
    pub fn filter_by_asset_class(&self, ac: AssetClass) -> Vec<&TemplateMetadata>;

    /// Filter by severity
    pub fn filter_by_severity(&self, severity: Severity) -> Vec<&TemplateMetadata>;
}
```

## Integration Points

### No Changes Required

The template library is purely additive. No modifications to:
- `ScenarioEngine` or any adapter
- `ScenarioSpec` or `OperationSpec` enums
- `ExecutionContext` or `ApplicationReport`
- Portfolio valuation pipeline (`apply_scenario`, `apply_and_revalue`)

### Crate-Level Exports

In `finstack/scenarios/src/lib.rs`:

```rust
pub mod templates;
pub use templates::{
    TemplateRegistry, TemplateMetadata,
    ScenarioSpecBuilder, AssetClass, Severity,
};
```

### Portfolio Usage

```rust
use finstack_scenarios::templates;

let scenario = templates::gfc_2008()
    .override_curve("USD-SOFR", "MY_SOFR")
    .build()?;

let (valuation, report) = apply_and_revalue(&portfolio, &scenario, &market, &config)?;
```

### Cross-Event Composition

```rust
// GFC credit spreads + 2022 rate shock
let hybrid = ScenarioSpecBuilder::compose(vec![
    templates::gfc_2008_credit(),
    templates::rate_shock_2022_rates(),
]).build()?;
```

### Serde Round-Tripping

Templates produce standard `ScenarioSpec` which is `Serialize + Deserialize`:

```rust
let scenario = templates::gfc_2008().build()?;
let json = serde_json::to_string_pretty(&scenario)?;
```

### WASM / Python Bindings

The builder and registry are pure Rust with no engine dependency. Can be exposed through `finstack-wasm` and `finstack-py` as factory functions without binding architecture changes.

## Testing Strategy

1. **Builder tests** — Override resolution, composition, validation on `build()`
2. **Template tests** — Each template builds successfully, produces expected operation count and types
3. **Registry tests** — Registration, lookup, filtering by tag/asset-class/severity
4. **Integration tests** — Apply built-in templates to a mock `MarketContext` via `ScenarioEngine::apply()`
5. **Round-trip tests** — `build()` → serialize → deserialize → validate passes
