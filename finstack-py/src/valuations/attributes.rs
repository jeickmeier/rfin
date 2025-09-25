//! Python bindings for instrument attributes and tagging.

use finstack_valuations::instruments::Attributes;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PySet};

/// Attributes for instrument categorization and metadata.
///
/// Provides a flexible tagging and metadata system for instruments,
/// enabling scenario selection, filtering, and custom properties.
///
/// Examples:
///     >>> from finstack import Attributes
///     >>>
///     >>> # Create attributes with tags and metadata
///     >>> attrs = Attributes()
///     >>> attrs.add_tag("corporate")
///     >>> attrs.add_tag("investment_grade")
///     >>> attrs.set_meta("issuer", "AAPL")
///     >>> attrs.set_meta("rating", "AA+")
///     >>>
///     >>> # Check if instrument matches criteria
///     >>> assert attrs.has_tag("corporate")
///     >>> assert attrs.get_meta("issuer") == "AAPL"
///     >>> assert attrs.matches_selector("tag:corporate")
#[pyclass(name = "Attributes", module = "finstack.valuations")]
#[derive(Clone)]
pub struct PyAttributes {
    pub(crate) inner: Attributes,
}

#[pymethods]
impl PyAttributes {
    /// Create new empty attributes.
    #[new]
    fn new() -> Self {
        Self {
            inner: Attributes::new(),
        }
    }

    /// Add a tag for categorization.
    ///
    /// Args:
    ///     tag: Tag string to add
    ///
    /// Returns:
    ///     Self for chaining
    ///
    /// Examples:
    ///     >>> attrs = Attributes()
    ///     >>> attrs.add_tag("high_yield").add_tag("energy_sector")
    fn add_tag(&mut self, tag: String) -> PyResult<()> {
        self.inner.tags.insert(tag);
        Ok(())
    }

    /// Add multiple tags at once.
    ///
    /// Args:
    ///     tags: List of tag strings
    ///
    /// Examples:
    ///     >>> attrs.add_tags(["corporate", "investment_grade", "tech"])
    fn add_tags(&mut self, tags: Vec<String>) -> PyResult<()> {
        for tag in tags {
            self.inner.tags.insert(tag);
        }
        Ok(())
    }

    /// Remove a tag.
    ///
    /// Args:
    ///     tag: Tag to remove
    ///
    /// Returns:
    ///     True if the tag was present
    fn remove_tag(&mut self, tag: &str) -> bool {
        self.inner.tags.remove(tag)
    }

    /// Check if a tag exists.
    ///
    /// Args:
    ///     tag: Tag to check
    ///
    /// Returns:
    ///     True if the tag exists
    fn has_tag(&self, tag: &str) -> bool {
        self.inner.has_tag(tag)
    }

    /// Get all tags as a list.
    ///
    /// Returns:
    ///     List of all tags
    #[getter]
    fn tags(&self, py: Python) -> PyResult<Py<PySet>> {
        let set = PySet::new(py, self.inner.tags.iter())?;
        Ok(set.into())
    }

    /// Set a metadata key-value pair.
    ///
    /// Args:
    ///     key: Metadata key
    ///     value: Metadata value
    ///
    /// Examples:
    ///     >>> attrs.set_meta("issuer", "Microsoft")
    ///     >>> attrs.set_meta("sector", "Technology")
    fn set_meta(&mut self, key: String, value: String) -> PyResult<()> {
        self.inner.meta.insert(key, value);
        Ok(())
    }

    /// Get a metadata value by key.
    ///
    /// Args:
    ///     key: Metadata key
    ///
    /// Returns:
    ///     The value if present, None otherwise
    fn get_meta(&self, key: &str) -> Option<String> {
        self.inner.get_meta(key).map(|s| s.to_string())
    }

    /// Remove a metadata entry.
    ///
    /// Args:
    ///     key: Key to remove
    ///
    /// Returns:
    ///     The removed value if present
    fn remove_meta(&mut self, key: &str) -> Option<String> {
        self.inner.meta.remove(key)
    }

    /// Check if metadata key exists.
    ///
    /// Args:
    ///     key: Key to check
    ///
    /// Returns:
    ///     True if the key exists
    fn has_meta(&self, key: &str) -> bool {
        self.inner.meta.contains_key(key)
    }

    /// Get all metadata as a dictionary.
    ///
    /// Returns:
    ///     Dictionary of all metadata
    #[getter]
    fn metadata(&self, py: Python) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        for (key, value) in &self.inner.meta {
            dict.set_item(key, value)?;
        }
        Ok(dict.into())
    }

    /// Check if attributes match a selector string.
    ///
    /// Selectors support:
    /// - Tag matching: "tag:corporate"
    /// - Metadata matching: "meta:issuer=AAPL"
    /// - Wildcards: "tag:*" or "meta:sector=*"
    ///
    /// Args:
    ///     selector: Selector string
    ///
    /// Returns:
    ///     True if attributes match the selector
    ///
    /// Examples:
    ///     >>> attrs.add_tag("corporate")
    ///     >>> attrs.set_meta("rating", "AA")
    ///     >>> assert attrs.matches_selector("tag:corporate")
    ///     >>> assert attrs.matches_selector("meta:rating=AA")
    fn matches_selector(&self, selector: &str) -> bool {
        self.inner.matches_selector(selector)
    }

    /// Clear all tags.
    fn clear_tags(&mut self) {
        self.inner.tags.clear();
    }

    /// Clear all metadata.
    fn clear_meta(&mut self) {
        self.inner.meta.clear();
    }

    /// Clear all attributes (tags and metadata).
    fn clear(&mut self) {
        self.clear_tags();
        self.clear_meta();
    }

    /// Create a copy of the attributes.
    fn copy(&self) -> Self {
        self.clone()
    }

    /// Convert to dictionary representation.
    ///
    /// Returns:
    ///     Dictionary with 'tags' and 'meta' fields
    fn to_dict(&self, py: Python) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);

        // Add tags as list
        let tags_list = PyList::new(py, self.inner.tags.iter())?;
        dict.set_item("tags", tags_list)?;

        // Add meta as dict
        let meta_dict = PyDict::new(py);
        for (key, value) in &self.inner.meta {
            meta_dict.set_item(key, value)?;
        }
        dict.set_item("meta", meta_dict)?;

        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        let tag_count = self.inner.tags.len();
        let meta_count = self.inner.meta.len();
        format!("Attributes(tags={}, metadata={})", tag_count, meta_count)
    }

    fn __str__(&self) -> String {
        let tags: Vec<String> = self.inner.tags.iter().cloned().collect();
        let meta: Vec<String> = self
            .inner
            .meta
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        format!(
            "Attributes(tags=[{}], meta=[{}])",
            tags.join(", "),
            meta.join(", ")
        )
    }
}

impl PyAttributes {
    /// Create from Rust Attributes
    pub fn from_inner(attrs: Attributes) -> Self {
        Self { inner: attrs }
    }

    /// Get reference to inner Attributes
    pub fn inner_ref(&self) -> &Attributes {
        &self.inner
    }

    /// Get mutable reference to inner Attributes
    pub fn inner_mut(&mut self) -> &mut Attributes {
        &mut self.inner
    }
}
