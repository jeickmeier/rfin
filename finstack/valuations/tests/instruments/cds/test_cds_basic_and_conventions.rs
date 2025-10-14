#![cfg(test)]

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::money::Money;
use finstack_valuations::instruments::cds::CDSConvention;
use finstack_valuations::instruments::CreditDefaultSwap;
use time::Month;

#[test]
fn test_cds_creation_and_basic_pricing() {
    // Create a CDS instrument
    let notional = Money::new(10_000_000.0, Currency::USD);
    let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let end = Date::from_calendar_date(2030, Month::January, 1).unwrap();

    let spread_bp = 100.0; // 100bp
    // Create CDS with buy_protection and customize recovery
    let mut cds = CreditDefaultSwap::buy_protection(
        "CDS_TEST",
        notional,
        spread_bp,
        start,
        end,
        finstack_core::types::CurveId::new("USD-OIS"),
        finstack_core::types::CurveId::new("ABC-SENIOR"),
    );
    // Customize recovery rate
    cds.protection.recovery_rate = 0.4;

    assert_eq!(cds.id.as_str(), "CDS_TEST");
    assert_eq!(cds.premium.spread_bp, 100.0);
    assert_eq!(cds.protection.recovery_rate, 0.4);
    assert_eq!(cds.convention, CDSConvention::IsdaNa);
}

#[test]
fn test_cds_isda_conventions() {
    // Test different ISDA conventions
    assert_eq!(CDSConvention::IsdaNa.day_count(), DayCount::Act360);
    assert_eq!(CDSConvention::IsdaEu.day_count(), DayCount::Act360);
    assert_eq!(CDSConvention::IsdaAs.day_count(), DayCount::Act365F);

    assert_eq!(CDSConvention::IsdaNa.frequency(), Frequency::quarterly());
    assert_eq!(CDSConvention::IsdaEu.frequency(), Frequency::quarterly());
}


