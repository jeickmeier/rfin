use finstack_core::collections::HashMap;
use finstack_core::expr::ExpressionContext;

/// Lightweight expression context used across evaluator tests.
#[derive(Default)]
pub(crate) struct TestExprCtx {
    columns: HashMap<String, usize>,
}

impl TestExprCtx {
    pub(crate) fn new() -> Self {
        Self {
            columns: HashMap::default(),
        }
    }

    /// Register a column name → index mapping and return the updated context.
    pub(crate) fn with_column(mut self, name: impl Into<String>, index: usize) -> Self {
        self.columns.insert(name.into(), index);
        self
    }
}

impl ExpressionContext for TestExprCtx {
    fn resolve_index(&self, name: &str) -> Option<usize> {
        self.columns.get(name).copied()
    }
}
