//! Integration tests for CommodityForward pricing.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::commodity::commodity_forward::CommodityForward;
use finstack_valuations::instruments::Attributes;
use time::Month;

/// Helper to create a test discount curve.
fn create_test_market() -> MarketContext {
    // Create a flat 5% discount curve for USD-OIS
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let discount_curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (5.0, (-0.05_f64 * 5.0).exp())])
        .build()
        .expect("discount curve should build");

    // Create a flat curve for forward prices (using same curve structure)
    let forward_curve = DiscountCurve::builder("WTI-FORWARD")
        .base_date(base_date)
        .knots([(0.0, 1.0), (5.0, (-0.05_f64 * 5.0).exp())])
        .build()
        .expect("forward curve should build");

    MarketContext::new()
        .insert_discount(discount_curve)
        .insert_discount(forward_curve)
}

#[test]
fn test_commodity_forward_pricing_with_quoted_price() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let settlement = Date::from_calendar_date(2025, Month::June, 15).unwrap();
    let market = create_test_market();

    let forward = CommodityForward::builder()
        .id(InstrumentId::new("WTI-TEST"))
        .commodity_type("Energy".to_string())
        .ticker("CL".to_string())
        .quantity(1000.0)
        .unit("BBL".to_string())
        .multiplier(1.0)
        .settlement_date(settlement)
        .currency(Currency::USD)
        .quoted_price_opt(Some(75.0)) // Quoted at $75/BBL
        .forward_curve_id(CurveId::new("WTI-FORWARD"))
        .discount_curve_id(CurveId::new("USD-OIS"))
        .attributes(Attributes::new())
        .build()
        .expect("should build");

    let npv = forward.npv(&market, as_of).expect("should price");

    // Verify basic properties
    assert_eq!(npv.currency(), Currency::USD);
    assert!(
        npv.amount() > 0.0,
        "NPV should be positive for a long forward"
    );

    // The forward value should be roughly F * Q * M * DF
    // With F=75, Q=1000, M=1, and some discount factor < 1
    // NPV should be less than 75 * 1000 = 75000
    assert!(npv.amount() < 75000.0);
    assert!(npv.amount() > 70000.0); // Given short tenor and 5% rate
}

#[test]
fn test_commodity_forward_pricing_expired() {
    // Test that an expired forward has zero value
    let as_of = Date::from_calendar_date(2025, Month::June, 1).unwrap();
    let settlement = Date::from_calendar_date(2025, Month::May, 15).unwrap(); // Already settled
    let market = create_test_market();

    let forward = CommodityForward::builder()
        .id(InstrumentId::new("EXPIRED"))
        .commodity_type("Energy".to_string())
        .ticker("CL".to_string())
        .quantity(1000.0)
        .unit("BBL".to_string())
        .multiplier(1.0)
        .settlement_date(settlement)
        .currency(Currency::USD)
        .quoted_price_opt(Some(75.0))
        .forward_curve_id(CurveId::new("WTI-FORWARD"))
        .discount_curve_id(CurveId::new("USD-OIS"))
        .attributes(Attributes::new())
        .build()
        .expect("should build");

    let npv = forward.npv(&market, as_of).expect("should price");

    // Expired forward should have zero value
    assert_eq!(npv.amount(), 0.0);
}

#[test]
fn test_commodity_forward_instrument_trait() {
    use finstack_valuations::instruments::Instrument;
    use finstack_valuations::pricer::InstrumentType;

    let forward = CommodityForward::example();

    assert_eq!(forward.id(), "WTI-FWD-2025M03");
    assert_eq!(forward.key(), InstrumentType::CommodityForward);
}

#[test]
fn test_commodity_forward_forward_price_from_curve() {
    // Test forward price calculation without quoted price
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let settlement = Date::from_calendar_date(2025, Month::March, 15).unwrap();
    let market = create_test_market();

    let forward = CommodityForward::builder()
        .id(InstrumentId::new("CURVE-TEST"))
        .commodity_type("Energy".to_string())
        .ticker("CL".to_string())
        .quantity(100.0)
        .unit("BBL".to_string())
        .multiplier(1.0)
        .settlement_date(settlement)
        .currency(Currency::USD)
        // No quoted_price - should use curve
        .forward_curve_id(CurveId::new("WTI-FORWARD"))
        .discount_curve_id(CurveId::new("USD-OIS"))
        .attributes(Attributes::new())
        .build()
        .expect("should build");

    let price = forward
        .forward_price(&market, as_of)
        .expect("should get price");

    // Without quoted price, should interpolate from curve
    assert!(price > 0.0);
}

#[cfg(feature = "serde")]
#[test]
fn test_commodity_forward_serialization() {
    let forward = CommodityForward::example();

    let json = serde_json::to_string_pretty(&forward).expect("serialize");
    let parsed: CommodityForward = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(forward.id.as_str(), parsed.id.as_str());
    assert_eq!(forward.ticker, parsed.ticker);
    assert_eq!(forward.quantity, parsed.quantity);
    assert_eq!(forward.currency, parsed.currency);
}
