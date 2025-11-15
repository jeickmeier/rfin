//! Tests for the new UnifiedDv01Calculator implementation.

use crate::instruments::Bond;
use crate::instruments::common::traits::Instrument;
use crate::metrics::{
    UnifiedDv01Calculator, Dv01CalculatorConfig,
    MetricContext, MetricId,
};
use crate::metrics::traits::MetricCalculator;
use finstack_core::currency::Currency;
use finstack_core::dates::{create_date, DayCount};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use std::sync::Arc;
use time::Month;

fn setup_simple_market() -> (MarketContext, finstack_core::dates::Date) {
    let base_date = create_date(2024, Month::January, 1).unwrap();
    
    // Simple USD discount curve
    let usd_disc = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .day_count(DayCount::Act360)
        .knots(vec![
            (0.0, 1.0),
            (1.0, 0.98),
            (2.0, 0.96),
            (5.0, 0.90),
            (10.0, 0.80),
        ])
        .build()
        .unwrap();
    
    let context = MarketContext::new()
        .insert_discount(usd_disc);
        
    (context, base_date)
}

#[test]
fn test_parallel_dv01_unified() {
    let (market, as_of) = setup_simple_market();
    
    // Create a simple 5-year bond
    let bond = Bond::fixed(
        "BOND-PARITY-TEST",
        Money::new(1_000_000.0, Currency::USD),
        0.05,  // 5% coupon
        as_of,
        create_date(2029, Month::January, 1).unwrap(),
        "USD-OIS",
    );
    
    let base_value = bond.value(&market, as_of).unwrap();
    
    // Calculate using new implementation
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
    
    println!("Unified DV01: {:.6}", dv01);
    
    // DV01 should be negative for a fixed-rate bond (standard convention: PV(bumped +1bp) - PV(base))
    assert!(dv01 < 0.0, "DV01 should be negative for fixed-rate bond: {}", dv01);
    
    // DV01 should be reasonable (not too large in magnitude)
    assert!(dv01.abs() < 1000.0, "DV01 magnitude seems too large: {:.6}", dv01);
}

#[test]
fn test_bucketed_dv01_unified() {
    let (market, as_of) = setup_simple_market();
    
    // Create a 10-year bond for more interesting buckets
    let bond = Bond::fixed(
        "BOND-BUCKET-TEST",
        Money::new(1_000_000.0, Currency::USD),
        0.045,  // 4.5% coupon
        as_of,
        create_date(2034, Month::January, 1).unwrap(),
        "USD-OIS",
    );
    
    let base_value = bond.value(&market, as_of).unwrap();
    
    // Calculate using new implementation
    let mut context = MetricContext::new(
        Arc::new(bond.clone()),
        Arc::new(market),
        as_of,
        base_value,
    );
    
    let calculator = UnifiedDv01Calculator::<Bond>::new(
        Dv01CalculatorConfig::key_rate()
    );
    let total = calculator.calculate(&mut context).unwrap();
    let series = context.get_series(&MetricId::BucketedDv01).unwrap();
    
    println!("Unified total DV01: {:.6}", total);
    println!("Number of bucket points: {}", series.len());
    
    // Print bucket series for debugging
    for (i, (time, dv01)) in series.iter().enumerate() {
        println!("Bucket {}: time={:?}, dv01={:.6}", i, time, dv01);
    }
    
    // DV01 should be negative for a fixed-rate bond (standard convention: PV(bumped +1bp) - PV(base))
    assert!(total < 0.0, "DV01 should be negative for fixed-rate bond: {}", total);
    
    // Should have multiple bucket points for a 10-year bond
    assert!(series.len() > 1, "Should have multiple bucket points for 10-year bond");
    
    // Verify bucket structure - should contain standard bucket labels in order
    let expected_buckets = vec!["3m", "6m", "1y", "2y", "3y", "5y", "7y", "10y", "15y", "20y", "30y"];
    assert_eq!(series.len(), expected_buckets.len(), "Should have exactly {} bucket points", expected_buckets.len());
    
    for (i, (bucket_label, _dv01)) in series.iter().enumerate() {
        assert_eq!(bucket_label, expected_buckets[i], "Bucket {} should be '{}'", i, expected_buckets[i]);
    }
}
