# Cluster 2: Factor Matching Engine — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the three composable factor matching strategies (MappingTable, Cascade, Hierarchical) and the `FactorMatcher` trait, plus the config types for declarative matching. Also implement the `decompose()` bridge function in valuations that converts `MarketDependencies` → `Vec<MarketDependency>`.

**Architecture:** The `FactorMatcher` trait and all three built-in matchers live in `finstack/core/src/factor_model/matching/`. The `decompose()` bridge lives in `finstack/valuations` since it depends on valuations' `MarketDependencies` type. Config types (`MatchingConfig`, `MappingRule`, `DependencyFilter`, `AttributeFilter`) live alongside the matchers.

**Tech Stack:** Rust, serde

**Spec Reference:** `docs/superpowers/specs/2026-03-14-statistical-risk-factor-model-design.md` — Section 1 (Factor Matching) + Section 4 (Config types)

**Depends on:** Cluster 1 (FactorId, FactorType, MarketDependency, CurveType, Attributes)

---

## Task 1: Create `FactorMatcher` trait, `AttributeFilter`, and `DependencyFilter`

**Files:**

- Create: `finstack/core/src/factor_model/matching/mod.rs`
- Create: `finstack/core/src/factor_model/matching/filter.rs`
- Create: `finstack/core/src/factor_model/matching/traits.rs`
- Modify: `finstack/core/src/factor_model/mod.rs` — add `pub mod matching;`

- [ ] **Step 1: Write failing tests for filters**

Create `finstack/core/src/factor_model/matching/filter.rs` with tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Attributes;

    #[test]
    fn test_attribute_filter_empty_matches_all() {
        let filter = AttributeFilter::default();
        let attrs = Attributes::default().with_tag("energy").with_meta("region", "NA");
        assert!(filter.matches(&attrs));
    }

    #[test]
    fn test_attribute_filter_tag_match() {
        let filter = AttributeFilter {
            tags: vec!["energy".into()],
            meta: vec![],
        };
        let matching = Attributes::default().with_tag("energy");
        let not_matching = Attributes::default().with_tag("financials");
        assert!(filter.matches(&matching));
        assert!(!filter.matches(&not_matching));
    }

    #[test]
    fn test_attribute_filter_meta_match() {
        let filter = AttributeFilter {
            tags: vec![],
            meta: vec![("region".into(), "NA".into())],
        };
        let matching = Attributes::default().with_meta("region", "NA");
        let not_matching = Attributes::default().with_meta("region", "EU");
        assert!(filter.matches(&matching));
        assert!(!filter.matches(&not_matching));
    }

    #[test]
    fn test_attribute_filter_conjunction() {
        let filter = AttributeFilter {
            tags: vec!["energy".into()],
            meta: vec![("region".into(), "NA".into()), ("rating".into(), "CCC".into())],
        };
        let full_match = Attributes::default()
            .with_tag("energy")
            .with_meta("region", "NA")
            .with_meta("rating", "CCC");
        let partial = Attributes::default()
            .with_tag("energy")
            .with_meta("region", "NA");
        assert!(filter.matches(&full_match));
        assert!(!filter.matches(&partial)); // missing rating
    }

    #[test]
    fn test_dependency_filter_by_type() {
        let filter = DependencyFilter {
            dependency_type: Some(CurveType::Hazard),
            id: None,
        };
        let credit = MarketDependency::CreditCurve {
            id: CurveId::new("ACME-HAZARD"),
        };
        let rate = MarketDependency::Curve {
            id: CurveId::new("USD-OIS"),
            curve_type: CurveType::Discount,
        };
        assert!(filter.matches(&credit));
        assert!(!filter.matches(&rate));
    }

    #[test]
    fn test_dependency_filter_by_id() {
        let filter = DependencyFilter {
            dependency_type: None,
            id: Some("USD-OIS".into()),
        };
        let matching = MarketDependency::Curve {
            id: CurveId::new("USD-OIS"),
            curve_type: CurveType::Discount,
        };
        let not_matching = MarketDependency::Curve {
            id: CurveId::new("EUR-OIS"),
            curve_type: CurveType::Discount,
        };
        assert!(filter.matches(&matching));
        assert!(!filter.matches(&not_matching));
    }

    #[test]
    fn test_dependency_filter_empty_matches_all() {
        let filter = DependencyFilter::default();
        let dep = MarketDependency::Spot { id: "AAPL".into() };
        assert!(filter.matches(&dep));
    }

    #[test]
    fn test_filters_serde_roundtrip() {
        let af = AttributeFilter {
            tags: vec!["energy".into()],
            meta: vec![("region".into(), "NA".into())],
        };
        let json = serde_json::to_string(&af).unwrap();
        let back: AttributeFilter = serde_json::from_str(&json).unwrap();
        assert_eq!(af, back);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p finstack-core factor_model::matching::filter --no-default-features`
Expected: FAIL

- [ ] **Step 3: Implement AttributeFilter and DependencyFilter**

```rust
use crate::factor_model::dependency::{CurveType, MarketDependency};
use crate::types::Attributes;
use serde::{Deserialize, Serialize};

/// Filters on instrument metadata (tags + key-value meta). All conditions are AND.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AttributeFilter {
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub meta: Vec<(String, String)>,
}

impl AttributeFilter {
    pub fn matches(&self, attrs: &Attributes) -> bool {
        let tags_ok = self.tags.iter().all(|t| attrs.has_tag(t));
        let meta_ok = self.meta.iter().all(|(k, v)| attrs.get_meta(k) == Some(v.as_str()));
        tags_ok && meta_ok
    }
}

/// Filters on a single MarketDependency.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DependencyFilter {
    #[serde(default)]
    pub dependency_type: Option<CurveType>,
    #[serde(default)]
    pub id: Option<String>,
}

impl DependencyFilter {
    pub fn matches(&self, dep: &MarketDependency) -> bool {
        let type_ok = match &self.dependency_type {
            None => true,
            Some(ct) => dep.matches_curve_type(ct),
        };
        let id_ok = match &self.id {
            None => true,
            Some(expected_id) => dep.id_str() == Some(expected_id.as_str()),
        };
        type_ok && id_ok
    }
}
```

This requires adding `matches_curve_type()` and `id_str()` helper methods to `MarketDependency` in `finstack/core/src/factor_model/dependency.rs`:

```rust
impl MarketDependency {
    /// Check if this dependency matches the given CurveType.
    pub fn matches_curve_type(&self, ct: &CurveType) -> bool {
        match (self, ct) {
            (Self::Curve { curve_type, .. }, expected) => curve_type == expected,
            (Self::CreditCurve { .. }, CurveType::Hazard) => true,
            _ => false,
        }
    }

    /// Return the string ID of this dependency, if applicable.
    pub fn id_str(&self) -> Option<&str> {
        match self {
            Self::Curve { id, .. } | Self::CreditCurve { id } => Some(id.as_ref()),
            Self::Spot { id } | Self::VolSurface { id } | Self::Series { id } => Some(id.as_str()),
            Self::FxPair { .. } => None,
        }
    }
}
```

- [ ] **Step 4: Create the FactorMatcher trait**

Create `finstack/core/src/factor_model/matching/traits.rs`:

```rust
use crate::factor_model::dependency::MarketDependency;
use crate::factor_model::types::FactorId;
use crate::types::Attributes;

/// Trait for matching a market dependency + instrument attributes to a factor.
pub trait FactorMatcher: Send + Sync {
    fn match_factor(
        &self,
        dependency: &MarketDependency,
        attributes: &Attributes,
    ) -> Option<FactorId>;
}
```

- [ ] **Step 5: Wire up `matching/mod.rs`**

```rust
mod filter;
mod traits;

pub use filter::{AttributeFilter, DependencyFilter};
pub use traits::FactorMatcher;
```

- [ ] **Step 6: Update `factor_model/mod.rs`**

Add `pub mod matching;` and re-export key types.

- [ ] **Step 7: Run tests**

Run: `cargo test -p finstack-core factor_model::matching --no-default-features`
Expected: 8 tests PASS

- [ ] **Step 8: Commit**

```bash
git add finstack/core/src/factor_model/matching/ finstack/core/src/factor_model/mod.rs \
       finstack/core/src/factor_model/dependency.rs
git commit -m "feat(factor-model): add FactorMatcher trait, AttributeFilter, DependencyFilter"
```

---

## Task 2: Implement `MappingTableMatcher`

**Files:**

- Create: `finstack/core/src/factor_model/matching/mapping_table.rs`
- Modify: `finstack/core/src/factor_model/matching/mod.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::factor_model::dependency::{CurveType, MarketDependency};
    use crate::factor_model::matching::filter::{AttributeFilter, DependencyFilter};
    use crate::factor_model::types::FactorId;
    use crate::types::Attributes;
    use crate::types::id::CurveId;

    #[test]
    fn test_empty_table_matches_nothing() {
        let matcher = MappingTableMatcher::new(vec![]);
        let dep = MarketDependency::Spot { id: "AAPL".into() };
        let attrs = Attributes::default();
        assert_eq!(matcher.match_factor(&dep, &attrs), None);
    }

    #[test]
    fn test_first_match_wins() {
        let matcher = MappingTableMatcher::new(vec![
            MappingRule {
                dependency_filter: DependencyFilter {
                    dependency_type: Some(CurveType::Hazard),
                    id: None,
                },
                attribute_filter: AttributeFilter {
                    tags: vec!["energy".into()],
                    meta: vec![("rating".into(), "CCC".into())],
                },
                factor_id: FactorId::new("NA-Energy-CCC"),
            },
            MappingRule {
                dependency_filter: DependencyFilter {
                    dependency_type: Some(CurveType::Hazard),
                    id: None,
                },
                attribute_filter: AttributeFilter::default(),
                factor_id: FactorId::new("Generic-Credit"),
            },
        ]);

        let dep = MarketDependency::CreditCurve { id: CurveId::new("ACME-HAZARD") };

        // Matches first rule (energy + CCC)
        let attrs1 = Attributes::default().with_tag("energy").with_meta("rating", "CCC");
        assert_eq!(matcher.match_factor(&dep, &attrs1), Some(FactorId::new("NA-Energy-CCC")));

        // Falls through to second rule (generic credit)
        let attrs2 = Attributes::default().with_tag("financials");
        assert_eq!(matcher.match_factor(&dep, &attrs2), Some(FactorId::new("Generic-Credit")));
    }

    #[test]
    fn test_no_match_returns_none() {
        let matcher = MappingTableMatcher::new(vec![
            MappingRule {
                dependency_filter: DependencyFilter {
                    dependency_type: Some(CurveType::Hazard),
                    id: None,
                },
                attribute_filter: AttributeFilter::default(),
                factor_id: FactorId::new("Credit"),
            },
        ]);

        // Equity spot doesn't match hazard filter
        let dep = MarketDependency::Spot { id: "AAPL".into() };
        let attrs = Attributes::default();
        assert_eq!(matcher.match_factor(&dep, &attrs), None);
    }

    #[test]
    fn test_config_serde_roundtrip() {
        let rules = vec![MappingRule {
            dependency_filter: DependencyFilter::default(),
            attribute_filter: AttributeFilter {
                tags: vec!["energy".into()],
                meta: vec![],
            },
            factor_id: FactorId::new("Energy"),
        }];
        let json = serde_json::to_string(&rules).unwrap();
        let back: Vec<MappingRule> = serde_json::from_str(&json).unwrap();
        assert_eq!(rules.len(), back.len());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

- [ ] **Step 3: Implement MappingTableMatcher**

```rust
use super::filter::{AttributeFilter, DependencyFilter};
use super::traits::FactorMatcher;
use crate::factor_model::dependency::MarketDependency;
use crate::factor_model::types::FactorId;
use crate::types::Attributes;
use serde::{Deserialize, Serialize};

/// A single matching rule: dependency filter + attribute filter → factor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MappingRule {
    pub dependency_filter: DependencyFilter,
    pub attribute_filter: AttributeFilter,
    pub factor_id: FactorId,
}

/// Flat lookup table matcher. First matching rule wins.
#[derive(Debug, Clone)]
pub struct MappingTableMatcher {
    rules: Vec<MappingRule>,
}

impl MappingTableMatcher {
    pub fn new(rules: Vec<MappingRule>) -> Self {
        Self { rules }
    }
}

impl FactorMatcher for MappingTableMatcher {
    fn match_factor(
        &self,
        dependency: &MarketDependency,
        attributes: &Attributes,
    ) -> Option<FactorId> {
        self.rules
            .iter()
            .find(|rule| {
                rule.dependency_filter.matches(dependency)
                    && rule.attribute_filter.matches(attributes)
            })
            .map(|rule| rule.factor_id.clone())
    }
}
```

- [ ] **Step 4: Register in matching/mod.rs**

Add `mod mapping_table;` and `pub use mapping_table::{MappingRule, MappingTableMatcher};`.

- [ ] **Step 5: Run tests**

Run: `cargo test -p finstack-core factor_model::matching::mapping_table --no-default-features`
Expected: 4 tests PASS

- [ ] **Step 6: Commit**

```bash
git add finstack/core/src/factor_model/matching/
git commit -m "feat(factor-model): add MappingTableMatcher"
```

---

## Task 3: Implement `CascadeMatcher`

**Files:**

- Create: `finstack/core/src/factor_model/matching/cascade.rs`
- Modify: `finstack/core/src/factor_model/matching/mod.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::factor_model::dependency::{CurveType, MarketDependency};
    use crate::factor_model::matching::filter::{AttributeFilter, DependencyFilter};
    use crate::factor_model::matching::mapping_table::{MappingRule, MappingTableMatcher};
    use crate::factor_model::types::FactorId;
    use crate::types::Attributes;
    use crate::types::id::CurveId;

    #[test]
    fn test_cascade_tries_in_order() {
        // First matcher: exact curve ID
        let exact = MappingTableMatcher::new(vec![MappingRule {
            dependency_filter: DependencyFilter {
                dependency_type: None,
                id: Some("ACME-HAZARD".into()),
            },
            attribute_filter: AttributeFilter::default(),
            factor_id: FactorId::new("ACME-Specific"),
        }]);

        // Second matcher: generic credit fallback
        let fallback = MappingTableMatcher::new(vec![MappingRule {
            dependency_filter: DependencyFilter {
                dependency_type: Some(CurveType::Hazard),
                id: None,
            },
            attribute_filter: AttributeFilter::default(),
            factor_id: FactorId::new("Generic-Credit"),
        }]);

        let cascade = CascadeMatcher::new(vec![Box::new(exact), Box::new(fallback)]);
        let attrs = Attributes::default();

        // Exact match
        let dep1 = MarketDependency::CreditCurve { id: CurveId::new("ACME-HAZARD") };
        assert_eq!(cascade.match_factor(&dep1, &attrs), Some(FactorId::new("ACME-Specific")));

        // Falls through to generic
        let dep2 = MarketDependency::CreditCurve { id: CurveId::new("OTHER-HAZARD") };
        assert_eq!(cascade.match_factor(&dep2, &attrs), Some(FactorId::new("Generic-Credit")));

        // Nothing matches
        let dep3 = MarketDependency::Spot { id: "AAPL".into() };
        assert_eq!(cascade.match_factor(&dep3, &attrs), None);
    }

    #[test]
    fn test_empty_cascade() {
        let cascade = CascadeMatcher::new(vec![]);
        let dep = MarketDependency::Spot { id: "AAPL".into() };
        assert_eq!(cascade.match_factor(&dep, &Attributes::default()), None);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

- [ ] **Step 3: Implement CascadeMatcher**

```rust
use super::traits::FactorMatcher;
use crate::factor_model::dependency::MarketDependency;
use crate::factor_model::types::FactorId;
use crate::types::Attributes;

/// Tries each matcher in order, returns the first match.
pub struct CascadeMatcher {
    matchers: Vec<Box<dyn FactorMatcher>>,
}

impl CascadeMatcher {
    pub fn new(matchers: Vec<Box<dyn FactorMatcher>>) -> Self {
        Self { matchers }
    }
}

impl FactorMatcher for CascadeMatcher {
    fn match_factor(
        &self,
        dependency: &MarketDependency,
        attributes: &Attributes,
    ) -> Option<FactorId> {
        self.matchers
            .iter()
            .find_map(|m| m.match_factor(dependency, attributes))
    }
}
```

- [ ] **Step 4: Register in mod.rs**

- [ ] **Step 5: Run tests**

Run: `cargo test -p finstack-core factor_model::matching::cascade --no-default-features`
Expected: 2 tests PASS

- [ ] **Step 6: Commit**

```bash
git add finstack/core/src/factor_model/matching/
git commit -m "feat(factor-model): add CascadeMatcher"
```

---

## Task 4: Implement `HierarchicalMatcher`

**Files:**

- Create: `finstack/core/src/factor_model/matching/hierarchical.rs`
- Modify: `finstack/core/src/factor_model/matching/mod.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::factor_model::dependency::{CurveType, MarketDependency};
    use crate::factor_model::matching::filter::AttributeFilter;
    use crate::factor_model::types::FactorId;
    use crate::types::Attributes;
    use crate::types::id::CurveId;

    fn credit_tree() -> FactorNode {
        FactorNode {
            factor_id: Some(FactorId::new("Generic-Credit")),
            filter: AttributeFilter::default(), // root matches all
            children: vec![
                FactorNode {
                    factor_id: Some(FactorId::new("NA-Credit")),
                    filter: AttributeFilter {
                        tags: vec![],
                        meta: vec![("region".into(), "NA".into())],
                    },
                    children: vec![
                        FactorNode {
                            factor_id: None, // no factor at Energy level, must go deeper
                            filter: AttributeFilter {
                                tags: vec!["energy".into()],
                                meta: vec![],
                            },
                            children: vec![
                                FactorNode {
                                    factor_id: Some(FactorId::new("NA-Energy-CCC")),
                                    filter: AttributeFilter {
                                        tags: vec![],
                                        meta: vec![("rating".into(), "CCC".into())],
                                    },
                                    children: vec![],
                                },
                                FactorNode {
                                    factor_id: Some(FactorId::new("NA-Energy-IG")),
                                    filter: AttributeFilter {
                                        tags: vec![],
                                        meta: vec![("rating".into(), "IG".into())],
                                    },
                                    children: vec![],
                                },
                            ],
                        },
                    ],
                },
                FactorNode {
                    factor_id: Some(FactorId::new("EU-Credit")),
                    filter: AttributeFilter {
                        tags: vec![],
                        meta: vec![("region".into(), "EU".into())],
                    },
                    children: vec![],
                },
            ],
        }
    }

    #[test]
    fn test_deepest_match_wins() {
        let matcher = HierarchicalMatcher::new(credit_tree());
        let dep = MarketDependency::CreditCurve { id: CurveId::new("X") };

        let attrs = Attributes::default()
            .with_meta("region", "NA")
            .with_tag("energy")
            .with_meta("rating", "CCC");

        assert_eq!(matcher.match_factor(&dep, &attrs), Some(FactorId::new("NA-Energy-CCC")));
    }

    #[test]
    fn test_rolls_up_to_parent() {
        let matcher = HierarchicalMatcher::new(credit_tree());
        let dep = MarketDependency::CreditCurve { id: CurveId::new("X") };

        // NA but not energy — rolls up to NA-Credit
        let attrs = Attributes::default()
            .with_meta("region", "NA")
            .with_tag("financials");

        assert_eq!(matcher.match_factor(&dep, &attrs), Some(FactorId::new("NA-Credit")));
    }

    #[test]
    fn test_root_fallback() {
        let matcher = HierarchicalMatcher::new(credit_tree());
        let dep = MarketDependency::CreditCurve { id: CurveId::new("X") };

        // APAC — no child matches, falls to root
        let attrs = Attributes::default().with_meta("region", "APAC");

        assert_eq!(matcher.match_factor(&dep, &attrs), Some(FactorId::new("Generic-Credit")));
    }

    #[test]
    fn test_eu_branch() {
        let matcher = HierarchicalMatcher::new(credit_tree());
        let dep = MarketDependency::CreditCurve { id: CurveId::new("X") };

        let attrs = Attributes::default().with_meta("region", "EU");

        assert_eq!(matcher.match_factor(&dep, &attrs), Some(FactorId::new("EU-Credit")));
    }

    #[test]
    fn test_energy_without_rating_rolls_up_to_na() {
        let matcher = HierarchicalMatcher::new(credit_tree());
        let dep = MarketDependency::CreditCurve { id: CurveId::new("X") };

        // NA + energy but no rating — Energy node has no factor_id, rolls up to NA-Credit
        let attrs = Attributes::default()
            .with_meta("region", "NA")
            .with_tag("energy");

        assert_eq!(matcher.match_factor(&dep, &attrs), Some(FactorId::new("NA-Credit")));
    }

    #[test]
    fn test_factor_node_serde_roundtrip() {
        let tree = credit_tree();
        let json = serde_json::to_string(&tree).unwrap();
        let back: FactorNode = serde_json::from_str(&json).unwrap();
        // Verify structure survived
        assert_eq!(back.children.len(), 2);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

- [ ] **Step 3: Implement HierarchicalMatcher and FactorNode**

```rust
use super::filter::AttributeFilter;
use super::traits::FactorMatcher;
use crate::factor_model::dependency::MarketDependency;
use crate::factor_model::types::FactorId;
use crate::types::Attributes;
use serde::{Deserialize, Serialize};

/// A node in the factor hierarchy tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorNode {
    /// Factor assigned at this node (if leaf or valid assignment level)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub factor_id: Option<FactorId>,
    /// Filter that must match for traversal into this node
    pub filter: AttributeFilter,
    /// Child nodes (more specific classifications)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<FactorNode>,
}

/// Tree-based matcher. Matches to the deepest node with a factor_id.
pub struct HierarchicalMatcher {
    root: FactorNode,
}

impl HierarchicalMatcher {
    pub fn new(root: FactorNode) -> Self {
        Self { root }
    }

    /// Depth-first search: returns the deepest matching node's factor_id.
    fn find_deepest(&self, node: &FactorNode, attrs: &Attributes) -> Option<FactorId> {
        if !node.filter.matches(attrs) {
            return None;
        }

        // Try children first (depth-first, first match wins at each level)
        for child in &node.children {
            if let Some(id) = self.find_deepest(child, attrs) {
                return Some(id);
            }
        }

        // No child matched deeper — return this node's factor_id if it has one
        node.factor_id.clone()
    }
}

impl FactorMatcher for HierarchicalMatcher {
    fn match_factor(
        &self,
        _dependency: &MarketDependency,
        attributes: &Attributes,
    ) -> Option<FactorId> {
        self.find_deepest(&self.root, attributes)
    }
}
```

- [ ] **Step 4: Register in mod.rs**

- [ ] **Step 5: Run tests**

Run: `cargo test -p finstack-core factor_model::matching::hierarchical --no-default-features`
Expected: 6 tests PASS

- [ ] **Step 6: Commit**

```bash
git add finstack/core/src/factor_model/matching/
git commit -m "feat(factor-model): add HierarchicalMatcher with FactorNode tree"
```

---

## Task 5: Create `MatchingConfig` and config-to-matcher construction

**Files:**

- Create: `finstack/core/src/factor_model/matching/config.rs`
- Modify: `finstack/core/src/factor_model/matching/mod.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::factor_model::dependency::{CurveType, MarketDependency};
    use crate::factor_model::matching::filter::{AttributeFilter, DependencyFilter};
    use crate::factor_model::matching::mapping_table::MappingRule;
    use crate::factor_model::types::FactorId;
    use crate::types::Attributes;
    use crate::types::id::CurveId;

    #[test]
    fn test_matching_config_mapping_table() {
        let config = MatchingConfig::MappingTable(vec![MappingRule {
            dependency_filter: DependencyFilter {
                dependency_type: Some(CurveType::Hazard),
                id: None,
            },
            attribute_filter: AttributeFilter::default(),
            factor_id: FactorId::new("Credit"),
        }]);

        let matcher = config.build_matcher();
        let dep = MarketDependency::CreditCurve { id: CurveId::new("X") };
        assert_eq!(matcher.match_factor(&dep, &Attributes::default()), Some(FactorId::new("Credit")));
    }

    #[test]
    fn test_matching_config_cascade() {
        let config = MatchingConfig::Cascade(vec![
            MatchingConfig::MappingTable(vec![MappingRule {
                dependency_filter: DependencyFilter { dependency_type: None, id: Some("SPECIAL".into()) },
                attribute_filter: AttributeFilter::default(),
                factor_id: FactorId::new("Special"),
            }]),
            MatchingConfig::MappingTable(vec![MappingRule {
                dependency_filter: DependencyFilter::default(),
                attribute_filter: AttributeFilter::default(),
                factor_id: FactorId::new("Fallback"),
            }]),
        ]);

        let matcher = config.build_matcher();

        let dep1 = MarketDependency::Spot { id: "SPECIAL".into() };
        assert_eq!(matcher.match_factor(&dep1, &Attributes::default()), Some(FactorId::new("Special")));

        let dep2 = MarketDependency::Spot { id: "OTHER".into() };
        assert_eq!(matcher.match_factor(&dep2, &Attributes::default()), Some(FactorId::new("Fallback")));
    }

    #[test]
    fn test_matching_config_serde_roundtrip() {
        let config = MatchingConfig::Cascade(vec![
            MatchingConfig::MappingTable(vec![]),
            MatchingConfig::Hierarchical(FactorNode {
                factor_id: Some(FactorId::new("Root")),
                filter: AttributeFilter::default(),
                children: vec![],
            }),
        ]);
        let json = serde_json::to_string(&config).unwrap();
        let back: MatchingConfig = serde_json::from_str(&json).unwrap();
        // Verify it round-trips (serialize again and compare JSON)
        assert_eq!(json, serde_json::to_string(&back).unwrap());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

- [ ] **Step 3: Implement MatchingConfig**

```rust
use super::cascade::CascadeMatcher;
use super::hierarchical::{FactorNode, HierarchicalMatcher};
use super::mapping_table::{MappingRule, MappingTableMatcher};
use super::traits::FactorMatcher;
use serde::{Deserialize, Serialize};

/// Declarative matching configuration — serializable alternative to trait objects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchingConfig {
    MappingTable(Vec<MappingRule>),
    Cascade(Vec<MatchingConfig>),
    Hierarchical(FactorNode),
}

impl MatchingConfig {
    /// Build a concrete FactorMatcher from this config.
    pub fn build_matcher(&self) -> Box<dyn FactorMatcher> {
        match self {
            Self::MappingTable(rules) => {
                Box::new(MappingTableMatcher::new(rules.clone()))
            }
            Self::Cascade(configs) => {
                let matchers: Vec<Box<dyn FactorMatcher>> =
                    configs.iter().map(|c| c.build_matcher()).collect();
                Box::new(CascadeMatcher::new(matchers))
            }
            Self::Hierarchical(root) => {
                Box::new(HierarchicalMatcher::new(root.clone()))
            }
        }
    }
}
```

- [ ] **Step 4: Register in mod.rs and update re-exports**

- [ ] **Step 5: Run tests**

Run: `cargo test -p finstack-core factor_model::matching --no-default-features`
Expected: All matching tests PASS

- [ ] **Step 6: Commit**

```bash
git add finstack/core/src/factor_model/matching/
git commit -m "feat(factor-model): add MatchingConfig with declarative-to-trait bridge"
```

---

## Task 6: Implement `decompose()` bridge in valuations

**Files:**

- Create: `finstack/valuations/src/factor_model/mod.rs`
- Create: `finstack/valuations/src/factor_model/decompose.rs`
- Modify: `finstack/valuations/src/lib.rs` — add `pub mod factor_model;`

**Context:** `MarketDependencies` is at `finstack/valuations/src/instruments/common/dependencies.rs:28-41`. It has `curves: InstrumentCurves`, `spot_ids`, `vol_surface_ids`, `fx_pairs`, `series_ids`. The `InstrumentCurves` struct contains `discount_curves`, `forward_curves`, `credit_curves` (each `Vec<CurveId>`).

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::factor_model::{CurveType, MarketDependency};
    use finstack_core::types::id::CurveId;

    #[test]
    fn test_decompose_empty_dependencies() {
        let deps = MarketDependencies::new();
        let result = decompose(&deps);
        assert!(result.is_empty());
    }

    #[test]
    fn test_decompose_discount_curves() {
        let mut deps = MarketDependencies::new();
        deps.curves.discount_curves.push(CurveId::new("USD-OIS"));
        let result = decompose(&deps);
        assert_eq!(result.len(), 1);
        match &result[0] {
            MarketDependency::Curve { id, curve_type } => {
                assert_eq!(id.as_ref(), "USD-OIS");
                assert_eq!(*curve_type, CurveType::Discount);
            }
            _ => panic!("expected Curve variant"),
        }
    }

    #[test]
    fn test_decompose_credit_curves() {
        let mut deps = MarketDependencies::new();
        deps.curves.credit_curves.push(CurveId::new("ACME-HAZARD"));
        let result = decompose(&deps);
        assert_eq!(result.len(), 1);
        match &result[0] {
            MarketDependency::CreditCurve { id } => {
                assert_eq!(id.as_ref(), "ACME-HAZARD");
            }
            _ => panic!("expected CreditCurve variant"),
        }
    }

    #[test]
    fn test_decompose_mixed() {
        let mut deps = MarketDependencies::new();
        deps.curves.discount_curves.push(CurveId::new("USD-OIS"));
        deps.curves.credit_curves.push(CurveId::new("ACME-HAZARD"));
        deps.spot_ids.push("AAPL".into());
        deps.vol_surface_ids.push("AAPL-VOL".into());
        let result = decompose(&deps);
        assert_eq!(result.len(), 4);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p finstack-valuations factor_model::decompose --no-default-features`
Expected: FAIL

- [ ] **Step 3: Implement decompose()**

```rust
use crate::instruments::common::dependencies::MarketDependencies;
use finstack_core::factor_model::{CurveType, MarketDependency};

/// Flatten a MarketDependencies struct into individual MarketDependency entries.
pub fn decompose(deps: &MarketDependencies) -> Vec<MarketDependency> {
    let mut result = Vec::new();

    for id in &deps.curves.discount_curves {
        result.push(MarketDependency::Curve {
            id: id.clone(),
            curve_type: CurveType::Discount,
        });
    }

    for id in &deps.curves.forward_curves {
        result.push(MarketDependency::Curve {
            id: id.clone(),
            curve_type: CurveType::Forward,
        });
    }

    for id in &deps.curves.credit_curves {
        result.push(MarketDependency::CreditCurve { id: id.clone() });
    }

    for id in &deps.spot_ids {
        result.push(MarketDependency::Spot { id: id.clone() });
    }

    for id in &deps.vol_surface_ids {
        result.push(MarketDependency::VolSurface { id: id.clone() });
    }

    for pair in &deps.fx_pairs {
        result.push(MarketDependency::FxPair {
            base: pair.base,
            quote: pair.quote,
        });
    }

    for id in &deps.series_ids {
        result.push(MarketDependency::Series { id: id.clone() });
    }

    result
}
```

Note: Adjust field names to match the actual `InstrumentCurves` and `FxPair` structs — the explorer found these field names but they may vary slightly. Check the actual code.

- [ ] **Step 4: Wire up modules**

Create `finstack/valuations/src/factor_model/mod.rs`:

```rust
mod decompose;
pub use decompose::decompose;
```

Add `pub mod factor_model;` to `finstack/valuations/src/lib.rs`.

- [ ] **Step 5: Run tests**

Run: `cargo test -p finstack-valuations factor_model --no-default-features`
Expected: 4 tests PASS

- [ ] **Step 6: Run workspace build**

Run: `cargo build --workspace`
Expected: SUCCESS

- [ ] **Step 7: Commit**

```bash
git add finstack/valuations/src/factor_model/ finstack/valuations/src/lib.rs
git commit -m "feat(factor-model): add decompose() bridge from MarketDependencies to MarketDependency"
```
