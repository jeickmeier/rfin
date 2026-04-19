//! Split from `builder.rs` for readability.

use super::*;

impl CashFlowBuilder {
    /// Adds a fee specification.
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn fee(&mut self, spec: FeeSpec) -> &mut Self {
        self.fees.push(spec);
        self
    }
}
