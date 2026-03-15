# Market Data Hierarchy Design

**Date:** 2026-03-15
**Status:** Draft
**Crate:** `finstack-core` (primary), integration across `finstack-scenarios`, `finstack-valuations`, `finstack-portfolio`

## Problem

The current `MarketContext` uses flat `HashMap<CurveId, _>` storage. Scenarios and factor models must enumerate every `CurveId` they want to target. There is no way to express "shock all DM FX rates by -10%" or "apply +50bp to all BBB financial credit curves." Different institutions organize their market data differently — a multi-strat fund groups by desk, a macro fund by region, a credit fund by rating and sector.

## Goals

1. **Scenario/stress resolution** (primary) — Define shocks at any level of a hierarchy tree; the engine resolves them to individual curves via inheritance.
2. **Factor model mapping** (primary) — Factor models reference hierarchy nodes to specify which curves carry a factor's loading; the hierarchy resolves to `CurveId` sets.
3. **Market data completeness tracking** (secondary) — The hierarchy defines what curves *should* exist; the `MarketContext` holds what *does* exist. The gap is the completeness report.

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Hierarchy shape | **Hybrid: tree + tags** | Tree gives deterministic inheritance for scenario/factor resolution. Tags enable cross-cutting queries (e.g., all BBB-rated curves regardless of region). |
| Inheritance semantics | **Configurable per-scenario** | `MostSpecificWins` (override) and `Cumulative` (additive). Real stress tests use both patterns. |
| Hierarchy definition | **Fully user-defined at runtime** | As a library, we don't impose structure. No default hierarchy shipped. Users build whatever tree fits their universe. |
| Integration with MarketContext | **Optional field** | `hierarchy: Option<MarketDataHierarchy>` inside `MarketContext`. Backwards compatible — existing code unaffected when `None`. |
| Factor model relationship | **Factors reference hierarchy nodes** | Factor models are separate structures that use hierarchy targets for resolution. Keeps statistical decomposition separate from organizational structure. |

## Architecture

### Core Data Structure

The hierarchy is a tree of nodes. Each node has a name, optional tags, child nodes, and leaf references (`CurveId`s pointing into `MarketContext`'s flat storage).

```
Rates
├── USD
│   ├── SOFR       → [USD-SOFR-3M, USD-SOFR-6M]
│   ├── OIS        → [USD-OIS]
│   └── Treasury   → [USD-TSY-ON, USD-TSY-3M, ...]
├── EUR
│   ├── ESTR       → [EUR-ESTR-3M]
│   └── Euribor    → [EUR-EURIBOR-6M]

Credit
├── US
│   ├── IG
│   │   ├── Financials  {sector=Financials, rating=A}  → [JPM-5Y, GS-5Y]
│   │   └── Technology  {sector=Technology, rating=AA}  → [AAPL-5Y, MSFT-5Y]
│   └── HY
│       └── Energy      {sector=Energy, rating=BB}      → [OXY-5Y]

FX
├── DM  {classification=developed}  → [EURUSD, GBPUSD, JPYUSD]
├── EM  {classification=emerging}   → [BRLUSD, ZARUSD, MXNUSD]
```

### Rust Types

```rust
/// A path through the hierarchy tree, e.g., ["Credit", "US", "IG", "Financials"].
pub type NodePath = Vec<String>;

pub struct HierarchyNode {
    pub name: String,
    pub tags: HashMap<String, String>,
    pub children: IndexMap<String, HierarchyNode>,
    pub curve_ids: Vec<CurveId>,
}

#[derive(Serialize, Deserialize)]
pub struct MarketDataHierarchy {
    roots: IndexMap<String, HierarchyNode>,
}
```

### Integration with MarketContext

```rust
pub struct MarketContext {
    // ... all existing fields unchanged ...
    hierarchy: Option<MarketDataHierarchy>,
}

impl MarketContext {
    pub fn hierarchy(&self) -> Option<&MarketDataHierarchy>;
    pub fn set_hierarchy(&mut self, h: MarketDataHierarchy);
}
```

All existing code that accesses `MarketContext` by `CurveId` continues to work unchanged.

## Resolution Engine

### Resolution Modes

```rust
pub enum ResolutionMode {
    /// Most specific matching node wins. Ford at Credit→US→BBB→Ford
    /// gets only the Ford-level shock, ignoring parent shocks.
    MostSpecificWins,

    /// Shocks accumulate walking down the tree. Ford gets the sum of
    /// Credit + US + BBB + Ford-level shocks.
    Cumulative,
}
```

### Targeting

```rust
pub enum OperationTarget {
    /// Existing: target a specific curve by ID.
    Direct(CurveId),

    /// Existing: target all curves of a kind.
    ByKind(CurveKind),

    /// New: target via hierarchy path, resolved at execution time.
    HierarchyNode {
        path: NodePath,
        tag_filter: Option<TagFilter>,
    },
}

pub struct TagFilter {
    pub predicates: Vec<TagPredicate>,
}

pub enum TagPredicate {
    Equals { key: String, value: String },
    In { key: String, values: Vec<String> },
    Exists { key: String },
}
```

### Resolution Process

Given operations targeting hierarchy nodes, resolve to per-curve shocks:

1. **Walk the tree** from root to each leaf (`CurveId`).
2. **Collect** all operations matching nodes along the path.
3. **Apply resolution mode:**
   - `MostSpecificWins`: take the deepest matching node's operation.
   - `Cumulative`: sum/compose all operations along the path.
4. **Return** `HashMap<CurveId, Vec<ResolvedOperation>>` — the scenario engine applies these to individual curves as it does today.

### Resolution API

```rust
impl MarketDataHierarchy {
    /// Resolve a hierarchy target to the set of CurveIds it covers.
    pub fn resolve(&self, target: &HierarchyTarget) -> Vec<CurveId>;

    /// Given operations at various hierarchy nodes, resolve to per-curve operations.
    pub fn resolve_operations(
        &self,
        ops: &[(HierarchyTarget, OperationSpec)],
        mode: ResolutionMode,
    ) -> HashMap<CurveId, Vec<OperationSpec>>;

    /// Get the full path from root to a specific curve.
    pub fn path_for_curve(&self, curve_id: &CurveId) -> Option<NodePath>;

    /// Find all curves matching a tag filter (cross-cutting query).
    pub fn query_by_tags(&self, filter: &TagFilter) -> Vec<CurveId>;
}
```

### Scenario Integration

`ScenarioSpec` gains a resolution mode field:

```rust
pub struct ScenarioSpec {
    // ... existing fields ...
    pub resolution_mode: ResolutionMode,  // default: MostSpecificWins
}
```

Existing `OperationSpec` variants (`CurveParallelBp`, `MarketFxPct`, etc.) work unchanged — they gain an alternative targeting mechanism via `OperationTarget::HierarchyNode`. The scenario engine checks for `market.hierarchy()` and resolves hierarchy-targeted operations before applying them.

### Factor Model Integration

Factor models reference hierarchy nodes to specify scope. They are defined separately from the hierarchy.

```rust
pub struct FactorDefinition {
    pub name: String,
    pub target: HierarchyTarget,
    pub loading: f64,  // or more complex loading structure
}
```

The hierarchy resolves `target` to a set of `CurveId`s. The factor model's statistical structure (covariance, PCA loadings, etc.) remains independent.

## Builder API

### Ergonomic Construction

```rust
let hierarchy = MarketDataHierarchy::builder()
    .add_node("Rates/USD/SOFR")
        .curve_ids(&["USD-SOFR-3M", "USD-SOFR-6M"])
    .add_node("Rates/USD/OIS")
        .curve_ids(&["USD-OIS"])
    .add_node("Credit/US/IG/Financials")
        .tag("rating", "A")
        .tag("sector", "Financials")
        .curve_ids(&["JPM-5Y", "GS-5Y"])
    .add_node("FX/DM")
        .tag("classification", "developed")
        .curve_ids(&["EURUSD", "GBPUSD", "JPYUSD"])
    .build()?;
```

The `/`-separated path syntax auto-creates intermediate nodes. Tags set at a node are inherited by children (a child can override). `build()` validates:
- No duplicate `CurveId` across different leaf positions (a curve has exactly one path).
- Warns on empty intermediate nodes with no children and no curves.

### Mutation API

```rust
impl MarketDataHierarchy {
    pub fn insert_curve(&mut self, path: &str, curve_id: CurveId);
    pub fn remove_curve(&mut self, curve_id: &CurveId) -> bool;
    pub fn move_curve(&mut self, curve_id: &CurveId, new_path: &str);
    pub fn set_tag(&mut self, path: &str, key: &str, value: &str);
    pub fn add_subtree(&mut self, path: &str, subtree: HierarchyNode);
    pub fn remove_subtree(&mut self, path: &str) -> Option<HierarchyNode>;
}
```

### JSON Serialization

The hierarchy is fully serializable. A firm defines their universe in a single config file:

```json
{
  "roots": {
    "Rates": {
      "tags": {},
      "curves": [],
      "children": {
        "USD": {
          "tags": { "currency": "USD" },
          "curves": [],
          "children": {
            "SOFR": {
              "tags": { "index": "SOFR" },
              "curves": ["USD-SOFR-3M", "USD-SOFR-6M"],
              "children": {}
            },
            "OIS": {
              "curves": ["USD-OIS"],
              "children": {}
            }
          }
        }
      }
    },
    "Credit": {
      "children": {
        "US": {
          "tags": { "region": "US" },
          "children": {
            "IG": {
              "tags": { "grade": "investment" },
              "children": {
                "Financials": {
                  "tags": { "sector": "Financials" },
                  "curves": ["JPM-5Y", "GS-5Y"]
                }
              }
            }
          }
        }
      }
    },
    "FX": {
      "children": {
        "DM": {
          "tags": { "classification": "developed" },
          "curves": ["EURUSD", "GBPUSD", "JPYUSD"]
        },
        "EM": {
          "tags": { "classification": "emerging" },
          "curves": ["BRLUSD", "ZARUSD", "MXNUSD"]
        }
      }
    }
  }
}
```

## Market Data Loading Workflows

The hierarchy supports two distinct workflows for populating a `MarketContext`. Both are first-class; neither is preferred.

### Workflow 1: Quotes + Calibration Plan

The production workflow for institutions that calibrate curves from raw market observables. Uses the existing calibration engine.

```
hierarchy.json           → defines the universe (what curves exist, how organized)
calibration_plan.json    → defines how to build them (quotes → curves via bootstrap/solver)
daily_quotes/            → refreshes daily (deposit rates, swap rates, CDS spreads, etc.)

  ┌──────────────┐    ┌──────────────────┐    ┌──────────────────┐
  │ hierarchy     │    │ calibration plan  │    │ daily quotes     │
  │ (structure)   │    │ (how to build)    │    │ (market values)  │
  └──────┬────────┘    └────────┬──────────┘    └────────┬─────────┘
         │                      │                        │
         │                      └────────┬───────────────┘
         │                               ▼
         │                      ┌─────────────────┐
         │                      │ Calibration      │
         │                      │ Engine           │
         │                      └────────┬─────────┘
         │                               ▼
         │                      ┌─────────────────┐
         ▼                      │ MarketContext    │
  ┌──────────────┐              │ (calibrated     │
  │ attach       │─────────────▶│  curves + hier) │
  │ hierarchy    │              └─────────────────┘
  └──────────────┘
```

The calibration plan's `CalibrationStep` entries each produce a curve with a `curve_id`. These are the same `CurveId`s referenced in the hierarchy. The hierarchy does not need to know *how* curves are built — only *what* they are and *where they sit* in the tree.

**Key property:** The hierarchy JSON and calibration plan JSON are relatively stable (change when onboarding new issuers or restructuring). Daily quotes are the only thing that refreshes.

### Workflow 2: Pre-Calibrated Curves

For institutions that receive pre-built curves from external systems, or for loading saved snapshots (e.g., end-of-day state for next-day attribution).

```
hierarchy.json           → defines the universe
curves/*.json            → pre-calibrated DiscountCurve, HazardCurve, etc.
surfaces/*.json          → pre-calibrated VolSurface, FxDeltaVolSurface, etc.
prices/*.json            → MarketScalar spot prices
fx/fx_matrix.json        → FxMatrix cross rates

  ┌──────────────┐    ┌───────────────────────────────────┐
  │ hierarchy     │    │ pre-calibrated market data files   │
  │ (structure)   │    │ (curves, surfaces, prices, FX)     │
  └──────┬────────┘    └─────────────────┬─────────────────┘
         │                               │
         │                               ▼
         │                      ┌─────────────────┐
         │                      │ Load into        │
         │                      │ MarketContext     │
         │                      └────────┬─────────┘
         │                               ▼
         │                      ┌─────────────────┐
         ▼                      │ MarketContext    │
  ┌──────────────┐              │ (loaded curves  │
  │ attach       │─────────────▶│  + hierarchy)   │
  │ hierarchy    │              └─────────────────┘
  └──────────────┘
```

All curve types already derive `Serialize`/`Deserialize`. The library provides convenience methods for directory-based loading:

```rust
impl MarketContext {
    pub fn load_hierarchy(&mut self, path: &Path) -> Result<()>;
    pub fn load_curves_from_dir(&mut self, path: &Path) -> Result<()>;
    pub fn load_surfaces_from_dir(&mut self, path: &Path) -> Result<()>;
    pub fn load_prices_from_dir(&mut self, path: &Path) -> Result<()>;
    pub fn load_fx_matrix(&mut self, path: &Path) -> Result<()>;
}
```

### Workflow Compatibility

Both workflows produce the same end state: a `MarketContext` with populated curves and an attached `MarketDataHierarchy`. All downstream consumers (scenarios, attribution, factor models, portfolio valuation) are agnostic to which workflow was used.

The completeness report works identically in both cases — it compares the hierarchy's declared `CurveId`s against what is actually present in the `MarketContext`.

## Completeness Tracking

```rust
pub struct CompletenessReport {
    /// Curves in hierarchy that are missing from MarketContext.
    pub missing: Vec<(NodePath, CurveId)>,

    /// Curves in MarketContext that aren't in any hierarchy node.
    pub unclassified: Vec<CurveId>,

    /// Summary stats per subtree.
    pub coverage: Vec<SubtreeCoverage>,
}

pub struct SubtreeCoverage {
    pub path: NodePath,
    pub total_expected: usize,
    pub total_present: usize,
    pub percent: f64,
}

impl MarketContext {
    pub fn completeness_report(&self) -> Option<CompletenessReport>;
}
```

Use cases:
- **Pre-valuation checks:** "Do I have all curves before pricing this portfolio?"
- **Data pipeline monitoring:** "Which curves failed to load today?"
- **Scenario validation:** "Does my stress scenario cover all curves, or are some unshocked?"

## Cross-Crate Boundaries

### File Layout

```
finstack-core/src/market_data/
├── hierarchy/
│   ├── mod.rs          — MarketDataHierarchy, HierarchyNode, NodePath
│   ├── builder.rs      — HierarchyBuilder
│   ├── resolution.rs   — ResolutionMode, resolve logic, TagFilter
│   └── completeness.rs — CompletenessReport, coverage checks
├── context/
│   └── mod.rs          — MarketContext gains hierarchy: Option<MarketDataHierarchy>
```

The hierarchy lives in `finstack-core` because it is a market data concept. The resolution engine is pure data transformation (tree walk → flat map) with no pricing logic.

### Integration Points

| Crate | Integration | Change Scope |
|-------|------------|--------------|
| `finstack-core` | `MarketContext` gains `hierarchy` field; new `hierarchy/` module | New module + minor field addition |
| `finstack-scenarios` | Scenario engine resolves `HierarchyNode` targets before applying operations | Additive — existing direct/by-kind targeting unchanged |
| `finstack-valuations` | Attribution can scope bumps to subtrees for sub-factor P&L decomposition | Future enhancement, not day-one |
| `finstack-portfolio` | No changes needed; position tags and hierarchy tags are complementary | None |

### Backwards Compatibility

- `MarketContext` with `hierarchy: None` behaves identically to today.
- All existing `CurveId`-based lookups unchanged.
- Existing `ScenarioSpec` with direct `CurveId` targeting unchanged.
- The hierarchy is opt-in at every level.
