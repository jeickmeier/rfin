# `/statements` Crate — Technical Design (Core‑only)

**Status:** Draft (implementation‑ready)
**Last updated:** 2025‑01‑25
**MSRV:** 1.75 (target)
**License:** Apache‑2.0 (project standard)

---

## 1) Purpose & Scope

The `statements` crate provides a deterministic, currency‑aware financial statements engine that models business metrics as a directed graph of nodes evaluated over discrete periods. It relies exclusively on the capabilities of `finstack-core`:

- Period system (`Period`, `PeriodPlan`, parsing) and calendar/day‑count utilities
- Strong money/types (`Amount`, `Currency`, `Rate`) and numeric policies
- Expression engine (AST, compilation, evaluation, context)
- Validation framework (optional) for model checks
- Polars time‑series via core prelude re‑exports for vectorized per‑period evaluation

Out of scope: pricing/valuations, portfolio aggregation, scenario engines, Arrow/Parquet IO. Those live in sibling crates and consume the outputs of `statements`.

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

---

## 11) Deliverables & Next Steps

1) Implement `ModelBuilder`, `Evaluator`, and `StatementContext` using core.
2) Implement minimal `ForecastMethod`s with Decimal arithmetic and idempotent expansion.
3) Provide a small built‑in metrics namespace `fin.basic` in `Registry`.
4) Add unit/property/golden tests; ensure determinism gates are wired through core features.
5) Document the wire format and provide examples alongside tests.

---

## 12) Compatibility & Feature Flags

- Mirrors `finstack-core` numeric flags: `deterministic`, `fast_f64`. No independent numeric policies.
- No optional third‑party backends. Polars usage is via core re‑exports only.

---

This document defines the core‑only `statements` crate: a deterministic, currency‑safe statements engine that composes the `finstack-core` period system, expression engine, numeric policies, validation, and Polars re‑exports without introducing cross‑crate domain coupling.


