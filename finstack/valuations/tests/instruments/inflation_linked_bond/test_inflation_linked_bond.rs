#![cfg(test)]

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_valuations::instruments::inflation_linked_bond::parameters::InflationLinkedBondParams;
use finstack_valuations::instruments::inflation_linked_bond::IndexationMethod;
use finstack_valuations::instruments::InflationLinkedBond;
use time::Month;

#[test]
fn test_inflation_linked_bond_creation() {
    let notional = Money::new(1_000_000.0, Currency::USD);
    let issue = Date::from_calendar_date(2020, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 15).unwrap();

    let bond_params = InflationLinkedBondParams::tips(
        notional, 0.0125, // 1.25% real coupon
        issue, maturity, 250.0, // Base CPI
    );

    let tips = InflationLinkedBond::new_tips("US_TIPS_2030", &bond_params, "USD-REAL", "US-CPI-U");

    assert_eq!(tips.id, "US_TIPS_2030");
    assert_eq!(tips.indexation_method, IndexationMethod::TIPS);
    assert_eq!(tips.real_coupon, 0.0125);
    assert_eq!(tips.base_index, 250.0);

    // Test UK linker creation
    let gbp_notional = Money::new(1_000_000.0, Currency::GBP);
    let base_date = Date::from_calendar_date(2019, Month::November, 1).unwrap();

    let uk_bond_params = InflationLinkedBondParams::uk_linker(
        gbp_notional,
        0.00625, // 0.625% real coupon
        issue,
        maturity,
        280.0, // Base RPI
    );

    let uk_linker = InflationLinkedBond::new_uk_linker(
        "UK_LINKER_2040",
        &uk_bond_params,
        base_date,
        "GBP-NOMINAL",
        "UK-RPI",
    );

    assert_eq!(uk_linker.id, "UK_LINKER_2040");
    assert_eq!(uk_linker.indexation_method, IndexationMethod::UK);
}


