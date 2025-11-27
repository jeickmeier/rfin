//! Serialization tests for market data types not covered in test_curve_serde.rs.
//!
//! Note: DiscountCurve, ForwardCurve, HazardCurve, and InflationCurve roundtrip tests
//! are now in test_curve_serde.rs which tests all interpolator styles comprehensively.

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
use time::Month;

fn test_date() -> Date {
    Date::from_calendar_date(2025, Month::January, 15).unwrap()
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
