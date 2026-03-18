use crate::factor_model::dependency::MarketDependency;
use crate::factor_model::types::FactorId;
use crate::types::Attributes;

/// Matches a market dependency and instrument attributes to a factor identifier.
pub trait FactorMatcher: Send + Sync {
    /// Returns the matching factor identifier, if any.
    fn match_factor(
        &self,
        dependency: &MarketDependency,
        attributes: &Attributes,
    ) -> Option<FactorId>;
}
