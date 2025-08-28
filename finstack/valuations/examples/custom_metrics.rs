//! Example demonstrating how to create custom metrics and extend the framework.

use finstack_core::prelude::*;
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::metrics::{MetricCalculator, MetricContext};
use finstack_valuations::pricing::discountable::Discountable;
use finstack_valuations::traits::CashflowProvider;
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use std::sync::Arc;
use time::Month;

// Custom metric: Calculate the ratio of accrued interest to coupon payment
struct AccruedRatioCalculator;

impl MetricCalculator for AccruedRatioCalculator {
    fn id(&self) -> &str {
        "accrued_ratio"
    }
    
    fn description(&self) -> &str {
        "Ratio of accrued interest to full coupon payment"
    }
    
    fn is_applicable(&self, instrument_type: &str) -> bool {
        instrument_type == "Bond"
    }
    
    fn dependencies(&self) -> Vec<&str> {
        vec!["accrued"]
    }
    
    fn calculate(&self, context: &MetricContext) -> finstack_core::Result<finstack_core::F> {
        let bond = context.instrument_as::<Bond>()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        
        let accrued = context.computed.get("accrued").copied()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))?;
        
        // Calculate full period coupon
        let periods_per_year = match bond.freq {
            finstack_core::dates::Frequency::Months(m) => 12.0 / m as f64,
            finstack_core::dates::Frequency::Days(d) => 365.25 / d as f64,
            _ => 1.0,  // Default to annual for any future frequency types
        };
        let period_yf = 1.0 / periods_per_year;
        let full_coupon = bond.notional.amount() * bond.coupon * period_yf;
        
        Ok(accrued / full_coupon)
    }
}

// Custom metric: Calculate spread to treasury (mock implementation)
struct SpreadCalculator {
    treasury_rate: f64,
}

impl MetricCalculator for SpreadCalculator {
    fn id(&self) -> &str {
        "spread_to_treasury"
    }
    
    fn description(&self) -> &str {
        "Spread to treasury benchmark in basis points"
    }
    
    fn is_applicable(&self, instrument_type: &str) -> bool {
        instrument_type == "Bond"
    }
    
    fn dependencies(&self) -> Vec<&str> {
        vec!["ytm"]
    }
    
    fn calculate(&self, context: &MetricContext) -> finstack_core::Result<finstack_core::F> {
        let ytm = context.computed.get("ytm").copied()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))?;
        
        // Convert spread to basis points
        Ok((ytm - self.treasury_rate) * 10000.0)
    }
}

// Custom metric: Calculate time to maturity in years
struct TimeToMaturityCalculator;

impl MetricCalculator for TimeToMaturityCalculator {
    fn id(&self) -> &str {
        "time_to_maturity"
    }
    
    fn description(&self) -> &str {
        "Time to maturity in years"
    }
    
    fn is_applicable(&self, instrument_type: &str) -> bool {
        instrument_type == "Bond"
    }
    
    fn calculate(&self, context: &MetricContext) -> finstack_core::Result<finstack_core::F> {
        let bond = context.instrument_as::<Bond>()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        
        let years = DiscountCurve::year_fraction(context.as_of, bond.maturity, bond.dc);
        Ok(years)
    }
}

fn main() -> finstack_core::Result<()> {
    // Setup market data
    let as_of = Date::from_calendar_date(2025, Month::March, 15).unwrap();
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (1.0, 0.98), (2.0, 0.96)])
        .linear_df()
        .build()?;
    let curves = Arc::new(CurveSet::new().with_discount(disc));
    
    // Create a bond instrument
    let bond = Bond {
        id: "CUSTOM-BOND".to_string(),
        notional: Money::new(1_000_000.0, Currency::USD),
        coupon: 0.045,  // 4.5% annual coupon
        freq: finstack_core::dates::Frequency::quarterly(),
        dc: DayCount::Act365F,
        issue: Date::from_calendar_date(2025, Month::January, 1).unwrap(),
        maturity: Date::from_calendar_date(2027, Month::January, 1).unwrap(),
        disc_id: "USD-OIS",
        quoted_clean: Some(985.0),  // Clean price per 1000 notional
        call_put: None,
        amortization: None,
    };
    
    // Compute base value
    let flows = bond.build_schedule(&curves, as_of)?;
    let disc_curve = curves.discount("USD-OIS")?;
    let base_value = flows.npv(&*disc_curve, disc_curve.base_date(), bond.dc)?;
    
    // Create registry and register custom metrics
    let mut registry = finstack_valuations::metrics::standard_registry();
    
    // Add our custom metrics
    registry
        .register(Arc::new(AccruedRatioCalculator))
        .register(Arc::new(SpreadCalculator { treasury_rate: 0.04 }))
        .register(Arc::new(TimeToMaturityCalculator));
    
    println!("=== Custom Metrics Demo ===\n");
    
    // Create context
    let mut context = MetricContext::new(
        Arc::new(bond.clone()) as Arc<dyn std::any::Any + Send + Sync>,
        "Bond".to_string(),
        curves.clone(),
        as_of,
        base_value,
    );
    
    // Compute standard + custom metrics
    let all_metrics = [
        "accrued",
        "ytm",
        "duration_mod",
        "accrued_ratio",  // custom
        "spread_to_treasury",  // custom
        "time_to_maturity",  // custom
    ];
    
    let results = registry.compute(&all_metrics, &mut context)?;
    
    println!("Bond: {}", bond.id);
    println!("Present Value: {:.2} {}\n", base_value.amount(), base_value.currency());
    
    println!("Standard Metrics:");
    println!("  Accrued Interest: {:.2}", results["accrued"]);
    println!("  Yield to Maturity: {:.4}%", results["ytm"] * 100.0);
    println!("  Modified Duration: {:.4}", results["duration_mod"]);
    
    println!("\nCustom Metrics:");
    println!("  Accrued Ratio: {:.2}%", results["accrued_ratio"] * 100.0);
    println!("  Spread to Treasury: {:.0} bps", results["spread_to_treasury"]);
    println!("  Time to Maturity: {:.2} years", results["time_to_maturity"]);
    
    println!("\n=== Dependency Resolution ===");
    println!("The framework automatically resolved dependencies:");
    println!("  - 'accrued_ratio' required 'accrued' to be computed first");
    println!("  - 'spread_to_treasury' required 'ytm' to be computed first");
    println!("  - 'duration_mod' also required 'ytm'");
    println!("  - Each metric was computed exactly once and cached");
    
    Ok(())
}
