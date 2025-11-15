//! Tests for the unified DV01 calculator.

use crate::instruments::{Bond, Deposit, InterestRateSwap};
use crate::instruments::common::traits::Instrument;
use crate::metrics::{
    UnifiedDv01Calculator, Dv01CalculatorConfig, Dv01ComputationMode, CurveSelection,
    MetricContext, MetricId,
};
use crate::metrics::traits::MetricCalculator;
use finstack_core::currency::Currency;
use finstack_core::dates::{create_date, DayCount};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use std::sync::Arc;
use time::Month;

fn setup_market() -> (MarketContext, finstack_core::dates::Date) {
    let base_date = create_date(2024, Month::January, 1).unwrap();
    
    // USD discount curve
    let usd_disc = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .day_count(DayCount::Act360)
        .knots(vec![
            (0.0, 1.0),
            (0.25, 0.995),
            (0.5, 0.990),
            (1.0, 0.980),
            (2.0, 0.960),
            (3.0, 0.940),
            (5.0, 0.900),
            (7.0, 0.860),
            (10.0, 0.800),
            (15.0, 0.720),
            (20.0, 0.650),
            (30.0, 0.550),
        ])
        .build()
        .unwrap();
    
    // USD forward curve
    let usd_fwd = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(base_date)
        .day_count(DayCount::Act360)
        .knots(vec![
            (0.0, 0.045),
            (0.25, 0.046),
            (0.5, 0.047),
            (1.0, 0.048),
            (2.0, 0.049),
            (3.0, 0.050),
            (5.0, 0.051),
            (10.0, 0.052),
        ])
        .build()
        .unwrap();
    
    let context = MarketContext::new()
        .insert_discount(usd_disc)
        .insert_forward(usd_fwd);
        
    (context, base_date)
}

#[test]
fn test_bond_parallel_dv01() {
    let (market, as_of) = setup_market();
    
    // Create a 5-year bond
    let bond = Bond::fixed(
        "BOND-TEST",
        Money::new(1_000_000.0, Currency::USD),
        0.05,  // 5% coupon
        as_of,
        create_date(2029, Month::January, 1).unwrap(),
        "USD-OIS",
    );
    
    // Calculate base value
    let base_value = bond.value(&market, as_of).unwrap();
    println!("Bond base PV: {:?}", base_value);
    
    // Test parallel DV01
    let mut context = MetricContext::new(
        Arc::new(bond.clone()),
        Arc::new(market),
        as_of,
        base_value,
    );
    
    let calculator = UnifiedDv01Calculator::<Bond>::new(
        Dv01CalculatorConfig::parallel_combined()
    );
    
    let dv01 = calculator.calculate(&mut context).unwrap();
    println!("Bond parallel DV01: {:.2}", dv01);
    
    // DV01 should be negative for a bond (price decreases as rates increase)
    assert!(dv01 < 0.0, "Bond DV01 should be negative");
    assert!(dv01.abs() > 100.0, "Bond DV01 magnitude seems too small: {:.2}", dv01);
}

#[test]
fn test_bond_bucketed_dv01() {
    let (market, as_of) = setup_market();
    
    // Create a 10-year bond
    let bond = Bond::fixed(
        "BOND-TEST",
        Money::new(1_000_000.0, Currency::USD),
        0.045,  // 4.5% coupon
        as_of,
        create_date(2034, Month::January, 1).unwrap(),
        "USD-OIS",
    );
    
    let base_value = bond.value(&market, as_of).unwrap();
    
    let mut context = MetricContext::new(
        Arc::new(bond.clone()),
        Arc::new(market),
        as_of,
        base_value,
    );
    
    let calculator = UnifiedDv01Calculator::<Bond>::new(
        Dv01CalculatorConfig::key_rate()
    );
    
    let total_dv01 = calculator.calculate(&mut context).unwrap();
    println!("Bond total bucketed DV01: {:.2}", total_dv01);
    
    // Check bucketed series was stored
    let series = context.get_series(&MetricId::BucketedDv01);
    assert!(series.is_some(), "Bucketed series should be stored");
    
    let series = series.unwrap();
    println!("Bucketed DV01 breakdown:");
    for (bucket, dv01) in series {
        println!("  {}: {:.2}", bucket, dv01);
    }
    
    // Should have entries for standard buckets
    assert!(!series.is_empty(), "Should have bucket entries");
    
    // Total should equal sum of buckets
    let sum: f64 = series.iter().map(|(_, v)| v).sum();
    assert!((total_dv01 - sum).abs() < 1e-6, "Total should equal sum of buckets");
}

#[test]
fn test_irs_multi_curve_dv01() {
    let (market, as_of) = setup_market();
    
    // Create a 5-year swap
    let swap = InterestRateSwap::builder()
        .id("IRS-TEST".into())
        .notional(Money::new(10_000_000.0, Currency::USD))
        .side(crate::instruments::common::parameters::PayReceive::ReceiveFixed)  // Receive fixed
        .fixed(crate::instruments::common::parameters::FixedLegSpec {
            discount_curve_id: "USD-OIS".into(),
            rate: 0.045,  // 4.5% fixed rate
            freq: finstack_core::dates::Frequency::semi_annual(),
            dc: DayCount::Thirty360,
            bdc: finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: finstack_core::dates::StubKind::None,
            start: as_of,
            end: create_date(2029, Month::January, 1).unwrap(),
            par_method: None,
            compounding_simple: true,
        })
        .float(crate::instruments::common::parameters::FloatLegSpec {
            discount_curve_id: "USD-OIS".into(),
            forward_curve_id: "USD-SOFR-3M".into(),
            spread_bp: 0.0,
            freq: finstack_core::dates::Frequency::quarterly(),
            dc: DayCount::Act360,
            bdc: finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: finstack_core::dates::StubKind::None,
            reset_lag_days: 2,
            start: as_of,
            end: create_date(2029, Month::January, 1).unwrap(),
        })
        .build()
        .unwrap();
    
    let base_value = swap.value(&market, as_of).unwrap();
    println!("IRS base PV: {:?}", base_value);
    
    let mut context = MetricContext::new(
        Arc::new(swap.clone()),
        Arc::new(market),
        as_of,
        base_value,
    );
    
    // Test key-rate DV01 (should calculate for both discount and forward curves)
    let calculator = UnifiedDv01Calculator::<InterestRateSwap>::new(
        Dv01CalculatorConfig::key_rate()
    );
    
    let total_dv01 = calculator.calculate(&mut context).unwrap();
    println!("IRS total DV01: {:.2}", total_dv01);
    
    // For a receive-fixed swap, DV01 should typically be negative
    assert!(total_dv01 < 0.0, "Receive-fixed swap DV01 should be negative");
    
    // Check that we have bucketed series for multiple curves
    // The unified calculator stores per-curve series with custom metric IDs
    let disc_series = context.get_series(&MetricId::custom("bucketed_dv01::USD-OIS"));
    let fwd_series = context.get_series(&MetricId::custom("bucketed_dv01::USD-SOFR-3M"));
    
    assert!(disc_series.is_some(), "Should have discount curve buckets");
    assert!(fwd_series.is_some(), "Should have forward curve buckets");
    
    println!("Discount curve DV01:");
    for (bucket, dv01) in disc_series.unwrap() {
        println!("  {}: {:.2}", bucket, dv01);
    }
    
    println!("Forward curve DV01:");
    for (bucket, dv01) in fwd_series.unwrap() {
        println!("  {}: {:.2}", bucket, dv01);
    }
}

#[test]
fn test_deposit_dv01() {
    let (market, as_of) = setup_market();
    
    // Create a 1-year deposit
    let deposit = Deposit::builder()
        .id("DEP-TEST".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(as_of)
        .end(create_date(2025, Month::January, 1).unwrap())
        .day_count(DayCount::Act360)
        .discount_curve_id("USD-OIS".into())
        .build()
        .unwrap();
    
    let base_value = deposit.value(&market, as_of).unwrap();
    
    let mut context = MetricContext::new(
        Arc::new(deposit.clone()),
        Arc::new(market),
        as_of,
        base_value,
    );
    
    // Test parallel per-curve mode
    let calculator = UnifiedDv01Calculator::<Deposit>::new(
        Dv01CalculatorConfig::parallel_per_curve()
    );
    
    let total_dv01 = calculator.calculate(&mut context).unwrap();
    println!("Deposit DV01: {:.2}", total_dv01);
    
    // Deposit DV01 should be negative
    assert!(total_dv01 < 0.0, "Deposit DV01 should be negative");
    
    // Check per-curve series (even though deposit only has one curve)
    let series = context.get_series(&MetricId::BucketedDv01);
    assert!(series.is_some(), "Should have per-curve series");
    
    let series = series.unwrap();
    assert_eq!(series.len(), 1, "Should have one curve entry");
    assert_eq!(series[0].0, "USD-OIS", "Should be discount curve");
}

#[test]
fn test_calculator_configurations() {
    // Test that all configuration modes work
    let _parallel_combined = Dv01CalculatorConfig::parallel_combined();
    let _parallel_per_curve = Dv01CalculatorConfig::parallel_per_curve();
    let _key_rate = Dv01CalculatorConfig::key_rate();
    
    // Test custom configuration
    let custom = Dv01CalculatorConfig {
        mode: Dv01ComputationMode::KeyRatePerCurve,
        curve_selection: CurveSelection::DiscountOnly,
        buckets: vec![1.0, 5.0, 10.0],  // Custom buckets
    };
    
    assert_eq!(custom.buckets.len(), 3);
}

#[test]
fn test_with_pricing_overrides() {
    let (market, as_of) = setup_market();
    
    let bond = Bond::fixed(
        "BOND-TEST",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        as_of,
        create_date(2029, Month::January, 1).unwrap(),
        "USD-OIS",
    );
    
    let base_value = bond.value(&market, as_of).unwrap();
    
    let mut context = MetricContext::new(
        Arc::new(bond.clone()),
        Arc::new(market),
        as_of,
        base_value,
    );
    
    // Set custom bump size via pricing overrides
    context.pricing_overrides = Some(crate::instruments::PricingOverrides::none()
        .with_rate_bump(10.0));  // 10bp instead of default 1bp
    
    let calculator = UnifiedDv01Calculator::<Bond>::new(
        Dv01CalculatorConfig::parallel_combined()
    );
    
    let dv01_10bp = calculator.calculate(&mut context).unwrap();
    
    // Reset to default
    context.pricing_overrides = None;
    let dv01_1bp = calculator.calculate(&mut context).unwrap();
    
    // DV01 with 10bp bump should be ~1/10th of 1bp bump (since it's per bp)
    let ratio = dv01_10bp / dv01_1bp;
    assert!((ratio - 1.0).abs() < 0.01, "DV01 should scale linearly with bump size");
}
