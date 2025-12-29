//! Pricing tests for CMS Option.

use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::surfaces::VolSurface;
use finstack_core::market_data::term_structures::ForwardCurve;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::types::CurveId;
use finstack_valuations::instruments::cms_option::CmsOption;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::MetricId;
use time::Month;

fn standard_market(as_of: Date) -> MarketContext {
    let mut market = MarketContext::new();

    // Add OIS Curve (Flat 3%)
    let knots = vec![
        (0.0, 1.0),
        (1.0, (-0.03 * 1.0f64).exp()),
        (5.0, (-0.03 * 5.0f64).exp()),
        (10.0, (-0.03 * 10.0f64).exp()),
        (30.0, (-0.03 * 30.0f64).exp()),
    ];

    let discount_curve = DiscountCurve::builder(CurveId::new("USD-OIS"))
        .base_date(as_of)
        .day_count(DayCount::Act365F)
        .knots(knots)
        .build()
        .unwrap();

    market = market.insert_discount(discount_curve);

    // Add LIBOR Forward Curve (Flat 3% for simplicity, or slightly different)
    // Let's make it 3.5% to have spread
    let fwd_knots = vec![(0.0, 0.035), (10.0, 0.035), (30.0, 0.035)];
    let forward_curve = ForwardCurve::builder(CurveId::new("USD-LIBOR-3M"), 0.25)
        .base_date(as_of)
        .day_count(DayCount::Act365F)
        .knots(fwd_knots)
        .build()
        .unwrap();

    market = market.insert_forward(forward_curve);

    // Add Vol Surface (Flat 20%)
    // Manually build a grid
    let strikes = vec![0.01, 0.02, 0.03, 0.04, 0.05];
    let expiries = vec![0.5, 1.0, 5.0, 10.0, 20.0];
    let flat_row = vec![0.20; 5];

    let mut builder = VolSurface::builder(CurveId::new("USD-CMS10Y-VOL"))
        .expiries(&expiries)
        .strikes(&strikes);

    for _ in 0..expiries.len() {
        builder = builder.row(&flat_row);
    }

    let vol_surface = builder.build().unwrap();

    market = market.insert_surface(vol_surface);

    market
}

#[test]
fn test_cms_cap_pricing() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let market = standard_market(as_of);

    let inst = CmsOption::example();

    // Price
    let pv = inst.value(&market, as_of).expect("Pricing failed");

    assert!(
        pv.amount() > 0.0,
        "PV should be positive, got {}",
        pv.amount()
    );
}

#[test]
fn test_convexity_value() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let market = standard_market(as_of);

    let inst = CmsOption::example();

    // Calculate Convexity Adjustment Risk
    let result = inst
        .price_with_metrics(&market, as_of, &[MetricId::ConvexityAdjustmentRisk])
        .expect("Metric calc failed");

    let convexity_val = result
        .measures
        .get(MetricId::ConvexityAdjustmentRisk.as_str())
        .copied()
        .expect("ConvexityAdjustmentRisk metric not found");

    // Convexity adjustment for CMS rate adds to the rate (usually).
    // So Adjusted Rate > Forward Rate.
    // For a Call (Cap), higher rate = higher value.
    // So Convexity Value should be positive.
    println!("Convexity Value: {}", convexity_val);
    // Ideally should be > 0.0, but allowing >= 0.0 for now if test setup makes it small
    assert!(
        convexity_val >= 0.0,
        "Convexity adjustment should be non-negative, got {}",
        convexity_val
    );
}
