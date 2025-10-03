//! Phase 3 Example — Evaluator
//!
//! This example demonstrates the Phase 3 features of the finstack-statements crate:
//! - Evaluation context
//! - Basic evaluator with formula evaluation
//! - DAG construction and topological sorting
//! - Precedence resolution (Value > Forecast > Formula)
//! - Circular dependency detection
//! - Complete P&L model evaluation

use finstack_statements::prelude::*;

fn main() -> Result<()> {
    println!("=== Phase 3: Evaluator Examples ===\n");

    // Example 1: Simple Evaluation
    example_1_simple_evaluation()?;

    // Example 2: Arithmetic Operations
    example_2_arithmetic_operations()?;

    // Example 3: Dependency Chain
    example_3_dependency_chain()?;

    // Example 4: Precedence Resolution
    example_4_precedence_resolution()?;

    // Example 5: Complete P&L Model
    example_5_complete_pl_model()?;

    // Example 6: Circular Dependency Detection
    example_6_circular_dependency()?;

    // Example 7: Multi-Period Evaluation
    example_7_multi_period_evaluation()?;

    Ok(())
}

/// Example 1: Simple Evaluation
///
/// Demonstrates basic model evaluation with value nodes and calculated nodes.
fn example_1_simple_evaluation() -> Result<()> {
    println!("📊 Example 1: Simple Evaluation");
    println!("--------------------------------");

    let model = ModelBuilder::new("Simple Model")
        .periods("2025Q1..Q2", None)?
        .value(
            "revenue",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(100_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(110_000.0),
                ),
            ],
        )
        .value(
            "cogs",
            &[
                (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(60_000.0)),
                (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(66_000.0)),
            ],
        )
        .build()?;

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false)?;

    println!("Results:");
    for period in &model.periods {
        let revenue = results.get("revenue", &period.id).unwrap();
        let cogs = results.get("cogs", &period.id).unwrap();
        println!(
            "  {} → Revenue: {:.0}, COGS: {:.0}",
            period.id, revenue, cogs
        );
    }

    println!("\n✅ Simple evaluation complete\n");
    Ok(())
}

/// Example 2: Arithmetic Operations
///
/// Demonstrates formula evaluation with arithmetic operations.
fn example_2_arithmetic_operations() -> Result<()> {
    println!("📊 Example 2: Arithmetic Operations");
    println!("------------------------------------");

    let model = ModelBuilder::new("Arithmetic Model")
        .periods("2025Q1..Q2", None)?
        .value(
            "revenue",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(100_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(110_000.0),
                ),
            ],
        )
        .compute("cogs", "revenue * 0.6")?
        .compute("gross_profit", "revenue - cogs")?
        .compute("gross_margin", "gross_profit / revenue")?
        .build()?;

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false)?;

    println!("Results:");
    for period in &model.periods {
        let revenue = results.get("revenue", &period.id).unwrap();
        let cogs = results.get("cogs", &period.id).unwrap();
        let gross_profit = results.get("gross_profit", &period.id).unwrap();
        let gross_margin = results.get("gross_margin", &period.id).unwrap();

        println!("  {}:", period.id);
        println!("    Revenue:       {:>12.0}", revenue);
        println!("    COGS:          {:>12.0}", cogs);
        println!("    Gross Profit:  {:>12.0}", gross_profit);
        println!("    Gross Margin:  {:>11.2}%", gross_margin * 100.0);
    }

    println!("\n✅ Arithmetic operations evaluated\n");
    Ok(())
}

/// Example 3: Dependency Chain
///
/// Demonstrates DAG construction and topological sorting.
fn example_3_dependency_chain() -> Result<()> {
    println!("📊 Example 3: Dependency Chain");
    println!("--------------------------------");

    let model = ModelBuilder::new("Dependency Chain")
        .periods("2025Q1..Q1", None)?
        .value(
            "base",
            &[(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100.0))],
        )
        .compute("step1", "base * 2")?
        .compute("step2", "step1 + 10")?
        .compute("step3", "step2 * 3")?
        .compute("final_result", "step3 - 50")?
        .build()?;

    println!("Dependency chain: base → step1 → step2 → step3 → final_result");

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false)?;

    let period = PeriodId::quarter(2025, 1);
    println!("\nEvaluation order:");
    println!(
        "  base         = {:.0}",
        results.get("base", &period).unwrap()
    );
    println!(
        "  step1        = base * 2     = {:.0}",
        results.get("step1", &period).unwrap()
    );
    println!(
        "  step2        = step1 + 10   = {:.0}",
        results.get("step2", &period).unwrap()
    );
    println!(
        "  step3        = step2 * 3    = {:.0}",
        results.get("step3", &period).unwrap()
    );
    println!(
        "  final_result = step3 - 50   = {:.0}",
        results.get("final_result", &period).unwrap()
    );

    println!("\n✅ Dependency chain evaluated in topological order\n");
    Ok(())
}

/// Example 4: Precedence Resolution
///
/// Demonstrates that explicit values take precedence.
/// Note: Full precedence (Value > Forecast > Formula) for Mixed nodes
/// will be available in Phase 4 when forecast methods are implemented.
fn example_4_precedence_resolution() -> Result<()> {
    println!("📊 Example 4: Value vs Calculated Nodes");
    println!("----------------------------------------");

    let model = ModelBuilder::new("Precedence Model")
        .periods("2025Q1..Q4", Some("2025Q2"))?
        // Revenue: explicit values for all periods (Value node)
        .value(
            "revenue",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(100_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(110_000.0),
                ),
                (
                    PeriodId::quarter(2025, 3),
                    AmountOrScalar::scalar(115_000.0),
                ),
                (
                    PeriodId::quarter(2025, 4),
                    AmountOrScalar::scalar(120_000.0),
                ),
            ],
        )
        // COGS: always calculated from revenue (Calculated node)
        .compute("cogs", "revenue * 0.6")?
        // Gross profit: calculated (Calculated node)
        .compute("gross_profit", "revenue - cogs")?
        .build()?;

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false)?;

    println!("Node types:");
    println!("  revenue       → Value node (explicit values)");
    println!("  cogs          → Calculated node (formula: revenue * 0.6)");
    println!("  gross_profit  → Calculated node (formula: revenue - cogs)");
    println!("\nResults:");
    println!(
        "{:<10} {:>12} {:>12} {:>12}",
        "Period", "Revenue", "COGS", "Gross Profit"
    );
    println!("{}", "-".repeat(50));

    for period in &model.periods {
        let revenue = results.get("revenue", &period.id).unwrap();
        let cogs = results.get("cogs", &period.id).unwrap();
        let gross_profit = results.get("gross_profit", &period.id).unwrap();
        println!(
            "{:<10} {:>12.0} {:>12.0} {:>12.0}",
            period.id, revenue, cogs, gross_profit
        );
    }

    println!("\n✅ Value nodes always use explicit values");
    println!("✅ Calculated nodes derive from formulas\n");
    Ok(())
}

/// Example 5: Complete P&L Model
///
/// Demonstrates a realistic profit & loss statement.
fn example_5_complete_pl_model() -> Result<()> {
    println!("📊 Example 5: Complete P&L Model");
    println!("----------------------------------");

    let model = ModelBuilder::new("Acme Corp P&L")
        .periods("2025Q1..Q2", None)?
        // Revenue
        .value(
            "revenue",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(10_000_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(11_000_000.0),
                ),
            ],
        )
        // Cost of goods sold (60% of revenue)
        .compute("cogs", "revenue * 0.6")?
        // Operating expenses
        .value(
            "opex",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(2_000_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(2_100_000.0),
                ),
            ],
        )
        // Derived metrics
        .compute("gross_profit", "revenue - cogs")?
        .compute("operating_income", "gross_profit - opex")?
        .compute("gross_margin", "gross_profit / revenue")?
        .compute("operating_margin", "operating_income / revenue")?
        .build()?;

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false)?;

    println!("Acme Corp - Profit & Loss Statement");
    println!("\n{:<25} {:>15} {:>15}", "Metric", "2025Q1", "2025Q2");
    println!("{}", "-".repeat(57));

    let metrics = vec![
        ("Revenue", "revenue"),
        ("Cost of Goods Sold", "cogs"),
        ("Gross Profit", "gross_profit"),
        ("Operating Expenses", "opex"),
        ("Operating Income", "operating_income"),
    ];

    for (label, node_id) in &metrics {
        let q1 = results.get(node_id, &PeriodId::quarter(2025, 1)).unwrap();
        let q2 = results.get(node_id, &PeriodId::quarter(2025, 2)).unwrap();
        println!("{:<25} {:>15.0} {:>15.0}", label, q1, q2);
    }

    println!("{}", "-".repeat(57));

    // Margins
    let margins = vec![
        ("Gross Margin %", "gross_margin"),
        ("Operating Margin %", "operating_margin"),
    ];

    for (label, node_id) in &margins {
        let q1 = results.get(node_id, &PeriodId::quarter(2025, 1)).unwrap() * 100.0;
        let q2 = results.get(node_id, &PeriodId::quarter(2025, 2)).unwrap() * 100.0;
        println!("{:<25} {:>14.1}% {:>14.1}%", label, q1, q2);
    }

    println!("\nEvaluation metadata:");
    println!("  Nodes evaluated:    {}", results.meta.num_nodes);
    println!("  Periods evaluated:  {}", results.meta.num_periods);
    if let Some(time_ms) = results.meta.eval_time_ms {
        println!("  Evaluation time:    {}ms", time_ms);
    }

    println!("\n✅ Complete P&L model evaluated\n");
    Ok(())
}

/// Example 6: Circular Dependency Detection
///
/// Demonstrates circular dependency detection.
fn example_6_circular_dependency() -> Result<()> {
    println!("📊 Example 6: Circular Dependency Detection");
    println!("--------------------------------------------");

    println!("Creating a model with circular dependency:");
    println!("  a = b + 1");
    println!("  b = c + 1");
    println!("  c = a + 1  ← Creates cycle!");

    let result = ModelBuilder::new("Circular Model")
        .periods("2025Q1..Q1", None)?
        .compute("a", "b + 1")?
        .compute("b", "c + 1")?
        .compute("c", "a + 1")? // Creates circular dependency
        .build();

    match result {
        Ok(model) => {
            // Model builds successfully, but evaluation should fail
            let mut evaluator = Evaluator::new();
            match evaluator.evaluate(&model, false) {
                Err(Error::CircularDependency(path)) => {
                    println!("\n✅ Circular dependency detected!");
                    println!("   Cycle path: {}", path.join(" → "));
                }
                Err(e) => println!("\n❌ Unexpected error: {}", e),
                Ok(_) => println!("\n❌ Should have detected circular dependency!"),
            }
        }
        Err(e) => {
            println!(
                "\n✅ Error during build (may catch circular dependency early): {}",
                e
            );
        }
    }

    println!();
    Ok(())
}

/// Example 7: Multi-Period Evaluation
///
/// Demonstrates sequential period evaluation.
fn example_7_multi_period_evaluation() -> Result<()> {
    println!("📊 Example 7: Multi-Period Evaluation");
    println!("--------------------------------------");

    let model = ModelBuilder::new("Multi-Period Model")
        .periods("2025Q1..Q4", Some("2025Q1"))?
        // Q1 has actual value
        .value(
            "revenue",
            &[(
                PeriodId::quarter(2025, 1),
                AmountOrScalar::scalar(1_000_000.0),
            )],
        )
        // Q2-Q4 will use formula fallback
        .compute("revenue", "1100000")?
        // COGS is 60% of revenue
        .compute("cogs", "revenue * 0.6")?
        // Gross profit margin
        .compute("gross_profit", "revenue - cogs")?
        .compute("margin", "gross_profit / revenue")?
        .build()?;

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false)?;

    println!("Sequential period evaluation:\n");
    println!(
        "{:<10} {:>15} {:>15} {:>15} {:>10}",
        "Period", "Revenue", "COGS", "Gross Profit", "Margin %"
    );
    println!("{}", "-".repeat(70));

    for period in &model.periods {
        let revenue = results.get("revenue", &period.id).unwrap();
        let cogs = results.get("cogs", &period.id).unwrap();
        let gross_profit = results.get("gross_profit", &period.id).unwrap();
        let margin = results.get("margin", &period.id).unwrap() * 100.0;

        println!(
            "{:<10} {:>15.0} {:>15.0} {:>15.0} {:>9.1}%",
            period.id, revenue, cogs, gross_profit, margin
        );
    }

    println!("\n✅ Multi-period evaluation complete\n");
    Ok(())
}
