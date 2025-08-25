# `/core` Crate — Technical Design (Revised)

**Status:** Draft (implementation-ready)
**Last updated:** 2025‑01‑25
**MSRV:** 1.75 (target)
**License:** Apache‑2.0 (project standard)

## 1) Purpose & Scope

`/core` is the foundational crate for finstack. It contains:

* **Domain‑agnostic primitives**: strong **types** (Amount, Currency, Rate, Id<…>, Date/Timestamp).
* **Expression engine**: AST, DAG compilation, and evaluation framework used across crates.
* **FX infrastructure**: FxProvider trait, FxMatrix, and caching for currency conversions.
* Numerically careful **math kernels**: root finding, summation, robust statistics building blocks.
* **Time utilities**: unified period system, day‑count conventions, and business‑day calendars/schedules.
* **Validation framework**: trait-based validation system for composable validators.
* **Errors**: a single error type for predictable failure modes.
* **Polars as the canonical time‑series runtime**: we depend on and re‑export Polars to unify time‑series representations across the workspace.

> **Out of scope and moved out**
>
> * Cash flows & valuations (XIRR/TWR/MWR) → **`valuations` crate**
> * Portfolio accounting & scenarios → **`portfolio`** / **`scenario`** crates
> * Risk analytics → (later) in **`risk`** or under **`portfolio`/`scenario`**
> * Arrow & file I/O → **`io` crate** (optional, feature‑gated in the workspace, not in `/core`)

---

## 2) Workspace Architecture (updated)

```
finstack/
├─ core/                  # THIS crate: types, math, time, errors; + Polars as dependency
│  └─ src/...
├─ valuations/            # CashFlow, NPV, XIRR/TWR/MWR; depends on core (+Polars via core)
├─ portfolio/             # Positions, events, NAV; optional, separate
├─ scenario/              # Scenario engines, shock frameworks; optional
├─ io/                    # Arrow, Parquet/CSV, adapters (optional)
├─ bindings-python/       # PyO3 wrappers for valuations/portfolio/etc. (not for core)
├─ bindings-wasm/         # wasm-bindgen wrappers for valuations/portfolio/etc.
└─ udf-xirr-wasm/         # libSQL UDF for XIRR (uses valuations)
```

**Why Polars in `/core`?**

* Establishes a **single** DataFrame/Series representation across crates.
* Avoids re‑inventing a `Series<T>`; consumers share one ABI and set of semantics.
* Keeps Arrow concerns out of `/core` (they live in `io`), while letting other crates use Polars immediately.

---

## 3) Cargo & Feature Plan

**`core/Cargo.toml` (sketch):**

```toml
[package]
name = "finstack-core"
version = "0.2.0"
edition = "2021"
license = "Apache-2.0"

[lib]
name = "finstack_core"
crate-type = ["rlib"]

[features]
default = ["std", "serde"]
std = []
alloc = []
serde = ["dep:serde"]
simd = []             # gate std::simd usage in math kernels
fast_f64 = []         # enable optional fast f64 compute paths (types remain Decimal)
rayon = ["dep:rayon"]
deterministic = []    # strict floating-point modes + stable summation

[dependencies]
thiserror = "1"
tracing = "0.1"
time = "0.3"
hashbrown = "0.14"
smallvec = "1"
serde = { version = "1", features = ["derive"], optional = true }
rust_decimal = { version = "1", optional = true, features = ["serde"] }
rayon = { version = "1", optional = true }
once_cell = "1"

# Canonical time-series library for the whole workspace
polars = { version = "X.Y", default-features = false, features = [
  "lazy",         # expressions & query plans
  "temporal",     # date/time types & ops
  "dtype-datetime",
  "dtype-duration",
  "fmt"
] }

[dev-dependencies]
proptest = "1"
criterion = "0.5"
```

> **Note:** No Arrow crates in `/core`. Arrow/Parquet live in the optional `io` crate.

---

## 4) Modules & Responsibilities (trimmed)

### 4.1 `types`

Strong newtypes and units:

* `Amount`: currency‑aware numeric carrier using `rust_decimal::Decimal` for accounting‑grade precision.
* `Currency`: ISO‑4217 code representation or internal mapping.
* `Rate`, `Bps`, `Percentage` helpers and conversions.
* `Date`, `Timestamp`: wrappers over `time` crate types.
* `Id<T>`: phantom‑typed identifiers to prevent mixups across domains.

**Design notes**

* Keep arithmetic on `Amount` explicit (no implicit FX); FX conversion uses explicit FxProvider.
* Provide clear rounding/scale helpers; rounding policy is opt‑in (caller chooses).

### 4.2 `money`

**FX Infrastructure:**

* `FxProvider` trait for currency conversion rates
* `FxMatrix` for efficient multi-currency operations
* LRU caching with configurable TTL
* Closure checking (A/B × B/C ≈ A/C within tolerance)

Shared FX policy types (for reuse across crates):

```rust
/// Standard FX conversion strategies used across the workspace
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum FxConversionPolicy {
    CashflowDate,
    PeriodEnd,
    PeriodAverage,
    Custom,
}

/// Metadata describing an applied FX policy
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FxPolicyMeta {
    pub strategy: FxConversionPolicy,
    pub target_ccy: Option<Currency>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}
```

### 4.3 `expr`

**Expression Engine:**

* AST representation for formulas and calculations
* DAG compilation with cycle detection
* Generic evaluation framework with type-safe contexts
* Constant folding and optimization passes
* Used by statements, valuations, and scenarios crates

#### 4.3.1 Time‑Series Function Library (Scope)

To support performant financial time‑series operations, the expression language MUST expose a small, well‑defined set of functions that deterministically map to Polars primitives:

- lag(expr, n)
  - Purpose: prior period/value lookup
  - Polars mapping: `col(expr).shift(n)`
- lead(expr, n)
  - Purpose: forward period/value lookup
  - Polars mapping: `col(expr).shift(-n)`
- diff(expr, n=1)
  - Purpose: first/lagged difference
  - Polars mapping: `col(expr).diff(n)`
- pct_change(expr, n=1)
  - Purpose: percentage change over n steps
  - Polars mapping: `col(expr).pct_change(n)`
- cumsum(expr), cumprod(expr), cummin(expr), cummax(expr)
  - Polars mapping: `cumsum`, `cumprod`, `cummin`, `cummax`
- rolling_sum(expr, window), rolling_mean(expr, window), rolling_min/max/std/var/median(expr, window)
  - Purpose: windowed aggregations over fixed‑size windows
  - Polars mapping: `rolling_*` with `window_size=window` (row‑count based)
- rolling_time_mean(expr, window: Duration, time_col)
  - Purpose: time‑based windows (e.g., 30D, 12M)
  - Polars mapping: `group_by_dynamic(time_col, every=window).agg([col(expr).mean()])`
- ewm_mean(expr, alpha | span | halflife, adjust=true)
  - Purpose: exponentially weighted moving average
  - Polars mapping: `ewm_mean(alpha=?, adjust=?)`

Function shapes are intentionally narrow to preserve determinism and enable static planning. All functions operate over the evaluation order of periods unless an explicit `time_col` is provided for time‑based windows.

Notes:
- Window may be specified as a positive integer (row count) or duration string for time windows (e.g., "30d", "12m"). Duration windows require a `time_col` bound at compile time.
- NaN/None handling follows Polars semantics unless overridden by explicit `coalesce`/`fill_null` helpers in the expression.

#### 4.3.2 Pushdown & Execution Strategy

- Compilation produces a dual representation: a scalar evaluator and, when possible, a Polars expression plan.
- During vectorized evaluation (statements/valuations), compatible sub‑graphs are lowered to Polars `Expr` and executed via `LazyFrame` for kernel‑level performance and operator fusion.
- Mixed graphs (unsupported nodes or custom Rust functions) are evaluated with a fallback scalar path, but boundaries are minimized via block‑wise materialization.
- Stable ordering: period order is explicit; any group/window operation MUST specify its ordering column (period index or `time_col`).

Lowering rules (illustrative):

```text
lag(x, n)                 -> col("x").shift(n)
rolling_mean(x, 3)        -> col("x").rolling_mean(window_size=3)
rolling_time_mean(x, "30d", dt)
                          -> col("x").mean().over(group_by_dynamic(col("dt"), every="30d"))
pct_change(x, 1)          -> col("x").pct_change(1)
ewm_mean(x, alpha=0.2)    -> col("x").ewm_mean(alpha=0.2, adjust=true)
```

#### 4.3.3 Performance & Determinism Guarantees

- Operator fusion: letting Polars optimize the plan typically yields O(n) passes with SIMD where available.
- Memory: rolling/ewm kernels are streaming where possible; large windows avoid quadratic behavior.
- Determinism: Decimal mode requires stable order and no fast‑math; Polars pushdown is constrained to kernels that preserve determinism under the same input order.
- Parity tests: Each function has a scalar reference implementation; CI asserts Polars vs scalar parity within exact equality for Decimal or tolerance for FastF64.
- Caching: compiled expression DAGs and lowered Polars plans are cached by content hash.

#### 4.3.4 Extensibility

- The engine exposes a registry for pure, deterministic functions. Functions annotated as “vectorizable” provide a lowering hook to Polars `Expr`; otherwise they execute via the scalar path.
- Cross‑crate usage (statements, valuations, scenarios) MUST call only registered, serde‑stable functions; closures are prohibited.

### 4.4 `math`

* **Root finders:** Brent, safeguarded Newton with bracket checks.
* **Summation:** Pairwise/Kahan; `deterministic` enforces stable paths.
* **Stats kernels:** mean/variance (Welford), covariance/correlation (building blocks only).
* **SIMD (optional):** identical results to scalar in deterministic mode.

> These are **building blocks** used by other crates (e.g., `valuations`, `portfolio`), not high‑level financial algorithms themselves.

### 4.5 `time`

* **Unified Period System:**
  * `Period` type with Duration/Instant semantics for different statement types
  * `PeriodId`, `PeriodKey` for consistent identification
  * Period parsing with range support ("2025Q1..Q2")
  * `PeriodPlan` builder with actual/forecast tracking
* **Day‑count conventions:** `Act365F`, `Act360`, `ActAct(ISDA/ICMA)`, `30/360(US/EU)`.
* **Business calendars:** minimal trait + weekend rules; holiday sets supplied by consumers.
* **Schedules:** coupon schedules & stubs (kept generic, usable by `valuations`).

### 4.6 `validation`

**Validation Framework:**

* `Validator` trait for composable validation
* `ValidationResult<T>` with warnings and pass/fail status
* Building blocks for domain-specific validators in other crates

### 4.7 `prelude`

* Re‑exports of the most commonly used items to smooth ergonomics across the workspace:

  * `types::*`, `time::{DayCount, BusinessCalendar, Period, PeriodPlan, …}`, `math::*`
  * `money::{FxProvider, FxMatrix, …}`
  * `expr::{Expr, ExpressionContext, …}`
  * `validation::{Validator, ValidationResult, …}`
  * **Polars** essentials for a unified TS API surface:

    * `polars::prelude::{DataFrame, Series, Expr as PolarsExpr, LazyFrame, col, lit, when, …}`

DataFrame interoperability contracts:

```rust
/// Helpers to construct DataFrames from nested IndexMaps without extra copies
pub mod df {
    use polars::prelude::*;

    /// Build a long DataFrame from node->period->value
    pub fn long_from_nested<K1: AsRef<str>, K2: AsRef<str>, V: Into<f64>>(
        nested: &indexmap::IndexMap<K1, indexmap::IndexMap<K2, V>>,
        col1: &str,
        col2: &str,
        colv: &str,
    ) -> DataFrame { /* implemented in consumer crates; spec here */ }

    /// Build a wide DataFrame (rows by second key, columns by first key)
    pub fn wide_from_nested<K1: AsRef<str>, K2: AsRef<str>, V: Into<f64>>(
        nested: &indexmap::IndexMap<K1, indexmap::IndexMap<K2, V>>,
        row_key: &str,
    ) -> DataFrame { /* implemented in consumer crates; spec here */ }
}
```

### 4.8 `errors`

* One crate‑wide error enum:

  ```rust
  #[derive(thiserror::Error, Debug)]
  pub enum CoreError {
    #[error("invalid input: {0}")] InvalidInput(String),
    #[error("shape mismatch")] Shape,
    #[error("unsupported operation: {0}")] Unsupported(&'static str),
    #[error("numeric failure: {0}")] Numeric(&'static str),
    // …
  }
  ```

> **Removed sections per your request**
>
> * **(Old 4.4) `cashflow`** → moved to `valuations` crate
> * **(Old 4.5) `ts`** → replaced by **Polars** dependency & re‑exports
> * **(Old 4.6) `risk`** → deferred to portfolio/scenario layer
> * **(Old 4.7) `portfolio`** → separate larger crate
> * **(Old 4.8) `io`** → separate optional crate

### 4.9 `config`

Global configuration primitives and accessors used across crates.

```rust
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum RoundingMode { Bankers, AwayFromZero, TowardZero, Floor, Ceil }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurrencyScalePolicy {
    pub default_scale: u32,
    pub overrides: indexmap::IndexMap<Currency, u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoundingPolicy {
    pub mode: RoundingMode,
    pub ingest_scale: CurrencyScalePolicy,
    pub output_scale: CurrencyScalePolicy,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoundingContext {
    pub mode: RoundingMode,
    pub ingest_scale_by_ccy: indexmap::IndexMap<Currency, u32>,
    pub output_scale_by_ccy: indexmap::IndexMap<Currency, u32>,
    pub version: u32,
}

#[derive(Clone, Debug)]
pub struct FinstackConfig { pub rounding: RoundingPolicy }

/// Global config accessor; thread-safe and cheap to clone via Arc
pub fn config() -> std::sync::Arc<FinstackConfig>;

/// Execute a closure with a temporary config override (for tests/tools)
pub fn with_temp_config<T>(cfg: FinstackConfig, f: impl FnOnce() -> T) -> T;
```

Usage requirements:
- Ingest paths should apply `ingest_scale` per currency when normalizing `Decimal`s.
- Output/serialization should apply `output_scale` per currency.
- Results envelopes must stamp `RoundingContext` in `ResultsMeta` (see Overall §2.7 and §11.2).

### 4.10 `index_series`

Purpose: shared time-series indices (e.g., CPI/RPI) with interpolation and lag rules, reusable across crates (valuations/statements).

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexId(pub String);

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum IndexInterpolation {
    Step,          // last observation carried forward (monthly CPI typical)
    Linear,        // linear between observed points
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum IndexLag {
    Months(u8),    // e.g., 3-month lag
    Days(u16),
    None,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum SeasonalityPolicy {
    None,
    Multiplicative,  // optional monthly factors
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexSeries {
    pub id: IndexId,
    /// Observation dates (typically month-ends) and values
    pub observations: Vec<(time::Date, rust_decimal::Decimal)>,
    pub interpolation: IndexInterpolation,
    pub lag: IndexLag,
    pub seasonality: Option<[rust_decimal::Decimal; 12]>,
}

impl IndexSeries {
    /// Value applicable on a given date after applying lag, interpolation, and seasonality
    pub fn value_on(&self, date: time::Date) -> Result<rust_decimal::Decimal, CoreError> { /* spec */ }

    /// Convenience: index ratio I(settle)/I(base)
    pub fn ratio(&self, base: time::Date, settle: time::Date) -> Result<rust_decimal::Decimal, CoreError> { /* spec */ }
}
```

Contracts:
- Deterministic interpolation and lag application; inputs assumed sorted, validated at construction.
- Seasonality factors indexed by calendar month (1..=12), applied per policy.
- Exposed in `prelude` for reuse; valuations will consume for ILBs (TIPS/ILBs) indexation, statements may reference for KPI deflators.

---

## 5) Public API (selected)

> `/core` exports **primitives** and **helpers**; no valuations or portfolio functions.

```rust
// types
pub struct Amount {
    pub value: rust_decimal::Decimal,
    pub ccy: Currency,
}
pub struct Currency(pub u16);
pub struct Rate(pub rust_decimal::Decimal);
pub struct Bps(pub i32);
pub struct Percentage(pub f64);

// money/fx
pub trait FxProvider: Send + Sync {
    fn rate(&self, from: Currency, to: Currency, on: time::Date) -> Result<Decimal, CoreError>;
}
pub struct FxMatrix { /* multi-currency operations */ }

// expr
pub trait ExpressionContext {
    type Value;
    fn resolve(&self, name: &str) -> Option<Self::Value>;
}
pub struct Expr { /* AST nodes */ }
pub struct CompiledExpr { /* optimized DAG */ }

// time
pub struct Period {
    pub id: PeriodId,
    pub start: time::Date,
    pub end: time::Date,
    pub frequency: Frequency,
    pub period_type: PeriodType,
}
pub enum PeriodType { Duration, Instant }
pub enum DayCount { Act365F, Act360, ActActISDA, ActActICMA, Thirty360US, Thirty360EU }
pub trait BusinessCalendar { fn is_business_day(&self, d: Date) -> bool; }
pub fn build_periods(range: &str, actuals: Option<&str>) -> Result<PeriodPlan, CoreError>;

// validation
pub trait Validator {
    type Input;
    type Output;
    fn validate(&self, input: &Self::Input) -> ValidationResult<Self::Output>;
}

// math
pub fn brent<F: Fn(f64) -> f64>(f: F, lo: f64, hi: f64, tol: f64, max_iter: usize) -> Result<f64, CoreError>;
pub fn kahan_sum(xs: impl IntoIterator<Item = f64>) -> f64;

// prelude (re-exports)
pub use polars::prelude::{DataFrame, Series, LazyFrame, col, lit, when};
pub mod config; // exposes RoundingPolicy/RoundingContext and accessors
// Index series re-export
pub use crate::time::IndexSeries;
```

---

## 6) Inter‑Crate Contracts

* **`statements`** (depends on `core`):

  * Uses: `Period` system for financial periods, `FxProvider` for currency conversion, `expr` for formulas.
  * Implements `ExpressionContext` for statement-specific calculations.
  * Implements domain-specific `Validator`s using core's framework.
  * Exports data as Polars DataFrames using core's re-exports.

* **`valuations`** (depends on `core`):

  * Owns: `CashFlow`, `CashFlows`, `npv`, `xirr`, `twr`, `mwr`.
  * Uses: `time::DayCount` & `year_fraction`, `math::brent`, `types::Amount/Rate`, `money::FxProvider`.
  * Uses `expr` for valuation formulas and scenario calculations.
  * Time‑series inputs/outputs should be `polars::Series`/`DataFrame` (via core's re‑exports).
  * Python/WASM bindings for XIRR live next to/above `valuations` (not in `/core`).

* **`portfolio`** / **`scenario`** (depend on `core` and likely on `valuations`):

  * Portfolio events, positions, NAV, scenario shocks, analytics.
  * Uses `expr` for scenario DSL and portfolio calculations.
  * Time‑series via Polars; Arrow/file I/O via `io` crate when needed.

* **`io`** (optional):

  * Arrow/Parquet/CSV adapters, interop with external systems (DuckDB, etc.).
  * Converts between Polars DataFrames and Arrow for interchange.
  * Depends on Polars as needed; no changes to `/core`.

---

## 7) Determinism & Numerics

* `deterministic` feature enforces:

  * Pairwise/Kahan summation in helpers.
  * No fast‑math; explicit tolerances.
  * Brent as the preferred root solver for stability (used by consumers like `valuations`).

* Canonical numeric policy:

  * Public, accounting‑grade APIs use `rust_decimal::Decimal` via the `Amount` type by default.
  * Optional `fast_f64` feature enables f64 compute kernels where safe; results are stamped with `NumericMode::FastF64` and converted back to Decimal at boundaries.

---

## 8) Concurrency & Performance

* `rayon` (optional) for data‑parallel math building blocks where appropriate.
* SIMD kernels (optional `simd`) with outputs matching scalar in deterministic mode.
* Criterion benchmarks for core primitives (no valuations/portfolio benches here).

**Benchmark targets (core primitives only)**

* `brent` with smooth functions converges within typical microseconds (\~ tens) on modern CPUs.
* Kahan/pairwise summation throughput competitive with naive sum while improving stability.

---

## 9) Testing Strategy

* **Unit tests** per module (`types`, `math`, `time`).
* **Property tests** for math and time (e.g., NPV monotonicity will move to `valuations`; core keeps generic function properties).
* **Golden tests** for day‑count and calendar rules using fixed date fixtures.
* **Cross‑crate tests** live with the consumer crates (`valuations`, `portfolio`) to verify integration on Polars Series/DataFrames.

---

## 10) Observability

* `tracing` at `debug` for algorithm steps in math primitives when helpful.
* No info/warn logs in hot paths by default.

---

## 11) Deliverables & Next Steps

1. **Refactor `/core` layout** to only: `types`, `math`, `time`, `errors`, `prelude`.
2. **Add Polars dependency** and **re‑export** key APIs in `prelude` to unify usage:

   * `pub use polars::prelude::{DataFrame, Series, Expr, LazyFrame, col, lit, when};`
3. **Create `valuations` crate**:

   * Move `CashFlow` types, `npv`, `xirr`, `twr`, `mwr` here.
   * Accept/produce Polars Series/DataFrame where time‑series is relevant.
   * Unit/property tests for cash‑flow edge cases, multiple roots, etc.
   * (Later) Bindings for Python/WASM + libSQL UDF (`udf-xirr-wasm`).
4. **Create optional `io` crate**:

   * Arrow/Parquet/CSV integrations; avoid contaminating `/core`.
5. **Document cross‑crate contracts** in `README`s and a short ADR (Architecture Decision Record) about Polars standardization.

---

## 12) File Skeleton (updated)

```
core/src/
  lib.rs
  prelude.rs
  errors.rs
  types/
    mod.rs
    id.rs
    rates.rs
  money/
    mod.rs
    currency.rs
    fx.rs
    amount.rs
  expr/
    mod.rs
    ast.rs
    dag.rs
    eval.rs
    context.rs
  math/
    mod.rs
    root_finding.rs
    summation.rs
    stats.rs
  time/
    mod.rs
    periods.rs
    daycount.rs
    calendar.rs
    schedule.rs
    index_series.rs
  validation/
    mod.rs
    traits.rs
    result.rs
```

**`lib.rs`**

```rust
pub mod prelude;
pub mod errors;
pub mod types;
pub mod money;
pub mod expr;
pub mod math;
pub mod time;
pub mod validation;

pub use errors::CoreError;
```

**`prelude.rs`**

```rust
//! finstack-core prelude: commonly used items + Polars re-exports

pub use crate::types::*;
pub use crate::money::{FxProvider, FxMatrix, Currency, Amount};
pub use crate::expr::{Expr, ExpressionContext, CompiledExpr};
pub use crate::time::{DayCount, BusinessCalendar, Period, PeriodPlan, PeriodType, year_fraction, add_business_days};
pub use crate::validation::{Validator, ValidationResult};
pub use crate::math::*;

// Re-export Polars with alias to avoid Expr name collision
pub use polars::prelude::{DataFrame, Series, Expr as PolarsExpr, LazyFrame, col, lit, when};
```

---

## 13) Compatibility Notes

* **Bindings** (Python/WASM) should target **`valuations`** and higher‑level crates, not `/core`.
* **Version pinning:** align `polars` version across **all** crates to prevent duplicate versions in the workspace.
* **No Arrow in `/core`**; keep those in `io` to avoid heavy transitive deps where not needed.

---
