//! Phase 7 Example: Results Export to Polars DataFrames
//!
//! This example demonstrates the DataFrame export functionality added in Phase 7,
//! showing how to export evaluation results to both long and wide format DataFrames.
//!
//! **Features Required:** `polars_export`
//!
//! Run with:
//! ```bash
//! cargo run --example statements_phase7_example --features polars_export
//! ```

#[cfg(feature = "polars_export")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use finstack_statements::prelude::*;
    use indexmap::indexmap;
    use serde_json::json;

    println!("=== Phase 7: Results Export Example ===\n");

    // Build a comprehensive P&L model with forecasts
    println!("Building financial model...");
    let model = ModelBuilder::new("Tech Startup P&L")
        .periods("2025Q1..Q4", Some("2025Q2"))?
        // Revenue with 10% quarter-over-quarter growth
        .value(
            "revenue",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(500_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(550_000.0),
                ),
            ],
        )
        .forecast(
            "revenue",
            ForecastSpec {
                method: ForecastMethod::GrowthPct,
                params: indexmap! { "rate".into() => json!(0.10) },
            },
        )
        // Cost of Goods Sold (40% of revenue)
        .compute("cogs", "revenue * 0.40")?
        // Operating Expenses
        .value(
            "opex",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(200_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(210_000.0),
                ),
            ],
        )
        .forecast(
            "opex",
            ForecastSpec {
                method: ForecastMethod::ForwardFill,
                params: indexmap! {},
            },
        )
        // Marketing spend (15% of revenue)
        .compute("marketing", "revenue * 0.15")?
        // Derived metrics
        .compute("gross_profit", "revenue - cogs")?
        .compute("gross_margin", "gross_profit / revenue")?
        .compute("ebitda", "gross_profit - opex - marketing")?
        .compute("ebitda_margin", "ebitda / revenue")?
        .build()?;

    println!(
        "Model built successfully with {} periods and {} nodes\n",
        model.periods.len(),
        model.nodes.len()
    );

    // Evaluate the model
    println!("Evaluating model...");
    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false)?;

    println!("Evaluation complete!");
    println!("  - {} nodes evaluated", results.meta.num_nodes);
    println!("  - {} periods evaluated", results.meta.num_periods);
    if let Some(eval_time) = results.meta.eval_time_ms {
        println!("  - Evaluation time: {}ms\n", eval_time);
    }

    // Example 1: Export to Long Format
    println!("--- Example 1: Long Format Export ---");
    println!("Schema: (node_id, period_id, value)\n");

    let df_long = results.to_polars_long()?;
    println!("Long format DataFrame:");
    println!("  - Rows: {}", df_long.height());
    println!("  - Columns: {}", df_long.width());
    println!("\nFirst 10 rows:");
    println!("{}\n", df_long.head(Some(10)));

    // Example 2: Export to Wide Format
    println!("--- Example 2: Wide Format Export ---");
    println!("Schema: periods as rows, nodes as columns\n");

    let df_wide = results.to_polars_wide()?;
    println!("Wide format DataFrame:");
    println!("  - Rows (periods): {}", df_wide.height());
    println!("  - Columns (period_id + nodes): {}", df_wide.width());
    println!("\nFull DataFrame:");
    println!("{}\n", df_wide);

    // Example 3: Filtered Export (Key Metrics Only)
    println!("--- Example 3: Filtered Export ---");
    println!("Exporting only key P&L metrics\n");

    let key_metrics = vec![
        "revenue",
        "gross_profit",
        "gross_margin",
        "ebitda",
        "ebitda_margin",
    ];

    let df_filtered = results.to_polars_long_filtered(&key_metrics)?;
    println!("Filtered long format (key metrics only):");
    println!("  - Rows: {}", df_filtered.height());
    println!("  - Columns: {}", df_filtered.width());
    println!("\nFiltered DataFrame:");
    println!("{}\n", df_filtered);

    // Example 4: Analyzing Results via Wide Format
    println!("--- Example 4: Financial Analysis via Wide Format ---\n");

    // Extract specific metrics for analysis
    let revenue = df_wide.column("revenue")?.f64()?;
    let ebitda = df_wide.column("ebitda")?.f64()?;
    let ebitda_margin = df_wide.column("ebitda_margin")?.f64()?;
    let period_ids = df_wide.column("period_id")?.str()?;

    println!("Quarter-by-Quarter Analysis:");
    println!(
        "{:<12} {:>15} {:>15} {:>18}",
        "Period", "Revenue", "EBITDA", "EBITDA Margin %"
    );
    println!("{}", "-".repeat(65));

    for i in 0..df_wide.height() {
        let period = period_ids.get(i).unwrap_or("N/A");
        let rev = revenue.get(i).unwrap_or(0.0);
        let ebit = ebitda.get(i).unwrap_or(0.0);
        let margin = ebitda_margin.get(i).unwrap_or(0.0) * 100.0;

        println!(
            "{:<12} ${:>14.0} ${:>14.0} {:>17.1}%",
            period, rev, ebit, margin
        );
    }
    println!();

    // Example 5: Year-over-Year Growth Analysis
    println!("--- Example 5: Growth Analysis ---\n");

    let q1_revenue = revenue.get(0).unwrap_or(0.0);
    let q4_revenue = revenue.get(3).unwrap_or(0.0);
    let total_growth = ((q4_revenue - q1_revenue) / q1_revenue) * 100.0;

    println!("Q1 Revenue: ${:.0}", q1_revenue);
    println!("Q4 Revenue: ${:.0}", q4_revenue);
    println!("Total Growth: {:.1}%\n", total_growth);

    // Example 6: Export Comparison
    println!("--- Example 6: Format Comparison ---\n");

    println!("Long Format:");
    println!("  - Best for: Time-series analysis, grouping operations, joins");
    println!(
        "  - Shape: {} rows × {} columns",
        df_long.height(),
        df_long.width()
    );
    println!("  - Use case: Export to time-series databases, pandas analysis\n");

    println!("Wide Format:");
    println!("  - Best for: Pivot tables, cross-period comparisons, human readability");
    println!(
        "  - Shape: {} rows × {} columns",
        df_wide.height(),
        df_wide.width()
    );
    println!("  - Use case: Excel export, financial reports, dashboards\n");

    println!("Filtered Export:");
    println!("  - Best for: Focused analysis, reducing data size, specific reports");
    println!(
        "  - Shape: {} rows × {} columns",
        df_filtered.height(),
        df_filtered.width()
    );
    println!("  - Use case: Executive summaries, KPI dashboards\n");

    println!("=== Phase 7 Example Complete ===");
    println!("\nKey Takeaways:");
    println!("  ✓ Long format: Ideal for time-series operations");
    println!("  ✓ Wide format: Ideal for human-readable reports");
    println!("  ✓ Filtered export: Control exactly which nodes to export");
    println!("  ✓ All formats preserve period ordering and metadata");
    println!("  ✓ Seamless integration with Polars DataFrame ecosystem");

    Ok(())
}

#[cfg(not(feature = "polars_export"))]
fn main() {
    eprintln!("This example requires the 'polars_export' feature.");
    eprintln!("Run with: cargo run --example statements_phase7_example --features polars_export");
    std::process::exit(1);
}
