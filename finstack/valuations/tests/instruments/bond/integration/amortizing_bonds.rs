#![cfg(feature = "slow")]
//! Amortizing bond integration tests.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_valuations::cashflow::primitives::AmortizationSpec;
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::common::traits::Instrument;
use time::macros::date;

fn create_curve(base_date: Date) -> MarketContext {
    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (5.0, 0.85)])
        .build()
        .unwrap();
    MarketContext::new().insert_discount(curve)
}

#[test]
fn test_linear_amortization() {
    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "AMORT_LINEAR",
        Money::new(1000.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    );

    bond.amortization = Some(AmortizationSpec::LinearTo {
        final_notional: Money::new(400.0, Currency::USD),
    });

    let market = create_curve(as_of);
    let pv = bond.value(&market, as_of);

    // Amortizing bond should price successfully
    assert!(pv.is_ok(), "Amortizing bond should price without error");

    let pv_amount = pv.unwrap().amount();

    // PV should be finite
    assert!(pv_amount.is_finite(), "PV should be finite");
}

#[test]
fn test_full_amortization() {
    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "AMORT_FULL",
        Money::new(1000.0, Currency::USD),
        0.06,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    );

    bond.amortization = Some(AmortizationSpec::LinearTo {
        final_notional: Money::new(0.0, Currency::USD),
    });

    let market = create_curve(as_of);
    let pv = bond.value(&market, as_of);

    // Fully amortizing bond should price successfully
    assert!(
        pv.is_ok(),
        "Fully amortizing bond should price without error"
    );

    let pv_amount = pv.unwrap().amount();

    // PV should be finite
    assert!(pv_amount.is_finite(), "PV should be finite");
}
