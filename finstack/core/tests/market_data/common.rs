use std::sync::Arc;

use finstack_core::dates::Date;
use finstack_core::market_data::surfaces::vol_surface::VolSurface;
use finstack_core::market_data::term_structures::base_correlation::BaseCorrelationCurve;
use finstack_core::market_data::term_structures::discount_curve::{DiscountCurve, DiscountCurveBuilder};
use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
use finstack_core::market_data::term_structures::inflation::InflationCurve;
use finstack_core::math::interp::InterpStyle;
use time::Month;

pub(crate) fn sample_base_date() -> Date {
    Date::from_calendar_date(2024, Month::January, 1).unwrap()
}

pub(crate) fn sample_discount_curve(id: &str) -> DiscountCurve {
    DiscountCurve::builder(id)
        .base_date(sample_base_date())
        .knots([(0.0, 1.0), (1.0, 0.98), (2.0, 0.96)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap()
}

pub(crate) fn sample_forward_curve(id: &str) -> ForwardCurve {
    ForwardCurve::builder(id, 0.25)
        .base_date(sample_base_date())
        .knots([(0.0, 0.02), (1.0, 0.021), (2.0, 0.022)])
        .build()
        .unwrap()
}

pub(crate) fn sample_hazard_curve(id: &str) -> HazardCurve {
    HazardCurve::builder(id)
        .base_date(sample_base_date())
        .knots([(0.0, 0.01), (3.0, 0.015), (5.0, 0.02)])
        .build()
        .unwrap()
}

pub(crate) fn sample_inflation_curve(id: &str) -> InflationCurve {
    InflationCurve::builder(id)
        .base_cpi(100.0)
        .knots([(0.0, 100.0), (1.0, 102.0), (2.0, 104.0)])
        .build()
        .unwrap()
}

pub(crate) fn sample_base_correlation_curve(id: &str) -> BaseCorrelationCurve {
    BaseCorrelationCurve::builder(id)
        .points([(3.0, 0.25), (7.0, 0.4), (10.0, 0.55)])
        .build()
        .unwrap()
}

pub(crate) fn sample_vol_surface() -> Arc<VolSurface> {
    Arc::new(
        VolSurface::builder("EQ-VOL")
            .expiries(&[0.25, 0.5, 1.0])
            .strikes(&[0.9, 1.0, 1.1])
            .row(&[0.25, 0.24, 0.23])
            .row(&[0.22, 0.21, 0.2])
            .row(&[0.2, 0.19, 0.18])
            .build()
            .unwrap(),
    )
}

#[allow(dead_code)]
pub(crate) fn discount_curve_with_custom_builder<F>(id: &str, mut build: F) -> DiscountCurve
where
    F: FnMut(DiscountCurveBuilder) -> DiscountCurveBuilder,
{
    let builder = DiscountCurve::builder(id).base_date(sample_base_date());
    build(builder)
        .knots([(0.0, 1.0), (1.0, 0.97), (5.0, 0.9)])
        .set_interp(InterpStyle::MonotoneConvex)
        .build()
        .unwrap()
}
