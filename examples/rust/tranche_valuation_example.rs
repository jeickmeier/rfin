//! Example demonstrating tranche-specific valuation for structured credit instruments.
//!
//! This example shows how to:
//! 1. Create a CLO with multiple tranches
//! 2. Get cashflows for a specific tranche
//! 3. Calculate PV and metrics for individual tranches
//! 4. Compare metrics across different tranches

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_valuations::instruments::structured_credit::{
    StructuredCredit, AssetPool, DealType, PoolAsset, Tranche, TrancheCoupon, TrancheStructure,
    TrancheSeniority, TrancheValuationExt, WaterfallBuilder, CreditRating,
};
use finstack_valuations::metrics::MetricId;
use std::error::Error;
use time::macros::date;

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== Tranche-Specific Valuation Example ===\n");

    // Valuation date
    let as_of = date!(2024 - 01 - 01);

    // Create a simple CLO structure with three tranches
    let clo = create_sample_clo()?;

    // Create market context with discount curves
    let market = create_market_context(as_of)?;

    // Demonstrate valuation for each tranche separately
    println!("Tranche-Specific Valuations:");
    println!("{}", "-".repeat(60));

    for tranche in &clo.tranches.tranches {
        let tranche_id = tranche.id.as_str();
        println!("\nTranche: {}", tranche_id);
        println!("  Seniority: {:?}", tranche.seniority);
        println!("  Original Balance: {}", tranche.original_balance);
        println!("  Attachment: {:.1}%", tranche.attachment_point);
        println!("  Detachment: {:.1}%", tranche.detachment_point);

        // Get tranche-specific cashflows
        let cashflow_result = clo.get_tranche_cashflows(tranche_id, &market, as_of)?;
        
        println!("\n  Cashflow Summary:");
        println!("    Total Cashflows: {}", cashflow_result.cashflows.len());
        println!("    Total Interest: {}", cashflow_result.total_interest);
        println!("    Total Principal: {}", cashflow_result.total_principal);
        println!("    Final Balance: {}", cashflow_result.final_balance);

        // Calculate tranche PV
        let pv = clo.value_tranche(tranche_id, &market, as_of)?;
        println!("\n  Present Value: {}", pv);
        
        // Calculate price as percentage of par
        let price_pct = (pv.amount() / tranche.original_balance.amount()) * 100.0;
        println!("  Price (% of par): {:.2}%", price_pct);

        // Get full valuation with metrics
        let metrics = vec![
            MetricId::WAL,
            MetricId::DurationMod,
            MetricId::ZSpread,
            MetricId::Cs01,
        ];
        
        let valuation = clo.value_tranche_with_metrics(
            tranche_id,
            &market,
            as_of,
            &metrics,
        )?;

        println!("\n  Metrics:");
        println!("    Clean Price: {:.2}%", valuation.clean_price);
        println!("    Dirty Price: {:.2}%", valuation.dirty_price);
        println!("    WAL: {:.2} years", valuation.wal);
        println!("    Modified Duration: {:.2} years", valuation.modified_duration);
        println!("    Z-Spread: {:.0} bps", valuation.z_spread_bps);
        println!("    CS01: ${:.2}", valuation.cs01);
        println!("    YTM: {:.2}%", valuation.ytm * 100.0);
    }

    // Demonstrate cashflow details for a specific tranche
    println!("\n{}", "=".repeat(60));
    println!("Detailed Cashflows for Senior Tranche (AAA):");
    println!("{}", "-".repeat(60));
    
    let aaa_cashflows = clo.get_tranche_cashflows("AAA", &market, as_of)?;
    
    println!("\nFirst 5 Total Cashflows:");
    for (i, (date, amount)) in aaa_cashflows.cashflows.iter().take(5).enumerate() {
        println!("  {}: {} - {}", i + 1, date, amount);
    }
    
    println!("\nFirst 5 Interest Payments:");
    for (i, (date, amount)) in aaa_cashflows.interest_flows.iter().take(5).enumerate() {
        println!("  {}: {} - {}", i + 1, date, amount);
    }
    
    println!("\nFirst 5 Principal Payments:");
    for (i, (date, amount)) in aaa_cashflows.principal_flows.iter().take(5).enumerate() {
        println!("  {}: {} - {}", i + 1, date, amount);
    }

    // Show how different tranches have different risk/return profiles
    println!("\n{}", "=".repeat(60));
    println!("Risk/Return Comparison Across Tranches:");
    println!("{}", "-".repeat(60));
    println!("{:<10} {:>12} {:>10} {:>10} {:>10}", 
             "Tranche", "Price (%)", "WAL (yrs)", "Z-Spr(bps)", "CS01 ($)");
    println!("{}", "-".repeat(60));
    
    for tranche in &clo.tranches.tranches {
        let tranche_id = tranche.id.as_str();
        let valuation = clo.value_tranche_with_metrics(
            tranche_id,
            &market,
            as_of,
            &[MetricId::WAL, MetricId::ZSpread, MetricId::Cs01],
        )?;
        
        println!("{:<10} {:>12.2} {:>10.2} {:>10.0} {:>10.2}",
                 tranche_id,
                 valuation.dirty_price,
                 valuation.wal,
                 valuation.z_spread_bps,
                 valuation.cs01);
    }

    println!("\n=== Example Complete ===");
    Ok(())
}

/// Create a sample CLO structure with multiple tranches
fn create_sample_clo() -> Result<StructuredCredit, Box<dyn Error>> {
    let base_currency = Currency::USD;
    let _as_of = date!(2024 - 01 - 01);
    
    // Create asset pool with $500M of loans
    let mut pool = AssetPool::new("CLO_2024_01", DealType::CLO, base_currency);
    
    // Add some sample loans to the pool
    for i in 1..=50 {
        let loan = PoolAsset::floating_rate_loan(
            format!("LOAN_{:03}", i),
            Money::new(10_000_000.0, base_currency), // $10M each
            "SOFR-3M",
            450.0, // 450 bps spread
            date!(2030 - 01 - 01),
        ).with_rating(CreditRating::BB);
        pool.assets.push(loan);
    }
    
    // Create tranche structure
    let aaa_tranche = Tranche::new(
        "AAA",
        20.0, // attachment (20%)
        100.0, // detachment (100%)
        TrancheSeniority::Senior,
        Money::new(400_000_000.0, base_currency),
        TrancheCoupon::Floating {
            forward_curve_id: CurveId::new("SOFR-3M".to_string()),
            spread_bp: 150.0,
            floor: Some(0.0),
            cap: None,
        },
        date!(2034 - 01 - 01),
    )?;
    
    let aa_tranche = Tranche::new(
        "AA",
        10.0, // attachment (10%)
        20.0, // detachment (20%)
        TrancheSeniority::Mezzanine,
        Money::new(50_000_000.0, base_currency),
        TrancheCoupon::Floating {
            forward_curve_id: CurveId::new("SOFR-3M".to_string()),
            spread_bp: 300.0,
            floor: Some(0.0),
            cap: None,
        },
        date!(2034 - 01 - 01),
    )?;
    
    let equity_tranche = Tranche::new(
        "Equity",
        0.0, // attachment (0%)
        10.0, // detachment (10%)
        TrancheSeniority::Equity,
        Money::new(50_000_000.0, base_currency),
        TrancheCoupon::Fixed {
            rate: 0.15, // 15% target return
        },
        date!(2034 - 01 - 01),
    )?;
    
    let tranches = TrancheStructure::new(vec![aaa_tranche, aa_tranche, equity_tranche])?;
    
    // Create waterfall with pro-rata principal distribution
    let waterfall = WaterfallBuilder::new(base_currency)
        .add_senior_expenses(Money::new(100_000.0, base_currency), "Trustee")
        .add_tranche_interest("AAA", false)
        .add_tranche_interest("AA", false)
        .add_tranche_interest("Equity", false)
        .add_tranche_principal("AAA")
        .add_tranche_principal("AA")
        .add_equity_distribution()
        .build();
    
    // Create CLO with realistic assumptions for investment-grade tranches
    use finstack_valuations::instruments::structured_credit::{
        PrepaymentModelSpec, DefaultModelSpec, RecoveryModelSpec,
    };
    
    let mut clo = StructuredCredit::new_clo(
        "CLO_2024_01",
        pool,
        tranches,
        waterfall,
        date!(2024 - 01 - 01),
        date!(2034 - 01 - 01),
        "USD_DISC",
    );
    
    // Set conservative prepayment/default assumptions for better pricing
    clo.prepayment_spec = PrepaymentModelSpec::ConstantCpr { cpr: 0.10 }; // 10% CPR (lower)
    clo.default_spec = DefaultModelSpec::ConstantCdr { cdr: 0.01 }; // 1% CDR (lower default)
    clo.recovery_spec = RecoveryModelSpec::Constant { rate: 0.70 }; // 70% recovery
    
    Ok(clo)
}

/// Create market context with discount curves
fn create_market_context(as_of: Date) -> Result<MarketContext, Box<dyn Error>> {
    // Build discount curves with appropriate spreads for structured credit
    // Use a rate close to the tranche coupons to get prices near par
    let disc_rate = 0.045_f64 + 0.020_f64; // SOFR + 200bps for weighted average
    let disc_curve = DiscountCurve::builder("USD_DISC")
        .base_date(as_of)
        .day_count(DayCount::Act365F)
        .knots([
            (0.0, 1.0),                               // Today: df = 1
            (1.0, (-disc_rate).exp()),                // 1 year
            (5.0, (-disc_rate * 5.0).exp()),          // 5 years
            (10.0, (-disc_rate * 10.0).exp()),        // 10 years
        ])
        .build()?;
    
    // Build SOFR-3M curve for floating rate index
    let sofr_rate = 0.045_f64; // 4.5%
    let sofr_curve = DiscountCurve::builder("SOFR-3M")
        .base_date(as_of)
        .day_count(DayCount::Act365F)
        .knots([
            (0.0, 1.0),                               // Today: df = 1
            (1.0, (-sofr_rate).exp()),                // 1 year
            (5.0, (-sofr_rate * 5.0).exp()),         // 5 years
            (10.0, (-sofr_rate * 10.0).exp()),       // 10 years
        ])
        .build()?;
    
    // Create market context and add curves
    let context = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_discount(sofr_curve);
    
    Ok(context)
}

