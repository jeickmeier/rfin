# Configurable Portfolio Optimization Design

**Date:** 2026-04-11
**Status:** Approved
**Scope:** `finstack/portfolio/src/optimization/`, Python/WASM bindings, notebook examples

## Problem

The portfolio optimization module has hard-coded business logic (e.g., `optimize_max_yield_with_ccc_limit`) and redundant constraint variants (`TagExposureLimit`, `TagExposureMinimum`) that duplicate what the generic `MetricBound` can already express. Position attributes are string-only (`tags: IndexMap<String, String>`), preventing numeric attribute-based objectives and constraints. Filters lack boolean composition (`And`/`Or`), making cross-constraints like "max 5% in CCC Energy names" inexpressible.

## Goals

1. One canonical path for metric-based constraints (`MetricBound`)
2. Unified attribute system supporting both categorical and numeric values
3. Full boolean filter composition with attribute-aware predicates
4. Remove all hard-coded optimization helpers
5. Simplify the LP solver (fewer match arms, same power)

## Non-Goals

- Quadratic/non-linear constraints (SOCP, QP) — out of scope for LP solver
- Cardinality constraints (max N positions) — requires MIP
- Expression tree arithmetic (`Add`, `Mul` on `MetricExpr`) — YAGNI, risks accepting non-linear specs

## Design

### 1. `AttributeValue`

Unified value type replacing string-only tags on positions.

```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AttributeValue {
    /// Categorical / string attribute (e.g., rating = "CCC", sector = "Energy").
    Text(String),
    /// Numeric attribute (e.g., credit_score = 650.0, esg_score = 72.5).
    Number(f64),
}
```

**Placement:** `finstack/portfolio/src/types.rs`, re-exported at crate root.

### 2. `ComparisonOp`

Full comparison operator for attribute-based filtering. Distinct from `Inequality` (which only has `Le`/`Ge`/`Eq` for LP constraint relations).

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}
```

**Placement:** `finstack/portfolio/src/types.rs`.

### 3. `AttributeTest`

Reusable predicate packaging key + operator + value.

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttributeTest {
    /// Attribute key to test.
    pub key: String,
    /// Comparison operator.
    pub op: ComparisonOp,
    /// Value to compare against.
    pub value: AttributeValue,
}

impl AttributeTest {
    pub fn evaluate(&self, attributes: &IndexMap<String, AttributeValue>) -> bool {
        let Some(attr) = attributes.get(&self.key) else { return false };
        match (&self.op, attr, &self.value) {
            (ComparisonOp::Eq, AttributeValue::Text(a), AttributeValue::Text(b)) => a == b,
            (ComparisonOp::Ne, AttributeValue::Text(a), AttributeValue::Text(b)) => a != b,
            (ComparisonOp::Eq, AttributeValue::Number(a), AttributeValue::Number(b)) => {
                (a - b).abs() < f64::EPSILON
            }
            (ComparisonOp::Ne, AttributeValue::Number(a), AttributeValue::Number(b)) => {
                (a - b).abs() >= f64::EPSILON
            }
            (ComparisonOp::Lt, AttributeValue::Number(a), AttributeValue::Number(b)) => a < b,
            (ComparisonOp::Le, AttributeValue::Number(a), AttributeValue::Number(b)) => a <= b,
            (ComparisonOp::Gt, AttributeValue::Number(a), AttributeValue::Number(b)) => a > b,
            (ComparisonOp::Ge, AttributeValue::Number(a), AttributeValue::Number(b)) => a >= b,
            _ => false, // type mismatch
        }
    }
}
```

**Placement:** `finstack/portfolio/src/types.rs`.

### 4. Position Layer Changes

`Position`, `PositionSpec`, and `CandidatePosition` all change from string tags to unified attributes.

**`Position`:**
- `tags: IndexMap<String, String>` → `attributes: IndexMap<String, AttributeValue>`
- `with_tag(key, value)` → `with_attribute(key, AttributeValue)` + convenience `with_text_attribute(key, &str)`, `with_numeric_attribute(key, f64)`
- `with_tags(...)` → `with_attributes(...)`
- Backward-compatible JSON deserialization: string values auto-deserialize as `Text` via `#[serde(untagged)]`

**`PositionSpec`:**
- `tags: IndexMap<String, String>` → `attributes: IndexMap<String, AttributeValue>`

**`CandidatePosition`:**
- `tags: IndexMap<String, String>` → `attributes: IndexMap<String, AttributeValue>`
- `with_tag(key, value)` → `with_attribute(key, AttributeValue)` + convenience methods

**`DecisionFeatures`:**
- `tags: IndexMap<String, String>` → `attributes: IndexMap<String, AttributeValue>`

### 5. `PerPositionMetric`

Gains `Attribute` for numeric attribute access and generalizes `TagEquals` to `AttributeIndicator`.

```rust
pub enum PerPositionMetric {
    /// From ValuationResult::measures using a standard MetricId.
    Metric(MetricId),
    /// From ValuationResult::measures using a string key (custom/bucketed).
    CustomKey(String),
    /// Numeric attribute value from position attributes.
    Attribute(String),
    /// Base currency PV of the position.
    PvBase,
    /// Native-currency PV of the position.
    PvNative,
    /// 1.0 if the attribute test passes, 0.0 otherwise.
    AttributeIndicator(AttributeTest),
    /// Constant scalar for all positions.
    Constant(f64),
}
```

**Changes vs. current:**
- Added `Attribute(String)` — reads numeric attribute by key, applies `MissingMetricPolicy` if absent or non-numeric.
- Renamed `TagEquals { key, value }` → `AttributeIndicator(AttributeTest)` — generalized to support all comparison operators and both text/numeric values.

### 6. `MetricExpr`

Removes `TagExposureShare`, adds optional `filter` for scoped aggregations.

```rust
pub enum MetricExpr {
    /// Σ w_i * m_i, optionally filtered.
    WeightedSum {
        metric: PerPositionMetric,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filter: Option<PositionFilter>,
    },
    /// Value-weighted average: Σ w_i * m_i (with implicit Σ w_i == 1), optionally filtered.
    ValueWeightedAverage {
        metric: PerPositionMetric,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filter: Option<PositionFilter>,
    },
}
```

**Removed:** `TagExposureShare { tag_key, tag_value }` — now expressed as `WeightedSum { metric: AttributeIndicator(...), filter: None }`.

**Filter semantics:** When `filter` is `Some`, positions not matching the filter get coefficient 0 in the LP. The filter is evaluated at problem setup time (not over decision variables), so linearity is preserved.

**Equivalence table:**

| Old | New |
|-----|-----|
| `TagExposureShare { tag_key: "rating", tag_value: "CCC" }` | `WeightedSum { metric: AttributeIndicator(rating == "CCC"), filter: None }` |
| N/A | `ValueWeightedAverage { metric: Attribute("esg_score"), filter: None }` |
| N/A | `WeightedSum { metric: Metric(DV01), filter: Some(ByAttribute(sector == "Energy")) }` |

### 7. `Constraint`

Consolidated from 7 variants to 4.

```rust
pub enum Constraint {
    /// Portfolio-level metric bound: metric_expr {op} rhs.
    MetricBound {
        label: Option<String>,
        metric: MetricExpr,
        op: Inequality,
        rhs: f64,
    },
    /// Per-position weight bounds, optionally filtered.
    WeightBounds {
        label: Option<String>,
        filter: PositionFilter,
        min: f64,
        max: f64,
    },
    /// Maximum turnover: Σ |w_new - w_current| <= max_turnover.
    MaxTurnover {
        label: Option<String>,
        max_turnover: f64,
    },
    /// Budget normalization: Σ w_i == rhs.
    Budget {
        rhs: f64,
    },
}
```

**Removed variants and equivalences:**

| Old Variant | New Expression |
|-------------|---------------|
| `TagExposureLimit { tag_key, tag_value, max_share }` | `MetricBound { metric: WeightedSum { metric: AttributeIndicator(key == value) }, op: Le, rhs: max_share }` |
| `TagExposureMinimum { tag_key, tag_value, min_share }` | `MetricBound { metric: WeightedSum { metric: AttributeIndicator(key == value) }, op: Ge, rhs: min_share }` |
| `MaxPositionDelta { filter, max_delta }` | Desugared at problem construction: compute `current_weight ± max_delta`, emit `WeightBounds` |

**Why `MaxTurnover` remains separate:** It requires auxiliary LP variables (`t_i >= |w_i - w0_i|`) that change LP structure. Not expressible as a simple `coefficients · w <= rhs` row.

**Convenience constructors:**

```rust
impl Constraint {
    pub fn exposure_limit(key: &str, value: &str, max_share: f64) -> Result<Self, ConstraintValidationError>;
    pub fn exposure_minimum(key: &str, value: &str, min_share: f64) -> Result<Self, ConstraintValidationError>;
    pub fn metric_band(metric: MetricExpr, min: f64, max: f64) -> Vec<Self>;
}
```

### 8. `PositionFilter`

Full boolean composition with attribute awareness.

```rust
pub enum PositionFilter {
    All,
    ByEntityId(EntityId),
    ByAttribute(AttributeTest),
    ByPositionIds(Vec<PositionId>),
    Not(Box<PositionFilter>),
    And(Vec<PositionFilter>),
    Or(Vec<PositionFilter>),
}
```

**Changes vs. current:**
- Removed `ByTag { key, value }` — replaced by `ByAttribute(AttributeTest { key, op: Eq, value: Text(v) })`
- Added `And(Vec<PositionFilter>)` and `Or(Vec<PositionFilter>)`

**Filter evaluation:**

```rust
fn matches_filter(attrs: &IndexMap<String, AttributeValue>, entity_id: &EntityId, position_id: &PositionId, filter: &PositionFilter) -> bool {
    match filter {
        All => true,
        ByEntityId(id) => entity_id == id,
        ByAttribute(test) => test.evaluate(attrs),
        ByPositionIds(ids) => ids.contains(position_id),
        Not(inner) => !matches_filter(attrs, entity_id, position_id, inner),
        And(filters) => filters.iter().all(|f| matches_filter(attrs, entity_id, position_id, f)),
        Or(filters) => filters.iter().any(|f| matches_filter(attrs, entity_id, position_id, f)),
    }
}
```

### 9. LP Solver Changes

The `DefaultLpOptimizer` constraint-lowering loop drops from 7 match arms to 4:

1. `MetricBound` → build coefficients via `build_metric_coefficients`, add one LP row
2. `WeightBounds` → tighten `DecisionFeatures.min_weight / max_weight`
3. `MaxTurnover` → auxiliary variables for L1 norm (unchanged logic)
4. `Budget` → equality row (unchanged logic)

**`build_metric_coefficients`** loses the `TagExposureShare` arm. The `filter` field on `MetricExpr` is evaluated per-position: non-matching positions get coefficient 0.

**`per_position_metric_value`** gains two arms:
- `Attribute(key)` → lookup `feat.attributes.get(key)`, return `Number(f64)`, apply `MissingMetricPolicy` on absence/type mismatch.
- `AttributeIndicator(test)` → `test.evaluate(&feat.attributes)` returns 1.0/0.0.

**Filter matching** is unified into a single function handling `And`/`Or`/`ByAttribute`, shared between `lp_solver.rs` and `decision.rs`.

### 10. Deletions

**Rust (`finstack/portfolio`):**
- `helpers.rs`: Delete `optimize_max_yield_with_ccc_limit`, `MaxYieldWithCccLimitResult`
- `mod.rs` / `lib.rs`: Remove re-exports of deleted types
- `constraints.rs`: Remove `TagExposureLimit`, `TagExposureMinimum`, `MaxPositionDelta` variants and constructor methods

**Python bindings (`finstack-py/src/bindings/portfolio/optimization.rs`):**
- Delete `optimize_max_yield` function

**Python stubs (`finstack-py/finstack/portfolio/__init__.pyi`):**
- Remove `optimize_max_yield` signature

**Python module (`finstack-py/finstack/portfolio/__init__.py`):**
- Remove `optimize_max_yield` from exports and `__all__`

**WASM bindings (`finstack-wasm/src/api/portfolio/mod.rs`):**
- Delete `optimizeMaxYield` function

### 11. Notebook Update

Rewrite `05_portfolio_and_scenarios/portfolio_optimization.ipynb` to demonstrate:
1. Maximize YTM via `ValueWeightedAverage { metric: Metric(Ytm) }`
2. Exposure limit via `MetricBound` + `AttributeIndicator`
3. Duration band via two `MetricBound` constraints (Le and Ge)
4. Numeric attribute constraint (e.g., ESG score)
5. Filtered aggregation (sector-scoped metric bound)
6. Cross-constraint with `And` filter (e.g., CCC Energy limit)

## Migration Summary

| Component | Action |
|-----------|--------|
| `AttributeValue`, `ComparisonOp`, `AttributeTest` | New types in `types.rs` |
| `Position.tags` | Rename to `attributes`, type change to `IndexMap<String, AttributeValue>` |
| `PositionSpec.tags` | Same |
| `CandidatePosition.tags` | Same |
| `DecisionFeatures.tags` | Same |
| `PerPositionMetric::TagEquals` | Rename to `AttributeIndicator(AttributeTest)` |
| `PerPositionMetric::Attribute` | New variant |
| `MetricExpr::TagExposureShare` | Remove |
| `MetricExpr` variants | Add optional `filter: Option<PositionFilter>` |
| `Constraint::TagExposureLimit` | Remove (use `MetricBound`) |
| `Constraint::TagExposureMinimum` | Remove (use `MetricBound`) |
| `Constraint::MaxPositionDelta` | Remove (desugar to `WeightBounds` at setup) |
| `PositionFilter::ByTag` | Replace with `ByAttribute(AttributeTest)` |
| `PositionFilter::And`, `Or` | New variants |
| `optimize_max_yield_with_ccc_limit` | Delete |
| `MaxYieldWithCccLimitResult` | Delete |
| Python `optimize_max_yield` | Delete |
| WASM `optimizeMaxYield` | Delete |
| LP solver constraint loop | 7 arms → 4 arms |
| Convenience constructors | Add `exposure_limit`, `exposure_minimum`, `metric_band` on `Constraint` |
