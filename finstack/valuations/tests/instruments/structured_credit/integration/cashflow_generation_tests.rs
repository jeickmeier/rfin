//! Integration tests for cashflow generation.
//!
//! Tests end-to-end waterfall execution and cashflow scheduling
//! across different deal types.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_core::types::ratings::CreditRating;
use finstack_core::types::InstrumentId;
use finstack_valuations::cashflow::traits::CashflowProvider;
use finstack_valuations::instruments::structured_credit::{
    AssetPool, AssetType, DealType, ManagementFeeType, PaymentCalculation, PaymentRecipient,
    Recipient, StructuredCredit, Tranche, TrancheCoupon, TrancheSeniority, TrancheStructure,
    WaterfallEngine,
};
use time::Month;

// ============================================================================
// Test Helpers
// ============================================================================

fn test_date() -> Date {
    Date::from_calendar_date(2025, Month::October, 5).unwrap()
}

fn closing_date() -> Date {
    Date::from_calendar_date(2024, Month::January, 1).unwrap()
}

fn maturity_date() -> Date {
    Date::from_calendar_date(2030, Month::December, 31).unwrap()
}

fn create_test_pool() -> AssetPool {
    let mut pool = AssetPool::new("TEST_POOL", DealType::CLO, Currency::USD);

    for i in 0..5 {
        let asset = finstack_valuations::instruments::structured_credit::PoolAsset {
            day_count: Some(finstack_core::dates::DayCount::Act360),
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
        TrancheCoupon::Floating(finstack_valuations::cashflow::builder::FloatingRateSpec {
            index_id: finstack_core::types::CurveId::new("SOFR-3M".to_string()),
            spread_bp: 200.0,
            gearing: 1.0,
            gearing_includes_spread: true,
            floor_bp: None,
            cap_bp: None,
            all_in_floor_bp: None,
            index_cap_bp: None,
            reset_freq: finstack_core::dates::Frequency::quarterly(),
            reset_lag_days: 2,
            dc: finstack_core::dates::DayCount::Act360,
            bdc: finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            fixing_calendar_id: None,
        }),
        maturity_date(),
    )
    .expect("Failed to create senior tranche");

    TrancheStructure::new(vec![equity, senior]).expect("Failed to create tranche structure")
}

fn create_test_waterfall() -> WaterfallEngine {
    let fees = vec![
        Recipient::new(
            "trustee_fees",
            PaymentRecipient::ServiceProvider("Trustee".to_string()),
            PaymentCalculation::FixedAmount {
                amount: Money::new(12_500.0, Currency::USD),
            },
        ),
        Recipient::new(
            "senior_mgmt_fee",
            PaymentRecipient::ManagerFee(ManagementFeeType::Senior),
            PaymentCalculation::PercentageOfCollateral {
                rate: 0.0040,
                annualized: true,
            },
        ),
    ];

    let tranches = create_test_tranches();
    WaterfallEngine::standard_sequential(Currency::USD, &tranches, fees)
}

fn create_test_market() -> MarketContext {
    let discount_curve = DiscountCurve::builder("USD_OIS")
        .base_date(test_date())
        .knots(vec![(0.0, 1.0), (0.25, 0.9875), (1.0, 0.95), (5.0, 0.78)])
        .set_interp(InterpStyle::Linear)
        .build()
        .expect("Failed to create discount curve");

    MarketContext::new().insert_discount(discount_curve)
}

// ============================================================================
// Cashflow Generation Tests
// ============================================================================

#[test]
fn test_clo_generates_cashflows() {
    // Arrange
    let clo = StructuredCredit::new_clo(
        "TEST_CLO",
        create_test_pool(),
        create_test_tranches(),
        create_test_waterfall(),
        closing_date(),
        maturity_date(),
        "USD_OIS",
    );

    let market = create_test_market();

    // Act
    let result = clo.build_schedule(&market, test_date());

    // Assert
    assert!(
        result.is_ok(),
        "Cashflow generation should succeed: {:?}",
        result.err()
    );

    let flows = result.unwrap();
    assert!(!flows.is_empty(), "Should generate cashflows");

    // Verify all cashflows are in the future
    for (date, _amount) in &flows {
        assert!(*date >= test_date(), "All cashflows should be in future");
    }
}

#[test]
fn test_abs_generates_cashflows() {
    // Arrange
    let abs = StructuredCredit::new_abs(
        "TEST_ABS",
        create_test_pool(),
        create_test_tranches(),
        create_test_waterfall(),
        closing_date(),
        maturity_date(),
        "USD_OIS",
    );

    let market = create_test_market();

    // Act
    let result = abs.build_schedule(&market, test_date());

    // Assert
    assert!(result.is_ok());
    let flows = result.unwrap();
    assert!(!flows.is_empty());
}

#[test]
fn test_rmbs_generates_cashflows() {
    // Arrange
    let rmbs = StructuredCredit::new_rmbs(
        "TEST_RMBS",
        create_test_pool(),
        create_test_tranches(),
        create_test_waterfall(),
        closing_date(),
        maturity_date(),
        "USD_OIS",
    );

    let market = create_test_market();

    // Act
    let result = rmbs.build_schedule(&market, test_date());

    // Assert
    assert!(result.is_ok());
    let flows = result.unwrap();
    assert!(!flows.is_empty());
}

#[test]
fn test_cmbs_generates_cashflows() {
    // Arrange
    let cmbs = StructuredCredit::new_cmbs(
        "TEST_CMBS",
        create_test_pool(),
        create_test_tranches(),
        create_test_waterfall(),
        closing_date(),
        maturity_date(),
        "USD_OIS",
    );

    let market = create_test_market();

    // Act
    let result = cmbs.build_schedule(&market, test_date());

    // Assert
    assert!(result.is_ok());
    let flows = result.unwrap();
    assert!(!flows.is_empty());
}

#[test]
fn test_cashflow_dates_respect_payment_frequency() {
    // Arrange: CLO with quarterly payments
    let clo = StructuredCredit::new_clo(
        "TEST_CLO",
        create_test_pool(),
        create_test_tranches(),
        create_test_waterfall(),
        closing_date(),
        maturity_date(),
        "USD_OIS",
    );

    let market = create_test_market();

    // Act
    let flows = clo.build_schedule(&market, test_date()).unwrap();

    // Assert: Payment dates should be quarterly (roughly 3 months apart)
    if flows.len() >= 2 {
        let first_date = flows[0].0;
        let second_date = flows[1].0;
        let days_diff = (second_date - first_date).whole_days();

        // Quarterly is approximately 90 days (allow some variance)
        assert!(
            (days_diff - 90).abs() < 10,
            "Payment dates should be quarterly"
        );
    }
}

#[test]
fn test_cashflow_amounts_are_positive() {
    // Arrange
    let clo = StructuredCredit::new_clo(
        "TEST_CLO",
        create_test_pool(),
        create_test_tranches(),
        create_test_waterfall(),
        closing_date(),
        maturity_date(),
        "USD_OIS",
    );

    let market = create_test_market();

    // Act
    let flows = clo.build_schedule(&market, test_date()).unwrap();

    // Assert
    for (date, amount) in flows {
        assert!(
            amount.amount() >= 0.0,
            "Cashflow at {:?} should be non-negative",
            date
        );
    }
}
