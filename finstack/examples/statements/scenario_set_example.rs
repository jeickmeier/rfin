//! Example demonstrating multi-scenario management and comparison.
//!
//! Run with:
//! ```bash
//! cargo run --example scenario_set_example -p finstack-statements
//! ```
//!
//! To enable the optional Polars comparison DataFrame export, run:
//!
//! ```bash
//! cargo run --example scenario_set_example -p finstack-statements --features dataframes
//! ```

use finstack_core::dates::PeriodId;
use finstack_statements::analysis::{ScenarioDefinition, ScenarioSet};
use finstack_statements::builder::ModelBuilder;
use finstack_statements::types::AmountOrScalar;
use finstack_statements::Result;
use indexmap::IndexMap;

fn main() -> Result<()> {
    println!("=== Scenario Set Example ===\n");

    // Build a simple revenue model
    let period_q1 = PeriodId::quarter(2025, 1);
    let period_q2 = PeriodId::quarter(2025, 2);

    let model = ModelBuilder::new("acme_scenarios")
        .periods("2025Q1..Q2", None)?
        .value(
            "revenue",
            &[
                (period_q1, AmountOrScalar::scalar(100_000.0)),
                (period_q2, AmountOrScalar::scalar(100_000.0)),
            ],
        )
        .compute("cogs", "revenue * 0.4")?
        .compute("ebitda", "revenue - cogs")?
        .build()?;

    // Define scenarios
    let mut scenarios = IndexMap::new();

    // Base scenario: no overrides
    scenarios.insert(
        "base".to_string(),
        ScenarioDefinition {
            model_id: Some(model.id.clone()),
            parent: None,
            overrides: IndexMap::new(),
        },
    );

    // Downside scenario: lower revenue across all periods
    let mut downside_overrides = IndexMap::new();
    downside_overrides.insert("revenue".to_string(), 90_000.0);
    scenarios.insert(
        "downside".to_string(),
        ScenarioDefinition {
            model_id: Some(model.id.clone()),
            parent: Some("base".to_string()),
            overrides: downside_overrides,
        },
    );

    // Stress scenario: even lower revenue, inheriting from downside
    let mut stress_overrides = IndexMap::new();
    stress_overrides.insert("revenue".to_string(), 80_000.0);
    scenarios.insert(
        "stress".to_string(),
        ScenarioDefinition {
            model_id: Some(model.id.clone()),
            parent: Some("downside".to_string()),
            overrides: stress_overrides,
        },
    );

    let set = ScenarioSet { scenarios };
    let results = set.evaluate_all(&model)?;

    println!("Scenarios evaluated: {}", results.len());
    let period = period_q1;
    for (name, scenario_results) in &results.scenarios {
        let revenue = scenario_results
            .get("revenue", &period)
            .unwrap_or_default();
        let ebitda = scenario_results
            .get("ebitda", &period)
            .unwrap_or_default();

        println!(
            "  {:<8} -> revenue @ {} = {:>8.0}, ebitda = {:>8.0}",
            name, period, revenue, ebitda
        );
    }

    #[cfg(feature = "dataframes")]
    {
        use finstack_statements::analysis::ScenarioResults as _;

        println!("\nComparison DataFrame (metrics = revenue, ebitda):\n");
        let df = results.to_comparison_df(&["revenue", "ebitda"])?;
        println!("{df}");
    }

    Ok(())
}


