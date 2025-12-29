//! Complete LBO Model Example with Capital Structure Integration
//!
//! This example demonstrates a full LBO (Leveraged Buyout) model with:
//! - Operating performance metrics (revenue, EBITDA, etc.)
//! - Multiple debt instruments (senior notes, subordinated notes)
//! - Proper market context with USD discount curve (~5% rate)
//! - Formulas that reference capital structure via cs.* namespace
//! - Credit metrics (leverage ratios, coverage ratios)
//! - Full evaluation with automatic cashflow computation
//!
//! Run with:
//! ```bash
//! cargo run --example lbo_model_complete
//! ```

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, PeriodId};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_statements::builder::ModelBuilder;
use finstack_statements::evaluator::Evaluator;
use finstack_statements::types::{AmountOrScalar, ForecastSpec};
use time::Month;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Complete LBO Model with Capital Structure ===\n");

    // Define dates
    let issue_date = Date::from_calendar_date(2025, Month::January, 1)?;
    let maturity_date = Date::from_calendar_date(2030, Month::January, 1)?;
    let as_of_date = Date::from_calendar_date(2025, Month::January, 15)?;

    println!("Building LBO Model...");
    println!("  Issue Date: {}", issue_date);
    println!("  Maturity Date: {}", maturity_date);
    println!("  Valuation Date: {}\n", as_of_date);

    // Build the model
    let model = ModelBuilder::new("Acme Corp LBO")
        .periods("2025Q1..2025Q4", Some("2025Q1"))?
        // Operating Metrics - Q1 actuals
        .value(
            "revenue",
            &[(
                PeriodId::quarter(2025, 1),
                AmountOrScalar::scalar(25_000_000.0),
            )],
        )
        .value(
            "cogs",
            &[(
                PeriodId::quarter(2025, 1),
                AmountOrScalar::scalar(15_000_000.0),
            )],
        )
        .value(
            "opex",
            &[(
                PeriodId::quarter(2025, 1),
                AmountOrScalar::scalar(5_000_000.0),
            )],
        )
        .value(
            "depreciation",
            &[(
                PeriodId::quarter(2025, 1),
                AmountOrScalar::scalar(500_000.0),
            )],
        )
        .value(
            "amortization",
            &[(
                PeriodId::quarter(2025, 1),
                AmountOrScalar::scalar(250_000.0),
            )],
        )
        // Forecast operating metrics (5% quarterly growth)
        .forecast("revenue", ForecastSpec::growth(0.05))
        .forecast("cogs", ForecastSpec::growth(0.04)) // Improving margin
        .forecast("opex", ForecastSpec::growth(0.03)) // Operating leverage
        .forecast("depreciation", ForecastSpec::forward_fill())
        .forecast("amortization", ForecastSpec::forward_fill())
        // P&L Calculations
        .compute("gross_profit", "revenue - cogs")?
        .compute("ebitda", "revenue - cogs - opex")?
        .compute("ebit", "ebitda - depreciation - amortization")?
        // Capital Structure: Senior + Subordinated Notes
        .add_bond(
            "SENIOR-NOTES",
            Money::new(100_000_000.0, Currency::USD),
            0.06, // 6% coupon
            issue_date,
            maturity_date,
            "USD-OIS",
        )?
        .add_bond(
            "SUB-NOTES",
            Money::new(50_000_000.0, Currency::USD),
            0.09, // 9% coupon (higher risk)
            issue_date,
            maturity_date,
            "USD-OIS",
        )?
        // P&L with CS Integration
        .compute("interest_expense", "cs.interest_expense.total")?
        .compute("ebt", "ebit - cs.interest_expense.total")?
        .compute("taxes", "if(ebt > 0, ebt * 0.25, 0)")? // 25% tax rate
        .compute("net_income", "ebt - taxes")?
        // Margin Analysis
        .compute("gross_margin", "gross_profit / revenue")?
        .compute("ebitda_margin", "ebitda / revenue")?
        .compute("net_margin", "net_income / revenue")?
        // Credit Metrics
        .compute("leverage", "cs.debt_balance.total / ebitda")?
        .compute("interest_coverage", "ebitda / cs.interest_expense.total")?
        .compute(
            "debt_service",
            "cs.interest_expense.total + cs.principal_payment.total",
        )?
        .compute("debt_service_coverage", "ebitda / debt_service")?
        .build()?;

    println!("Model built successfully!");
    println!("  Nodes: {}", model.nodes.len());
    println!("  Periods: {}", model.periods.len());
    println!(
        "  Debt Instruments: {}\n",
        model
            .capital_structure
            .as_ref()
            .unwrap()
            .debt_instruments
            .len()
    );

    // Create market context with discount curve
    println!("Creating market context with USD discount curve...");

    // Build USD-OIS discount curve (5-year flat at 5% for simplicity)
    // In production, this would come from market data feeds
    let usd_curve = DiscountCurve::builder("USD-OIS")
        .base_date(issue_date)
        .knots([
            (0.0, 1.0),     // Today: DF = 1.0
            (0.25, 0.9877), // 3M: ~5% rate
            (0.5, 0.9756),  // 6M
            (1.0, 0.9512),  // 1Y
            (2.0, 0.9048),  // 2Y
            (3.0, 0.8607),  // 3Y
            (5.0, 0.7788),  // 5Y: DF = exp(-0.05 * 5)
        ])
        .set_interp(InterpStyle::Linear)
        .build()?;

    let market_ctx = MarketContext::new().insert_discount(usd_curve);

    println!("  USD-OIS curve: 7 knots from 0Y to 5Y");
    println!("  Approx rate: ~5.0% flat\n");

    // Evaluate the model
    println!("Evaluating model with capital structure...");
    let mut evaluator = Evaluator::new();

    let results =
        evaluator.evaluate_with_market_context(&model, Some(&market_ctx), Some(as_of_date))?;

    println!("Evaluation complete!\n");

    // Display results
    println!("=== Operating Performance (Q1 2025) ===");
    let q1 = PeriodId::quarter(2025, 1);

    let revenue = results.get("revenue", &q1).unwrap();
    let cogs = results.get("cogs", &q1).unwrap();
    let opex = results.get("opex", &q1).unwrap();
    let gross_profit = results.get("gross_profit", &q1).unwrap();
    let ebitda = results.get("ebitda", &q1).unwrap();
    let ebit = results.get("ebit", &q1).unwrap();

    println!("Revenue:       ${:>15.2}", revenue);
    println!("COGS:          ${:>15.2}", cogs);
    println!("Gross Profit:  ${:>15.2}", gross_profit);
    println!("OpEx:          ${:>15.2}", opex);
    println!("EBITDA:        ${:>15.2}", ebitda);
    println!("EBIT:          ${:>15.2}", ebit);

    println!("\n=== Margin Analysis ===");
    let gross_margin = results.get("gross_margin", &q1).unwrap();
    let ebitda_margin = results.get("ebitda_margin", &q1).unwrap();
    let net_margin = results.get("net_margin", &q1).unwrap();

    println!("Gross Margin:  {:>15.1}%", gross_margin * 100.0);
    println!("EBITDA Margin: {:>15.1}%", ebitda_margin * 100.0);
    println!("Net Margin:    {:>15.1}%", net_margin * 100.0);

    println!("\n=== Capital Structure (Q1 2025) ===");
    if let Some(interest) = results.get("interest_expense", &q1) {
        println!("Interest Expense: ${:>15.2}", interest);
    } else {
        println!("Interest Expense: N/A");
    }

    let q3 = PeriodId::quarter(2025, 3);
    println!("\n=== Capital Structure (Q3 2025) - First Coupon Payment ===");
    if let Some(interest) = results.get("interest_expense", &q3) {
        println!("Interest Expense: ${:>15.2}", interest);
        let ebitda_q3 = results.get("ebitda", &q3).unwrap_or(0.0);
        let coverage = if interest > 0.0 {
            ebitda_q3 / interest
        } else {
            0.0
        };
        println!("EBITDA:           ${:>15.2}", ebitda_q3);
        println!("Interest Coverage:       {:>10.2}x", coverage);
    }

    println!("\n=== Credit Metrics ===");
    if let Some(leverage) = results.get("leverage", &q1) {
        println!("Leverage Ratio:       {:>15.2}x", leverage);
    } else {
        println!("Leverage Ratio:       N/A (requires CS data)");
    }

    if let Some(coverage) = results.get("interest_coverage", &q1) {
        println!("Interest Coverage:    {:>15.2}x", coverage);
    } else {
        println!("Interest Coverage:    N/A (requires CS data)");
    }

    println!("\n=== P&L Summary (Q1 2025) ===");
    let ebt = results.get("ebt", &q1);
    let taxes = results.get("taxes", &q1);
    let net_income = results.get("net_income", &q1);

    if let (Some(ebt), Some(taxes), Some(net_income)) = (ebt, taxes, net_income) {
        println!("EBIT:          ${:>15.2}", ebit);
        println!(
            "Interest:      ${:>15.2}",
            results.get("interest_expense", &q1).unwrap_or(0.0)
        );
        println!("EBT:           ${:>15.2}", ebt);
        println!("Taxes:         ${:>15.2}", taxes);
        println!("Net Income:    ${:>15.2}", net_income);
    }

    println!("\n=== Forecast Periods ===");
    for quarter in 2..=4 {
        let period = PeriodId::quarter(2025, quarter);
        if let Some(rev) = results.get("revenue", &period) {
            let eb = results.get("ebitda", &period).unwrap_or(0.0);
            println!(
                "Q{} 2025: Revenue=${:>12.0}  EBITDA=${:>12.0}",
                quarter, rev, eb
            );
        }
    }

    println!("\n=== Notes ===");
    println!("✅ This example demonstrates proper market context setup with discount curves.");
    println!("   The USD-OIS curve enables accurate pricing of debt instruments.");
    println!();
    println!("📊 Market Data:");
    println!("   - USD-OIS discount curve: ~5% flat rate");
    println!("   - Senior Notes: 6% coupon (spread over risk-free)");
    println!("   - Subordinated Notes: 9% coupon (higher spread)");
    println!();
    println!("💡 For production models, enhance with:");
    println!("   - Live market data feeds (discount curves, forward curves)");
    println!("   - FX rates and multi-currency support");
    println!("   - Credit spreads for instrument-specific pricing");
    println!("   - Volatility surfaces for embedded options");

    println!("\n=== Example Complete ===");

    Ok(())
}
