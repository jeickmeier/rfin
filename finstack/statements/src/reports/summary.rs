//! P&L and summary reports.

use super::{tables::*, Report};
use crate::evaluator::Results;
use finstack_core::dates::PeriodId;

/// P&L summary report.
///
/// # Examples
///
/// ```rust
/// # use finstack_statements::builder::ModelBuilder;
/// # use finstack_statements::evaluator::Evaluator;
/// # use finstack_statements::reports::{Report, PLSummaryReport};
/// # use finstack_core::dates::PeriodId;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let period = PeriodId::quarter(2025, 1);
/// # let model = ModelBuilder::new("demo")
/// #     .periods("2025Q1..Q2", None)?
/// #     .compute("revenue", "100000")?
/// #     .compute("cogs", "40000")?
/// #     .build()?;
/// # let mut evaluator = Evaluator::new();
/// # let results = evaluator.evaluate(&model)?;
/// let report = PLSummaryReport::new(&results, vec!["revenue", "cogs"], vec![period]);
/// println!("{}", report.to_string());
/// # Ok(())
/// # }
/// ```
pub struct PLSummaryReport<'a> {
    results: &'a Results,
    line_items: Vec<String>,
    periods: Vec<PeriodId>,
}

impl<'a> PLSummaryReport<'a> {
    /// Create a new P&L summary report.
    ///
    /// # Arguments
    ///
    /// * `results` - Evaluation results
    /// * `line_items` - Node IDs to include
    /// * `periods` - Periods to display
    pub fn new(
        results: &'a Results,
        line_items: Vec<impl Into<String>>,
        periods: Vec<PeriodId>,
    ) -> Self {
        Self {
            results,
            line_items: line_items.into_iter().map(|s| s.into()).collect(),
            periods,
        }
    }
}

impl Report for PLSummaryReport<'_> {
    fn to_string(&self) -> String {
        let mut table = TableBuilder::new();

        // Header row with periods
        table.add_header("Line Item");
        for period in &self.periods {
            table.add_header_with_alignment(period.to_string(), Alignment::Right);
        }

        // Data rows
        for line_item in &self.line_items {
            let mut row = vec![line_item.clone()];
            for period in &self.periods {
                let value = self.results.get(line_item, period).unwrap_or(0.0);
                row.push(format!("{:.2}", value));
            }
            table.add_row(row);
        }

        format!("P&L Summary\n\n{}", table.build())
    }
}

/// Credit assessment report.
///
/// # Examples
///
/// ```rust
/// # use finstack_statements::builder::ModelBuilder;
/// # use finstack_statements::evaluator::Evaluator;
/// # use finstack_statements::reports::{Report, CreditAssessmentReport};
/// # use finstack_core::dates::PeriodId;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let period = PeriodId::quarter(2025, 1);
/// # let model = ModelBuilder::new("demo")
/// #     .periods("2025Q1..Q2", None)?
/// #     .compute("revenue", "100000")?
/// #     .build()?;
/// # let mut evaluator = Evaluator::new();
/// # let results = evaluator.evaluate(&model)?;
/// let report = CreditAssessmentReport::new(&results, period);
/// println!("{}", report.to_string());
/// # Ok(())
/// # }
/// ```
pub struct CreditAssessmentReport<'a> {
    results: &'a Results,
    as_of: PeriodId,
}

impl<'a> CreditAssessmentReport<'a> {
    /// Create a new credit assessment report.
    pub fn new(results: &'a Results, as_of: PeriodId) -> Self {
        Self { results, as_of }
    }

    fn calculate_leverage_ratio(&self) -> Option<f64> {
        let debt = self.results.get("total_debt", &self.as_of)?;
        let ebitda = self.results.get("ebitda", &self.as_of)?;
        if ebitda != 0.0 {
            Some(debt / ebitda)
        } else {
            None
        }
    }

    fn calculate_interest_coverage(&self) -> Option<f64> {
        let ebitda = self.results.get("ebitda", &self.as_of)?;
        let interest = self.results.get("interest_expense", &self.as_of)?;
        if interest != 0.0 {
            Some(ebitda / interest)
        } else {
            None
        }
    }
}

impl Report for CreditAssessmentReport<'_> {
    fn to_string(&self) -> String {
        let mut output = format!("Credit Assessment as of {}\n\n", self.as_of);

        if let Some(leverage) = self.calculate_leverage_ratio() {
            output.push_str(&format!("Total Debt / EBITDA:        {:.2}x\n", leverage));
        }

        if let Some(coverage) = self.calculate_interest_coverage() {
            output.push_str(&format!(
                "EBITDA / Interest Expense:  {:.2}x\n",
                coverage
            ));
        }

        if let Some(fcf) = self.results.get("free_cash_flow", &self.as_of) {
            output.push_str(&format!("Free Cash Flow:             ${:.2}M\n", fcf / 1_000_000.0));
        }

        output
    }
}

