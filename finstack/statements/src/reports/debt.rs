//! Debt-specific reports.

use super::{tables::*, Report};
use crate::evaluator::Results;
use crate::types::FinancialModelSpec;
use finstack_core::dates::PeriodId;

/// Simple debt summary report (placeholder for future enhancement).
///
/// This is a basic implementation that can be extended with capital structure integration.
///
/// # Examples
///
/// ```rust,ignore
/// use finstack_statements::reports::{Report, DebtSummaryReport};
///
/// let report = DebtSummaryReport::new(&model, &results, period);
/// println!("{}", report.to_string());
/// ```
pub struct DebtSummaryReport<'a> {
    #[allow(dead_code)]
    model: &'a FinancialModelSpec,
    results: &'a Results,
    as_of: PeriodId,
}

impl<'a> DebtSummaryReport<'a> {
    /// Create a new debt summary report.
    pub fn new(model: &'a FinancialModelSpec, results: &'a Results, as_of: PeriodId) -> Self {
        Self {
            model,
            results,
            as_of,
        }
    }
}

impl Report for DebtSummaryReport<'_> {
    fn to_string(&self) -> String {
        let mut table = TableBuilder::new();
        table.add_header("Debt Component");
        table.add_header_with_alignment("Amount", Alignment::Right);

        // Try to find common debt-related nodes
        let debt_nodes = ["total_debt", "long_term_debt", "short_term_debt"];
        for node_id in &debt_nodes {
            if let Some(value) = self.results.get(node_id, &self.as_of) {
                table.add_row(vec![node_id.to_string(), format!("{:.2}", value)]);
            }
        }

        format!("Debt Summary as of {}\n\n{}", self.as_of, table.build())
    }
}

/// Convenience function to print debt summary.
pub fn print_debt_summary(model: &FinancialModelSpec, results: &Results, as_of: PeriodId) {
    DebtSummaryReport::new(model, results, as_of).print();
}

