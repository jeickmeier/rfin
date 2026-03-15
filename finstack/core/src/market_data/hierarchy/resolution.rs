//! Resolution engine for hierarchy-targeted operations.
//!
//! Resolves hierarchy paths + tag filters to sets of `CurveId`s, with
//! configurable inheritance modes (most-specific-wins vs. cumulative).

use super::{HierarchyNode, MarketDataHierarchy, NodePath};
use crate::collections::HashMap;
use crate::types::CurveId;
use serde::{Deserialize, Serialize};

/// Controls how shocks at multiple hierarchy levels combine for a single curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionMode {
    /// The deepest (most specific) matching node's shock wins.
    /// Parent-level shocks are ignored if a more specific one exists.
    #[default]
    MostSpecificWins,

    /// Shocks accumulate walking down the tree.
    /// A curve gets the sum of all shocks from root to its leaf node.
    Cumulative,
}

/// A predicate for filtering nodes by their tags.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TagPredicate {
    /// Tag value must exactly equal the given value.
    Equals {
        /// Tag key to match.
        key: String,
        /// Expected tag value.
        value: String,
    },
    /// Tag value must be one of the given values.
    In {
        /// Tag key to match.
        key: String,
        /// Accepted tag values.
        values: Vec<String>,
    },
    /// Tag key must exist (any value).
    Exists {
        /// Tag key that must be present.
        key: String,
    },
}

impl TagPredicate {
    /// Check if a node's tags satisfy this predicate.
    pub fn matches(&self, tags: &HashMap<String, String>) -> bool {
        match self {
            TagPredicate::Equals { key, value } => tags.get(key).is_some_and(|v| v == value),
            TagPredicate::In { key, values } => tags.get(key).is_some_and(|v| values.contains(v)),
            TagPredicate::Exists { key } => tags.contains_key(key),
        }
    }
}

/// A filter combining multiple tag predicates (AND semantics).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TagFilter {
    /// All predicates must match (AND semantics).
    pub predicates: Vec<TagPredicate>,
}

impl TagFilter {
    /// Check if a node's tags satisfy all predicates.
    pub fn matches(&self, tags: &HashMap<String, String>) -> bool {
        self.predicates.iter().all(|p| p.matches(tags))
    }

    /// An empty filter matches everything.
    pub fn is_empty(&self) -> bool {
        self.predicates.is_empty()
    }
}

/// A target specifying a hierarchy path with optional tag filtering.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HierarchyTarget {
    /// Path through the hierarchy (e.g., `["Credit", "US", "IG"]`).
    pub path: NodePath,
    /// Optional tag filter applied to nodes in the subtree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag_filter: Option<TagFilter>,
}

impl MarketDataHierarchy {
    /// Resolve a hierarchy target to the set of `CurveId`s it covers.
    ///
    /// Walks to the node at `target.path`, then collects all `CurveId`s in that
    /// subtree. If a `tag_filter` is provided, only curves under nodes matching
    /// the filter are included.
    pub fn resolve(&self, target: &HierarchyTarget) -> Vec<CurveId> {
        let Some(node) = self.get_node(&target.path) else {
            return Vec::new();
        };

        match &target.tag_filter {
            None => node.all_curve_ids(),
            Some(filter) => {
                let mut ids = Vec::new();
                collect_filtered(node, filter, &mut ids);
                ids
            }
        }
    }

    /// Find all curves matching a tag filter across the entire hierarchy.
    pub fn query_by_tags(&self, filter: &TagFilter) -> Vec<CurveId> {
        let mut ids = Vec::new();
        for root in self.roots.values() {
            collect_filtered(root, filter, &mut ids);
        }
        ids
    }
}

/// Recursively collect curve IDs from nodes whose tags match the filter.
fn collect_filtered(node: &HierarchyNode, filter: &TagFilter, ids: &mut Vec<CurveId>) {
    if filter.matches(node.tags()) {
        ids.extend(node.curve_ids().iter().cloned());
    }
    for child in node.children().values() {
        collect_filtered(child, filter, ids);
    }
}
