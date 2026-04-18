//! Matching primitives for mapping market dependencies to factor identifiers.

mod config;
mod filter;
mod matchers;

pub use config::{HierarchicalConfig, MatchingConfig};
pub use filter::{AttributeFilter, DependencyFilter};
pub use matchers::{
    CascadeMatcher, FactorMatcher, FactorNode, HierarchicalMatcher, MappingRule,
    MappingTableMatcher,
};
