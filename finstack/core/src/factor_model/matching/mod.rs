//! Matching primitives for mapping market dependencies to factor identifiers.

mod cascade;
mod config;
mod filter;
mod hierarchical;
mod mapping_table;
mod traits;

pub use cascade::CascadeMatcher;
pub use config::{HierarchicalConfig, MatchingConfig};
pub use filter::{AttributeFilter, DependencyFilter};
pub use hierarchical::{FactorNode, HierarchicalMatcher};
pub use mapping_table::{MappingRule, MappingTableMatcher};
pub use traits::FactorMatcher;
