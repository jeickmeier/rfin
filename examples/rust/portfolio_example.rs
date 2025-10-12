//! Comprehensive portfolio example demonstrating:
//! - Entity-based and standalone instrument positions
//! - Portfolio valuation and aggregation
//! - Cross-currency FX conversion
//! - Attribute-based grouping
//! - Scenario application and re-valuation
//! - DataFrame exports

use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::fx::providers::SimpleFxProvider;
use finstack_core::money::fx::FxMatrix;
use finstack_core::prelude::*;
use finstack_portfolio::{self, *};
use finstack_scenarios::spec::{CurveKind, OperationSpec, ScenarioSpec};
use finstack_valuations::instruments::deposit::Deposit;
use std::sync::Arc;
use time::macros::date;

fn main() -> finstack_portfolio::Result<()> {
    println!("=== Finstack Portfolio Example ===\n");

    // 1. Create market data
    let as_of = date!(2024 - 01 - 01);
    let market = build_market_data(as_of);
    let config = FinstackConfig::default();

    // 2. Build portfolio with mixed instruments
    let portfolio = build_sample_portfolio(as_of)?;
    println!("Created portfolio '{}' with:", portfolio.id);
    println!("  - {} entities", portfolio.entities.len());
    println!("  - {} positions", portfolio.positions.len());
    println!("  - Base currency: {}\n", portfolio.base_ccy);

    // 3. Value the portfolio
    println!("--- Portfolio Valuation ---");
    let valuation = value_portfolio(&portfolio, &market, &config)?;
    println!("Total value: {}", valuation.total_base_ccy);
    println!("\nBy entity:");
    for (entity_id, total) in &valuation.by_entity {
        println!("  {}: {}", entity_id, total);
    }

    // 4. Aggregate metrics
    println!("\n--- Metrics Aggregation ---");
    let metrics = aggregate_metrics(&valuation)?;
    println!("Aggregated {} metrics", metrics.aggregated.len());
    println!("Position-level metrics: {} positions", metrics.by_position.len());
    
    // Show aggregated metrics
    if !metrics.aggregated.is_empty() {
        println!("\nAggregated metrics:");
        for (metric_id, agg_metric) in &metrics.aggregated {
            println!("  {}: {:.2}", metric_id, agg_metric.total);
            println!("    By entity:");
            for (entity_id, value) in &agg_metric.by_entity {
                println!("      {}: {:.2}", entity_id, value);
            }
        }
    }
    
    // Show position-level metrics
    if !metrics.by_position.is_empty() {
        println!("\nPosition-level metrics:");
        for (position_id, position_metrics) in metrics.by_position.iter().take(3) {
            println!("  {}:", position_id);
            for (metric_id, value) in position_metrics {
                println!("    {}: {:.4}", metric_id, value);
            }
        }
        if metrics.by_position.len() > 3 {
            println!("  ... and {} more positions", metrics.by_position.len() - 3);
        }
    }

    // 5. Group by attributes
    println!("\n--- Attribute-Based Grouping ---");
    let by_rating = aggregate_by_attribute(
        &valuation,
        &portfolio.positions,
        "rating",
        portfolio.base_ccy,
    )?;
    println!("By rating:");
    for (rating, total) in &by_rating {
        println!("  {}: {}", rating, total);
    }

    let by_instrument_type = aggregate_by_attribute(
        &valuation,
        &portfolio.positions,
        "instrument_type",
        portfolio.base_ccy,
    )?;
    println!("\nBy instrument type:");
    for (inst_type, total) in &by_instrument_type {
        println!("  {}: {}", inst_type, total);
    }

    // 6. Apply scenario and re-value
    println!("\n--- Scenario Application ---");
    let scenario = ScenarioSpec {
        id: "stress_test".to_string(),
        name: Some("Rate Stress (+50bp)".to_string()),
        description: Some("Parallel 50bp shift to USD discount curve".to_string()),
        operations: vec![OperationSpec::CurveParallelBp {
            curve_kind: CurveKind::Discount,
            curve_id: "USD".to_string(),
            bp: 50.0,
        }],
        priority: 0,
    };

    println!("Scenario: {}", scenario.name.as_ref().unwrap());
    println!("Description: {}", scenario.description.as_ref().unwrap());
    println!("Operations:");
    for (i, op) in scenario.operations.iter().enumerate() {
        match op {
            OperationSpec::CurveParallelBp { curve_kind, curve_id, bp } => {
                println!("  {}. {:?} curve '{}': {:+.0}bp parallel shift", i + 1, curve_kind, curve_id, bp);
            }
            _ => println!("  {}. {:?}", i + 1, op),
        }
    }

    let (stressed_valuation, report) =
        apply_and_revalue(&portfolio, &scenario, &market, &config)?;

    println!("\nScenario Results:");
    println!("  Operations applied: {}", report.operations_applied);
    if !report.warnings.is_empty() {
        println!("  Warnings: {}", report.warnings.len());
        for warning in &report.warnings {
            println!("    - {}", warning);
        }
    }
    
    println!("\nValue Impact:");
    println!("  Base case value:    {}", valuation.total_base_ccy);
    println!("  Stressed value:     {}", stressed_valuation.total_base_ccy);
    let delta = stressed_valuation.total_base_ccy.checked_sub(valuation.total_base_ccy)?;
    let pct_change = if valuation.total_base_ccy.amount() != 0.0 {
        (delta.amount() / valuation.total_base_ccy.amount().abs()) * 100.0
    } else {
        0.0
    };
    println!("  Change:             {} ({:+.2}%)", delta, pct_change);
    
    // Show per-entity impact
    println!("\nPer-Entity Impact:");
    for (entity_id, base_value) in &valuation.by_entity {
        if let Some(stressed_value) = stressed_valuation.by_entity.get(entity_id) {
            let entity_delta = stressed_value.checked_sub(*base_value)?;
            println!("  {}: {} → {} (change: {})", 
                entity_id, base_value, stressed_value, entity_delta);
        }
    }
    
    // Show metrics impact if available
    let stressed_metrics = aggregate_metrics(&stressed_valuation)?;
    if !metrics.aggregated.is_empty() && !stressed_metrics.aggregated.is_empty() {
        println!("\nMetrics Impact:");
        for (metric_id, base_agg) in &metrics.aggregated {
            if let Some(stressed_agg) = stressed_metrics.aggregated.get(metric_id) {
                let metric_delta = stressed_agg.total - base_agg.total;
                println!("  {}: {:.2} → {:.2} (change: {:+.2})", 
                    metric_id, base_agg.total, stressed_agg.total, metric_delta);
            }
        }
    }

    // 7. Export to DataFrames
    println!("\n--- DataFrame Exports ---");
    let df_positions = finstack_portfolio::dataframe::positions_to_dataframe(&valuation)?;
    println!("Positions DataFrame: {} rows x {} columns", 
        df_positions.height(), 
        df_positions.width());

    let df_entities = finstack_portfolio::dataframe::entities_to_dataframe(&valuation)?;
    println!("Entities DataFrame:  {} rows x {} columns",
        df_entities.height(),
        df_entities.width());

    println!("\n=== Portfolio Example Complete ===");
    Ok(())
}

fn build_market_data(as_of: Date) -> MarketContext {
    // Create USD discount curve with realistic rates (~5% rate)
    let usd_curve = DiscountCurve::builder("USD")
        .base_date(as_of)
        .knots(vec![
            (0.0, 1.0),
            (1.0/365.0, 0.999863),  // 1 day: ~5% rate
            (7.0/365.0, 0.999042),  // 1 week
            (30.0/365.0, 0.995890), // 1 month
            (0.25, 0.9875),         // 3 months: ~5% rate
            (0.5, 0.975),           // 6 months
            (1.0, 0.95),            // 1 year: ~5.13% rate
            (2.0, 0.90),            // 2 years
            (5.0, 0.80),            // 5 years
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    // Create EUR discount curve with realistic rates (~3% rate)
    let eur_curve = DiscountCurve::builder("EUR")
        .base_date(as_of)
        .knots(vec![
            (0.0, 1.0),
            (1.0/365.0, 0.999918),  // 1 day: ~3% rate
            (7.0/365.0, 0.999426),  // 1 week
            (30.0/365.0, 0.997534), // 1 month
            (0.25, 0.9925),         // 3 months: ~3% rate
            (0.5, 0.985),           // 6 months
            (1.0, 0.97),            // 1 year: ~3.05% rate
            (2.0, 0.94),            // 2 years
            (5.0, 0.86),            // 5 years
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    // Create FX matrix
    let fx_provider = SimpleFxProvider::new();
    fx_provider.set_quote(Currency::EUR, Currency::USD, 1.1);
    fx_provider.set_quote(Currency::GBP, Currency::USD, 1.25);
    let fx_matrix = FxMatrix::new(Arc::new(fx_provider));

    MarketContext::new()
        .insert_discount(usd_curve)
        .insert_discount(eur_curve)
        .insert_fx(fx_matrix)
}

fn build_sample_portfolio(as_of: Date) -> finstack_portfolio::Result<Portfolio> {
    // Create entities
    let acme_corp = Entity::new("ACME_CORP")
        .with_name("Acme Corporation")
        .with_tag("sector", "Technology")
        .with_tag("rating", "BBB");

    let fund_a = Entity::new("FUND_A")
        .with_name("Investment Fund A")
        .with_tag("sector", "Finance")
        .with_tag("rating", "AA");

    // Create instruments

    // 1. Deposit for ACME Corp (entity-based)
    let mut deposit1 = Deposit::builder()
        .id("DEP_ACME_3M".into())
        .notional(Money::new(10_000_000.0, Currency::USD))
        .start(as_of)
        .end(date!(2024 - 04 - 01))
        .day_count(DayCount::Act360)
        .disc_id("USD".into())
        .build()
        .unwrap();
    deposit1.quote_rate = Some(0.05); // 5% quoted rate

    let deposit1_position = Position::new(
        "POS_DEP_001",
        "ACME_CORP",
        "DEP_ACME_3M",
        Arc::new(deposit1),
        1.0,
        PositionUnit::Units,
    )
    .with_tag("rating", "BBB")
    .with_tag("instrument_type", "deposit")
    .with_tag("sector", "Technology");

    // 2. Deposit for FUND_A (entity-based)
    let mut deposit2 = Deposit::builder()
        .id("DEP_FUND_6M".into())
        .notional(Money::new(5_000_000.0, Currency::USD))
        .start(as_of)
        .end(date!(2024 - 07 - 01))
        .day_count(DayCount::Act360)
        .disc_id("USD".into())
        .build()
        .unwrap();
    deposit2.quote_rate = Some(0.045); // 4.5% quoted rate

    let deposit2_position = Position::new(
        "POS_DEP_002",
        "FUND_A",
        "DEP_FUND_6M",
        Arc::new(deposit2),
        1.0,
        PositionUnit::Units,
    )
    .with_tag("rating", "AA")
    .with_tag("instrument_type", "deposit")
    .with_tag("sector", "Finance");

    // 3. EUR-denominated deposit (standalone - demonstrates cross-currency)
    let mut deposit_eur = Deposit::builder()
        .id("DEP_EUR_3M".into())
        .notional(Money::new(2_000_000.0, Currency::EUR))
        .start(as_of)
        .end(date!(2024 - 04 - 01))
        .day_count(DayCount::Act360)
        .disc_id("EUR".into())
        .build()
        .unwrap();
    deposit_eur.quote_rate = Some(0.03); // 3% quoted rate

    let deposit_eur_position = Position::new(
        "POS_DEP_003",
        DUMMY_ENTITY_ID, // Standalone instrument
        "DEP_EUR_3M",
        Arc::new(deposit_eur),
        1.0,
        PositionUnit::Units,
    )
    .with_tag("rating", "AAA")
    .with_tag("instrument_type", "deposit")
    .with_tag("sector", "Banking");

    // Build the portfolio
    let portfolio = PortfolioBuilder::new("SAMPLE_PORTFOLIO")
        .name("Sample Mixed Portfolio")
        .base_ccy(Currency::USD)
        .as_of(as_of)
        .entity(acme_corp)
        .entity(fund_a)
        .position(deposit1_position)
        .position(deposit2_position)
        .position(deposit_eur_position)
        .tag("strategy", "fixed_income")
        .build()?;

    Ok(portfolio)
}

