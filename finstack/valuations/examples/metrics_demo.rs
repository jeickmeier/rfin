//! Example demonstrating the new metrics framework.

use finstack_core::prelude::*;
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::metrics::{MetricContext, standard_registry};
use finstack_valuations::pricing::discountable::Discountable;
use finstack_valuations::traits::CashflowProvider;
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use std::sync::Arc;
use time::Month;

fn main() -> finstack_core::Result<()> {
    // Setup market data
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (1.0, 0.98), (2.0, 0.96)])
        .linear_df()
        .build()?;
    let curves = Arc::new(CurveSet::new().with_discount(disc));
    
    // Create a bond instrument
    let bond = Bond {
        id: "US912828YY8".to_string(),
        notional: Money::new(1_000_000.0, Currency::USD),
        coupon: 0.05,  // 5% annual coupon
        freq: finstack_core::dates::Frequency::semi_annual(),
        dc: DayCount::Act365F,
        issue: as_of,
        maturity: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
        disc_id: "USD-OIS",
        quoted_clean: Some(980.0),  // Clean price per 1000 notional
        call_put: None,
        amortization: None,
    };
    
    // Step 1: Compute core value only (fast, no metrics)
    println!("=== Core Valuation Only ===");
    let flows = bond.build_schedule(&curves, as_of)?;
    let disc_curve = curves.discount("USD-OIS")?;
    let base_value = flows.npv(&*disc_curve, disc_curve.base_date(), bond.dc)?;
    println!("Present Value: {:.2} {}", base_value.amount(), base_value.currency());
    
    // Step 2: Compute specific metrics on demand
    println!("\n=== On-Demand Metrics ===");
    let mut context = MetricContext::new(
        Arc::new(bond.clone()) as Arc<dyn std::any::Any + Send + Sync>,
        "Bond".to_string(),
        curves.clone(),
        as_of,
        base_value,
    );
    
    let registry = standard_registry();
    
    // Compute only the metrics we need
    let requested = ["accrued", "ytm"];
    let metrics = registry.compute(&requested, &mut context)?;
    
    for (name, value) in &metrics {
        println!("{}: {:.4}", name, value);
    }
    
    // Step 3: Compute additional metrics (with dependency handling)
    println!("\n=== Additional Metrics (with Dependencies) ===");
    let risk_metrics = ["duration_mac", "duration_mod", "convexity"];
    let additional = registry.compute(&risk_metrics, &mut context)?;
    
    for (name, value) in &additional {
        println!("{}: {:.4}", name, value);
    }
    
    // Notice that YTM was already cached and not recomputed
    println!("\n=== All Computed Metrics ===");
    for (name, value) in &context.computed {
        println!("{}: {:.4}", name, value);
    }
    
    // Step 4: Show which metrics are available
    println!("\n=== Available Bond Metrics ===");
    let bond_metrics = registry.metrics_for_instrument("Bond");
    println!("Bond metrics: {:?}", bond_metrics);
    
    Ok(())
}
