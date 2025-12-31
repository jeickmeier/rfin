//! Revolving credit construction and validation tests.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Tenor};
use finstack_core::money::Money;
use finstack_valuations::instruments::fixed_income::revolving_credit::{
    BaseRateSpec, DrawRepaySpec, RevolvingCredit, RevolvingCreditFees,
};
use time::macros::date;

#[test]
fn test_builder_fixed_rate_facility() {
    // Arrange & Act
    let facility = RevolvingCredit::builder()
        .id("RC-FIXED-001".into())
        .commitment_amount(Money::new(10_000_000.0, Currency::USD))
        .drawn_amount(Money::new(5_000_000.0, Currency::USD))
        .commitment_date(date!(2025 - 01 - 01))
        .maturity_date(date!(2028 - 01 - 01))
        .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 }) // 5%
        .day_count(DayCount::Act360)
        .payment_frequency(Tenor::quarterly())
        .fees(RevolvingCreditFees::flat(25.0, 10.0, 5.0))
        .draw_repay_spec(DrawRepaySpec::Scheduled(vec![]))
        .discount_curve_id("USD-OIS".into())
        .build();

    // Assert
    assert!(facility.is_ok());
    let facility = facility.unwrap();
    assert_eq!(facility.id.as_str(), "RC-FIXED-001");
    assert!(matches!(facility.base_rate_spec, BaseRateSpec::Fixed { .. }));
}

#[test]
fn test_builder_floating_rate_facility() {
    // Arrange & Act
    let facility = RevolvingCredit::builder()
        .id("RC-FLOAT-001".into())
        .commitment_amount(Money::new(20_000_000.0, Currency::USD))
        .drawn_amount(Money::new(10_000_000.0, Currency::USD))
        .commitment_date(date!(2025 - 01 - 01))
        .maturity_date(date!(2030 - 01 - 01))
        .base_rate_spec(BaseRateSpec::Floating {
            index_curve_id: "USD-SOFR".into(),
            spread: 0.0250, // 250 bps
        })
        .day_count(DayCount::Act360)
        .payment_frequency(Tenor::quarterly())
        .fees(RevolvingCreditFees::flat(30.0, 15.0, 8.0))
        .draw_repay_spec(DrawRepaySpec::Scheduled(vec![]))
        .discount_curve_id("USD-OIS".into())
        .build();

    // Assert
    assert!(facility.is_ok());
    let facility = facility.unwrap();
    assert!(matches!(
        facility.base_rate_spec,
        BaseRateSpec::Floating { .. }
    ));
}

#[test]
fn test_builder_with_tiered_fees() {
    // Arrange & Act
    let facility = RevolvingCredit::builder()
        .id("RC-TIERED-001".into())
        .commitment_amount(Money::new(50_000_000.0, Currency::USD))
        .drawn_amount(Money::new(0.0, Currency::USD))
        .commitment_date(date!(2025 - 01 - 01))
        .maturity_date(date!(2030 - 01 - 01))
        .base_rate_spec(BaseRateSpec::Fixed { rate: 0.055 })
        .day_count(DayCount::Act360)
        .payment_frequency(Tenor::quarterly())
        .fees(RevolvingCreditFees::tiered(
            25.0,
            vec![(0.0, 10.0), (0.5, 15.0), (0.75, 20.0)],
            5.0,
        ))
        .draw_repay_spec(DrawRepaySpec::Scheduled(vec![]))
        .discount_curve_id("USD-OIS".into())
        .build();

    // Assert
    assert!(facility.is_ok());
}

#[test]
fn test_validation_maturity_after_commitment() {
    // Arrange & Act - maturity before commitment should fail
    let facility = RevolvingCredit::builder()
        .id("RC-INVALID-001".into())
        .commitment_amount(Money::new(10_000_000.0, Currency::USD))
        .drawn_amount(Money::new(5_000_000.0, Currency::USD))
        .commitment_date(date!(2030 - 01 - 01))
        .maturity_date(date!(2025 - 01 - 01)) // Before commitment!
        .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
        .day_count(DayCount::Act360)
        .payment_frequency(Tenor::quarterly())
        .fees(RevolvingCreditFees::flat(25.0, 10.0, 5.0))
        .draw_repay_spec(DrawRepaySpec::Scheduled(vec![]))
        .discount_curve_id("USD-OIS".into())
        .build();

    // Assert
    assert!(facility.is_err());
}

#[test]
fn test_validation_drawn_within_commitment() {
    // Arrange & Act - drawn > commitment should fail
    let facility = RevolvingCredit::builder()
        .id("RC-OVERDRAWN".into())
        .commitment_amount(Money::new(10_000_000.0, Currency::USD))
        .drawn_amount(Money::new(15_000_000.0, Currency::USD)) // Over commitment!
        .commitment_date(date!(2025 - 01 - 01))
        .maturity_date(date!(2030 - 01 - 01))
        .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
        .day_count(DayCount::Act360)
        .payment_frequency(Tenor::quarterly())
        .fees(RevolvingCreditFees::flat(25.0, 10.0, 5.0))
        .draw_repay_spec(DrawRepaySpec::Scheduled(vec![]))
        .discount_curve_id("USD-OIS".into())
        .build();

    // Assert
    assert!(facility.is_err());
}
