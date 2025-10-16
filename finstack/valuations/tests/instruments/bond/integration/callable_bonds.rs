//! Callable and putable bond integration tests.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_valuations::instruments::bond::{Bond, CallPut, CallPutSchedule};
use finstack_valuations::instruments::common::traits::Instrument;
use time::macros::date;

fn create_curve(base_date: Date) -> MarketContext {
    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (5.0, 0.80)])
        .build()
        .unwrap();
    MarketContext::new().insert_discount(curve)
}

#[test]
fn test_callable_bond_basic() {
    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "CALLABLE",
        Money::new(1000.0, Currency::USD),
        0.06,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    );

    let mut schedule = CallPutSchedule::default();
    schedule.calls.push(CallPut {
        date: date!(2027 - 01 - 01),
        price_pct_of_par: 102.0,
    });
    bond.call_put = Some(schedule);

    let market = create_curve(as_of);
    let pv = bond.value(&market, as_of).unwrap();

    assert!(pv.amount() > 0.0);
    assert!(pv.amount() < 1200.0);
}

#[test]
fn test_putable_bond_basic() {
    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "PUTABLE",
        Money::new(1000.0, Currency::USD),
        0.04,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    );

    let mut schedule = CallPutSchedule::default();
    schedule.puts.push(CallPut {
        date: date!(2027 - 01 - 01),
        price_pct_of_par: 98.0,
    });
    bond.call_put = Some(schedule);

    let market = create_curve(as_of);
    let pv = bond.value(&market, as_of).unwrap();

    assert!(pv.amount() > 0.0);
}
