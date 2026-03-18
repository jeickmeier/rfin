use super::filter::{AttributeFilter, DependencyFilter};
use super::traits::FactorMatcher;
use crate::factor_model::dependency::MarketDependency;
use crate::factor_model::types::FactorId;
use crate::types::Attributes;
use serde::{Deserialize, Serialize};

/// A node in a hierarchical factor classification tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactorNode {
    /// Factor assigned at this node when it is a valid classification level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub factor_id: Option<FactorId>,
    /// Filter that must match for this node to participate in traversal.
    pub filter: AttributeFilter,
    /// Child nodes representing more specific classifications.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<FactorNode>,
}

/// Tree-based matcher where the deepest matching factor assignment wins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HierarchicalMatcher {
    dependency_filter: DependencyFilter,
    root: FactorNode,
}

impl HierarchicalMatcher {
    /// Creates a matcher from the provided root node.
    #[must_use]
    pub fn new(root: FactorNode) -> Self {
        Self {
            dependency_filter: DependencyFilter::default(),
            root,
        }
    }

    /// Creates a matcher scoped to dependencies satisfying the provided filter.
    #[must_use]
    pub fn new_scoped(dependency_filter: DependencyFilter, root: FactorNode) -> Self {
        Self {
            dependency_filter,
            root,
        }
    }

    fn find_best_match(
        node: &FactorNode,
        attrs: &Attributes,
        depth: usize,
    ) -> Option<(usize, FactorId)> {
        if !node.filter.matches(attrs) {
            return None;
        }

        let mut best = node.factor_id.clone().map(|factor_id| (depth, factor_id));

        for child in &node.children {
            if let Some(candidate) = Self::find_best_match(child, attrs, depth + 1) {
                let should_replace = match &best {
                    Some((best_depth, _)) => candidate.0 > *best_depth,
                    None => true,
                };
                if should_replace {
                    best = Some(candidate);
                }
            }
        }

        best
    }
}

impl FactorMatcher for HierarchicalMatcher {
    fn match_factor(
        &self,
        dependency: &MarketDependency,
        attributes: &Attributes,
    ) -> Option<FactorId> {
        if !self.dependency_filter.matches(dependency) {
            return None;
        }
        Self::find_best_match(&self.root, attributes, 0).map(|(_, factor_id)| factor_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factor_model::dependency::MarketDependency;
    use crate::types::CurveId;

    fn credit_tree() -> FactorNode {
        FactorNode {
            factor_id: Some(FactorId::new("Generic-Credit")),
            filter: AttributeFilter::default(),
            children: vec![
                FactorNode {
                    factor_id: Some(FactorId::new("NA-Credit")),
                    filter: AttributeFilter {
                        tags: vec![],
                        meta: vec![("region".into(), "NA".into())],
                    },
                    children: vec![FactorNode {
                        factor_id: None,
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
                    }],
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
        let dep = MarketDependency::CreditCurve {
            id: CurveId::new("X"),
        };

        let attrs = Attributes::default()
            .with_meta("region", "NA")
            .with_tag("energy")
            .with_meta("rating", "CCC");

        assert_eq!(
            matcher.match_factor(&dep, &attrs),
            Some(FactorId::new("NA-Energy-CCC"))
        );
    }

    #[test]
    fn test_rolls_up_to_parent() {
        let matcher = HierarchicalMatcher::new(credit_tree());
        let dep = MarketDependency::CreditCurve {
            id: CurveId::new("X"),
        };

        let attrs = Attributes::default()
            .with_meta("region", "NA")
            .with_tag("financials");

        assert_eq!(
            matcher.match_factor(&dep, &attrs),
            Some(FactorId::new("NA-Credit"))
        );
    }

    #[test]
    fn test_root_fallback() {
        let matcher = HierarchicalMatcher::new(credit_tree());
        let dep = MarketDependency::CreditCurve {
            id: CurveId::new("X"),
        };

        let attrs = Attributes::default().with_meta("region", "APAC");

        assert_eq!(
            matcher.match_factor(&dep, &attrs),
            Some(FactorId::new("Generic-Credit"))
        );
    }

    #[test]
    fn test_eu_branch() {
        let matcher = HierarchicalMatcher::new(credit_tree());
        let dep = MarketDependency::CreditCurve {
            id: CurveId::new("X"),
        };

        let attrs = Attributes::default().with_meta("region", "EU");

        assert_eq!(
            matcher.match_factor(&dep, &attrs),
            Some(FactorId::new("EU-Credit"))
        );
    }

    #[test]
    fn test_energy_without_rating_rolls_up_to_na() {
        let matcher = HierarchicalMatcher::new(credit_tree());
        let dep = MarketDependency::CreditCurve {
            id: CurveId::new("X"),
        };

        let attrs = Attributes::default()
            .with_meta("region", "NA")
            .with_tag("energy");

        assert_eq!(
            matcher.match_factor(&dep, &attrs),
            Some(FactorId::new("NA-Credit"))
        );
    }

    #[test]
    fn test_factor_node_serde_roundtrip() {
        let tree = credit_tree();
        let json_result = serde_json::to_string(&tree);
        assert!(json_result.is_ok());
        let Ok(json) = json_result else {
            return;
        };

        let roundtrip_result: Result<FactorNode, _> = serde_json::from_str(&json);
        assert!(roundtrip_result.is_ok());
        let Ok(roundtrip) = roundtrip_result else {
            return;
        };

        assert_eq!(roundtrip.children.len(), 2);
    }

    #[test]
    fn test_scoped_hierarchical_matcher_filters_dependency_class() {
        let matcher = HierarchicalMatcher::new_scoped(
            DependencyFilter {
                dependency_type: Some(crate::factor_model::DependencyType::Credit),
                curve_type: None,
                id: None,
            },
            credit_tree(),
        );
        let credit_dep = MarketDependency::CreditCurve {
            id: CurveId::new("X"),
        };
        let spot_dep = MarketDependency::Spot { id: "AAPL".into() };
        let attrs = Attributes::default().with_meta("region", "EU");

        assert_eq!(
            matcher.match_factor(&credit_dep, &attrs),
            Some(FactorId::new("EU-Credit"))
        );
        assert_eq!(matcher.match_factor(&spot_dep, &attrs), None);
    }
}
