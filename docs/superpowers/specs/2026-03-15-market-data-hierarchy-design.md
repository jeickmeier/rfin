# Market Data Hierarchy Design

**Date:** 2026-03-15
**Status:** Draft
**Crate:** `finstack-core` (primary), integration across `finstack-scenarios`, `finstack-valuations`, `finstack-portfolio`

## Problem

The current `MarketContext` uses flat `HashMap<CurveId, _>` storage. Scenarios and factor models must enumerate every `CurveId` they want to target. There is no way to express "shock all DM FX rates by -10%" or "apply +50bp to all BBB financial credit curves." Different institutions organize their market data differently — a multi-strat fund groups by beta and idiosyncratic, a macro fund by macro factors, a credit fund by rating and sector.

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

## Market Data Coverage

The hierarchy organizes **all** market data types stored in `MarketContext`. Every `CurveId`-keyed entry can appear as a leaf in the hierarchy tree.

### Complete MarketContext Inventory

| Category | Storage Type | Example CurveIds | Hierarchy Placement |
|----------|-------------|------------------|---------------------|
| **Discount curves** | `CurveStorage::Discount` | `USD-OIS`, `EUR-ESTR` | Rates/{ccy}/{index} |
| **Forward curves** | `CurveStorage::Forward` | `USD-SOFR-3M`, `EUR-EURIBOR-6M` | Rates/{ccy}/{index} |
| **Hazard/credit curves** | `CurveStorage::Hazard` | `JPM-5Y`, `OXY-5Y` | Credit/{region}/{grade}/{sector} |
| **Inflation curves** | `CurveStorage::Inflation` | `USD-CPI-ZC`, `EUR-HICP-ZC` | Inflation/{ccy}/{index} |
| **Base correlation curves** | `CurveStorage::BaseCorrelation` | `CDX-IG-5Y-BASECORR` | Credit/Indices/{index} |
| **Price curves** (commodity/index fwd) | `CurveStorage::Price` | `WTI-FWD`, `GOLD-FWD` | Commodities/{sector}/{commodity} |
| **Vol index curves** | `CurveStorage::VolIndex` | `VIX-FWD`, `VSTOXX-FWD` | Volatility/Indices/{index} |
| **Equity/FX vol surfaces** | `VolSurface` | `SPX-VOL`, `AAPL-VOL` | Volatility/Equity/{underlying} |
| **FX delta vol surfaces** | `FxDeltaVolSurface` | `EURUSD-VOL`, `USDJPY-VOL` | Volatility/FX/{pair} |
| **Market scalar prices** | `MarketScalar` | `AAPL-SPOT`, `EURUSD-SPOT` | Equity/Prices/{sector} or FX/Spot/{classification} |
| **Time series** | `ScalarTimeSeries` | `SPX-HIST`, `VIX-HIST` | Series/{asset_class}/{name} |
| **Inflation indices** | `InflationIndex` | `USA-CPI-U`, `EUR-HICP` | Inflation/Indices/{region} |
| **Credit index data** | `CreditIndexData` | `CDX-IG-S42`, `ITRX-MAIN-S41` | Credit/Indices/{index} |
| **Dividend schedules** | `DividendSchedule` | `AAPL-DIVS`, `MSFT-DIVS` | Equity/Dividends/{sector} |
| **FX matrix** | `FxMatrix` (singleton) | N/A — not CurveId-keyed | Not in hierarchy (singleton) |
| **Collateral mappings** | `HashMap<String, CurveId>` | CSA codes → curve IDs | Not in hierarchy (references) |

**Note:** `FxMatrix` and collateral mappings are not `CurveId`-keyed and live outside the hierarchy. Everything else — all 14 `CurveId`-keyed data types — can be organized in the tree.

## Architecture

### Core Data Structure

The hierarchy is a tree of nodes. Each node has a name, optional tags, child nodes, and leaf references (`CurveId`s pointing into `MarketContext`'s flat storage). The example below shows a comprehensive hierarchy covering all major data types:

```
Rates
├── USD
│   ├── Discount
│   │   └── OIS        → [USD-OIS]
│   ├── Forward
│   │   ├── SOFR       → [USD-SOFR-3M, USD-SOFR-6M]
│   │   └── Treasury   → [USD-TSY-ON, USD-TSY-3M, ...]
├── EUR
│   ├── Discount
│   │   └── ESTR       → [EUR-ESTR]
│   └── Forward
│       └── Euribor    → [EUR-EURIBOR-6M]

Credit
├── US
│   ├── IG
│   │   ├── Financials  {sector=Financials, rating=A}  → [JPM-5Y, GS-5Y]
│   │   └── Technology  {sector=Technology, rating=AA}  → [AAPL-5Y, MSFT-5Y]
│   └── HY
│       └── Energy      {sector=Energy, rating=BB}      → [OXY-5Y]
├── EU
│   └── IG
│       └── Industrials {sector=Industrials, rating=A}  → [SIE-5Y]
├── Indices
│   ├── CDX-IG         → [CDX-IG-S42, CDX-IG-5Y-BASECORR]
│   └── ITRX-MAIN      → [ITRX-MAIN-S41]

FX
├── Spot
│   ├── DM  {classification=developed}  → [EURUSD-SPOT, GBPUSD-SPOT, JPYUSD-SPOT]
│   └── EM  {classification=emerging}   → [BRLUSD-SPOT, ZARUSD-SPOT, MXNUSD-SPOT]

Equity
├── Prices
│   ├── Technology  {sector=Technology}   → [AAPL-SPOT, MSFT-SPOT]
│   └── Financials  {sector=Financials}   → [JPM-SPOT, GS-SPOT]
├── Dividends
│   ├── Technology   → [AAPL-DIVS, MSFT-DIVS]
│   └── Financials   → [JPM-DIVS, GS-DIVS]

Volatility
├── Equity
│   ├── SingleName   → [AAPL-VOL, MSFT-VOL]
│   └── Index        → [SPX-VOL]
├── FX
│   ├── DM           → [EURUSD-VOL, USDJPY-VOL]
│   └── EM           → [USDBRL-VOL, USDZAR-VOL]
├── Swaption         → [USD-SWPTVOL, EUR-SWPTVOL]
├── Credit           → [CDX-IG-VOL]
└── Indices          → [VIX-FWD, VSTOXX-FWD]

Commodities
├── Energy
│   └── Crude        → [WTI-FWD, BRENT-FWD]
├── Metals
│   └── Precious     → [GOLD-FWD, SILVER-FWD]

Inflation
├── USD
│   ├── Curves       → [USD-CPI-ZC]
│   └── Indices      → [USA-CPI-U]
├── EUR
│   ├── Curves       → [EUR-HICP-ZC]
│   └── Indices      → [EUR-HICP]

Series
├── Equity           → [SPX-HIST, AAPL-HIST]
├── Volatility       → [VIX-HIST]
└── Macro            → [FED-FUNDS-HIST, US-GDP-HIST]
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
    // Rates: discount + forward curves
    .add_node("Rates/USD/Discount/OIS")
        .curve_ids(&["USD-OIS"])
    .add_node("Rates/USD/Forward/SOFR")
        .curve_ids(&["USD-SOFR-3M", "USD-SOFR-6M"])
    .add_node("Rates/EUR/Discount/ESTR")
        .curve_ids(&["EUR-ESTR"])

    // Credit: hazard curves, indices, base correlation
    .add_node("Credit/US/IG/Financials")
        .tag("rating", "A").tag("sector", "Financials")
        .curve_ids(&["JPM-5Y", "GS-5Y"])
    .add_node("Credit/Indices/CDX-IG")
        .curve_ids(&["CDX-IG-S42", "CDX-IG-5Y-BASECORR"])

    // FX: spot prices
    .add_node("FX/Spot/DM")
        .tag("classification", "developed")
        .curve_ids(&["EURUSD-SPOT", "GBPUSD-SPOT"])
    .add_node("FX/Spot/EM")
        .tag("classification", "emerging")
        .curve_ids(&["BRLUSD-SPOT", "ZARUSD-SPOT"])

    // Equity: spot prices + dividends
    .add_node("Equity/Prices/Technology")
        .tag("sector", "Technology")
        .curve_ids(&["AAPL-SPOT", "MSFT-SPOT"])
    .add_node("Equity/Dividends/Technology")
        .curve_ids(&["AAPL-DIVS", "MSFT-DIVS"])

    // Volatility: equity, FX, swaption surfaces + vol index curves
    .add_node("Volatility/Equity/SingleName")
        .curve_ids(&["AAPL-VOL", "MSFT-VOL"])
    .add_node("Volatility/FX/DM")
        .curve_ids(&["EURUSD-VOL", "USDJPY-VOL"])
    .add_node("Volatility/Swaption")
        .curve_ids(&["USD-SWPTVOL", "EUR-SWPTVOL"])
    .add_node("Volatility/Indices")
        .curve_ids(&["VIX-FWD", "VSTOXX-FWD"])

    // Commodities: forward price curves
    .add_node("Commodities/Energy/Crude")
        .curve_ids(&["WTI-FWD", "BRENT-FWD"])

    // Inflation: curves + index fixings
    .add_node("Inflation/USD/Curves")
        .curve_ids(&["USD-CPI-ZC"])
    .add_node("Inflation/USD/Indices")
        .curve_ids(&["USA-CPI-U"])

    // Time series
    .add_node("Series/Equity")
        .curve_ids(&["SPX-HIST", "AAPL-HIST"])
    .add_node("Series/Volatility")
        .curve_ids(&["VIX-HIST"])

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
      "children": {
        "USD": {
          "tags": { "currency": "USD" },
          "children": {
            "Discount": {
              "children": {
                "OIS": { "curves": ["USD-OIS"] }
              }
            },
            "Forward": {
              "children": {
                "SOFR": { "curves": ["USD-SOFR-3M", "USD-SOFR-6M"] },
                "Treasury": { "curves": ["USD-TSY-ON", "USD-TSY-3M"] }
              }
            }
          }
        },
        "EUR": {
          "tags": { "currency": "EUR" },
          "children": {
            "Discount": {
              "children": { "ESTR": { "curves": ["EUR-ESTR"] } }
            },
            "Forward": {
              "children": { "Euribor": { "curves": ["EUR-EURIBOR-6M"] } }
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
                  "tags": { "sector": "Financials", "rating": "A" },
                  "curves": ["JPM-5Y", "GS-5Y"]
                },
                "Technology": {
                  "tags": { "sector": "Technology", "rating": "AA" },
                  "curves": ["AAPL-5Y", "MSFT-5Y"]
                }
              }
            },
            "HY": {
              "tags": { "grade": "high_yield" },
              "children": {
                "Energy": {
                  "tags": { "sector": "Energy", "rating": "BB" },
                  "curves": ["OXY-5Y"]
                }
              }
            }
          }
        },
        "Indices": {
          "children": {
            "CDX-IG": { "curves": ["CDX-IG-S42", "CDX-IG-5Y-BASECORR"] },
            "ITRX-MAIN": { "curves": ["ITRX-MAIN-S41"] }
          }
        }
      }
    },
    "FX": {
      "children": {
        "Spot": {
          "children": {
            "DM": {
              "tags": { "classification": "developed" },
              "curves": ["EURUSD-SPOT", "GBPUSD-SPOT", "JPYUSD-SPOT"]
            },
            "EM": {
              "tags": { "classification": "emerging" },
              "curves": ["BRLUSD-SPOT", "ZARUSD-SPOT", "MXNUSD-SPOT"]
            }
          }
        }
      }
    },
    "Equity": {
      "children": {
        "Prices": {
          "children": {
            "Technology": {
              "tags": { "sector": "Technology" },
              "curves": ["AAPL-SPOT", "MSFT-SPOT"]
            },
            "Financials": {
              "tags": { "sector": "Financials" },
              "curves": ["JPM-SPOT", "GS-SPOT"]
            }
          }
        },
        "Dividends": {
          "children": {
            "Technology": { "curves": ["AAPL-DIVS", "MSFT-DIVS"] },
            "Financials": { "curves": ["JPM-DIVS", "GS-DIVS"] }
          }
        }
      }
    },
    "Volatility": {
      "children": {
        "Equity": {
          "children": {
            "SingleName": { "curves": ["AAPL-VOL", "MSFT-VOL"] },
            "Index": { "curves": ["SPX-VOL"] }
          }
        },
        "FX": {
          "children": {
            "DM": { "curves": ["EURUSD-VOL", "USDJPY-VOL"] },
            "EM": { "curves": ["USDBRL-VOL", "USDZAR-VOL"] }
          }
        },
        "Swaption": { "curves": ["USD-SWPTVOL", "EUR-SWPTVOL"] },
        "Credit": { "curves": ["CDX-IG-VOL"] },
        "Indices": { "curves": ["VIX-FWD", "VSTOXX-FWD"] }
      }
    },
    "Commodities": {
      "children": {
        "Energy": {
          "children": {
            "Crude": { "curves": ["WTI-FWD", "BRENT-FWD"] }
          }
        },
        "Metals": {
          "children": {
            "Precious": { "curves": ["GOLD-FWD", "SILVER-FWD"] }
          }
        }
      }
    },
    "Inflation": {
      "children": {
        "USD": {
          "children": {
            "Curves": { "curves": ["USD-CPI-ZC"] },
            "Indices": { "curves": ["USA-CPI-U"] }
          }
        },
        "EUR": {
          "children": {
            "Curves": { "curves": ["EUR-HICP-ZC"] },
            "Indices": { "curves": ["EUR-HICP"] }
          }
        }
      }
    },
    "Series": {
      "children": {
        "Equity": { "curves": ["SPX-HIST", "AAPL-HIST"] },
        "Volatility": { "curves": ["VIX-HIST"] },
        "Macro": { "curves": ["FED-FUNDS-HIST", "US-GDP-HIST"] }
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
hierarchy.json              → defines the universe (tree structure + CurveId assignments)
curves/*.json               → DiscountCurve, ForwardCurve, HazardCurve, InflationCurve,
                               BaseCorrelationCurve, PriceCurve, VolatilityIndexCurve
surfaces/*.json             → VolSurface, FxDeltaVolSurface
prices/*.json               → MarketScalar (spot prices, rates, factors)
series/*.json               → ScalarTimeSeries (historical data)
inflation_indices/*.json    → InflationIndex (CPI/RPI fixings with lag/seasonality)
credit_indices/*.json       → CreditIndexData (index hazard + base corr + issuer curves)
dividends/*.json            → DividendSchedule
fx/fx_matrix.json           → FxMatrix cross rates

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
    pub fn load_curves_from_dir(&mut self, path: &Path) -> Result<()>;       // all CurveStorage types
    pub fn load_surfaces_from_dir(&mut self, path: &Path) -> Result<()>;     // VolSurface + FxDeltaVolSurface
    pub fn load_prices_from_dir(&mut self, path: &Path) -> Result<()>;       // MarketScalar
    pub fn load_series_from_dir(&mut self, path: &Path) -> Result<()>;       // ScalarTimeSeries
    pub fn load_inflation_indices_from_dir(&mut self, path: &Path) -> Result<()>;  // InflationIndex
    pub fn load_credit_indices_from_dir(&mut self, path: &Path) -> Result<()>;     // CreditIndexData
    pub fn load_dividends_from_dir(&mut self, path: &Path) -> Result<()>;    // DividendSchedule
    pub fn load_fx_matrix(&mut self, path: &Path) -> Result<()>;             // FxMatrix
}
```

### Workflow Compatibility

Both workflows produce the same end state: a `MarketContext` with populated curves and an attached `MarketDataHierarchy`. All downstream consumers (scenarios, attribution, factor models, portfolio valuation) are agnostic to which workflow was used.

The completeness report works identically in both cases — it compares the hierarchy's declared `CurveId`s against what is actually present in the `MarketContext`.

## Completeness Tracking

```rust
pub struct CompletenessReport {
    /// CurveIds in hierarchy that are missing from MarketContext (across all data types).
    pub missing: Vec<(NodePath, CurveId)>,

    /// CurveIds in MarketContext that aren't in any hierarchy node.
    /// Covers all CurveId-keyed stores: curves, surfaces, prices, series,
    /// inflation_indices, credit_indices, dividends, fx_delta_vol_surfaces.
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
    /// Check hierarchy against all CurveId-keyed data in MarketContext.
    /// Searches across curves, surfaces, fx_delta_vol_surfaces, prices,
    /// series, inflation_indices, credit_indices, and dividends.
    pub fn completeness_report(&self) -> Option<CompletenessReport>;
}
```

The completeness check searches **all** `CurveId`-keyed stores in `MarketContext`, not just the `curves` map. A `CurveId` in the hierarchy is considered "present" if it exists in any of the 8 keyed stores (curves, surfaces, fx_delta_vol_surfaces, prices, series, inflation_indices, credit_indices, dividends). Conversely, the "unclassified" list reports any `CurveId` in any store that has no hierarchy entry.

Use cases:

- **Pre-valuation checks:** "Do I have all curves, surfaces, prices, and dividends before pricing?"
- **Data pipeline monitoring:** "Which of the 500 expected market data items failed to load today?"
- **Scenario validation:** "Does my stress scenario cover all curves, or are some falling through unshocked?"
- **Onboarding audit:** "I added 3 new issuers — are their hazard curves, spot prices, and dividend schedules all classified?"

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
