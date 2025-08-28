#![cfg(test)]

use finstack_core::dates::Date;
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
use finstack_core::prelude::*;
use finstack_valuations::instruments::irs;
use finstack_valuations::metrics::{MetricContext, standard_registry};
use std::sync::Arc;
use time::Month;

#[test]
fn debug_metric_registry() {
    let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .knots([(0.0, 1.0), (10.0, 1.0)])
        .linear_df()
        .build()
        .unwrap();
    let fwd_rate = 0.05;
    let fwd = ForwardCurve::builder("USD-SOFR3M", 0.25)
        .base_date(base)
        .knots([(0.0, fwd_rate), (10.0, fwd_rate)])
        .linear_df()
        .build()
        .unwrap();
    let curves = Arc::new(CurveSet::new().with_discount(disc).with_forecast(fwd));

    let irs = irs::InterestRateSwap {
        id: "IRS-TEST".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        side: irs::PayReceive::PayFixed,
        fixed: irs::FixedLegSpec {
            disc_id: "USD-OIS",
            rate: fwd_rate,
            freq: finstack_core::dates::Frequency::quarterly(),
            dc: DayCount::Act365F,
            bdc: finstack_core::dates::BusinessDayConvention::Following,
            calendar_id: None,
            stub: finstack_core::dates::StubKind::None,
            start: base,
            end: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
        },
        float: irs::FloatLegSpec {
            disc_id: "USD-OIS",
            fwd_id: "USD-SOFR3M",
            spread_bp: 0.0,
            freq: finstack_core::dates::Frequency::quarterly(),
            dc: DayCount::Act365F,
            bdc: finstack_core::dates::BusinessDayConvention::Following,
            calendar_id: None,
            stub: finstack_core::dates::StubKind::None,
            start: base,
            end: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
        },
    };
    
    // Get base value
    use finstack_valuations::traits::Priceable;
    let base_value = irs.value(&curves, base).unwrap();
    
    // Create metric context
    let mut context = MetricContext::new(
        Arc::new(irs.clone()) as Arc<dyn std::any::Any + Send + Sync>,
        "IRS".to_string(),
        curves.clone(),
        base,
        base_value,
    );
    
    // Get registry
    let registry = standard_registry();
    
    // Check available metrics for IRS
    let available = registry.metrics_for_instrument("IRS");
    println!("Available metrics for IRS: {:?}", available);
    
    // Try to compute par_rate explicitly
    let requested = ["annuity", "par_rate", "dv01", "pv_fixed", "pv_float"];
    match registry.compute(&requested, &mut context) {
        Ok(measures) => {
            println!("Successfully computed metrics:");
            for (key, value) in &measures {
                println!("  {}: {}", key, value);
            }
            
            // Check if par_rate exists
            assert!(measures.contains_key("par_rate"), "par_rate metric not found! Available metrics: {:?}", measures.keys().collect::<Vec<_>>());
        }
        Err(e) => {
            println!("Error computing metrics: {:?}", e);
            panic!("Failed to compute metrics!");
        }
    }
}
