//! Phase 1 Example: Financial Statement Builder Foundation
//!
//! This example demonstrates the core features implemented in Phase 1:
//! - Type-state builder pattern with compile-time safety
//! - Period integration with actuals/forecast split
//! - Value nodes (explicit period values)
//! - Calculated nodes (formulas, not yet evaluated)
//! - Currency-aware and unitless values
//! - JSON serialization/deserialization
//!
//! Run with: cargo run --example statements_phase1_example

use finstack_statements::prelude::*;

fn main() -> Result<()> {
    println!("🎯 Finstack Statements - Phase 1 Example\n");
    println!("═══════════════════════════════════════════════════════════\n");

    // ========================================================================
    // Example 1: Basic P&L Model with Type-State Builder
    // ========================================================================
    println!("📊 Example 1: Basic P&L Model");
    println!("───────────────────────────────────────────────────────────\n");

    let model = ModelBuilder::new("Acme Corp Q1-Q4 2025")
        // Step 1: Define periods (this transitions from NeedPeriods → Ready state)
        .periods("2025Q1..Q4", Some("2025Q2"))?  // Q1-Q2 actuals, Q3-Q4 forecast
        
        // Step 2: Add value nodes (explicit period values)
        .value("revenue", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(10_000_000.0)),
            (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(11_000_000.0)),
        ])
        
        .value("operating_expenses", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(2_000_000.0)),
            (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(2_100_000.0)),
        ])
        
        // Step 3: Add calculated nodes (formulas will be evaluated in Phase 3)
        .compute("cogs", "revenue * 0.6")?
        .compute("gross_profit", "revenue - cogs")?
        .compute("operating_income", "gross_profit - operating_expenses")?
        .compute("gross_margin", "gross_profit / revenue")?
        
        // Step 4: Add metadata
        .with_meta("author", serde_json::json!("Finance Team"))
        .with_meta("version", serde_json::json!("1.0"))
        
        // Step 5: Build the final specification
        .build()?;

    // Display model structure
    println!("Model ID: {}", model.id);
    println!("Periods: {} total", model.periods.len());
    println!("Nodes: {} total\n", model.nodes.len());

    // Show period details
    println!("Period Breakdown:");
    for period in &model.periods {
        let period_type = if period.is_actual { "Actual  " } else { "Forecast" };
        println!("  {} | {} | {} to {}", 
            period.id, 
            period_type,
            period.start,
            period.end
        );
    }
    println!();

    // Show node details
    println!("Node Structure:");
    for (node_id, node_spec) in &model.nodes {
        let node_type = match node_spec.node_type {
            NodeType::Value => "Value     ",
            NodeType::Calculated => "Calculated",
            NodeType::Mixed => "Mixed     ",
        };
        
        let detail = if node_spec.values.is_some() {
            format!("{} values", node_spec.values.as_ref().unwrap().len())
        } else if let Some(formula) = &node_spec.formula_text {
            format!("formula: {}", formula)
        } else {
            "no data".to_string()
        };
        
        println!("  {:20} | {} | {}", node_id, node_type, detail);
    }
    println!("\n");

    // ========================================================================
    // Example 2: Currency-Aware Values
    // ========================================================================
    println!("💰 Example 2: Currency-Aware Values");
    println!("───────────────────────────────────────────────────────────\n");

    let multi_currency_model = ModelBuilder::new("Global Revenue Model")
        .periods("2025Q1..Q2", None)?
        
        // USD revenue
        .value("usd_revenue", &[
            (PeriodId::quarter(2025, 1), 
             AmountOrScalar::amount(5_000_000.0, Currency::USD)),
            (PeriodId::quarter(2025, 2), 
             AmountOrScalar::amount(5_500_000.0, Currency::USD)),
        ])
        
        // EUR revenue
        .value("eur_revenue", &[
            (PeriodId::quarter(2025, 1), 
             AmountOrScalar::amount(4_000_000.0, Currency::EUR)),
            (PeriodId::quarter(2025, 2), 
             AmountOrScalar::amount(4_200_000.0, Currency::EUR)),
        ])
        
        // Unitless metrics (ratios, percentages)
        .value("growth_rate", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(0.05)),  // 5%
            (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(0.07)),  // 7%
        ])
        
        .build()?;

    println!("Multi-Currency Model: {}", multi_currency_model.id);
    println!("Nodes: {}", multi_currency_model.nodes.len());
    
    // Inspect currency-aware values
    if let Some(node) = multi_currency_model.get_node("usd_revenue") {
        if let Some(values) = &node.values {
            println!("\nUSD Revenue Values:");
            for (period_id, amount) in values {
                println!("  {} = ${:.2} (currency: {:?})", 
                    period_id, 
                    amount.value(),
                    amount.currency()
                );
            }
        }
    }
    
    if let Some(node) = multi_currency_model.get_node("growth_rate") {
        if let Some(values) = &node.values {
            println!("\nGrowth Rate Values (unitless scalars):");
            for (period_id, scalar) in values {
                println!("  {} = {:.1}% (no currency)", 
                    period_id, 
                    scalar.value() * 100.0
                );
            }
        }
    }
    println!("\n");

    // ========================================================================
    // Example 3: Model Structure Inspection
    // ========================================================================
    println!("🔍 Example 3: Model Structure Inspection");
    println!("───────────────────────────────────────────────────────────\n");

    println!("Model Schema Version: {}", model.schema_version);
    println!();
    
    // Inspect a specific node
    if let Some(revenue_node) = model.get_node("revenue") {
        println!("Revenue Node Details:");
        println!("  Type: {:?}", revenue_node.node_type);
        println!("  Has explicit values: {}", revenue_node.values.is_some());
        println!("  Has formula: {}", revenue_node.formula_text.is_some());
        
        if let Some(values) = &revenue_node.values {
            println!("  Period values:");
            for (period_id, value) in values {
                println!("    {} = {:.2}", period_id, value.value());
            }
        }
    }
    println!();
    
    // Inspect a calculated node
    if let Some(gross_profit_node) = model.get_node("gross_profit") {
        println!("Gross Profit Node Details:");
        println!("  Type: {:?}", gross_profit_node.node_type);
        println!("  Formula: {:?}", gross_profit_node.formula_text);
    }
    println!("\n");

    // ========================================================================
    // Example 4: Type-State Pattern Demo
    // ========================================================================
    println!("🔒 Example 4: Compile-Time Type Safety");
    println!("───────────────────────────────────────────────────────────\n");

    println!("The builder uses type-state pattern to prevent invalid usage:");
    println!();
    println!("✓ Valid: ModelBuilder::new() → .periods() → .value() → .build()");
    println!("✗ Invalid: ModelBuilder::new() → .value() (compile error!)");
    println!();
    println!("This ensures you can't add nodes before defining periods.");
    println!("The Rust compiler enforces this at compile-time, not runtime!\n");

    // Valid usage
    let _valid = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None)?
        .value("x", &[(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100.0))])
        .build()?;
    
    println!("✓ Valid model created successfully\n");

    // The following would not compile (uncomment to see):
    // let _invalid = ModelBuilder::new("test")
    //     .value("x", &[(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100.0))])
    //     .build()?;
    // Error: no method named `value` found for struct `ModelBuilder<NeedPeriods>`

    // ========================================================================
    // Example 5: Complex Financial Model
    // ========================================================================
    println!("📈 Example 5: Complete Income Statement Model");
    println!("───────────────────────────────────────────────────────────\n");

    let income_statement = ModelBuilder::new("FY2025 Income Statement")
        .periods("2025Q1..2025Q4", Some("2025Q2"))?
        
        // ─── Revenue ───
        .value("product_revenue", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(8_000_000.0)),
            (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(8_500_000.0)),
        ])
        .value("service_revenue", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(2_000_000.0)),
            (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(2_200_000.0)),
        ])
        .compute("total_revenue", "product_revenue + service_revenue")?
        
        // ─── Cost of Revenue ───
        .compute("product_cogs", "product_revenue * 0.55")?
        .compute("service_cogs", "service_revenue * 0.35")?
        .compute("total_cogs", "product_cogs + service_cogs")?
        
        // ─── Gross Profit ───
        .compute("gross_profit", "total_revenue - total_cogs")?
        .compute("gross_margin_pct", "gross_profit / total_revenue")?
        
        // ─── Operating Expenses ───
        .value("sales_marketing", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(1_500_000.0)),
            (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(1_600_000.0)),
        ])
        .value("research_development", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(1_000_000.0)),
            (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(1_100_000.0)),
        ])
        .value("general_admin", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(800_000.0)),
            (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(850_000.0)),
        ])
        .compute("total_opex", "sales_marketing + research_development + general_admin")?
        
        // ─── Operating Income ───
        .compute("operating_income", "gross_profit - total_opex")?
        .compute("operating_margin_pct", "operating_income / total_revenue")?
        
        // ─── Interest & Taxes (placeholders for future) ───
        .value("interest_expense", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(50_000.0)),
            (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(50_000.0)),
        ])
        .compute("ebt", "operating_income - interest_expense")?
        .compute("taxes", "ebt * 0.25")?  // 25% tax rate
        .compute("net_income", "ebt - taxes")?
        .compute("net_margin_pct", "net_income / total_revenue")?
        
        // Metadata
        .with_meta("prepared_by", serde_json::json!("CFO"))
        .with_meta("fiscal_year", serde_json::json!(2025))
        .with_meta("currency", serde_json::json!("USD"))
        
        .build()?;

    println!("Model: {}", income_statement.id);
    println!("Schema Version: {}", income_statement.schema_version);
    println!();
    
    // Count nodes by type
    let mut value_count = 0;
    let mut calc_count = 0;
    for node in income_statement.nodes.values() {
        match node.node_type {
            NodeType::Value => value_count += 1,
            NodeType::Calculated => calc_count += 1,
            NodeType::Mixed => {},
        }
    }
    
    println!("Summary:");
    println!("  Total Periods: {}", income_statement.periods.len());
    println!("  Actual Periods: {}", 
        income_statement.periods.iter().filter(|p| p.is_actual).count());
    println!("  Forecast Periods: {}", 
        income_statement.periods.iter().filter(|p| !p.is_actual).count());
    println!();
    println!("  Total Nodes: {}", income_statement.nodes.len());
    println!("  Value Nodes: {}", value_count);
    println!("  Calculated Nodes: {}", calc_count);
    println!();

    println!("Metadata:");
    for (key, value) in &income_statement.meta {
        println!("  {}: {}", key, value);
    }

    println!("\n═══════════════════════════════════════════════════════════");
    println!("✅ Phase 1 features demonstrated successfully!");
    println!("═══════════════════════════════════════════════════════════\n");

    println!("Next Steps:");
    println!("  • Phase 2: DSL parser to evaluate formulas");
    println!("  • Phase 3: Evaluator to compute node values");
    println!("  • Phase 4: Forecast methods (growth, forward fill, etc.)");
    println!("  • Phase 5: Dynamic metric registry");
    println!("  • Phase 6: Capital structure integration");
    println!("  • Phase 7: Results export to DataFrames\n");

    Ok(())
}

