//! Tests for Repurchase Agreement (Repo) instruments.

use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::MarketContext;
use finstack_core::math::interp::InterpStyle;
use finstack_core::prelude::*;
use finstack_valuations::cashflow::traits::CashflowProvider;
use finstack_valuations::instruments::common::traits::*;
use finstack_valuations::instruments::{CollateralSpec, CollateralType, Repo, RepoType};
use finstack_valuations::metrics::*;
use time::Month;

fn test_date(year: i32, month: u8, day: u8) -> Date {
    Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap()
}

fn create_test_discount_curve() -> DiscountCurve {
    DiscountCurve::builder("USD-OIS")
        .base_date(test_date(2025, 1, 1))
        .knots([(0.0, 1.0), (0.25, 0.9875), (1.0, 0.95), (5.0, 0.78)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap()
}

fn create_test_market_context() -> MarketContext {
    let disc_curve = create_test_discount_curve();

    MarketContext::new()
        .insert_discount(disc_curve)
        .insert_price("TREASURY_BOND_PRICE", MarketScalar::Unitless(1.02)) // Treasury at 102%
        .insert_price("CORPORATE_BOND_PRICE", MarketScalar::Unitless(0.98)) // Corporate at 98%
        .insert_price("SPECIAL_BOND_PRICE", MarketScalar::Unitless(1.05)) // Special collateral at 105%
}

fn create_general_collateral() -> CollateralSpec {
    CollateralSpec::new("TREASURY_BOND", 1_000_000.0, "TREASURY_BOND_PRICE")
}

fn create_special_collateral() -> CollateralSpec {
    CollateralSpec::special(
        "SPECIAL_BOND_ID",
        "SPECIAL_BOND",
        500_000.0,
        "SPECIAL_BOND_PRICE",
        Some(-25.0), // 25bp lower rate for special collateral
    )
}

#[test]
fn test_overnight_repo_creation() {
    let collateral = create_general_collateral();
    let start_date = test_date(2025, 1, 15);

    let repo = Repo::overnight(
        "REPO_OVERNIGHT_001",
        Money::new(1_000_000.0, Currency::USD),
        collateral,
        0.05, // 5% repo rate
        start_date,
        "USD-OIS",
    )
    .unwrap();

    assert_eq!(repo.repo_type, RepoType::Overnight);
    assert_eq!(repo.start_date, start_date);
    // Maturity should be next business day
    assert!(repo.maturity > start_date);
    assert_eq!(repo.cash_amount.amount(), 1_000_000.0);
    assert_eq!(repo.repo_rate, 0.05);
}

#[test]
fn test_term_repo_creation() {
    let collateral = create_general_collateral();
    let start_date = test_date(2025, 1, 15);
    let maturity = test_date(2025, 4, 15);

    let repo = Repo::term(
        "REPO_TERM_001",
        Money::new(2_000_000.0, Currency::USD),
        collateral,
        0.045, // 4.5% repo rate
        start_date,
        maturity,
        "USD-OIS",
    );

    assert_eq!(repo.repo_type, RepoType::Term);
    assert_eq!(repo.start_date, start_date);
    assert_eq!(repo.maturity, maturity);
    assert_eq!(repo.cash_amount.amount(), 2_000_000.0);
}

#[test]
fn test_repo_builder_pattern() {
    let collateral = create_general_collateral();

    let repo = Repo::builder()
        .id("REPO_BUILDER_001".into())
        .cash_amount(Money::new(500_000.0, Currency::USD))
        .collateral(collateral)
        .repo_rate(0.055)
        .start_date(test_date(2025, 1, 10))
        .maturity(test_date(2025, 7, 10))
        .repo_type(RepoType::Term)
        .haircut(0.025) // 2.5% haircut
        .day_count(DayCount::Act360)
        .bdc(BusinessDayConvention::Following)
        .calendar_id_opt(Some("target2"))
        .triparty(true)
        .disc_id("USD-OIS")
        .attributes(
            Attributes::new()
                .with_tag("funding")
                .with_meta("desk", "repo_trading"),
        )
        .build()
        .unwrap();

    assert_eq!(repo.id.as_str(), "REPO_BUILDER_001");
    assert_eq!(repo.cash_amount.amount(), 500_000.0);
    assert_eq!(repo.repo_rate, 0.055);
    assert_eq!(repo.haircut, 0.025);
    assert!(repo.triparty);
    assert!(repo.attributes.has_tag("funding"));
    assert_eq!(repo.attributes.get_meta("desk"), Some("repo_trading"));
}

#[test]
fn test_collateral_value_calculation() {
    let context = create_test_market_context();
    let collateral = create_general_collateral();

    let market_value = collateral.market_value(&context).unwrap();

    // Expected: 1,000,000 * 1.02 = 1,020,000
    assert_eq!(market_value.amount(), 1_020_000.0);
    assert_eq!(market_value.currency(), Currency::USD);
}

#[test]
fn test_haircut_calculation() {
    let collateral = create_general_collateral();

    let repo = Repo::term(
        "REPO_HAIRCUT_TEST",
        Money::new(1_000_000.0, Currency::USD),
        collateral,
        0.05,
        test_date(2025, 1, 15),
        test_date(2025, 4, 15),
        "USD-OIS",
    );

    let required_collateral = repo.required_collateral_value();

    // Expected: 1,000,000 * (1 + 0.02) = 1,020,000
    assert_eq!(required_collateral.amount(), 1_020_000.0);
}

#[test]
fn test_collateral_adequacy_check() {
    let context = create_test_market_context();
    let collateral = create_general_collateral();

    let repo = Repo::term(
        "REPO_ADEQUACY_TEST",
        Money::new(1_000_000.0, Currency::USD),
        collateral,
        0.05,
        test_date(2025, 1, 15),
        test_date(2025, 4, 15),
        "USD-OIS",
    );

    let is_adequate = repo.is_adequately_collateralized(&context).unwrap();

    // Collateral value: 1,020,000, Required: 1,020,000 -> adequate
    assert!(is_adequate);
}

#[test]
fn test_insufficient_collateral() {
    let context = create_test_market_context();

    // Create collateral worth less than required
    let collateral = CollateralSpec::new("CORPORATE_BOND", 1_000_000.0, "CORPORATE_BOND_PRICE");

    let repo = Repo::term(
        "REPO_INSUFFICIENT_TEST",
        Money::new(1_000_000.0, Currency::USD),
        collateral,
        0.05,
        test_date(2025, 1, 15),
        test_date(2025, 4, 15),
        "USD-OIS",
    );

    let is_adequate = repo.is_adequately_collateralized(&context).unwrap();

    // Collateral value: 980,000, Required: 1,020,000 -> inadequate
    assert!(!is_adequate);
}

#[test]
fn test_special_collateral_rate_adjustment() {
    let special_collateral = create_special_collateral();

    let repo = Repo::term(
        "REPO_SPECIAL_TEST",
        Money::new(1_000_000.0, Currency::USD),
        special_collateral,
        0.05, // Base 5% rate
        test_date(2025, 1, 15),
        test_date(2025, 4, 15),
        "USD-OIS",
    );

    let effective_rate = repo.effective_rate();

    // Expected: 5% - 25bp = 4.75%
    assert!((effective_rate - 0.0475).abs() < 1e-9);
}

#[test]
fn test_repo_interest_calculation() {
    let collateral = create_general_collateral();

    let repo = Repo::term(
        "REPO_INTEREST_TEST",
        Money::new(1_000_000.0, Currency::USD),
        collateral,
        0.05, // 5% annual rate
        test_date(2025, 1, 15),
        test_date(2025, 4, 15), // 3-month term (90 days approximately)
        "USD-OIS",
    );

    let interest = repo.interest_amount().unwrap();

    // Using Act/360: approximately 90/360 = 0.25 years
    // Expected interest: 1,000,000 * 0.05 * 0.25 = 12,500
    let expected = 1_000_000.0 * 0.05 * (90.0 / 360.0);
    assert!((interest.amount() - expected).abs() < 100.0); // Allow for day count differences
}

#[test]
fn test_repo_present_value() {
    let context = create_test_market_context();
    let collateral = create_general_collateral();

    let repo = Repo::term(
        "REPO_PV_TEST",
        Money::new(1_000_000.0, Currency::USD),
        collateral,
        0.05,
        test_date(2025, 1, 15),
        test_date(2025, 4, 15),
        "USD-OIS",
    );

    let valuation_date = test_date(2025, 1, 10); // Before repo start
    let pv = repo.value(&context, valuation_date).unwrap();

    // PV calculated successfully

    // For a repo valued before start date, the PV is the NPV of future cash flows
    // The repo involves: pay cash at start, receive principal+interest at maturity
    // Net PV should be close to the present value of the interest component
    assert_eq!(pv.currency(), Currency::USD);

    // The PV could be negative if the discount rate exceeds the repo rate
    // Let's just verify it's a reasonable value and currency is correct
    assert!(pv.amount().abs() < 100_000.0); // Should be much less than principal
}

#[test]
fn test_repo_cashflow_schedule() {
    let context = create_test_market_context();
    let collateral = create_general_collateral();

    let repo = Repo::term(
        "REPO_CF_TEST",
        Money::new(1_000_000.0, Currency::USD),
        collateral,
        0.05,
        test_date(2025, 1, 15),
        test_date(2025, 4, 15),
        "USD-OIS",
    );

    let cashflows = repo
        .build_schedule(&context, test_date(2025, 1, 10))
        .unwrap();

    assert_eq!(cashflows.len(), 2);

    // First cashflow: initial cash outflow
    let (start_date, cash_outflow) = &cashflows[0];
    assert_eq!(*start_date, test_date(2025, 1, 15));
    assert_eq!(cash_outflow.amount(), -1_000_000.0); // Negative for outflow

    // Second cashflow: principal + interest inflow
    let (maturity_date, cash_inflow) = &cashflows[1];
    assert_eq!(*maturity_date, test_date(2025, 4, 15));
    assert!(cash_inflow.amount() > 1_000_000.0); // Principal + interest
}

#[test]
fn test_repo_metrics_calculation() {
    let context = create_test_market_context();
    let collateral = create_general_collateral();

    let repo = Repo::term(
        "REPO_METRICS_TEST",
        Money::new(1_000_000.0, Currency::USD),
        collateral,
        0.05,
        test_date(2025, 1, 15),
        test_date(2025, 4, 15),
        "USD-OIS",
    );

    let metrics = vec![
        MetricId::CollateralValue,
        MetricId::RequiredCollateral,
        MetricId::CollateralCoverage,
        MetricId::RepoInterest,
        MetricId::EffectiveRate,
    ];

    let result = repo
        .price_with_metrics(&context, test_date(2025, 1, 10), &metrics)
        .unwrap();

    // Verify base valuation completed
    assert!(result.value.amount() != 0.0);

    // Check that all requested metrics are present
    assert!(result.measures.contains_key("collateral_value"));
    assert!(result.measures.contains_key("required_collateral"));
    assert!(result.measures.contains_key("collateral_coverage"));
    assert!(result.measures.contains_key("repo_interest"));
    assert!(result.measures.contains_key("effective_rate"));
}

#[test]
fn test_builder_validation() {
    // Test missing required fields
    let result = Repo::builder().build();
    assert!(result.is_err()); // Should fail due to missing required fields

    // Test invalid dates
    let collateral = create_general_collateral();
    let result = Repo::builder()
        .id("INVALID_DATES".into())
        .cash_amount(Money::new(1_000_000.0, Currency::USD))
        .collateral(collateral)
        .repo_rate(0.05)
        .start_date(test_date(2025, 4, 15))
        .maturity(test_date(2025, 1, 15)) // End before start
        .disc_id("USD-OIS")
        .build();
    assert!(result.is_err()); // Should fail due to invalid date range

    // Test negative repo rate
    let collateral = create_general_collateral();
    let result = Repo::builder()
        .id("NEGATIVE_RATE".into())
        .cash_amount(Money::new(1_000_000.0, Currency::USD))
        .collateral(collateral)
        .repo_rate(-0.01) // Negative rate
        .start_date(test_date(2025, 1, 15))
        .maturity(test_date(2025, 4, 15))
        .disc_id("USD-OIS")
        .build();
    assert!(result.is_err()); // Should fail due to negative rate
}

#[test]
fn test_overnight_repo_maturity_calculation() {
    let collateral = create_general_collateral();

    // Friday start should mature on Monday
    let friday = test_date(2025, 1, 17); // Friday

    let repo = Repo::builder()
        .id("OVERNIGHT_WEEKEND".into())
        .cash_amount(Money::new(1_000_000.0, Currency::USD))
        .collateral(collateral)
        .repo_rate(0.05)
        // simulate overnight via convenience constructor
        // leave builder path for broader coverage
        .start_date(friday)
        .maturity(
            friday
                .add_business_days(1, &finstack_core::dates::calendar::Target2)
                .unwrap(),
        )
        .repo_type(RepoType::Overnight)
        .haircut(0.02)
        .day_count(DayCount::Act360)
        .bdc(BusinessDayConvention::Following)
        .calendar_id_opt(Some("target2"))
        .triparty(false)
        .disc_id("USD-OIS")
        .build()
        .unwrap();

    // Should mature on Monday (next business day)
    let monday = test_date(2025, 1, 20);
    assert_eq!(repo.maturity, monday);
}

#[test]
fn test_open_repo_functionality() {
    let collateral = create_general_collateral();

    let repo = Repo::builder()
        .id("OPEN_REPO_001".into())
        .cash_amount(Money::new(1_000_000.0, Currency::USD))
        .collateral(collateral)
        .repo_rate(0.05)
        .start_date(test_date(2025, 1, 15))
        .maturity(test_date(2025, 12, 15)) // Initial 1-year term
        .repo_type(RepoType::Open)
        .haircut(0.02)
        .day_count(DayCount::Act360)
        .bdc(BusinessDayConvention::Following)
        .calendar_id_opt(Some("target2"))
        .triparty(false)
        .disc_id("USD-OIS")
        .build()
        .unwrap();

    assert_eq!(repo.repo_type, RepoType::Open);
    // Open repos should be priced like term repos initially
    assert_eq!(repo.maturity, test_date(2025, 12, 15));
}

#[test]
fn test_triparty_repo_flag() {
    let collateral = create_general_collateral();

    let repo = Repo::builder()
        .id("TRIPARTY_TEST".into())
        .cash_amount(Money::new(1_000_000.0, Currency::USD))
        .collateral(collateral)
        .repo_rate(0.05)
        .start_date(test_date(2025, 1, 15))
        .maturity(test_date(2025, 4, 15))
        .repo_type(RepoType::Term)
        .haircut(0.02)
        .day_count(DayCount::Act360)
        .bdc(BusinessDayConvention::Following)
        .calendar_id_opt(Some("target2"))
        .triparty(true)
        .disc_id("USD-OIS")
        .build()
        .unwrap();

    assert!(repo.triparty);
}

#[test]
fn test_collateral_types() {
    // Test general collateral
    let general = CollateralSpec::new("TREASURY", 1_000_000.0, "TREASURY_PRICE");
    assert!(matches!(general.collateral_type, CollateralType::General));

    // Test special collateral
    let special = CollateralSpec::special(
        "ON_THE_RUN_10Y",
        "TREASURY_10Y",
        1_000_000.0,
        "TREASURY_10Y_PRICE",
        Some(-15.0), // 15bp special
    );

    if let CollateralType::Special {
        security_id,
        rate_adjustment_bp,
    } = &special.collateral_type
    {
        assert_eq!(security_id, "ON_THE_RUN_10Y");
        assert_eq!(*rate_adjustment_bp, Some(-15.0));
    } else {
        panic!("Expected special collateral type");
    }
}

#[test]
fn test_repo_with_different_currencies() {
    let _context = create_test_market_context();

    // EUR cash with EUR collateral
    let eur_collateral = CollateralSpec::new("EUR_BOND", 1_000_000.0, "TREASURY_BOND_PRICE");

    let eur_repo = Repo::term(
        "EUR_REPO_001",
        Money::new(1_000_000.0, Currency::EUR),
        eur_collateral,
        0.035, // Lower EUR rates
        test_date(2025, 1, 15),
        test_date(2025, 4, 15),
        "USD-OIS", // Using USD curve for simplicity in test
    );

    // Should calculate interest in EUR
    let interest = eur_repo.interest_amount().unwrap();
    assert_eq!(interest.currency(), Currency::EUR);
    assert!(interest.amount() > 0.0);
}

#[test]
fn test_different_day_count_conventions() {
    let collateral = create_general_collateral();

    // Test with Act/365 instead of default Act/360
    let repo = Repo::builder()
        .id("DAYCOUNT_TEST".into())
        .cash_amount(Money::new(1_000_000.0, Currency::USD))
        .collateral(collateral)
        .repo_rate(0.05)
        .start_date(test_date(2025, 1, 15))
        .maturity(test_date(2025, 4, 15))
        .repo_type(RepoType::Term)
        .haircut(0.02)
        .day_count(DayCount::Act365F)
        .bdc(BusinessDayConvention::Following)
        .calendar_id_opt(Some("target2"))
        .triparty(false)
        .disc_id("USD-OIS")
        .build()
        .unwrap();

    let interest_365 = repo.interest_amount().unwrap();

    // Create similar repo with Act/360
    let collateral2 = create_general_collateral();
    let repo_360 = Repo::builder()
        .id("DAYCOUNT_360_TEST".into())
        .cash_amount(Money::new(1_000_000.0, Currency::USD))
        .collateral(collateral2)
        .repo_rate(0.05)
        .start_date(test_date(2025, 1, 15))
        .maturity(test_date(2025, 4, 15))
        .repo_type(RepoType::Term)
        .haircut(0.02)
        .day_count(DayCount::Act360)
        .bdc(BusinessDayConvention::Following)
        .calendar_id_opt(Some("target2"))
        .triparty(false)
        .disc_id("USD-OIS")
        .build()
        .unwrap();

    let interest_360 = repo_360.interest_amount().unwrap();

    // Act/360 should give slightly higher interest than Act/365 for same period
    assert!(interest_360.amount() > interest_365.amount());
}

#[test]
fn test_repo_total_repayment() {
    let collateral = create_general_collateral();

    let repo = Repo::term(
        "REPAYMENT_TEST",
        Money::new(1_000_000.0, Currency::USD),
        collateral,
        0.05,
        test_date(2025, 1, 15),
        test_date(2025, 4, 15),
        "USD-OIS",
    );

    let total = repo.total_repayment().unwrap();

    // Should be principal + interest
    assert!(total.amount() > 1_000_000.0);
    assert_eq!(total.currency(), Currency::USD);

    // Verify it equals principal + interest
    let interest = repo.interest_amount().unwrap();
    let expected_total = repo.cash_amount.checked_add(interest).unwrap();
    assert_eq!(total.amount(), expected_total.amount());
}

#[test]
fn test_zero_rate_repo() {
    let collateral = create_general_collateral();

    let repo = Repo::term(
        "ZERO_RATE_TEST",
        Money::new(1_000_000.0, Currency::USD),
        collateral,
        0.0, // Zero rate
        test_date(2025, 1, 15),
        test_date(2025, 4, 15),
        "USD-OIS",
    );

    let interest = repo.interest_amount().unwrap();
    let total = repo.total_repayment().unwrap();

    assert_eq!(interest.amount(), 0.0);
    assert_eq!(total.amount(), 1_000_000.0); // Just principal
}

#[test]
fn test_repo_metrics_registry_integration() {
    let registry = standard_registry();
    // Verify key Repo metrics are available in the standard registry
    assert!(registry.has_metric(MetricId::CollateralValue));
    assert!(registry.has_metric(MetricId::RequiredCollateral));
    assert!(registry.has_metric(MetricId::CollateralCoverage));
    assert!(registry.has_metric(MetricId::RepoInterest));
    assert!(registry.has_metric(MetricId::EffectiveRate));
    assert!(registry.has_metric(MetricId::Dv01));
    assert!(registry.has_metric(MetricId::FundingRisk));
    assert!(registry.has_metric(MetricId::TimeToMaturity));
    assert!(registry.has_metric(MetricId::ImpliedCollateralReturn));

    // Check applicability to Repo instruments
    assert!(registry.is_applicable(&MetricId::CollateralValue, "Repo"));
    assert!(registry.is_applicable(&MetricId::RequiredCollateral, "Repo"));
    assert!(registry.is_applicable(&MetricId::Dv01, "Repo"));
}

#[test]
fn test_edge_case_same_day_maturity() {
    let collateral = create_general_collateral();

    // Same day start and maturity should fail validation
    let same_date = test_date(2025, 1, 15);
    let result = Repo::builder()
        .id("SAME_DAY_TEST".into())
        .cash_amount(Money::new(1_000_000.0, Currency::USD))
        .collateral(collateral)
        .repo_rate(0.05)
        .start_date(same_date)
        .maturity(same_date)
        .disc_id("USD-OIS")
        .build();

    assert!(result.is_err()); // Should fail due to invalid date range
}

#[test]
fn test_high_haircut_scenario() {
    let collateral = create_general_collateral();

    let repo = Repo::builder()
        .id("HIGH_HAIRCUT_TEST".into())
        .cash_amount(Money::new(1_000_000.0, Currency::USD))
        .collateral(collateral)
        .repo_rate(0.05)
        .start_date(test_date(2025, 1, 15))
        .maturity(test_date(2025, 4, 15))
        .haircut(0.20) // 20% haircut
        .repo_type(RepoType::Term)
        .day_count(DayCount::Act360)
        .bdc(BusinessDayConvention::Following)
        .calendar_id_opt(Some("target2"))
        .triparty(false)
        .disc_id("USD-OIS")
        .build()
        .unwrap();

    let required = repo.required_collateral_value();

    // Expected: 1,000,000 * (1 + 0.20) = 1,200,000
    assert_eq!(required.amount(), 1_200_000.0);
}

#[test]
fn test_repo_attributes_and_tagging() {
    let collateral = create_general_collateral();

    let repo = Repo::builder()
        .id("TAGGED_REPO".into())
        .cash_amount(Money::new(1_000_000.0, Currency::USD))
        .collateral(collateral)
        .repo_rate(0.05)
        .start_date(test_date(2025, 1, 15))
        .maturity(test_date(2025, 4, 15))
        .repo_type(RepoType::Term)
        .haircut(0.02)
        .day_count(DayCount::Act360)
        .bdc(BusinessDayConvention::Following)
        .calendar_id_opt(Some("target2"))
        .triparty(false)
        .disc_id("USD-OIS")
        .attributes(
            Attributes::new()
                .with_tag("treasury")
                .with_tag("short_term")
                .with_meta("counterparty", "PRIMARY_DEALER_A")
                .with_meta("settlement", "DVP"),
        )
        .build()
        .unwrap();

    assert!(repo.attributes.has_tag("treasury"));
    assert!(repo.attributes.has_tag("short_term"));
    assert_eq!(
        repo.attributes.get_meta("counterparty"),
        Some("PRIMARY_DEALER_A")
    );
    assert_eq!(repo.attributes.get_meta("settlement"), Some("DVP"));
}
