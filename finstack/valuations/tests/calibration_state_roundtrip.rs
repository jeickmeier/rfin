//! MarketContextState round-trip tests.
//!
//! Verifies that MarketContext can be serialized to MarketContextState,
//! converted to JSON, and deserialized back with identical content.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::{MarketContext, MarketContextState};
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
use finstack_core::market_data::term_structures::Seniority;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use time::Month;

#[test]
fn test_empty_context_roundtrip() {
    let ctx = MarketContext::new();
    let state: MarketContextState = (&ctx).into();

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&state)
        .expect("Failed to serialize empty context");

    // Deserialize back
    let parsed_state: MarketContextState = serde_json::from_str(&json)
        .expect("Failed to deserialize context state");

    // Verify empty
    assert!(parsed_state.curves.is_empty());
    assert!(parsed_state.surfaces.is_empty());
    assert!(parsed_state.prices.is_empty());
}

#[test]
fn test_discount_curve_roundtrip() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    
    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(vec![(0.0, 1.0), (1.0, 0.98), (5.0, 0.90)])
        .set_interp(InterpStyle::MonotoneConvex)
        .build()
        .unwrap();

    let ctx = MarketContext::new().insert_discount(curve);
    let state: MarketContextState = (&ctx).into();

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&state)
        .expect("Failed to serialize context with discount curve");

    // Deserialize back
    let parsed_state: MarketContextState = serde_json::from_str(&json)
        .expect("Failed to deserialize context state");

    // Reconstruct context
    let reconstructed_ctx: MarketContext = parsed_state.try_into()
        .expect("Failed to reconstruct context from state");

    // Verify curve exists and matches
    let retrieved_curve = reconstructed_ctx.get_discount("USD-OIS")
        .expect("Discount curve not found in reconstructed context");

    assert_eq!(retrieved_curve.id().as_str(), "USD-OIS");
    assert_eq!(retrieved_curve.base_date(), base_date);
    
    // Verify discount factors match
    assert!((retrieved_curve.df(0.0) - 1.0).abs() < 1e-12);
    assert!((retrieved_curve.df(1.0) - 0.98).abs() < 1e-12);
    assert!((retrieved_curve.df(5.0) - 0.90).abs() < 1e-12);
}

#[test]
fn test_multiple_curves_roundtrip() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(vec![(0.0, 1.0), (1.0, 0.98), (5.0, 0.90)])
        .build()
        .unwrap();

    let fwd = ForwardCurve::builder("USD-SOFR-3M-FWD", 0.25)
        .base_date(base_date)
        .knots(vec![(0.0, 0.045), (1.0, 0.046), (5.0, 0.048)])
        .build()
        .unwrap();

    let hazard = HazardCurve::builder("AAPL-Senior")
        .base_date(base_date)
        .seniority(Seniority::Senior)
        .recovery_rate(0.40)
        .knots(vec![(1.0, 0.010), (3.0, 0.012), (5.0, 0.015)])
        .build()
        .unwrap();

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_forward(fwd)
        .insert_hazard(hazard)
        .insert_price("AAPL", MarketScalar::Price(Money::new(180.0, Currency::USD)));

    let state: MarketContextState = (&ctx).into();

    // Serialize
    let json = serde_json::to_string_pretty(&state)
        .expect("Failed to serialize complex context");

    // Deserialize
    let parsed_state: MarketContextState = serde_json::from_str(&json)
        .expect("Failed to deserialize context state");

    // Reconstruct
    let reconstructed: MarketContext = parsed_state.try_into()
        .expect("Failed to reconstruct context");

    // Verify all curves present
    assert!(reconstructed.get_discount("USD-OIS").is_ok());
    assert!(reconstructed.get_forward("USD-SOFR-3M-FWD").is_ok());
    assert!(reconstructed.get_hazard("AAPL-Senior").is_ok());
    
    // Verify price
    let aapl_price = reconstructed.price("AAPL").expect("AAPL price not found");
    match aapl_price {
        MarketScalar::Price(money) => {
            assert_eq!(money.currency(), Currency::USD);
            assert!((money.amount() - 180.0).abs() < 1e-9);
        }
        _ => panic!("Expected price scalar"),
    }
}

#[test]
fn test_context_stats_preserved() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(vec![(0.0, 1.0), (5.0, 0.90)])
        .build()
        .unwrap();

    let original_ctx = MarketContext::new().insert_discount(disc);
    let original_stats = original_ctx.stats();

    // Convert to state and back
    let state: MarketContextState = (&original_ctx).into();
    let json = serde_json::to_string_pretty(&state).unwrap();
    let parsed_state: MarketContextState = serde_json::from_str(&json).unwrap();
    let reconstructed_ctx: MarketContext = parsed_state.try_into().unwrap();
    
    let reconstructed_stats = reconstructed_ctx.stats();

    // Verify stats match
    assert_eq!(reconstructed_stats.total_curves, original_stats.total_curves);
    assert_eq!(reconstructed_stats.surface_count, original_stats.surface_count);
    assert_eq!(reconstructed_stats.price_count, original_stats.price_count);
}

