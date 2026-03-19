mod config;
mod covariance;
mod definition;
mod dependency;
mod error;
/// Matching primitives and built-in matcher components.
pub mod matching;
mod types;

pub use config::{BumpSizeConfig, FactorModelConfig, PricingMode, RiskMeasure};
pub use covariance::FactorCovarianceMatrix;
pub use definition::{FactorDefinition, MarketMapping};
pub use dependency::{CurveType, DependencyType, MarketDependency};
pub use error::{FactorModelError, UnmatchedPolicy};
pub use matching::{
    AttributeFilter, CascadeMatcher, DependencyFilter, FactorMatcher, FactorNode,
    HierarchicalConfig, HierarchicalMatcher, MappingRule, MappingTableMatcher, MatchingConfig,
};
pub use types::{FactorId, FactorType};
