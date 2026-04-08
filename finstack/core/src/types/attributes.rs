use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// User-defined tags and key-value metadata for instrument classification.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Attributes {
    /// User-defined tags for categorization.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub tags: BTreeSet<String>,
    /// Structured metadata associated with the instrument.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub meta: BTreeMap<String, String>,
}

impl Attributes {
    /// Create an empty set of attributes.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a single tag and return the updated attributes.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.insert(tag.into());
        self
    }

    /// Add multiple tags and return the updated attributes.
    #[must_use]
    pub fn with_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags.extend(tags.into_iter().map(Into::into));
        self
    }

    /// Add a metadata entry and return the updated attributes.
    #[must_use]
    pub fn with_meta(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.meta.insert(key.into(), value.into());
        self
    }

    /// Set a metadata entry in place.
    pub fn set(&mut self, key: &str, value: impl ToString) {
        self.meta.insert(key.to_string(), value.to_string());
    }

    /// Check whether a tag is present.
    #[must_use]
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(tag)
    }

    /// Look up a metadata value by key.
    #[must_use]
    pub fn get_meta(&self, key: &str) -> Option<&str> {
        self.meta.get(key).map(String::as_str)
    }

    /// Match the attributes against a simple selector string.
    ///
    /// Unrecognized selector prefixes return `false` rather than erroring,
    /// to allow forward-compatible selector syntax extension.
    #[must_use]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attributes_default_is_empty() {
        let attrs = Attributes::default();
        assert!(attrs.tags.is_empty());
        assert!(attrs.meta.is_empty());
    }

    #[test]
    fn test_attributes_builder_methods() {
        let attrs = Attributes::default()
            .with_tag("energy")
            .with_meta("region", "NA")
            .with_meta("rating", "CCC");

        assert!(attrs.has_tag("energy"));
        assert!(!attrs.has_tag("financials"));
        assert_eq!(attrs.get_meta("region"), Some("NA"));
        assert_eq!(attrs.get_meta("rating"), Some("CCC"));
        assert_eq!(attrs.get_meta("nonexistent"), None);
    }

    #[test]
    fn test_attributes_with_tags_batch() {
        let attrs = Attributes::default().with_tags(["a", "b", "c"]);
        assert!(attrs.has_tag("a"));
        assert!(attrs.has_tag("b"));
        assert!(attrs.has_tag("c"));
    }

    #[test]
    fn test_attributes_serde_roundtrip() {
        let attrs = Attributes::default()
            .with_tag("energy")
            .with_meta("region", "NA");

        let json_result = serde_json::to_string(&attrs);
        assert!(json_result.is_ok());
        let json = json_result.unwrap_or_default();

        let attrs_result: Result<Attributes, _> = serde_json::from_str(&json);
        assert!(attrs_result.is_ok());
        let deserialized = attrs_result.unwrap_or_default();

        assert_eq!(attrs.tags, deserialized.tags);
        assert_eq!(attrs.meta, deserialized.meta);
    }
}
