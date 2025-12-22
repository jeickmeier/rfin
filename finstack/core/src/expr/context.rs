//! Expression evaluation context for column name resolution.
//!
//! Provides the context interface used by expression evaluators to resolve
//! column references to array indices at runtime.

use crate::collections::HashMap;

/// A simple context that resolves column names to series indices.
/// Simple name→index context for column resolution.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SimpleContext {
    /// Column name to index mapping for O(1) resolution.
    column_indices: HashMap<String, usize>,
}

impl SimpleContext {
    /// Construct from an iterator of column names.
    pub fn new(columns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let column_indices = columns
            .into_iter()
            .enumerate()
            .map(|(idx, name)| (name.into(), idx))
            .collect();

        Self { column_indices }
    }
    /// Find the index of a column by name.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.column_indices.get(name).copied()
    }
}

/// Trait for pluggable expression contexts used by evaluators.
/// Context trait used by evaluators to resolve column references.
pub trait ExpressionContext {
    /// Resolve a column name to its index in an input frame.
    fn resolve_index(&self, name: &str) -> Option<usize>;
}

impl ExpressionContext for SimpleContext {
    fn resolve_index(&self, name: &str) -> Option<usize> {
        self.index_of(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_context_maps_columns_in_insertion_order() {
        let ctx = SimpleContext::new(["price", "volume", "flag"]);
        assert_eq!(ctx.index_of("price"), Some(0));
        assert_eq!(ctx.index_of("volume"), Some(1));
        assert_eq!(ctx.index_of("flag"), Some(2));
        assert_eq!(ctx.resolve_index("missing"), None);
    }
}
