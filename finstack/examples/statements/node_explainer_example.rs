//! Example demonstrating node explanation and dependency tracing.
//!
//! Run with: cargo run --example node_explainer_example

use finstack_core::dates::PeriodId;
use finstack_statements::analysis::{DependencyTracer, FormulaExplainer};
use finstack_statements::builder::ModelBuilder;
use finstack_statements::evaluator::{DependencyGraph, Evaluator};
use finstack_statements::types::AmountOrScalar;
use finstack_statements::Result;

fn main() -> Result<()> {
    println!("=== Node Explainer Example ===\n");

    // Build a simple P&L model
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
        .build()?;

    // Evaluate the model
    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model)?;

    // Build dependency graph
    let graph = DependencyGraph::from_model(&model)?;

    // Trace dependencies
    println!("1. Dependency Tracing\n");
    let tracer = DependencyTracer::new(&model, &graph);

    println!("Direct dependencies of 'ebitda':");
    let deps = tracer.direct_dependencies("ebitda")?;
    for dep in deps {
        println!("  - {}", dep);
    }

    println!("\nAll transitive dependencies of 'ebitda':");
    let all_deps = tracer.all_dependencies("ebitda")?;
    for dep in all_deps {
        println!("  - {}", dep);
    }

    println!("\nNodes that depend on 'revenue':");
    let dependents = tracer.dependents("revenue")?;
    for dependent in dependents {
        println!("  - {}", dependent);
    }

    // Show dependency tree
    println!("\n2. Dependency Tree Visualization\n");
    let tree = tracer.dependency_tree("ebitda")?;
    println!("{}", tree.to_string_ascii());
    println!("Tree depth: {}", tree.depth());
    println!("Total nodes: {}", tree.node_count());

    // Explain formula
    println!("\n3. Formula Explanation\n");
    let explainer = FormulaExplainer::new(&model, &results);
    let explanation = explainer.explain("ebitda", &period_q1)?;
    println!("{}", explanation.to_string_detailed());

    println!("\nCompact format:");
    println!("{}", explanation.to_string_compact());

    Ok(())
}
