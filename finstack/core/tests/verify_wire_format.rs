//! Verify that the refactored serialization produces the same wire format.

#![cfg(feature = "serde")]

use finstack_core::dates::Date;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use time::Month;

#[test]
fn discount_curve_wire_format_contains_expected_fields() {
    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
        .knots([(0.0, 1.0), (5.0, 0.9)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let json = serde_json::to_string_pretty(&curve).unwrap();
    
    println!("Serialized JSON:\n{}", json);
    
    // Verify key fields are present in the JSON
    assert!(json.contains(r#""id""#), "Should have id field");
    assert!(json.contains(r#""USD-OIS""#), "Should have curve ID");
    assert!(json.contains(r#""base""#), "Should have base date field");
    assert!(json.contains(r#""2025-01-01""#), "Should have base date value");
    assert!(json.contains(r#""knot_points""#), "Should have knot_points field");
    assert!(json.contains(r#""interp_style""#), "Should have interp_style field");
    assert!(json.contains(r#""day_count""#), "Should have day_count field");
}

