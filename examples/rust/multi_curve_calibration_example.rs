//! Multi-curve yield curve calibration example.
//!
//! Demonstrates post-2008 multi-curve framework with:
//! - OIS discount curve for discounting
//! - Separate forward curves per tenor (1M, 3M, 6M)

use finstack::calibration::{
    bootstrap::{DiscountCurveCalibrator, ForwardCurveCalibrator},
    quote::{FutureSpecs, RatesQuote},
    CalibrationConfig, Calibrator,
};
use finstack::currency::Currency;
use finstack::dates::{Date, DayCount, Frequency};
use finstack::market_data::{
    context::MarketContext,
    interp::types::InterpStyle,
    traits::{Discount, Forward, TermStructure},
};
use finstack::prelude::*;
use time::Month;

fn main() -> finstack::Result<()> {
    let base_date = Date::from_calendar_date(2025, Month::January, 1)?;
    
    // Step 1: Create market context
    let mut context = MarketContext::new();
    
    // Step 2: Calibrate OIS discount curve
    println!("Calibrating OIS discount curve...");
    let ois_quotes = vec![
        // Overnight deposit
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(1),
            rate: 0.0450,
            day_count: DayCount::Act365F,
        },
        // OIS swaps
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(30),
            rate: 0.0452,
            fixed_freq: Frequency::annual(),
            float_freq: Frequency::daily(),
            fixed_dc: DayCount::Act365F,
            float_dc: DayCount::Act365F,
            index: "USD-OIS".to_string(),
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(90),
            rate: 0.0455,
            fixed_freq: Frequency::annual(),
            float_freq: Frequency::daily(),
            fixed_dc: DayCount::Act365F,
            float_dc: DayCount::Act365F,
            index: "USD-OIS".to_string(),
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(180),
            rate: 0.0458,
            fixed_freq: Frequency::annual(),
            float_freq: Frequency::daily(),
            fixed_dc: DayCount::Act365F,
            float_dc: DayCount::Act365F,
            index: "USD-OIS".to_string(),
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365),
            rate: 0.0462,
            fixed_freq: Frequency::annual(),
            float_freq: Frequency::daily(),
            fixed_dc: DayCount::Act365F,
            float_dc: DayCount::Act365F,
            index: "USD-OIS".to_string(),
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365 * 2),
            rate: 0.0468,
            fixed_freq: Frequency::annual(),
            float_freq: Frequency::daily(),
            fixed_dc: DayCount::Act365F,
            float_dc: DayCount::Act365F,
            index: "USD-OIS".to_string(),
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365 * 5),
            rate: 0.0475,
            fixed_freq: Frequency::annual(),
            float_freq: Frequency::daily(),
            fixed_dc: DayCount::Act365F,
            float_dc: DayCount::Act365F,
            index: "USD-OIS".to_string(),
        },
    ];
    
    let ois_calibrator = DiscountCurveCalibrator::new(
        "USD-OIS-DISC",
        base_date,
        Currency::USD,
    )
    .with_solve_interp(InterpStyle::MonotoneConvex)
    .with_config(CalibrationConfig {
        tolerance: 1e-10,
        max_iterations: 100,
        ..Default::default()
    });
    
    let (ois_curve, ois_report) = ois_calibrator.calibrate(&ois_quotes, &context)?;
    
    println!("OIS curve calibrated:");
    println!("  - Success: {}", ois_report.success);
    println!("  - Iterations: {}", ois_report.iterations);
    println!("  - Max residual: {:.2e}", ois_report.max_residual);
    println!("  - RMSE: {:.2e}", ois_report.rmse);
    
    // Update context with OIS curve and set collateral mapping
    context = context
        .insert_discount(ois_curve)
        .map_collateral("USD-CSA", "USD-OIS-DISC".into());
    
    // Step 3: Calibrate 3M forward curve
    println!("\nCalibrating 3M SOFR forward curve...");
    let forward_3m_quotes = vec![
        // FRAs
        RatesQuote::FRA {
            start: base_date + time::Duration::days(30),
            end: base_date + time::Duration::days(120),
            rate: 0.0463,
            day_count: DayCount::Act360,
        },
        RatesQuote::FRA {
            start: base_date + time::Duration::days(90),
            end: base_date + time::Duration::days(180),
            rate: 0.0465,
            day_count: DayCount::Act360,
        },
        RatesQuote::FRA {
            start: base_date + time::Duration::days(180),
            end: base_date + time::Duration::days(270),
            rate: 0.0468,
            day_count: DayCount::Act360,
        },
        // Futures
        RatesQuote::Future {
            expiry: base_date + time::Duration::days(90),
            price: 95.30, // Implies 4.70% rate
            specs: FutureSpecs {
                multiplier: 2500.0,
                face_value: 1_000_000.0,
                delivery_months: 3,
                day_count: DayCount::Act360,
                convexity_adjustment: Some(0.0002), // 2bp convexity adjustment
            },
        },
        // Vanilla IRS vs 3M SOFR
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365),
            rate: 0.0470,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::quarterly(),
            fixed_dc: DayCount::Thirty360,
            float_dc: DayCount::Act360,
            index: "USD-SOFR-3M".to_string(),
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365 * 2),
            rate: 0.0475,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::quarterly(),
            fixed_dc: DayCount::Thirty360,
            float_dc: DayCount::Act360,
            index: "USD-SOFR-3M".to_string(),
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365 * 5),
            rate: 0.0482,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::quarterly(),
            fixed_dc: DayCount::Thirty360,
            float_dc: DayCount::Act360,
            index: "USD-SOFR-3M".to_string(),
        },
    ];
    
    let forward_3m_calibrator = ForwardCurveCalibrator::new(
        "USD-SOFR-3M-FWD",
        0.25, // 3 months = 0.25 years
        base_date,
        Currency::USD,
        "USD-OIS-DISC",
    )
    .with_solve_interp(InterpStyle::Linear)
    .with_config(CalibrationConfig {
        tolerance: 1e-10,
        max_iterations: 100,
        ..Default::default()
    });
    
    let (forward_3m_curve, forward_3m_report) = 
        forward_3m_calibrator.calibrate(&forward_3m_quotes, &context)?;
    
    println!("3M forward curve calibrated:");
    println!("  - Success: {}", forward_3m_report.success);
    println!("  - Iterations: {}", forward_3m_report.iterations);
    println!("  - Max residual: {:.2e}", forward_3m_report.max_residual);
    println!("  - RMSE: {:.2e}", forward_3m_report.rmse);
    
    context = context.insert_forward(forward_3m_curve);
    
    // Step 4: Display some curve values
    println!("\n=== Curve Values ===");
    
    let disc_curve = context.disc("USD-OIS-DISC")?;
    let fwd_3m_curve = context.fwd("USD-SOFR-3M-FWD")?;
    
    println!("\nOIS Discount Factors:");
    for t in &[0.25, 0.5, 1.0, 2.0, 5.0] {
        println!("  t = {:.2} years: DF = {:.6}", t, disc_curve.df(*t));
    }
    
    println!("\n3M Forward Rates:");
    for t in &[0.0, 0.25, 0.5, 1.0, 2.0, 5.0] {
        println!("  t = {:.2} years: Fwd = {:.2}%", t, fwd_3m_curve.rate(*t) * 100.0);
    }
    
    // Step 5: Demonstrate basis spread (simplified)
    println!("\n=== Basis Spreads ===");
    println!("In a full implementation, we would calibrate 1M and 6M curves");
    println!("and use basis swaps to ensure consistency between tenors.");
    
    // Example basis swap quote (not processed in this simple example)
    let _basis_swap = RatesQuote::BasisSwap {
        maturity: base_date + time::Duration::days(365 * 2),
        primary_index: "USD-SOFR-3M".to_string(),
        reference_index: "USD-SOFR-6M".to_string(),
        spread_bp: 2.5, // 3M pays 6M + 2.5bp
        primary_freq: Frequency::quarterly(),
        reference_freq: Frequency::semi_annual(),
        primary_dc: DayCount::Act360,
        reference_dc: DayCount::Act360,
        currency: Currency::USD,
    };
    
    println!("\nMulti-curve calibration complete!");
    println!("Market context contains:");
    println!("  - Discount curve: USD-OIS-DISC");
    println!("  - Forward curve: USD-SOFR-3M-FWD");
    println!("  - Collateral mapping: USD-CSA -> USD-OIS-DISC");
    
    Ok(())
}
