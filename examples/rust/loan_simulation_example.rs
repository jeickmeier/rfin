//! Enhanced Loan Simulation Example
//!
//! Demonstrates the forward simulation methodology for DDTL and Revolver instruments,
//! showing how the enhanced valuation captures the economics of undrawn commitments.

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::money::Money;
use finstack_valuations::instruments::fixed_income::loan::{
    DelayedDrawTermLoan, DrawEvent, ExpectedFundingCurve, 
    RevolvingCreditFacility, UtilizationFeeSchedule
};
use finstack_valuations::instruments::fixed_income::loan::revolver::RevolverFundingCurve;
use finstack_valuations::instruments::fixed_income::loan::term_loan::InterestSpec;
use finstack_valuations::instruments::traits::Priceable;
use finstack_valuations::metrics::MetricId;
use time::Month;

fn main() -> finstack_core::Result<()> {
    println!("Enhanced Loan Simulation Example");
    println!("{}", "=".repeat(50));
    
    // Setup market data
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    
    // Create discount curve
    let usd_ois = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (1.0, 0.97), (3.0, 0.91), (5.0, 0.84)])
        .set_interp(finstack_core::market_data::interp::InterpStyle::MonotoneConvex)
        .build()?;
    
    // Create forward curve for floating rate loans
    let usd_sofr_3m = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(base_date)
        .knots([(0.0, 0.045), (1.0, 0.048), (3.0, 0.050), (5.0, 0.052)])
        .set_interp(finstack_core::market_data::interp::InterpStyle::FlatFwd)
        .build()?;
    
    let curves = MarketContext::new()
        .with_discount(usd_ois)
        .with_forecast(usd_sofr_3m);
    
    println!("\n1. Delayed-Draw Term Loan (DDTL) Example");
    println!("{}", "-".repeat(40));
    
    // Create DDTL with expected draws
    let commitment_expiry = Date::from_calendar_date(2026, Month::December, 31).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();
    
    let expected_draws = vec![
        DrawEvent {
            date: Date::from_calendar_date(2025, Month::June, 1).unwrap(),
            amount: Money::new(3_000_000.0, Currency::USD),
            purpose: Some("Working capital".to_string()),
            conditional: false,
        },
        DrawEvent {
            date: Date::from_calendar_date(2025, Month::December, 1).unwrap(),
            amount: Money::new(2_000_000.0, Currency::USD),
            purpose: Some("Expansion".to_string()),
            conditional: false,
        },
    ];
    
    let funding_curve = ExpectedFundingCurve::with_probabilities(
        expected_draws,
        vec![0.95, 0.80], // Decreasing probabilities
    );
    
    let ddtl = DelayedDrawTermLoan::new(
        "DDTL_ENHANCED",
        Money::new(10_000_000.0, Currency::USD),
        commitment_expiry,
        maturity,
        InterestSpec::Floating {
            index_id: "USD-SOFR-3M",
            spread_bp: 275.0,
            spread_step_ups: None,
            gearing: 1.0,
            reset_lag_days: 2,
        },
    )
    .with_expected_funding_curve(funding_curve)
    .with_commitment_fee(0.0050); // 50 bps
    
    // Enhanced valuation
    let ddtl_value = ddtl.value(&curves, base_date)?;
    println!("DDTL Value: ${:.2} {}", ddtl_value.amount(), ddtl_value.currency());
    
    // Compute expected exposure and other metrics
    let ddtl_metrics = vec![
        MetricId::custom("expected_exposure_1y"),
        MetricId::custom("commitment_fee_pv"),
        MetricId::custom("incremental_interest_pv"),
        MetricId::custom("utilization"),
        MetricId::custom("undrawn_amount"),
    ];
    
    let ddtl_result = ddtl.price_with_metrics(&curves, base_date, &ddtl_metrics)?;
    println!("Expected Exposure (1Y): ${:.0}", 
        ddtl_result.measures.get("expected_exposure_1y").unwrap_or(&0.0));
    println!("Commitment Fee PV: ${:.2}", 
        ddtl_result.measures.get("commitment_fee_pv").unwrap_or(&0.0));
    println!("Incremental Interest PV: ${:.2}", 
        ddtl_result.measures.get("incremental_interest_pv").unwrap_or(&0.0));
    
    println!("\n2. Revolving Credit Facility Example");
    println!("{}", "-".repeat(40));
    
    // Create revolver with utilization fee tiers
    let util_schedule = UtilizationFeeSchedule::new()
        .with_tier(0.0, 0.33, 12.5)   // < 33%: 12.5 bps
        .with_tier(0.33, 0.66, 25.0)  // 33-66%: 25 bps  
        .with_tier(0.66, 1.0, 50.0);  // > 66%: 50 bps
    
    // Expected seasonal draw/repay pattern
    let expected_events = vec![
        super::revolver::DrawRepayEvent {
            date: Date::from_calendar_date(2025, Month::March, 1).unwrap(),
            amount: Money::new(15_000_000.0, Currency::USD),
            mandatory: false,
            description: Some("Spring inventory".to_string()),
        },
        super::revolver::DrawRepayEvent {
            date: Date::from_calendar_date(2025, Month::August, 1).unwrap(),
            amount: Money::new(-10_000_000.0, Currency::USD),
            mandatory: false,
            description: Some("Summer paydown".to_string()),
        },
    ];
    
    let revolver_curve = RevolverFundingCurve::with_probabilities(
        expected_events,
        vec![0.90, 0.85], // High probability seasonal pattern
    );
    
    let revolver = RevolvingCreditFacility::new(
        "RCF_ENHANCED",
        Money::new(50_000_000.0, Currency::USD),
        base_date,
        Date::from_calendar_date(2028, Month::January, 1).unwrap(),
        Date::from_calendar_date(2030, Month::January, 1).unwrap(),
    )
    .with_interest(InterestSpec::Floating {
        index_id: "USD-SOFR-3M",
        spread_bp: 225.0,
        spread_step_ups: None,
        gearing: 1.0,
        reset_lag_days: 2,
    })
    .with_commitment_fee(0.0040)
    .with_utilization_fees(util_schedule)
    .with_expected_funding_curve(revolver_curve);
    
    // Price with deterministic simulation
    let revolver_value = revolver.value(&curves, base_date)?;
    println!("Revolver Value (Deterministic): ${:.2} {}", 
        revolver_value.amount(), revolver_value.currency());
    
    // Compare with Monte Carlo for utilization tier accuracy
    let mc_metrics = vec![
        MetricId::custom("expected_exposure_mc_1y"),
        MetricId::custom("utilization_fee_pv"),
    ];
    
    let revolver_mc = revolver.price_with_metrics(&curves, base_date, &mc_metrics)?;
    println!("Expected Exposure (MC 1Y): ${:.0}", 
        revolver_mc.measures.get("expected_exposure_mc_1y").unwrap_or(&0.0));
    println!("Utilization Fee PV (MC): ${:.2}", 
        revolver_mc.measures.get("utilization_fee_pv").unwrap_or(&0.0));
    
    println!("\n3. Enhanced Methodology Benefits");
    println!("{}", "-".repeat(40));
    println!("✓ Forward rate projections for floating-rate interest");
    println!("✓ PIK capitalization effects on outstanding balances");
    println!("✓ Mid-point averaging for accurate fee accruals");
    println!("✓ Monte Carlo for utilization tier step functions");
    println!("✓ Event-driven simulation timeline");
    println!("✓ Expected Exposure term structure for risk management");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_enhanced_loan_valuation() {
        // Simple smoke test to ensure the enhanced methodology works
        assert!(main().is_ok());
    }
}
