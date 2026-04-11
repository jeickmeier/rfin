# Configurable Portfolio Optimization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace hard-coded optimization constraints with a unified, composable constraint model using `AttributeValue`, consolidated `MetricBound`, boolean `PositionFilter` composition, and filtered `MetricExpr` aggregations.

**Architecture:** New foundational types (`AttributeValue`, `ComparisonOp`, `AttributeTest`) in `types.rs`. Position layer migrates from `tags: IndexMap<String, String>` to `attributes: IndexMap<String, AttributeValue>`. `Constraint` enum consolidated from 7 to 4 variants. LP solver simplified. Hard-coded CCC helper deleted. Bindings and notebook updated.

**Tech Stack:** Rust (serde, good_lp, indexmap), PyO3, wasm-bindgen

**Spec:** `docs/superpowers/specs/2026-04-11-configurable-portfolio-optimization-design.md`

---

## File Map

**Create:** (none — all types go into existing files)

**Modify (Rust core):**
- `finstack/portfolio/src/types.rs` — add `AttributeValue`, `ComparisonOp`, `AttributeTest`
- `finstack/portfolio/src/position.rs` — `tags` → `attributes`, update methods
- `finstack/portfolio/src/optimization/universe.rs` — `CandidatePosition.tags` → `.attributes`, `PositionFilter` update
- `finstack/portfolio/src/optimization/types.rs` — `PerPositionMetric`, `MetricExpr` changes
- `finstack/portfolio/src/optimization/constraints.rs` — remove 3 variants, add convenience constructors
- `finstack/portfolio/src/optimization/decision.rs` — `DecisionFeatures.tags` → `.attributes`
- `finstack/portfolio/src/optimization/lp_solver.rs` — adapt solver to new types
- `finstack/portfolio/src/optimization/helpers.rs` — delete CCC helper
- `finstack/portfolio/src/optimization/mod.rs` — update re-exports
- `finstack/portfolio/src/lib.rs` — update re-exports
- `finstack/portfolio/src/grouping.rs` — update `position.tags` references
- `finstack/portfolio/src/portfolio.rs` — update `positions_with_tag`

**Modify (Bindings):**
- `finstack-py/src/bindings/portfolio/optimization.rs` — delete `optimize_max_yield`
- `finstack-py/src/bindings/portfolio/mod.rs` — update `__all__`
- `finstack-py/finstack/portfolio/__init__.py` — update exports
- `finstack-py/finstack/portfolio/__init__.pyi` — update stubs
- `finstack-wasm/src/api/portfolio/mod.rs` — delete `optimizeMaxYield`
- `finstack-wasm/exports/portfolio.js` — remove export
- `finstack-wasm/index.d.ts` — remove type

**Modify (Examples):**
- `finstack-py/examples/notebooks/05_portfolio_and_scenarios/portfolio_optimization.ipynb`

---

### Task 1: Add Foundational Types

Purely additive — adds `AttributeValue`, `ComparisonOp`, `AttributeTest` to the existing `types.rs`. Nothing else changes. The crate compiles before and after.

**Files:**
- Modify: `finstack/portfolio/src/types.rs`
- Modify: `finstack/portfolio/src/lib.rs` (re-exports)

- [ ] **Step 1: Add `AttributeValue` enum to `types.rs`**

Append after the `Entity` impl block, before `#[cfg(test)]`:

```rust
/// Value stored in a position or candidate attribute.
///
/// Positions carry key/value attributes for grouping, filtering, and
/// optimization constraints.  Text values represent categorical data
/// (rating, sector), while numeric values represent continuous data
/// (credit score, ESG score) usable in metric expressions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AttributeValue {
    /// Categorical / string attribute (e.g., rating = "CCC", sector = "Energy").
    Text(String),
    /// Numeric attribute (e.g., credit_score = 650.0, esg_score = 72.5).
    Number(f64),
}

impl AttributeValue {
    /// Return the text value if this is a `Text` variant.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(s) => Some(s),
            Self::Number(_) => None,
        }
    }

    /// Return the numeric value if this is a `Number` variant.
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            Self::Text(_) => None,
        }
    }
}

impl fmt::Display for AttributeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(s) => write!(f, "{s}"),
            Self::Number(n) => write!(f, "{n}"),
        }
    }
}

impl From<&str> for AttributeValue {
    fn from(s: &str) -> Self {
        Self::Text(s.to_string())
    }
}

impl From<String> for AttributeValue {
    fn from(s: String) -> Self {
        Self::Text(s)
    }
}

impl From<f64> for AttributeValue {
    fn from(n: f64) -> Self {
        Self::Number(n)
    }
}
```

- [ ] **Step 2: Add `ComparisonOp` enum to `types.rs`**

Append after `AttributeValue`:

```rust
/// Comparison operator for attribute-based filtering.
///
/// For [`AttributeValue::Text`] attributes, only [`ComparisonOp::Eq`] and
/// [`ComparisonOp::Ne`] are meaningful; ordering comparisons on text return
/// `false`.  For [`AttributeValue::Number`] attributes, all six operators
/// apply.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOp {
    /// Equal.
    Eq,
    /// Not equal.
    Ne,
    /// Less than (numeric only).
    Lt,
    /// Less than or equal (numeric only).
    Le,
    /// Greater than (numeric only).
    Gt,
    /// Greater than or equal (numeric only).
    Ge,
}
```

- [ ] **Step 3: Add `AttributeTest` struct to `types.rs`**

Append after `ComparisonOp`:

```rust
/// Predicate that tests a single position attribute against a value.
///
/// Reusable building block for [`crate::optimization::PositionFilter::ByAttribute`]
/// and [`crate::optimization::PerPositionMetric::AttributeIndicator`].
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
    /// Create a new attribute test.
    pub fn new(
        key: impl Into<String>,
        op: ComparisonOp,
        value: impl Into<AttributeValue>,
    ) -> Self {
        Self {
            key: key.into(),
            op,
            value: value.into(),
        }
    }

    /// Convenience: text equality test.
    pub fn text_eq(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(key, ComparisonOp::Eq, AttributeValue::Text(value.into()))
    }

    /// Convenience: numeric comparison test.
    pub fn numeric(key: impl Into<String>, op: ComparisonOp, value: f64) -> Self {
        Self::new(key, op, AttributeValue::Number(value))
    }

    /// Evaluate this test against a set of attributes.
    ///
    /// Returns `false` if the key is absent or types are incompatible
    /// (e.g., ordering comparison on text).
    pub fn evaluate(&self, attributes: &IndexMap<String, AttributeValue>) -> bool {
        let Some(attr) = attributes.get(&self.key) else {
            return false;
        };
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
            _ => false,
        }
    }
}
```

- [ ] **Step 4: Add tests for new types**

Add to the existing `#[cfg(test)] mod tests` in `types.rs`:

```rust
#[test]
fn test_attribute_value_text() {
    let v = AttributeValue::Text("CCC".to_string());
    assert_eq!(v.as_text(), Some("CCC"));
    assert_eq!(v.as_number(), None);
    assert_eq!(format!("{v}"), "CCC");
}

#[test]
fn test_attribute_value_number() {
    let v = AttributeValue::Number(72.5);
    assert_eq!(v.as_text(), None);
    assert_eq!(v.as_number(), Some(72.5));
    assert_eq!(format!("{v}"), "72.5");
}

#[test]
fn test_attribute_value_from() {
    let t: AttributeValue = "hello".into();
    assert!(matches!(t, AttributeValue::Text(_)));
    let n: AttributeValue = 42.0_f64.into();
    assert!(matches!(n, AttributeValue::Number(_)));
}

#[test]
fn test_attribute_value_serde_text() {
    let v = AttributeValue::Text("CCC".to_string());
    let json = serde_json::to_string(&v).unwrap();
    assert_eq!(json, "\"CCC\"");
    let round: AttributeValue = serde_json::from_str(&json).unwrap();
    assert_eq!(round, v);
}

#[test]
fn test_attribute_value_serde_number() {
    let v = AttributeValue::Number(72.5);
    let json = serde_json::to_string(&v).unwrap();
    assert_eq!(json, "72.5");
    let round: AttributeValue = serde_json::from_str(&json).unwrap();
    assert_eq!(round, v);
}

#[test]
fn test_attribute_test_text_eq() {
    let attrs = IndexMap::from([
        ("rating".to_string(), AttributeValue::Text("CCC".to_string())),
    ]);
    assert!(AttributeTest::text_eq("rating", "CCC").evaluate(&attrs));
    assert!(!AttributeTest::text_eq("rating", "BB").evaluate(&attrs));
    assert!(!AttributeTest::text_eq("missing", "CCC").evaluate(&attrs));
}

#[test]
fn test_attribute_test_numeric_comparisons() {
    let attrs = IndexMap::from([
        ("score".to_string(), AttributeValue::Number(650.0)),
    ]);
    assert!(AttributeTest::numeric("score", ComparisonOp::Ge, 600.0).evaluate(&attrs));
    assert!(!AttributeTest::numeric("score", ComparisonOp::Lt, 600.0).evaluate(&attrs));
    assert!(AttributeTest::numeric("score", ComparisonOp::Le, 650.0).evaluate(&attrs));
    assert!(AttributeTest::numeric("score", ComparisonOp::Eq, 650.0).evaluate(&attrs));
    assert!(!AttributeTest::numeric("score", ComparisonOp::Ne, 650.0).evaluate(&attrs));
    assert!(AttributeTest::numeric("score", ComparisonOp::Gt, 600.0).evaluate(&attrs));
}

#[test]
fn test_attribute_test_type_mismatch() {
    let attrs = IndexMap::from([
        ("rating".to_string(), AttributeValue::Text("CCC".to_string())),
    ]);
    // Numeric comparison on text attribute returns false
    assert!(!AttributeTest::numeric("rating", ComparisonOp::Gt, 5.0).evaluate(&attrs));
}
```

- [ ] **Step 5: Update `lib.rs` re-exports**

Add to the `pub use types::` line in `finstack/portfolio/src/lib.rs`:

```rust
pub use types::{AttributeTest, AttributeValue, ComparisonOp, Entity, EntityId, PositionId, DUMMY_ENTITY_ID};
```

- [ ] **Step 6: Run tests and compile**

Run: `cargo test -p finstack-portfolio --lib types`
Expected: All new tests pass, existing tests still pass.

- [ ] **Step 7: Commit**

```bash
git add finstack/portfolio/src/types.rs finstack/portfolio/src/lib.rs
git commit -m "feat(portfolio): add AttributeValue, ComparisonOp, AttributeTest types"
```

---

### Task 2: Migrate Position Layer — `tags` to `attributes`

Change `Position.tags` and `PositionSpec.tags` from `IndexMap<String, String>` to `IndexMap<String, AttributeValue>`. Update all methods and crate-wide references. This is a breaking change within the crate, so all consumers must update in the same compilation pass.

**Files:**
- Modify: `finstack/portfolio/src/position.rs`
- Modify: `finstack/portfolio/src/portfolio.rs`
- Modify: `finstack/portfolio/src/grouping.rs`
- Modify: `finstack/portfolio/src/builder.rs` (if it references position tags)

- [ ] **Step 1: Update `Position` struct and methods in `position.rs`**

Replace the `tags` field and all tag-related methods:

1. Change field: `pub tags: IndexMap<String, String>` → `pub attributes: IndexMap<String, AttributeValue>`
2. Replace `with_tag` method:

```rust
/// Add a text attribute to the position.
pub fn with_text_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
    self.attributes.insert(key.into(), AttributeValue::Text(value.into()));
    self
}

/// Add a numeric attribute to the position.
pub fn with_numeric_attribute(mut self, key: impl Into<String>, value: f64) -> Self {
    self.attributes.insert(key.into(), AttributeValue::Number(value));
    self
}

/// Add an attribute to the position.
pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<AttributeValue>) -> Self {
    self.attributes.insert(key.into(), value.into());
    self
}
```

3. Replace `with_tags`:

```rust
/// Add multiple text attributes at once.
pub fn with_text_attributes<K, V, I>(mut self, attrs: I) -> Self
where
    K: Into<String>,
    V: Into<String>,
    I: IntoIterator<Item = (K, V)>,
{
    for (k, v) in attrs {
        self.attributes.insert(k.into(), AttributeValue::Text(v.into()));
    }
    self
}
```

4. Update `Position::new` to initialize `attributes: IndexMap::new()` instead of `tags: IndexMap::new()`.
5. Update `Debug` impl to use `attributes` field.
6. Add `use crate::types::AttributeValue;` import.

- [ ] **Step 2: Update `PositionSpec` in `position.rs`**

1. Change field: `pub tags: IndexMap<String, String>` → `pub attributes: IndexMap<String, AttributeValue>`
2. Update serde attribute: `#[serde(default, skip_serializing_if = "IndexMap::is_empty")]`
3. Update `to_spec()`: `tags: self.tags.clone()` → `attributes: self.attributes.clone()`
4. Update `from_spec()`: `position.tags = tags` → `position.attributes = attributes`

- [ ] **Step 3: Update `portfolio.rs` — `positions_with_tag` method**

Replace:
```rust
pub fn positions_with_tag(&self, key: &str, value: &str) -> Vec<&Position> {
    self.positions
        .iter()
        .filter(|p| p.tags.get(key).map(|v| v.as_str()) == Some(value))
        .collect()
}
```
With:
```rust
pub fn positions_with_attribute(&self, key: &str, value: &AttributeValue) -> Vec<&Position> {
    self.positions
        .iter()
        .filter(|p| p.attributes.get(key) == Some(value))
        .collect()
}
```

Add `use crate::types::AttributeValue;` import.

- [ ] **Step 4: Update `grouping.rs` — attribute-based grouping**

The grouping functions access `position.tags.get(attr_key)` which returns `Option<&String>`. After migration it returns `Option<&AttributeValue>`. Update to extract text representation:

Replace `position.tags.get(attr_key)` usages with:
```rust
position.attributes.get(attr_key).and_then(|v| v.as_text()).map(|s| s.to_string())
```

Update the `group_by_attribute` function to group by the display form of `AttributeValue`, and `aggregate_by_attribute` similarly.

- [ ] **Step 5: Update tests in `position.rs`**

Replace `.with_tag("type", "cash").with_tag("rating", "AAA")` with:
```rust
.with_text_attribute("type", "cash")
.with_text_attribute("rating", "AAA")
```

Update assertions: `position.tags.get("type")` → `position.attributes.get("type")` and compare against `Some(&AttributeValue::Text("cash".to_string()))`.

- [ ] **Step 6: Update tests in `grouping.rs`**

Replace all `.with_tag(...)` calls with `.with_text_attribute(...)`.

- [ ] **Step 7: Compile and run tests**

Run: `cargo test -p finstack-portfolio --lib position grouping`
Expected: All tests pass.

- [ ] **Step 8: Commit**

```bash
git add finstack/portfolio/src/position.rs finstack/portfolio/src/portfolio.rs finstack/portfolio/src/grouping.rs finstack/portfolio/src/builder.rs
git commit -m "refactor(portfolio): migrate Position.tags to Position.attributes with AttributeValue"
```

---

### Task 3: Migrate CandidatePosition and Update PositionFilter

Update `CandidatePosition.tags` → `.attributes` and add `And`/`Or`/`ByAttribute` to `PositionFilter`, removing `ByTag`.

**Files:**
- Modify: `finstack/portfolio/src/optimization/universe.rs`

- [ ] **Step 1: Update `CandidatePosition` struct**

1. Change field: `pub tags: IndexMap<String, String>` → `pub attributes: IndexMap<String, AttributeValue>`
2. Replace `with_tag`:

```rust
/// Add a text attribute to the candidate.
pub fn with_text_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
    self.attributes.insert(key.into(), AttributeValue::Text(value.into()));
    self
}

/// Add a numeric attribute to the candidate.
pub fn with_numeric_attribute(mut self, key: impl Into<String>, value: f64) -> Self {
    self.attributes.insert(key.into(), AttributeValue::Number(value));
    self
}

/// Add an attribute to the candidate.
pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<AttributeValue>) -> Self {
    self.attributes.insert(key.into(), value.into());
    self
}
```

3. Update `CandidatePosition::new`: `tags: IndexMap::new()` → `attributes: IndexMap::new()`.
4. Update `Debug` impl: `.field("tags", &self.tags)` → `.field("attributes", &self.attributes)`.
5. Add `use crate::types::{AttributeTest, AttributeValue};` import.

- [ ] **Step 2: Update `PositionFilter` enum**

Replace the entire enum:

```rust
/// Filters for selecting which positions are included in a rule.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PositionFilter {
    /// All positions in the portfolio.
    All,

    /// Filter by entity ID.
    ByEntityId(EntityId),

    /// Filter by attribute test (text equality, numeric comparison, etc.).
    ByAttribute(AttributeTest),

    /// Filter by multiple position IDs.
    ByPositionIds(Vec<PositionId>),

    /// Exclude positions matching the inner filter.
    Not(Box<PositionFilter>),

    /// Conjunction: positions matching ALL inner filters.
    And(Vec<PositionFilter>),

    /// Disjunction: positions matching ANY inner filter.
    Or(Vec<PositionFilter>),
}
```

- [ ] **Step 3: Compile — fix all `ByTag` references**

Grep for `ByTag` across the crate and replace each occurrence. The main spots are:
- `decision.rs:matches_filter` — the `ByTag { key, value }` arm
- `lp_solver.rs:matches_filter` — same
- `lp_solver.rs:matches_candidate_filter` — same
- Any test files using `ByTag`

Replace each `PositionFilter::ByTag { key, value }` match arm with:
```rust
PositionFilter::ByAttribute(test) => test.evaluate(&position.attributes),
```

Add the new `And`/`Or` arms:
```rust
PositionFilter::And(filters) => filters.iter().all(|f| matches_filter(position, f)),
PositionFilter::Or(filters) => filters.iter().any(|f| matches_filter(position, f)),
```

- [ ] **Step 4: Compile and run tests**

Run: `cargo test -p finstack-portfolio`
Expected: Compiles. Existing tests that used `ByTag` need updating (use `ByAttribute(AttributeTest::text_eq(key, value))` instead).

- [ ] **Step 5: Commit**

```bash
git add finstack/portfolio/src/optimization/universe.rs finstack/portfolio/src/optimization/decision.rs finstack/portfolio/src/optimization/lp_solver.rs
git commit -m "refactor(portfolio): migrate CandidatePosition to attributes, add And/Or/ByAttribute to PositionFilter"
```

---

### Task 4: Update `DecisionFeatures` and `build_decision_space`

Migrate the internal decision space from string tags to `AttributeValue` attributes.

**Files:**
- Modify: `finstack/portfolio/src/optimization/decision.rs`

- [ ] **Step 1: Update `DecisionFeatures` struct**

Change: `pub tags: IndexMap<String, String>` → `pub attributes: IndexMap<String, AttributeValue>`

Add import: `use crate::types::AttributeValue;`

- [ ] **Step 2: Update `build_decision_space` function**

1. Replace `tags: position.tags.clone()` → `attributes: position.attributes.clone()` in the existing-position loop.
2. Replace `tags: candidate.tags.clone()` → `attributes: candidate.attributes.clone()` in the candidate loop.
3. Update `matches_filter` function in decision.rs to use `position.attributes` instead of `position.tags`.
4. Add `And`/`Or`/`ByAttribute` arms to the local `matches_filter` (same pattern as Task 3 Step 3).

- [ ] **Step 3: Update `.with_tags(candidate.tags.clone())` call**

In the candidate pricing section where a temporary `Position` is built for candidates, replace:
```rust
.with_tags(candidate.tags.clone())
```
With iteration that converts `AttributeValue` back or use the new method:
```rust
.with_text_attributes(
    candidate.attributes.iter()
        .filter_map(|(k, v)| v.as_text().map(|t| (k.as_str(), t)))
)
```

Or better, add a helper on Position that accepts `IndexMap<String, AttributeValue>` directly.

- [ ] **Step 4: Compile and run tests**

Run: `cargo test -p finstack-portfolio`
Expected: Compiles cleanly.

- [ ] **Step 5: Commit**

```bash
git add finstack/portfolio/src/optimization/decision.rs
git commit -m "refactor(portfolio): migrate DecisionFeatures to AttributeValue attributes"
```

---

### Task 5: Update `PerPositionMetric`

Rename `TagEquals` to `AttributeIndicator`, add `Attribute` variant.

**Files:**
- Modify: `finstack/portfolio/src/optimization/types.rs`
- Modify: `finstack/portfolio/src/optimization/lp_solver.rs`

- [ ] **Step 1: Update `PerPositionMetric` enum in `types.rs`**

Replace the `TagEquals` variant and add `Attribute`:

```rust
pub enum PerPositionMetric {
    /// Directly from `ValuationResult::measures` using a standard `MetricId`.
    Metric(MetricId),

    /// From `ValuationResult::measures` using a string key (for custom or
    /// bucketed metrics stored by name).
    CustomKey(String),

    /// Numeric attribute value from position attributes.
    ///
    /// Returns the `f64` value if the attribute is [`AttributeValue::Number`].
    /// Treated as missing (subject to [`MissingMetricPolicy`]) if absent or
    /// if the attribute is [`AttributeValue::Text`].
    Attribute(String),

    /// Use the base currency PV of the position (after scaling).
    PvBase,

    /// Use the native-currency PV of the position (after scaling).
    PvNative,

    /// 1.0 if the attribute test passes, 0.0 otherwise.
    ///
    /// Generalizes the former `TagEquals` variant to support all comparison
    /// operators and both text and numeric attributes.
    AttributeIndicator(AttributeTest),

    /// Constant scalar for all positions.
    Constant(f64),
}
```

Add import: `use crate::types::AttributeTest;`

- [ ] **Step 2: Update `per_position_metric_value` in `lp_solver.rs`**

Replace the `TagEquals` match arm:
```rust
PerPositionMetric::TagEquals { key, value } => {
    let matches = feat.tags.get(key) == Some(value);
    Some(if matches { 1.0 } else { 0.0 })
}
```

With:
```rust
PerPositionMetric::Attribute(key) => {
    feat.attributes.get(key).and_then(|v| v.as_number())
}
PerPositionMetric::AttributeIndicator(test) => {
    Some(if test.evaluate(&feat.attributes) { 1.0 } else { 0.0 })
}
```

- [ ] **Step 3: Compile and fix any remaining `TagEquals` references**

Search crate for `TagEquals` — update any test code or helper code that references it.

- [ ] **Step 4: Run tests**

Run: `cargo test -p finstack-portfolio`
Expected: Compiles. Tests using `TagEquals` need updating to `AttributeIndicator`.

- [ ] **Step 5: Commit**

```bash
git add finstack/portfolio/src/optimization/types.rs finstack/portfolio/src/optimization/lp_solver.rs
git commit -m "refactor(portfolio): rename TagEquals to AttributeIndicator, add Attribute metric variant"
```

---

### Task 6: Update `MetricExpr` — Remove `TagExposureShare`, Add `filter`

**Files:**
- Modify: `finstack/portfolio/src/optimization/types.rs`
- Modify: `finstack/portfolio/src/optimization/lp_solver.rs`

- [ ] **Step 1: Update `MetricExpr` enum in `types.rs`**

Replace the entire enum:

```rust
/// Portfolio-level scalar metric expressed in terms of position metrics + weights.
///
/// These expressions are intentionally restricted to linear or linearized forms
/// so they can be represented by the LP-based optimizer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MetricExpr {
    /// `sum_i w_i * m_i`, where `m_i` comes from a [`PerPositionMetric`].
    ///
    /// When `filter` is set, only positions matching the filter contribute
    /// (non-matching positions get coefficient 0).
    WeightedSum {
        /// Per-position metric to aggregate.
        metric: PerPositionMetric,
        /// Optional filter to scope the aggregation.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filter: Option<super::universe::PositionFilter>,
    },

    /// Value-weighted average: `sum_i w_i * m_i`, with implicit `sum_i w_i == 1`.
    ///
    /// When `filter` is set, only positions matching the filter contribute.
    ValueWeightedAverage {
        /// Per-position metric to average.
        metric: PerPositionMetric,
        /// Optional filter to scope the aggregation.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filter: Option<super::universe::PositionFilter>,
    },
}
```

- [ ] **Step 2: Update `build_metric_coefficients` in `lp_solver.rs`**

Remove the `TagExposureShare` match arm entirely.

Update the `WeightedSum`/`ValueWeightedAverage` arm to handle the `filter` field:

```rust
MetricExpr::WeightedSum { metric, filter }
| MetricExpr::ValueWeightedAverage { metric, filter } => {
    for (item, feat) in items.iter().zip(feats) {
        if let Some(f) = filter {
            if !Self::matches_decision_filter(item, feat, f, portfolio) {
                coeffs.push(0.0);
                continue;
            }
        }
        let m_i = match metric {
            PerPositionMetric::PvNative => feat.pv_base,
            _ => Self::per_position_metric_value(metric, feat, missing_policy)?,
        };
        coeffs.push(m_i);
    }
}
```

Add a helper `matches_decision_filter` that evaluates a `PositionFilter` against a `DecisionItem` + `DecisionFeatures`:

```rust
fn matches_decision_filter(
    item: &DecisionItem,
    feat: &DecisionFeatures,
    filter: &PositionFilter,
    portfolio: &Portfolio,
) -> bool {
    match filter {
        PositionFilter::All => true,
        PositionFilter::ByEntityId(id) => {
            if let Some(pos) = portfolio.get_position(item.position_id.as_str()) {
                pos.entity_id == *id
            } else {
                false
            }
        }
        PositionFilter::ByAttribute(test) => test.evaluate(&feat.attributes),
        PositionFilter::ByPositionIds(ids) => ids.contains(&item.position_id),
        PositionFilter::Not(inner) => {
            !Self::matches_decision_filter(item, feat, inner, portfolio)
        }
        PositionFilter::And(filters) => filters
            .iter()
            .all(|f| Self::matches_decision_filter(item, feat, f, portfolio)),
        PositionFilter::Or(filters) => filters
            .iter()
            .any(|f| Self::matches_decision_filter(item, feat, f, portfolio)),
    }
}
```

- [ ] **Step 3: Update `required_metrics` scan in `lp_solver.rs`**

The scan over `MetricExpr` variants no longer has `TagExposureShare`. Remove that arm. The `WeightedSum`/`ValueWeightedAverage` arms are unchanged.

- [ ] **Step 4: Compile and run tests**

Run: `cargo test -p finstack-portfolio`
Expected: Compiles. Any tests using `TagExposureShare` need updating.

- [ ] **Step 5: Commit**

```bash
git add finstack/portfolio/src/optimization/types.rs finstack/portfolio/src/optimization/lp_solver.rs
git commit -m "refactor(portfolio): remove TagExposureShare, add filter to MetricExpr"
```

---

### Task 7: Consolidate `Constraint` Enum

Remove `TagExposureLimit`, `TagExposureMinimum`, `MaxPositionDelta`. Add convenience constructors. Update LP solver constraint loop.

**Files:**
- Modify: `finstack/portfolio/src/optimization/constraints.rs`
- Modify: `finstack/portfolio/src/optimization/lp_solver.rs`

- [ ] **Step 1: Remove three variants from `Constraint` enum**

Delete:
- `TagExposureLimit { label, tag_key, tag_value, max_share }`
- `TagExposureMinimum { label, tag_key, tag_value, min_share }`
- `MaxPositionDelta { label, filter, max_delta }`

The enum should now have exactly: `MetricBound`, `WeightBounds`, `MaxTurnover`, `Budget`.

- [ ] **Step 2: Update `Constraint::label()` method**

Remove the three deleted match arms.

- [ ] **Step 3: Remove old constructors, add new convenience constructors**

Delete: `tag_exposure_limit`, `tag_exposure_limit_with_label`, `tag_exposure_minimum`, `tag_exposure_minimum_with_label`.

Add:

```rust
/// Shorthand for attribute exposure limit: `Σ w_i * I[attr == value] <= max_share`.
pub fn exposure_limit(
    key: impl Into<String>,
    value: impl Into<String>,
    max_share: f64,
) -> Result<Self, ConstraintValidationError> {
    Self::exposure_limit_with_label(None, key, value, max_share)
}

/// Attribute exposure limit with a label.
pub fn exposure_limit_with_label(
    label: Option<String>,
    key: impl Into<String>,
    value: impl Into<String>,
    max_share: f64,
) -> Result<Self, ConstraintValidationError> {
    if !(0.0..=1.0).contains(&max_share) {
        return Err(ConstraintValidationError {
            message: format!("max_share must be in [0, 1], got {max_share}"),
        });
    }
    Ok(Self::MetricBound {
        label,
        metric: super::types::MetricExpr::WeightedSum {
            metric: super::types::PerPositionMetric::AttributeIndicator(
                crate::types::AttributeTest::text_eq(key, value),
            ),
            filter: None,
        },
        op: super::constraints::Inequality::Le,
        rhs: max_share,
    })
}

/// Shorthand for attribute exposure minimum: `Σ w_i * I[attr == value] >= min_share`.
pub fn exposure_minimum(
    key: impl Into<String>,
    value: impl Into<String>,
    min_share: f64,
) -> Result<Self, ConstraintValidationError> {
    Self::exposure_minimum_with_label(None, key, value, min_share)
}

/// Attribute exposure minimum with a label.
pub fn exposure_minimum_with_label(
    label: Option<String>,
    key: impl Into<String>,
    value: impl Into<String>,
    min_share: f64,
) -> Result<Self, ConstraintValidationError> {
    if !(0.0..=1.0).contains(&min_share) {
        return Err(ConstraintValidationError {
            message: format!("min_share must be in [0, 1], got {min_share}"),
        });
    }
    Ok(Self::MetricBound {
        label,
        metric: super::types::MetricExpr::WeightedSum {
            metric: super::types::PerPositionMetric::AttributeIndicator(
                crate::types::AttributeTest::text_eq(key, value),
            ),
            filter: None,
        },
        op: super::constraints::Inequality::Ge,
        rhs: min_share,
    })
}

/// Create two `MetricBound` constraints forming a band: `min <= metric <= max`.
pub fn metric_band(
    metric: super::types::MetricExpr,
    min: f64,
    max: f64,
) -> Vec<Self> {
    vec![
        Self::MetricBound {
            label: None,
            metric: metric.clone(),
            op: Inequality::Ge,
            rhs: min,
        },
        Self::MetricBound {
            label: None,
            metric,
            op: Inequality::Le,
            rhs: max,
        },
    ]
}
```

- [ ] **Step 4: Update LP solver constraint loop in `lp_solver.rs`**

Remove the `TagExposureLimit`, `TagExposureMinimum`, and `MaxPositionDelta` match arms from the constraint-lowering loop in `optimize()`.

The loop should now only match: `MetricBound`, `WeightBounds`, `MaxTurnover`, `Budget`.

Also remove the `MaxPositionDelta` handling from the weight-bounds application loop (Step 3 in the current solver).

- [ ] **Step 5: Update tests in `constraints.rs`**

Rewrite tests to use `exposure_limit` / `exposure_minimum` convenience constructors and verify they produce `MetricBound` variants:

```rust
#[test]
fn test_exposure_limit_validation() {
    assert!(Constraint::exposure_limit("rating", "CCC", 0.0).is_ok());
    assert!(Constraint::exposure_limit("rating", "CCC", 1.0).is_ok());
    assert!(Constraint::exposure_limit("rating", "CCC", 0.5).is_ok());
    let result = Constraint::exposure_limit("rating", "CCC", -0.1);
    assert!(result.is_err());
    let result = Constraint::exposure_limit("rating", "CCC", 1.5);
    assert!(result.is_err());
}

#[test]
fn test_exposure_limit_produces_metric_bound() {
    let c = Constraint::exposure_limit("rating", "CCC", 0.10).unwrap();
    assert!(matches!(c, Constraint::MetricBound { .. }));
}
```

- [ ] **Step 6: Compile and run tests**

Run: `cargo test -p finstack-portfolio`
Expected: All tests pass. The LP solver has fewer match arms.

- [ ] **Step 7: Commit**

```bash
git add finstack/portfolio/src/optimization/constraints.rs finstack/portfolio/src/optimization/lp_solver.rs
git commit -m "refactor(portfolio): consolidate Constraint enum from 7 to 4 variants"
```

---

### Task 8: Delete Hard-Coded Helpers and Update Re-Exports

Remove `optimize_max_yield_with_ccc_limit`, `MaxYieldWithCccLimitResult`, and clean up re-exports.

**Files:**
- Modify: `finstack/portfolio/src/optimization/helpers.rs`
- Modify: `finstack/portfolio/src/optimization/mod.rs`
- Modify: `finstack/portfolio/src/lib.rs`

- [ ] **Step 1: Delete CCC helper from `helpers.rs`**

Remove the `MaxYieldWithCccLimitResult` struct and the `optimize_max_yield_with_ccc_limit` function entirely. Keep `PortfolioOptimizationSpec`, `PortfolioOptimizationResultJson`, and `optimize_from_spec`.

Also update `optimize_from_spec` if it references any removed types (it shouldn't — it uses the generic API).

- [ ] **Step 2: Update `mod.rs` re-exports**

Remove from the `pub use helpers::` line:
- `optimize_max_yield_with_ccc_limit`
- `MaxYieldWithCccLimitResult`

Also remove re-exports of deleted constraint variants. The `pub use constraints::` line should still export `Constraint`, `ConstraintValidationError`, `Inequality`.

- [ ] **Step 3: Update `lib.rs` re-exports**

Change:
```rust
pub use optimization::{
    optimize_from_spec, optimize_max_yield_with_ccc_limit, MaxYieldWithCccLimitResult,
    PortfolioOptimizationProblem, PortfolioOptimizationResult, PortfolioOptimizationResultJson,
    PortfolioOptimizationSpec,
};
```

To:
```rust
pub use optimization::{
    optimize_from_spec, PortfolioOptimizationProblem, PortfolioOptimizationResult,
    PortfolioOptimizationResultJson, PortfolioOptimizationSpec,
};
```

- [ ] **Step 4: Compile and run tests**

Run: `cargo test -p finstack-portfolio`
Expected: Compiles. No tests should reference the deleted helper.

- [ ] **Step 5: Commit**

```bash
git add finstack/portfolio/src/optimization/helpers.rs finstack/portfolio/src/optimization/mod.rs finstack/portfolio/src/lib.rs
git commit -m "refactor(portfolio): delete optimize_max_yield_with_ccc_limit helper"
```

---

### Task 9: Full Rust Crate Verification

Run the full test suite and clippy to ensure everything compiles cleanly.

**Files:** (none modified — verification only)

- [ ] **Step 1: Run full test suite**

Run: `cargo test -p finstack-portfolio`
Expected: All tests pass.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy -p finstack-portfolio -- -D warnings`
Expected: No warnings.

- [ ] **Step 3: Run formatter**

Run: `cargo fmt -p finstack-portfolio`
Expected: No changes (or apply formatting).

- [ ] **Step 4: Fix any issues found**

If any tests fail or clippy warnings appear, fix them and re-run.

- [ ] **Step 5: Commit any fixes**

```bash
git add -A finstack/portfolio/
git commit -m "fix(portfolio): resolve clippy warnings and test failures from optimization refactor"
```

---

### Task 10: Update Python Bindings

Delete `optimize_max_yield`, update module registration and `__all__`.

**Files:**
- Modify: `finstack-py/src/bindings/portfolio/optimization.rs`
- Modify: `finstack-py/src/bindings/portfolio/mod.rs`

- [ ] **Step 1: Delete `optimize_max_yield` function from `optimization.rs`**

Remove the entire `#[pyfunction] fn optimize_max_yield(...)` function (lines 40-84 of the current file).

Update `register` to remove the `optimize_max_yield` registration:

```rust
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(optimize_portfolio, m)?)?;
    Ok(())
}
```

- [ ] **Step 2: Update `__all__` in `mod.rs`**

Remove `"optimize_max_yield"` from the exports list:

```rust
let exports = vec![
    "parse_portfolio_spec",
    "build_portfolio_from_spec",
    "portfolio_result_total_value",
    "portfolio_result_get_metric",
    "aggregate_metrics",
    "value_portfolio",
    "aggregate_cashflows",
    "apply_scenario_and_revalue",
    "optimize_portfolio",
];
```

- [ ] **Step 3: Build Python bindings**

Run: `make python-dev`
Expected: Compiles successfully.

- [ ] **Step 4: Commit**

```bash
git add finstack-py/src/bindings/portfolio/optimization.rs finstack-py/src/bindings/portfolio/mod.rs
git commit -m "refactor(python): remove optimize_max_yield binding"
```

---

### Task 11: Update WASM Bindings

Delete `optimizeMaxYield` and update exports.

**Files:**
- Modify: `finstack-wasm/src/api/portfolio/mod.rs`
- Modify: `finstack-wasm/exports/portfolio.js`
- Modify: `finstack-wasm/index.d.ts`

- [ ] **Step 1: Delete `optimize_max_yield` function from WASM bindings**

Remove the `#[wasm_bindgen(js_name = optimizeMaxYield)]` function from `finstack-wasm/src/api/portfolio/mod.rs`.

- [ ] **Step 2: Remove from JS exports**

In `finstack-wasm/exports/portfolio.js`, remove:
```js
optimizeMaxYield: wasm.optimizeMaxYield,
```

- [ ] **Step 3: Remove from TypeScript types**

In `finstack-wasm/index.d.ts`, remove the `optimizeMaxYield` function declaration.

- [ ] **Step 4: Compile WASM**

Run: `cargo build -p finstack-wasm --target wasm32-unknown-unknown`
Expected: Compiles.

- [ ] **Step 5: Commit**

```bash
git add finstack-wasm/src/api/portfolio/mod.rs finstack-wasm/exports/portfolio.js finstack-wasm/index.d.ts
git commit -m "refactor(wasm): remove optimizeMaxYield binding"
```

---

### Task 12: Update Python Stubs and `__init__.py`

Remove `optimize_max_yield` from the public Python API surface.

**Files:**
- Modify: `finstack-py/finstack/portfolio/__init__.py`
- Modify: `finstack-py/finstack/portfolio/__init__.pyi`

- [ ] **Step 1: Update `__init__.py`**

Remove `optimize_max_yield` from the import and `__all__` list.

- [ ] **Step 2: Update `__init__.pyi`**

Remove the `optimize_max_yield` function signature and docstring. Update the `optimize_portfolio` docstring to show the new JSON spec format with `AttributeValue`, `MetricBound`, `AttributeIndicator`, etc.

- [ ] **Step 3: Commit**

```bash
git add finstack-py/finstack/portfolio/__init__.py finstack-py/finstack/portfolio/__init__.pyi
git commit -m "refactor(python): update stubs and exports for optimization API changes"
```

---

### Task 13: Rewrite Portfolio Optimization Notebook

Rewrite the example notebook to demonstrate the new generic optimization API.

**Files:**
- Modify: `finstack-py/examples/notebooks/05_portfolio_and_scenarios/portfolio_optimization.ipynb`

- [ ] **Step 1: Rewrite notebook cells**

The notebook should demonstrate:

1. **Basic setup** — portfolio with positions carrying both text and numeric attributes:
   ```python
   "attributes": {
       "rating": "CCC",
       "sector": "Energy",
       "esg_score": 72.5
   }
   ```

2. **Maximize YTM** — objective using `ValueWeightedAverage`:
   ```python
   "objective": {
       "Maximize": {
           "ValueWeightedAverage": {
               "metric": {"Metric": "ytm"},
               "filter": null
           }
       }
   }
   ```

3. **Exposure limit** — CCC limit via `MetricBound`:
   ```python
   {
       "MetricBound": {
           "label": "ccc_limit",
           "metric": {
               "WeightedSum": {
                   "metric": {
                       "AttributeIndicator": {
                           "key": "rating", "op": "Eq",
                           "value": "CCC"
                       }
                   },
                   "filter": null
               }
           },
           "op": "Le",
           "rhs": 0.10
       }
   }
   ```

4. **Duration band** — two `MetricBound` constraints (Ge and Le).

5. **Numeric attribute constraint** — ESG score example:
   ```python
   {
       "MetricBound": {
           "label": "min_esg",
           "metric": {
               "ValueWeightedAverage": {
                   "metric": {"Attribute": "esg_score"},
                   "filter": null
               }
           },
           "op": "Ge",
           "rhs": 60.0
       }
   }
   ```

6. **Filtered aggregation** — sector-scoped DV01.

7. **Cross-constraint** — CCC + Energy combined using `And` filter.

- [ ] **Step 2: Run the notebook**

Run: `uv run python finstack-py/examples/notebooks/run_all_notebooks.py --filter portfolio_optimization`
Expected: Notebook runs without errors.

- [ ] **Step 3: Commit**

```bash
git add finstack-py/examples/notebooks/05_portfolio_and_scenarios/portfolio_optimization.ipynb
git commit -m "docs(examples): rewrite optimization notebook for configurable constraint API"
```

---

### Task 14: Final Cross-Crate Verification

Full build and test across all crates.

- [ ] **Step 1: Cargo test all**

Run: `cargo test --workspace`
Expected: All tests pass across all crates.

- [ ] **Step 2: Clippy all**

Run: `cargo clippy --workspace -- -D warnings`
Expected: No warnings.

- [ ] **Step 3: Format all**

Run: `make fmt`
Expected: Clean.

- [ ] **Step 4: Python tests**

Run: `uv run pytest finstack-py/tests/ -v`
Expected: All Python tests pass.

- [ ] **Step 5: Fix any remaining issues and commit**

```bash
git add -A
git commit -m "chore: final verification pass for configurable optimization refactor"
```
