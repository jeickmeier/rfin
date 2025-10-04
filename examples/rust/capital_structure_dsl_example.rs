//! Capital Structure DSL Integration Example
//!
//! This example demonstrates how to use the `cs.*` namespace in formulas
//! to reference capital structure data (interest expense, principal payments, debt balance).
//!
//! Run with:
//! ```bash
//! cargo run --example capital_structure_dsl_example
//! ```

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, PeriodId};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_statements::builder::ModelBuilder;
use finstack_statements::evaluator::Evaluator;
use finstack_statements::types::AmountOrScalar;
use time::Month;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Capital Structure DSL Integration Example ===\n");

    // Example 1: Parsing cs.* references
    println!("1. Parsing cs.* references:");
    println!("   cs.interest_expense.total");
    println!("   cs.principal_payment.BOND-001");
    println!("   cs.debt_balance.SWAP-001\n");

    use finstack_statements::dsl::{ast::StmtExpr, parse_formula};

    let ast1 = parse_formula("cs.interest_expense.total")?;
    match ast1 {
        StmtExpr::CSRef {
            component,
            instrument_or_total,
        } => {
            println!(
                "   ✓ Parsed as CSRef: component='{}', target='{}'",
                component, instrument_or_total
            );
        }
        _ => println!("   ✗ Unexpected parse result"),
    }

    // Example 2: Building a model with capital structure
    println!("\n2. Building a model with capital structure:");

    let as_of = Date::from_calendar_date(2025, Month::January, 1)?;
    let issue_date = Date::from_calendar_date(2024, Month::July, 1)?; // Issue 6 months before to see coupons
    let maturity_date = Date::from_calendar_date(2030, Month::January, 1)?;

    let model = ModelBuilder::new("LBO Model")
        .periods("2025Q1..2025Q1", Some("2025Q1"))? // Single period for simplicity
        .value(
            "revenue",
            &[(
                PeriodId::quarter(2025, 1),
                AmountOrScalar::scalar(10_000_000.0),
            )],
        )
        .value(
            "cogs",
            &[(
                PeriodId::quarter(2025, 1),
                AmountOrScalar::scalar(6_000_000.0),
            )],
        )
        .value(
            "opex",
            &[(
                PeriodId::quarter(2025, 1),
                AmountOrScalar::scalar(2_000_000.0),
            )],
        )
        .compute("gross_profit", "revenue - cogs")?
        .compute("ebitda", "revenue - cogs - opex")?
        // Add bonds to capital structure
        .add_bond(
            "BOND-001",
            Money::new(50_000_000.0, Currency::USD),
            0.06, // 6% coupon
            issue_date,
            maturity_date,
            "USD-OIS",
        )?
        .add_bond(
            "BOND-002",
            Money::new(25_000_000.0, Currency::USD),
            0.08, // 8% coupon
            issue_date,
            maturity_date,
            "USD-OIS",
        )?
        .build()?;

    println!("   ✓ Model built with {} nodes", model.nodes.len());
    println!(
        "   ✓ Capital structure defined: {} debt instruments",
        model
            .capital_structure
            .as_ref()
            .unwrap()
            .debt_instruments
            .len()
    );

    // Example 3: Create market context for pricing
    println!("\n3. Creating market context:");

    let discount_curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([
            (0.0, 1.0),
            (0.25, 0.985), // 3M: ~6% rate
            (0.5, 0.97),   // 6M: ~6% rate
            (1.0, 0.94),   // 1Y: ~6.2% rate
            (5.0, 0.74),   // 5Y: ~6% rate
        ])
        .set_interp(InterpStyle::Linear)
        .build()?;

    let market_ctx = MarketContext::new().insert_discount(discount_curve);
    println!("   ✓ Market context created with discount curve: USD-OIS");

    // Example 4: Evaluate model with automatic cashflow computation
    println!("\n4. Evaluating model (cashflows computed from bond definitions):");

    let mut evaluator = Evaluator::new();
    let results =
        evaluator.evaluate_with_market_context(&model, false, Some(&market_ctx), Some(as_of))?;

    println!(
        "   ✓ Model evaluated: {} nodes, {} periods",
        results.meta.num_nodes, results.meta.num_periods
    );

    // Display Q1 results
    let q1 = PeriodId::quarter(2025, 1);
    println!("\n   Q1 2025 Results:");
    for (node_id, period_values) in &results.nodes {
        if let Some(value) = period_values.get(&q1) {
            println!("     {}: ${:.2}", node_id, value);
        }
    }

    // Example 5: Formulas with cs.* references
    println!("\n5. Formula examples with cs.* references:");
    println!("   - Net income: revenue - cogs - opex - cs.interest_expense.total");
    println!("   - Interest coverage: ebitda / cs.interest_expense.total");
    println!("   - Debt service coverage: ebitda / (cs.interest_expense.total + cs.principal_payment.total)");
    println!("   - Leverage: cs.debt_balance.total / ebitda");

    // Example 6: Build extended model with cs.* formulas
    println!("\n6. Building model with cs.* formulas:");

    let extended_model = ModelBuilder::new("LBO Model Extended")
        .periods("2025Q1..2025Q1", Some("2025Q1"))? // Single period for simplicity
        .value(
            "revenue",
            &[(
                PeriodId::quarter(2025, 1),
                AmountOrScalar::scalar(10_000_000.0),
            )],
        )
        .value(
            "cogs",
            &[(
                PeriodId::quarter(2025, 1),
                AmountOrScalar::scalar(6_000_000.0),
            )],
        )
        .value(
            "opex",
            &[(
                PeriodId::quarter(2025, 1),
                AmountOrScalar::scalar(2_000_000.0),
            )],
        )
        .compute("gross_profit", "revenue - cogs")?
        .compute("ebitda", "revenue - cogs - opex")?
        .add_bond(
            "BOND-001",
            Money::new(50_000_000.0, Currency::USD),
            0.06,
            issue_date,
            maturity_date,
            "USD-OIS",
        )?
        .add_bond(
            "BOND-002",
            Money::new(25_000_000.0, Currency::USD),
            0.08,
            issue_date,
            maturity_date,
            "USD-OIS",
        )?
        // Now add formulas that reference capital structure
        .compute("interest_expense", "cs.interest_expense.total")?
        .compute("net_income", "ebitda - cs.interest_expense.total")?
        .compute("total_debt", "cs.debt_balance.total")?
        .compute("leverage_ratio", "cs.debt_balance.total / ebitda")?
        .build()?;

    println!("   ✓ Extended model built with cs.* formulas");

    // Evaluate extended model
    let mut extended_evaluator = Evaluator::new();
    let extended_results = extended_evaluator.evaluate_with_market_context(
        &extended_model,
        false,
        Some(&market_ctx),
        Some(as_of),
    )?;

    println!("\n   Extended model results for Q1 2025:");
    let q1 = PeriodId::quarter(2025, 1);

    // Display key metrics
    for node_id in &[
        "interest_expense",
        "net_income",
        "total_debt",
        "leverage_ratio",
    ] {
        if let Some(value) = extended_results.get(node_id, &q1) {
            if *node_id == "leverage_ratio" {
                println!("     {}: {:.2}x", node_id, value);
            } else {
                println!("     {}: ${:.2}", node_id, value);
            }
        }
    }

    println!("\n=== Example Complete ===");
    println!("\nKey features demonstrated:");
    println!("  ✓ Cashflows are computed automatically from bond definitions");
    println!("  ✓ No manual cashflow hardcoding required");
    println!("  ✓ cs.* references work in formulas");
    println!("  ✓ Interest expense, principal, and debt balance tracked per instrument");
    println!("  ✓ Market context integration for proper bond pricing");

    Ok(())
}
