//! Comprehensive tests for DV01 calculators.
//!
//! This consolidates tests for the unified DV01 calculator, including:
//! - Parallel DV01 (combined and per-curve)
//! - Key-rate (bucketed) DV01
//! - Multi-curve instruments
//! - Custom configurations
//! - Pricing overrides

use crate::instruments::common::traits::Instrument;
use crate::instruments::{Bond, Deposit, InterestRateSwap};
use crate::metrics::MetricCalculator;
use crate::metrics::{
    CurveSelection, Dv01CalculatorConfig, Dv01ComputationMode, MetricContext, MetricId,
    UnifiedDv01Calculator,
};
use finstack_core::currency::Currency;
use finstack_core::dates::{create_date, DayCount};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use std::sync::Arc;
use time::Month;

// ===== Test Fixtures =====

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

    let context = MarketContext::new().insert_discount(usd_disc);

    (context, base_date)
}

fn setup_market_with_forward() -> (MarketContext, finstack_core::dates::Date) {
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

// ===== Bond Tests =====

#[test]
fn test_bond_parallel_dv01_combined() {
    let (market, as_of) = setup_simple_market();

    // Create a 5-year bond
    let bond = Bond::fixed(
        "BOND-TEST",
        Money::new(1_000_000.0, Currency::USD),
        0.05, // 5% coupon
        as_of,
        create_date(2029, Month::January, 1).unwrap(),
        "USD-OIS",
    );

    let base_value = bond.value(&market, as_of).unwrap();

    let mut context =
        MetricContext::new(Arc::new(bond.clone()), Arc::new(market), as_of, base_value);

    let calculator = UnifiedDv01Calculator::<Bond>::new(Dv01CalculatorConfig::parallel_combined());
    let dv01 = calculator.calculate(&mut context).unwrap();

    println!("Bond parallel DV01: {:.6}", dv01);

    // DV01 should be negative for a fixed-rate bond (price decreases as rates increase)
    assert!(dv01 < 0.0, "Bond DV01 should be negative: {}", dv01);

    // DV01 should be reasonable in magnitude
    assert!(
        dv01.abs() < 10_000.0,
        "DV01 magnitude seems too large: {:.6}",
        dv01
    );
}

#[test]
fn test_bond_bucketed_dv01() {
    let (market, as_of) = setup_simple_market();

    // Create a 10-year bond for more interesting buckets
    let bond = Bond::fixed(
        "BOND-BUCKET-TEST",
        Money::new(1_000_000.0, Currency::USD),
        0.045, // 4.5% coupon
        as_of,
        create_date(2034, Month::January, 1).unwrap(),
        "USD-OIS",
    );

    let base_value = bond.value(&market, as_of).unwrap();

    let mut context =
        MetricContext::new(Arc::new(bond.clone()), Arc::new(market), as_of, base_value);

    let calculator = UnifiedDv01Calculator::<Bond>::new(Dv01CalculatorConfig::key_rate());
    let total = calculator.calculate(&mut context).unwrap();
    let series = context.get_series(&MetricId::BucketedDv01).unwrap();

    println!("Bond total bucketed DV01: {:.6}", total);
    println!("Number of bucket points: {}", series.len());

    // DV01 should be negative for a fixed-rate bond
    assert!(
        total < 0.0,
        "DV01 should be negative for fixed-rate bond: {}",
        total
    );

    // Should have multiple bucket points for a 10-year bond
    assert!(
        series.len() > 1,
        "Should have multiple bucket points for 10-year bond"
    );

    // Verify bucket structure - should contain standard bucket labels
    let expected_buckets = [
        "3m", "6m", "1y", "2y", "3y", "5y", "7y", "10y", "15y", "20y", "30y",
    ];
    assert_eq!(
        series.len(),
        expected_buckets.len(),
        "Should have exactly {} bucket points",
        expected_buckets.len()
    );

    for (i, (bucket_label, _dv01)) in series.iter().enumerate() {
        assert_eq!(
            bucket_label, expected_buckets[i],
            "Bucket {} should be '{}'",
            i, expected_buckets[i]
        );
    }

    // Total should equal sum of buckets
    let sum: f64 = series.iter().map(|(_, v)| v).sum();
    assert!(
        (total - sum).abs() < 1e-6,
        "Total should equal sum of buckets: {} vs {}",
        total,
        sum
    );
}

#[test]
fn test_bond_parallel_per_curve() {
    let (market, as_of) = setup_simple_market();

    let bond = Bond::fixed(
        "BOND-TEST",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        as_of,
        create_date(2029, Month::January, 1).unwrap(),
        "USD-OIS",
    );

    let base_value = bond.value(&market, as_of).unwrap();

    let mut context =
        MetricContext::new(Arc::new(bond.clone()), Arc::new(market), as_of, base_value);

    let calculator = UnifiedDv01Calculator::<Bond>::new(Dv01CalculatorConfig::parallel_per_curve());
    let total = calculator.calculate(&mut context).unwrap();

    // Check per-curve series
    let series = context.get_series(&MetricId::BucketedDv01).unwrap();

    // Bond has only one curve, so should have one entry
    assert_eq!(series.len(), 1, "Bond should have one curve entry");
    assert_eq!(series[0].0, "USD-OIS", "Should be discount curve");

    // Total should equal the single curve value
    assert!(
        (total - series[0].1).abs() < 1e-6,
        "Total should equal single curve value"
    );
}

// ===== Swap Tests =====

#[test]
fn test_irs_multi_curve_dv01() {
    let (market, as_of) = setup_market_with_forward();

    // Create a 5-year swap
    let swap = InterestRateSwap::builder()
        .id("IRS-TEST".into())
        .notional(Money::new(10_000_000.0, Currency::USD))
        .side(crate::instruments::common::parameters::PayReceive::ReceiveFixed) // Receive fixed
        .fixed(crate::instruments::common::parameters::FixedLegSpec {
            discount_curve_id: "USD-OIS".into(),
            rate: 0.045, // 4.5% fixed rate
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
            compounding: Default::default(),
            start: as_of,
            end: create_date(2029, Month::January, 1).unwrap(),
        })
        .build()
        .unwrap();

    let base_value = swap.value(&market, as_of).unwrap();

    let mut context =
        MetricContext::new(Arc::new(swap.clone()), Arc::new(market), as_of, base_value);

    // Test key-rate DV01 (should calculate for both discount and forward curves)
    let calculator =
        UnifiedDv01Calculator::<InterestRateSwap>::new(Dv01CalculatorConfig::key_rate());

    let total_dv01 = calculator.calculate(&mut context).unwrap();
    println!("IRS total DV01: {:.2}", total_dv01);

    // For a receive-fixed swap, DV01 should typically be negative
    assert!(
        total_dv01 < 0.0,
        "Receive-fixed swap DV01 should be negative"
    );

    // Check that we have bucketed series for multiple curves
    let disc_series = context.get_series(&MetricId::custom("bucketed_dv01::USD-OIS"));
    let fwd_series = context.get_series(&MetricId::custom("bucketed_dv01::USD-SOFR-3M"));

    assert!(disc_series.is_some(), "Should have discount curve buckets");
    assert!(fwd_series.is_some(), "Should have forward curve buckets");
}

// ===== Deposit Tests =====

#[test]
fn test_deposit_dv01() {
    let (market, as_of) = setup_simple_market();

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
    let calculator =
        UnifiedDv01Calculator::<Deposit>::new(Dv01CalculatorConfig::parallel_per_curve());

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

// ===== Configuration Tests =====

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
        buckets: vec![1.0, 5.0, 10.0], // Custom buckets
    };

    assert_eq!(custom.buckets.len(), 3);
}

#[test]
fn test_with_pricing_overrides() {
    let (market, as_of) = setup_simple_market();

    let bond = Bond::fixed(
        "BOND-TEST",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        as_of,
        create_date(2029, Month::January, 1).unwrap(),
        "USD-OIS",
    );

    let base_value = bond.value(&market, as_of).unwrap();

    let mut context =
        MetricContext::new(Arc::new(bond.clone()), Arc::new(market), as_of, base_value);

    // Set custom bump size via pricing overrides
    context.pricing_overrides =
        Some(crate::instruments::PricingOverrides::none().with_rate_bump(10.0)); // 10bp instead of default 1bp

    let calculator = UnifiedDv01Calculator::<Bond>::new(Dv01CalculatorConfig::parallel_combined());

    let dv01_10bp = calculator.calculate(&mut context).unwrap();

    // Reset to default
    context.pricing_overrides = None;
    let dv01_1bp = calculator.calculate(&mut context).unwrap();

    // DV01 should be consistent regardless of bump size (normalized per bp)
    let ratio = dv01_10bp / dv01_1bp;
    assert!(
        (ratio - 1.0).abs() < 0.01,
        "DV01 should scale linearly with bump size, ratio: {:.6}",
        ratio
    );
}

#[test]
fn test_custom_buckets() {
    let (market, as_of) = setup_simple_market();

    let bond = Bond::fixed(
        "BOND-TEST",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        as_of,
        create_date(2034, Month::January, 1).unwrap(),
        "USD-OIS",
    );

    let base_value = bond.value(&market, as_of).unwrap();

    let mut context =
        MetricContext::new(Arc::new(bond.clone()), Arc::new(market), as_of, base_value);

    // Use custom buckets
    let custom_config = Dv01CalculatorConfig {
        mode: Dv01ComputationMode::KeyRatePerCurve,
        curve_selection: CurveSelection::AllRateCurves,
        buckets: vec![1.0, 5.0, 10.0], // Only 3 custom buckets
    };

    let calculator = UnifiedDv01Calculator::<Bond>::new(custom_config);
    let total = calculator.calculate(&mut context).unwrap();
    let series = context.get_series(&MetricId::BucketedDv01).unwrap();

    // Should have exactly 3 buckets
    assert_eq!(series.len(), 3, "Should have 3 custom buckets");
    assert_eq!(series[0].0, "1y");
    assert_eq!(series[1].0, "5y");
    assert_eq!(series[2].0, "10y");

    // Total should equal sum
    let sum: f64 = series.iter().map(|(_, v)| v).sum();
    assert!(
        (total - sum).abs() < 1e-6,
        "Total should equal sum of buckets"
    );
}
