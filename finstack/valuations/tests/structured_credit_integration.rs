//! Comprehensive integration tests for structured credit metrics.
//!
//! Tests all 4 structured credit instrument types (CLO, ABS, RMBS, CMBS) with
//! realistic test data to verify cashflow generation and metric computation.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_core::types::InstrumentId;
use finstack_valuations::instruments::structured_credit::StructuredCredit;
use finstack_valuations::instruments::common::structured_credit::{
    AssetPool, AssetType, CreditRating, DealType, Tranche, TrancheCoupon,
    TrancheSeniority, TrancheStructure, WaterfallEngine,
};
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::MetricId;
use time::Month;

// ============================================================================
// Test Data Helpers
// ============================================================================

fn test_date() -> Date {
    Date::from_calendar_date(2025, Month::October, 5).unwrap()
}

fn maturity_date() -> Date {
    Date::from_calendar_date(2030, Month::December, 31).unwrap()
}

/// Create a realistic CLO asset pool with corporate loans
fn create_clo_pool() -> AssetPool {
    let mut pool = AssetPool::new("CLO_POOL", DealType::CLO, Currency::USD);
    
    // Add 5 corporate loans
    for i in 0..5 {
        let asset = finstack_valuations::instruments::common::structured_credit::pool::PoolAsset {
            id: InstrumentId::new(format!("LOAN_{}", i)),
            asset_type: AssetType::FirstLienLoan {
                industry: Some(format!("Industry_{}", i % 3)),
            },
            balance: Money::new(30_000_000.0, Currency::USD),
            rate: 0.08,
            spread_bps: Some(450.0 + i as f64 * 50.0),
            index_id: Some("SOFR-3M".to_string()),
            maturity: maturity_date(),
            credit_quality: Some(CreditRating::BB),
            industry: Some(format!("Industry_{}", i % 3)),
            obligor_id: Some(format!("OBLIGOR_{}", i)),
            is_defaulted: false,
            recovery_amount: None,
            purchase_price: None,
            acquisition_date: Some(test_date()),
        };
        pool.assets.push(asset);
    }
    
    pool
}

/// Create a realistic tranche structure for testing
fn create_test_tranches() -> TrancheStructure {
    let equity = Tranche::new(
        "EQUITY",
        0.0,
        10.0,
        TrancheSeniority::Equity,
        Money::new(15_000_000.0, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.15 },
        maturity_date(),
    )
    .expect("Failed to create equity tranche");

    let senior = Tranche::new(
        "SENIOR_A",
        10.0,
        100.0,
        TrancheSeniority::Senior,
        Money::new(135_000_000.0, Currency::USD),
        TrancheCoupon::Floating {
            forward_curve_id: finstack_core::types::CurveId::new("SOFR-3M".to_string()),
            spread_bp: 200.0,
            floor: None,
            cap: None,
        },
        maturity_date(),
    )
    .expect("Failed to create senior tranche");

    TrancheStructure::new(vec![equity, senior]).expect("Failed to create tranche structure")
}

/// Create a simple waterfall engine for testing
fn create_test_waterfall() -> WaterfallEngine {
    WaterfallEngine::standard_clo(Currency::USD)
}

/// Create market context with discount curve
fn create_test_market() -> MarketContext {
    let discount_curve = DiscountCurve::builder("USD_OIS")
        .base_date(test_date())
        .knots(vec![
            (0.0, 1.0),
            (0.25, 0.9875),
            (1.0, 0.95),
            (5.0, 0.78),
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .expect("Failed to create discount curve");

    MarketContext::new().insert_discount(discount_curve)
}

// ============================================================================
// Instrument Creation Tests
// ============================================================================

#[test]
fn test_clo_creation_with_realistic_data() {
    let clo = StructuredCredit::new_clo(
        "TEST_CLO",
        create_clo_pool(),
        create_test_tranches(),
        create_test_waterfall(),
        maturity_date(),
        "USD_OIS",
    );

    assert_eq!(clo.id.as_str(), "TEST_CLO");
    assert_eq!(clo.deal_type, DealType::CLO);
    assert_eq!(clo.pool.assets.len(), 5);
    assert_eq!(clo.tranches.tranches.len(), 2);
}

#[test]
fn test_abs_creation_with_realistic_data() {
    let abs = StructuredCredit::new_abs(
        "TEST_ABS",
        create_clo_pool(),
        create_test_tranches(),
        create_test_waterfall(),
        maturity_date(),
        "USD_OIS",
    );

    assert_eq!(abs.id.as_str(), "TEST_ABS");
    assert_eq!(abs.deal_type, DealType::ABS);
}

// ============================================================================
// Cashflow Generation Tests
// ============================================================================

#[test]
fn test_clo_generates_cashflows() {
    use finstack_valuations::cashflow::traits::CashflowProvider;
    
    let clo = StructuredCredit::new_clo(
        "TEST_CLO",
        create_clo_pool(),
        create_test_tranches(),
        create_test_waterfall(),
        maturity_date(),
        "USD_OIS",
    );

    let market = create_test_market();
    let result = clo.build_schedule(&market, test_date());

    assert!(result.is_ok(), "Cashflow generation should work: {:?}", result.err());

    let flows = result.unwrap();
    assert!(!flows.is_empty(), "Should generate at least some cashflows");

    // Verify cashflows are in the future
    for (date, _amount) in &flows {
        assert!(*date >= test_date(), "All cashflows should be in future");
    }
}

// ============================================================================
// Basic Metric Tests
// ============================================================================

#[test]
fn test_clo_dirty_price() {
    let clo = StructuredCredit::new_clo(
        "TEST_CLO",
        create_clo_pool(),
        create_test_tranches(),
        create_test_waterfall(),
        maturity_date(),
        "USD_OIS",
    );

    let market = create_test_market();
    let result = clo.price_with_metrics(&market, test_date(), &[MetricId::DirtyPrice]);

    assert!(result.is_ok(), "DirtyPrice should compute: {:?}", result.err());

    let result = result.unwrap();
    assert!(
        result.measures.contains_key("dirty_price"),
        "Should contain dirty_price"
    );

    // Price should be reasonable (not negative, not extreme)
    let price = result.measures["dirty_price"];
    assert!(
        (0.0..=200.0).contains(&price),
        "Price should be reasonable: {}",
        price
    );
}

#[test]
fn test_clo_wal() {
    let clo = StructuredCredit::new_clo(
        "TEST_CLO",
        create_clo_pool(),
        create_test_tranches(),
        create_test_waterfall(),
        maturity_date(),
        "USD_OIS",
    );

    let market = create_test_market();
    let result = clo.price_with_metrics(&market, test_date(), &[MetricId::WAL]);

    assert!(result.is_ok(), "WAL should compute: {:?}", result.err());

    let result = result.unwrap();
    assert!(result.measures.contains_key("wal"), "Should contain WAL");

    // WAL should be positive and reasonable
    let wal = result.measures["wal"];
    assert!(wal >= 0.0, "WAL should be non-negative: {}", wal);
    assert!(wal <= 10.0, "WAL should be reasonable: {}", wal);
}

// ============================================================================
// Advanced Metric Tests
// ============================================================================

#[test]
fn test_clo_durations() {
    let clo = StructuredCredit::new_clo(
        "TEST_CLO",
        create_clo_pool(),
        create_test_tranches(),
        create_test_waterfall(),
        maturity_date(),
        "USD_OIS",
    );

    let market = create_test_market();
    let result = clo.price_with_metrics(
        &market,
        test_date(),
        &[MetricId::DurationMac, MetricId::DurationMod],
    );

    assert!(result.is_ok(), "Durations should compute: {:?}", result.err());

    let result = result.unwrap();
    assert!(
        result.measures.contains_key("duration_mac"),
        "Should have Macaulay duration"
    );
    assert!(
        result.measures.contains_key("duration_mod"),
        "Should have Modified duration"
    );

    // Durations should be positive and reasonable
    let mac = result.measures["duration_mac"];
    let mod_dur = result.measures["duration_mod"];
    assert!(mac >= 0.0, "Macaulay duration should be non-negative");
    assert!(mod_dur >= 0.0, "Modified duration should be non-negative");
    assert!(mac <= 20.0, "Macaulay duration should be reasonable");
}

#[test]
fn test_clo_spread_metrics() {
    let clo = StructuredCredit::new_clo(
        "TEST_CLO",
        create_clo_pool(),
        create_test_tranches(),
        create_test_waterfall(),
        maturity_date(),
        "USD_OIS",
    );

    let market = create_test_market();
    let result = clo.price_with_metrics(
        &market,
        test_date(),
        &[MetricId::ZSpread, MetricId::Cs01, MetricId::SpreadDuration],
    );

    assert!(result.is_ok(), "Spread metrics should compute: {:?}", result.err());

    let result = result.unwrap();
    assert!(result.measures.contains_key("z_spread"), "Should have Z-spread");
    assert!(result.measures.contains_key("cs01"), "Should have CS01");
    assert!(
        result.measures.contains_key("spread_duration"),
        "Should have spread duration"
    );

    // CS01 should be positive (price falls when spread rises)
    let cs01 = result.measures["cs01"];
    assert!(cs01 >= 0.0, "CS01 should be non-negative: {}", cs01);
}

#[test]
fn test_clo_ytm() {
    let clo = StructuredCredit::new_clo(
        "TEST_CLO",
        create_clo_pool(),
        create_test_tranches(),
        create_test_waterfall(),
        maturity_date(),
        "USD_OIS",
    );

    let market = create_test_market();
    let result = clo.price_with_metrics(&market, test_date(), &[MetricId::Ytm]);

    assert!(result.is_ok(), "YTM should compute: {:?}", result.err());

    let result = result.unwrap();
    assert!(result.measures.contains_key("ytm"), "Should have YTM");

    // YTM should be reasonable (typically 2-10%)
    let ytm = result.measures["ytm"];
    assert!(
        (-0.05..=0.30).contains(&ytm),
        "YTM should be reasonable: {}",
        ytm
    );
}

#[test]
fn test_clo_pool_metrics() {
    let clo = StructuredCredit::new_clo(
        "TEST_CLO",
        create_clo_pool(),
        create_test_tranches(),
        create_test_waterfall(),
        maturity_date(),
        "USD_OIS",
    );

    let market = create_test_market();
    let result = clo.price_with_metrics(
        &market,
        test_date(),
        &[MetricId::WAM, MetricId::CPR, MetricId::CDR],
    );

    assert!(result.is_ok(), "Pool metrics should compute: {:?}", result.err());

    let result = result.unwrap();
    assert!(result.measures.contains_key("wam"), "Should have WAM");
    assert!(result.measures.contains_key("cpr"), "Should have CPR");
    assert!(result.measures.contains_key("cdr"), "Should have CDR");

    // All should be non-negative
    assert!(result.measures["wam"] >= 0.0);
    assert!(result.measures["cpr"] >= 0.0);
    assert!(result.measures["cdr"] >= 0.0);
}

// ============================================================================
// Full Suite Tests
// ============================================================================

#[test]
fn test_clo_full_metric_suite() {
    let clo = StructuredCredit::new_clo(
        "TEST_CLO",
        create_clo_pool(),
        create_test_tranches(),
        create_test_waterfall(),
        maturity_date(),
        "USD_OIS",
    );

    let market = create_test_market();

    // Request all 13 structured credit metrics
    let result = clo.price_with_metrics(
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

    assert!(result.is_ok(), "All metrics should compute: {:?}", result.err());

    let result = result.unwrap();

    // Verify all 13 metrics are present
    assert_eq!(
        result.measures.len(),
        13,
        "Should have all 13 metrics, got: {:?}",
        result.measures.keys()
    );

    // Spot check key metrics
    assert!(result.measures.contains_key("accrued"));
    assert!(result.measures.contains_key("dirty_price"));
    assert!(result.measures.contains_key("clean_price"));
    assert!(result.measures.contains_key("wal"));
    assert!(result.measures.contains_key("duration_mac"));
    assert!(result.measures.contains_key("duration_mod"));
    assert!(result.measures.contains_key("z_spread"));
    assert!(result.measures.contains_key("cs01"));
    assert!(result.measures.contains_key("spread_duration"));
    assert!(result.measures.contains_key("ytm"));
    assert!(result.measures.contains_key("wam"));
    assert!(result.measures.contains_key("cpr"));
    assert!(result.measures.contains_key("cdr"));
}

// ============================================================================
// Cross-Instrument Tests
// ============================================================================

#[test]
fn test_all_instruments_compute_basic_metrics() {
    // Test that all 4 instrument types support core metrics
    let market = create_test_market();
    let as_of = test_date();

    let metrics = &[
        MetricId::Accrued,
        MetricId::DirtyPrice,
        MetricId::WAL,
        MetricId::DurationMod,
    ];

    // CLO
    let clo = StructuredCredit::new_clo(
        "CLO",
        create_clo_pool(),
        create_test_tranches(),
        create_test_waterfall(),
        maturity_date(),
        "USD_OIS",
    );
    let clo_result = clo.price_with_metrics(&market, as_of, metrics);
    assert!(clo_result.is_ok(), "CLO metrics failed: {:?}", clo_result.err());

    // ABS
    let abs = StructuredCredit::new_abs(
        "ABS",
        create_clo_pool(),
        create_test_tranches(),
        create_test_waterfall(),
        maturity_date(),
        "USD_OIS",
    );
    let abs_result = abs.price_with_metrics(&market, as_of, metrics);
    assert!(abs_result.is_ok(), "ABS metrics failed: {:?}", abs_result.err());

    // RMBS
    let rmbs = StructuredCredit::new_rmbs(
        "RMBS",
        create_clo_pool(),
        create_test_tranches(),
        create_test_waterfall(),
        maturity_date(),
        "USD_OIS",
    );
    let rmbs_result = rmbs.price_with_metrics(&market, as_of, metrics);
    assert!(rmbs_result.is_ok(), "RMBS metrics failed: {:?}", rmbs_result.err());

    // CMBS
    let cmbs = StructuredCredit::new_cmbs(
        "CMBS",
        create_clo_pool(),
        create_test_tranches(),
        create_test_waterfall(),
        maturity_date(),
        "USD_OIS",
    );
    let cmbs_result = cmbs.price_with_metrics(&market, as_of, metrics);
    assert!(cmbs_result.is_ok(), "CMBS metrics failed: {:?}", cmbs_result.err());

    // All should compute successfully
    println!("CLO metrics: {:?}", clo_result.unwrap().measures.len());
    println!("ABS metrics: {:?}", abs_result.unwrap().measures.len());
    println!("RMBS metrics: {:?}", rmbs_result.unwrap().measures.len());
    println!("CMBS metrics: {:?}", cmbs_result.unwrap().measures.len());
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_empty_metrics_request() {
    let clo = StructuredCredit::new_clo(
        "TEST_CLO",
        create_clo_pool(),
        create_test_tranches(),
        create_test_waterfall(),
        maturity_date(),
        "USD_OIS",
    );

    let market = create_test_market();

    // Request NO metrics
    let result = clo.price_with_metrics(&market, test_date(), &[]);

    assert!(result.is_ok(), "Empty metrics should work");
    let result = result.unwrap();

    // Should have no metrics (just base NPV)
    assert!(
        result.measures.is_empty(),
        "Empty request should return no metrics"
    );
}

#[test]
fn test_cs01_is_positive() {
    // CS01 should be positive (price falls when spread rises)
    let clo = StructuredCredit::new_clo(
        "TEST_CLO",
        create_clo_pool(),
        create_test_tranches(),
        create_test_waterfall(),
        maturity_date(),
        "USD_OIS",
    );

    let market = create_test_market();
    let result = clo.price_with_metrics(&market, test_date(), &[MetricId::Cs01]);

    if let Ok(result) = result {
        if let Some(cs01) = result.measures.get("cs01") {
            assert!(*cs01 >= 0.0, "CS01 should be non-negative: {}", cs01);
        }
    }
}

// ============================================================================
// Dependency Resolution Tests
// ============================================================================

#[test]
fn test_metric_dependency_resolution() {
    // Test that dependencies are resolved automatically
    // CleanPrice depends on DirtyPrice and Accrued
    let clo = StructuredCredit::new_clo(
        "TEST_CLO",
        create_clo_pool(),
        create_test_tranches(),
        create_test_waterfall(),
        maturity_date(),
        "USD_OIS",
    );

    let market = create_test_market();

    // Request ONLY CleanPrice (dependencies should auto-compute)
    let result = clo.price_with_metrics(&market, test_date(), &[MetricId::CleanPrice]);

    assert!(
        result.is_ok(),
        "Dependency resolution should work: {:?}",
        result.err()
    );

    let result = result.unwrap();
    assert!(
        result.measures.contains_key("clean_price"),
        "CleanPrice should be computed"
    );
}

#[test]
fn test_spread_duration_from_cs01() {
    // SpreadDuration depends on CS01 which depends on ZSpread which depends on DirtyPrice
    let clo = StructuredCredit::new_clo(
        "TEST_CLO",
        create_clo_pool(),
        create_test_tranches(),
        create_test_waterfall(),
        maturity_date(),
        "USD_OIS",
    );

    let market = create_test_market();

    // Request SpreadDuration (should auto-compute all dependencies)
    let result = clo.price_with_metrics(&market, test_date(), &[MetricId::SpreadDuration]);

    assert!(
        result.is_ok(),
        "SpreadDuration dependency resolution should work: {:?}",
        result.err()
    );

    let result = result.unwrap();
    assert!(
        result.measures.contains_key("spread_duration"),
        "Should have spread_duration"
    );
}

// ============================================================================
// Pool Characteristic Tests
// ============================================================================

#[test]
fn test_pool_statistics() {
    let pool = create_clo_pool();

    // Total balance should equal sum of assets
    let total = pool.total_balance();
    assert_eq!(total.amount(), 150_000_000.0); // 5 loans × $30M

    // WAC should be positive
    let wac = pool.weighted_avg_coupon();
    assert!(wac > 0.0, "WAC should be positive: {}", wac);

    // WAM should be reasonable
    let wam = pool.weighted_avg_maturity(test_date());
    assert!(wam > 0.0, "WAM should be positive: {}", wam);
    assert!(wam < 10.0, "WAM should be reasonable: {}", wam);
}
