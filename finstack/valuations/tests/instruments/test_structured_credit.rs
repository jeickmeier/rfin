//! Integration tests for structured credit instruments.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_valuations::instruments::loan::Loan;
use finstack_valuations::instruments::structured_credit::*;
use time::Month;

fn test_date() -> Date {
    Date::from_calendar_date(2025, Month::January, 15).unwrap()
}

fn create_sample_loans() -> Vec<Loan> {
    let base_date = test_date();
    let maturity = Date::from_calendar_date(2030, Month::January, 15).unwrap();

    vec![
        Loan::fixed_rate(
            "LOAN_1",
            Money::new(100_000_000.0, Currency::USD),
            0.08,
            base_date,
            maturity,
        )
        .with_borrower("TechCorp1"),
        Loan::fixed_rate(
            "LOAN_2",
            Money::new(75_000_000.0, Currency::USD),
            0.09,
            base_date,
            maturity,
        )
        .with_borrower("HealthCorp1"),
        Loan::fixed_rate(
            "LOAN_3",
            Money::new(125_000_000.0, Currency::USD),
            0.12,
            base_date,
            maturity,
        )
        .with_borrower("EnergyCorp1"),
    ]
}

#[test]
fn test_clo_creation() {
    let loans = create_sample_loans();

    // Create asset pool
    let mut pool = AssetPool::new("TEST_POOL", DealType::CLO, Currency::USD);
    for loan in &loans {
        pool.add_loan(loan, Some("Technology".to_string()));
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
    let clo = StructuredCredit::new(
        "CLO_TEST_1",
        DealType::CLO,
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
    let loans = create_sample_loans();

    // Create pool
    let mut pool = AssetPool::new("BUILDER_POOL", DealType::CLO, Currency::USD);
    for loan in &loans {
        pool.add_loan(loan, Some("Mixed".to_string()));
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
    let mut clo = StructuredCredit::new(
        "CLO_BUILDER_TEST",
        DealType::CLO,
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
