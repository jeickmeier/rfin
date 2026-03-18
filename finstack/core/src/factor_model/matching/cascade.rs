use super::traits::FactorMatcher;
use crate::factor_model::dependency::MarketDependency;
use crate::factor_model::types::FactorId;
use crate::types::Attributes;

/// Ordered matcher chain that returns the first successful factor match.
pub struct CascadeMatcher {
    matchers: Vec<Box<dyn FactorMatcher>>,
}

impl CascadeMatcher {
    /// Creates a cascade from matchers tried in priority order.
    #[must_use]
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
            .find_map(|matcher| matcher.match_factor(dependency, attributes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factor_model::dependency::{DependencyType, MarketDependency};
    use crate::factor_model::matching::{
        AttributeFilter, DependencyFilter, MappingRule, MappingTableMatcher,
    };
    use crate::types::CurveId;

    #[test]
    fn test_cascade_tries_in_order() {
        let exact = MappingTableMatcher::new(vec![MappingRule {
            dependency_filter: DependencyFilter {
                dependency_type: Some(DependencyType::Credit),
                curve_type: None,
                id: Some("ACME-HAZARD".into()),
            },
            attribute_filter: AttributeFilter::default(),
            factor_id: FactorId::new("ACME-Specific"),
        }]);

        let fallback = MappingTableMatcher::new(vec![MappingRule {
            dependency_filter: DependencyFilter {
                dependency_type: Some(DependencyType::Credit),
                curve_type: None,
                id: None,
            },
            attribute_filter: AttributeFilter::default(),
            factor_id: FactorId::new("Generic-Credit"),
        }]);

        let cascade = CascadeMatcher::new(vec![Box::new(exact), Box::new(fallback)]);
        let attrs = Attributes::default();

        let dep1 = MarketDependency::CreditCurve {
            id: CurveId::new("ACME-HAZARD"),
        };
        assert_eq!(
            cascade.match_factor(&dep1, &attrs),
            Some(FactorId::new("ACME-Specific"))
        );

        let dep2 = MarketDependency::CreditCurve {
            id: CurveId::new("OTHER-HAZARD"),
        };
        assert_eq!(
            cascade.match_factor(&dep2, &attrs),
            Some(FactorId::new("Generic-Credit"))
        );

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
