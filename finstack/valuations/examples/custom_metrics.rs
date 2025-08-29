//! Example demonstrating how to create custom metrics and extend the framework.

use finstack_core::prelude::*;
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_valuations::pricing::discountable::Discountable;
use finstack_valuations::traits::CashflowProvider;
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use std::sync::Arc;
use time::Month;

// Custom metric: Calculate the ratio of accrued interest to coupon payment
struct AccruedRatioCalculator;

impl MetricCalculator for AccruedRatioCalculator {

    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Accrued]
    }
    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<finstack_core::F> {
        let bond = match &*context.instrument {
            finstack_valuations::instruments::Instrument::Bond(bond) => bond,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
        let accrued = context.computed.get(&MetricId::Accrued).copied()
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

    
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Ytm]
    }
    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<finstack_core::F> {
        let ytm = context.computed.get(&MetricId::Ytm).copied()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))?;
        
        // Convert spread to basis points
        Ok((ytm - self.treasury_rate) * 10000.0)
    }
}

// Custom metric: Calculate time to maturity in years
struct TimeToMaturityCalculator;

impl MetricCalculator for TimeToMaturityCalculator {

    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<finstack_core::F> {
        let bond = match &*context.instrument {
            finstack_valuations::instruments::Instrument::Bond(bond) => bond,
            _ => return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid)),
        };
        
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
        custom_cashflows: None,
        attributes: finstack_valuations::traits::Attributes::new(),
    };
    
    // Compute base value
    let flows = bond.build_schedule(&curves, as_of)?;
    let disc_curve = curves.discount("USD-OIS")?;
    let base_value = flows.npv(&*disc_curve, disc_curve.base_date(), bond.dc)?;
    
    // Create registry and register custom metrics
    let mut registry = finstack_valuations::metrics::standard_registry();
    
    // Add our custom metrics
    registry
        .register_metric(MetricId::custom("accrued_ratio"), Arc::new(AccruedRatioCalculator), &["Bond"])
        .register_metric(MetricId::custom("spread_to_treasury"), Arc::new(SpreadCalculator { treasury_rate: 0.04 }), &["Bond"])
        .register_metric(MetricId::custom("time_to_maturity"), Arc::new(TimeToMaturityCalculator), &["Bond"]);
    
    println!("=== Custom Metrics Demo ===\n");
    
    // Create context
    let mut context = MetricContext::new(
        Arc::new(finstack_valuations::instruments::Instrument::Bond(bond.clone())),
        curves.clone(),
        as_of,
        base_value,
    );
    
    // Compute standard + custom metrics
    let all_metrics = [
        MetricId::Accrued,
        MetricId::Ytm,
        MetricId::DurationMod,
        MetricId::custom("accrued_ratio"),  // custom
        MetricId::custom("spread_to_treasury"),  // custom
        MetricId::custom("time_to_maturity"),  // custom
    ];
    
    let results = registry.compute(&all_metrics, &mut context)?;
    
    println!("Bond: {}", bond.id);
    println!("Present Value: {:.2} {}\n", base_value.amount(), base_value.currency());
    
    println!("Standard Metrics:");
    println!("  Accrued Interest: {:.2}", results.get(&MetricId::Accrued).unwrap());
    println!("  Yield to Maturity: {:.4}%", results.get(&MetricId::Ytm).unwrap() * 100.0);
    println!("  Modified Duration: {:.4}", results.get(&MetricId::DurationMod).unwrap());
    
    println!("\nCustom Metrics:");
    println!("  Accrued Ratio: {:.2}%", results.get(&MetricId::custom("accrued_ratio")).unwrap() * 100.0);
    println!("  Spread to Treasury: {:.0} bps", results.get(&MetricId::custom("spread_to_treasury")).unwrap());
    println!("  Time to Maturity: {:.2} years", results.get(&MetricId::custom("time_to_maturity")).unwrap());
    
    println!("\n=== Dependency Resolution ===");
    println!("The framework automatically resolved dependencies:");
    println!("  - 'accrued_ratio' required 'accrued' to be computed first");
    println!("  - 'spread_to_treasury' required 'ytm' to be computed first");
    println!("  - 'duration_mod' also required 'ytm'");
    println!("  - Each metric was computed exactly once and cached");
    
    Ok(())
}
