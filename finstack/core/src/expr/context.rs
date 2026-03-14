//! Column name resolution context for expression evaluation.
//!
//! Provides [`SimpleContext`], the single concrete context type used by
//! expression evaluators to resolve column references to array indices.

use crate::collections::HashMap;

/// Column name → index context for expression evaluation.
///
/// This is the single concrete context type accepted by [`CompiledExpr::eval`].
/// Construct it from any ordered iterator of column names; the index of each
/// name in the iterator becomes its column index in the data arrays passed to
/// `eval`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn simple_context_maps_columns_in_insertion_order() {
        let ctx = SimpleContext::new(["price", "volume", "flag"]);
        assert_eq!(ctx.index_of("price"), Some(0));
        assert_eq!(ctx.index_of("volume"), Some(1));
        assert_eq!(ctx.index_of("flag"), Some(2));
        assert_eq!(ctx.index_of("missing"), None);
    }
}
