# Statistical Risk Factor Model — Design Specification

**Date:** 2026-03-14
**Status:** Approved
**Scope:** Factor decomposition for cross-asset risk budgeting

## Overview

A flexible statistical risk factor model that decomposes portfolio risk into user-defined factors (rates, credit, equity, FX, volatility, etc.), enabling "what % of my risk is rates vs credit" analysis. The library provides the decomposition engine; users supply their own factor definitions, covariance matrices, and loadings (vendor-agnostic).

### Key Design Decisions

- **User-supplied factors** — the library does not estimate factors or covariances. Users bring their own (Barra, Axioma, PCA, proprietary).
- **Two pricing modes** — delta-based (fast, linear) and full repricing (captures nonlinearity/convexity).
- **Flexible factor matching** — three composable strategies (mapping table, cascade, hierarchical) for assigning instrument dependencies to factors.
- **Hybrid API** — declarative `FactorModelConfig` (serde-friendly, covers 90% of use cases) with trait escape hatches for custom logic.
- **Split across crates** — definitions in `core`, sensitivity computation in `valuations`, risk budgeting/what-if in `portfolio`.

### Crate Dependency Note

The `Attributes` type currently lives in `finstack/valuations`. The `FactorMatcher` trait in `core` needs access to instrument metadata for matching. To avoid a circular dependency, we will move `Attributes` (and its `tags`/`meta` fields) to `finstack/core` as a shared primitive, and re-export from `valuations` for backwards compatibility. This is a natural home — attributes are metadata, not pricing logic.

---

## Section 1: Core Factor Definitions (`finstack/core`)

Defines the vocabulary — what a factor is, how factors relate to market data, and how instruments get matched to factors. No pricing logic.

### Key Types

#### `FactorId`

Newtype string identifier for a factor (e.g., `"USD-Rates"`, `"NA-Energy-CCC-Spread"`).

#### `FactorType`

Broad classification:

```rust
pub enum FactorType {
    Rates,
    Credit,
    Equity,
    FX,
    Volatility,
    Commodity,
    Inflation,
    Custom(String),
}
```

#### `MarketDependency`

A single dependency extracted from an instrument's `MarketDependencies` (plural). The existing `MarketDependencies` struct aggregates all dependencies; this enum represents one individual dependency for matching:

```rust
pub enum MarketDependency {
    /// Discount, forward, or other rate curve
    Curve { id: CurveId, curve_type: CurveType },
    /// Credit/hazard curve
    CreditCurve { id: CurveId },
    /// Equity or commodity spot price
    Spot { id: String },
    /// Volatility surface
    VolSurface { id: String },
    /// FX pair
    FxPair { base: Currency, quote: Currency },
    /// Time series (e.g., inflation index)
    Series { id: String },
}

pub enum CurveType {
    Discount,
    Forward,
    Hazard,
    Inflation,
    BaseCorrelation,
}
```

A utility function `decompose(deps: &MarketDependencies) -> Vec<MarketDependency>` flattens the existing struct into individual entries for the matcher.

#### `FactorDefinition`

Describes a single factor:

```rust
pub struct FactorDefinition {
    pub id: FactorId,
    pub factor_type: FactorType,
    pub market_mapping: MarketMapping,
    pub description: Option<String>,
}
```

#### `MarketMapping`

How a factor movement translates to market data changes — the bridge between abstract factor and concrete curve bumps:

```rust
#[derive(Serialize, Deserialize)]
pub enum MarketMapping {
    /// Parallel shift to a set of curves
    CurveParallel { curve_ids: Vec<CurveId>, units: BumpUnits },
    /// Bucketed shifts (key-rate style) — vector of (tenor_years, weight) per tenor
    CurveBucketed { curve_id: CurveId, tenor_weights: Vec<(f64, f64)> },
    /// Equity spot move
    EquitySpot { tickers: Vec<String> },
    /// FX rate move
    FxRate { pair: (Currency, Currency) },
    /// Vol surface shift
    VolShift { surface_ids: Vec<String>, units: BumpUnits },
    /// User-defined function: factor move → BumpSpecs.
    /// Only available via builder path (not serializable).
    #[serde(skip)]
    Custom(Arc<dyn Fn(f64) -> Vec<BumpSpec> + Send + Sync>),
}
```

**Note:** The `Custom` variant is `#[serde(skip)]` — it is only available via the `FactorModelBuilder` trait override path, not through JSON/YAML config. Attempting to deserialize a config containing a `Custom` mapping will fail with a clear error.

#### `FactorCovarianceMatrix`

User-supplied covariance matrix with proper linear algebra layout:

```rust
pub struct FactorCovarianceMatrix {
    pub factor_ids: Vec<FactorId>,
    n: usize,                    // number of factors (== factor_ids.len())
    data: Vec<f64>,              // row-major flat storage, n × n
}
```

Constructed via `FactorCovarianceMatrix::new(factor_ids, data)` which validates:
- Dimensions: `data.len() == n * n`
- Symmetry: `|data[i*n+j] - data[j*n+i]| < ε` for all i, j
- Positive semi-definiteness: via Cholesky decomposition (O(n³); for large models >100 factors, provide `new_unchecked()` to skip PSD validation for trusted inputs)

Accessors: `variance(factor_id)`, `covariance(f1, f2)`, `correlation(f1, f2)`, `as_slice() -> &[f64]`, `n_factors() -> usize`.

### Factor Matching

#### `FactorMatcher` Trait

```rust
pub trait FactorMatcher: Send + Sync {
    fn match_factor(
        &self,
        dependency: &MarketDependency,
        attributes: &Attributes,
    ) -> Option<FactorId>;
}
```

The `FactorModel` calls this for each `MarketDependency` (from decomposing `instrument.market_dependencies()`) combined with the instrument's `Attributes`. Each dependency is independently matched to a factor.

#### `FactorNode` (for Hierarchical Matching)

```rust
pub struct FactorNode {
    /// Factor assigned at this node (if leaf or if this level is a valid assignment)
    pub factor_id: Option<FactorId>,
    /// Filter that must match for traversal into this node
    pub filter: AttributeFilter,
    /// Child nodes (more specific classifications)
    pub children: Vec<FactorNode>,
}
```

Traversal: depth-first, all children whose `filter` matches are explored. The deepest matching node with a `factor_id` wins. If multiple leaf nodes at the same depth match, the first in order wins (deterministic via Vec ordering).

Example tree for credit factors:

```
Credit (factor_id: "Generic-Credit")
├── NA (filter: meta region=NA, factor_id: "NA-Credit")
│   ├── Energy (filter: meta sector=Energy)
│   │   ├── CCC (filter: meta rating=CCC, factor_id: "NA-Energy-CCC")
│   │   └── IG (filter: meta rating=IG, factor_id: "NA-Energy-IG")
│   └── Financials (filter: meta sector=Financials, factor_id: "NA-Financials")
└── EU (filter: meta region=EU, factor_id: "EU-Credit")
```

#### Three Built-in Strategies

1. **`MappingTableMatcher`** — flat lookup: `Vec<(DependencyFilter, AttributeFilter, FactorId)>`. First match wins. Filters support wildcards and conjunctions.

2. **`CascadeMatcher`** — ordered list of matchers (each implementing `FactorMatcher`), tries each in priority order, returns first match. Enables "try exact curve ID → fall back to sector+rating → fall back to asset-class default."

3. **`HierarchicalMatcher`** — tree of `FactorNode`s. Positions match the deepest (most specific) node. Unmatched risk rolls up to parent.

All three implement `FactorMatcher` and are composable (a `CascadeMatcher` can contain a `HierarchicalMatcher` as one of its stages).

---

## Section 2: Factor Sensitivity Engine (`finstack/valuations`)

Computes how much each position moves per unit of each factor. Lives in `valuations` because it needs pricing/repricing infrastructure.

### Pricing Modes

```rust
pub enum PricingMode {
    /// Compute linear sensitivities via finite-difference bumps, then use
    /// delta × factor_move for risk decomposition. Fast, one bump per factor.
    DeltaBased,
    /// Rebuild MarketContext with factor-implied bumps at multiple shift sizes,
    /// reprice everything. Captures convexity and nonlinearity.
    FullRepricing,
}
```

### `FactorSensitivityEngine` Trait

```rust
pub trait FactorSensitivityEngine: Send + Sync {
    fn compute_sensitivities(
        &self,
        positions: &[(String, &dyn Instrument, f64)],  // (position_id, instrument, notional_weight)
        factors: &[FactorDefinition],
        market: &MarketContext,
        as_of: Date,
    ) -> Result<SensitivityMatrix>;
}
```

The `FactorModel::analyze()` method extracts `(position_id, &dyn Instrument, weight)` tuples from `Portfolio` by iterating its `Vec<Position>`, using each position's instrument reference and notional as the weight.

### `SensitivityMatrix`

Positions × factors matrix with flat storage:

```rust
pub struct SensitivityMatrix {
    pub position_ids: Vec<String>,
    pub factor_ids: Vec<FactorId>,
    data: Vec<f64>,  // row-major [n_positions × n_factors]
}
```

Accessors: `delta(position_idx, factor_idx)`, `position_deltas(position_idx) -> &[f64]`, `factor_deltas(factor_idx) -> Vec<f64>`, `n_positions()`, `n_factors()`.

All positions appear in the matrix, including zero-sensitivity ones (their row is all zeros). This keeps indexing consistent and avoids sparse-matrix complexity.

### `BumpSizeConfig`

Override default bump sizes per factor type:

```rust
pub struct BumpSizeConfig {
    pub rates_bp: f64,      // default: 1.0 bp
    pub credit_bp: f64,     // default: 1.0 bp
    pub equity_pct: f64,    // default: 1.0 %
    pub fx_pct: f64,        // default: 1.0 %
    pub vol_points: f64,    // default: 1.0 vol point
    pub overrides: BTreeMap<FactorId, f64>,  // per-factor override
}
```

### Built-in Implementations

**`DeltaBasedEngine`** — the default:
1. Inspects factor's `MarketMapping` to determine what to bump
2. Converts to `BumpSpec` (reuses existing bump infrastructure)
3. Applies bump to `MarketContext` → reprices → finite-difference delta
4. Bump size from `BumpSizeConfig` (per-factor overrides take priority)
5. Uses central differencing where supported

**`FullRepricingEngine`** — for nonlinear risk:
1. Constructs scenario `MarketContext`s at multiple shift sizes (e.g., ±1σ, ±2σ) per factor
2. Reprices full portfolio under each scenario
3. Stores full P&L profile per factor (not just linear delta)
4. Used by `SimulationDecomposer` for non-parametric VaR/ES

Both engines parallelize across positions via Rayon. `FullRepricingEngine` also parallelizes across factor scenarios.

### Integration with Existing Infrastructure

Delegates to:
- `MarketContext::bump()` / `apply_bumps()` for scenario construction
- `instrument.price_with_metrics()` for repricing
- `extract_risk_factors()` for auto-discovery of curve dependencies

---

## Section 3: Risk Decomposition & Budgeting (`finstack/portfolio`)

Factor sensitivities + covariance matrix → risk attribution, budgeting, and what-if analysis.

### Risk Measures

```rust
pub enum RiskMeasure {
    Variance,
    Volatility,
    VaR { confidence: f64 },
    ExpectedShortfall { confidence: f64 },
}
```

**Sign convention:** VaR and ES are expressed as positive loss amounts. `confidence` is the right-tail probability (e.g., 0.99 means "99% of losses are below this level"). Parametric VaR = `σ × z_α` where `z_α = Φ⁻¹(confidence)`. Parametric ES = `σ × φ(z_α) / (1 - confidence)` under normality.

### `RiskDecomposer` Trait

```rust
pub trait RiskDecomposer: Send + Sync {
    fn decompose(
        &self,
        sensitivities: &SensitivityMatrix,
        covariance: &FactorCovarianceMatrix,
        position_weights: &[f64],
        measure: &RiskMeasure,
    ) -> Result<RiskDecomposition>;
}
```

### Built-in Implementations

**`ParametricDecomposer`** — for Variance/Volatility and normal VaR/ES:
- Portfolio factor exposure: `e = W' × S` (weights × sensitivity matrix)
- Portfolio variance: `σ² = e' × Σ × e` (exposure × covariance × exposure)
- Component risk via Euler decomposition: `CR_i = e_i × (Σ × e)_i / σ²`
- Per-position component risk: further decompose each factor's risk to contributing positions
- VaR/ES under normality: scale by `z_α` or `φ(z_α) / (1 - confidence)` respectively

**`SimulationDecomposer`** — for non-parametric VaR/ES:
- Correlated factor scenarios via Cholesky decomposition (pivoted Cholesky for rank-deficient matrices)
- Uses `FullRepricingEngine` P&L profiles for nonlinear positions
- Component ES via Euler allocation: `ES_i = E[L_i | L > VaR_α]` — average of component P&Ls in the tail beyond VaR
- Component VaR via Hallerbach approximation: scale component ES contributions by `VaR / ES` ratio to maintain additivity
- Handles fat tails and nonlinearity

### Output Types

```rust
pub struct RiskDecomposition {
    pub total_risk: f64,
    pub measure: RiskMeasure,
    pub factor_contributions: Vec<FactorContribution>,
    pub residual_risk: f64,
    pub position_factor_contributions: Vec<PositionFactorContribution>,
}

pub struct FactorContribution {
    pub factor_id: FactorId,
    pub absolute_risk: f64,       // in portfolio currency units
    pub relative_risk: f64,       // as fraction of total (sums to 1.0)
    pub marginal_risk: f64,       // ∂(total_risk)/∂(factor_exposure)
}

pub struct PositionFactorContribution {
    pub position_id: String,
    pub factor_id: FactorId,
    pub risk_contribution: f64,
}
```

### What-If Engine

```rust
pub struct WhatIfEngine<'a> {
    model: &'a FactorModel,
    base_decomposition: &'a RiskDecomposition,
    base_sensitivities: &'a SensitivityMatrix,
    portfolio: &'a Portfolio,
    market: &'a MarketContext,
    as_of: Date,
}
```

**Three operations:**

#### 1. Position What-If

Add/remove/resize positions:

```rust
pub enum PositionChange {
    Add { position_id: String, instrument: Box<dyn Instrument>, weight: f64 },
    Remove { position_id: String },
    Resize { position_id: String, new_weight: f64 },
}

pub struct WhatIfResult {
    pub before: RiskDecomposition,
    pub after: RiskDecomposition,
    pub delta: Vec<FactorContributionDelta>,  // per-factor change
}

pub struct FactorContributionDelta {
    pub factor_id: FactorId,
    pub absolute_change: f64,
    pub relative_change: f64,
}

fn position_what_if(&self, changes: &[PositionChange]) -> Result<WhatIfResult>;
```

Recomputes sensitivities only for changed positions; reuses cached sensitivities for unchanged.

#### 2. Factor Stress

Shock specific factors:

```rust
pub struct StressResult {
    pub total_pnl: f64,                          // portfolio-level P&L impact
    pub position_pnl: Vec<(String, f64)>,        // per-position P&L
    pub stressed_decomposition: RiskDecomposition, // risk budget under stress
}

fn factor_stress(&self, stresses: &[(FactorId, f64)]) -> Result<StressResult>;
```

Converts factor moves to `BumpSpec`s via `MarketMapping`, applies to `MarketContext`, reprices.

#### 3. Factor-Constrained Optimization

Rebalance with factor limits:

```rust
pub enum FactorConstraint {
    /// Factor's absolute risk contribution must not exceed this value
    MaxFactorRisk { factor_id: FactorId, max_risk: f64 },
    /// Factor's relative risk contribution must not exceed this fraction
    MaxFactorConcentration { factor_id: FactorId, max_fraction: f64 },
    /// Factor exposure must be zero (market-neutral to this factor)
    FactorNeutral { factor_id: FactorId },
}

fn optimize(
    &self,
    objective: Objective,  // reuses existing portfolio Objective enum
    constraints: &[FactorConstraint],
) -> Result<OptimizedPortfolio>;
```

Translates `FactorConstraint`s into the existing portfolio optimizer's `Constraint` type by expressing factor exposures as linear functions of position weights, then delegates to the existing optimizer.

### Factor Assignment Report

For debugging and inspection of the matching step:

```rust
pub struct FactorAssignmentReport {
    /// Per-position: which dependencies matched which factors
    pub assignments: Vec<PositionAssignment>,
    /// Dependencies that did not match any factor (subject to UnmatchedPolicy)
    pub unmatched: Vec<UnmatchedEntry>,
}

pub struct PositionAssignment {
    pub position_id: String,
    pub mappings: Vec<(MarketDependency, FactorId)>,
}

pub struct UnmatchedEntry {
    pub position_id: String,
    pub dependency: MarketDependency,
}
```

---

## Section 4: Top-Level API

### `FactorModelConfig` (Declarative Path)

```rust
#[derive(Serialize, Deserialize)]
pub struct FactorModelConfig {
    pub factors: Vec<FactorDefinition>,
    pub covariance: FactorCovarianceMatrix,
    pub matching: MatchingConfig,
    pub pricing_mode: PricingMode,
    pub risk_measure: RiskMeasure,
    pub bump_size: Option<BumpSizeConfig>,
    pub unmatched_policy: Option<UnmatchedPolicy>,  // default: Residual
}

#[derive(Serialize, Deserialize)]
pub enum MatchingConfig {
    MappingTable(Vec<MappingRule>),
    Cascade(Vec<MatchingConfig>),      // recursive — cascade of strategies
    Hierarchical(FactorNode),
}

#[derive(Serialize, Deserialize)]
pub struct MappingRule {
    pub dependency_filter: DependencyFilter,
    pub attribute_filter: AttributeFilter,
    pub factor_id: FactorId,
}

#[derive(Serialize, Deserialize)]
pub struct DependencyFilter {
    /// Match by dependency classification (Discount, Forward, Credit, Spot, Vol, FX, Series)
    pub dependency_type: Option<CurveType>,
    /// Match by exact curve/spot/surface ID
    pub id: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct AttributeFilter {
    /// All tags must be present on the instrument
    pub tags: Vec<String>,
    /// All key-value pairs must match in the instrument's meta
    pub meta: Vec<(String, String)>,
}
```

Entire config is `Serialize + Deserialize`. The `MarketMapping::Custom` variant is excluded from serialization (only available via builder).

### `FactorModelBuilder` (with Trait Overrides)

```rust
impl FactorModelBuilder {
    pub fn new() -> Self;
    pub fn config(self, config: FactorModelConfig) -> Self;
    pub fn with_custom_matcher(self, m: impl FactorMatcher + 'static) -> Self;
    pub fn with_custom_sensitivity_engine(self, e: impl FactorSensitivityEngine + 'static) -> Self;
    pub fn with_custom_decomposer(self, d: impl RiskDecomposer + 'static) -> Self;
    pub fn build(self) -> Result<FactorModel>;
}
```

### `FactorModel` (Runtime Object)

```rust
impl FactorModel {
    /// Full pipeline: match → compute sensitivities → decompose
    pub fn analyze(&self, portfolio: &Portfolio, market: &MarketContext, as_of: Date) -> Result<RiskDecomposition>;

    /// Just matching (for inspection/debugging)
    pub fn assign_factors(&self, portfolio: &Portfolio) -> Result<FactorAssignmentReport>;

    /// Just sensitivities (for caching)
    pub fn compute_sensitivities(&self, portfolio: &Portfolio, market: &MarketContext, as_of: Date) -> Result<SensitivityMatrix>;

    /// What-if engine from a base analysis
    pub fn what_if<'a>(
        &'a self,
        base: &'a RiskDecomposition,
        sensitivities: &'a SensitivityMatrix,
        portfolio: &'a Portfolio,
        market: &'a MarketContext,
        as_of: Date,
    ) -> WhatIfEngine<'a>;
}
```

### Python API

Full PyO3 parity via config objects:

```python
config = FactorModelConfig(
    factors=[
        FactorDefinition(id="USD-Rates", factor_type="Rates",
            market_mapping=MarketMapping.curve_parallel(["USD-OIS"], units="bp")),
        FactorDefinition(id="NA-Energy-CCC", factor_type="Credit",
            market_mapping=MarketMapping.curve_parallel(["NA-ENERGY-CCC-HAZARD"], units="bp")),
    ],
    covariance=FactorCovarianceMatrix(
        factor_ids=["USD-Rates", "NA-Energy-CCC"],
        matrix=[[0.0004, 0.0001], [0.0001, 0.0009]],
    ),
    matching=MatchingConfig.mapping_table([
        MappingRule(
            dependency_filter=DependencyFilter(dependency_type="Credit"),
            attribute_filter=AttributeFilter(meta=[("region", "NA"), ("sector", "Energy"), ("rating", "CCC")]),
            factor_id="NA-Energy-CCC",
        ),
    ]),
    pricing_mode="DeltaBased",
    risk_measure="Variance",
)

model = FactorModelBuilder().config(config).build()
decomposition = model.analyze(portfolio, market, as_of)

for fc in decomposition.factor_contributions:
    print(f"{fc.factor_id}: {fc.relative_risk:.1%}")

# What-if: factor stress
sensitivities = model.compute_sensitivities(portfolio, market, as_of)
engine = model.what_if(decomposition, sensitivities, portfolio, market, as_of)
stress = engine.factor_stress([("USD-Rates", 2.0)])  # +2σ shock

# What-if: remove a position
result = engine.position_what_if([PositionChange.remove("ACME-BOND-001")])
print(f"Rates risk change: {result.delta[0].relative_change:+.1%}")
```

---

## Section 5: Error Handling, Testing & Edge Cases

### Error Types

```rust
pub enum FactorModelError {
    /// Position has market dependencies that no factor matches (when policy is Strict)
    UnmatchedDependency { position_id: String, dependency: MarketDependency },
    /// Factor referenced in config/rules but missing from covariance matrix
    MissingFactor { factor_id: FactorId },
    /// Covariance matrix failed validation
    InvalidCovariance { reason: String },
    /// Repricing failed for a position under a factor scenario
    RepricingFailed { position_id: String, factor_id: FactorId, source: Box<dyn Error> },
    /// Multiple matchers at the same priority matched the same dependency
    AmbiguousMatch { position_id: String, candidates: Vec<FactorId> },
    /// Factor-constrained optimization has no feasible solution
    InfeasibleConstraints { reason: String },
}
```

### Unmatched Dependency Policy

```rust
pub enum UnmatchedPolicy {
    /// Fail hard — any unmatched dependency is an error
    Strict,
    /// Assign to a catch-all "Residual" factor (default)
    Residual,
    /// Log a warning and exclude from factor decomposition
    Warn,
}
```

### Testing Strategy

**Unit tests per layer:**
- `core`: Factor matching — verify each matcher strategy with (dependency, attributes) → factor_id cases. Cascade ordering, hierarchical depth, wildcards.
- `core`: Covariance validation — symmetry, PSD, dimension mismatch. Verify `new_unchecked()` skips validation.
- `core`: `MarketDependency::decompose()` — verify correct flattening of `MarketDependencies` struct.
- `valuations`: Sensitivity engine — compare against hand-computed finite differences for a simple bond + equity portfolio. Verify `FullRepricingEngine` captures gamma that `DeltaBased` misses (e.g., an at-the-money option).
- `portfolio`: Decomposition — verify component risks sum to total (Euler property) with known 2-factor/3-position example where answer is hand-computable.

**Integration tests:**
- End-to-end: multi-asset portfolio (bond + CDS + equity + FX forward), 4-5 factors, verify sensible attribution (rates factor dominates for bond, credit for CDS, etc.).
- Round-trip: serialize `FactorModelConfig` to JSON, deserialize, verify identical decomposition results (determinism).
- What-if consistency: `position_what_if(remove X)` gives the same result as `analyze()` on the portfolio without X.
- Python bindings: mirror the integration test in Python for PyO3 parity.

### Edge Cases

- **Single-factor model** — degenerate case, 100% attribution to one factor.
- **Perfectly correlated factors** — rank-deficient covariance. `SimulationDecomposer` uses pivoted Cholesky or eigendecomposition fallback.
- **Zero-sensitivity positions** — cash, expired options. Present in `SensitivityMatrix` with all-zero rows. Contribute zero to all factors, no division-by-zero in relative risk (guard: if total_risk ≈ 0, set all relative contributions to 0).
- **Cross-currency portfolio** — sensitivities computed in each position's native currency, then converted to portfolio base currency via `MarketContext` FX rates, consistent with existing `portfolio/valuation.rs`.
- **Large factor models (>100 factors)** — `new_unchecked()` skips O(n³) PSD validation for trusted inputs. `SensitivityMatrix` uses flat storage for cache locality.
