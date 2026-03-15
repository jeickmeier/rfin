# Market Data Hierarchy Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a hierarchical organizational layer on top of `MarketContext`'s flat `HashMap<CurveId, _>` storage, enabling hierarchy-targeted scenario shocks, factor model mapping, and completeness tracking.

**Architecture:** A tree of `HierarchyNode`s lives in a new `finstack-core/src/market_data/hierarchy/` module. Each node has a name, tags, children (`IndexMap`), and leaf `CurveId` references. `MarketContext` gains an `Option<MarketDataHierarchy>` field. The scenario engine in `finstack-scenarios` resolves `HierarchyTarget` operations to per-curve shocks before applying them.

**Tech Stack:** Rust, serde/serde_json, indexmap (new dep for finstack-core), existing finstack-core types (`CurveId`, `HashMap`, `MarketContext`).

**Spec:** `docs/superpowers/specs/2026-03-15-market-data-hierarchy-design.md`

---

## Chunk 1: Core Types & Serde

### Task 1: Add `indexmap` dependency to finstack-core

`IndexMap` preserves insertion order for deterministic tree iteration — critical for reproducible scenario resolution and JSON round-trips.

**Files:**
- Modify: `finstack/core/Cargo.toml`

- [ ] **Step 1: Add indexmap dependency**

Add `indexmap` with serde support to `[dependencies]` in `finstack/core/Cargo.toml`:

```toml
indexmap = { workspace = true, features = ["serde"] }
```

Place it alphabetically among existing deps (after `lru`, before `nalgebra`).

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p finstack-core`
Expected: SUCCESS (no code changes yet, just dep addition)

- [ ] **Step 3: Commit**

```bash
git add finstack/core/Cargo.toml
git commit -m "build: add indexmap dependency to finstack-core for hierarchy module"
```

---

### Task 2: Create hierarchy module with core types

The core types: `NodePath`, `HierarchyNode`, `MarketDataHierarchy`. These are the data structures everything else builds on.

**Files:**
- Create: `finstack/core/src/market_data/hierarchy/mod.rs`
- Modify: `finstack/core/src/market_data/mod.rs` (add `pub mod hierarchy;`)

- [ ] **Step 1: Write the failing test**

Create the test file first. Tests go in the existing integration test structure under `finstack/core/tests/market_data/`.

Create `finstack/core/tests/market_data/hierarchy.rs`:

```rust
use finstack_core::market_data::hierarchy::{HierarchyNode, MarketDataHierarchy, NodePath};
use finstack_core::types::CurveId;

#[test]
fn empty_hierarchy_has_no_roots() {
    let h = MarketDataHierarchy::new();
    assert!(h.roots().is_empty());
}

#[test]
fn hierarchy_node_stores_name_and_curves() {
    let node = HierarchyNode::new("USD");
    assert_eq!(node.name(), "USD");
    assert!(node.curve_ids().is_empty());
    assert!(node.children().is_empty());
    assert!(node.tags().is_empty());
}

#[test]
fn node_path_is_vec_of_strings() {
    let path: NodePath = vec!["Rates".into(), "USD".into()];
    assert_eq!(path.len(), 2);
    assert_eq!(path[0], "Rates");
}
```

Register it in `finstack/core/tests/market_data.rs` by adding:

```rust
// Hierarchy tests
#[path = "market_data/hierarchy.rs"]
mod hierarchy;
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p finstack-core --test market_data hierarchy -- --no-run 2>&1 | head -30`
Expected: FAIL — module `hierarchy` not found

- [ ] **Step 3: Create the hierarchy module**

Create `finstack/core/src/market_data/hierarchy/mod.rs`:

```rust
//! Market data hierarchy for organizational grouping of curves.
//!
//! Provides a tree structure where each node can hold tags (key-value metadata),
//! child nodes, and leaf references (`CurveId`s) pointing into `MarketContext`'s
//! flat storage. The hierarchy enables:
//!
//! - **Scenario targeting**: Define shocks at any tree level; resolve to per-curve operations.
//! - **Factor model mapping**: Factor models reference hierarchy nodes for scope.
//! - **Completeness tracking**: Compare declared vs. present curves.
//!
//! # Example
//!
//! ```rust
//! use finstack_core::market_data::hierarchy::MarketDataHierarchy;
//!
//! let hierarchy = MarketDataHierarchy::builder()
//!     .add_node("Rates/USD/OIS")
//!         .curve_ids(&["USD-OIS"])
//!     .add_node("Credit/US/IG/Financials")
//!         .tag("sector", "Financials")
//!         .curve_ids(&["JPM-5Y", "GS-5Y"])
//!     .build()
//!     .expect("valid hierarchy");
//! ```

mod builder;
mod completeness;
mod resolution;

pub use builder::HierarchyBuilder;
pub use completeness::{CompletenessReport, SubtreeCoverage};
pub use resolution::{ResolutionMode, TagFilter, TagPredicate};

use crate::collections::HashMap;
use crate::types::CurveId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// A path through the hierarchy tree, e.g., `["Credit", "US", "IG", "Financials"]`.
pub type NodePath = Vec<String>;

/// A single node in the market data hierarchy tree.
///
/// Nodes form a tree: each has a name, optional key-value tags for cross-cutting
/// queries, ordered children, and leaf `CurveId` references.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchyNode {
    /// Node name. Populated from the parent map key during deserialization.
    /// Skipped during serde to match the spec's JSON format where the name
    /// is the map key (e.g., `{ "Rates": { "children": { "USD": ... } } }`).
    #[serde(skip)]
    name: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    tags: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    children: IndexMap<String, HierarchyNode>,
    #[serde(default, rename = "curves", skip_serializing_if = "Vec::is_empty")]
    curve_ids: Vec<CurveId>,
}

impl HierarchyNode {
    /// Create a new node with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tags: HashMap::default(),
            children: IndexMap::new(),
            curve_ids: Vec::new(),
        }
    }

    /// Node display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Tags attached to this node (key-value metadata).
    pub fn tags(&self) -> &HashMap<String, String> {
        &self.tags
    }

    /// Ordered child nodes.
    pub fn children(&self) -> &IndexMap<String, HierarchyNode> {
        &self.children
    }

    /// Leaf `CurveId` references at this node.
    pub fn curve_ids(&self) -> &[CurveId] {
        &self.curve_ids
    }

    /// Collect all `CurveId`s in this subtree (this node + all descendants).
    pub fn all_curve_ids(&self) -> Vec<CurveId> {
        let mut ids = self.curve_ids.clone();
        for child in self.children.values() {
            ids.extend(child.all_curve_ids());
        }
        ids
    }

    /// Set a tag on this node.
    pub fn set_tag(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.tags.insert(key.into(), value.into());
    }

    /// Add a child node. Returns a mutable reference to the child.
    pub fn add_child(&mut self, child: HierarchyNode) -> &mut HierarchyNode {
        let name = child.name.clone();
        self.children.insert(name.clone(), child);
        self.children.get_mut(&name).unwrap()
    }

    /// Get or create a child node by name.
    pub fn get_or_create_child(&mut self, name: &str) -> &mut HierarchyNode {
        if !self.children.contains_key(name) {
            self.children
                .insert(name.to_string(), HierarchyNode::new(name));
        }
        self.children.get_mut(name).unwrap()
    }

    /// Add a curve ID to this node's leaf set.
    pub fn add_curve_id(&mut self, id: impl Into<CurveId>) {
        self.curve_ids.push(id.into());
    }
}

/// The top-level market data hierarchy containing root nodes.
///
/// Each root represents a major asset class or category (e.g., "Rates", "Credit",
/// "FX", "Equity", "Volatility"). The hierarchy is fully serializable and can be
/// loaded from JSON configuration files.
#[derive(Debug, Clone, Serialize, Default)]
pub struct MarketDataHierarchy {
    roots: IndexMap<String, HierarchyNode>,
}

/// Custom deserialize that populates each node's `name` field from its map key.
impl<'de> Deserialize<'de> for MarketDataHierarchy {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Raw {
            roots: IndexMap<String, HierarchyNode>,
        }
        let mut raw = Raw::deserialize(deserializer)?;
        // Recursively set name from map key
        fn fixup_names(children: &mut IndexMap<String, HierarchyNode>) {
            for (key, node) in children.iter_mut() {
                node.name = key.clone();
                fixup_names(&mut node.children);
            }
        }
        fixup_names(&mut raw.roots);
        Ok(MarketDataHierarchy { roots: raw.roots })
    }
}

impl MarketDataHierarchy {
    /// Create an empty hierarchy.
    pub fn new() -> Self {
        Self::default()
    }

    /// Start building a hierarchy with the fluent builder API.
    pub fn builder() -> HierarchyBuilder {
        HierarchyBuilder::new()
    }

    /// Root nodes of the hierarchy.
    pub fn roots(&self) -> &IndexMap<String, HierarchyNode> {
        &self.roots
    }

    /// Mutable access to root nodes.
    pub fn roots_mut(&mut self) -> &mut IndexMap<String, HierarchyNode> {
        &mut self.roots
    }

    /// Look up a node by path. Returns `None` if the path doesn't exist.
    pub fn get_node(&self, path: &[String]) -> Option<&HierarchyNode> {
        let mut path_iter = path.iter();
        let root_name = path_iter.next()?;
        let mut current = self.roots.get(root_name.as_str())?;
        for segment in path_iter {
            current = current.children.get(segment.as_str())?;
        }
        Some(current)
    }

    /// Mutable lookup by path.
    pub fn get_node_mut(&mut self, path: &[String]) -> Option<&mut HierarchyNode> {
        let mut path_iter = path.iter();
        let root_name = path_iter.next()?;
        let mut current = self.roots.get_mut(root_name.as_str())?;
        for segment in path_iter {
            current = current.children.get_mut(segment.as_str())?;
        }
        Some(current)
    }

    /// Collect all `CurveId`s across the entire hierarchy.
    pub fn all_curve_ids(&self) -> Vec<CurveId> {
        let mut ids = Vec::new();
        for root in self.roots.values() {
            ids.extend(root.all_curve_ids());
        }
        ids
    }

    /// Insert a curve at a `/`-separated path, creating intermediate nodes as needed.
    pub fn insert_curve(&mut self, path: &str, curve_id: impl Into<CurveId>) {
        let segments: Vec<&str> = path.split('/').collect();
        assert!(!segments.is_empty(), "path must not be empty");

        let root_name = segments[0];
        if !self.roots.contains_key(root_name) {
            self.roots
                .insert(root_name.to_string(), HierarchyNode::new(root_name));
        }
        let mut current = self.roots.get_mut(root_name).unwrap();

        for &segment in &segments[1..] {
            current = current.get_or_create_child(segment);
        }

        current.curve_ids.push(curve_id.into());
    }

    /// Remove a curve from wherever it sits in the tree. Returns `true` if found.
    pub fn remove_curve(&mut self, curve_id: &CurveId) -> bool {
        fn remove_from_node(node: &mut HierarchyNode, target: &CurveId) -> bool {
            if let Some(pos) = node.curve_ids.iter().position(|id| id == target) {
                node.curve_ids.remove(pos);
                return true;
            }
            for child in node.children.values_mut() {
                if remove_from_node(child, target) {
                    return true;
                }
            }
            false
        }

        for root in self.roots.values_mut() {
            if remove_from_node(root, curve_id) {
                return true;
            }
        }
        false
    }

    /// Find the path from root to a specific curve. Returns `None` if not found.
    pub fn path_for_curve(&self, curve_id: &CurveId) -> Option<NodePath> {
        fn find_in_node(
            node: &HierarchyNode,
            target: &CurveId,
            path: &mut Vec<String>,
        ) -> bool {
            path.push(node.name().to_string());
            if node.curve_ids.iter().any(|id| id == target) {
                return true;
            }
            for child in node.children.values() {
                if find_in_node(child, target, path) {
                    return true;
                }
            }
            path.pop();
            false
        }

        for root in self.roots.values() {
            let mut path = Vec::new();
            if find_in_node(root, curve_id, &mut path) {
                return Some(path);
            }
        }
        None
    }

    /// Set a tag on a node at the given `/`-separated path.
    pub fn set_tag(&mut self, path: &str, key: &str, value: &str) {
        let segments: Vec<String> = path.split('/').map(String::from).collect();
        if let Some(node) = self.get_node_mut(&segments) {
            node.set_tag(key, value);
        }
    }
}
```

- [ ] **Step 4: Register the hierarchy module**

In `finstack/core/src/market_data/mod.rs`, add after the `pub mod traits;` line:

```rust
/// Market data hierarchy for organizational grouping and scenario targeting.
pub mod hierarchy;
```

- [ ] **Step 5: Create stub files for sub-modules**

Create `finstack/core/src/market_data/hierarchy/builder.rs`:

```rust
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
    pub fn add_node(mut self, path: &str) -> Self {
        let segments: Vec<String> = path.split('/').map(String::from).collect();
        assert!(!segments.is_empty(), "path must not be empty");

        // Ensure all intermediate nodes exist
        let root_name = &segments[0];
        if !self.hierarchy.roots.contains_key(root_name.as_str()) {
            self.hierarchy
                .roots
                .insert(root_name.clone(), HierarchyNode::new(root_name.as_str()));
        }

        let mut current = self.hierarchy.roots.get_mut(root_name.as_str()).unwrap();
        for segment in &segments[1..] {
            current = current.get_or_create_child(segment);
        }

        self.current_path = Some(segments);
        self
    }

    /// Set a tag on the current node.
    pub fn tag(mut self, key: &str, value: &str) -> Self {
        let path = self
            .current_path
            .as_ref()
            .expect("call add_node before tag");
        let node = self
            .hierarchy
            .get_node_mut(path)
            .expect("current path must exist");
        node.set_tag(key, value);
        self
    }

    /// Add curve IDs to the current node.
    pub fn curve_ids(mut self, ids: &[&str]) -> Self {
        let path = self
            .current_path
            .as_ref()
            .expect("call add_node before curve_ids");
        let node = self
            .hierarchy
            .get_node_mut(path)
            .expect("current path must exist");
        for id in ids {
            node.add_curve_id(*id);
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
                return Err(crate::Error::InvalidInput(format!(
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
```

Create `finstack/core/src/market_data/hierarchy/resolution.rs`:

```rust
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
    Equals { key: String, value: String },
    /// Tag value must be one of the given values.
    In { key: String, values: Vec<String> },
    /// Tag key must exist (any value).
    Exists { key: String },
}

impl TagPredicate {
    /// Check if a node's tags satisfy this predicate.
    pub fn matches(&self, tags: &HashMap<String, String>) -> bool {
        match self {
            TagPredicate::Equals { key, value } => tags.get(key).map_or(false, |v| v == value),
            TagPredicate::In { key, values } => {
                tags.get(key).map_or(false, |v| values.contains(v))
            }
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
```

Create `finstack/core/src/market_data/hierarchy/completeness.rs`:

```rust
//! Completeness tracking — compare hierarchy-declared curves against MarketContext.

use super::NodePath;
use crate::types::CurveId;
use serde::{Deserialize, Serialize};

/// Report comparing hierarchy-declared `CurveId`s against what exists in `MarketContext`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletenessReport {
    /// CurveIds declared in hierarchy but missing from MarketContext.
    pub missing: Vec<(NodePath, CurveId)>,

    /// CurveIds in MarketContext that aren't in any hierarchy node.
    pub unclassified: Vec<CurveId>,

    /// Per-subtree coverage statistics.
    pub coverage: Vec<SubtreeCoverage>,
}

/// Coverage statistics for a single subtree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtreeCoverage {
    /// Path to the subtree root.
    pub path: NodePath,
    /// Number of CurveIds declared in this subtree.
    pub total_expected: usize,
    /// Number of those CurveIds that are present in MarketContext.
    pub total_present: usize,
    /// Coverage percentage (0.0–100.0).
    pub percent: f64,
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p finstack-core --test market_data hierarchy`
Expected: 3 tests PASS

- [ ] **Step 7: Commit**

```bash
git add finstack/core/src/market_data/hierarchy/ finstack/core/src/market_data/mod.rs \
  finstack/core/tests/market_data/hierarchy.rs finstack/core/tests/market_data.rs
git commit -m "feat: add market data hierarchy core types (HierarchyNode, MarketDataHierarchy, NodePath)"
```

---

### Task 3: Test builder API and serde round-trip

**Files:**
- Modify: `finstack/core/tests/market_data/hierarchy.rs`

- [ ] **Step 1: Write builder and serde tests**

Append to `finstack/core/tests/market_data/hierarchy.rs`:

```rust
#[test]
fn builder_creates_hierarchy_with_slash_paths() {
    let h = MarketDataHierarchy::builder()
        .add_node("Rates/USD/OIS")
        .curve_ids(&["USD-OIS"])
        .add_node("Rates/USD/Forward/SOFR")
        .curve_ids(&["USD-SOFR-3M", "USD-SOFR-6M"])
        .add_node("Credit/US/IG/Financials")
        .tag("sector", "Financials")
        .tag("rating", "A")
        .curve_ids(&["JPM-5Y", "GS-5Y"])
        .build()
        .unwrap();

    // Check structure
    assert_eq!(h.roots().len(), 2); // Rates, Credit
    assert!(h.roots().contains_key("Rates"));
    assert!(h.roots().contains_key("Credit"));

    // Check deep path
    let path: NodePath = vec!["Rates".into(), "USD".into(), "OIS".into()];
    let node = h.get_node(&path).unwrap();
    assert_eq!(node.curve_ids().len(), 1);
    assert_eq!(node.curve_ids()[0], CurveId::from("USD-OIS"));

    // Check tags
    let credit_path: NodePath = vec![
        "Credit".into(),
        "US".into(),
        "IG".into(),
        "Financials".into(),
    ];
    let credit_node = h.get_node(&credit_path).unwrap();
    assert_eq!(credit_node.tags().get("sector").unwrap(), "Financials");
    assert_eq!(credit_node.tags().get("rating").unwrap(), "A");
}

#[test]
fn builder_rejects_duplicate_curve_ids() {
    let result = MarketDataHierarchy::builder()
        .add_node("Rates/USD")
        .curve_ids(&["USD-OIS"])
        .add_node("Credit/US")
        .curve_ids(&["USD-OIS"]) // duplicate!
        .build();

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("USD-OIS"), "Error should mention the duplicate: {err}");
}

#[test]
fn all_curve_ids_collects_entire_tree() {
    let h = MarketDataHierarchy::builder()
        .add_node("Rates/USD/OIS")
        .curve_ids(&["USD-OIS"])
        .add_node("Rates/EUR/ESTR")
        .curve_ids(&["EUR-ESTR"])
        .add_node("Credit/US/IG")
        .curve_ids(&["JPM-5Y"])
        .build()
        .unwrap();

    let all = h.all_curve_ids();
    assert_eq!(all.len(), 3);
}

#[test]
fn path_for_curve_finds_correct_location() {
    let h = MarketDataHierarchy::builder()
        .add_node("Rates/USD/OIS")
        .curve_ids(&["USD-OIS"])
        .add_node("Credit/US/IG/Financials")
        .curve_ids(&["JPM-5Y"])
        .build()
        .unwrap();

    let path = h.path_for_curve(&CurveId::from("JPM-5Y")).unwrap();
    assert_eq!(
        path,
        vec!["Credit", "US", "IG", "Financials"]
    );

    assert!(h.path_for_curve(&CurveId::from("NONEXISTENT")).is_none());
}

#[test]
fn serde_round_trip() {
    let h = MarketDataHierarchy::builder()
        .add_node("Rates/USD/OIS")
        .curve_ids(&["USD-OIS"])
        .add_node("Credit/US/IG/Financials")
        .tag("sector", "Financials")
        .curve_ids(&["JPM-5Y", "GS-5Y"])
        .build()
        .unwrap();

    let json = serde_json::to_string_pretty(&h).unwrap();
    let deserialized: MarketDataHierarchy = serde_json::from_str(&json).unwrap();

    // Verify structure preserved
    assert_eq!(deserialized.roots().len(), h.roots().len());
    assert_eq!(deserialized.all_curve_ids().len(), h.all_curve_ids().len());

    let path: NodePath = vec![
        "Credit".into(),
        "US".into(),
        "IG".into(),
        "Financials".into(),
    ];
    let node = deserialized.get_node(&path).unwrap();
    assert_eq!(node.tags().get("sector").unwrap(), "Financials");
    assert_eq!(node.curve_ids().len(), 2);
}

#[test]
fn insert_and_remove_curve() {
    let mut h = MarketDataHierarchy::builder()
        .add_node("Rates/USD")
        .curve_ids(&["USD-OIS"])
        .build()
        .unwrap();

    h.insert_curve("Rates/USD", "USD-SOFR-3M");
    let path: NodePath = vec!["Rates".into(), "USD".into()];
    assert_eq!(h.get_node(&path).unwrap().curve_ids().len(), 2);

    assert!(h.remove_curve(&CurveId::from("USD-OIS")));
    assert_eq!(h.get_node(&path).unwrap().curve_ids().len(), 1);
    assert!(!h.remove_curve(&CurveId::from("NONEXISTENT")));
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p finstack-core --test market_data hierarchy`
Expected: All tests PASS

- [ ] **Step 3: Commit**

```bash
git add finstack/core/tests/market_data/hierarchy.rs
git commit -m "test: add builder, serde round-trip, and mutation tests for hierarchy"
```

---

## Chunk 2: Resolution Engine & Completeness

### Task 4: Test and implement resolution engine

The resolution engine resolves hierarchy targets (path + optional tag filter) to sets of `CurveId`s.

**Files:**
- Modify: `finstack/core/tests/market_data/hierarchy.rs`
- Modify: `finstack/core/src/market_data/hierarchy/resolution.rs` (already has types, add resolve tests)

- [ ] **Step 1: Write resolution tests**

Append to `finstack/core/tests/market_data/hierarchy.rs`:

```rust
use finstack_core::market_data::hierarchy::resolution::{
    HierarchyTarget, ResolutionMode, TagFilter, TagPredicate,
};

#[test]
fn resolve_target_collects_subtree_curves() {
    let h = MarketDataHierarchy::builder()
        .add_node("Credit/US/IG/Financials")
        .curve_ids(&["JPM-5Y", "GS-5Y"])
        .add_node("Credit/US/IG/Technology")
        .curve_ids(&["AAPL-5Y"])
        .add_node("Credit/US/HY/Energy")
        .curve_ids(&["OXY-5Y"])
        .add_node("Credit/EU/IG")
        .curve_ids(&["SIE-5Y"])
        .build()
        .unwrap();

    // Target all Credit curves
    let target = HierarchyTarget {
        path: vec!["Credit".into()],
        tag_filter: None,
    };
    let ids = h.resolve(&target);
    assert_eq!(ids.len(), 5);

    // Target US IG only
    let target = HierarchyTarget {
        path: vec!["Credit".into(), "US".into(), "IG".into()],
        tag_filter: None,
    };
    let ids = h.resolve(&target);
    assert_eq!(ids.len(), 3); // JPM, GS, AAPL

    // Target non-existent path
    let target = HierarchyTarget {
        path: vec!["Nonexistent".into()],
        tag_filter: None,
    };
    assert!(h.resolve(&target).is_empty());
}

#[test]
fn resolve_with_tag_filter() {
    let h = MarketDataHierarchy::builder()
        .add_node("Credit/US/IG/Financials")
        .tag("sector", "Financials")
        .curve_ids(&["JPM-5Y", "GS-5Y"])
        .add_node("Credit/US/IG/Technology")
        .tag("sector", "Technology")
        .curve_ids(&["AAPL-5Y"])
        .add_node("Credit/US/HY/Energy")
        .tag("sector", "Energy")
        .curve_ids(&["OXY-5Y"])
        .build()
        .unwrap();

    // Filter for Energy sector under all of Credit
    let target = HierarchyTarget {
        path: vec!["Credit".into()],
        tag_filter: Some(TagFilter {
            predicates: vec![TagPredicate::Equals {
                key: "sector".into(),
                value: "Energy".into(),
            }],
        }),
    };
    let ids = h.resolve(&target);
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0], CurveId::from("OXY-5Y"));
}

#[test]
fn query_by_tags_searches_entire_hierarchy() {
    let h = MarketDataHierarchy::builder()
        .add_node("Credit/US/IG/Financials")
        .tag("sector", "Financials")
        .curve_ids(&["JPM-5Y"])
        .add_node("Equity/Prices/Financials")
        .tag("sector", "Financials")
        .curve_ids(&["JPM-SPOT"])
        .add_node("Equity/Prices/Technology")
        .tag("sector", "Technology")
        .curve_ids(&["AAPL-SPOT"])
        .build()
        .unwrap();

    let filter = TagFilter {
        predicates: vec![TagPredicate::Equals {
            key: "sector".into(),
            value: "Financials".into(),
        }],
    };
    let ids = h.query_by_tags(&filter);
    assert_eq!(ids.len(), 2); // JPM-5Y and JPM-SPOT
}

#[test]
fn tag_predicate_in_matches_any_value() {
    let h = MarketDataHierarchy::builder()
        .add_node("FX/Spot/DM")
        .tag("classification", "developed")
        .curve_ids(&["EURUSD-SPOT"])
        .add_node("FX/Spot/EM")
        .tag("classification", "emerging")
        .curve_ids(&["BRLUSD-SPOT"])
        .add_node("FX/Spot/Frontier")
        .tag("classification", "frontier")
        .curve_ids(&["NGNEUR-SPOT"])
        .build()
        .unwrap();

    let target = HierarchyTarget {
        path: vec!["FX".into()],
        tag_filter: Some(TagFilter {
            predicates: vec![TagPredicate::In {
                key: "classification".into(),
                values: vec!["developed".into(), "emerging".into()],
            }],
        }),
    };
    let ids = h.resolve(&target);
    assert_eq!(ids.len(), 2); // DM + EM, not Frontier
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p finstack-core --test market_data hierarchy`
Expected: All tests PASS (resolution logic was already implemented in Step 5 of Task 2)

- [ ] **Step 3: Commit**

```bash
git add finstack/core/tests/market_data/hierarchy.rs
git commit -m "test: add resolution engine tests for hierarchy targeting and tag filtering"
```

---

### Task 5: Implement and test completeness tracking

**Files:**
- Modify: `finstack/core/src/market_data/hierarchy/completeness.rs` (add implementation)
- Modify: `finstack/core/src/market_data/hierarchy/mod.rs` (add completeness method to `MarketDataHierarchy`)
- Modify: `finstack/core/src/market_data/context/mod.rs` (add `completeness_report` method)
- Modify: `finstack/core/tests/market_data/hierarchy.rs`

- [ ] **Step 1: Write the failing test**

Append to `finstack/core/tests/market_data/hierarchy.rs`:

```rust
use finstack_core::market_data::hierarchy::CompletenessReport;

#[test]
fn completeness_report_returns_none_without_hierarchy() {
    use finstack_core::market_data::context::MarketContext;
    let market = MarketContext::new();
    assert!(market.completeness_report().is_none());
}

#[test]
fn completeness_report_detects_missing_and_unclassified() {
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use time::Month;

    let base = finstack_core::dates::Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let h = MarketDataHierarchy::builder()
        .add_node("Rates/USD/OIS")
        .curve_ids(&["USD-OIS"])
        .add_node("Rates/EUR/ESTR")
        .curve_ids(&["EUR-ESTR"])  // will be missing from MarketContext
        .build()
        .unwrap();

    // Build MarketContext with only USD-OIS (EUR-ESTR is missing)
    // Also add an unclassified curve not in hierarchy
    let usd_curve = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .knots([(0.0, 1.0), (1.0, 0.95)])
        .build()
        .unwrap();
    let extra_curve = DiscountCurve::builder("GBP-SONIA")
        .base_date(base)
        .knots([(0.0, 1.0), (1.0, 0.96)])
        .build()
        .unwrap();

    let mut market = MarketContext::new()
        .insert(usd_curve)
        .insert(extra_curve);
    market.set_hierarchy(h);

    let report = market.completeness_report().unwrap();

    // EUR-ESTR is declared but missing
    assert_eq!(report.missing.len(), 1);
    assert_eq!(report.missing[0].1, CurveId::from("EUR-ESTR"));

    // GBP-SONIA is present but not in hierarchy
    assert_eq!(report.unclassified.len(), 1);
    assert_eq!(report.unclassified[0], CurveId::from("GBP-SONIA"));

    // Coverage: Rates root has 2 expected, 1 present = 50%
    assert!(!report.coverage.is_empty());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p finstack-core --test market_data hierarchy::completeness -- --no-run 2>&1 | head -20`
Expected: FAIL — `set_hierarchy` and `completeness_report` not found on `MarketContext`

- [ ] **Step 3: Add `hierarchy` field and methods to `MarketContext`**

In `finstack/core/src/market_data/context/mod.rs`, add the import and field:

After the existing imports (line ~96), add:

```rust
use super::hierarchy::{CompletenessReport, MarketDataHierarchy, SubtreeCoverage};
```

Add field to `MarketContext` struct (after `collateral`):

```rust
    /// Optional market data hierarchy for organizational grouping.
    hierarchy: Option<MarketDataHierarchy>,
```

Add methods to `impl MarketContext`:

```rust
    /// Get the attached hierarchy, if any.
    pub fn hierarchy(&self) -> Option<&MarketDataHierarchy> {
        self.hierarchy.as_ref()
    }

    /// Attach a market data hierarchy.
    pub fn set_hierarchy(&mut self, h: MarketDataHierarchy) {
        self.hierarchy = Some(h);
    }

    /// Generate a completeness report comparing hierarchy declarations against
    /// all `CurveId`-keyed data stores. Returns `None` if no hierarchy is attached.
    pub fn completeness_report(&self) -> Option<CompletenessReport> {
        let hierarchy = self.hierarchy.as_ref()?;

        // Collect all CurveIds present in any store
        let mut present: crate::collections::HashSet<CurveId> =
            crate::collections::HashSet::default();
        present.extend(self.curves.keys().cloned());
        present.extend(self.surfaces.keys().cloned());
        present.extend(self.prices.keys().cloned());
        present.extend(self.series.keys().cloned());
        present.extend(self.inflation_indices.keys().cloned());
        present.extend(self.credit_indices.keys().cloned());
        present.extend(self.dividends.keys().cloned());
        present.extend(self.fx_delta_vol_surfaces.keys().cloned());

        // Find missing: declared in hierarchy but not in any store
        let declared = hierarchy.all_curve_ids();
        let declared_set: crate::collections::HashSet<CurveId> =
            declared.iter().cloned().collect();

        let mut missing = Vec::new();
        for id in &declared {
            if !present.contains(id) {
                if let Some(path) = hierarchy.path_for_curve(id) {
                    missing.push((path, id.clone()));
                }
            }
        }

        // Find unclassified: present in stores but not in hierarchy
        let unclassified: Vec<CurveId> = present
            .iter()
            .filter(|id| !declared_set.contains(id))
            .cloned()
            .collect();

        // Coverage per root subtree
        let mut coverage = Vec::new();
        for (name, root) in hierarchy.roots() {
            let subtree_ids = root.all_curve_ids();
            let total_expected = subtree_ids.len();
            let total_present = subtree_ids.iter().filter(|id| present.contains(id)).count();
            let percent = if total_expected == 0 {
                100.0
            } else {
                (total_present as f64 / total_expected as f64) * 100.0
            };
            coverage.push(SubtreeCoverage {
                path: vec![name.clone()],
                total_expected,
                total_present,
                percent,
            });
        }

        Some(CompletenessReport {
            missing,
            unclassified,
            coverage,
        })
    }
```

Also update the `Debug` impl to include hierarchy, and update `Default` — the `hierarchy` field defaults to `None` which is already handled by `Default` derive. Add hierarchy to the Debug output:

In the existing `fmt::Debug` impl, add before `.finish()`:

```rust
            .field("hierarchy", &self.hierarchy.is_some())
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p finstack-core --test market_data hierarchy`
Expected: All tests PASS

- [ ] **Step 5: Run full finstack-core test suite to check nothing is broken**

Run: `cargo test -p finstack-core`
Expected: All existing tests PASS

- [ ] **Step 6: Commit**

```bash
git add finstack/core/src/market_data/context/mod.rs \
  finstack/core/src/market_data/hierarchy/completeness.rs \
  finstack/core/tests/market_data/hierarchy.rs
git commit -m "feat: add completeness tracking and MarketContext hierarchy integration"
```

---

## Chunk 3: Scenario Engine Integration

### Task 6: Add `HierarchyTarget` to scenario types

The scenario spec needs a `CurveTarget` enum so operations can target either a direct `CurveId` or a hierarchy path. This is backwards compatible — existing JSON with `curve_id` still deserializes.

**Files:**
- Modify: `finstack/scenarios/src/spec.rs`
- Create: `finstack/scenarios/tests/hierarchy_targeting.rs` (or add to existing test file)

- [ ] **Step 1: Write the failing test**

Create `finstack/scenarios/tests/hierarchy_targeting.rs`:

```rust
//! Tests for hierarchy-targeted scenario operations.

use finstack_scenarios::{OperationSpec, CurveKind, ScenarioSpec};

/// Existing direct-targeted JSON must still deserialize.
#[test]
fn existing_direct_target_json_round_trips() {
    let json = r#"{
        "id": "test",
        "operations": [
            {
                "kind": "curve_parallel_bp",
                "curve_kind": "discount",
                "curve_id": "USD-OIS",
                "bp": 50.0
            }
        ]
    }"#;
    let spec: ScenarioSpec = serde_json::from_str(json).unwrap();
    assert_eq!(spec.operations.len(), 1);
    match &spec.operations[0] {
        OperationSpec::CurveParallelBp { curve_id, bp, .. } => {
            assert_eq!(curve_id, "USD-OIS");
            assert!((bp - 50.0).abs() < f64::EPSILON);
        }
        other => panic!("Expected CurveParallelBp, got: {:?}", other),
    }
}
```

Register in `finstack/scenarios/tests/mod.rs` or as a standalone test file. Check the test structure:

- [ ] **Step 2: Verify existing tests still pass before making changes**

Run: `cargo test -p finstack-scenarios`
Expected: All existing tests PASS

- [ ] **Step 3: Add `resolution_mode` field to `ScenarioSpec`**

In `finstack/scenarios/src/spec.rs`, add the import:

```rust
use finstack_core::market_data::hierarchy::resolution::ResolutionMode;
```

Add field to `ScenarioSpec` (after `priority`):

```rust
    /// Resolution mode for hierarchy-targeted operations.
    /// Only relevant when operations use hierarchy targeting.
    /// Default: `MostSpecificWins`.
    #[serde(default)]
    pub resolution_mode: ResolutionMode,
```

Note: `ScenarioSpec` uses `#[serde(deny_unknown_fields)]`. The new field has `#[serde(default)]` so existing JSON without it still deserializes. New JSON with `resolution_mode` also works.

- [ ] **Step 4: Run all scenario tests to verify backwards compatibility**

Run: `cargo test -p finstack-scenarios`
Expected: All existing tests PASS (the new field defaults to `MostSpecificWins`)

- [ ] **Step 5: Commit**

```bash
git add finstack/scenarios/src/spec.rs finstack/scenarios/tests/
git commit -m "feat: add resolution_mode to ScenarioSpec for hierarchy-targeted operations"
```

---

### Task 7: Add hierarchy resolution to the scenario engine

The engine must resolve hierarchy-targeted operations to per-curve operations before dispatching to adapters. This is a **pre-processing step** that runs before the existing adapter loop.

**Files:**
- Modify: `finstack/scenarios/src/engine.rs`
- Modify: `finstack/scenarios/src/spec.rs` (add `HierarchyTarget` re-export)

- [ ] **Step 1: Write the failing integration test**

Add to `finstack/scenarios/tests/hierarchy_targeting.rs`:

```rust
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::hierarchy::MarketDataHierarchy;
use finstack_core::market_data::hierarchy::resolution::ResolutionMode;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_scenarios::{ScenarioEngine, ExecutionContext};
use finstack_statements::FinancialModelSpec;
use time::macros::date;

#[test]
fn engine_works_with_resolution_mode_field() {
    // Verify that adding resolution_mode to ScenarioSpec doesn't break existing engine flow.
    let h = MarketDataHierarchy::builder()
        .add_node("Rates/USD/OIS")
        .curve_ids(&["USD-OIS"])
        .add_node("Rates/USD/Treasury")
        .curve_ids(&["USD-TSY"])
        .build()
        .unwrap();

    let base = date!(2025 - 01 - 01);
    let ois = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.78)])
        .build()
        .unwrap();
    let tsy = DiscountCurve::builder("USD-TSY")
        .base_date(base)
        .knots([(0.0, 1.0), (1.0, 0.96), (5.0, 0.80)])
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert(ois).insert(tsy);
    market.set_hierarchy(h);

    // Bump all Rates/USD curves by +50bp
    let scenario = ScenarioSpec {
        id: "test_hierarchy".into(),
        name: None,
        description: None,
        operations: vec![
            OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Discount,
                curve_id: "USD-OIS".into(),
                bp: 50.0,
            },
        ],
        priority: 0,
        resolution_mode: ResolutionMode::default(),
    };

    let mut model = FinancialModelSpec::new("test", vec![]);
    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: base,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 1);
}
```

This first test just ensures the new `resolution_mode` field doesn't break existing engine flow. The actual hierarchy-expansion feature will come in a follow-up task (Task 8) since it requires adding new `OperationSpec` variants or a pre-processing step.

- [ ] **Step 2: Run the test**

Run: `cargo test -p finstack-scenarios hierarchy`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add finstack/scenarios/tests/hierarchy_targeting.rs
git commit -m "test: verify engine works with resolution_mode field on ScenarioSpec"
```

---

### Task 8: Add hierarchy-targeted operation variants

This is the key integration: OperationSpec variants that accept a `HierarchyTarget` instead of a direct `curve_id`. The engine pre-resolves these to individual curve operations.

**Files:**
- Modify: `finstack/scenarios/src/spec.rs` (add new variants)
- Modify: `finstack/scenarios/src/engine.rs` (add pre-resolution step)
- Modify: `finstack/scenarios/src/adapters/traits.rs` (if needed)
- Modify: `finstack/scenarios/tests/hierarchy_targeting.rs`

- [ ] **Step 1: Write the failing test**

Add to `finstack/scenarios/tests/hierarchy_targeting.rs`:

```rust
use finstack_core::market_data::hierarchy::resolution::HierarchyTarget;

#[test]
fn hierarchy_curve_parallel_bp_resolves_to_individual_bumps() {
    let h = MarketDataHierarchy::builder()
        .add_node("Rates/USD/OIS")
        .curve_ids(&["USD-OIS"])
        .add_node("Rates/USD/Treasury")
        .curve_ids(&["USD-TSY"])
        .build()
        .unwrap();

    let base = date!(2025 - 01 - 01);
    let ois = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.78)])
        .build()
        .unwrap();
    let tsy = DiscountCurve::builder("USD-TSY")
        .base_date(base)
        .knots([(0.0, 1.0), (1.0, 0.96), (5.0, 0.80)])
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert(ois).insert(tsy);
    market.set_hierarchy(h);

    // Use hierarchy-targeted operation
    let scenario = ScenarioSpec {
        id: "hierarchy_bump".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::HierarchyCurveParallelBp {
            curve_kind: CurveKind::Discount,
            target: HierarchyTarget {
                path: vec!["Rates".into(), "USD".into()],
                tag_filter: None,
            },
            bp: 50.0,
        }],
        priority: 0,
        resolution_mode: ResolutionMode::default(),
    };

    let mut model = FinancialModelSpec::new("test", vec![]);
    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: base,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    // Should have applied 2 bumps (one per resolved curve)
    assert_eq!(report.operations_applied, 2);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p finstack-scenarios hierarchy_curve_parallel -- --no-run 2>&1 | head -20`
Expected: FAIL — `HierarchyCurveParallelBp` variant not found

- [ ] **Step 3: Add hierarchy-targeted variants to `OperationSpec`**

In `finstack/scenarios/src/spec.rs`, add import at top:

```rust
use finstack_core::market_data::hierarchy::resolution::HierarchyTarget;
```

Re-export for test convenience:

```rust
/// Re-export [`HierarchyTarget`] for hierarchy-targeted operations.
pub use finstack_core::market_data::hierarchy::resolution::HierarchyTarget;
```

Add new variants to `OperationSpec` enum (before the `TimeRollForward` variant). These mirror the existing direct-targeted variants but accept `HierarchyTarget`:

```rust
    /// Hierarchy-targeted parallel curve shift (resolved to individual curves at execution).
    HierarchyCurveParallelBp {
        /// Type of curve (Discount, Forward, Hazard, etc.).
        curve_kind: CurveKind,
        /// Hierarchy target to resolve to curves.
        target: HierarchyTarget,
        /// Basis point shift (additive).
        bp: f64,
    },

    /// Hierarchy-targeted vol surface parallel shift.
    HierarchyVolSurfaceParallelPct {
        /// Type of volatility surface.
        surface_kind: VolSurfaceKind,
        /// Hierarchy target to resolve to surfaces.
        target: HierarchyTarget,
        /// Percentage change in volatility.
        pct: f64,
    },

    /// Hierarchy-targeted equity price shift.
    HierarchyEquityPricePct {
        /// Hierarchy target to resolve to equity IDs.
        target: HierarchyTarget,
        /// Percentage price change.
        pct: f64,
    },

    /// Hierarchy-targeted base correlation parallel shift.
    HierarchyBaseCorrParallelPts {
        /// Hierarchy target to resolve to surfaces.
        target: HierarchyTarget,
        /// Absolute shift in correlation points.
        points: f64,
    },
```

Add validation for the new variants in `OperationSpec::validate()`:

```rust
            OperationSpec::HierarchyCurveParallelBp { target, bp, .. } => {
                if target.path.is_empty() {
                    return Err(crate::error::Error::Validation(
                        "Hierarchy target path cannot be empty".into(),
                    ));
                }
                check_finite(*bp, "bp")?;
            }
            OperationSpec::HierarchyVolSurfaceParallelPct { target, pct, .. } => {
                if target.path.is_empty() {
                    return Err(crate::error::Error::Validation(
                        "Hierarchy target path cannot be empty".into(),
                    ));
                }
                check_finite(*pct, "pct")?;
                check_pct_floor(*pct, "pct")?;
            }
            OperationSpec::HierarchyEquityPricePct { target, pct } => {
                if target.path.is_empty() {
                    return Err(crate::error::Error::Validation(
                        "Hierarchy target path cannot be empty".into(),
                    ));
                }
                check_finite(*pct, "pct")?;
                check_pct_floor(*pct, "pct")?;
            }
            OperationSpec::HierarchyBaseCorrParallelPts { target, points } => {
                if target.path.is_empty() {
                    return Err(crate::error::Error::Validation(
                        "Hierarchy target path cannot be empty".into(),
                    ));
                }
                check_finite(*points, "points")?;
            }
```

- [ ] **Step 4: Add hierarchy resolution pre-processing to the engine**

In `finstack/scenarios/src/engine.rs`, add a function that expands hierarchy-targeted operations into direct-targeted operations:

```rust
use crate::spec::HierarchyTarget;
use finstack_core::market_data::hierarchy::resolution::ResolutionMode;
use finstack_core::types::CurveId;

/// A hierarchy-sourced operation: the target path length and the direct operation it expands to.
struct HierarchyExpansion {
    /// Depth of the hierarchy target path (longer = more specific).
    path_depth: usize,
    /// The direct operation this expands to (already resolved to a single CurveId).
    operation: OperationSpec,
    /// The CurveId this operation targets.
    curve_id: CurveId,
}

/// Expand hierarchy-targeted operations into direct-targeted operations.
///
/// - `Cumulative`: All matching hierarchy operations expand independently. A curve
///   under multiple targeted subtrees gets all shocks (they stack additively).
/// - `MostSpecificWins`: For each curve, only the deepest (longest path) hierarchy
///   operation applies. If two operations at the same depth target the same curve,
///   both apply (tie-breaking is additive).
fn expand_hierarchy_operations(
    operations: &[OperationSpec],
    market: &finstack_core::market_data::context::MarketContext,
    mode: ResolutionMode,
) -> Vec<OperationSpec> {
    let hierarchy = match market.hierarchy() {
        Some(h) => h,
        None => {
            // No hierarchy attached — pass operations through unchanged.
            // Hierarchy ops will produce "not supported" warnings in the adapter loop.
            return operations.to_vec();
        }
    };

    let mut non_hierarchy_ops = Vec::new();
    let mut hierarchy_expansions: Vec<HierarchyExpansion> = Vec::new();

    for op in operations {
        match op {
            OperationSpec::HierarchyCurveParallelBp {
                curve_kind,
                target,
                bp,
            } => {
                let ids = hierarchy.resolve(target);
                for id in ids {
                    hierarchy_expansions.push(HierarchyExpansion {
                        path_depth: target.path.len(),
                        curve_id: id.clone(),
                        operation: OperationSpec::CurveParallelBp {
                            curve_kind: *curve_kind,
                            curve_id: id.as_str().to_string(),
                            bp: *bp,
                        },
                    });
                }
            }
            OperationSpec::HierarchyVolSurfaceParallelPct {
                surface_kind,
                target,
                pct,
            } => {
                let ids = hierarchy.resolve(target);
                for id in ids {
                    hierarchy_expansions.push(HierarchyExpansion {
                        path_depth: target.path.len(),
                        curve_id: id.clone(),
                        operation: OperationSpec::VolSurfaceParallelPct {
                            surface_kind: *surface_kind,
                            surface_id: id.as_str().to_string(),
                            pct: *pct,
                        },
                    });
                }
            }
            OperationSpec::HierarchyEquityPricePct { target, pct } => {
                let ids = hierarchy.resolve(target);
                for id in &ids {
                    hierarchy_expansions.push(HierarchyExpansion {
                        path_depth: target.path.len(),
                        curve_id: id.clone(),
                        operation: OperationSpec::EquityPricePct {
                            ids: vec![id.as_str().to_string()],
                            pct: *pct,
                        },
                    });
                }
            }
            OperationSpec::HierarchyBaseCorrParallelPts { target, points } => {
                let ids = hierarchy.resolve(target);
                for id in ids {
                    hierarchy_expansions.push(HierarchyExpansion {
                        path_depth: target.path.len(),
                        curve_id: id.clone(),
                        operation: OperationSpec::BaseCorrParallelPts {
                            surface_id: id.as_str().to_string(),
                            points: *points,
                        },
                    });
                }
            }
            other => non_hierarchy_ops.push(other.clone()),
        }
    }

    // Apply resolution mode
    let resolved_hierarchy_ops = match mode {
        ResolutionMode::Cumulative => {
            // All expansions pass through — shocks stack additively.
            hierarchy_expansions
                .into_iter()
                .map(|e| e.operation)
                .collect()
        }
        ResolutionMode::MostSpecificWins => {
            // For each curve, keep only operations at the maximum path depth.
            // Group by CurveId, find max depth, keep only ops at that depth.
            use std::collections::HashMap;
            let mut max_depth: HashMap<CurveId, usize> = HashMap::new();
            for exp in &hierarchy_expansions {
                let entry = max_depth.entry(exp.curve_id.clone()).or_insert(0);
                *entry = (*entry).max(exp.path_depth);
            }
            hierarchy_expansions
                .into_iter()
                .filter(|exp| {
                    max_depth
                        .get(&exp.curve_id)
                        .map_or(false, |&max| exp.path_depth == max)
                })
                .map(|e| e.operation)
                .collect()
        }
    };

    // Non-hierarchy ops first (preserves original ordering), then resolved hierarchy ops.
    // Actually, we should interleave to preserve original operation order.
    // Simpler: non-hierarchy ops maintain their order, hierarchy ops are appended.
    non_hierarchy_ops.extend(resolved_hierarchy_ops);
    non_hierarchy_ops
}
```

Then in `ScenarioEngine::apply`, add the expansion step at the very beginning (before Phase 0). Replace:

```rust
        // Phase 0: Time Roll Forward
        for op in &spec.operations {
```

With:

```rust
        // Phase -1: Expand hierarchy-targeted operations to direct operations
        let expanded_ops = expand_hierarchy_operations(
            &spec.operations,
            ctx.market,
            spec.resolution_mode,
        );

        // Phase 0: Time Roll Forward
        for op in &expanded_ops {
```

Update **only the Phase 1 loop** (`for op in &spec.operations` at the second occurrence) to use `&expanded_ops`. Phase 0 (TimeRollForward) should also use `&expanded_ops` since it's the same list. Both loops iterate `expanded_ops`.

- [ ] **Step 5: Run the hierarchy targeting test**

Run: `cargo test -p finstack-scenarios hierarchy_curve_parallel`
Expected: PASS — 2 operations applied (one per resolved curve)

- [ ] **Step 6: Run the full scenario test suite**

Run: `cargo test -p finstack-scenarios`
Expected: All tests PASS

- [ ] **Step 7: Commit**

```bash
git add finstack/scenarios/src/spec.rs finstack/scenarios/src/engine.rs \
  finstack/scenarios/tests/hierarchy_targeting.rs
git commit -m "feat: add hierarchy-targeted scenario operations with engine pre-resolution"
```

---

### Task 9: Test cumulative resolution mode

**Files:**
- Modify: `finstack/scenarios/tests/hierarchy_targeting.rs`

- [ ] **Step 1: Write cumulative resolution test**

This tests the scenario from the design spec where operations at multiple hierarchy levels combine cumulatively.

```rust
#[test]
fn cumulative_mode_stacks_shocks_down_tree() {
    // Build a hierarchy: Credit -> US -> HY -> Energy
    let h = MarketDataHierarchy::builder()
        .add_node("Credit/US/IG/Financials")
        .curve_ids(&["JPM-5Y"])
        .add_node("Credit/US/HY/Energy")
        .curve_ids(&["OXY-5Y"])
        .build()
        .unwrap();

    let base = date!(2025 - 01 - 01);
    // Create hazard curves (use discount as stand-in for simplicity)
    let jpm = DiscountCurve::builder("JPM-5Y")
        .base_date(base)
        .knots([(0.0, 1.0), (5.0, 0.90)])
        .build()
        .unwrap();
    let oxy = DiscountCurve::builder("OXY-5Y")
        .base_date(base)
        .knots([(0.0, 1.0), (5.0, 0.85)])
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert(jpm).insert(oxy);
    market.set_hierarchy(h);

    // Two operations: +50bp on all Credit, +100bp on US/HY
    // With cumulative mode, OXY-5Y should get both (+150bp total)
    // JPM-5Y should get only +50bp (not under HY)
    let scenario = ScenarioSpec {
        id: "cumulative_test".into(),
        name: None,
        description: None,
        operations: vec![
            OperationSpec::HierarchyCurveParallelBp {
                curve_kind: CurveKind::Discount,
                target: HierarchyTarget {
                    path: vec!["Credit".into()],
                    tag_filter: None,
                },
                bp: 50.0,
            },
            OperationSpec::HierarchyCurveParallelBp {
                curve_kind: CurveKind::Discount,
                target: HierarchyTarget {
                    path: vec!["Credit".into(), "US".into(), "HY".into()],
                    tag_filter: None,
                },
                bp: 100.0,
            },
        ],
        priority: 0,
        resolution_mode: ResolutionMode::Cumulative,
    };

    let mut model = FinancialModelSpec::new("test", vec![]);
    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: base,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    // Credit→all: 2 curves (JPM, OXY) + HY: 1 curve (OXY) = 3 operations
    assert_eq!(report.operations_applied, 3);
}
```

Note: In cumulative mode, the same curve can appear in multiple expanded operations (shocks stack additively). In most-specific-wins mode, the `expand_hierarchy_operations` function groups by CurveId and keeps only the deepest-path operations.

- [ ] **Step 2: Write most-specific-wins test**

Add this test to verify that `MostSpecificWins` correctly deduplicates:

```rust
#[test]
fn most_specific_wins_keeps_only_deepest_shock() {
    // Same hierarchy and curves as cumulative test
    let h = MarketDataHierarchy::builder()
        .add_node("Credit/US/IG/Financials")
        .curve_ids(&["JPM-5Y"])
        .add_node("Credit/US/HY/Energy")
        .curve_ids(&["OXY-5Y"])
        .build()
        .unwrap();

    let base = date!(2025 - 01 - 01);
    let jpm = DiscountCurve::builder("JPM-5Y")
        .base_date(base)
        .knots([(0.0, 1.0), (5.0, 0.90)])
        .build()
        .unwrap();
    let oxy = DiscountCurve::builder("OXY-5Y")
        .base_date(base)
        .knots([(0.0, 1.0), (5.0, 0.85)])
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert(jpm).insert(oxy);
    market.set_hierarchy(h);

    // +50bp on all Credit (depth 1), +100bp on US/HY (depth 3)
    // With MostSpecificWins:
    //   OXY-5Y matched by both Credit (depth 1) and Credit/US/HY (depth 3)
    //     → keeps only depth-3 shock (+100bp), drops depth-1 shock
    //   JPM-5Y matched only by Credit (depth 1) → keeps +50bp
    // Total: 2 operations (one per curve)
    let scenario = ScenarioSpec {
        id: "msw_test".into(),
        name: None,
        description: None,
        operations: vec![
            OperationSpec::HierarchyCurveParallelBp {
                curve_kind: CurveKind::Discount,
                target: HierarchyTarget {
                    path: vec!["Credit".into()],
                    tag_filter: None,
                },
                bp: 50.0,
            },
            OperationSpec::HierarchyCurveParallelBp {
                curve_kind: CurveKind::Discount,
                target: HierarchyTarget {
                    path: vec!["Credit".into(), "US".into(), "HY".into()],
                    tag_filter: None,
                },
                bp: 100.0,
            },
        ],
        priority: 0,
        resolution_mode: ResolutionMode::MostSpecificWins,
    };

    let mut model = FinancialModelSpec::new("test", vec![]);
    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: base,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    // 2 operations: JPM-5Y gets +50bp (only match), OXY-5Y gets +100bp (deepest wins)
    assert_eq!(report.operations_applied, 2);
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p finstack-scenarios hierarchy`
Expected: All hierarchy tests PASS

- [ ] **Step 4: Commit**

```bash
git add finstack/scenarios/tests/hierarchy_targeting.rs finstack/scenarios/src/engine.rs
git commit -m "test: add cumulative resolution mode integration test"
```

---

### Task 10: Serde round-trip tests for hierarchy-targeted scenarios

Ensure that JSON serialization of hierarchy-targeted scenarios works correctly.

**Files:**
- Modify: `finstack/scenarios/tests/hierarchy_targeting.rs`

- [ ] **Step 1: Write serde tests**

```rust
#[test]
fn hierarchy_operation_json_round_trip() {
    let op = OperationSpec::HierarchyCurveParallelBp {
        curve_kind: CurveKind::Discount,
        target: HierarchyTarget {
            path: vec!["Credit".into(), "US".into(), "IG".into()],
            tag_filter: Some(finstack_core::market_data::hierarchy::resolution::TagFilter {
                predicates: vec![
                    finstack_core::market_data::hierarchy::resolution::TagPredicate::Equals {
                        key: "sector".into(),
                        value: "Financials".into(),
                    },
                ],
            }),
        },
        bp: 50.0,
    };

    let json = serde_json::to_string_pretty(&op).unwrap();
    let deserialized: OperationSpec = serde_json::from_str(&json).unwrap();

    match deserialized {
        OperationSpec::HierarchyCurveParallelBp {
            curve_kind,
            target,
            bp,
        } => {
            assert_eq!(curve_kind, CurveKind::Discount);
            assert_eq!(target.path, vec!["Credit", "US", "IG"]);
            assert!(target.tag_filter.is_some());
            assert!((bp - 50.0).abs() < f64::EPSILON);
        }
        other => panic!("Expected HierarchyCurveParallelBp, got: {:?}", other),
    }
}

#[test]
fn scenario_with_resolution_mode_json_round_trip() {
    let scenario = ScenarioSpec {
        id: "hierarchy_test".into(),
        name: Some("Hierarchy Test".into()),
        description: None,
        operations: vec![OperationSpec::HierarchyCurveParallelBp {
            curve_kind: CurveKind::Discount,
            target: HierarchyTarget {
                path: vec!["Rates".into()],
                tag_filter: None,
            },
            bp: 25.0,
        }],
        priority: 0,
        resolution_mode: ResolutionMode::Cumulative,
    };

    let json = serde_json::to_string_pretty(&scenario).unwrap();
    assert!(json.contains("cumulative"));

    let deserialized: ScenarioSpec = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.resolution_mode, ResolutionMode::Cumulative);
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p finstack-scenarios hierarchy`
Expected: All PASS

- [ ] **Step 3: Commit**

```bash
git add finstack/scenarios/tests/hierarchy_targeting.rs
git commit -m "test: add serde round-trip tests for hierarchy-targeted scenario operations"
```

---

### Task 11: Run full workspace test suite and fix any issues

**Files:** None new — fixing anything that broke.

- [ ] **Step 1: Run full workspace tests**

Run: `cargo test --workspace`
Expected: All tests PASS

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: No warnings

- [ ] **Step 3: Fix any issues found**

If any tests fail or clippy warnings appear, fix them before proceeding.

- [ ] **Step 4: Final commit**

```bash
git commit -m "chore: fix any clippy warnings from hierarchy implementation"
```

(Only if there were fixes needed)

---

## Summary of Changes

| Crate | Files Changed | What |
|-------|--------------|------|
| `finstack-core` | `Cargo.toml` | Add `indexmap` dependency |
| `finstack-core` | `src/market_data/mod.rs` | Add `pub mod hierarchy` |
| `finstack-core` | `src/market_data/hierarchy/mod.rs` | `MarketDataHierarchy`, `HierarchyNode`, `NodePath`, tree ops |
| `finstack-core` | `src/market_data/hierarchy/builder.rs` | `HierarchyBuilder` with `/`-path syntax |
| `finstack-core` | `src/market_data/hierarchy/resolution.rs` | `ResolutionMode`, `TagFilter`, `TagPredicate`, `HierarchyTarget`, resolve logic |
| `finstack-core` | `src/market_data/hierarchy/completeness.rs` | `CompletenessReport`, `SubtreeCoverage` |
| `finstack-core` | `src/market_data/context/mod.rs` | `hierarchy` field, `set_hierarchy()`, `completeness_report()` |
| `finstack-core` | `tests/market_data/hierarchy.rs` | All hierarchy unit + integration tests |
| `finstack-scenarios` | `src/spec.rs` | `resolution_mode` on `ScenarioSpec`, `Hierarchy*` operation variants |
| `finstack-scenarios` | `src/engine.rs` | `expand_hierarchy_operations()` pre-processing step |
| `finstack-scenarios` | `tests/hierarchy_targeting.rs` | Scenario integration tests |

## Not In Scope (Future Tasks)

- **FX hierarchy targeting**: `MarketFxPct` operates on currency pairs, not `CurveId`s. Hierarchy targeting for FX requires a separate mapping design.
- **Bucket-level hierarchy variants**: `HierarchyCurveNodeBp`, `HierarchyBaseCorrBucketPts`, `HierarchyVolSurfaceBucketPct` — these mirror the existing bucket-level operations but with hierarchy targeting. Straightforward to add once the parallel variants are stable.
- **Mutation API**: `move_curve`, `add_subtree`, `remove_subtree` — tree mutation helpers from the spec. Simple to add as needed.
- **`resolve_operations` method on `MarketDataHierarchy`**: The spec defines `resolve_operations(&self, ops, mode) -> HashMap<CurveId, Vec<OperationSpec>>`. The plan uses `expand_hierarchy_operations` in the engine instead (equivalent functionality, different API surface). Can be added later as a convenience method.
- **`OperationTarget` unified enum**: The spec envisions a single `OperationTarget` enum (Direct/ByKind/HierarchyNode). The plan uses separate `Hierarchy*` operation variants for simplicity and backwards compatibility. Unification is a possible future refactor.
- **Entity/Instrument hierarchies**: Extension points are documented in the spec but not implemented.
- **Directory-based market data loading**: `load_curves_from_dir`, etc. — useful utility but not core to hierarchy.
- **Factor model integration**: Factor models reference hierarchy nodes but the `FactorDefinition` type is not implemented here.
