//! Fluent builder for constructing hierarchies from `/`-separated paths.

use super::{HierarchyNode, MarketDataHierarchy};
use crate::collections::HashMap;
use crate::types::CurveId;

/// Fluent builder for `MarketDataHierarchy`.
///
/// Uses `/`-separated paths to auto-create intermediate nodes:
///
/// ```rust
/// use finstack_core::market_data::hierarchy::MarketDataHierarchy;
///
/// let h = MarketDataHierarchy::builder()
///     .add_node("Rates/USD/OIS").curve_ids(&["USD-OIS"])
///     .build()
///     .unwrap();
/// ```
pub struct HierarchyBuilder {
    hierarchy: MarketDataHierarchy,
    current_path: Option<Vec<String>>,
}

impl HierarchyBuilder {
    pub(super) fn new() -> Self {
        Self {
            hierarchy: MarketDataHierarchy::new(),
            current_path: None,
        }
    }

    /// Start or switch to a node at the given `/`-separated path.
    /// Creates intermediate nodes as needed.
    ///
    /// # Panics
    ///
    /// Panics if `path` is empty (i.e. an empty string `""`).
    pub fn add_node(mut self, path: &str) -> Self {
        let segments: Vec<String> = path.split('/').map(String::from).collect();

        // Ensure the root exists
        let root_name = segments[0].clone();
        if !self.hierarchy.roots.contains_key(root_name.as_str()) {
            self.hierarchy
                .roots
                .insert(root_name.clone(), HierarchyNode::new(root_name.as_str()));
        }

        // Navigate down creating children as needed — we avoid unwrap by using
        // `get_or_create_child` which always returns a valid mutable reference.
        if let Some(root) = self.hierarchy.roots.get_mut(root_name.as_str()) {
            let mut current = root;
            for segment in &segments[1..] {
                current = current.get_or_create_child(segment);
            }
        }

        self.current_path = Some(segments);
        self
    }

    /// Set a tag on the current node.
    ///
    /// No-op if `add_node` has not been called first.
    pub fn tag(mut self, key: &str, value: &str) -> Self {
        if let Some(path) = self.current_path.clone() {
            if let Some(node) = self.hierarchy.get_node_mut(&path) {
                node.set_tag(key, value);
            }
        }
        self
    }

    /// Add curve IDs to the current node.
    ///
    /// No-op if `add_node` has not been called first.
    pub fn curve_ids(mut self, ids: &[&str]) -> Self {
        if let Some(path) = self.current_path.clone() {
            if let Some(node) = self.hierarchy.get_node_mut(&path) {
                for id in ids {
                    node.add_curve_id(*id);
                }
            }
        }
        self
    }

    /// Finalize and validate the hierarchy.
    ///
    /// Validates:
    /// - No duplicate `CurveId` across different nodes (each curve has exactly one path).
    pub fn build(self) -> crate::Result<MarketDataHierarchy> {
        // Validate no duplicate curve IDs
        let mut seen: HashMap<CurveId, Vec<String>> = HashMap::default();
        Self::collect_curve_locations(&self.hierarchy, &mut seen);

        for (curve_id, locations) in &seen {
            if locations.len() > 1 {
                return Err(crate::Error::Validation(format!(
                    "CurveId '{}' appears in multiple hierarchy locations: {}",
                    curve_id.as_str(),
                    locations.join(", ")
                )));
            }
        }

        Ok(self.hierarchy)
    }

    fn collect_curve_locations(
        hierarchy: &MarketDataHierarchy,
        seen: &mut HashMap<CurveId, Vec<String>>,
    ) {
        fn visit(node: &HierarchyNode, path: &str, seen: &mut HashMap<CurveId, Vec<String>>) {
            let current_path = if path.is_empty() {
                node.name().to_string()
            } else {
                format!("{}/{}", path, node.name())
            };
            for id in node.curve_ids() {
                seen.entry(id.clone())
                    .or_default()
                    .push(current_path.clone());
            }
            for child in node.children().values() {
                visit(child, &current_path, seen);
            }
        }

        for root in hierarchy.roots().values() {
            visit(root, "", seen);
        }
    }
}
