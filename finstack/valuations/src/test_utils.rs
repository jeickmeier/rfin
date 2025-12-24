//! Shared helpers for unit tests to reduce boilerplate market setup.
use finstack_core::{
    dates::Date,
    market_data::{
        surfaces::vol_surface::VolSurface,
        term_structures::{discount_curve::DiscountCurve, forward_curve::ForwardCurve},
    },
};
use time::Month;

/// Convenience date helper for tests.
pub fn date(year: i32, month: u8, day: u8) -> Date {
    Date::from_calendar_date(year, Month::try_from(month).expect("valid month"), day)
        .expect("valid date")
}

/// Build a flat discount curve with two knots: (0, 1.0) and (1y, exp(-rate)).
pub fn flat_discount(id: &str, as_of: Date, rate: f64) -> DiscountCurve {
    flat_discount_with_tenor(id, as_of, rate, 1.0)
}

/// Build a flat discount curve with a configurable far-tenor knot.
pub fn flat_discount_with_tenor(
    id: &str,
    as_of: Date,
    rate: f64,
    tenor_years: f64,
) -> DiscountCurve {
    DiscountCurve::builder(id)
        .base_date(as_of)
        .knots([(0.0, 1.0), (tenor_years, (-rate * tenor_years).exp())])
        .build()
        .expect("discount curve should build in tests")
}

/// Build a flat forward curve with two knots and a constant rate.
pub fn flat_forward_with_tenor(id: &str, as_of: Date, rate: f64, tenor_years: f64) -> ForwardCurve {
    ForwardCurve::builder(id, tenor_years)
        .base_date(as_of)
        .knots([(0.0, rate), (tenor_years, rate)])
        .build()
        .expect("forward curve should build in tests")
}

/// Build a constant vol surface using provided expiries/strikes grid.
pub fn flat_vol_surface(id: &str, expiries: &[f64], strikes: &[f64], vol: f64) -> VolSurface {
    let mut builder = VolSurface::builder(id).expiries(expiries).strikes(strikes);
    for _ in expiries {
        builder = builder.row(&vec![vol; strikes.len()]);
    }
    builder.build().expect("vol surface should build in tests")
}
