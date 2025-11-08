//! Example demonstrating sensitivity analysis.
//!
//! Run with: cargo run --example sensitivity_analysis_example

use finstack_core::dates::PeriodId;
use finstack_statements::analysis::{
    ParameterSpec, SensitivityAnalyzer, SensitivityConfig, SensitivityMode,
};
use finstack_statements::builder::ModelBuilder;
use finstack_statements::types::AmountOrScalar;
use finstack_statements::Result;

fn main() -> Result<()> {
    println!("=== Sensitivity Analysis Example ===\n");

    // Build a simple revenue model
    let period_q1 = PeriodId::quarter(2025, 1);
    let period_q2 = PeriodId::quarter(2025, 2);

    let model = ModelBuilder::new("acme_corp")
        .periods("2025Q1..Q2", None)?
        .value(
            "revenue",
            &[
                (period_q1, AmountOrScalar::scalar(100_000.0)),
                (period_q2, AmountOrScalar::scalar(100_000.0)),
            ],
        )
        .compute("cogs", "revenue * 0.4")?
        .compute("gross_profit", "revenue - cogs")?
        .build()?;

    // Create sensitivity analyzer
    let analyzer = SensitivityAnalyzer::new(&model);

    // Configure sensitivity analysis
    let mut config = SensitivityConfig::new(SensitivityMode::Diagonal);

    // Add parameter: vary Q1 revenue by ±20%
    config.add_parameter(ParameterSpec::with_percentages(
        "revenue",
        period_q1,
        100_000.0,
        vec![-20.0, -10.0, 0.0, 10.0, 20.0],
    ));

    // Track gross_profit
    config.add_target_metric("gross_profit");

    // Run analysis
    println!("Running diagonal sensitivity analysis...\n");
    let result = analyzer.run(&config)?;

    println!("Generated {} scenarios\n", result.scenarios.len());

    // Display results
    println!("Revenue Sensitivity Results:");
    println!("{:<15} {:<20}", "Revenue (Q1)", "Gross Profit (Q1)");
    println!("{}", "-".repeat(35));

    for scenario in &result.scenarios {
        if let Some(&rev) = scenario.parameter_values.get("revenue") {
            if let Some(gp) = scenario.results.get("gross_profit", &period_q1) {
                println!("{:<15.0} {:<20.0}", rev, gp);
            }
        }
    }

    Ok(())
}
