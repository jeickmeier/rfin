//! Example demonstrating Repurchase Agreement (Repo) functionality.
//!
//! This example shows how to:
//! - Create different types of repos (overnight, term, open)
//! - Handle general vs special collateral
//! - Calculate collateral haircuts and adequacy
//! - Compute repo pricing and risk metrics

use finstack_core::prelude::*;
use finstack_core::market_data::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::interp::InterpStyle;
use finstack_valuations::instruments::fixed_income::repo::*;
use finstack_valuations::instruments::traits::Priceable;
use finstack_valuations::cashflow::traits::CashflowProvider;
use finstack_valuations::metrics::{MetricId, MetricRegistry};
use time::Month;

fn main() -> Result<()> {
    println!("=== Repo Example ===\n");

    // Create market context with discount curve and collateral prices
    let market_context = create_market_context();
    
    // Example 1: Overnight repo
    println!("1. Overnight Repo");
    let overnight_repo = create_overnight_repo()?;
    demonstrate_repo(&overnight_repo, &market_context, "Overnight")?;

    // Example 2: Term repo with general collateral
    println!("\n2. Term Repo (General Collateral)");
    let term_repo = create_term_repo()?;
    demonstrate_repo(&term_repo, &market_context, "Term")?;

    // Example 3: Special collateral repo
    println!("\n3. Special Collateral Repo");
    let special_repo = create_special_collateral_repo()?;
    demonstrate_repo(&special_repo, &market_context, "Special")?;

    // Example 4: Tri-party repo
    println!("\n4. Tri-party Repo");
    let triparty_repo = create_triparty_repo()?;
    demonstrate_repo(&triparty_repo, &market_context, "Tri-party")?;

    // Example 5: Risk analysis
    println!("\n5. Risk Analysis");
    demonstrate_risk_analysis(&term_repo, &market_context)?;

    Ok(())
}

fn test_date(year: i32, month: u8, day: u8) -> Date {
    Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap()
}

fn create_market_context() -> MarketContext {
    // Create a simple discount curve
    let discount_curve = DiscountCurve::builder("USD-OIS")
        .base_date(test_date(2025, 1, 1))
        .knots([
            (0.0, 1.0),
            (0.25, 0.9875),    // 3M: ~5% rate
            (1.0, 0.95),       // 1Y: ~5.13% rate  
            (5.0, 0.78),       // 5Y: ~4.97% rate
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    MarketContext::new()
        .insert_discount(discount_curve)
        .insert_price("UST_10Y_PRICE", MarketScalar::Unitless(1.02))      // Treasury at 102%
        .insert_price("CORPORATE_BOND_PRICE", MarketScalar::Unitless(0.98)) // Corporate at 98%
        .insert_price("ON_THE_RUN_PRICE", MarketScalar::Unitless(1.001))    // Special security at slight premium
}

fn create_overnight_repo() -> Result<Repo> {
    let collateral = CollateralSpec::new(
        "UST_10Y_BOND", 
        1_020_000.0,  // Slightly over-collateralized
        "UST_10Y_PRICE"
    );

    Repo::overnight(
        "OVERNIGHT_REPO_001",
        Money::new(1_000_000.0, Currency::USD),
        collateral,
        0.0525, // 5.25% overnight rate
        test_date(2025, 1, 15),
        "USD-OIS",
    )
}

fn create_term_repo() -> Result<Repo> {
    let collateral = CollateralSpec::new(
        "UST_10Y_BOND",
        1_050_000.0, // Higher collateral for term repo
        "UST_10Y_PRICE"
    );

    Ok(Repo::term(
        "TERM_REPO_001",
        Money::new(1_000_000.0, Currency::USD),
        collateral,
        0.055, // 5.5% term rate (higher than overnight)
        test_date(2025, 1, 15),
        test_date(2025, 4, 15), // 3-month term
        "USD-OIS",
    ))
}

fn create_special_collateral_repo() -> Result<Repo> {
    let special_collateral = CollateralSpec::special(
        "ON_THE_RUN_10Y",       // Specific on-the-run Treasury
        "ON_THE_RUN_BOND",
        1_030_000.0,
        "ON_THE_RUN_PRICE",
        Some(-25.0),            // 25bp special rate reduction
    );

    Ok(Repo::term(
        "SPECIAL_REPO_001",
        Money::new(1_000_000.0, Currency::USD),
        special_collateral,
        0.055, // Base rate before special adjustment
        test_date(2025, 1, 15),
        test_date(2025, 2, 15), // 1-month term
        "USD-OIS",
    ))
}

fn create_triparty_repo() -> Result<Repo> {
    let collateral = CollateralSpec::new(
        "CORPORATE_BOND_BASKET",
        1_100_000.0, // Higher haircut for corporate bonds
        "CORPORATE_BOND_PRICE"
    );

    Repo::builder()
        .id("TRIPARTY_REPO_001")
        .cash_amount(Money::new(1_000_000.0, Currency::USD))
        .collateral(collateral)
        .repo_rate(0.06) // Higher rate for corporate collateral
        .dates(test_date(2025, 1, 15), test_date(2025, 3, 15))
        .haircut(0.08) // 8% haircut for riskier collateral
        .triparty(true)
        .disc_id("USD-OIS")
        .with_tag("triparty")
        .with_meta("agent", "BANK_OF_NY_MELLON")
        .build()
}

fn demonstrate_repo(repo: &Repo, context: &MarketContext, repo_name: &str) -> Result<()> {
    println!("--- {} Repo Analysis ---", repo_name);
    println!("ID: {}", repo.id.as_str());
    println!("Cash Amount: {}", repo.cash_amount);
    println!("Repo Rate: {:.3}%", repo.repo_rate * 100.0);
    println!("Effective Rate: {:.3}%", repo.effective_rate() * 100.0);
    println!("Haircut: {:.2}%", repo.haircut * 100.0);
    println!("Tri-party: {}", repo.triparty);
    
    // Collateral analysis
    let collateral_value = repo.collateral.market_value(context)?;
    let required_collateral = repo.required_collateral_value();
    let is_adequate = repo.is_adequately_collateralized(context)?;
    
    println!("Collateral Value: {}", collateral_value);
    println!("Required Collateral: {}", required_collateral);
    println!("Adequately Collateralized: {}", is_adequate);
    
    // Financial metrics
    let interest = repo.interest_amount()?;
    let total_repayment = repo.total_repayment()?;
    
    println!("Interest Amount: {}", interest);
    println!("Total Repayment: {}", total_repayment);
    
    // Present value analysis
    let valuation_date = test_date(2025, 1, 10);
    let pv = repo.value(context, valuation_date)?;
    println!("Present Value (as of {}): {}", valuation_date, pv);
    
    // Cashflow schedule
    let cashflows = repo.build_schedule(context, valuation_date)?;
    println!("Cashflow Schedule:");
    for (date, amount) in &cashflows {
        println!("  {}: {}", date, amount);
    }
    
    Ok(())
}

fn demonstrate_risk_analysis(repo: &Repo, context: &MarketContext) -> Result<()> {
    println!("--- Risk Analysis ---");
    
    // Calculate key risk metrics
    let metrics = vec![
        MetricId::CollateralValue,
        MetricId::RequiredCollateral,
        MetricId::CollateralCoverage,
        MetricId::RepoInterest,
        MetricId::EffectiveRate,
        MetricId::TimeToMaturity,
    ];
    
    let valuation_date = test_date(2025, 1, 10);
    let result = repo.price_with_metrics(context, valuation_date, &metrics)?;
    
    println!("Risk Metrics:");
    for (metric_name, value) in &result.measures {
        println!("  {}: {:.6}", metric_name, value);
    }
    
    // Collateral coverage analysis
    if let Some(&coverage) = result.measures.get("collateral_coverage") {
        if coverage < 1.0 {
            println!("⚠️  WARNING: Undercollateralized! Coverage ratio: {:.3}", coverage);
        } else {
            println!("✅ Adequately collateralized. Coverage ratio: {:.3}", coverage);
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example_runs_without_error() {
        main().unwrap();
    }
}
