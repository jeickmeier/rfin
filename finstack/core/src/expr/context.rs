//! Expression evaluation context.

/// A simple context that resolves column names to series indices.
/// Simple name→index context for column resolution.
#[derive(Clone, Debug)]
pub struct SimpleContext {
    /// Column names in order; used to resolve indices.
    pub columns: std::vec::Vec<String>,
}

impl SimpleContext {
    /// Construct from an iterator of column names.
    pub fn new(columns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            columns: columns.into_iter().map(Into::into).collect(),
        }
    }
    /// Find the index of a column by name.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c == name)
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
