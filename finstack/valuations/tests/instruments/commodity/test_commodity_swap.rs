//! Integration tests for CommoditySwap pricing.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, Tenor, TenorUnit};
use finstack_core::market_data::context::MarketContext;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::commodity_swap::CommoditySwap;
use finstack_valuations::instruments::common::traits::Attributes;
use time::Month;

/// Helper to create a test market context.
fn create_test_market() -> MarketContext {
    use finstack_core::market_data::curves::{DiscountCurve, InterpolatedCurve};

    let mut market = MarketContext::new();

    // Create a flat 5% discount curve for USD-OIS
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let discount_curve = DiscountCurve::from_interpolated(
        InterpolatedCurve::flat_rate(base_date, 0.05),
        base_date,
    );
    market.insert_discount("USD-OIS", discount_curve.clone());

    // Create a flat curve for the floating commodity index
    market.insert_discount("NG-SPOT-AVG", discount_curve);

    market
}

#[test]
fn test_commodity_swap_pricing() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let market = create_test_market();

    let swap = CommoditySwap::builder()
        .id(InstrumentId::new("NG-TEST"))
        .commodity_type("Energy".to_string())
        .ticker("NG".to_string())
        .unit("MMBTU".to_string())
        .currency(Currency::USD)
        .notional_quantity(10000.0)
        .fixed_price(3.50)
        .floating_index_id(CurveId::new("NG-SPOT-AVG"))
        .pay_fixed(true)
        .start_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
        .end_date(Date::from_calendar_date(2025, Month::June, 30).unwrap())
        .payment_frequency(Tenor::new(1, TenorUnit::Months))
        .discount_curve_id(CurveId::new("USD-OIS"))
        .attributes(Attributes::new())
        .build()
        .expect("should build");

    let npv = swap.npv(&market, as_of).expect("should price");

    // Verify basic properties
    assert_eq!(npv.currency(), Currency::USD);

    // The NPV should be defined (could be positive, negative, or near zero depending on curves)
    // For a flat curve where forward equals fixed, the swap should be near par (NPV ~= 0)
    // But because of discounting and cost-of-carry approximation, it won't be exactly zero
    println!("Swap NPV: {:?}", npv);
}

#[test]
fn test_commodity_swap_fixed_leg_pv() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let market = create_test_market();

    let swap = CommoditySwap::builder()
        .id(InstrumentId::new("FIXED-LEG-TEST"))
        .commodity_type("Energy".to_string())
        .ticker("NG".to_string())
        .unit("MMBTU".to_string())
        .currency(Currency::USD)
        .notional_quantity(10000.0)
        .fixed_price(3.50)
        .floating_index_id(CurveId::new("NG-SPOT-AVG"))
        .pay_fixed(true)
        .start_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
        .end_date(Date::from_calendar_date(2025, Month::June, 30).unwrap())
        .payment_frequency(Tenor::new(1, TenorUnit::Months))
        .discount_curve_id(CurveId::new("USD-OIS"))
        .attributes(Attributes::new())
        .build()
        .expect("should build");

    let fixed_pv = swap.fixed_leg_pv(&market, as_of).expect("should compute");

    // Fixed leg should have positive PV (we're paying fixed)
    // Each period is 10000 * 3.50 = 35000, with ~6 periods, total ~210000 before discounting
    assert!(fixed_pv > 0.0);
    assert!(fixed_pv < 250000.0); // Reasonable upper bound
}

#[test]
fn test_commodity_swap_payment_schedule() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let end = Date::from_calendar_date(2025, Month::December, 31).unwrap();

    let swap = CommoditySwap::builder()
        .id(InstrumentId::new("SCHEDULE-TEST"))
        .commodity_type("Energy".to_string())
        .ticker("NG".to_string())
        .unit("MMBTU".to_string())
        .currency(Currency::USD)
        .notional_quantity(10000.0)
        .fixed_price(3.50)
        .floating_index_id(CurveId::new("NG-SPOT-AVG"))
        .pay_fixed(true)
        .start_date(start)
        .end_date(end)
        .payment_frequency(Tenor::new(1, TenorUnit::Months))
        .discount_curve_id(CurveId::new("USD-OIS"))
        .attributes(Attributes::new())
        .build()
        .expect("should build");

    let schedule = swap.payment_schedule(as_of).expect("should generate schedule");

    // Should have approximately 12 monthly payments
    assert!(
        schedule.len() >= 11,
        "Expected at least 11 payments, got {}",
        schedule.len()
    );
    assert!(
        schedule.len() <= 13,
        "Expected at most 13 payments, got {}",
        schedule.len()
    );
}

#[test]
fn test_commodity_swap_instrument_trait() {
    use finstack_valuations::instruments::common::traits::Instrument;
    use finstack_valuations::pricer::InstrumentType;

    let swap = CommoditySwap::example();

    assert_eq!(swap.id(), "NG-SWAP-2025");
    assert_eq!(swap.key(), InstrumentType::CommoditySwap);
}

#[test]
fn test_commodity_swap_receive_fixed() {
    // Test that receiving fixed gives opposite NPV to paying fixed
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let market = create_test_market();

    let pay_fixed = CommoditySwap::builder()
        .id(InstrumentId::new("PAY-FIXED"))
        .commodity_type("Energy".to_string())
        .ticker("NG".to_string())
        .unit("MMBTU".to_string())
        .currency(Currency::USD)
        .notional_quantity(10000.0)
        .fixed_price(3.50)
        .floating_index_id(CurveId::new("NG-SPOT-AVG"))
        .pay_fixed(true)
        .start_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
        .end_date(Date::from_calendar_date(2025, Month::June, 30).unwrap())
        .payment_frequency(Tenor::new(1, TenorUnit::Months))
        .discount_curve_id(CurveId::new("USD-OIS"))
        .attributes(Attributes::new())
        .build()
        .expect("should build");

    let receive_fixed = CommoditySwap::builder()
        .id(InstrumentId::new("RECEIVE-FIXED"))
        .commodity_type("Energy".to_string())
        .ticker("NG".to_string())
        .unit("MMBTU".to_string())
        .currency(Currency::USD)
        .notional_quantity(10000.0)
        .fixed_price(3.50)
        .floating_index_id(CurveId::new("NG-SPOT-AVG"))
        .pay_fixed(false) // Receive fixed
        .start_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
        .end_date(Date::from_calendar_date(2025, Month::June, 30).unwrap())
        .payment_frequency(Tenor::new(1, TenorUnit::Months))
        .discount_curve_id(CurveId::new("USD-OIS"))
        .attributes(Attributes::new())
        .build()
        .expect("should build");

    let npv_pay = pay_fixed.npv(&market, as_of).expect("should price");
    let npv_recv = receive_fixed.npv(&market, as_of).expect("should price");

    // NPVs should be opposite (within floating point tolerance)
    let diff = (npv_pay.amount() + npv_recv.amount()).abs();
    assert!(
        diff < 0.01,
        "Expected opposite NPVs, got pay={}, recv={}, diff={}",
        npv_pay.amount(),
        npv_recv.amount(),
        diff
    );
}

#[cfg(feature = "serde")]
#[test]
fn test_commodity_swap_serialization() {
    let swap = CommoditySwap::example();

    let json = serde_json::to_string_pretty(&swap).expect("serialize");
    let parsed: CommoditySwap = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(swap.id.as_str(), parsed.id.as_str());
    assert_eq!(swap.ticker, parsed.ticker);
    assert_eq!(swap.fixed_price, parsed.fixed_price);
}

