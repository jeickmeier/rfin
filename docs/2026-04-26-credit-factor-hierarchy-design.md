# Hierarchical Credit Factor Decomposition — Design Spec

**Status:** Approved (brainstorming sign-off)
**Date:** 2026-04-26
**Owner:** finstack/valuations + finstack/portfolio
**Schema:** `finstack.credit_factor_model/1`

## 1. Motivation

Today, credit attribution and credit risk in `finstack/valuations` treat each issuer's credit curve as an opaque, single-bucket entity. PnL explain reports `credit_curves_pnl` as one number; risk reports per-curve sensitivity but not what *common factor* the move loaded against; carry slices by source (coupon / pull-to-par / roll-down) but not by what part of the spread is driving each component.

This spec introduces a hierarchical credit factor model that:

- Decomposes every issuer's spread into a sequence of common factors (a user-designated **generic** factor at the top, then user-configurable **bucket** levels — typically rating, region, sector — in any order or depth) plus an issuer-specific **adder** residual.
- Slices PnL attribution into per-level contributions across all four existing attribution methods (Waterfall / Taylor / Parallel / MetricsBased).
- Slices carry into rates vs credit components, with the credit component further allocated to each level.
- Forecasts forward volatility per factor (via existing GARCH/EWMA infrastructure) and emits a vol-contribution-by-credit-level report.
- Plugs into the existing `portfolio::factor_model` (Barra/Axioma-style) infrastructure rather than building a parallel system. Calibration produces a `FactorModelConfig` consumable by the existing `FactorModel` directly.

The design is **non-breaking and opt-in**: every consumer takes `Option<&CreditFactorModel>` and falls back to today's behavior when absent.

## 2. Architecture Overview

### 2.1 Reuse of existing infrastructure

The bulk of the machinery already exists in `finstack/portfolio/src/factor_model/` and `finstack/core/src/factor_model/`. The new credit hierarchy slots in as a *configured instance* of that machinery, not a parallel system.

| Existing piece | Role in this design |
|---|---|
| `FactorDefinition` + `FactorId` + `FactorType::Credit` | Each common factor (PC, every bucket at every level) becomes a `FactorDefinition`. |
| `FactorCovarianceMatrix` | Holds `Σ_factor` — the cross-factor covariance after calibration. |
| `FactorModelConfig` | Declarative: factor list + covariance + matcher + risk measure. |
| `MappingTableMatcher` / `HierarchicalMatcher` | New `MatchingConfig::CreditHierarchical` variant maps each issuer's curve → bucket factors via tags + β lookup. |
| `FactorModel.assign_factors / compute_sensitivities / analyze` | Once a calibrated config exists, these work unchanged. |
| `ParametricDecomposer` (Euler / variance) | Closed-form risk contribution per factor — gives "vol contribution by credit level" for free. |
| `SimulationDecomposer` | Tail-risk version (VaR/ES contribution by credit level). |
| `HistoricalPositionDecomposer` / `ParametricPositionDecomposer` | Per-position contribution to each credit factor. |
| `WhatIfEngine` | Stress: "shock IG-EU-FIN bucket by 5bp, what's portfolio P&L". |
| `RiskBudget` | If risk budgeting at credit-factor level becomes a use case. |
| `analytics::benchmark::beta` | Single-factor OLS for issuer-level regressions in calibration. |
| `core::math::stats::covariance` | Sample covariance for static `ρ`. |
| `valuations::correlation::nearest_correlation` | PSD repair for sample covariance if needed. |
| `analytics::timeseries::garch*` | Per-factor variance forecasting. |

**No new general utilities required.** The "missing OLS / Ledoit-Wolf" story turned out to be wrong — `analytics::benchmark::beta` covers our single-factor needs, and Σ construction defaults to diagonal (factors are constructed to be approximately orthogonal by design) with optional ridge as a one-line variant. Ledoit-Wolf and a general `core::math::regression` are surfaced as v2 nice-to-haves only.

### 2.2 Module placement

| New module | Crate | Contents |
|---|---|---|
| `core::factor_model::credit_hierarchy` | `core` | `CreditHierarchySpec`, `HierarchyDimension`, `IssuerTags`, new `MarketMapping::CreditHierarchical` variant, factor-id naming convention. |
| `valuations::factor_model::credit_calibration` | `valuations` | `CreditCalibrator`, `CreditCalibrationInputs`, `CreditCalibrationConfig`. |
| `valuations::factor_model::credit_decomposition` | `valuations` | `decompose_levels`, `decompose_period`, `LevelsAtDate`, `PeriodDecomposition`. |
| `valuations::factor_model::credit_vol_forecast` | `valuations` | `FactorCovarianceForecast`, `VolHorizon`, `CreditVolReport`. |
| `valuations::attribution::credit_factor` | `valuations` | Wires the calibrated `CreditFactorModel` into all four attribution methods + carry. |

The artifact envelope type `CreditFactorModel` lives in `core::factor_model::credit_hierarchy` (since it embeds the existing `FactorModelConfig` and is needed by both calibration and consumption sites).

### 2.3 Data flow

**Calibration (offline, periodic — typically monthly):**
- Inputs: history panel of issuer Δspreads + per-issuer point-in-time tags + user-designated generic factor series + as-of date + per-issuer current spreads + optional caller-supplied idiosyncratic vol overrides.
- Pipeline: `CreditFactorCalibrator`.
- Output: `CreditFactorModel` JSON artifact (factor list, per-issuer betas, factor histories, idiosyncratic state, anchor levels, static correlation, GARCH state, diagnostics).

**Consumption (per-call):**
- Inputs: portfolio + market context + as-of + `Option<&CreditFactorModel>`.
- Three independent flows:
  - **Attribution:** waterfall / Taylor / parallel / metrics-based methods read the model and populate new `credit_factor_detail` field.
  - **Risk decomposition:** model feeds the existing `FactorModel.analyze()`; `ParametricDecomposer` / `SimulationDecomposer` produce per-factor contributions.
  - **Vol forecast:** `FactorCovarianceForecast` builds a fresh `FactorCovarianceMatrix(t,h)` per as-of/horizon from `vol_state`; same downstream pipeline.

When `CreditFactorModel: None`, every consumer falls back to today's behavior unchanged.

## 3. The Calibrated `CreditFactorModel` Artifact

### 3.1 Lifecycle

Calibration runs offline (monthly is the assumed cadence) and produces a JSON artifact. Consumption sites load the artifact at startup and pass `&CreditFactorModel` into attribution / risk / vol-forecast calls.

### 3.2 Top-level shape

```rust
pub struct CreditFactorModel {
    pub schema_version: String,           // "finstack.credit_factor_model/1"
    pub as_of: Date,                      // calibration anchor
    pub calibration_window: DateRange,    // history span
    pub policy: IssuerBetaPolicy,         // mode-decision policy used
    pub generic_factor: GenericFactorSpec,// user-designated reference series
    pub hierarchy: CreditHierarchySpec,   // levels in spec order

    pub config: FactorModelConfig,        // EXISTING type — feeds existing FactorModel directly
    pub issuer_betas: Vec<IssuerBetaRow>, // per-issuer β + mode + adder anchor + adder vol
    pub anchor_state: LevelsAtAnchor,     // L_PC, L_<level>(g), ... at as_of
    pub static_correlation: FactorCorrelationMatrix, // ρ for Σ(t) = D(t)·ρ·D(t)
    pub vol_state: VolState,              // GARCH/EWMA params per factor + per-issuer adder vol
    pub factor_histories: Option<FactorHistories>,   // embedded; recommended for self-containment
    pub diagnostics: CalibrationDiagnostics,         // per-bucket coverage, mode counts, fold-ups
}
```

### 3.3 Key sub-types

```rust
pub enum IssuerBetaPolicy {
    Dynamic {
        min_history: usize,                              // default 24
        overrides: HashMap<IssuerId, IssuerBetaMode>,    // {Auto | ForceIssuerBeta | ForceBucketOnly}
    },
    GloballyOff,  // every issuer treated as BucketOnly; no per-issuer regression
}

pub enum IssuerBetaMode { IssuerBeta, BucketOnly }

pub struct CreditHierarchySpec {
    pub levels: Vec<HierarchyDimension>,  // ordered, broadest → narrowest
}

pub enum HierarchyDimension {
    Rating, Region, Sector,
    Custom(String),  // reads tags[<key>] for arbitrary user dimensions (e.g. Currency, AssetType)
}

pub struct IssuerTags(pub BTreeMap<String, String>);  // BTreeMap for deterministic serialization

pub struct IssuerBetaRow {
    pub issuer_id: IssuerId,
    pub tags: IssuerTags,
    pub mode: IssuerBetaMode,
    pub betas: IssuerBetas,             // β_PC, β_<level1>, β_<level2>, ... — all 1.0 if BucketOnly
    pub adder_at_anchor: f64,           // L_adder(i) at as_of — for carry
    pub adder_vol_annualized: f64,      // for vol forecasting
    pub adder_vol_source: AdderVolSource, // FromHistory | BucketPeerProxy | CallerSupplied | Default
    pub fit_quality: Option<FitQuality>,  // R², residual std, n_obs (None for BucketOnly)
}
```

### 3.4 Design notes

- **`config: FactorModelConfig`** — the existing `core::factor_model::FactorModelConfig`. The factor list is auto-generated in canonical order: `["credit::generic", "credit::level0::<dim>::<val>", "credit::level1::<dim_path>::<val_path>", ...]`. Factor IDs are deterministic so two calibrations on the same input produce identical IDs.
- **Embedded vs external histories.** Default: embedded (~100 KB scale for typical configs). External reference path supported for very large calibrations.
- **Diagnostics as first-class structured field.** Consumers can programmatically check coverage (e.g. "did at least 95% of buckets have ≥ 5 issuers").
- **Dual purpose:** the artifact serves both "as-of date covariance" (via `config.covariance`) and "forecast forward volatility" (via `vol_state` + `static_correlation` → `FactorCovarianceForecast`).
- **Tag taxonomy consistency.** The artifact carries the canonical taxonomy in diagnostics so a runtime mismatch with instrument metadata is detectable and surfaces as a clean error rather than silent miscategorization.

### 3.5 Loading and instantiation

```rust
let model = CreditFactorModel::from_json(&bytes)?;

let factor_model = FactorModelBuilder::new()
    .config(model.config.clone())
    .build()?;
```

## 4. Calibration Pipeline

### 4.1 Inputs and config

```rust
pub struct CreditCalibrationInputs {
    pub history_panel: HistoryPanel,                // sparse panel of per-issuer Δspreads or levels
    pub issuer_tags: IssuerTagPanel,                // point-in-time per-issuer tags
    pub generic_factor: GenericFactorSeries,        // user-designated reference series at panel dates
    pub as_of: Date,                                // anchor date
    pub asof_spreads: HashMap<IssuerId, Spread>,    // current spread per issuer
    pub idiosyncratic_overrides: HashMap<IssuerId, f64>, // optional caller-supplied adder vols
}

pub struct CreditCalibrationConfig {
    pub policy: IssuerBetaPolicy,
    pub hierarchy: CreditHierarchySpec,
    pub min_bucket_size_per_level: BucketSizeThresholds, // default 5 per level
    pub vol_model: VolModelChoice,                  // GarchMle | Egarch | Ewma{lambda} | Sample
    pub covariance_strategy: CovarianceStrategy,    // Diagonal (default) | Ridge{alpha} | FullSampleRepaired
    pub beta_shrinkage: BetaShrinkage,              // None | TowardOne{alpha} for small N
    pub use_returns_or_levels: PanelSpace,          // Returns is the default
}
```

### 4.2 Algorithm (sequential peel-the-onion, hierarchy-agnostic)

```
1. Mode classification per issuer (per IssuerBetaPolicy)
2. Bucket inventory + sparsity check at every level
   - Buckets below threshold are folded into parent (issuer's β at that level = 0)
   - Diagnostics record every fold-up
3. PC step:
   - For IssuerBeta issuers: fit β_i^PC = OLS(S_i on F_PC) (with optional shrinkage for small N)
   - For BucketOnly issuers: β_i^PC = 1
   - Compute r1_i(t) = S_i(t) − β_i^PC × F_PC(t)
4. For each level_idx in 0..hierarchy.levels.len():
   - Build bucket factors: F_<level>(g, t) = mean_{i ∈ g}(r_i^(level_idx)(t))
   - For IssuerBeta issuers in surviving buckets: fit β_i^<level> = OLS(r_i^(level_idx) on F_<level>(g_i))
   - For BucketOnly issuers: β_i^<level> = 1
   - Compute r_i^(level_idx+1)(t) = r_i^(level_idx)(t) − β_i^<level> × F_<level>(g_i, t)
5. After last level: r_i^(L)(t) = adder series (for IssuerBeta) / undefined per-period (for BucketOnly)
6. Anchor levels at as_of: identical math in level space using asof_spreads as inputs
   - L_PC = generic_factor.value_at(as_of)
   - L_<level>(g) computed by cross-sectional means after subtracting prior levels
   - adder_at_anchor(i) = S_i(as_of) − Σ_levels β_i^<level> × L_<level>(g_i)
7. Idiosyncratic vol:
   - IssuerBeta: fit vol_model on adder return series
   - BucketOnly: bucket-peer proxy from IssuerBeta peers (cascade fallback to parent buckets, then global default)
   - Caller override always wins
8. Per-factor variance forecast model fit (vol_model on factor return series)
9. Static correlation matrix:
   - Default: ρ = identity (factors assumed orthogonal by construction → diagonal Σ)
   - Optional: ρ = corr(factor_returns); apply ridge or PSD repair if needed
10. Assemble FactorModelConfig:
    - Factor list in canonical order
    - Σ_anchor = D_anchor · ρ · D_anchor
    - Matcher = MatchingConfig::CreditHierarchical { hierarchy, beta_lookup }
11. Diagnostics assembly: per-bucket sample size, variance explained, mode counts, fold-up record, R² histogram, fall-back tracking
12. Bundle into CreditFactorModel and serialize
```

### 4.3 Properties

- **Deterministic.** Same inputs + same config → bit-identical artifact.
- **Streaming-friendly.** Cost is `O(N × T × L)` where `L = #levels`.
- **Sparsity-tolerant.** Buckets fold up, sparse issuers go `BucketOnly`, no crashes on sparse data.
- **Per-issuer panel gaps handled.** Missing dates dropped from regressions; bucket means over present-on-date issuers only — critical for private credit.

### 4.4 Edge cases

- **Bucket dominated by `BucketOnly` issuers.** The bucket factor is essentially the cross-sectional spread distribution of those private-credit-style issuers. This is correct behavior, not a bug.
- **Empty hierarchy `[]`.** Produces a model with only PC + per-issuer adders. Valid configuration.
- **All issuers `BucketOnly`** (under `GloballyOff` or all issuers below `min_history`). No per-issuer regression; bucket factors are pure cross-sectional means.

## 5. Per-Period Decomposition Utility

Pure function module (`valuations::factor_model::credit_decomposition`). Used by attribution to allocate ΔPnL to levels and by carry to allocate spread *level* at as-of.

### 5.1 API

```rust
impl CreditFactorModel {
    /// Compute factor LEVELS at a specific date from observed issuer spreads.
    pub fn decompose_levels(
        &self,
        observed_spreads: &HashMap<IssuerId, Spread>,
        observed_generic: f64,
        as_of: Date,
    ) -> Result<LevelsAtDate>;

    /// Compute factor RETURNS over a period from two snapshots.
    pub fn decompose_period(
        &self,
        from: &LevelsAtDate,
        to: &LevelsAtDate,
    ) -> PeriodDecomposition;
}
```

### 5.2 Mechanics

`decompose_levels`:
1. Validate every issuer in input has tags for every dimension in the hierarchy.
2. Look up `β_i^PC`; subtract `β_i^PC × observed_generic` → PC-residual per issuer.
3. For each level in hierarchy order: bucket means = level value; subtract `β_i^<level> × L_<level>(g_i)`.
4. Final residual per issuer = `adder_at(i, date)`.
5. Return `LevelsAtDate { generic, by_level, adder }`.

`decompose_period`: trivially `to - from` for each component.

### 5.3 Reconciliation invariant (tested)

For every issuer present in both snapshots:
```
observed_Δspread_i ≡ β_i^PC × ΔF_PC + Σ_levels β_i^<level> × ΔF_<level>(g_i) + Δadder_i
```

### 5.4 Universe-difference handling

- **New issuer at consumption** (not in calibration): valid only if all needed tags present; treated as `BucketOnly` with `β = 1` and zero historical adder. Recorded in diagnostics.
- **Issuer in calibration but not in `observed_spreads`**: skipped. Bucket means computed over present issuers only.
- **Bucket present in calibration but no current issuers at this date**: `L_<level>(g) = 0`; issuers tagged into that bucket fall back to `β = 0` for that level.

## 6. Attribution Integration

### 6.1 Per-method integration

| Method | Today | With `CreditFactorModel` |
|---|---|---|
| **Waterfall** | Single "Credit" step in `default_waterfall_order()` | Replaced by cascade in hierarchy spec order: `Credit::PC, Credit::<level0>, ..., Credit::<levelN-1>, Credit::Adder`. Each step bumps curves by `β_i^<level> × ΔF_<level>(g_i)` (or `Δadder_i` for adder), reprices, captures level PnL. Residual ≈ 0 by construction. |
| **Taylor** | `−CS01 × Δspread` per curve | Per level: `−Σ_i CS01_i × β_i^<level> × ΔF_<level>(g_i)`. Per-issuer Adder: `−CS01_i × Δadder_i`. Sums to today's Taylor credit total. |
| **Parallel** | Each factor isolated reval | Each level becomes its own factor in the parallel set. Cross-effect residual stays in existing `cross_factor_pnl`. |
| **MetricsBased** | `−CS01 × Δspread` | `−Σ_i CS01_i × β_i^<level> × ΔF_<level>(g_i)` per level + per-issuer Adder. Linear, fast, deterministic. |

### 6.2 New result types

```rust
pub struct PnlAttribution {
    // ...all existing fields unchanged...
    pub credit_factor_detail: Option<CreditFactorAttribution>,
}

pub struct CreditFactorAttribution {
    pub model_id: String,                    // hash of CreditFactorModel for traceability
    pub generic_pnl: SignedAmount,           // PC-level
    pub levels: Vec<LevelPnl>,               // one per HierarchyDimension in spec order
    pub adder_pnl_total: SignedAmount,
    pub adder_pnl_by_issuer: Option<HashMap<IssuerId, SignedAmount>>,
}

pub struct LevelPnl {
    pub level_name: String,
    pub total: SignedAmount,
    pub by_bucket: HashMap<BucketId, SignedAmount>,
}
```

### 6.3 Reconciliation invariant (tested)

```
generic_pnl + Σ_levels(level.total) + adder_pnl_total ≡ credit_curves_pnl
```

Holds by construction for Waterfall, by linearity for Taylor / MetricsBased, modulo cross-effect residual for Parallel (which stays in existing `cross_factor_pnl` bucket).

### 6.4 Routing and ergonomics

`AttributionSpec` gains:
```rust
pub credit_factor_model: Option<CreditFactorModelRef>,
pub credit_factor_detail_options: CreditFactorDetailOptions,

pub struct CreditFactorDetailOptions {
    pub include_per_issuer_adder: bool,    // default false (large portfolio payload control)
    pub include_per_bucket_breakdown: bool, // default true
}
```

`CreditFactorModelRef` accepts inline serialized model or path/handle.

### 6.5 Performance

Waterfall has `L+2` reprice steps for credit (PC + L levels + Adder) versus 1 today. For typical L = 1–3 and portfolios of thousands of instruments, acceptable. MetricsBased and Taylor remain available — linear, no reprice — for cost-sensitive use cases.

## 7. Carry Decomposition

Two parallel lenses on the same total carry, both populated only when a `CreditFactorModel` is provided.

### 7.1 Lens 1 — extended source-cut (`CarryDetail`)

```rust
pub struct CarryDetail {
    pub coupon_income: SourceLine,    // total preserved; rates_part / credit_part populated with model
    pub roll_down:     SourceLine,    // same pattern
    pub pull_to_par:   SignedAmount,  // unsplit for v1
    pub funding_cost:  SignedAmount,  // pure rates, never split
    pub theta:         SignedAmount,  // residual catch-all, unsplit
}

pub struct SourceLine {
    pub total: SignedAmount,
    pub rates_part: Option<SignedAmount>,   // None when no model
    pub credit_part: Option<SignedAmount>,  // None when no model
}
```

When no model: `SourceLine` collapses to scalar (current behavior preserved).

### 7.2 Lens 2 — factor-cut (`CreditCarryDecomposition`)

```rust
pub struct CreditCarryDecomposition {
    pub model_id: String,
    pub rates_carry_total: SignedAmount,
    pub credit_carry_total: SignedAmount,
    pub credit_by_level: CreditCarryByLevel,
}

pub struct CreditCarryByLevel {
    pub generic: SignedAmount,
    pub levels: Vec<LevelCarry>,
    pub adder_total: SignedAmount,
    pub adder_by_issuer: Option<HashMap<IssuerId, SignedAmount>>,
}
```

### 7.3 Per-level carry math

For each issuer `i` over period `dt`:

```
coupon_credit_per_level:
    generic_share = β_i^PC × L_PC × notional_i × dt
    level_k_share = β_i^<level_k> × L_<level_k>(g_i^k) × notional_i × dt
    adder_share = adder_at(i, as_of) × notional_i × dt

roll_credit_per_level:
    generic_share = 0           (level factors are scalar — no term-structure contribution under v1)
    level_k_share = 0
    adder_share = duration_i × (adder_curve(i, T) − adder_curve(i, T − dt))
                  // all credit roll-down → adder under v1 design
```

`notional × dt` follows the existing carry engine's DTS conventions.

### 7.4 Reconciliation invariants (tested)

1. `coupon_income.total ≡ rates_part + credit_part` (model present)
2. `roll_down.total ≡ rates_part + credit_part` (model present)
3. `credit_carry_total ≡ Σ_lines SourceLine.credit_part`
4. `credit_carry_total ≡ generic + Σ_levels(level.total) + adder_total`
5. `rates_carry_total ≡ Σ_lines SourceLine.rates_part − funding_cost`

### 7.5 Rates/credit split per source line

- **`coupon_income`:** at as-of, decompose coupon yield into base-rate + spread (using base discount curve and issuer hazard curve at the bond's tenor); allocate coupon × notional × dt as `base_rate / total_yield` and `spread / total_yield`. Small new helper inside the carry calculator.
- **`roll_down`:** existing roll-down already separates rates_curve_old/new and spread_curve_old/new internally; splitting them is a small internal refactor.

## 8. Volatility Forecasting

### 8.1 Math

For each common factor `k`:
```
σ_k(t, h) = forecast_variance(vol_state.factors[k], horizon = h)
D(t, h) = diag(σ_k(t, h))
Σ_factor(t, h) = D(t, h) · ρ · D(t, h)
```

For each issuer `i`:
```
σ_idio_i(t, h) = forecast_variance(vol_state.idiosyncratic[i], horizon = h)  // IssuerBeta
σ_idio_i(t, h) = adder_vol_annualized                                          // BucketOnly
```

Per-issuer total spread variance:
```
σ²_spread_i(t, h) = β_i^T · Σ_factor(t, h) · β_i + σ²_idio_i(t, h)
```

### 8.2 Wiring layer

```rust
pub struct FactorCovarianceForecast<'a> {
    model: &'a CreditFactorModel,
}

impl<'a> FactorCovarianceForecast<'a> {
    pub fn covariance_at(&self, horizon: VolHorizon) -> Result<FactorCovarianceMatrix>;
    pub fn idiosyncratic_vol(&self, issuer_id: &IssuerId, horizon: VolHorizon) -> Result<f64>;
    pub fn factor_model_at(&self, horizon: VolHorizon, risk_measure: RiskMeasure) -> Result<FactorModel>;
}

pub enum VolHorizon {
    OneStep,
    NSteps(usize),
    Unconditional,
    Custom(Box<dyn FnOnce(...)>),
}
```

The constructed `FactorCovarianceMatrix(t, h)` feeds the existing `FactorModel.analyze()` unchanged. Downstream `ParametricDecomposer` produces:
- `total_risk` per configured `RiskMeasure`
- `factor_contributions` (Euler-allocated per factor → per credit level after grouping)
- `position_factor_contributions` (per-issuer per-factor)

### 8.3 Vol contribution by credit level

```rust
pub struct CreditVolReport {
    pub total: f64,
    pub measure: RiskMeasure,
    pub generic: f64,
    pub by_level: Vec<LevelVolContribution>,
    pub idiosyncratic_total: f64,
    pub by_position_optional: Option<Vec<PositionVolContribution>>,
}

pub struct LevelVolContribution {
    pub level_name: String,
    pub total: f64,
    pub by_bucket: HashMap<BucketId, f64>,
}
```

Built by walking `RiskDecomposition.factor_contributions` and grouping by level prefix of the factor ID. Pure aggregation.

### 8.4 `RiskDecomposition` extension

Add a sibling `position_residual_contributions: Vec<PositionResidualContribution>` field for per-issuer idiosyncratic breakout. Non-breaking additive change.

### 8.5 Performance

GARCH one-step is O(1) per factor; multi-step O(h). Σ(t) construction is sub-millisecond for ~100 factors. Dominant cost remains `ParametricDecomposer.decompose()` at O(K² + N·K), unchanged.

## 9. Schema, Bindings, Parity Contract

### 9.1 JSON schemas

| Schema | Status |
|---|---|
| `factor_model/credit_factor_model.schema.json` (`finstack.credit_factor_model/1`) | NEW |
| `factor_model/credit_calibration_inputs.schema.json` | NEW |
| `factor_model/credit_calibration_config.schema.json` | NEW |
| `attribution/1/attribution.schema.json` | EXTENDED (additive: `credit_factor_detail`, extended `carry_detail`) |
| `attribution/1/attribution_result.schema.json` | EXTENDED (additive) |

All schema additions are **additive and optional**. No breaking changes.

### 9.2 Python bindings (`finstack-py/src/bindings/valuations/`)

```
factor_model/
  credit_factor_model.rs      # NEW — wrapper, from_json/to_json, accessors
  credit_calibrator.rs        # NEW — calibrate() entry point
  credit_decomposition.rs     # NEW — decompose_levels, decompose_period
  credit_vol_forecast.rs      # NEW — FactorCovarianceForecast wrapper
attribution/
  attribution.rs              # EXTENDED — accepts credit_factor_model param
```

Wrapper pattern: `pub(crate) inner: RustType` + `from_inner()`. Errors via central `core_to_py()`.

### 9.3 WASM bindings (`finstack-wasm/src/api/valuations/`)

Mirror Python module structure. `js_name` mappings: `creditFactorModel`, `creditCalibrator`, `decomposeLevels`, `decomposePeriod`, `factorCovarianceForecast`. Public API exposed via `index.js` facade.

### 9.4 Parity contract

`parity_contract.toml` updates: every new public type and function gets an entry asserting Rust/Python/WASM name alignment. Names follow project triplet convention (Rust/Python `snake_case` identical; WASM `camelCase`).

### 9.5 Canonical names

| Concept | Rust / Python | WASM |
|---|---|---|
| Calibrated artifact | `CreditFactorModel` | `CreditFactorModel` |
| Calibrator | `CreditCalibrator` | `CreditCalibrator` |
| Calibration inputs | `CreditCalibrationInputs` | `CreditCalibrationInputs` |
| Calibration config | `CreditCalibrationConfig` | `CreditCalibrationConfig` |
| Hierarchy spec | `CreditHierarchySpec` | `CreditHierarchySpec` |
| Per-period decomposition | `decompose_period` / `decompose_levels` | `decomposePeriod` / `decomposeLevels` |
| Vol forecast wrapper | `FactorCovarianceForecast` | `FactorCovarianceForecast` |
| Attribution result field | `credit_factor_detail` | `creditFactorDetail` |
| Carry result field | `credit_carry_decomposition` | `creditCarryDecomposition` |
| Per-issuer beta row | `IssuerBetaRow` | `IssuerBetaRow` |

### 9.6 Notebook

One new notebook: `finstack-py/examples/notebooks/credit_factor_hierarchy.ipynb`. Covers: build calibration inputs from a synthetic panel → calibrate → attribute a sample portfolio → vol forecast report → load/save the artifact JSON.

## 10. Testing Strategy

### 10.1 Calibration tests (`finstack/valuations/tests/factor_model/calibration/`)

- Determinism golden (bit-identical artifact for same inputs)
- Round-trip JSON
- Mode classification (Dynamic, GloballyOff, per-issuer override)
- Sparse history graceful (0 / 1 / `min_history − 1` observations)
- Bucket fold-up at every level
- All-bucket-only path
- Empty hierarchy `[]`
- Single-level hierarchy `[Rating]`
- Per-issuer panel gaps
- Adder-vol fallback chain (caller > peer proxy > parent > default)
- Diagnostic completeness

### 10.2 Per-period decomposition tests (`tests/factor_model/decomposition/`)

- **Reconciliation invariant**: `Σ_levels (β × ΔF) + Δadder ≡ ΔS_i` for every issuer, tolerance 1e-10
- Universe drift (new issuer at consumption)
- Tag missing → clean error
- Empty bucket at as-of degrades gracefully

### 10.3 Attribution tests (`tests/attribution/credit_factor/`)

- **Reconciliation invariant per method**: `generic_pnl + Σ_levels(level.total) + adder_pnl_total ≡ credit_curves_pnl`, tolerance 1e-8
- Cross-method consistency (Waterfall = Taylor under linearity; Parallel deviates by cross-effects only)
- No-model fallback (existing behavior preserved)
- Per-issuer adder gating
- Hierarchy variation (same portfolio, different specs → different decomposition, identical `credit_curves_pnl`)

### 10.4 Carry tests (`tests/attribution/carry_credit_factor/`)

All five reconciliation invariants from §7.4. All-credit-roll-to-adder (under v1 scalar level factors). No-model fallback.

### 10.5 Vol forecast tests (`tests/factor_model/vol_forecast/`)

- Σ(t) PSD
- `OneStep` vs `Unconditional` consistency
- Vol decomposition aggregation
- `BucketOnly` issuer vol = cached scalar regardless of horizon
- Multi-step GARCH matches closed-form

### 10.6 Cross-cutting

- Synthetic panel end-to-end (recover known true betas + variances within statistical tolerance)
- Real-world golden (small fixed panel, artifact JSON checked in)
- Performance bench (calibration 500 issuers × 60 months × 3 levels under 5s; attribution + vol forecast on 200-position portfolio under 100ms — targets to be confirmed during implementation)
- Parity test (Rust/Python/WASM parity to 1e-10 via existing `finstack-py/tests/parity` framework)

### 10.7 Property-based tests (`proptest`)

- Random portfolios + random calibration → reconciliation invariants hold
- Random Δspreads → `decompose_period` reconciliation
- Random hierarchy specs → calibration completes without error

### 10.8 Out of scope for testing

- Numerical agreement with external reference implementations (no canonical exists)
- Stress-scenario consistency across methods (v2)

## 11. Out of Scope (v2 Candidates)

| Topic | Why deferred |
|---|---|
| Term-structure level factors | v1 uses scalar level factors; revisit when meaningful credit roll-down by level becomes a need. |
| PCA-derived generic factor | User chose user-designated observable; PCA infra exists if needed. |
| Multivariate / DCC GARCH | Static `ρ` + per-factor GARCH covers 90% of value at 10% cost. |
| Online covariance updating | Calibrator is sole writer; recompute per-call from `vol_state`. |
| Joint loadings calibration with regularization | Sequential is more interpretable; revisit if order-dependence problematic. |
| Ledoit-Wolf shrinkage estimator | Diagonal Σ + ridge covers v1; LW is a refinement. |
| `core::math::regression` general utility | `analytics::benchmark::beta` covers single-factor needs. |
| Stress-scenario consistency across methods | Tested on observed periods only. |
| Per-issuer adder term structure | v1 uses single anchor + scalar vol. |
| FRTB / regulatory adapter | Separate adapter likely needed if requested. |

## 12. Summary of New Pieces

**New types** (Rust → Python → WASM, all exposed):
- `CreditFactorModel`, `CreditCalibrator`, `CreditCalibrationInputs`, `CreditCalibrationConfig`
- `CreditHierarchySpec`, `HierarchyDimension`, `IssuerTags`, `IssuerBetaPolicy`, `IssuerBetaMode`
- `IssuerBetaRow`, `LevelsAtAnchor`, `VolState`, `FactorHistories`, `CalibrationDiagnostics`
- `FactorCovarianceForecast`, `VolHorizon`, `CreditVolReport`
- `CreditFactorAttribution`, `LevelPnl`, `CreditCarryDecomposition`, `LevelCarry`, `SourceLine`

**New free functions:**
- `decompose_levels`, `decompose_period`

**Extended types:**
- `PnlAttribution` — adds `credit_factor_detail`, extends `carry_detail`
- `CarryDetail` — `coupon_income` and `roll_down` become `SourceLine`
- `RiskDecomposition` — adds `position_residual_contributions`
- `MarketMapping` — new `CreditHierarchical` variant
- `AttributionSpec` — adds `credit_factor_model` and `credit_factor_detail_options`

**New schemas:** `factor_model/credit_factor_model.schema.json` (and inputs/config siblings); minor bumps to attribution schemas (additive).

**New module placement:**
- `core::factor_model::credit_hierarchy` — primitives, artifact envelope, `MarketMapping::CreditHierarchical`
- `valuations::factor_model::credit_calibration` — calibrator
- `valuations::factor_model::credit_decomposition` — `decompose_*`
- `valuations::factor_model::credit_vol_forecast` — Σ(t) wiring
- `valuations::attribution::credit_factor` — attribution + carry integration

**No new general utilities required.** `analytics::benchmark::beta`, `core::math::stats::covariance`, `valuations::correlation::nearest_correlation` cover everything; Ledoit-Wolf and a general regression module are v2 nice-to-haves only.

## 13. Open Questions / TBD During Implementation

- Exact performance targets (calibration runtime, attribution+vol per call) — to be confirmed against representative portfolios.
- Schema version-bump format for attribution schemas (`/1.1` vs sibling file) — match existing project convention.
- Whether `CreditFactorModelRef` accepts inline JSON, file path, or both — confirm with binding consumers.
