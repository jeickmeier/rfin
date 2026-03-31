use crate::core::error::{core_to_js, js_error};
use finstack_core::market_data::hierarchy::{
    HierarchyNode as CoreHierarchyNode, HierarchyTarget as CoreHierarchyTarget,
    MarketDataHierarchy as CoreHierarchy, ResolutionMode as CoreResolutionMode,
    TagFilter as CoreTagFilter, TagPredicate as CoreTagPredicate,
};
use finstack_core::types::CurveId;
use wasm_bindgen::prelude::*;

// ======================================================================
// ResolutionMode
// ======================================================================

/// Controls how shocks at multiple hierarchy levels combine for a single curve.
#[wasm_bindgen(js_name = ResolutionMode)]
#[derive(Clone, Copy, Debug)]
pub struct JsResolutionMode {
    inner: CoreResolutionMode,
}

impl JsResolutionMode {
    pub(crate) fn inner(&self) -> CoreResolutionMode {
        self.inner
    }
}

#[wasm_bindgen(js_class = ResolutionMode)]
impl JsResolutionMode {
    /// The deepest matching node's shock wins; parent-level shocks are ignored.
    #[wasm_bindgen(js_name = MostSpecificWins)]
    pub fn most_specific_wins() -> JsResolutionMode {
        JsResolutionMode {
            inner: CoreResolutionMode::MostSpecificWins,
        }
    }

    /// Shocks accumulate walking down the tree from root to leaf.
    #[wasm_bindgen(js_name = Cumulative)]
    pub fn cumulative() -> JsResolutionMode {
        JsResolutionMode {
            inner: CoreResolutionMode::Cumulative,
        }
    }

    /// String representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("{:?}", self.inner)
    }
}

// ======================================================================
// TagPredicate
// ======================================================================

/// A predicate for filtering hierarchy nodes by their tags.
#[wasm_bindgen(js_name = TagPredicate)]
#[derive(Clone, Debug)]
pub struct JsTagPredicate {
    inner: CoreTagPredicate,
}

impl JsTagPredicate {
    pub(crate) fn inner(&self) -> &CoreTagPredicate {
        &self.inner
    }
}

#[wasm_bindgen(js_class = TagPredicate)]
impl JsTagPredicate {
    /// Tag value must exactly equal the given value.
    #[wasm_bindgen(js_name = Equals)]
    pub fn equals(key: &str, value: &str) -> JsTagPredicate {
        JsTagPredicate {
            inner: CoreTagPredicate::Equals {
                key: key.to_string(),
                value: value.to_string(),
            },
        }
    }

    /// Tag value must be one of the given values.
    #[wasm_bindgen(js_name = In)]
    pub fn in_values(key: &str, values: Vec<String>) -> JsTagPredicate {
        JsTagPredicate {
            inner: CoreTagPredicate::In {
                key: key.to_string(),
                values,
            },
        }
    }

    /// Tag key must exist (any value).
    #[wasm_bindgen(js_name = Exists)]
    pub fn exists(key: &str) -> JsTagPredicate {
        JsTagPredicate {
            inner: CoreTagPredicate::Exists {
                key: key.to_string(),
            },
        }
    }
}

// ======================================================================
// TagFilter
// ======================================================================

/// A filter combining multiple tag predicates with AND semantics.
#[wasm_bindgen(js_name = TagFilter)]
#[derive(Clone, Debug)]
pub struct JsTagFilter {
    inner: CoreTagFilter,
}

impl JsTagFilter {
    pub(crate) fn inner(&self) -> &CoreTagFilter {
        &self.inner
    }
}

impl Default for JsTagFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = TagFilter)]
impl JsTagFilter {
    /// Create an empty tag filter (matches everything).
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsTagFilter {
        JsTagFilter {
            inner: CoreTagFilter::default(),
        }
    }

    /// Add a predicate to the filter (AND semantics).
    #[wasm_bindgen(js_name = addPredicate)]
    pub fn add_predicate(&mut self, predicate: &JsTagPredicate) {
        self.inner.predicates.push(predicate.inner().clone());
    }

    /// Whether this filter is empty (matches everything).
    #[wasm_bindgen(getter, js_name = isEmpty)]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

// ======================================================================
// HierarchyTarget
// ======================================================================

/// A target specifying a hierarchy path with optional tag filtering.
#[wasm_bindgen(js_name = HierarchyTarget)]
#[derive(Clone, Debug)]
pub struct JsHierarchyTarget {
    inner: CoreHierarchyTarget,
}

impl JsHierarchyTarget {
    pub(crate) fn inner(&self) -> &CoreHierarchyTarget {
        &self.inner
    }
}

#[wasm_bindgen(js_class = HierarchyTarget)]
impl JsHierarchyTarget {
    /// Create a target from a `/`-separated path string (e.g., `"Credit/US/IG"`).
    #[wasm_bindgen(constructor)]
    pub fn new(path: &str) -> Result<JsHierarchyTarget, JsValue> {
        let segments: Vec<String> = path.split('/').map(String::from).collect();
        if segments.iter().any(|s| s.is_empty()) {
            return Err(js_error("HierarchyTarget path must not contain empty segments"));
        }
        Ok(JsHierarchyTarget {
            inner: CoreHierarchyTarget {
                path: segments,
                tag_filter: None,
            },
        })
    }

    /// Create a target with both path and tag filter.
    #[wasm_bindgen(js_name = withFilter)]
    pub fn with_filter(path: &str, filter: &JsTagFilter) -> Result<JsHierarchyTarget, JsValue> {
        let segments: Vec<String> = path.split('/').map(String::from).collect();
        if segments.iter().any(|s| s.is_empty()) {
            return Err(js_error("HierarchyTarget path must not contain empty segments"));
        }
        Ok(JsHierarchyTarget {
            inner: CoreHierarchyTarget {
                path: segments,
                tag_filter: Some(filter.inner().clone()),
            },
        })
    }

    /// Path segments.
    #[wasm_bindgen(getter)]
    pub fn path(&self) -> Vec<String> {
        self.inner.path.clone()
    }
}

// ======================================================================
// HierarchyNode (read-only view)
// ======================================================================

/// A node in the market data hierarchy tree (read-only view).
#[wasm_bindgen(js_name = HierarchyNode)]
#[derive(Clone, Debug)]
pub struct JsHierarchyNode {
    inner: CoreHierarchyNode,
}

impl JsHierarchyNode {
    pub(crate) fn from_core(inner: CoreHierarchyNode) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = HierarchyNode)]
impl JsHierarchyNode {
    /// Node display name.
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.inner.name().to_string()
    }

    /// Curve IDs at this node (leaf references).
    #[wasm_bindgen(getter, js_name = curveIds)]
    pub fn curve_ids(&self) -> Vec<String> {
        self.inner
            .curve_ids()
            .iter()
            .map(|id| id.as_str().to_string())
            .collect()
    }

    /// All curve IDs in this subtree (this node + all descendants).
    #[wasm_bindgen(js_name = allCurveIds)]
    pub fn all_curve_ids(&self) -> Vec<String> {
        self.inner
            .all_curve_ids()
            .into_iter()
            .map(|id| id.as_str().to_string())
            .collect()
    }

    /// Child node names.
    #[wasm_bindgen(getter, js_name = childNames)]
    pub fn child_names(&self) -> Vec<String> {
        self.inner.children().keys().cloned().collect()
    }

    /// Get a child node by name. Returns `undefined` if not found.
    #[wasm_bindgen(js_name = getChild)]
    pub fn get_child(&self, name: &str) -> Option<JsHierarchyNode> {
        self.inner
            .children()
            .get(name)
            .cloned()
            .map(JsHierarchyNode::from_core)
    }
}

// ======================================================================
// HierarchyBuilder (fluent)
// ======================================================================

/// Fluent builder for `MarketDataHierarchy`.
///
/// Uses `/`-separated paths to auto-create intermediate nodes.
///
/// @example
/// ```javascript
/// const hierarchy = new HierarchyBuilder()
///     .addNode("Rates/USD/OIS").curveIds(["USD-OIS"])
///     .addNode("Credit/US/IG/Financials").tag("sector", "Financials").curveIds(["JPM-5Y", "GS-5Y"])
///     .build();
/// ```
#[wasm_bindgen(js_name = HierarchyBuilder)]
pub struct JsHierarchyBuilder {
    inner: Option<finstack_core::market_data::hierarchy::HierarchyBuilder>,
}

impl Default for JsHierarchyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = HierarchyBuilder)]
impl JsHierarchyBuilder {
    /// Create a new hierarchy builder.
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsHierarchyBuilder {
        JsHierarchyBuilder {
            inner: Some(CoreHierarchy::builder()),
        }
    }

    /// Start or switch to a node at the given `/`-separated path.
    #[wasm_bindgen(js_name = addNode)]
    pub fn add_node(mut self, path: &str) -> JsHierarchyBuilder {
        if let Some(builder) = self.inner.take() {
            self.inner = Some(builder.add_node(path));
        }
        self
    }

    /// Set a tag on the current node.
    pub fn tag(mut self, key: &str, value: &str) -> JsHierarchyBuilder {
        if let Some(builder) = self.inner.take() {
            self.inner = Some(builder.tag(key, value));
        }
        self
    }

    /// Add curve IDs to the current node.
    #[wasm_bindgen(js_name = curveIds)]
    pub fn curve_ids(mut self, ids: Vec<String>) -> JsHierarchyBuilder {
        if let Some(builder) = self.inner.take() {
            let refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
            self.inner = Some(builder.curve_ids(&refs));
        }
        self
    }

    /// Finalize and validate the hierarchy.
    pub fn build(mut self) -> Result<JsMarketDataHierarchy, JsValue> {
        let builder = self
            .inner
            .take()
            .ok_or_else(|| js_error("Builder already consumed"))?;
        let hierarchy = builder.build().map_err(core_to_js)?;
        Ok(JsMarketDataHierarchy { inner: hierarchy })
    }
}

// ======================================================================
// MarketDataHierarchy
// ======================================================================

/// Top-level market data hierarchy containing root nodes.
///
/// Each root represents a major asset class or category (e.g., "Rates", "Credit").
///
/// @example
/// ```javascript
/// const hierarchy = new HierarchyBuilder()
///     .addNode("Rates/USD/OIS").curveIds(["USD-OIS"])
///     .build();
///
/// const ids = hierarchy.allCurveIds();
/// const node = hierarchy.getNode("Rates/USD/OIS");
/// ```
#[wasm_bindgen(js_name = MarketDataHierarchy)]
#[derive(Clone, Debug)]
pub struct JsMarketDataHierarchy {
    inner: CoreHierarchy,
}

impl JsMarketDataHierarchy {
    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &CoreHierarchy {
        &self.inner
    }
}

impl Default for JsMarketDataHierarchy {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = MarketDataHierarchy)]
impl JsMarketDataHierarchy {
    /// Create an empty hierarchy.
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsMarketDataHierarchy {
        JsMarketDataHierarchy {
            inner: CoreHierarchy::new(),
        }
    }

    /// Root node names.
    #[wasm_bindgen(getter, js_name = rootNames)]
    pub fn root_names(&self) -> Vec<String> {
        self.inner.roots().keys().cloned().collect()
    }

    /// Get a root node by name.
    #[wasm_bindgen(js_name = getRoot)]
    pub fn get_root(&self, name: &str) -> Option<JsHierarchyNode> {
        self.inner
            .roots()
            .get(name)
            .cloned()
            .map(JsHierarchyNode::from_core)
    }

    /// Look up a node by `/`-separated path.
    #[wasm_bindgen(js_name = getNode)]
    pub fn get_node(&self, path: &str) -> Option<JsHierarchyNode> {
        let segments: Vec<String> = path.split('/').map(String::from).collect();
        self.inner
            .get_node(&segments)
            .cloned()
            .map(JsHierarchyNode::from_core)
    }

    /// Collect all curve IDs across the entire hierarchy.
    #[wasm_bindgen(js_name = allCurveIds)]
    pub fn all_curve_ids(&self) -> Vec<String> {
        self.inner
            .all_curve_ids()
            .into_iter()
            .map(|id| id.as_str().to_string())
            .collect()
    }

    /// Insert a curve at a `/`-separated path, creating intermediate nodes.
    #[wasm_bindgen(js_name = insertCurve)]
    pub fn insert_curve(&mut self, path: &str, curve_id: &str) -> Result<(), JsValue> {
        self.inner.insert_curve(path, curve_id).map_err(core_to_js)
    }

    /// Remove a curve from wherever it sits in the tree.
    #[wasm_bindgen(js_name = removeCurve)]
    pub fn remove_curve(&mut self, curve_id: &str) -> bool {
        self.inner.remove_curve(&CurveId::from(curve_id))
    }

    /// Find the path from root to a specific curve.
    #[wasm_bindgen(js_name = pathForCurve)]
    pub fn path_for_curve(&self, curve_id: &str) -> Option<Vec<String>> {
        self.inner.path_for_curve(&CurveId::from(curve_id))
    }

    /// Set a tag on a node at the given `/`-separated path.
    #[wasm_bindgen(js_name = setTag)]
    pub fn set_tag(&mut self, path: &str, key: &str, value: &str) -> Result<(), JsValue> {
        self.inner.set_tag(path, key, value).map_err(core_to_js)
    }

    /// Resolve a hierarchy target to the set of curve IDs it covers.
    pub fn resolve(
        &self,
        target: &JsHierarchyTarget,
        mode: &JsResolutionMode,
    ) -> Vec<String> {
        self.inner
            .resolve(target.inner(), mode.inner())
            .into_iter()
            .map(|id| id.as_str().to_string())
            .collect()
    }

    /// Find all curves matching a tag filter across the entire hierarchy.
    #[wasm_bindgen(js_name = queryByTags)]
    pub fn query_by_tags(&self, filter: &JsTagFilter) -> Vec<String> {
        self.inner
            .query_by_tags(filter.inner())
            .into_iter()
            .map(|id| id.as_str().to_string())
            .collect()
    }

    /// Validate structural hierarchy invariants.
    pub fn validate(&self) -> Result<(), JsValue> {
        self.inner.validate().map_err(core_to_js)
    }
}
