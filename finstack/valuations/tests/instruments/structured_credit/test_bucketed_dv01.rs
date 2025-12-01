//! StructuredCredit BucketedDv01 smoke tests

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::structured_credit::{
    DealType, PaymentCalculation, Pool, PoolAsset, Recipient, RecipientType, Seniority,
    StructuredCredit, Tranche, TrancheCoupon, TrancheStructure, Waterfall,
};
use finstack_valuations::metrics::MetricId;
use time::Month;

fn flat_discount_curve(rate: f64, base: Date) -> DiscountCurve {
    DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .knots(vec![
            (0.0, 1.0),
            (0.5, (-rate * 0.5).exp()),
            (1.0, (-rate).exp()),
            (2.0, (-rate * 2.0).exp()),
            (5.0, (-rate * 5.0).exp()),
            (10.0, (-rate * 10.0).exp()),
        ])
        .build()
        .unwrap()
}

fn create_simple_pool() -> Pool {
    let mut pool = Pool::new("POOL", DealType::ABS, Currency::USD);
    pool.assets.push(PoolAsset::fixed_rate_bond(
        "A1",
        Money::new(5_000_000.0, Currency::USD),
        0.06,
        Date::from_calendar_date(2029, Month::January, 1).unwrap(),
        finstack_core::dates::DayCount::Thirty360,
    ));
    pool
}

fn create_simple_tranches() -> TrancheStructure {
    let senior = Tranche::new(
        "SENIOR",
        0.0,
        100.0,
        Seniority::Senior,
        Money::new(5_000_000.0, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.035 },
        Date::from_calendar_date(2030, Month::January, 1).unwrap(),
    )
    .unwrap();
    TrancheStructure::new(vec![senior]).unwrap()
}

fn create_simple_waterfall() -> Waterfall {
    let fees = vec![Recipient::new(
        "trustee",
        RecipientType::ServiceProvider("Trustee".to_string()),
        PaymentCalculation::FixedAmount {
            amount: Money::new(10_000.0, Currency::USD),
            rounding: None,
        },
    )];

    let tranches = create_simple_tranches();
    Waterfall::standard_sequential(Currency::USD, &tranches, fees)
}

#[test]
fn test_structured_credit_bucketed_dv01_computed() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::December, 31).unwrap();

    let sc = StructuredCredit::new_abs(
        "TEST_ABS",
        create_simple_pool(),
        create_simple_tranches(),
        create_simple_waterfall(),
        Date::from_calendar_date(2024, Month::January, 1).unwrap(),
        maturity,
        "USD-OIS",
    )
    .with_payment_calendar("nyse");

    let market = MarketContext::new().insert_discount(flat_discount_curve(0.04, as_of));

    let result = sc
        .price_with_metrics(&market, as_of, &[MetricId::BucketedDv01])
        .unwrap();

    // BucketedDv01 should be present
    assert!(
        result.measures.contains_key("bucketed_dv01"),
        "BucketedDv01 should be computed"
    );

    let bucketed_dv01 = *result.measures.get("bucketed_dv01").unwrap();
    assert!(
        bucketed_dv01.is_finite(),
        "BucketedDv01 should be finite, got {}",
        bucketed_dv01
    );
}
