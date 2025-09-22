//! Instrument-level traits and metadata types.

use crate::metrics::MetricId;
use crate::metrics::traits::MetricContext;
use finstack_core::market_data::MarketContext;
use finstack_core::prelude::*;
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use std::any::Any;

/// Attributes for scenario selection and tagging.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Attributes {
    /// User-defined tags for categorization.
    pub tags: HashSet<String>,
    /// Key-value metadata pairs.
    pub meta: HashMap<String, String>,
}

impl Attributes {
    /// Create empty attributes.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a tag.
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.insert(tag.to_string());
        self
    }

    /// Add multiple tags.
    pub fn with_tags(mut self, tags: &[&str]) -> Self {
        for tag in tags {
            self.tags.insert(tag.to_string());
        }
        self
    }

    /// Add a metadata key-value pair.
    pub fn with_meta(mut self, key: &str, value: &str) -> Self {
        self.meta.insert(key.to_string(), value.to_string());
        self
    }

    /// Check if a tag exists.
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(tag)
    }

    /// Get a metadata value by key.
    pub fn get_meta(&self, key: &str) -> Option<&str> {
        self.meta.get(key).map(|s| s.as_str())
    }

    /// Check if attributes match a selector pattern.
    pub fn matches_selector(&self, selector: &str) -> bool {
        if selector == "*" {
            return true;
        }
        if let Some(tag) = selector.strip_prefix("tag:") {
            return self.has_tag(tag);
        }
        if let Some(meta_spec) = selector.strip_prefix("meta:") {
            if let Some((key, value)) = meta_spec.split_once('=') {
                return self.get_meta(key) == Some(value);
            }
        }
        false
    }
}

/// Trait for instruments with attributes.
pub trait Attributable: Send + Sync {
    /// Get the instrument's attributes.
    fn attributes(&self) -> &Attributes;
    /// Get mutable access to attributes.
    fn attributes_mut(&mut self) -> &mut Attributes;

    /// Check if the instrument matches a selector.
    fn matches_selector(&self, selector: &str) -> bool {
        self.attributes().matches_selector(selector)
    }
    /// Check if the instrument has a specific tag.
    fn has_tag(&self, tag: &str) -> bool {
        self.attributes().has_tag(tag)
    }
    /// Get a metadata value by key.
    fn get_meta(&self, key: &str) -> Option<&str> {
        self.attributes().get_meta(key)
    }
}

/// Unified instrument trait combining identity, attributes, and pricing.
///
/// This is the primary trait for all financial instruments, providing both
/// metadata/identity methods and pricing functionality. All instruments
/// should implement this single trait.
pub trait Instrument: Send + Sync {
    /// Get the instrument's unique identifier.
    fn id(&self) -> &str;

    /// Get the instrument type as a string identifier.
    fn instrument_type(&self) -> &'static str;

    /// Access to the concrete type for downcasting.
    fn as_any(&self) -> &dyn Any;

    /// Attributes accessors
    fn attributes(&self) -> &Attributes;
    fn attributes_mut(&mut self) -> &mut Attributes;

    /// Clone this instrument as a boxed trait object
    fn clone_box(&self) -> Box<dyn Instrument>;

    // === Pricing Methods ===

    /// Compute only the base present value (fast, no metrics).
    fn value(&self, curves: &MarketContext, as_of: Date) -> finstack_core::Result<Money>;

    /// Compute value with a specific set of metrics.
    ///
    /// Implementations should build on `value()` and compute only the requested metrics.
    fn price_with_metrics(
        &self,
        curves: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult>;

    /// Optional hook to prepare the metric context before calculation.
    ///
    /// Instruments can override this to cache cashflows, specify discount curve IDs,
    /// set day count conventions, or attach a bucket key resolver for bucketed metrics.
    /// The default implementation is a no-op.
    fn prepare_metric_context(
        &self,
        _context: &mut MetricContext,
    ) -> finstack_core::Result<()> {
        Ok(())
    }
}
