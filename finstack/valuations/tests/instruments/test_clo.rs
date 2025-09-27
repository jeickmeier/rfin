//! Integration tests for structured credit instruments.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, Frequency};
use finstack_core::money::Money;
use finstack_core::types::InstrumentId;
use finstack_valuations::instruments::common::structured_credit::{
    self as sc, AbsTranche, AssetPool, AssetType, CoverageTests, CreditRating, DealType, LoanType,
    PropertyType, TrancheCoupon, TrancheSeniority, TrancheStructure, WaterfallBuilder,
};
use finstack_valuations::instruments::{Abs, Cmbs, Clo, Rmbs};
use time::Month;

fn test_date() -> Date {
    Date::from_calendar_date(2025, Month::January, 15).unwrap()
}

fn create_sample_assets() -> Vec<sc::pool::PoolAsset> {
    let maturity = Date::from_calendar_date(2030, Month::January, 15).unwrap();

    (0..3)
        .map(|idx| sc::pool::PoolAsset {
            id: InstrumentId::new(format!("ASSET_{}", idx)),
            asset_type: AssetType::Loan {
                loan_type: LoanType::FirstLien,
                industry: Some(format!("Industry_{}", idx % 2)),
            },
            balance: Money::new(50_000_000.0, Currency::USD),
            rate: 0.08,
            maturity,
            credit_quality: Some(CreditRating::BB),
            industry: Some(format!("Industry_{}", idx % 2)),
            obligor_id: Some(format!("OBLIGOR_{}", idx)),
            is_defaulted: false,
            recovery_amount: None,
            purchase_price: None,
            acquisition_date: Some(test_date()),
        })
        .collect()
}

fn create_sample_mortgages(property: PropertyType) -> Vec<sc::pool::PoolAsset> {
    let maturity = Date::from_calendar_date(2035, Month::January, 15).unwrap();

    (0..3)
        .map(|idx| sc::pool::PoolAsset {
            id: InstrumentId::new(format!("MORTGAGE_{}", idx)),
            asset_type: AssetType::Mortgage {
                property_type: property.clone(),
                ltv: Some(0.65 + 0.05 * idx as f64),
            },
            balance: Money::new(80_000_000.0, Currency::USD),
            rate: 0.05,
            maturity,
            credit_quality: Some(CreditRating::BBB),
            industry: Some("RealEstate".to_string()),
            obligor_id: Some(format!("BORROWER_{}", idx)),
            is_defaulted: false,
            recovery_amount: None,
            purchase_price: None,
            acquisition_date: Some(test_date()),
        })
        .collect()
}

#[test]
fn test_clo_creation() {
    let loans = create_sample_assets();

    // Create asset pool
    let mut pool = AssetPool::new("TEST_POOL", DealType::CLO, Currency::USD);
    for asset in loans {
        pool.assets.push(asset);
    }

    // Create tranches
    let equity_tranche = AbsTranche::new(
        "EQUITY",
        0.0,
        10.0,
        TrancheSeniority::Equity,
        Money::new(30_000_000.0, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.15 },
        Date::from_calendar_date(2030, Month::January, 15).unwrap(),
    )
    .unwrap();

    let senior_tranche = AbsTranche::new(
        "SENIOR_A",
        10.0,
        100.0,
        TrancheSeniority::Senior,
        Money::new(270_000_000.0, Currency::USD),
        TrancheCoupon::Floating {
            index: "SOFR-3M".to_string(),
            spread_bp: 150.0,
            floor: None,
            cap: None,
        },
        Date::from_calendar_date(2030, Month::January, 15).unwrap(),
    )
    .unwrap();

    let tranche_structure = TrancheStructure::new(vec![equity_tranche, senior_tranche]).unwrap();
    let waterfall = WaterfallBuilder::standard_clo(&tranche_structure).build();

    // Create CLO
    let clo = Clo::new(
        "CLO_TEST_1",
        pool,
        tranche_structure,
        waterfall,
        Date::from_calendar_date(2030, Month::January, 15).unwrap(),
        "USD-OIS",
    );

    // Verify basic properties
    assert_eq!(clo.id.as_str(), "CLO_TEST_1");
    assert_eq!(clo.deal_type, DealType::CLO);
    assert_eq!(clo.tranches.tranches.len(), 2);
    assert_eq!(clo.pool.assets.len(), 3);
}

#[test]
fn test_abs_creation() {
    let assets = create_sample_assets();

    let mut pool = AssetPool::new("ABS_POOL", DealType::ABS, Currency::USD);
    for asset in assets {
        pool.assets.push(asset);
    }

    let equity_tranche = AbsTranche::new(
        "EQUITY",
        0.0,
        12.0,
        TrancheSeniority::Equity,
        Money::new(20_000_000.0, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.12 },
        Date::from_calendar_date(2030, Month::January, 15).unwrap(),
    )
    .unwrap();

    let senior_tranche = AbsTranche::new(
        "SENIOR_A",
        12.0,
        100.0,
        TrancheSeniority::Senior,
        Money::new(180_000_000.0, Currency::USD),
        TrancheCoupon::Floating {
            index: "SOFR-1M".to_string(),
            spread_bp: 100.0,
            floor: None,
            cap: None,
        },
        Date::from_calendar_date(2030, Month::January, 15).unwrap(),
    )
    .unwrap();

    let tranche_structure = TrancheStructure::new(vec![equity_tranche, senior_tranche]).unwrap();
    let waterfall = WaterfallBuilder::standard_clo(&tranche_structure).build();

    let abs = Abs::new(
        "ABS_TEST_1",
        pool,
        tranche_structure,
        waterfall,
        Date::from_calendar_date(2030, Month::January, 15).unwrap(),
        "USD-OIS",
    );

    assert_eq!(abs.id.as_str(), "ABS_TEST_1");
    assert_eq!(abs.deal_type, DealType::ABS);
    assert_eq!(abs.payment_frequency, Frequency::monthly());
    assert_eq!(abs.tranches.tranches.len(), 2);
}

#[test]
fn test_rmbs_creation() {
    let mortgages = create_sample_mortgages(PropertyType::SingleFamily);

    let mut pool = AssetPool::new("RMBS_POOL", DealType::RMBS, Currency::USD);
    for mortgage in mortgages {
        pool.assets.push(mortgage);
    }

    let equity_tranche = AbsTranche::new(
        "EQUITY",
        0.0,
        8.0,
        TrancheSeniority::Equity,
        Money::new(25_000_000.0, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.10 },
        Date::from_calendar_date(2035, Month::January, 15).unwrap(),
    )
    .unwrap();

    let senior_tranche = AbsTranche::new(
        "SENIOR_A",
        8.0,
        100.0,
        TrancheSeniority::Senior,
        Money::new(225_000_000.0, Currency::USD),
        TrancheCoupon::Floating {
            index: "SOFR-1M".to_string(),
            spread_bp: 90.0,
            floor: None,
            cap: None,
        },
        Date::from_calendar_date(2035, Month::January, 15).unwrap(),
    )
    .unwrap();

    let tranche_structure = TrancheStructure::new(vec![equity_tranche, senior_tranche]).unwrap();
    let waterfall = WaterfallBuilder::standard_clo(&tranche_structure).build();

    let rmbs = Rmbs::new(
        "RMBS_TEST_1",
        pool,
        tranche_structure,
        waterfall,
        Date::from_calendar_date(2035, Month::January, 15).unwrap(),
        "USD-OIS",
    );

    assert_eq!(rmbs.deal_type, DealType::RMBS);
    assert_eq!(rmbs.payment_frequency, Frequency::monthly());
    assert_eq!(rmbs.tranches.tranches.len(), 2);
}

#[test]
fn test_cmbs_creation() {
    let mortgages = create_sample_mortgages(PropertyType::Commercial);

    let mut pool = AssetPool::new("CMBS_POOL", DealType::CMBS, Currency::USD);
    for mortgage in mortgages {
        pool.assets.push(mortgage);
    }

    let equity_tranche = AbsTranche::new(
        "EQUITY",
        0.0,
        7.0,
        TrancheSeniority::Equity,
        Money::new(40_000_000.0, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.11 },
        Date::from_calendar_date(2032, Month::January, 15).unwrap(),
    )
    .unwrap();

    let senior_tranche = AbsTranche::new(
        "SENIOR_A",
        7.0,
        100.0,
        TrancheSeniority::Senior,
        Money::new(360_000_000.0, Currency::USD),
        TrancheCoupon::Floating {
            index: "SOFR-3M".to_string(),
            spread_bp: 140.0,
            floor: None,
            cap: None,
        },
        Date::from_calendar_date(2032, Month::January, 15).unwrap(),
    )
    .unwrap();

    let tranche_structure = TrancheStructure::new(vec![equity_tranche, senior_tranche]).unwrap();
    let waterfall = WaterfallBuilder::standard_clo(&tranche_structure).build();

    let cmbs = Cmbs::new(
        "CMBS_TEST_1",
        pool,
        tranche_structure,
        waterfall,
        Date::from_calendar_date(2032, Month::January, 15).unwrap(),
        "USD-OIS",
    );

    assert_eq!(cmbs.deal_type, DealType::CMBS);
    assert_eq!(cmbs.payment_frequency, Frequency::monthly());
    assert_eq!(cmbs.tranches.tranches.len(), 2);
}

#[test]
fn test_tranche_loss_allocation() {
    let equity_tranche = AbsTranche::new(
        "EQUITY",
        0.0,
        10.0,
        TrancheSeniority::Equity,
        Money::new(100_000_000.0, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.15 },
        test_date(),
    )
    .unwrap();

    let senior_tranche = AbsTranche::new(
        "SENIOR",
        10.0,
        100.0,
        TrancheSeniority::Senior,
        Money::new(900_000_000.0, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.05 },
        test_date(),
    )
    .unwrap();

    let pool_balance = Money::new(1_000_000_000.0, Currency::USD);

    // Test loss scenarios

    // No loss - equity should have no loss
    let equity_loss = equity_tranche.loss_allocation(0.0, pool_balance);
    assert_eq!(equity_loss.amount(), 0.0);

    // 5% loss - equity takes full loss
    let equity_loss = equity_tranche.loss_allocation(5.0, pool_balance);
    assert_eq!(equity_loss.amount(), 50_000_000.0); // 5% of 1B = 50M, but limited to tranche size

    // 15% loss - equity wiped out, senior takes some loss
    let equity_loss = equity_tranche.loss_allocation(15.0, pool_balance);
    assert_eq!(equity_loss.amount(), 100_000_000.0); // Full equity tranche

    let senior_loss = senior_tranche.loss_allocation(15.0, pool_balance);
    assert_eq!(senior_loss.amount(), 50_000_000.0); // 5% loss above 10% attachment = 5/90 * 900M = 50M
}

#[test]
fn test_coverage_test_framework() {
    let mut coverage_tests = CoverageTests::new();

    // Add OC test for senior tranche
    coverage_tests.add_oc_test("SENIOR_A".to_string(), 1.15, Some(1.20));

    // Add IC test
    coverage_tests.add_ic_test("SENIOR_A".to_string(), 1.10, Some(1.15));

    // Verify tests were added
    assert!(coverage_tests.test_definitions.contains_key("SENIOR_A_OC"));
    assert!(coverage_tests.test_definitions.contains_key("SENIOR_A_IC"));

    let oc_test = &coverage_tests.test_definitions["SENIOR_A_OC"];
    assert_eq!(oc_test.trigger_level, 1.15);
    assert_eq!(oc_test.cure_level, Some(1.20));
}

#[test]
fn test_pool_concentration_limits() {
    let mut pool = AssetPool::new("TEST_POOL", DealType::CLO, Currency::USD);

    // Add diverse assets
    for i in 0..5 {
        let asset = pool::PoolAsset {
            id: format!("ASSET_{}", i),
            asset_type: AssetType::Loan {
                loan_type: LoanType::FirstLien,
                industry: Some(format!("Industry_{}", i % 3)),
            },
            balance: Money::new(50_000_000.0, Currency::USD),
            rate: 0.08,
            maturity: test_date(),
            credit_quality: Some(CreditRating::B),
            industry: Some(format!("Industry_{}", i % 3)),
            obligor_id: Some(format!("Obligor_{}", i)),
            is_defaulted: false,
            recovery_amount: None,
            purchase_price: None,
            acquisition_date: None,
        };
        pool.assets.push(asset);
    }

    // Check concentration limits
    let result = pool.check_concentration_limits();

    // With 5 equal assets of 50M each (250M total), each obligor has 20% concentration
    // Default limit is 2%, so this should violate
    assert!(result.has_violations());
    assert!(!result.violations.is_empty());
}

#[test]
fn test_clo_builder() {
    let loans = create_sample_assets();

    // Create pool
    let mut pool = AssetPool::new("BUILDER_POOL", DealType::CLO, Currency::USD);
    for asset in loans {
        pool.assets.push(asset);
    }

    // Use direct constructor now that bespoke builder is removed
    let equity_tranche = AbsTranche::new(
        "EQUITY",
        0.0,
        15.0,
        TrancheSeniority::Equity,
        Money::new(45_000_000.0, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.18 },
        Date::from_calendar_date(2030, Month::January, 15).unwrap(),
    )
    .unwrap();
    let senior_tranche = AbsTranche::new(
        "SENIOR_A",
        15.0,
        100.0,
        TrancheSeniority::Senior,
        Money::new(255_000_000.0, Currency::USD),
        TrancheCoupon::Floating {
            index: "SOFR-3M".to_string(),
            spread_bp: 200.0,
            floor: None,
            cap: None,
        },
        Date::from_calendar_date(2030, Month::January, 15).unwrap(),
    )
    .unwrap();
    let tranche_structure = TrancheStructure::new(vec![equity_tranche, senior_tranche]).unwrap();
    let waterfall = WaterfallBuilder::standard_clo(&tranche_structure).build();
    let mut clo = Clo::new(
        "CLO_BUILDER_TEST",
        pool,
        tranche_structure,
        waterfall,
        Date::from_calendar_date(2030, Month::January, 15).unwrap(),
        "USD-OIS",
    );
    clo.manager_id = Some("TEST_MANAGER".to_string());

    assert_eq!(clo.id.as_str(), "CLO_BUILDER_TEST");
    assert_eq!(clo.manager_id.as_ref().unwrap(), "TEST_MANAGER");
    assert_eq!(clo.tranches.tranches.len(), 2);

    // Verify tranche structure
    let equity = &clo.tranches.tranches[0];
    assert_eq!(equity.attachment_point, 0.0);
    assert_eq!(equity.detachment_point, 15.0);
    assert_eq!(equity.thickness(), 15.0);
    assert!(equity.is_first_loss());
}
