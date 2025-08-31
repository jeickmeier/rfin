//! Instrument-level traits and metadata types.

use crate::metrics::MetricId;
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::prelude::*;
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

/// Priceable instruments produce a `ValuationResult` at `as_of` using curves.
///
/// The default implementation uses the metrics framework to compute
/// measures, delegating to `value()` for base NPV calculation.
pub trait Priceable: Send + Sync {
    /// Compute full valuation with all standard metrics (backward compatible).
    fn price(
        &self,
        curves: &CurveSet,
        as_of: Date,
    ) -> finstack_core::Result<crate::results::ValuationResult>;

    /// Compute only the base present value (fast, no metrics).
    fn value(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<Money> {
        // Default implementation for backward compatibility
        self.price(curves, as_of).map(|r| r.value)
    }

    /// Compute value with specific metrics.
    fn price_with_metrics(
        &self,
        curves: &CurveSet,
        as_of: Date,
        metrics: &[MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        // Default implementation: just calls price() and filters metrics
        let result = self.price(curves, as_of)?;
        let mut filtered_result = result.clone();

        // Convert MetricIds to strings for filtering
        let metric_strs: Vec<String> = metrics.iter().map(|m| m.as_str().to_string()).collect();
        filtered_result
            .measures
            .retain(|k, _| metric_strs.contains(k));
        Ok(filtered_result)
    }
}

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
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.insert(tag.into());
        self
    }

    /// Add multiple tags.
    pub fn with_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for tag in tags {
            self.tags.insert(tag.into());
        }
        self
    }

    /// Add a metadata key-value pair.
    pub fn with_meta(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.meta.insert(key.into(), value.into());
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

/// Object-safe trait that all instruments implement for unified handling.
///
/// This trait provides the minimal interface needed for metrics computation
/// and portfolio management without requiring hardcoded enum matching.
pub trait InstrumentLike: Priceable + Attributable + Send + Sync {
    /// Get the instrument's unique identifier.
    fn id(&self) -> &str;

    /// Get the instrument type as a string identifier.
    fn instrument_type(&self) -> &'static str;

    /// Get access to the concrete type for downcasting.
    fn as_any(&self) -> &dyn std::any::Any;
}
