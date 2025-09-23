# Finstack (Rust) — Full Technical Design

**Version:** 3.0 (comprehensive)
**Status:** Design complete
**Audience:** Library authors, maintainers, and advanced integrators (Python/WASM)

---

## 0) Scope, Goals, and Non‑Goals

### 0.1 Goals

* **Deterministic, reproducible results** across platforms in default (Decimal) mode.
* **Currency‑safe computation**: no silent cross‑currency arithmetic or aggregation.
* **Clear modular architecture** with feature‑gated subcrates and a single publishable meta‑crate.
* **Stable wire format**: 100% serde coverage for all public shapes used in Python/WASM.
* **High‑performance evaluation** of statements, valuations, and scenarios; parallel when requested without changing results (in Decimal mode).
* **Portfolio‑level aggregation** across many entities, positions, books, tags, and scenarios.

### 0.2 Non‑Goals

* Not a full accounting ledger or GL system.
* No real‑time market data connectivity (inputs are provided by the host).
* No GUI; bindings are headless (Python/WASM).

### 0.3 Glossary (selected)

* **Entity**: Logical unit with statements and/or instruments.
* **Model**: Financial statements graph for an entity.
* **CapitalStructure**: Container of instruments for an entity.
* **Book**: A node (folder) in a hierarchical portfolio tree.
* **Position**: A holding in a specific instrument of an entity.
* **Plan/Periods**: Ordered evaluation periods (quarterly/monthly/yearly).

---

## 1) Architecture Overview

```
Workspace
┌────────────────────┐
│   finstack (meta)  │  -> single publishable crate, re-exports below via features
└─────────┬──────────┘
         │
 ┌────────┴──────────────────────────────────────────────────────────────────────────┐
 │ Subcrates                                                                         │
 │                                                                                   │
 │  core         ← foundation: types, expr engine, FX, periods, validation framework│
 │  statements   ← financial statement graph/evaluator (nodes, formulas, metrics)   │
 │  valuations   ← depends on core; uses expr, FX, math from core                    │
 │  structured_credit ← feature-gated; depends on core+valuations; CLO/ABS engine    │
 │  analysis     ← depends on core+statements+valuations                             │
 │  scenarios    ← depends on core+statements+valuations; uses expr from core        │
 │  portfolio    ← depends on core+statements+valuations+scenarios (analysis opt.)   │
 │  io           ← Polars↔Arrow/CSV/Parquet interchange; depends on core for types   │
 │  py           ← pyo3 bindings                                                     │
 │  wasm         ← wasm-bindgen bindings                                             │
 └───────────────────────────────────────────────────────────────────────────────────┘
```

### 1.1 Meta‑crate features and re‑exports

`crates/finstack/Cargo.toml`

```toml
[features]
default = ["core"]
core = ["dep:finstack-core"]
statements = ["core", "dep:finstack-statements"]
valuations = ["core", "dep:finstack-valuations"]
structured_credit = ["valuations", "dep:finstack-structured-credit"]
analysis = ["statements", "valuations", "dep:finstack-analysis"]
scenarios = ["statements", "valuations", "dep:finstack-scenarios"]
portfolio = ["statements", "valuations", "scenarios", "dep:finstack-portfolio"]
io = ["dep:finstack-io"]
all = ["statements", "valuations", "structured_credit", "analysis", "scenarios", "portfolio", "io"]
```

`crates/finstack/src/lib.rs`

```rust
#[cfg(feature = "core")]         pub use finstack_core as core;
#[cfg(feature = "statements")]   pub use finstack_statements as statements;
#[cfg(feature = "valuations")]   pub use finstack_valuations as valuations;
#[cfg(feature = "structured_credit")] pub use finstack_structured_credit as structured_credit;
#[cfg(feature = "analysis")]     pub use finstack_analysis as analysis;
#[cfg(feature = "scenarios")]    pub use finstack_scenarios as scenarios;
#[cfg(feature = "portfolio")]    pub use finstack_portfolio as portfolio;
#[cfg(feature = "io")]           pub use finstack_io as io;
```

### 1.2 Cross‑cutting Invariants

* **Determinism:** stable topo ordering (IndexMap), seeded RNG, Decimal default, parallel ≡ serial in Decimal mode.
* **Currency‑safety:** arithmetic on `Amount` requires same ccy; period aggregation is currency‑preserving unless explicitly converted with FX.
* **Cache correctness:** caches keyed by *content hashes*; invalidation precisely scoped.
* **Serde stability:** all public types serialize with stable names; inbound models use `deny_unknown_fields`.
* **Attribute selectors:** instruments and statement nodes expose stable `tags` and `meta` for deterministic selector-based scenarios; selector expansion is deterministic and previewed.
* **Polars DataFrame outputs:** all APIs that produce tabular results MUST also expose a `polars::prelude::DataFrame` form (via `finstack_core::prelude`). Map‑based results remain for serde‑stable wire formats; DataFrames enable zero‑copy interop for Python users.
* **Global rounding & scale policy:** a workspace‑wide config defines ingest/output `rust_decimal` scale and rounding per currency and context. All results stamp a `RoundingContext` in `ResultsMeta` to stabilize CSV/JSON interop across hosts.
* **FX policy visibility:** each layer MUST stamp its FX conversion policy into `ResultsMeta` to avoid silent mismatches (see §2.7 and §17.1).

---

## 2) Core Crate

### 2.1 Numerics, Currency, Amounts

```rust
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum NumericMode { Decimal, FastF64 }

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Amount {
    pub value: rust_decimal::Decimal,
    pub ccy: Currency, // ISO-4217 enum; generated from a table
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Rate(pub rust_decimal::Decimal); // 0.05 == 5%

// FX Infrastructure
pub trait FxProvider: Send + Sync {
    fn rate(&self, from: Currency, to: Currency, on: time::Date) 
        -> Result<Decimal, CoreError>;
}

pub struct FxMatrix {
    // Efficient multi-currency operations with closure checking
}
```

**Requirements**

* `impl Add/Sub for Amount` must **require same `ccy`**; conversions require explicit `FxProvider`.
* `bp!()` and `pct!()` proc‑macros with compile‑time validation; trybuild tests for errors.
* `NumericMode` is stamped into all result envelopes.
* FX conversions are explicit and cached via LRU with configurable TTL.

**AmountOrScalar** (used in statements):

```rust
#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AmountOrScalar {
    Amount(Amount),   // currency-aware
    Scalar(Decimal),  // unitless or model-currency per policy
}
```

### 2.2 Calendars, Day Count, Conventions

* **Calendars:** `enum CalendarId` + data table (schema‑versioned). Override mechanism for ad‑hoc holidays.
* **BDC:** Following, Modified Following, Preceding; **EOM rule**.
* **Day count:** Actual/360, 30/360 (var.), Act/365F, Act/Act (ISDA/ICMA).
* Caching keyed by `(start, end, day_count, convention, calendar)`.

### 2.3 Expression Engine

**Core expression infrastructure for all crates:**

```rust
pub trait ExpressionContext {
    type Value;
    fn resolve(&self, name: &str) -> Option<Self::Value>;
}

pub struct Expr { /* AST nodes */ }
pub struct CompiledExpr { /* optimized DAG */ }

pub struct ExprBuilder {
    // DAG compilation with cycle detection
    // Constant folding and optimization
}
```

### 2.4 Validation Framework

```rust
pub trait Validator {
    type Input;
    type Output;
    fn validate(&self, input: &Self::Input) -> ValidationResult<Self::Output>;
}

pub struct ValidationResult<T> {
    pub value: T,
    pub warnings: Vec<ValidationWarning>,
    pub passed: bool,
}
```

### 2.5 Period System (Parsing, Identity, Plan)

**Types**

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Frequency { Annual, SemiAnnual, Quarterly, Monthly, Weekly, Daily }

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PeriodId(pub String); // "2025Q1", "2025-03", "2025"

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum PeriodKey { Q{year:i32,q:u8}, M{year:i32,m:u8}, Y{i32} }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Period {
    pub id: PeriodId,  // serde-stable
    pub start: time::Date,
    pub end: time::Date,
    pub freq: Frequency,
    pub is_actual: bool,
}
```

**Parser grammar (EBNF)**

```
range      := spec ".." spec
spec       := yq | ym | y
yq         := YYYY "Q" [1-4]
ym         := YYYY "-" MM
y          := YYYY
end-abbr   := "Q" [1-4] | "M" MM | "Y"  (* inherits start year *)
```

* Single range ⇒ single frequency; error on mixing; error if end < start.
* Abbreviated end inherits year (`"2025Q1..Q2"` ⇒ `2025Q1..2025Q2`).

**Builder**

```rust
pub struct PeriodPlan {
    pub all: Vec<Period>,                  // start asc; tiebreak id
    pub actual_set: std::collections::HashSet<PeriodId>,
}
pub fn build_periods(range:&str, actuals:Option<&str>) -> Result<PeriodPlan, Error>;
```

**Properties:** `rebuild(serialize(periods)) == periods`.

### 2.6 Time Series (via Polars)

* **No custom Series<T>** - use Polars Series/DataFrame throughout
* Core re-exports Polars types for consistent usage across all crates
* Expression engine integrates with Polars lazy evaluation when applicable

### 2.7 Errors & Results Metadata

```rust
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum FinstackError { /* PeriodParse, Dag, Formula, Calendar, IO, ... */ }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResultsMeta {
    pub numeric_mode: NumericMode,
    pub parallel: bool,
    pub seed: u64,
    pub model_currency: Option<Currency>,
    pub rounding: core::config::RoundingContext, // captures rounding/scale applied
}
```

In addition, all result envelopes MUST stamp the FX conversion policy(ies) actually used:

```rust
/// Shared FX conversion strategies
pub enum FxConversionPolicy { CashflowDate, PeriodEnd, PeriodAverage, Custom }

/// Metadata describing an applied FX policy
pub struct FxPolicyMeta {
    pub strategy: FxConversionPolicy,
    pub target_ccy: Option<Currency>,
    pub notes: String, // provenance, averaging window, source, etc.
}

/// ResultsMeta MUST include an fx_policies map keyed by layer
/// Required keys when applicable: "valuations", "statements", "portfolio".
/// Example (logical shape): IndexMap<String, FxPolicyMeta>
```

---

## 3) Statements Crate

### 3.1 Node & Semantics (Wire vs Runtime)

```rust
// wire (serde)
#[derive(Clone, Serialize, Deserialize)]
pub struct NodeSpec {
    pub node_id: String,
    pub name: Option<String>,
    pub values: Option<indexmap::IndexMap<PeriodId, AmountOrScalar>>,
    pub forecasts: Vec<ForecastSpec>,
    pub formula_text: Option<String>,       // <- string
    pub where_text: Option<String>,         // <- string
    pub node_type: NodeType,                // Value | Calculated | Mixed
    pub meta: indexmap::IndexMap<String, serde_json::Value>,
}

// runtime (internal)
pub struct Node {
    pub spec: NodeSpec,
    pub formula: Option<core::expr::CompiledExpr>,
    pub where_clause: Option<core::expr::CompiledExpr>,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum NodeType { Value, Calculated, Mixed }
```

**Resolution order per period (for `Mixed`)**

1. **Value** (if present)
2. Else **Forecast** (forward‑fill rules)
3. Else **Formula**

* `actuals` (from `PeriodPlan`) only constrain **forecast extension** (forecasts extend into non‑actuals). They do **not** invent values in actual periods.
* `where_clause` is a boolean mask; it must not change graph topology.

### 3.2 Model & Metrics Registry (Wire vs Runtime)

```rust
// wire (serde)
pub struct FinancialModelSpec {
    pub id: String,
    pub periods: Vec<Period>,
    pub nodes: indexmap::IndexMap<String, NodeSpec>,
}

// runtime (internal)
pub struct FinancialModel {
    pub id: String,
    pub periods: Vec<Period>,
    pub nodes: indexmap::IndexMap<String, Node>,
    pub metrics: Registry, // namespaced prebuilt derived nodes
}
// Built-ins use prefix (e.g., "fin.") to avoid collisions.
```

### 3.3 Evaluation & Results

* Uses core's expression engine for DAG building and evaluation
* Implements `core::expr::ExpressionContext` for statement-specific resolution
* Vectorized per‑period evaluation using Polars DataFrames; optional `rayon`.

```rust
pub struct Results {
    pub nodes: indexmap::IndexMap<String, indexmap::IndexMap<PeriodId, Decimal>>,
    pub periods: Vec<Period>,
    pub meta: ResultsMeta,
}
```

---

## 4) Valuations Crate

### 4.1 Traits & Contracts

```rust
pub trait CashflowProvider {
    fn build_schedule(&self, mkt:&MarketData, as_of: time::Date)
        -> Result<CashflowSchedule, FinstackError>;

    fn aggregate_period_cashflow(
        &self,
        periods: &[Period],
        tags: Option<&TagSet>,
    ) -> Result<indexmap::IndexMap<PeriodId, indexmap::IndexMap<Currency, Decimal>>, FinstackError>;

    fn aggregate_to_model_ccy(
        &self,
        periods: &[Period], tags: Option<&TagSet>,
        fx: &dyn core::money::FxProvider, model_ccy:Currency,
    ) -> Result<indexmap::IndexMap<PeriodId, Decimal>, FinstackError>;
}

pub trait Priceable {
    fn price(&self, mkt:&MarketData, as_of: time::Date) -> Result<ValuationResult, FinstackError>;
}

pub trait RiskMeasurable {
    fn risk_report(&self, mkt:&MarketData, as_of: time::Date, buckets: Option<&[Bucket]>)
        -> Result<RiskReport, FinstackError>;
}
```

**Requirements**

* `aggregate_period_cashflow` **must be currency‑preserving**.
* `aggregate_to_model_ccy` requires FX and a clear conversion policy (e.g., cashflow‑date midpoint; documented per instrument set).

### 4.2 Instrument Taxonomy & Schedule Rules

* **Loans/Notes/Facilities**: PIK/Cash/Combo, amortization, call schedules, fees, floors/caps.
* **Deposit/FRA**: Simple deposit and FRAs
* **IRS & Basis Swaps**: fixed/float legs, observation lags, resets.
* **FX Forwards/Swaps**.
* **FX Spot**: base/quote pair via `FxMatrix`; price is 1 unit of base in quote.
* **CDS, CDS Index**: IMM dates, premium/protection legs, accrual‑on‑default.
* **CDS Options**: CDS Index options.
* **Cap/Floor/Swaptions**: Vanilla interest rate options.
* **Equity Options (vanilla)**: exotics via feature flags.
* **Equity (spot)**: priced from `MarketData.prices` by `InstrumentId` or ticker; may optionally reference an entity's statements for analytics (pricing remains spot).
* **Hybrid Securities**: convertibles, prefs via feature flags

**Shared schedule builder:** BDC, EOM, stubs (short/long), day‑count; **single canonical implementation**.

**Tagging taxonomy** (non‑exhaustive):
`fees`, `interest_cash`, `interest_pik`, `principal_sched`, `prepay`, `premium`, `settlement`, `default`, `mtm`.
All instruments expose `attrs.tags` and `attrs.meta` (e.g., `rating`, `sector`, `seniority`) for scenario selection.

### 4.3 Market Data & Curves

```rust
pub struct MarketData {
    pub as_of: time::Date,
    pub discount: std::collections::HashMap<CurveId, DiscountCurve>,
    pub indices:  std::collections::HashMap<IndexId, RateIndex>,
    pub credit:   std::collections::HashMap<IssuerId, CreditCurve>,
    pub fx: core::money::FxMatrix,  // from core
    pub vol: std::collections::HashMap<SurfaceId, VolSurface>,
    // Spot pricing for equities and other quoted instruments
    pub dividends: std::collections::HashMap<Ticker, DividendSchedule>,
    pub prices: std::collections::HashMap<InstrumentId, Decimal>,
}
```

* Interpolation: linear/monotone‑convex; day‑count aware; cached.
  Cache keys include `(curve id, basis, comp, pillar hash)`.
* FX matrix with explicit base/terms; closure check (A/B × B/C ≈ A/C within tolerance).

### 4.4 Valuation & Risk Outputs

* NPV, clean/dirty, YTM/spread, DV01/CS01, duration/convexity.
* Options greeks; CDS par & risky PV01, OAS & YTC.
* `ValuationResult` includes `numeric_mode`, `as_of`, and `model_currency` (if collapsed).

### 4.5 Period Aggregation

* Group schedule cashflows into `PeriodId` with tags; currency‑preserving first.
* Property test: `sum_by_ccy(aggregate) == sum(schedule)` per tag set.

---

## 5) Analysis Crate (Plugin System)

### 5.1 Analyzer Interface

```rust
pub trait Analyzer: Send + Sync {
    fn meta(&self) -> AnalyzerMeta;
    fn analyze(&self, model: &FinancialModel, args: serde_json::Value)
        -> Result<serde_json::Value, FinstackError>;
    fn param_schema(&self) -> serde_json::Value; // schemars JSON Schema
}
```

### 5.2 Registration

* Link‑time via `linkme`/distributed slices (feature‑gated).
* Always support **manual registration**: `register(name: &'static str, Box<dyn Analyzer>)`.
* Built‑ins: `validation_report`, `node_explainer`, `sensitivity`, `grid`, `waterfall`, `waterfall_grid`, `recovery`, `implied_ratings`.

### 5.3 Expression Engine Scope and Performance

The centralized expression engine powers statements, valuations, and analysis. For efficient time‑series operations, its function set is narrowly defined and lowered to Polars primitives where possible.

Scope (required functions):
- lag(x, n), lead(x, n): maps to `shift(±n)`
- diff(x, n=1): maps to `diff(n)`
- pct_change(x, n=1): maps to `pct_change(n)`
- cumsum/cumprod/cummin/cummax(x): maps to cumulative kernels
- rolling_sum/mean/min/max/std/var/median(x, window): row‑window `rolling_*`
- rolling_time_mean(x, window:"30d"|"12m", time_col): dynamic group windows by time
- ewm_mean(x, alpha|span|halflife, adjust=true): exponentially weighted average

Execution policy:
- Vectorizable sub‑graphs are compiled to Polars `Expr` and executed via `LazyFrame` for kernel‑level performance and operator fusion.
- Fallback scalar evaluator exists for unsupported nodes/functions; boundaries are minimized and evaluated block‑wise.
- Period ordering is explicit; time‑window functions require a bound `time_col`.

Determinism & testing:
- Decimal mode enforces stable ordering and no fast‑math; Polars pushdown uses only deterministic kernels.
- Each function has a scalar reference; CI checks parity (exact for Decimal, tolerance for FastF64).
- Compiled DAGs and lowered plans are cached by content hash.

---

## 6) Scenarios Crate

Uses core's expression engine. Wire/runtime separation:
- Scenario wire types: `ScenarioSpec`/`OperationSpec`/`ModifierSpec` with string expressions (`condition_text`, `Shift(String)`, `ValueSpec::Expression(String)`).
- Runtime types: `Scenario`/`Operation`/`Modifier` with `CompiledExpr` after build.

### 6.1 DSL Grammar (EBNF)

```
path      := root "." segments
root      := "statements" | "valuations" | "market" | "portfolio" | "entities"
segments  := seg | seg "." segments
seg       := ident | quoted | glob
quoted    := '"' { any but '"' | '\"' } '"'
glob      := { [a-zA-Z0-9_./\-] | '*' | '?' }+
modifier  := ":= " expr | ":+%" number | ":+bp" number | ":shift" expr | ":*" number

example   := statements."Revenue.Core":+%5
          | valuations.instruments."Loan A".spread:+bp50
          | market.curves.USD_*:+bp10
          | market.fx.USD/EUR:+%2
          | portfolio.positions."TLB-1".quantity:=1200000
          | valuations.instruments?{rating:"CCC"}.spread:+bp50
```

### 6.2 Composition & Execution

* `Scenario::include(other, priority)`; total order by `(priority ASC, declaration_index ASC)`.
* Conflicts at equal priority resolve per `ConflictStrategy` (First/Last/Merge/Error), default Last. Effective strategy is visible in preview and stamped into results meta.
* **Strict/lenient** modes (missing path ⇒ error vs warn+skip).
* **Preview plan**: resolves to the final ordered list of concrete operations, shows glob expansion lists (with truncation flags) and effective composition rules.

### 6.3 Engine Phases & Cache Invalidation

1. Market shocks → rebuild market views; invalidate affected curve/vol/FX caches.
2. Instrument parameter shocks → rebuild schedules; invalidate instrument schedule caches.
3. Statement shocks/overrides → clear dependent node caches.
4. **Portfolio edits** (positions/book).
5. Evaluate statements; price instruments; run analyzers.

---

## 7) Portfolio Crate

### 7.1 Concepts & Types

```rust
pub type EntityId   = String;
pub type PositionId = String;
pub type BookId     = String;

pub struct EntityRefs {
    pub model: Option<std::sync::Arc<FinancialModel>>,
    pub capital: Option<std::sync::Arc<CapitalStructure>>,
    pub tags: TagSet,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum PositionUnit {
    Units,
    Notional(Option<Currency>),
    FaceValue,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Position {
    pub position_id: PositionId,
    pub entity_id: EntityId,
    pub instrument_id: String,
    pub quantity: Decimal, // signed
    pub unit: PositionUnit,
    pub open: time::Date,
    pub close: Option<time::Date>,
    pub tags: TagSet,
}

pub enum BookNode {
    Book { book_id: BookId, name: Option<String>, children: Vec<BookNode>, tags: TagSet },
    LeafPosition(PositionId),
}

// Map canonical node id -> per-entity node id
pub struct NodeAliasMap {
    pub canonical_to_entity: indexmap::IndexMap<String, indexmap::IndexMap<EntityId, String>>,
}
```

### 7.2 Portfolio Model & Period Alignment

```rust
#[derive(Serialize, Deserialize)]
pub struct Portfolio {
    pub id: String,
    pub base_ccy: Currency,
    pub as_of: time::Date,
    pub entities: indexmap::IndexMap<EntityId, EntityRefs>,
    pub positions: indexmap::IndexMap<PositionId, Position>,
    pub book: BookNode,
    pub node_alias: NodeAliasMap,
    pub periods: Vec<Period>,  // portfolio-wide plan
    pub meta: indexmap::IndexMap<String, serde_json::Value>,
}
```

* **Alignment policy:** portfolio plan chosen explicitly; entity series re‑sampled (sum/avg/last) per metric family (documented table).
* **Currency rules:** valuation aggregation is currency‑preserving, collapse to `base_ccy` requires FX from `MarketData.fx`.

### 7.3 Portfolio Results

```rust
pub struct PortfolioValuation {
    pub position_values_ccy: indexmap::IndexMap<PositionId, indexmap::IndexMap<Currency, Decimal>>,
    pub position_values_base: indexmap::IndexMap<PositionId, Decimal>,
    pub book_totals_base: indexmap::IndexMap<BookId, Decimal>,
    pub portfolio_total_base: Decimal,
}

pub struct PortfolioRiskReport {
    pub by_bucket: indexmap::IndexMap<String, Decimal>, // e.g., curve_key -> DV01
}

pub struct PortfolioStatements {
    pub per_node_per_period_base:
        indexmap::IndexMap<String, indexmap::IndexMap<PeriodId, Decimal>>,
}

 

pub struct PortfolioResults {
    pub valuation: PortfolioValuation,
    pub risk: PortfolioRiskReport,
    pub statements: PortfolioStatements,
    pub meta: ResultsMeta, // model_currency=Some(base_ccy)
}
```

### 7.4 Portfolio Scenario Paths

* `portfolio.base_ccy:=USD`
* `portfolio.positions."<pos_id>".quantity:+%10`
* `portfolio.positions."<pos_id>".close:=2026-03-31`
* `entities."<entity_id>".statements."<node>".:= ...` (delegated)

### 7.5 Portfolio Builder & Runner

```rust
pub struct PortfolioBuilder<S> { /* ... */ }
pub struct NeedPlan;
pub struct Ready;

impl PortfolioBuilder<NeedPlan> {
    pub fn plan(self, base_ccy: Currency, as_of: time::Date, periods: Vec<Period>)
        -> Result<PortfolioBuilder<Ready>, FinstackError>;
}
impl PortfolioBuilder<Ready> {
    pub fn entity(self, id:&str, refs:EntityRefs) -> Self;
    pub fn position(self, pos:Position) -> Self;
    pub fn book(self, book:BookNode) -> Self;
    pub fn node_alias(self, map:NodeAliasMap) -> Self;
    pub fn build(self) -> Result<Portfolio, BuildError>;
}

pub struct PortfolioRunner { /* per-run caches */ }

impl PortfolioRunner {
    pub fn run(
        &self,
        portfolio: &Portfolio,
        mkt: &MarketData,
        scenario: Option<&Scenario>,
    ) -> Result<PortfolioResults, FinstackError>;
}
```

### 7.6 Aggregation & Parallelism

* Parallel across positions/books with a **stable reduction order** to keep Decimal determinism.
* Aggregation supports **group‑by** on book, entity, currency, and arbitrary `TagSet` predicates.

---

## 8) Data Crate (Polars/Arrow Interchange)

Depends on core for types; provides interchange between Polars DataFrames and external formats. Renamed to "IO" to avoid confusion with domain data.

### 8.1 Feature‑Gated Backends

* `io/csv`, `io/parquet` via Polars
* `io/arrow` for IPC interchange (optional)

### 8.2 Schemas (Neutral Shapes)

* **Positions:** `(position_id, entity_id, instrument_id, quantity, unit, open_date, close_date?, tags:json)`
* **Transactions:** `(date, position_id, type, amount, currency, quantity?, fee?)`
* **Classifications:** `(id, key, value)` // id can be instrument\_id or entity\_id
* **Statements (long):** `(node, period, value, currency?)`
* **Instruments:** JSON‑tabular
* **Curves:** `(pillar, df|zero|forward)`

**Guarantees**

* No domain types; mapping happens in consumer crates.
* Round‑trip golden tests CSV↔JSON↔internal.

---

## 9) Bindings (Python & WASM)

### 9.1 Python (`finstack-py`)

* **pyo3 + maturin** wheels (Linux/macOS/Windows; Python 3.10–3.12).
* Pydantic v2 models mirror serde shapes; `model_dump()` / `model_validate_json()` round‑trip.
* Heavy compute releases GIL; errors map to `FinstackError` exceptions.
* Fluent façades for `ModelBuilder` and `PortfolioBuilder`.

### 9.2 WASM (`finstack-wasm`)

* `wasm-bindgen` + `serde_wasm_bindgen`; JSON IO matches serde.
* Feature flags for tree‑shaking (e.g., `--features statements,valuations`).
* Dev helpers: `console_error_panic_hook` (optional); memory: `wee_alloc` (optional).

---

## 10) Observability

* `tracing` spans wrap parse/build/run/price/analyze; JSON logs optional via env.
* Correlation IDs: `RunId`, `ScenarioId`, `BookId` in spans and results meta.
* Log levels configurable; structured fields (entity\_id, instrument\_id, node\_id, book\_id).

---

## 11) Performance

### 11.1 Targets

* Evaluate 10k nodes × 60 periods < 250ms (single thread, Decimal) on modern CPU.
* Price 5k vanilla instruments with cached curves < 150ms (single thread).

### 11.2 Techniques

* Vectorized per‑period evaluation; layer‑parallel with Rayon (opt‑in).
* Pre‑sized `IndexMap`/`HashMap`; reuse allocations in pricing kernels.
* Caches for curve interpolation grids, accrual grids, schedules.

### 11.3 Determinism

* Decimal mode: **byte‑identical** serial vs parallel.
* FastF64 mode: tolerance‑bounded; `numeric_mode` stamped in outputs.

---

## 12) Error Handling & API Stability

* `thiserror` enums per crate; unified `FinstackError`; `#[non_exhaustive]`.
* Public APIs covered by **semver**; `cargo-public-api` CI gate.
* **Schema versioning**: top‑level envelopes include `schema_version`. Inbound strict via `deny_unknown_fields`. Migration shims documented per bump.

---

## 13) Security & Safety

* `#![deny(unsafe_code)]` in all crates.
* No dynamic code execution from user expressions (expression language is closed set).
* Input validation for DSL and data crate; strict serde deserialization; no implicit filesystem/network operations.

---

## 14) Builders & Developer Experience

* **ModelBuilder** (type‑state): `periods(...) -> compute(...) -> register_metrics(...) -> build() -> Result<FinancialModel, BuildError>`.
* **PortfolioBuilder**: `plan(...) -> entity(...) -> position(...) -> book(...) -> build() -> Result<Portfolio, BuildError>`.
* Error messages include context (node\_id/period/path).

---

## 15) Testing & CI

### 15.1 Test Types

* **Unit**: period parsing (incl. `"2025Q1..Q2"`), calendars, BDC/EOM/stubs, day count, expressions, DAG cycles.
* **Parity/Quant**: IRS PV/DV01 vs references, option greeks, CDS par spreads.
* **Property**: `sum_by_ccy(aggregate) == sum(schedule)`; `rebuild(serialize(periods)) == periods`.
* **Golden**: metrics registry outputs; waterfalls; serialized snapshots (with schema\_version).
* **Compile‑time**: `trybuild` for macros (`bp!`, `pct!`).
* **Doctests**: public examples.
* **Miri**: UB checks (subset).
* **Bench**: Criterion micro‑benchmarks (curves, accruals, DAG).

### 15.2 CI Matrix

* OS: Linux/macOS/Windows
* Toolchains: stable; MSRV pinned and checked (`cargo-msrv`)
* Features: `--no-default-features`, each feature solo, and combinatorial (incl. `portfolio`)
* Python: build/test wheels via `maturin` (3.10–3.12)
* WASM: build/test `wasm32-unknown-unknown`

---

## 16) Acceptance Criteria (Complete)

* **Core**

  * Period ranges + actuals parsed; stable ordering; idempotent rebuild.
  * Calendars/BDC/EOM/day‑count correct; cached with keying.
  * DAG evaluation deterministic; Decimal mode serial == parallel.

* **Statements**

  * `NodeType` semantics: **Value > Forecast > Formula** per period.
  * `where_clause` acts as boolean mask only.
  * Metrics registry namespaced (`fin.`) and collision‑safe.

* **Valuations**

  * Instruments implement `CashflowProvider` + `Priceable` (RiskMeasurable where meaningful).
  * Currency‑preserving period aggregation; explicit FX collapse to model/base ccy.
  * Parity tests: IRS/Options/CDS within tolerances; greeks consistent.

* **Analysis**

  * Plugin registry supports link‑time and manual registration.
  * `param_schema` provided via `schemars` and validated at runtime.

* **Scenarios**

  * DSL paths support quoting and deterministic globs (`*`, `?`) with lexical expansion order and limits surfaced in preview.
  * Attribute selectors (`?{k:v,...}`) filter instruments/nodes by `tags`/`meta`; expansion is deterministic and visible in preview with truncation flags.
  * Modifiers include `:=`, `:+%`, `:+bp`, `:shift`, `:*`.
  * Path normalization + linter enforce canonical keys (quotes, indices, currency pairs).
  * Composition deterministic; conflict strategy (First/Last/Merge/Error) visible in preview and stamped into results meta; strict/lenient modes.
  * **Preview plan** includes expansion lists and composition rules; engine phases execute in documented order and invalidate caches precisely.

* **Portfolio**

  * Positions reference valid `(entity_id, instrument_id)`; builder validates.
  * Aggregation is currency‑preserving; collapse to `base_ccy` uses FX (as\_of policy documented).
  * Statements align via `node_alias` across entities and portfolio plan.
  * Book/position rollups deterministic; tag‑based group‑by supported.
  

* **Bindings**

  * Serde field names stable; `schema_version` present.
  * Python pydantic v2 round‑trip with Rust; heavy ops GIL‑free.
  * WASM JSON IO matches serde; features tree‑shake modules.

* **Observability**

  * Tracing spans for parse/build/run/price/analyze with IDs; JSON logging optional.

* **Packaging**

  * `cargo-public-api` enforces semver; MSRV pinned.
  * Feature builds succeed (solo and combos).

---

## 17) Detailed Policies

### 17.1 Currency Conversion Policy

* **Valuation collapse**: by default, convert at cashflow date using `MarketData.fx` spot or provided curve; document alternates (period‑end rate) and expose strategy as a parameter.
* **Statements**: if `Amount`, convert at **period end** FX unless node metadata overrides (e.g., average rate).

### 17.2 Resampling Rules for Statements

| Family      | Resample from M→Q/Y   | Resample from Q→Y |
| ----------- | --------------------- | ----------------- |
| Flow (P\&L) | **sum**               | **sum**           |
| Stock (B/S) | **last**              | **last**          |
| Ratios (%)  | time‑weighted **avg** | **avg**           |

Override per node via `meta` (e.g., `{ "resample": "sum" }`).

### 17.3 Missingness & Formula Semantics

* Arithmetic with `None` ⇒ `None` unless an explicit `coalesce(x,0)` function is used.
* Division by zero ⇒ `None` with typed error logged in tracing (optional “strict math” feature to error).

### 17.4 Parallelism Controls

* Top‑level `RunConfig { parallel: bool, threads: Option<usize> }` for both statements and portfolio runners.
* Host may provide Rayon thread pool; otherwise use global.

---

### 17.5 Global Configuration & Rounding/Scale Policy

**Purpose**

Stabilize ingest/output numeric representations across hosts and languages by centralizing rounding and scale rules for `rust_decimal::Decimal`, including per‑currency scale.

**Core types (in `finstack-core::config`)**

```rust
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum RoundingMode { Bankers, AwayFromZero, TowardZero, Floor, Ceil }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurrencyScalePolicy {
    pub default_scale: u32,                 // e.g., 2 for USD, 0 for JPY
    pub overrides: indexmap::IndexMap<Currency, u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoundingPolicy {
    pub mode: RoundingMode,
    pub ingest_scale: CurrencyScalePolicy,  // applied to inbound values (IO/constructors)
    pub output_scale: CurrencyScalePolicy,  // applied to serialization/exports
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoundingContext {
    pub mode: RoundingMode,
    pub ingest_scale_by_ccy: indexmap::IndexMap<Currency, u32>,
    pub output_scale_by_ccy: indexmap::IndexMap<Currency, u32>,
    pub version: u32, // bump on policy schema changes
}

pub struct FinstackConfig { pub rounding: RoundingPolicy, /* future: locale, calendars, etc. */ }

pub fn config() -> std::sync::Arc<FinstackConfig>;          // global accessor
pub fn with_temp_config<T>(cfg: FinstackConfig, f: impl FnOnce() -> T) -> T; // scoped override
```

**Behavior**

- Ingest paths (deserializers, builder APIs that accept external numeric input) round/scale using `rounding.ingest_scale` per currency.
- Output paths (serde serializers, CSV/JSON writers, display helpers) round/scale using `rounding.output_scale` per currency.
- When currency is unknown (unitless scalars), use `default_scale` from the policy; callers may override explicitly.
- All top‑level result envelopes include `ResultsMeta.rounding` to record the active `RoundingContext` (mode and per‑ccy scales) that produced the values.
- Policy is immutable during a run; tests may use `with_temp_config` to scope overrides.

**Defaults**

- `RoundingMode::Bankers` (half‑to‑even)
- `default_scale = 2`; overrides loaded for ISO‑4217 (e.g., `JPY=0`, `KWD=3`).

**Acceptance**

- Round‑trip property tests: `serialize(rounded(x))` produces stable strings across hosts for the same `RoundingContext`.
- Golden files include `ResultsMeta.rounding` and match on replay.

## 18) Example Flows

### 18.1 Build a Model (Rust)

```rust
use finstack::core::*;
use finstack::statements::*;

let plan = core::build_periods("2025Q1..2026Q4", Some("2025Q1..Q2"))?;
let model = ModelBuilder::new("Acme")
    .periods("2025Q1..2026Q4", Some("2025Q1..Q2"))?
    .compute("gross_margin", "gross_profit / revenue", None)?
    .register_metrics("fin.basic")?
    .build()?; // Result<FinancialModel, BuildError>
```

### 18.2 Price Instruments & Aggregate (Rust)

```rust
use finstack::core::prelude::*;
use finstack::valuations::*;
let mkt = MarketData { /* as_of, curves, fx: FxMatrix, vol */ };
let pv = instrument.price(&mkt, mkt.as_of)?;
let per = instrument.aggregate_to_model_ccy(&plan.all, None, &mkt.fx as &dyn FxProvider, Currency::USD)?;
```

### 18.3 Portfolio Run with Scenario (Rust)

```rust
use finstack::portfolio::*;
use finstack::scenarios::*;

let portfolio = PortfolioBuilder::new("Fund A")
    .plan(Currency::USD, mkt.as_of, plan.all.clone())?
    .entity("OpCo", EntityRefs { model: Some(Arc::new(model)), capital: Some(Arc::new(stack)), tags: TagSet::default() })
    .position(Position { /* ... */ })
    .book(BookNode::Book { /* ... */ })
    .build()?;

let sc = Scenario::parse(r#"
    market.fx.USD/EUR:+%2
    portfolio.positions."TLB-1".quantity:+%10
"#)?;
let runner = PortfolioRunner::new();
let out = runner.run(&portfolio, &mkt, Some(&sc))?;
```

---

## 19) Future Work (Feature‑Gated or Later)

* Covariance‑aware risk aggregation (VaR/ES).
* Stochastic scenario engines (Monte Carlo) with seeded RNG and deterministic sampling in Decimal mode.
* Extended instrument set (exotics) and analytics (credit equity hybrids).
* PE/RE underwriting within existing crates (no new crate):
  - In valuations: property cashflows (rent roll, opex/CAM, taxes, capex/reserves), real-estate debt (IO, construction/interest reserve, DSCR sweeps), deterministic equity waterfalls with clawback.
  - In scenarios: underwriting selectors/paths for lease cohorts, tenant types, markets; shocks for rents, vacancy, exit cap, TI/LC, DSCR thresholds.

---

### Appendix A — Period Parsing Examples

* `"2025Q1..2026Q4"` ⇒ 2025Q1–2026Q4.
* `actuals="2025Q1..Q2"` ⇒ {2025Q1, 2025Q2} actual; forecasts extend only into non‑actuals.
* `"2025-01..2025-06"` (monthly); `"2024..2026"` (yearly).
* Errors: frequency mixed; end < start; unknown tokens.

### Appendix B — Scenario DSL Examples

* `statements."Revenue.Core":+%5`
* `valuations.instruments."TermLoan A".spread:+bp50`
* `market.fx.USD/EUR:+%2`
* `portfolio.base_ccy:=USD`
* `portfolio.positions."CDS-ACME".close:=2026-06-30`

### Appendix C — Instrument Conformance Sketch

```rust
impl CashflowProvider for InterestRateSwap { /* tag coupons; build schedules */ }
impl Priceable for InterestRateSwap { /* discount legs; par rate; PV */ }
impl RiskMeasurable for InterestRateSwap { /* DV01 analytic or curve bumps */ }

impl CashflowProvider for EquityOption { /* premium/settlement */ }
impl Priceable for EquityOption { /* Black; delta/gamma/vega/theta/rho */ }
```

---

**This document is the authoritative specification for the Finstack library set.**
It encodes the invariants (determinism, currency‑safety), architectural boundaries, public contracts, and acceptance criteria to guide implementation, testing, and long‑term maintenance.
