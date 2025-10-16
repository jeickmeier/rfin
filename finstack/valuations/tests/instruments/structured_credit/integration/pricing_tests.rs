//! Integration tests for full pricing and metrics computation.
//!
//! Tests end-to-end pricing workflow with market data and metric requests.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_valuations::cashflow::traits::CashflowProvider;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::structured_credit::{
    AssetPool, DealType, PaymentCalculation, PaymentRecipient,
    PaymentRule, PoolAsset, StructuredCredit, Tranche, TrancheCoupon, TrancheSeniority,
    TrancheStructure, WaterfallEngine,
};
use finstack_valuations::metrics::MetricId;
use time::Month;

fn test_date() -> Date {
    Date::from_calendar_date(2025, Month::January, 1).unwrap()
}

fn maturity_date() -> Date {
    Date::from_calendar_date(2030, Month::December, 31).unwrap()
}

fn create_simple_pool() -> AssetPool {
    let mut pool = AssetPool::new("POOL", DealType::ABS, Currency::USD);
    pool.assets.push(PoolAsset::fixed_rate_bond(
        "A1",
        Money::new(5_000_000.0, Currency::USD),
        0.06,
        Date::from_calendar_date(2029, Month::January, 1).unwrap(),
    ));
    pool.assets.push(PoolAsset::fixed_rate_bond(
        "A2",
        Money::new(3_000_000.0, Currency::USD),
        0.05,
        Date::from_calendar_date(2028, Month::January, 1).unwrap(),
    ));
    pool
}

fn create_simple_tranches() -> TrancheStructure {
    let senior = Tranche::new(
        "SENIOR",
        0.0,
        100.0,
        TrancheSeniority::Senior,
        Money::new(8_000_000.0, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.035 },
        Date::from_calendar_date(2030, Month::January, 1).unwrap(),
    )
    .unwrap();
    TrancheStructure::new(vec![senior]).unwrap()
}

fn create_simple_waterfall() -> WaterfallEngine {
    let fees = vec![PaymentRule::new(
        "trustee",
        1,
        PaymentRecipient::ServiceProvider("Trustee".to_string()),
        PaymentCalculation::FixedAmount {
            amount: Money::new(10_000.0, Currency::USD),
        },
    )];

    let tranches = create_simple_tranches();
    WaterfallEngine::standard_sequential(Currency::USD, &tranches, fees)
}

fn flat_discount_curve(rate: f64, base: Date) -> DiscountCurve {
    DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .knots(vec![(0.0, 1.0), (1.0, (-rate).exp()), (5.0, (-rate * 5.0).exp())])
        .build()
        .unwrap()
}

// ============================================================================
// Basic Pricing Tests
// ============================================================================

#[test]
fn test_structured_credit_value_computation() {
    // Arrange
    let sc = StructuredCredit::new_abs(
        "TEST_ABS",
        create_simple_pool(),
        create_simple_tranches(),
        create_simple_waterfall(),
        Date::from_calendar_date(2024, Month::January, 1).unwrap(),
        maturity_date(),
        "USD-OIS",
    );

    let market = MarketContext::new()
        .insert_discount(flat_discount_curve(0.04, test_date()));

    // Act
    let result = sc.value(&market, test_date());

    // Assert
    assert!(result.is_ok());
    let value = result.unwrap();
    assert!(value.amount() > 0.0);
}

#[test]
fn test_structured_credit_dirty_price() {
    // Arrange
    let sc = StructuredCredit::new_abs(
        "TEST_ABS",
        create_simple_pool(),
        create_simple_tranches(),
        create_simple_waterfall(),
        Date::from_calendar_date(2024, Month::January, 1).unwrap(),
        maturity_date(),
        "USD-OIS",
    );

    let market = MarketContext::new()
        .insert_discount(flat_discount_curve(0.04, test_date()));

    // Act
    let result = sc.price_with_metrics(&market, test_date(), &[MetricId::DirtyPrice]);

    // Assert
    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(result.measures.contains_key("dirty_price"));
    
    let price = result.measures["dirty_price"];
    assert!(
        price > 0.0 && price < 200.0,
        "Price should be reasonable: {}",
        price
    );
}

#[test]
fn test_structured_credit_clean_price() {
    // Arrange
    let sc = StructuredCredit::new_abs(
        "TEST_ABS",
        create_simple_pool(),
        create_simple_tranches(),
        create_simple_waterfall(),
        Date::from_calendar_date(2024, Month::January, 1).unwrap(),
        maturity_date(),
        "USD-OIS",
    );

    let market = MarketContext::new()
        .insert_discount(flat_discount_curve(0.04, test_date()));

    // Act
    let result = sc.price_with_metrics(
        &market,
        test_date(),
        &[MetricId::DirtyPrice, MetricId::CleanPrice, MetricId::Accrued],
    );

    // Assert
    assert!(result.is_ok());
    let result = result.unwrap();
    
    let dirty = result.measures["dirty_price"];
    let clean = result.measures["clean_price"];
    let accrued = result.measures["accrued"];
    
    // Clean should be <= Dirty
    assert!(clean <= dirty + 0.01); // Small tolerance for rounding
    assert!(accrued >= 0.0);
}

// ============================================================================
// Metrics Suite Tests
// ============================================================================

#[test]
fn test_structured_credit_full_metric_suite() {
    // Arrange
    let sc = StructuredCredit::new_clo(
        "TEST_CLO",
        create_simple_pool(),
        create_simple_tranches(),
        create_simple_waterfall(),
        Date::from_calendar_date(2024, Month::January, 1).unwrap(),
        maturity_date(),
        "USD-OIS",
    );

    let market = MarketContext::new()
        .insert_discount(flat_discount_curve(0.04, test_date()));

    // Act: Request comprehensive metrics
    let result = sc.price_with_metrics(
        &market,
        test_date(),
        &[
            MetricId::Accrued,
            MetricId::DirtyPrice,
            MetricId::CleanPrice,
            MetricId::WAL,
            MetricId::DurationMac,
            MetricId::DurationMod,
            MetricId::ZSpread,
            MetricId::Cs01,
            MetricId::SpreadDuration,
            MetricId::Ytm,
            MetricId::WAM,
            MetricId::CPR,
            MetricId::CDR,
        ],
    );

    // Assert
    assert!(
        result.is_ok(),
        "Full metric suite should compute: {:?}",
        result.err()
    );

    let result = result.unwrap();
    assert_eq!(
        result.measures.len(),
        13,
        "Should compute all 13 metrics"
    );

    // Verify all metrics are finite
    for (key, value) in &result.measures {
        assert!(value.is_finite(), "Metric {} should be finite", key);
    }
}

#[test]
fn test_structured_credit_empty_metrics_request() {
    // Arrange
    let sc = StructuredCredit::new_clo(
        "TEST_CLO",
        create_simple_pool(),
        create_simple_tranches(),
        create_simple_waterfall(),
        Date::from_calendar_date(2024, Month::January, 1).unwrap(),
        maturity_date(),
        "USD-OIS",
    );

    let market = MarketContext::new()
        .insert_discount(flat_discount_curve(0.04, test_date()));

    // Act: Request NO metrics
    let result = sc.price_with_metrics(&market, test_date(), &[]);

    // Assert
    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(result.measures.is_empty());
}

#[test]
fn test_structured_credit_metric_dependency_resolution() {
    // Arrange: CleanPrice depends on DirtyPrice and Accrued
    let sc = StructuredCredit::new_abs(
        "TEST_ABS",
        create_simple_pool(),
        create_simple_tranches(),
        create_simple_waterfall(),
        Date::from_calendar_date(2024, Month::January, 1).unwrap(),
        maturity_date(),
        "USD-OIS",
    );

    let market = MarketContext::new()
        .insert_discount(flat_discount_curve(0.04, test_date()));

    // Act: Request only CleanPrice (dependencies should auto-compute)
    let result = sc.price_with_metrics(&market, test_date(), &[MetricId::CleanPrice]);

    // Assert
    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(result.measures.contains_key("clean_price"));
}

// ============================================================================
// Performance and Edge Cases
// ============================================================================

#[test]
fn test_structured_credit_pool_balance_cleanup() {
    // Arrange: Pool with very small remaining balance
    let mut pool = AssetPool::new("SMALL_POOL", DealType::ABS, Currency::USD);
    pool.assets.push(PoolAsset::fixed_rate_bond(
        "A1",
        Money::new(50.0, Currency::USD), // Below cleanup threshold
        0.06,
        maturity_date(),
    ));

    let tranches = create_simple_tranches();
    let waterfall = create_simple_waterfall();

    let sc = StructuredCredit::new_abs(
        "SMALL_ABS",
        pool,
        tranches,
        waterfall,
        Date::from_calendar_date(2024, Month::January, 1).unwrap(),
        maturity_date(),
        "USD-OIS",
    );

    let market = MarketContext::new()
        .insert_discount(flat_discount_curve(0.04, test_date()));

    // Act
    let result = sc.build_schedule(&market, test_date());

    // Assert: Should handle small balances gracefully
    assert!(result.is_ok());
}

