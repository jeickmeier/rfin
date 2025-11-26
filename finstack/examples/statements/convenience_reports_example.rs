//! Example demonstrating convenience reports.
//!
//! Run with: cargo run --example convenience_reports_example

use finstack_core::dates::PeriodId;
use finstack_statements::builder::ModelBuilder;
use finstack_statements::evaluator::Evaluator;
use finstack_statements::analysis::{CreditAssessmentReport, PLSummaryReport, Report};
use finstack_statements::types::AmountOrScalar;
use finstack_statements::Result;

fn main() -> Result<()> {
    println!("=== Convenience Reports Example ===\n");

    // Build a P&L model
    let period_q1 = PeriodId::quarter(2025, 1);
    let period_q2 = PeriodId::quarter(2025, 2);

    let model = ModelBuilder::new("acme_corp")
        .periods("2025Q1..Q2", None)?
        .value(
            "revenue",
            &[
                (period_q1, AmountOrScalar::scalar(100_000.0)),
                (period_q2, AmountOrScalar::scalar(110_000.0)),
            ],
        )
        .compute("cogs", "revenue * 0.4")?
        .compute("gross_profit", "revenue - cogs")?
        .compute("opex", "revenue * 0.25")?
        .compute("ebitda", "gross_profit - opex")?
        .compute("interest_expense", "5000")?
        .compute("total_debt", "200000")?
        .build()?;

    // Evaluate
    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model)?;

    // Generate P&L summary report
    println!("1. P&L Summary Report\n");
    let pl_report = PLSummaryReport::new(
        &results,
        vec!["revenue", "cogs", "gross_profit", "opex", "ebitda"],
        vec![period_q1, period_q2],
    );
    pl_report.print();

    // Generate credit assessment
    println!("\n2. Credit Assessment Report\n");
    let credit_report = CreditAssessmentReport::new(&results, period_q1);
    credit_report.print();

    Ok(())
}
