use super::filter::{AttributeFilter, DependencyFilter};
use super::traits::FactorMatcher;
use crate::factor_model::dependency::MarketDependency;
use crate::factor_model::types::FactorId;
use crate::types::Attributes;
use serde::{Deserialize, Serialize};

/// A single matching rule from dependency and attribute filters to a factor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MappingRule {
    /// Dependency-side filter.
    pub dependency_filter: DependencyFilter,
    /// Instrument metadata filter.
    pub attribute_filter: AttributeFilter,
    /// Factor assigned when both filters match.
    pub factor_id: FactorId,
}

/// Flat lookup-table matcher where the first matching rule wins.
#[derive(Debug, Clone, Default)]
pub struct MappingTableMatcher {
    rules: Vec<MappingRule>,
}

impl MappingTableMatcher {
    /// Creates a matcher from an ordered set of rules.
    #[must_use]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factor_model::dependency::{CurveType, DependencyType, MarketDependency};
    use crate::types::CurveId;

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
                    dependency_type: Some(DependencyType::Credit),
                    curve_type: None,
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
                    dependency_type: Some(DependencyType::Credit),
                    curve_type: None,
                    id: None,
                },
                attribute_filter: AttributeFilter::default(),
                factor_id: FactorId::new("Generic-Credit"),
            },
        ]);

        let dep = MarketDependency::CreditCurve {
            id: CurveId::new("ACME-HAZARD"),
        };

        let attrs1 = Attributes::default()
            .with_tag("energy")
            .with_meta("rating", "CCC");
        assert_eq!(
            matcher.match_factor(&dep, &attrs1),
            Some(FactorId::new("NA-Energy-CCC"))
        );

        let attrs2 = Attributes::default().with_tag("financials");
        assert_eq!(
            matcher.match_factor(&dep, &attrs2),
            Some(FactorId::new("Generic-Credit"))
        );
    }

    #[test]
    fn test_no_match_returns_none() {
        let matcher = MappingTableMatcher::new(vec![MappingRule {
            dependency_filter: DependencyFilter {
                dependency_type: Some(DependencyType::Credit),
                curve_type: None,
                id: None,
            },
            attribute_filter: AttributeFilter::default(),
            factor_id: FactorId::new("Credit"),
        }]);

        let dep = MarketDependency::Curve {
            id: CurveId::new("USD-OIS"),
            curve_type: CurveType::Discount,
        };
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
        let json_result = serde_json::to_string(&rules);
        assert!(json_result.is_ok());
        let Ok(json) = json_result else {
            return;
        };

        let roundtrip_result: Result<Vec<MappingRule>, _> = serde_json::from_str(&json);
        assert!(roundtrip_result.is_ok());
        let Ok(roundtrip) = roundtrip_result else {
            return;
        };

        assert_eq!(rules, roundtrip);
    }
}
