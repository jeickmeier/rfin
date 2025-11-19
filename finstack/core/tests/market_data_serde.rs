//! Serialization tests for market data types.
//!
//! Ensures that the refactored direct-serde implementation preserves wire format
//! compatibility and validation behavior.

#![cfg(feature = "serde")]

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::inflation_index::{
    InflationIndex, InflationInterpolation, InflationLag,
};
use finstack_core::market_data::scalars::{ScalarTimeSeries, SeriesInterpolation};
use finstack_core::market_data::surfaces::vol_surface::VolSurface;
use finstack_core::market_data::term_structures::base_correlation::BaseCorrelationCurve;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
use finstack_core::market_data::term_structures::inflation::InflationCurve;
use finstack_core::math::interp::InterpStyle;
use time::Month;

fn test_date() -> Date {
    Date::from_calendar_date(2025, Month::January, 15).unwrap()
}

#[test]
fn discount_curve_serde_roundtrip() {
    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(test_date())
        .knots([(0.0, 1.0), (1.0, 0.98), (5.0, 0.90)])
        .set_interp(InterpStyle::MonotoneConvex)
        .build()
        .unwrap();

    let json = serde_json::to_string_pretty(&curve).unwrap();
    let deserialized: DiscountCurve = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id(), curve.id());
    assert_eq!(deserialized.base_date(), curve.base_date());
    assert_eq!(deserialized.day_count(), curve.day_count());
    assert_eq!(deserialized.knots(), curve.knots());
    assert_eq!(deserialized.dfs(), curve.dfs());
}

#[test]
fn forward_curve_serde_roundtrip() {
    let curve = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(test_date())
        .knots([(0.0, 0.03), (1.0, 0.035), (5.0, 0.04)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let json = serde_json::to_string_pretty(&curve).unwrap();
    let deserialized: ForwardCurve = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id(), curve.id());
    assert_eq!(deserialized.base_date(), curve.base_date());
    assert_eq!(deserialized.tenor(), curve.tenor());
    assert_eq!(deserialized.knots(), curve.knots());
    assert_eq!(deserialized.forwards(), curve.forwards());
}

#[test]
fn hazard_curve_serde_roundtrip() {
    let curve = HazardCurve::builder("CREDIT-USD")
        .base_date(test_date())
        .knots([(0.0, 0.01), (5.0, 0.015)])
        .recovery_rate(0.4)
        .build()
        .unwrap();

    let json = serde_json::to_string_pretty(&curve).unwrap();
    let deserialized: HazardCurve = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id(), curve.id());
    assert_eq!(deserialized.base_date(), curve.base_date());
    assert_eq!(deserialized.recovery_rate(), curve.recovery_rate());
}

#[test]
fn inflation_curve_serde_roundtrip() {
    let curve = InflationCurve::builder("US-CPI")
        .base_cpi(300.0)
        .knots([(0.0, 300.0), (5.0, 327.0)])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .unwrap();

    let json = serde_json::to_string_pretty(&curve).unwrap();
    let deserialized: InflationCurve = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id(), curve.id());
    assert_eq!(deserialized.base_cpi(), curve.base_cpi());
    assert_eq!(deserialized.knots(), curve.knots());
    assert_eq!(deserialized.cpi_levels(), curve.cpi_levels());
}

#[test]
fn vol_surface_serde_roundtrip() {
    let surface = VolSurface::builder("EQ-VOL")
        .expiries(&[1.0, 2.0])
        .strikes(&[90.0, 100.0, 110.0])
        .row(&[0.2, 0.21, 0.22])
        .row(&[0.19, 0.2, 0.21])
        .build()
        .unwrap();

    let json = serde_json::to_string_pretty(&surface).unwrap();
    let deserialized: VolSurface = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id(), surface.id());
    assert_eq!(deserialized.expiries(), surface.expiries());
    assert_eq!(deserialized.strikes(), surface.strikes());
    assert_eq!(deserialized.grid_shape(), surface.grid_shape());
}

#[test]
fn base_correlation_curve_serde_roundtrip() {
    let curve = BaseCorrelationCurve::builder("CDX")
        .points([(3.0, 0.25), (7.0, 0.45), (10.0, 0.60)])
        .build()
        .unwrap();

    let json = serde_json::to_string_pretty(&curve).unwrap();
    let deserialized: BaseCorrelationCurve = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id(), curve.id());
    assert_eq!(deserialized.detachment_points(), curve.detachment_points());
    assert_eq!(deserialized.correlations(), curve.correlations());
}

#[test]
fn scalar_time_series_serde_roundtrip() {
    let d1 = Date::from_calendar_date(2024, Month::January, 31).unwrap();
    let d2 = Date::from_calendar_date(2024, Month::February, 29).unwrap();

    let series = ScalarTimeSeries::new("VOL-TS", vec![(d1, 0.2), (d2, 0.25)], None)
        .unwrap()
        .with_interpolation(SeriesInterpolation::Linear);

    let json = serde_json::to_string_pretty(&series).unwrap();
    let deserialized: ScalarTimeSeries = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id(), series.id());
    assert_eq!(deserialized.observations(), series.observations());
}

#[test]
fn inflation_index_serde_roundtrip() {
    let d1 = Date::from_calendar_date(2024, Month::January, 31).unwrap();
    let d2 = Date::from_calendar_date(2024, Month::February, 29).unwrap();

    let index = InflationIndex::new("US-CPI", vec![(d1, 300.0), (d2, 301.5)], Currency::USD)
        .unwrap()
        .with_interpolation(InflationInterpolation::Linear)
        .with_lag(InflationLag::Months(3));

    let json = serde_json::to_string_pretty(&index).unwrap();
    let deserialized: InflationIndex = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id, index.id);
    assert_eq!(deserialized.currency, index.currency);
    assert_eq!(deserialized.interpolation, index.interpolation);
    assert_eq!(deserialized.lag(), index.lag());
}

#[test]
fn market_context_serde_roundtrip() {
    let discount = DiscountCurve::builder("USD-OIS")
        .base_date(test_date())
        .knots([(0.0, 1.0), (5.0, 0.9)])
        .build()
        .unwrap();

    let forward = ForwardCurve::builder("USD-SOFR", 0.25)
        .base_date(test_date())
        .knots([(0.0, 0.03), (5.0, 0.04)])
        .build()
        .unwrap();

    let surface = VolSurface::builder("VOL")
        .expiries(&[1.0, 2.0])
        .strikes(&[90.0, 100.0])
        .row(&[0.2, 0.2])
        .row(&[0.2, 0.2])
        .build()
        .unwrap();

    let ctx = MarketContext::new()
        .insert_discount(discount)
        .insert_forward(forward)
        .insert_surface(surface);

    let json = serde_json::to_string_pretty(&ctx).unwrap();
    let deserialized: MarketContext = serde_json::from_str(&json).unwrap();

    assert!(deserialized.get_discount("USD-OIS").is_ok());
    assert!(deserialized.get_forward("USD-SOFR").is_ok());
    assert!(deserialized.surface("VOL").is_ok());
}

#[test]
fn discount_curve_validates_on_deserialization() {
    // Try to deserialize a curve with non-monotonic discount factors
    let bad_json = r#"{
        "id": "BAD",
        "base": "2025-01-15",
        "day_count": "Act365F",
        "knot_points": [[0.0, 1.0], [1.0, 1.01]],
        "interp_style": "Linear",
        "extrapolation": "FlatForward",
        "require_monotonic": true,
        "allow_non_monotonic": false
    }"#;

    let result: Result<DiscountCurve, _> = serde_json::from_str(bad_json);
    assert!(
        result.is_err(),
        "Should reject non-monotonic discount factors"
    );
}
