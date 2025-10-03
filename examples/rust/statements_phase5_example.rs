//! Phase 5 Example: Dynamic Metric Registry
//!
//! This example demonstrates the dynamic metric registry system that allows
//! loading financial metrics from JSON files without recompiling.
//!
//! Run with: cargo run --example statements_phase5_example

use finstack_statements::prelude::*;

fn main() -> Result<()> {
    println!("=== Phase 5: Dynamic Metric Registry Example ===\n");

    // -------------------------------------------------------------------------
    // 1. Load Built-in Metrics
    // -------------------------------------------------------------------------
    println!("1. Loading Built-in Metrics (fin.* namespace)");
    println!("   Built-in metrics are embedded at compile time from JSON files\n");

    let mut registry = Registry::new();
    registry.load_builtins()?;

    println!("   ✓ Loaded {} metrics", registry.len());
    println!("   ✓ Namespaces: {:?}\n", registry.namespaces());

    // -------------------------------------------------------------------------
    // 2. Explore Built-in Metrics
    // -------------------------------------------------------------------------
    println!("2. Exploring Built-in Metrics");
    println!("   Listing all fin.* metrics:\n");

    let mut fin_metrics: Vec<_> = registry.namespace("fin").collect();
    fin_metrics.sort_by_key(|(id, _)| *id);

    for (qualified_id, stored_metric) in fin_metrics.iter().take(6) {
        println!(
            "   • {:<30} {}",
            format!("{}:", qualified_id),
            stored_metric.definition.name
        );
        println!("     Formula: {}", stored_metric.definition.formula);
        if let Some(desc) = &stored_metric.definition.description {
            println!("     Description: {}", desc);
        }
        println!();
    }

    // -------------------------------------------------------------------------
    // 3. Create a Custom Metric Registry (JSON)
    // -------------------------------------------------------------------------
    println!("3. Creating Custom Metrics from JSON");
    println!("   Custom metrics can be defined in JSON and loaded at runtime\n");

    let custom_json = r#"{
        "namespace": "custom",
        "schema_version": 1,
        "metrics": [
            {
                "id": "total_revenue",
                "name": "Total Revenue",
                "formula": "revenue + other_income",
                "description": "Sum of all revenue sources",
                "category": "income_statement",
                "unit_type": "currency",
                "requires": ["revenue", "other_income"],
                "tags": ["revenue", "top_line"]
            },
            {
                "id": "total_expenses",
                "name": "Total Expenses",
                "formula": "cogs + opex + interest_expense",
                "description": "Sum of all operating expenses",
                "category": "income_statement",
                "unit_type": "currency",
                "requires": ["cogs", "opex", "interest_expense"],
                "tags": ["expenses"]
            },
            {
                "id": "profit_margin",
                "name": "Profit Margin %",
                "formula": "(revenue + other_income - cogs - opex - interest_expense) / (revenue + other_income)",
                "description": "Net profit as percentage of total revenue",
                "category": "margins",
                "unit_type": "percentage",
                "requires": ["revenue", "other_income", "cogs", "opex", "interest_expense"],
                "tags": ["margins", "profitability"]
            }
        ]
    }"#;

    registry.load_from_json_str(custom_json)?;

    println!("   ✓ Loaded custom metrics into 'custom' namespace");
    println!("   ✓ Total namespaces: {:?}\n", registry.namespaces());

    // List custom metrics
    println!("   Custom metrics:");
    for (qualified_id, stored_metric) in registry.namespace("custom") {
        println!(
            "   • {:<30} {}",
            format!("{}:", qualified_id),
            stored_metric.definition.name
        );
    }
    println!();

    // -------------------------------------------------------------------------
    // 4. Build Model with Select Built-in Metrics
    // -------------------------------------------------------------------------
    println!("4. Building Model with Built-in Metrics from Registry");
    println!("   Using registry metrics for income statement analysis\n");

    let model_with_registry = ModelBuilder::new("P&L with Registry Metrics")
        .periods("2025Q1..Q2", None)?
        // Base metrics
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
        .value(
            "cogs",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(6_000_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(6_600_000.0),
                ),
            ],
        )
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
        .value(
            "depreciation",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(200_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(200_000.0),
                ),
            ],
        )
        .value(
            "amortization",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(100_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(100_000.0),
                ),
            ],
        )
        .value(
            "interest_expense",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(150_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(145_000.0),
                ),
            ],
        )
        .value(
            "taxes",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(400_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(470_000.0),
                ),
            ],
        )
        // Add specific metrics from registry (only those with satisfied dependencies)
        .add_metric_from_registry("fin.gross_profit", &registry)?
        .add_metric_from_registry("fin.gross_margin", &registry)?
        .add_metric_from_registry("fin.operating_income", &registry)?
        .add_metric_from_registry("fin.operating_margin", &registry)?
        .add_metric_from_registry("fin.ebitda", &registry)?
        .add_metric_from_registry("fin.ebitda_margin", &registry)?
        .add_metric_from_registry("fin.ebit", &registry)?
        .add_metric_from_registry("fin.ebt", &registry)?
        .add_metric_from_registry("fin.net_income", &registry)?
        .add_metric_from_registry("fin.net_margin", &registry)?
        .add_metric_from_registry("fin.cogs_as_pct_revenue", &registry)?
        .add_metric_from_registry("fin.opex_as_pct_revenue", &registry)?
        .build()?;

    println!(
        "   ✓ Model has {} nodes (base + registry metrics)",
        model_with_registry.nodes.len()
    );

    // Evaluate
    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model_with_registry, false)?;

    println!("\n   Sample Results (2025Q1):");
    let q1 = PeriodId::quarter(2025, 1);
    println!(
        "   • Revenue:            ${:>12.2}",
        results.get("revenue", &q1).unwrap()
    );
    println!(
        "   • Gross Profit:       ${:>12.2}",
        results.get("fin.gross_profit", &q1).unwrap()
    );
    println!(
        "   • Gross Margin:       {:>13.1}%",
        results.get("fin.gross_margin", &q1).unwrap() * 100.0
    );
    println!(
        "   • EBITDA:             ${:>12.2}",
        results.get("fin.ebitda", &q1).unwrap()
    );
    println!(
        "   • EBITDA Margin:      {:>13.1}%",
        results.get("fin.ebitda_margin", &q1).unwrap() * 100.0
    );
    println!(
        "   • Net Income:         ${:>12.2}",
        results.get("fin.net_income", &q1).unwrap()
    );
    println!(
        "   • Net Margin:         {:>13.1}%",
        results.get("fin.net_margin", &q1).unwrap() * 100.0
    );
    println!();

    // -------------------------------------------------------------------------
    // 5. Build Model with Selective Metrics
    // -------------------------------------------------------------------------
    println!("5. Building Model with Selective Metrics");
    println!("   Using .add_metric_from_registry() for fine-grained control\n");

    let model_selective = ModelBuilder::new("P&L with Select Metrics")
        .periods("2025Q1..Q2", None)?
        // Base metrics
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
        .value(
            "cogs",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(6_000_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(6_600_000.0),
                ),
            ],
        )
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
        .value(
            "other_income",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(100_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(120_000.0),
                ),
            ],
        )
        .value(
            "interest_expense",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(150_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(145_000.0),
                ),
            ],
        )
        // Add only specific metrics we need
        .add_metric_from_registry("fin.gross_profit", &registry)?
        .add_metric_from_registry("fin.gross_margin", &registry)?
        .add_metric_from_registry("fin.operating_income", &registry)?
        .add_metric_from_registry("fin.operating_margin", &registry)?
        // Add custom metrics
        .add_metric_from_registry("custom.total_revenue", &registry)?
        .add_metric_from_registry("custom.total_expenses", &registry)?
        .add_metric_from_registry("custom.profit_margin", &registry)?
        .build()?;

    println!(
        "   ✓ Model has {} nodes (base + selected metrics)",
        model_selective.nodes.len()
    );

    // Evaluate
    let results = evaluator.evaluate(&model_selective, false)?;

    println!("\n   Results Comparison:");
    println!("   Period          | 2025Q1         | 2025Q2");
    println!("   ----------------|----------------|----------------");

    for node_id in &[
        "revenue",
        "fin.gross_profit",
        "fin.gross_margin",
        "custom.total_revenue",
        "custom.profit_margin",
    ] {
        let q1_val = results.get(node_id, &PeriodId::quarter(2025, 1)).unwrap();
        let q2_val = results.get(node_id, &PeriodId::quarter(2025, 2)).unwrap();

        let (q1_fmt, q2_fmt) = if node_id.contains("margin") {
            (
                format!("{:>13.1}%", q1_val * 100.0),
                format!("{:>13.1}%", q2_val * 100.0),
            )
        } else {
            (format!("${:>12.0}", q1_val), format!("${:>12.0}", q2_val))
        };

        println!("   {:<15} | {} | {}", node_id, q1_fmt, q2_fmt);
    }
    println!();

    // -------------------------------------------------------------------------
    // 6. Namespace Isolation Demo
    // -------------------------------------------------------------------------
    println!("6. Namespace Isolation");
    println!("   Demonstrating how namespaces prevent metric collisions\n");

    println!(
        "   Metrics in 'fin' namespace:    {}",
        registry.namespace("fin").count()
    );
    println!(
        "   Metrics in 'custom' namespace: {}",
        registry.namespace("custom").count()
    );
    println!("   Total metrics in registry:     {}", registry.len());
    println!();

    println!("   ✓ Namespaces ensure no collisions between different metric libraries");
    println!("   ✓ Metrics are referenced by qualified ID: 'namespace.metric_id'");
    println!();

    // -------------------------------------------------------------------------
    // 7. Summary
    // -------------------------------------------------------------------------
    println!("=== Phase 5 Summary ===\n");
    println!("✓ Dynamic Registry: Load metrics from JSON without recompiling");
    println!("✓ Built-in Metrics:  22 standard financial metrics in 'fin' namespace");
    println!("✓ Custom Metrics:    Define your own metrics in JSON");
    println!("✓ Namespaces:        Prevent collisions, organize metrics logically");
    println!("✓ Builder Integration: Easy API to add metrics to models");
    println!("✓ Formula Caching:   Metrics compiled once for performance");
    println!();

    println!("Next: Phase 6 will add Capital Structure integration for debt tracking");

    Ok(())
}
