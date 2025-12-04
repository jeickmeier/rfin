//! Period aggregation example — YTD, QTD, TTM, and fiscal YTD.
//!
//! This example demonstrates how to use the statements DSL period aggregation
//! helpers:
//! - `ttm(expr)` / `ltm(expr)` for trailing / last twelve months
//! - `ytd(expr)` for calendar year-to-date sums
//! - `qtd(expr)` for quarter-to-date sums (monthly models)
//! - `fiscal_ytd(expr, start_month)` for fiscal year-to-date sums
//!
//! Run with:
//! ```bash
//! cargo run --example period_aggregation_example --features "mc"
//! ```

use finstack_statements::prelude::*;

fn main() -> Result<()> {
    println!("=== Period Aggregation Example (YTD / QTD / TTM / Fiscal YTD) ===\n");

    example_quarterly_ytd_and_ttm()?;
    example_monthly_qtd_and_fiscal_ytd()?;

    Ok(())
}

fn example_quarterly_ytd_and_ttm() -> Result<()> {
    println!("📊 Quarterly model: YTD and TTM\n");

    let model = ModelBuilder::new("Quarterly Aggregations")
        .periods("2024Q1..2025Q2", None)?
        .value(
            "revenue",
            &[
                (PeriodId::quarter(2024, 1), AmountOrScalar::scalar(100.0)),
                (PeriodId::quarter(2024, 2), AmountOrScalar::scalar(105.0)),
                (PeriodId::quarter(2024, 3), AmountOrScalar::scalar(110.0)),
                (PeriodId::quarter(2024, 4), AmountOrScalar::scalar(115.0)),
                (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(120.0)),
                (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(125.0)),
            ],
        )
        .compute("revenue_ytd", "ytd(revenue)")?
        .compute("revenue_ttm", "ttm(revenue)")?
        .build()?;

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model)?;

    println!("{:<8} {:>10} {:>12} {:>12}", "Period", "Revenue", "YTD", "TTM");
    println!("{}", "-".repeat(50));
    for period in &model.periods {
        let id = &period.id;
        let revenue = results.get("revenue", id).unwrap();
        let ytd = results.get("revenue_ytd", id).unwrap();
        let ttm = results.get("revenue_ttm", id).unwrap();
        println!(
            "{:<8} {:>10.0} {:>12.0} {:>12.0}",
            id, revenue, ytd, ttm
        );
    }

    println!();
    Ok(())
}

fn example_monthly_qtd_and_fiscal_ytd() -> Result<()> {
    println!("📊 Monthly model: QTD and Fiscal YTD (April start)\n");

    let model = ModelBuilder::new("Monthly Aggregations")
        .periods("2024M01..2025M06", None)?
        .value(
            "revenue",
            &[
                // FY2024 (calendar)
                (PeriodId::month(2024, 10), AmountOrScalar::scalar(80.0)),
                (PeriodId::month(2024, 11), AmountOrScalar::scalar(90.0)),
                (PeriodId::month(2024, 12), AmountOrScalar::scalar(95.0)),
                // FY2025 (calendar)
                (PeriodId::month(2025, 1), AmountOrScalar::scalar(100.0)),
                (PeriodId::month(2025, 2), AmountOrScalar::scalar(110.0)),
                (PeriodId::month(2025, 3), AmountOrScalar::scalar(120.0)),
                (PeriodId::month(2025, 4), AmountOrScalar::scalar(130.0)),
                (PeriodId::month(2025, 5), AmountOrScalar::scalar(140.0)),
                (PeriodId::month(2025, 6), AmountOrScalar::scalar(150.0)),
            ],
        )
        .compute("revenue_qtd", "qtd(revenue)")?
        // April fiscal year (start_month = 4)
        .compute("revenue_fiscal_ytd", "fiscal_ytd(revenue, 4)")?
        .build()?;

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model)?;

    println!(
        "{:<8} {:>10} {:>12} {:>16}",
        "Period", "Revenue", "QTD", "Fiscal YTD (Apr)"
    );
    println!("{}", "-".repeat(60));

    for period in &model.periods {
        let id = &period.id;
        let revenue = results.get_or("revenue", id, f64::NAN);
        let qtd = results.get_or("revenue_qtd", id, f64::NAN);
        let fiscal_ytd = results.get_or("revenue_fiscal_ytd", id, f64::NAN);

        if revenue.is_nan() && qtd.is_nan() && fiscal_ytd.is_nan() {
            continue;
        }

        println!(
            "{:<8} {:>10.0} {:>12.0} {:>16.0}",
            id, revenue, qtd, fiscal_ytd
        );
    }

    println!();
    Ok(())
}



