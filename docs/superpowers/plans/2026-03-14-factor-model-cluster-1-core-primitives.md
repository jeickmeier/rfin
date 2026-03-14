# Cluster 1: Core Factor Primitives — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Define the foundational types for the factor model — FactorId, FactorType, MarketDependency, FactorDefinition, MarketMapping, FactorCovarianceMatrix — all in `finstack/core`. Also move `Attributes` from `valuations` to `core` to resolve the crate dependency issue.

**Architecture:** All types live in a new `finstack/core/src/factor_model/` module. The `Attributes` struct moves from `valuations/src/instruments/common/traits.rs` to `core/src/types/attributes.rs` and is re-exported from valuations for backwards compatibility. All types derive `Serialize + Deserialize` (except `MarketMapping::Custom` which is `#[serde(skip)]`).

**Tech Stack:** Rust, serde, BTreeMap/BTreeSet for deterministic ordering

**Spec Reference:** `docs/superpowers/specs/2026-03-14-statistical-risk-factor-model-design.md` — Section 1

---

## Task 1: Move `Attributes` from `valuations` to `core`

**Files:**

- Create: `finstack/core/src/types/attributes.rs`
- Modify: `finstack/core/src/types/mod.rs` — add `pub mod attributes;` and re-export
- Modify: `finstack/core/src/lib.rs` — ensure `types` is already exported (it is)
- Modify: `finstack/valuations/src/instruments/common/traits.rs:193-204` — remove `Attributes` struct, replace with re-export from core
- Modify: `finstack/valuations/Cargo.toml` — ensure `finstack-core` dependency exists (it does)
- Test: `finstack/core/tests/types/attributes_test.rs` (or inline `#[cfg(test)]`)

**Context:** `Attributes` is currently defined at `finstack/valuations/src/instruments/common/traits.rs:193-204`. It has two fields: `tags: BTreeSet<String>` and `meta: BTreeMap<String, String>`. It derives `Debug, Clone, Default, Serialize, Deserialize` with `#[serde(deny_unknown_fields)]`. It has builder methods `with_tag()`, `with_tags()`, `with_meta()`, `has_tag()`, `get_meta()`.

- [ ] **Step 1: Write the failing test for Attributes in core**

Create `finstack/core/src/types/attributes.rs` with just a test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attributes_default_is_empty() {
        let attrs = Attributes::default();
        assert!(attrs.tags.is_empty());
        assert!(attrs.meta.is_empty());
    }

    #[test]
    fn test_attributes_builder_methods() {
        let attrs = Attributes::default()
            .with_tag("energy")
            .with_meta("region", "NA")
            .with_meta("rating", "CCC");

        assert!(attrs.has_tag("energy"));
        assert!(!attrs.has_tag("financials"));
        assert_eq!(attrs.get_meta("region"), Some("NA"));
        assert_eq!(attrs.get_meta("rating"), Some("CCC"));
        assert_eq!(attrs.get_meta("nonexistent"), None);
    }

    #[test]
    fn test_attributes_with_tags_batch() {
        let attrs = Attributes::default()
            .with_tags(["a", "b", "c"]);
        assert!(attrs.has_tag("a"));
        assert!(attrs.has_tag("b"));
        assert!(attrs.has_tag("c"));
    }

    #[test]
    fn test_attributes_serde_roundtrip() {
        let attrs = Attributes::default()
            .with_tag("energy")
            .with_meta("region", "NA");

        let json = serde_json::to_string(&attrs).unwrap();
        let deserialized: Attributes = serde_json::from_str(&json).unwrap();
        assert_eq!(attrs.tags, deserialized.tags);
        assert_eq!(attrs.meta, deserialized.meta);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p finstack-core types::attributes::tests --no-default-features`
Expected: FAIL — `Attributes` struct not yet defined

- [ ] **Step 3: Implement Attributes in core**

Add the struct definition above the test module in `finstack/core/src/types/attributes.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attributes {
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub tags: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub meta: BTreeMap<String, String>,
}

impl Attributes {
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.insert(tag.into());
        self
    }

    pub fn with_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for tag in tags {
            self.tags.insert(tag.into());
        }
        self
    }

    pub fn with_meta(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.meta.insert(key.into(), value.into());
        self
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(tag)
    }

    pub fn get_meta(&self, key: &str) -> Option<&str> {
        self.meta.get(key).map(|s| s.as_str())
    }
}
```

- [ ] **Step 4: Register the module in `finstack/core/src/types/mod.rs`**

Add `pub mod attributes;` and `pub use attributes::Attributes;` to the types module.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p finstack-core types::attributes --no-default-features`
Expected: 4 tests PASS

- [ ] **Step 6: Update valuations to re-export from core**

In `finstack/valuations/src/instruments/common/traits.rs`:
- Remove the `Attributes` struct definition (lines 193-204) and its impl block
- Add: `pub use finstack_core::types::Attributes;`

Ensure all existing `use` paths in valuations still compile. The re-export preserves backwards compatibility.

- [ ] **Step 7: Run full workspace build to verify nothing broke**

Run: `cargo build --workspace`
Expected: SUCCESS — all crates compile with `Attributes` now in core

- [ ] **Step 8: Run full workspace tests**

Run: `cargo test --workspace`
Expected: All existing tests pass

- [ ] **Step 9: Commit**

```bash
git add finstack/core/src/types/attributes.rs finstack/core/src/types/mod.rs \
       finstack/valuations/src/instruments/common/traits.rs
git commit -m "refactor: move Attributes from valuations to core for factor model"
```

---

## Task 2: Create `FactorId` and `FactorType`

**Files:**

- Create: `finstack/core/src/factor_model/mod.rs`
- Create: `finstack/core/src/factor_model/types.rs`
- Modify: `finstack/core/src/lib.rs` — add `pub mod factor_model;`

- [ ] **Step 1: Write failing tests**

Create `finstack/core/src/factor_model/types.rs` with tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factor_id_from_string() {
        let id = FactorId::new("USD-Rates");
        assert_eq!(id.as_str(), "USD-Rates");
    }

    #[test]
    fn test_factor_id_equality() {
        let a = FactorId::new("USD-Rates");
        let b = FactorId::new("USD-Rates");
        assert_eq!(a, b);
    }

    #[test]
    fn test_factor_id_display() {
        let id = FactorId::new("NA-Energy-CCC");
        assert_eq!(format!("{id}"), "NA-Energy-CCC");
    }

    #[test]
    fn test_factor_id_serde_roundtrip() {
        let id = FactorId::new("USD-Rates");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"USD-Rates\"");
        let back: FactorId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn test_factor_type_serde() {
        let ft = FactorType::Credit;
        let json = serde_json::to_string(&ft).unwrap();
        let back: FactorType = serde_json::from_str(&json).unwrap();
        assert_eq!(ft, back);
    }

    #[test]
    fn test_factor_type_custom() {
        let ft = FactorType::Custom("Weather".into());
        let json = serde_json::to_string(&ft).unwrap();
        let back: FactorType = serde_json::from_str(&json).unwrap();
        assert_eq!(ft, back);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p finstack-core factor_model::types --no-default-features`
Expected: FAIL — module doesn't exist

- [ ] **Step 3: Implement FactorId and FactorType**

Add above the tests in `finstack/core/src/factor_model/types.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for a risk factor.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FactorId(String);

impl FactorId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FactorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Broad classification of a risk factor.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

- [ ] **Step 4: Create `finstack/core/src/factor_model/mod.rs`**

```rust
mod types;

pub use types::{FactorId, FactorType};
```

- [ ] **Step 5: Register the module in `finstack/core/src/lib.rs`**

Add `pub mod factor_model;` to the top-level module list.

- [ ] **Step 6: Run tests**

Run: `cargo test -p finstack-core factor_model::types --no-default-features`
Expected: 6 tests PASS

- [ ] **Step 7: Commit**

```bash
git add finstack/core/src/factor_model/ finstack/core/src/lib.rs
git commit -m "feat(factor-model): add FactorId and FactorType primitives"
```

---

## Task 3: Create `MarketDependency` and `CurveType` with decompose utility

**Files:**

- Create: `finstack/core/src/factor_model/dependency.rs`
- Modify: `finstack/core/src/factor_model/mod.rs` — add module + re-exports

**Context:** The existing `MarketDependencies` struct is at `finstack/valuations/src/instruments/common/dependencies.rs:28-41`. It has fields: `curves: InstrumentCurves`, `spot_ids: Vec<String>`, `vol_surface_ids: Vec<String>`, `fx_pairs: Vec<FxPair>`, `series_ids: Vec<String>`. The `decompose()` function will live in valuations (since it depends on valuations types), but `MarketDependency` and `CurveType` enums live in core.

- [ ] **Step 1: Write failing tests**

Create `finstack/core/src/factor_model/dependency.rs` with tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::id::CurveId;

    #[test]
    fn test_market_dependency_curve() {
        let dep = MarketDependency::Curve {
            id: CurveId::new("USD-OIS"),
            curve_type: CurveType::Discount,
        };
        if let MarketDependency::Curve { id, curve_type } = &dep {
            assert_eq!(id.as_ref(), "USD-OIS");
            assert_eq!(*curve_type, CurveType::Discount);
        } else {
            panic!("expected Curve variant");
        }
    }

    #[test]
    fn test_market_dependency_credit_curve() {
        let dep = MarketDependency::CreditCurve {
            id: CurveId::new("ACME-HAZARD"),
        };
        if let MarketDependency::CreditCurve { id } = &dep {
            assert_eq!(id.as_ref(), "ACME-HAZARD");
        } else {
            panic!("expected CreditCurve variant");
        }
    }

    #[test]
    fn test_market_dependency_serde_roundtrip() {
        let dep = MarketDependency::Spot {
            id: "AAPL".to_string(),
        };
        let json = serde_json::to_string(&dep).unwrap();
        let back: MarketDependency = serde_json::from_str(&json).unwrap();
        assert_eq!(dep, back);
    }

    #[test]
    fn test_curve_type_all_variants_serde() {
        for ct in [
            CurveType::Discount,
            CurveType::Forward,
            CurveType::Hazard,
            CurveType::Inflation,
            CurveType::BaseCorrelation,
        ] {
            let json = serde_json::to_string(&ct).unwrap();
            let back: CurveType = serde_json::from_str(&json).unwrap();
            assert_eq!(ct, back);
        }
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p finstack-core factor_model::dependency --no-default-features`
Expected: FAIL — module doesn't exist

- [ ] **Step 3: Implement MarketDependency and CurveType**

Add above the tests in `finstack/core/src/factor_model/dependency.rs`:

```rust
use crate::types::id::CurveId;
use crate::currency::Currency;
use serde::{Deserialize, Serialize};

/// Classification of a curve's role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CurveType {
    Discount,
    Forward,
    Hazard,
    Inflation,
    BaseCorrelation,
}

/// A single market data dependency extracted from an instrument.
///
/// The existing `MarketDependencies` struct aggregates all dependencies;
/// this enum represents one individual dependency for factor matching.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
```

- [ ] **Step 4: Register module in `finstack/core/src/factor_model/mod.rs`**

Add `mod dependency;` and re-exports for `MarketDependency` and `CurveType`.

- [ ] **Step 5: Run tests**

Run: `cargo test -p finstack-core factor_model::dependency --no-default-features`
Expected: 4 tests PASS

- [ ] **Step 6: Commit**

```bash
git add finstack/core/src/factor_model/dependency.rs finstack/core/src/factor_model/mod.rs
git commit -m "feat(factor-model): add MarketDependency and CurveType enums"
```

---

## Task 4: Create `MarketMapping` and `FactorDefinition`

**Files:**

- Create: `finstack/core/src/factor_model/definition.rs`
- Modify: `finstack/core/src/factor_model/mod.rs` — add module + re-exports

**Context:** `BumpSpec` is at `finstack/core/src/market_data/bumps.rs:96-108`. `BumpUnits` is at the same file line 68-81. `CurveId` is `Id<CurveTag>` from `finstack/core/src/types/id.rs:415`. These are all in core, so no cross-crate issues.

- [ ] **Step 1: Write failing tests**

Create `finstack/core/src/factor_model/definition.rs` with tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::id::CurveId;
    use crate::market_data::bumps::BumpUnits;

    #[test]
    fn test_factor_definition_construction() {
        let def = FactorDefinition {
            id: FactorId::new("USD-Rates"),
            factor_type: FactorType::Rates,
            market_mapping: MarketMapping::CurveParallel {
                curve_ids: vec![CurveId::new("USD-OIS")],
                units: BumpUnits::RateBp,
            },
            description: Some("US dollar rates factor".into()),
        };
        assert_eq!(def.id.as_str(), "USD-Rates");
        assert_eq!(def.factor_type, FactorType::Rates);
    }

    #[test]
    fn test_market_mapping_curve_parallel_serde() {
        let mapping = MarketMapping::CurveParallel {
            curve_ids: vec![CurveId::new("USD-OIS"), CurveId::new("USD-3M")],
            units: BumpUnits::RateBp,
        };
        let json = serde_json::to_string(&mapping).unwrap();
        let back: MarketMapping = serde_json::from_str(&json).unwrap();
        // Compare via re-serialization (PartialEq may not be derived for BumpUnits)
        assert_eq!(json, serde_json::to_string(&back).unwrap());
    }

    #[test]
    fn test_market_mapping_equity_spot() {
        let mapping = MarketMapping::EquitySpot {
            tickers: vec!["AAPL".into(), "MSFT".into()],
        };
        let json = serde_json::to_string(&mapping).unwrap();
        assert!(json.contains("AAPL"));
    }

    #[test]
    fn test_market_mapping_custom_not_serializable() {
        let mapping = MarketMapping::Custom(std::sync::Arc::new(|_| vec![]));
        // Custom variant should serialize to null/skip
        let json = serde_json::to_string(&mapping);
        // serde(skip) causes serialization to fail for the variant
        // This is expected — Custom is only used via builder
        assert!(json.is_err() || json.unwrap().contains("null"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p finstack-core factor_model::definition --no-default-features`
Expected: FAIL — module doesn't exist

- [ ] **Step 3: Implement MarketMapping and FactorDefinition**

```rust
use crate::currency::Currency;
use crate::market_data::bumps::{BumpSpec, BumpUnits};
use crate::types::id::CurveId;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::types::{FactorId, FactorType};

/// How a factor movement translates to market data perturbations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketMapping {
    /// Parallel shift to a set of curves
    CurveParallel { curve_ids: Vec<CurveId>, units: BumpUnits },
    /// Bucketed shifts (key-rate style) — (tenor_years, weight) pairs
    CurveBucketed { curve_id: CurveId, tenor_weights: Vec<(f64, f64)> },
    /// Equity spot move (percentage)
    EquitySpot { tickers: Vec<String> },
    /// FX rate move
    FxRate { pair: (Currency, Currency) },
    /// Vol surface shift
    VolShift { surface_ids: Vec<String>, units: BumpUnits },
    /// User-defined: factor move → BumpSpecs. Not serializable.
    #[serde(skip)]
    Custom(Arc<dyn Fn(f64) -> Vec<BumpSpec> + Send + Sync>),
}

/// Complete definition of a risk factor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorDefinition {
    pub id: FactorId,
    pub factor_type: FactorType,
    pub market_mapping: MarketMapping,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
```

Note: `Debug` for `MarketMapping::Custom` requires a manual impl — the `Arc<dyn Fn>` doesn't impl `Debug`. You'll need to manually implement `Debug` for `MarketMapping` or use a wrapper. The simplest approach: implement `Debug` manually, printing `"Custom(<closure>)"` for the Custom variant.

- [ ] **Step 4: Register module in mod.rs and re-export**

Add `mod definition;` and `pub use definition::{FactorDefinition, MarketMapping};` to `mod.rs`.

- [ ] **Step 5: Run tests**

Run: `cargo test -p finstack-core factor_model::definition --no-default-features`
Expected: 4 tests PASS (adjust the Custom serde test expectation based on actual behavior)

- [ ] **Step 6: Commit**

```bash
git add finstack/core/src/factor_model/definition.rs finstack/core/src/factor_model/mod.rs
git commit -m "feat(factor-model): add MarketMapping and FactorDefinition"
```

---

## Task 5: Create `FactorCovarianceMatrix`

**Files:**

- Create: `finstack/core/src/factor_model/covariance.rs`
- Modify: `finstack/core/src/factor_model/mod.rs` — add module + re-exports

- [ ] **Step 1: Write failing tests**

Create `finstack/core/src/factor_model/covariance.rs` with tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn two_factor_ids() -> Vec<FactorId> {
        vec![FactorId::new("Rates"), FactorId::new("Credit")]
    }

    #[test]
    fn test_valid_2x2_covariance() {
        let ids = two_factor_ids();
        // [0.04, 0.01]
        // [0.01, 0.09]
        let data = vec![0.04, 0.01, 0.01, 0.09];
        let cov = FactorCovarianceMatrix::new(ids, data).unwrap();
        assert_eq!(cov.n_factors(), 2);
    }

    #[test]
    fn test_variance_accessor() {
        let ids = two_factor_ids();
        let data = vec![0.04, 0.01, 0.01, 0.09];
        let cov = FactorCovarianceMatrix::new(ids, data).unwrap();
        assert!((cov.variance(&FactorId::new("Rates")) - 0.04).abs() < 1e-12);
        assert!((cov.variance(&FactorId::new("Credit")) - 0.09).abs() < 1e-12);
    }

    #[test]
    fn test_covariance_accessor() {
        let ids = two_factor_ids();
        let data = vec![0.04, 0.01, 0.01, 0.09];
        let cov = FactorCovarianceMatrix::new(ids, data).unwrap();
        let c = cov.covariance(&FactorId::new("Rates"), &FactorId::new("Credit"));
        assert!((c - 0.01).abs() < 1e-12);
    }

    #[test]
    fn test_correlation_accessor() {
        let ids = two_factor_ids();
        let data = vec![0.04, 0.01, 0.01, 0.09];
        let cov = FactorCovarianceMatrix::new(ids, data).unwrap();
        let rho = cov.correlation(&FactorId::new("Rates"), &FactorId::new("Credit"));
        // rho = 0.01 / sqrt(0.04 * 0.09) = 0.01 / 0.06 ≈ 0.1667
        assert!((rho - 1.0 / 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_wrong_dimensions_rejected() {
        let ids = two_factor_ids();
        let data = vec![0.04, 0.01, 0.01]; // 3 elements, need 4
        assert!(FactorCovarianceMatrix::new(ids, data).is_err());
    }

    #[test]
    fn test_asymmetric_matrix_rejected() {
        let ids = two_factor_ids();
        let data = vec![0.04, 0.02, 0.01, 0.09]; // not symmetric
        assert!(FactorCovarianceMatrix::new(ids, data).is_err());
    }

    #[test]
    fn test_not_psd_rejected() {
        let ids = two_factor_ids();
        // Not PSD: eigenvalues are negative
        let data = vec![1.0, 3.0, 3.0, 1.0];
        assert!(FactorCovarianceMatrix::new(ids, data).is_err());
    }

    #[test]
    fn test_new_unchecked_skips_validation() {
        let ids = two_factor_ids();
        // Not PSD but unchecked allows it
        let data = vec![1.0, 3.0, 3.0, 1.0];
        let cov = FactorCovarianceMatrix::new_unchecked(ids, data);
        assert_eq!(cov.n_factors(), 2);
    }

    #[test]
    fn test_single_factor() {
        let ids = vec![FactorId::new("Only")];
        let data = vec![0.25];
        let cov = FactorCovarianceMatrix::new(ids, data).unwrap();
        assert!((cov.variance(&FactorId::new("Only")) - 0.25).abs() < 1e-12);
    }

    #[test]
    fn test_as_slice() {
        let ids = two_factor_ids();
        let data = vec![0.04, 0.01, 0.01, 0.09];
        let cov = FactorCovarianceMatrix::new(ids, data.clone()).unwrap();
        assert_eq!(cov.as_slice(), &data[..]);
    }

    #[test]
    fn test_serde_roundtrip() {
        let ids = two_factor_ids();
        let data = vec![0.04, 0.01, 0.01, 0.09];
        let cov = FactorCovarianceMatrix::new(ids, data).unwrap();
        let json = serde_json::to_string(&cov).unwrap();
        let back: FactorCovarianceMatrix = serde_json::from_str(&json).unwrap();
        assert_eq!(cov.as_slice(), back.as_slice());
        assert_eq!(cov.n_factors(), back.n_factors());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p finstack-core factor_model::covariance --no-default-features`
Expected: FAIL

- [ ] **Step 3: Implement FactorCovarianceMatrix**

```rust
use super::types::FactorId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// User-supplied factor covariance matrix with flat row-major storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorCovarianceMatrix {
    factor_ids: Vec<FactorId>,
    n: usize,
    data: Vec<f64>,
    #[serde(skip)]
    index: HashMap<FactorId, usize>,
}

impl FactorCovarianceMatrix {
    /// Construct with full validation (symmetry + PSD check).
    pub fn new(factor_ids: Vec<FactorId>, data: Vec<f64>) -> crate::Result<Self> {
        let n = factor_ids.len();
        if data.len() != n * n {
            return Err(crate::FinstackError::invalid_input(format!(
                "Covariance data length {} does not match {}x{} = {}",
                data.len(), n, n, n * n
            )));
        }

        // Symmetry check
        let eps = 1e-12;
        for i in 0..n {
            for j in (i + 1)..n {
                let diff = (data[i * n + j] - data[j * n + i]).abs();
                if diff > eps {
                    return Err(crate::FinstackError::invalid_input(format!(
                        "Covariance matrix not symmetric: [{i},{j}]={} vs [{j},{i}]={}",
                        data[i * n + j],
                        data[j * n + i]
                    )));
                }
            }
        }

        // PSD check via Cholesky decomposition
        if !Self::is_psd(&data, n) {
            return Err(crate::FinstackError::invalid_input(
                "Covariance matrix is not positive semi-definite".to_string(),
            ));
        }

        let index = factor_ids
            .iter()
            .enumerate()
            .map(|(i, id)| (id.clone(), i))
            .collect();

        Ok(Self { factor_ids, n, data, index })
    }

    /// Construct without validation (for trusted inputs or large matrices).
    pub fn new_unchecked(factor_ids: Vec<FactorId>, data: Vec<f64>) -> Self {
        let n = factor_ids.len();
        let index = factor_ids
            .iter()
            .enumerate()
            .map(|(i, id)| (id.clone(), i))
            .collect();
        Self { factor_ids, n, data, index }
    }

    pub fn n_factors(&self) -> usize {
        self.n
    }

    pub fn factor_ids(&self) -> &[FactorId] {
        &self.factor_ids
    }

    pub fn as_slice(&self) -> &[f64] {
        &self.data
    }

    pub fn variance(&self, factor: &FactorId) -> f64 {
        let i = self.index[factor];
        self.data[i * self.n + i]
    }

    pub fn covariance(&self, f1: &FactorId, f2: &FactorId) -> f64 {
        let i = self.index[f1];
        let j = self.index[f2];
        self.data[i * self.n + j]
    }

    pub fn correlation(&self, f1: &FactorId, f2: &FactorId) -> f64 {
        let cov = self.covariance(f1, f2);
        let v1 = self.variance(f1);
        let v2 = self.variance(f2);
        if v1 <= 0.0 || v2 <= 0.0 {
            return 0.0;
        }
        cov / (v1.sqrt() * v2.sqrt())
    }

    fn is_psd(data: &[f64], n: usize) -> bool {
        // Cholesky decomposition: if it succeeds, matrix is PSD
        let mut l = vec![0.0f64; n * n];
        for i in 0..n {
            for j in 0..=i {
                let mut sum = 0.0;
                for k in 0..j {
                    sum += l[i * n + k] * l[j * n + k];
                }
                if i == j {
                    let diag = data[i * n + i] - sum;
                    if diag < -1e-12 {
                        return false;
                    }
                    l[i * n + j] = diag.max(0.0).sqrt();
                } else {
                    let denom = l[j * n + j];
                    if denom.abs() < 1e-15 {
                        l[i * n + j] = 0.0;
                    } else {
                        l[i * n + j] = (data[i * n + j] - sum) / denom;
                    }
                }
            }
        }
        true
    }
}

// Rebuild index on deserialization
impl<'de> FactorCovarianceMatrix {
    fn rebuild_index(&mut self) {
        self.index = self.factor_ids
            .iter()
            .enumerate()
            .map(|(i, id)| (id.clone(), i))
            .collect();
    }
}
```

Note: You'll need a custom `Deserialize` impl or a `#[serde(deserialize_with)]` to rebuild the `index` HashMap after deserialization. The simplest approach is implementing `Deserialize` manually or using a `PostDeserialize` pattern.

- [ ] **Step 4: Register in mod.rs**

Add `mod covariance;` and `pub use covariance::FactorCovarianceMatrix;`.

- [ ] **Step 5: Run tests**

Run: `cargo test -p finstack-core factor_model::covariance --no-default-features`
Expected: 11 tests PASS

- [ ] **Step 6: Commit**

```bash
git add finstack/core/src/factor_model/covariance.rs finstack/core/src/factor_model/mod.rs
git commit -m "feat(factor-model): add FactorCovarianceMatrix with PSD validation"
```

---

## Task 6: Create `FactorModelError` and `UnmatchedPolicy`

**Files:**

- Create: `finstack/core/src/factor_model/error.rs`
- Modify: `finstack/core/src/factor_model/mod.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_missing_factor() {
        let err = FactorModelError::MissingFactor {
            factor_id: FactorId::new("USD-Rates"),
        };
        let msg = format!("{err}");
        assert!(msg.contains("USD-Rates"));
    }

    #[test]
    fn test_unmatched_policy_default() {
        assert_eq!(UnmatchedPolicy::default(), UnmatchedPolicy::Residual);
    }

    #[test]
    fn test_unmatched_policy_serde() {
        let policy = UnmatchedPolicy::Strict;
        let json = serde_json::to_string(&policy).unwrap();
        let back: UnmatchedPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(policy, back);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

- [ ] **Step 3: Implement FactorModelError and UnmatchedPolicy**

```rust
use super::dependency::MarketDependency;
use super::types::FactorId;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug)]
pub enum FactorModelError {
    UnmatchedDependency { position_id: String, dependency: MarketDependency },
    MissingFactor { factor_id: FactorId },
    InvalidCovariance { reason: String },
    RepricingFailed { position_id: String, factor_id: FactorId, source: Box<dyn std::error::Error + Send + Sync> },
    AmbiguousMatch { position_id: String, candidates: Vec<FactorId> },
    InfeasibleConstraints { reason: String },
}

impl fmt::Display for FactorModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnmatchedDependency { position_id, dependency } => {
                write!(f, "No factor matched dependency {dependency:?} for position '{position_id}'")
            }
            Self::MissingFactor { factor_id } => {
                write!(f, "Factor '{factor_id}' referenced but not found in covariance matrix")
            }
            Self::InvalidCovariance { reason } => {
                write!(f, "Invalid covariance matrix: {reason}")
            }
            Self::RepricingFailed { position_id, factor_id, source } => {
                write!(f, "Repricing failed for position '{position_id}' under factor '{factor_id}': {source}")
            }
            Self::AmbiguousMatch { position_id, candidates } => {
                write!(f, "Ambiguous factor match for position '{position_id}': {candidates:?}")
            }
            Self::InfeasibleConstraints { reason } => {
                write!(f, "Factor-constrained optimization infeasible: {reason}")
            }
        }
    }
}

impl std::error::Error for FactorModelError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum UnmatchedPolicy {
    Strict,
    #[default]
    Residual,
    Warn,
}
```

- [ ] **Step 4: Register in mod.rs**

- [ ] **Step 5: Run tests**

Run: `cargo test -p finstack-core factor_model::error --no-default-features`
Expected: 3 tests PASS

- [ ] **Step 6: Run full workspace build**

Run: `cargo build --workspace`
Expected: SUCCESS

- [ ] **Step 7: Commit**

```bash
git add finstack/core/src/factor_model/error.rs finstack/core/src/factor_model/mod.rs
git commit -m "feat(factor-model): add FactorModelError and UnmatchedPolicy"
```

---

## Final mod.rs for Cluster 1

After all tasks, `finstack/core/src/factor_model/mod.rs` should look like:

```rust
mod covariance;
mod definition;
mod dependency;
mod error;
mod types;

pub use covariance::FactorCovarianceMatrix;
pub use definition::{FactorDefinition, MarketMapping};
pub use dependency::{CurveType, MarketDependency};
pub use error::{FactorModelError, UnmatchedPolicy};
pub use types::{FactorId, FactorType};
```
