# `/statements` Crate — Technical Design (Core‑only)

**Status:** Draft (implementation‑ready)
**Last updated:** 2025‑01‑25
**MSRV:** 1.75 (target)
**License:** Apache‑2.0 (project standard)

---

## 1) Purpose & Scope

The `statements` crate provides a deterministic, currency‑aware financial statements engine that models business metrics as a directed graph of nodes evaluated over discrete periods. It also provides specialized real estate underwriting capabilities including property cash flow modeling, construction loan tracking, and equity waterfall allocation. It relies exclusively on the capabilities of `finstack-core`:

- Period system (`Period`, `PeriodPlan`, parsing) and calendar/day‑count utilities
- Strong money/types (`Amount`, `Currency`, `Rate`) and numeric policies
- Expression engine (AST, compilation, evaluation, context)
- Validation framework (optional) for model checks
- Polars time‑series via core prelude re‑exports for vectorized per‑period evaluation

Out of scope: instrument pricing/valuations (except real estate property valuations), portfolio aggregation, scenario engines, Arrow/Parquet IO. Those live in sibling crates and consume the outputs of `statements`.

---

## 2) Crate Boundary & Dependencies

- Hard dependency: `finstack-core` only.
- No direct third‑party dependencies besides `serde`/`indexmap` for stable wire shapes and deterministic maps.
- Parallelism uses core’s policies; the crate itself does not depend on Rayon directly.
- No Arrow/Parquet; any IO mapping is done in the optional `io` crate.

```toml
[dependencies]
finstack-core = { version = ">=0.2", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
indexmap = "2"

[features]
default = ["serde"]
serde = []
deterministic = ["finstack-core/deterministic"]    # mirror numeric determinism
fast_f64 = ["finstack-core/fast_f64"]              # compute policy passthrough
```

---

## 3) Domain Model

### 3.1 Core Types

```rust
// From core
use finstack_core::prelude::{
  Period, PeriodPlan, PeriodId, ResultsMeta, ExpressionContext, CompiledExpr,
  Amount, Currency, Decimal, // Decimal is implied by Amount
  DataFrame, Series, LazyFrame, col, lit, when, // Polars re-exports
};
```

#### Wire vs Runtime Types

```rust
// wire (serde)
#[derive(Clone, Serialize, Deserialize)]
pub struct NodeSpec {
    pub node_id: String,
    pub name: Option<String>,
    pub node_type: NodeType,
    pub values: Option<indexmap::IndexMap<PeriodId, AmountOrScalar>>, // sparse
    pub forecasts: Vec<ForecastSpec>,
    pub formula_text: Option<String>,       // <- string
    pub where_text: Option<String>,         // <- string (boolean mask over periods)
    /// Optional: dedicated schedule definition when `node_type == NodeType::Corkscrew`
    pub schedule: Option<CorkscrewSpec>,
    #[serde(default)]
    pub tags: finstack_core::prelude::TagSet,
    pub meta: indexmap::IndexMap<String, serde_json::Value>,
}

// runtime (internal)
pub struct Node {
    pub spec: NodeSpec,
    pub formula: Option<CompiledExpr>,
    pub where_clause: Option<CompiledExpr>,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum NodeType { Value, Calculated, Mixed, Corkscrew }

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AmountOrScalar {
    Amount(Amount),
    Scalar(Decimal),
}
```

Semantics per period (for `Mixed`):
1) Value if present → 2) else Forecast → 3) else Formula. `where_clause` is a pure boolean mask; it must not alter graph topology.

Semantics for `Corkscrew`:
- Each schedule has `begin`, `flows` (typed legs), and `end` per period.
- Identities enforced: `end[t] = begin[t] + Σ flows[t]` and `begin[t] = end[t-1]` with an explicit first-period anchor.
- Currency rules mirror `Amount` semantics; no implicit FX. Unitless scalars permitted.

#### ForecastSpec (minimal, core‑only)

```rust
#[derive(Clone, Serialize, Deserialize)]
pub struct ForecastSpec {
    pub method: ForecastMethod,
    pub params: indexmap::IndexMap<String, serde_json::Value>,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum ForecastMethod {
    ForwardFill,      // carry last known value into non-actual periods
    GrowthPct,        // v[t] = v[t-1] * (1 + g)
    Override,         // explicit sparse period values in `params` map
}
```

Implementations are strictly deterministic and use only `Decimal` arithmetic from `core` via `Amount`/`Decimal` helpers.

#### CorkscrewSpec and Flow Legs

```rust
#[derive(Clone, Serialize, Deserialize)]
pub struct CorkscrewSpec {
    /// First-period anchor for begin; required for deterministic roll-forward
    pub anchor_begin: AmountOrScalar,
    /// Optional: explicit end for last period to validate reconciliation
    pub terminal_end: Option<AmountOrScalar>,
    /// Named flow legs and their sign conventions
    pub legs: indexmap::IndexMap<String, FlowLegSpec>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FlowLegSpec {
    /// Positive values increase the balance; negative decrease (deterministic sign)
    pub sign: i8, // +1 or -1
    /// Values by period OR formula text to compute leg per period
    pub values: Option<indexmap::IndexMap<PeriodId, AmountOrScalar>>,
    pub formula_text: Option<String>,
}
```

#### FinancialModelSpec & FinancialModel

```rust
// wire (serde)
#[derive(Clone, Serialize, Deserialize)]
pub struct FinancialModelSpec {
    pub id: String,
    pub periods: Vec<Period>,
    pub nodes: indexmap::IndexMap<String, NodeSpec>, // serde-stable, deterministic order
    /// Optional balance sheet articulation policy
    pub bs_articulation: Option<BalanceSheetArticulationSpec>,
    pub meta: indexmap::IndexMap<String, serde_json::Value>,
}

// runtime (internal)
pub struct FinancialModel {
    pub id: String,
    pub periods: Vec<Period>,
    pub nodes: indexmap::IndexMap<String, Node>, // compiled
    pub metrics: Registry,                       // prebuilt, namespaced
    pub bs_articulation: Option<BalanceSheetArticulation>,
    pub meta: indexmap::IndexMap<String, serde_json::Value>,
}

pub struct Registry {
    pub builtins: indexmap::IndexMap<String, CompiledExpr>, // e.g., "fin.gross_margin"
}
```

`Registry` is evaluated like normal nodes and can be overridden/extended by the host. Built‑ins are prefixed with `"fin."` to avoid collisions.

#### Balance Sheet Articulation

```rust
#[derive(Clone, Serialize, Deserialize)]
pub struct BalanceSheetArticulationSpec {
    /// Node IDs that define Assets and Liabilities+Equity sides
    pub assets_nodes: Vec<String>,
    pub liab_eq_nodes: Vec<String>,
    /// Ordered list of candidate plug node IDs to resolve residuals
    pub plug_candidates: Vec<String>,
    /// Optional absolute tolerance for Decimal comparisons (default zero)
    #[serde(default)]
    pub tolerance: Option<Decimal>,
}

pub struct BalanceSheetArticulation {
    pub assets_nodes: Vec<String>,
    pub liab_eq_nodes: Vec<String>,
    pub plug_candidates: Vec<String>,
    pub tolerance: Decimal,
}
```

#### Evaluation Results

```rust
#[derive(Clone, Serialize, Deserialize)]
pub struct Results {
    pub nodes: indexmap::IndexMap<String, indexmap::IndexMap<PeriodId, Decimal>>, // currency-preserving inputs collapse to Decimal per policy
    pub periods: Vec<Period>,
    pub meta: ResultsMeta, // numeric_mode, parallel, seed, model_currency (optional), rounding
}
```

Polars DataFrame outputs (required):

```rust
impl Results {
    /// Long format: (node_id, period_id, value)
    pub fn to_polars_long(&self) -> polars::prelude::DataFrame;

    /// Wide format: periods as rows, nodes as columns
    pub fn to_polars_wide(&self) -> polars::prelude::DataFrame;
}
```

Currency handling policy: inputs may be `Amount` (currency‑aware). Unless explicitly converted with an `FxProvider` inside node formulas, output values are unitless `Decimal` or model‑currency per policy flagged in `meta.model_currency` when collapse is explicit.

Articulation metadata:
- Record per‑period chosen plug node and value in `meta.bs_articulation`.
- If residual after all plugs is non‑zero beyond tolerance, raise a typed error and include the residual in error context.

---

## 3.5) Real Estate Underwriting Extensions

The statements engine provides specialized node types for real estate underwriting that extend the core statement model:

### 3.5.1 Property Cash Flow Nodes

```rust
use finstack_core::prelude::*;

/// Specialized node type for property cash flow modeling
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PropertyNodeSpec {
    pub node_id: String,
    pub property_spec: PropertySpec,
    /// Optional discount rate for NPV calculation
    pub discount_rate: Option<Decimal>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PropertySpec {
    pub currency: Currency,
    pub leases: Vec<LeaseSpec>,
    #[serde(default)]
    pub opex: Vec<OpexSpec>,
    #[serde(default)]
    pub taxes: Option<PropertyTaxSpec>,
    #[serde(default)]
    pub reserves: Option<ReserveSpec>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LeaseSpec {
    pub tenant: String,
    pub start: time::Date,
    pub end: time::Date,
    pub area_sqft: Decimal,
    pub base_rent: Vec<RentStep>,
    #[serde(default)]
    pub indexation: Option<IndexationSpec>,
    #[serde(default)]
    pub free_rent: Vec<FreeRentWindow>,
    #[serde(default)]
    pub renewal: Option<RenewalOption>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RentStep {
    pub start: time::Date,
    pub amount_per_sqft_per_year: Decimal,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexationSpec {
    pub index_node: String, // Reference to CPI/RPI node in statements
    pub lag_periods: i32,
    pub interpolation: IndexInterpolationType,
    pub cap: Option<Decimal>,
    pub floor: Option<Decimal>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum IndexInterpolationType {
    Linear,
    None, // Use value as-is
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FreeRentWindow {
    pub start: time::Date,
    pub end: time::Date,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RenewalOption {
    pub probability: Decimal,
    pub term_months: i32,
    #[serde(default)]
    pub rent_bump_pct: Option<Decimal>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OpexSpec {
    Fixed { name: String, amount_per_period: AmountOrScalar, frequency: Frequency },
    PercentOfRent { name: String, pct: Decimal },
    PerArea { name: String, amount_per_sqft_per_year: Decimal },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PropertyTaxSpec {
    pub assessed_value: Decimal,
    pub mill_rate: Decimal,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReserveSpec {
    pub accrual_per_period: AmountOrScalar,
    pub permitted_uses: Vec<String>,
}
```

### 3.5.2 Construction Loan Nodes

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConstructionLoanSpec {
    pub node_id: String,
    pub borrower: String,
    pub commitment: AmountOrScalar,
    pub draw_schedule: Vec<DrawEvent>,
    pub interest_rate_node: String, // Reference to rate node in statements
    pub fees: Vec<FeeSpec>,
    pub reserve: InterestReserveSpec,
    pub capitalization: CapitalizationPolicy,
    pub convert_to_term_on: Option<time::Date>,
    #[serde(default)]
    pub conversion: Option<TermConversionSpec>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DrawEvent {
    pub date: time::Date,
    pub amount: AmountOrScalar,
    pub purpose: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeeSpec {
    pub fee_type: String,
    pub amount: AmountOrScalar,
    pub payment_date: FeePaymentDate,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FeePaymentDate {
    Upfront,
    AtMaturity,
    Periodic(Frequency),
    OnEvent(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InterestReserveSpec {
    pub initial_funding: AmountOrScalar,
    pub top_up_on_shortfall: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum CapitalizationPolicy {
    CapitalizeInterest,
    PayFromReserveThenCapitalize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TermConversionSpec {
    pub term_maturity: time::Date,
    pub amortization_node: String, // Reference to amortization schedule node
    pub new_interest_rate_node: String, // Reference to term rate node
}
```

### 3.5.3 Equity Waterfall Nodes

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WaterfallSpec {
    pub node_id: String,
    pub tiers: Vec<WaterfallTier>,
    #[serde(default)]
    pub clawback: Option<ClawbackSpec>,
    pub contributions_node: String, // Reference to contributions node
    pub distributable_node: String, // Reference to distributable cash flow node
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WaterfallTier {
    pub hurdle: Hurdle,
    pub split: SplitSpec,   // e.g., 80/20 LP/GP
    #[serde(default)]
    pub catch_up: Option<CatchUpSpec>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Hurdle { 
    IRR(Decimal), 
    Multiple(Decimal),
    /// Reference to a statements node for dynamic hurdle
    Node(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SplitSpec { 
    pub lp_pct: Decimal, 
    pub gp_pct: Decimal 
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CatchUpSpec { 
    pub gp_pct: Decimal 
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClawbackSpec { 
    pub lookback_periods: i32 
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AllocationResult {
    pub period: PeriodId,
    pub distributable: AmountOrScalar,
    pub allocated_lp: AmountOrScalar,
    pub allocated_gp: AmountOrScalar,
    pub tier_index: usize,
}
```

Notes:
- Real estate nodes integrate with the core statements expression engine
- Property cash flows reference CPI/inflation nodes within the same statements model
- Construction loans reference interest rate and amortization nodes
- Equity waterfalls reference contribution and distributable cash flow nodes
- All real estate calculations preserve currency safety and deterministic evaluation

---

## 4) Builder & Public API

### 4.1 ModelBuilder (type‑state pattern)

```rust
pub struct ModelBuilder<S> { /* private fields, caches */ }
pub struct NeedPeriods;
pub struct Ready;

impl ModelBuilder<NeedPeriods> {
    pub fn new(id: impl Into<String>) -> Self;
    pub fn periods(self, range:&str, actuals:Option<&str>) -> Result<ModelBuilder<Ready>, BuildError>;
}

impl ModelBuilder<Ready> {
    pub fn value(self, node_id:&str, values:&[(PeriodId, AmountOrScalar)]) -> Self;
    pub fn forecast(self, node_id:&str, spec:ForecastSpec) -> Self;
    /// Registers a computed node using wire strings that are compiled at build time
    pub fn compute(self, node_id:&str, formula_text:&str, where_text:Option<&str>) -> Result<Self, BuildError>;
    pub fn register_metrics(self, ns:&str) -> Result<Self, BuildError>; // loads built-ins under `fin.` and optional namespaces
    /// Consumes a FinancialModelSpec, compiles formulas, and returns a runtime FinancialModel
    pub fn from_spec(self, spec: FinancialModelSpec) -> Result<Self, BuildError>;
    /// Declare a corkscrew schedule node with legs and anchors
    pub fn corkscrew(self, node_id:&str, spec: CorkscrewSpec) -> Result<Self, BuildError>;
    /// Configure Balance Sheet articulation and plug selection
    pub fn bs_articulation(self, spec: BalanceSheetArticulationSpec) -> Result<Self, BuildError>;
    /// Add a property cash flow node for real estate modeling
    pub fn property(self, node_id:&str, spec: PropertyNodeSpec) -> Result<Self, BuildError>;
    /// Add a construction loan tracking node
    pub fn construction_loan(self, node_id:&str, spec: ConstructionLoanSpec) -> Result<Self, BuildError>;
    /// Add an equity waterfall allocation node
    pub fn equity_waterfall(self, node_id:&str, spec: WaterfallSpec) -> Result<Self, BuildError>;
    pub fn build(self) -> Result<FinancialModel, BuildError>;
}
```

Notes:
- `periods(...)` delegates to `core::time::build_periods` and stamps `actual_set` for forecast extension rules.
- `compute(...)` accepts `formula_text`/`where_text` strings and compiles them to `CompiledExpr`.
- `from_spec(...)` compiles all `NodeSpec` entries (formula/where) into runtime `Node`s.
- Deterministic ordering: `IndexMap` insertion order produces a stable topo build.
- `corkscrew(...)` validates legs and anchors and expands into a deterministic per-period vector with begin/flows/end.
- `bs_articulation(...)` does not mutate user node values; it only determines which plug candidate is active per period and records that choice. If a candidate has an explicit Value for a period, it is skipped.
- `property(...)` creates specialized property cash flow nodes that generate rent, opex, taxes, and reserve flows per period.
- `construction_loan(...)` creates construction loan tracking nodes with interest reserve management and conversion handling.
- `equity_waterfall(...)` creates equity allocation nodes that distribute cash flows according to waterfall specifications.

### 4.2 Evaluator

```rust
pub struct Evaluator { /* caches keyed by content hashes */ }

impl Evaluator {
    pub fn evaluate(&self, model:&FinancialModel, parallel: bool) -> Result<Results, EvalError>;
}
```

Execution model:
- Build a DAG from `nodes + registry` using the core expression engine and a `StatementContext` implementing `ExpressionContext`.
- Evaluate per period using Polars `DataFrame`/`Series` primitives re‑exported from core for vectorization. Parallel layers are optional and must be byte‑identical to serial in deterministic mode.
- Caching: content‑addressed caches for compiled formulas and forecast expansions; precise invalidation when model content changes.

Articulation pass:
- After computing all non‑plug nodes for a period, compute residual `assets_total - (liabilities_total + equity_total)`.
- Select the first plug candidate without an explicit user Value for that period and set its Formula to the residual.
- If no candidates are available or residual exceeds tolerance after selection, raise `EvalError::ArticulationFailure`.

Corkscrew pass:
- For each `Corkscrew` node, validate begin/end identities. If violated, raise `EvalError::CorkscrewInconsistent` with period context.

#### Expression Context

```rust
pub struct StatementContext<'m> { pub model: &'m FinancialModel, pub period_ix: usize }

impl<'m> ExpressionContext for StatementContext<'m> {
    type Value = Decimal; // unitless or model currency per node policy
    fn resolve(&self, name:&str) -> Option<Self::Value> { /* resolve node values for current period */ }
}
```

---

## 5) Determinism, Currency Safety, and Policies

- Determinism: stable topo order, seeded caches, and identical serial/parallel execution in `deterministic` mode (delegated to core policies).
- Currency safety: arithmetic on `Amount` requires same `ccy`. Any cross‑currency operation must be explicit via formulas that use `core::money::FxProvider` passed by the host. The statements crate does not do implicit FX.
- FX policy default: when statements convert `Amount` values to a model currency, the default is `FxConversionPolicy::PeriodEnd`. Overrides (e.g., period average) must be explicit in node metadata and MUST be stamped into `Results.meta.fx_policies["statements"]` as a `FxPolicyMeta` entry.
- Missingness semantics: arithmetic with `None` → `None` unless `coalesce(x,0)` is used. Division by zero yields `None` and logs a typed error via `tracing`.
- Articulation determinism: plug selection is content‑addressed by candidate list order and presence of explicit user values per period.
- Corkscrew determinism: identities enforced exactly in Decimal mode; tolerance only applies where explicitly configured.

---

## 6) Serialization & Wire Format

- Serde coverage is limited to wire types with stable field names: `FinancialModelSpec`, `NodeSpec`, `ForecastSpec`, `AmountOrScalar`, etc.
- `#[serde(deny_unknown_fields)]` for inbound model documents.
- Top‑level envelopes include `schema_version` (managed at the meta‑crate level) and `ResultsMeta` from core.

Notes:
- Compiled artifacts (`CompiledExpr`, runtime `Node`, runtime `FinancialModel`) are never serialized.
- This avoids version‑sensitive wire formats and reduces cross‑FFI friction (Python/WASM).
 - `CorkscrewSpec`, `FlowLegSpec`, and `BalanceSheetArticulationSpec` are wire types with strict names; unknown fields denied.

---

## 7) Observability

- `tracing` spans for build/evaluate phases with `model_id`, `node_id`, and `period` labels.
- Optional JSON logs controlled by environment variables (delegated to the host).

---

## 8) Testing Strategy

- Unit: node resolution order, `where_clause` masking, forecast methods, expression parsing/compilation errors.
- Property: determinism (serial ≡ parallel in Decimal mode), idempotent forecast expansion, `rebuild(serialize(model)) == model`.
- Golden: metrics registry outputs against fixed fixtures; serialized snapshots with `schema_version`.
- Compile‑time: trybuild for expression macros if provided by core (e.g., `pct!`, `bp!`).
- Articulation: fixtures where explicit values block candidate selection; verify residuals exactly zero within tolerance.
- Corkscrew: fixtures with begin/flows/end and cross‑period continuity; assert typed errors when identities break.

---

## 9) Example Usage (Rust)

```rust
use finstack_core::prelude::*;
use finstack_statements::*;

let model = ModelBuilder::new("Acme")
    .periods("2025Q1..2026Q4", Some("2025Q1..Q2"))?
    .value("revenue", &[(PeriodId("2025Q1".into()), AmountOrScalar::Scalar(dec!(100)))])
    .compute("gross_margin", "gross_profit / revenue", None)?
    .corkscrew("ppe", CorkscrewSpec {
        anchor_begin: AmountOrScalar::Scalar(dec!(1000)),
        terminal_end: None,
        legs: indexmap::indexmap!{
            "additions".into() => FlowLegSpec { sign: 1, values: None, formula_text: Some("capex".into()) },
            "depreciation".into() => FlowLegSpec { sign: -1, values: None, formula_text: Some("dep_expense".into()) },
        }
    })?
    .bs_articulation(BalanceSheetArticulationSpec{
        assets_nodes: vec!["cash".into(), "ppe".into()],
        liab_eq_nodes: vec!["debt".into(), "equity".into()],
        plug_candidates: vec!["cash".into(), "retained_earnings".into()],
        tolerance: None,
    })?
    .register_metrics("fin.basic")?
    .build()?;

let evaluator = Evaluator::new();
let out = evaluator.evaluate(&model, /* parallel */ false)?;
```

---

## 10) Acceptance Criteria

- Node semantics honored per period: Value > Forecast > Formula.
- `where_clause` acts as boolean mask only; no topology changes.
- Deterministic evaluation; Decimal mode serial == parallel.
- Currency‑safety enforced; no implicit FX.
- Stable serde for all public types; snapshots gated by schema version.
- Metrics registry namespaced and collision‑safe (`fin.*`).
- Real estate nodes: property cash flows generate rent/opex/taxes/reserves correctly; construction loans track interest reserves and conversions; equity waterfalls produce deterministic allocation ledgers.

---

## 11) Deliverables & Next Steps

1) Implement `ModelBuilder`, `Evaluator`, and `StatementContext` using core.
2) Implement minimal `ForecastMethod`s with Decimal arithmetic and idempotent expansion.
3) Provide a small built‑in metrics namespace `fin.basic` in `Registry`.
4) Implement real estate node types: PropertyNodeSpec, ConstructionLoanSpec, and WaterfallSpec.
5) Add unit/property/golden tests; ensure determinism gates are wired through core features.
6) Document the wire format and provide examples alongside tests.

---

## 12) Compatibility & Feature Flags

- Mirrors `finstack-core` numeric flags: `deterministic`, `fast_f64`. No independent numeric policies.
- No optional third‑party backends. Polars usage is via core re‑exports only.

---

This document defines the core‑only `statements` crate: a deterministic, currency‑safe statements engine that composes the `finstack-core` period system, expression engine, numeric policies, validation, and Polars re‑exports without introducing cross‑crate domain coupling.


