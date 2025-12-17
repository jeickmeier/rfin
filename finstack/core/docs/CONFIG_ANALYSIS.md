# FinstackConfig Usage Analysis and Recommendations

## Executive Summary

This document analyzes the current state of `FinstackConfig` usage across the finstack library and proposes extensions to create a more unified configuration system.

## V1 decision (no new wrapper)

- Keep **`finstack_core::config::FinstackConfig` as the single top-level config type**.
- Add an optional **`extensions` map** (namespaced, versioned keys → JSON objects).
- Crates own their section schemas; core stays rounding-focused.
- First adopter: **valuations calibration** reads `extensions["valuations.calibration.v2"]` for overrides; otherwise uses existing defaults.

Canonical v1 JSON shape:

```json
{
  "rounding": {
    "mode": "Bankers",
    "ingest_scale": { "overrides": { "JPY": 0 } },
    "output_scale": { "overrides": { "USD": 4 } }
  },
  "extensions": {
    "valuations.calibration.v2": {
      "tolerance": 1e-8,
      "max_iterations": 250,
      "use_parallel": true,
      "random_seed": null,
      "solver_kind": "newton",
      "rate_bounds_policy": "explicit",
      "rate_bounds": { "min_rate": -0.01, "max_rate": 0.10 },
      "calibration_method": { "global_solve": { "use_analytical_jacobian": true } }
    }
  }
}
```

## Current State

### Core Configuration (`FinstackConfig`)

Located in `finstack/core/src/config.rs`, `FinstackConfig` currently handles:
- **Rounding policies**: Mode (Bankers, AwayFromZero, etc.) and per-currency scales
- **Numeric mode**: Currently F64 only (compile-time constant)
- **Result metadata**: Stamping outputs with configuration snapshots

### Domain-Specific Configs

Multiple separate configuration structs exist:

1. **`CalibrationConfig`** (`finstack/valuations/src/calibration/config.rs`)
   - Solver settings (tolerance, max_iterations, solver_kind)
   - Parallel execution (`use_parallel`)
   - Random seed (`random_seed: Option<u64>`)
   - Validation mode (Warn vs Error)
   - Explanation options (`ExplainOpts`)
   - Multi-curve framework settings

2. **`AttributionConfig`** (`finstack/valuations/src/attribution/spec.rs`)
   - Tolerances (absolute, percentage)
   - Metrics selection
   - Strict validation flag

3. **`McEngineConfig`** (`finstack/valuations/src/instruments/common/models/monte_carlo/engine.rs`)
   - Number of paths, seed, time grid
   - Parallel execution (`use_parallel`)
   - Chunk size for parallel processing
   - Path capture configuration
   - Antithetic variance reduction

4. **`SensitivityConfig`** (`finstack/statements/src/analysis/types.rs`)
   - Analysis mode (Diagonal, FullGrid, Tornado)
   - Parameter specifications
   - Target metrics

5. **Various other configs**: `ValidationConfig`, `PathCaptureConfig`, `PathDependentPricerConfig`, etc.

## Issues Identified

### 1. Inconsistent Config Threading

**Problem**: Many functions don't accept `FinstackConfig` as a parameter, instead using hardcoded defaults.

**Evidence**: 109 instances of `FinstackConfig::default()` across 56 files.

**Examples**:
- `AttributionSpec::execute()` uses `FinstackConfig::default()` hardcoded (line 139)
- Many portfolio valuation functions don't accept config
- Pricing functions in `pricer.rs` don't accept config

**Impact**: Users cannot customize rounding/numeric behavior without modifying code.

### 2. Duplication of Settings

**Problem**: Common settings appear in multiple configs with different defaults.

**Duplicated Settings**:
- **Parallel execution**: `CalibrationConfig.use_parallel`, `McEngineConfig.use_parallel`, `PathDependentPricerConfig.use_parallel`
- **Random seeds**: `CalibrationConfig.random_seed`, `McEngineConfig.seed`, `PathDependentPricerConfig.seed` (all default to 42)
- **Chunk sizes**: `McEngineConfig.chunk_size`, `PathDependentPricerConfig.chunk_size` (both default to 1000)

**Impact**: Inconsistent behavior across domains, difficult to maintain global policies.

### 3. No Configuration Hierarchy

**Problem**: Domain configs don't reference or extend `FinstackConfig`.

**Impact**: 
- Cannot pass a single config object through the entire stack
- Must thread both `FinstackConfig` and domain configs separately
- No way to enforce global policies (e.g., "all parallel execution disabled")

## Recommendations

## Deeper proposal: standard config + per-crate extensions (superseded by v1 wrapper-less approach)

> The section below captured the earlier idea of introducing a separate `EngineConfig`
> envelope. We moved to a simpler v1 that keeps `FinstackConfig` as the single top-level
> config and adds an `extensions` map. The concepts here remain useful if we ever reintroduce
> a dedicated envelope.

You asked for two specific properties:

- **(a) Consolidate configs to a standard config for the library** (one thing you can pass around, stamp, and serialize).
- **(b) Allow config to be extended per module/crate** *without* having to declare all module fields in `finstack-core`.

The most robust way to satisfy both—while preserving finstack’s “stable schemas / deny unknown fields” philosophy—is a **two-layer config model**:

1. **A small, standardized “core envelope”** for cross-cutting settings.
2. **A namespaced, versioned “extensions map”** that stores per-crate config sections as JSON blobs (or similar), owned and validated by each crate.

This keeps `finstack-core` small and stable while allowing each crate to evolve its config independently (including generating JSON schemas for UI).

### 0) Terminology (recommended renames)

Right now `finstack-core::config::FinstackConfig` is effectively “numeric/rounding config”.

To make the API clearer when we introduce a “library-wide config”, we should consider:

- `finstack_core::config::FinstackConfig` → **`NumericConfig`** (or `CoreNumericConfig`)
- New library-wide config → **`EngineConfig`** (or `FinstackConfig` in the meta-crate)

This is optional but strongly reduces confusion, especially in Python/WASM where `FinstackConfig` is exposed today.

### 1) Standard library-wide config envelope

Define a single top-level config that all major entry points accept (pricing, portfolio valuation, statements evaluation, scenarios, calibration).

Proposed shape:

```rust
/// Library-wide configuration envelope.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineConfig {
    /// Cross-cutting numeric and IO precision policy.
    pub numeric: finstack_core::config::FinstackConfig,

    /// Cross-cutting execution policy (determinism, parallel toggle, default seeds).
    #[serde(default)]
    pub execution: ExecutionConfig,

    /// Cross-cutting validation policy (warning vs error) used by modules that opt in.
    #[serde(default)]
    pub validation: ValidationConfig,

    /// Per-crate extension blobs (namespaced keys -> JSON values).
    ///
    /// Each crate owns the schema for its keys and validates by deserializing into its local type.
    #[serde(default, skip_serializing_if = "ConfigExtensions::is_empty")]
    pub extensions: ConfigExtensions,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionConfig {
    /// Determinism knob: keep default `false` for parallel, consistent with current behavior.
    pub use_parallel: bool,
    /// Default seed used by stochastic engines if not overridden at the module level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_seed: Option<u64>,
    /// Default chunk size for parallel work partitioning (where relevant).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_chunk_size: Option<usize>,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            use_parallel: false,
            default_seed: Some(42),
            default_chunk_size: Some(1000),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationMode {
    Warn,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationConfig {
    pub mode: ValidationMode,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self { mode: ValidationMode::Error }
    }
}
```

Key point: **`EngineConfig` is small and stable** (core-ish), while module-specific configuration lives in `extensions`.

### 2) Per-crate extension mechanism (no core fields required)

Represent extensions as a deterministic, schematized map:

```rust
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ConfigExtensions {
    #[serde(flatten)]
    inner: BTreeMap<String, JsonValue>,
}

impl ConfigExtensions {
    pub fn is_empty(&self) -> bool { self.inner.is_empty() }

    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        self.inner.get(key)
    }

    pub fn insert(&mut self, key: impl Into<String>, value: JsonValue) -> Option<JsonValue> {
        self.inner.insert(key.into(), value)
    }
}
```

#### Typed access without registries (recommended ergonomic layer)

To avoid ad-hoc `serde_json::from_value(...)` calls scattered across crates, add a tiny trait
that *each crate implements for its own config section types*. This does **not** require core
to “know” the types; it just provides a uniform accessor.

```rust
/// Implemented in the crate that owns the config type.
pub trait ConfigSection: Sized + serde::de::DeserializeOwned {
    /// Globally unique, versioned key.
    const KEY: &'static str;
}

impl ConfigExtensions {
    pub fn get_typed<T: ConfigSection>(&self) -> Result<Option<T>, serde_json::Error> {
        let Some(raw) = self.get(T::KEY) else { return Ok(None) };
        serde_json::from_value(raw.clone()).map(Some)
    }
}
```

Where this should live:

- **Preferred**: a tiny new crate like `finstack-config` (depends on `serde` + `serde_json`),
  used by `finstack-core`, `finstack-valuations`, `finstack-statements`, etc.
- **Alternative**: in the `finstack` meta-crate (keeps core small; meta-crate already depends
  on the subcrates anyway).

This preserves your requirement (b): adding a new module config type does **not** require
adding a field to `finstack-core`; only the module crate defines its key + struct.

#### Namespacing + versioning rules (important)

To keep schemas stable and avoid collisions, require:

- Key format: **`"{crate}.{domain}.v{N}"`**
  - Examples:
    - `"valuations.calibration.v2"`
    - `"valuations.monte_carlo.v1"`
    - `"statements.monte_carlo.v1"`
    - `"portfolio.aggregation.v1"`
- The value must be a JSON object conforming to the crate-owned schema for that key.
- Each crate owns the migration from `v1` → `v2` etc. (core does not).

This provides stable wire formats without forcing `finstack-core` to import every crate.

#### Schema ownership + UI friendliness (recommended)

The “extensions map” pattern is UI-friendly because it is already JSON.
To keep schemas auditable and golden-testable:

- Each crate should publish a JSON schema per section version, e.g.:
  - `finstack/valuations/schemas/calibration/2/calibration_config_v2.schema.json`
  - `finstack/valuations/schemas/monte_carlo/1/monte_carlo_config_v1.schema.json`
- The schema should be generated from the Rust type when possible (or maintained by hand if needed),
  but the key requirement is that it is **versioned and stable**.

This aligns with existing practice in valuations where you already ship schemas under
`finstack/valuations/schemas/`.

#### Typed access pattern (lives in each crate, not core)

Each crate defines its config struct locally with `#[serde(deny_unknown_fields)]` and a constant key:

```rust
// in finstack-valuations
pub const CALIBRATION_CONFIG_KEY_V2: &str = "valuations.calibration.v2";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationConfigV2 {
    pub tolerance: f64,
    pub max_iterations: usize,
    #[serde(default)]
    pub use_parallel: Option<bool>,
    #[serde(default)]
    pub random_seed: Option<u64>,
    // ...
}

impl CalibrationConfigV2 {
    pub fn from_engine(cfg: &EngineConfig) -> Result<Self> {
        if let Some(raw) = cfg.extensions.get(CALIBRATION_CONFIG_KEY_V2) {
            Ok(serde_json::from_value(raw.clone())?)
        } else {
            Ok(Self::default_from_engine(cfg))
        }
    }

    fn default_from_engine(engine: &EngineConfig) -> Self {
        // unify defaults via engine.execution
        Self {
            tolerance: 1e-10,
            max_iterations: 100,
            use_parallel: Some(engine.execution.use_parallel),
            random_seed: engine.execution.default_seed,
            // ...
        }
    }
}
```

This satisfies (b): **module configs live in their module crate**, and only cross-cutting defaults are centralized.

### 3) Consolidation strategy (how to converge scattered configs into the standard)

We consolidate by introducing a single “config source of truth”:

- All public entry points accept `&EngineConfig`.
- Module-level configs become **derived views**:
  - Either read from `engine.extensions` (if explicitly supplied),
  - Or default from `engine.execution` / `engine.validation` + existing hardcoded defaults.

This removes:
- hardcoded `FinstackConfig::default()` in deeper call stacks
- repeated `seed=42`, `use_parallel=...`, `chunk_size=1000` defaults across crates

### 4) Why the extensions-map approach fits the existing UI/WASM model

Your current worker does essentially:

- accept `configJson?: string`
- `JSON.parse`
- extract a couple of keys

So the worker is already compatible with an **“engine config envelope”**: it can continue extracting `numeric.rounding...` (or whatever we standardize), while future UI features can pass sections like `"valuations.calibration.v2"` without changing the core WASM bindings.

### 4.1) Concrete “standard JSON” shape (what gets passed as `configJson`)

Recommended canonical JSON shape for `EngineConfig`:

```json
{
  "numeric": {
    "rounding": {
      "mode": "Bankers",
      "ingest_scale": { "overrides": { "JPY": 0 } },
      "output_scale": { "overrides": { "USD": 4 } }
    }
  },
  "execution": {
    "use_parallel": false,
    "default_seed": 42,
    "default_chunk_size": 1000
  },
  "validation": { "mode": "error" },
  "extensions": {
    "valuations.calibration.v2": {
      "tolerance": 1e-10,
      "max_iterations": 100,
      "solver_kind": "brent"
    },
    "valuations.monte_carlo.v1": {
      "num_paths": 100000,
      "antithetic": false
    }
  }
}
```

Notes:

- `numeric` reuses the existing stable serde shape in `finstack-core::config`.
- `extensions` is intentionally “open” at the top level, but each *section value* is strict via
  `#[serde(deny_unknown_fields)]` on the owning crate’s struct.

### 4.2) WASM/UI impact (how this changes the worker)

Today `packages/finstack-ui/src/workers/finstackEngine.ts` looks for things like:

- `cfgParsed.outputScale` or `cfgParsed.rounding.scale`
- `cfgParsed.roundingModeLabel` or `cfgParsed.rounding.label`

If we standardize on the new envelope, the worker should instead read:

- `cfgParsed.numeric.rounding.output_scale.overrides.USD` (or similar)
- `cfgParsed.numeric.rounding.mode`

This is a mechanical refactor and keeps the “config-as-JSON string” contract intact.
It also unlocks passing `"valuations.calibration.v2"` into the worker without the worker
having to understand calibration fields—it can simply forward the JSON to Rust/WASM entrypoints.

### 4.3) Python impact (parity story)

Python currently exposes `FinstackConfig` as the numeric/rounding config (`finstack-py/src/core/config.rs`).
With an `EngineConfig` envelope, you have two good options:

- **Option A (minimal change)**: keep Python `FinstackConfig` as “numeric config”, and introduce a new
  `EngineConfig`/`ConfigEnvelope` Python type later.
- **Option B (cleaner long-term)**: rename the Python-exposed rounding config to `NumericConfig` and
  add a new Python `FinstackConfig` that represents `EngineConfig` (breaking, but consistent).

Either way, the extensions-map means Python doesn’t need to have every module config eagerly typed; advanced
users can inject extension sections as dicts and let the Rust layer validate them.

### 5) What *not* to do (and why)

- **Don’t use a dynamic “typetag registry” in core** (e.g., `typetag` with trait objects): it introduces implicit global registration and can be harder to keep deterministic and schema-stable.
- **Don’t put every module config as a struct field in core**: it violates your requirement (b) and forces core to depend on every crate.

### 6) Migration path (minimal breakage, but clear direction)

1. Add `EngineConfig` + `ConfigExtensions` (probably in the `finstack` meta-crate or a small new crate like `finstack-config`).
2. Update top-level APIs (pricing/portfolio/scenarios/statements) to accept `&EngineConfig`.
3. In each crate, implement `FooConfigV1::from_engine(&EngineConfig)` that:
   - reads its section key from `extensions`,
   - otherwise defaults using `engine.execution/validation` + existing defaults.
4. Gradually eliminate internal `::default()` calls in favor of passed config.
5. Optionally later: re-shape Python/WASM “config” objects to represent `EngineConfig` (or keep them as helpers that only build the `numeric` section of the JSON envelope).

---

### Option 1: Extend `FinstackConfig` with Optional Performance Settings

Add optional performance-related fields to `FinstackConfig` that can be referenced by domain configs:

```rust
pub struct FinstackConfig {
    /// Detailed rounding policy (ingest/output scales by currency).
    pub rounding: RoundingPolicy,
    
    /// Optional performance settings (defaults to None for backward compatibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub performance: Option<PerformanceConfig>,
    
    /// Optional validation settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<ValidationConfig>,
    
    /// Optional explanation/debugging settings
    #[serde(skip)]
    pub explain: ExplainOpts,
}
```

**Pros**:
- Backward compatible (all fields optional)
- Single config object can be passed through stack
- Global policies can be enforced

**Cons**:
- Makes `FinstackConfig` larger
- Some settings may not apply to all domains

### Option 2: Composition Pattern

Keep `FinstackConfig` focused on core concerns, but have domain configs reference it:

```rust
pub struct CalibrationConfig {
    // ... existing fields ...
    
    /// Core finstack configuration (rounding, numeric mode)
    #[serde(default)]
    pub finstack: FinstackConfig,
}
```

**Pros**:
- Clear separation of concerns
- Domain configs remain focused
- Can still pass single config object

**Cons**:
- Requires updating all domain configs
- More verbose (need to access `config.finstack.rounding.mode`)

### Option 3: Global Performance Context (Recommended)

Create a separate `PerformanceContext` that can be shared across domains, while keeping `FinstackConfig` focused:

```rust
/// Global performance and execution settings
#[derive(Clone, Debug, Default)]
pub struct PerformanceContext {
    /// Global parallel execution policy
    pub use_parallel: bool,
    
    /// Default random seed (can be overridden per-domain)
    pub default_seed: Option<u64>,
    
    /// Default chunk size for parallel processing
    pub default_chunk_size: usize,
    
    /// Thread pool configuration
    pub thread_pool_size: Option<usize>,
}

/// Combined configuration context
pub struct ExecutionContext {
    pub finstack: FinstackConfig,
    pub performance: PerformanceContext,
}
```

**Pros**:
- Clear separation: `FinstackConfig` = numeric/rounding, `PerformanceContext` = execution
- Can be passed alongside `FinstackConfig` or composed
- Domain configs can reference both
- Minimal changes to existing code

**Cons**:
- Two objects to thread through (though can be composed)

## Proposed Extensions to `FinstackConfig`

Even if we keep domain configs separate, consider adding these to `FinstackConfig` for consistency:

### 1. Performance Settings (Optional)

```rust
pub struct PerformanceConfig {
    /// Default parallel execution policy (can be overridden per-domain)
    pub use_parallel: bool,
    
    /// Default random seed for reproducible results
    pub default_seed: Option<u64>,
    
    /// Default chunk size for parallel processing
    pub default_chunk_size: usize,
}
```

### 2. Validation Mode

```rust
pub enum ValidationMode {
    Warn,   // Emit warnings, continue
    Error,  // Fail on validation errors
}

pub struct FinstackConfig {
    // ... existing fields ...
    
    /// Validation behavior (default: Error for strictness)
    #[serde(default)]
    pub validation_mode: ValidationMode,
}
```

### 3. Explanation Options

```rust
pub struct FinstackConfig {
    // ... existing fields ...
    
    /// Explanation/debugging options (opt-in, zero overhead when disabled)
    #[serde(skip)]
    pub explain: ExplainOpts,
}
```

## Migration Strategy

1. **Phase 1**: Add optional fields to `FinstackConfig` (backward compatible)
2. **Phase 2**: Update domain configs to reference `FinstackConfig` where applicable
3. **Phase 3**: Update function signatures to accept `FinstackConfig` (with defaults for backward compatibility)
4. **Phase 4**: Deprecate hardcoded `FinstackConfig::default()` usage

## Consistency Checklist

To improve consistency, consider:

- [ ] All valuation functions should accept `&FinstackConfig` parameter
- [ ] All pricing functions should accept `&FinstackConfig` parameter  
- [ ] Domain configs should reference `FinstackConfig` for rounding/numeric settings
- [ ] Parallel execution settings should have a global default in `FinstackConfig`
- [ ] Random seed management should be centralized
- [ ] Validation modes should be consistent across domains
- [ ] Explanation options should be accessible from `FinstackConfig`

## Conclusion

The current configuration system works but has room for improvement:

1. **Inconsistency**: Config not consistently threaded through functions
2. **Duplication**: Common settings repeated across domain configs
3. **No hierarchy**: Domain configs don't reference core config

**Recommended approach**: 
- Keep `FinstackConfig` focused on core concerns (rounding, numeric mode)
- Create optional `PerformanceContext` for execution settings
- Update domain configs to compose both where needed
- Gradually migrate function signatures to accept config parameters

This maintains backward compatibility while enabling a more unified configuration system.

