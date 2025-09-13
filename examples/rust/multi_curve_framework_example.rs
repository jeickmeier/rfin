//! Example demonstrating the post-2008 multi-curve framework with strict separation
//! between discount and forward curves, and support for basis swaps.

#![allow(clippy::useless_vec)] // vec! is more readable for examples

use finstack_core::{
    dates::{BusinessDayConvention, Date, DayCount, Frequency},
    market_data::{
        context::MarketContext,
        term_structures::{discount_curve::DiscountCurve, forward_curve::ForwardCurve},
    },
    money::Money,
    prelude::*,
};
use finstack_valuations::{
    calibration::{
        bootstrap::DiscountCurveCalibrator, MarketQuote, MultiCurveConfig, RatesQuote,
        SimpleCalibration,
    },
    instruments::fixed_income::{BasisSwap, BasisSwapLeg},
};
use time::Month;

// Helper function to create dates
fn date(year: i32, month: u8, day: u8) -> Date {
    Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap()
}

fn main() -> Result<()> {
    println!("=== Multi-Curve Framework Example ===\n");

    let base_date = date(2024, 1, 2);

    // Example 1: Multi-curve mode (post-2008 standard)
    println!("1. Multi-Curve Mode (Post-2008 Standard)");
    println!("-----------------------------------------");
    demo_multi_curve_mode(base_date)?;

    // Example 2: Single-curve mode (pre-2008 legacy)
    println!("\n2. Single-Curve Mode (Pre-2008 Legacy/Fallback)");
    println!("------------------------------------------------");
    demo_single_curve_mode(base_date)?;

    // Example 3: Basis swap pricing
    println!("\n3. Basis Swap Pricing");
    println!("----------------------");
    demo_basis_swap(base_date)?;

    Ok(())
}

fn demo_multi_curve_mode(base_date: Date) -> Result<()> {
    // Create multi-curve configuration (default)
    let multi_curve_config = MultiCurveConfig::multi_curve();

    println!("Configuration:");
    println!("  Mode: {:?}", multi_curve_config.mode);
    println!("  Calibrate basis: {}", multi_curve_config.calibrate_basis);
    println!(
        "  Enforce separation: {}",
        multi_curve_config.enforce_separation
    );

    // Set up calibration with multi-curve mode
    let _calibration = SimpleCalibration::new(base_date, Currency::USD)
        .with_multi_curve_config(multi_curve_config);

    // Create OIS quotes for discount curve
    let _ois_quotes = vec![
        MarketQuote::Rates(RatesQuote::Deposit {
            maturity: date(2024, 1, 9),
            rate: 0.0520,
            day_count: DayCount::Act360,
        }),
        MarketQuote::Rates(RatesQuote::Swap {
            maturity: date(2025, 1, 2),
            rate: 0.0515,
            fixed_freq: Frequency::annual(),
            float_freq: Frequency::annual(),
            fixed_dc: DayCount::Act360,
            float_dc: DayCount::Act360,
            index: "USD-OIS".to_string(),
        }),
        MarketQuote::Rates(RatesQuote::Swap {
            maturity: date(2026, 1, 2),
            rate: 0.0510,
            fixed_freq: Frequency::annual(),
            float_freq: Frequency::annual(),
            fixed_dc: DayCount::Act360,
            float_dc: DayCount::Act360,
            index: "USD-OIS".to_string(),
        }),
    ];

    // Create SOFR 3M quotes for forward curve
    let _sofr_3m_quotes = vec![
        MarketQuote::Rates(RatesQuote::Swap {
            maturity: date(2025, 1, 2),
            rate: 0.0525, // Higher than OIS due to tenor basis
            fixed_freq: Frequency::annual(),
            float_freq: Frequency::quarterly(),
            fixed_dc: DayCount::Act360,
            float_dc: DayCount::Act360,
            index: "USD-SOFR-3M".to_string(),
        }),
        MarketQuote::Rates(RatesQuote::Swap {
            maturity: date(2026, 1, 2),
            rate: 0.0520,
            fixed_freq: Frequency::annual(),
            float_freq: Frequency::quarterly(),
            fixed_dc: DayCount::Act360,
            float_dc: DayCount::Act360,
            index: "USD-SOFR-3M".to_string(),
        }),
    ];

    // In multi-curve mode:
    // - OIS curve is used ONLY for discounting
    // - Forward curves are calibrated independently
    // - No forward curve is derived from the discount curve

    println!("\nKey principle:");
    println!("  ✓ Discount curve (OIS) for present value only");
    println!("  ✓ Forward curves calibrated independently");
    println!("  ✓ Basis spreads capture tenor differences");

    Ok(())
}

fn demo_single_curve_mode(base_date: Date) -> Result<()> {
    // Create single-curve configuration (legacy/fallback)
    let multi_curve_config = MultiCurveConfig::single_curve(0.25); // 3M tenor

    println!("Configuration:");
    println!("  Mode: {:?}", multi_curve_config.mode);
    println!(
        "  Single curve tenor: {} years",
        multi_curve_config.single_curve_tenor
    );
    println!(
        "  Derive forward from discount: {}",
        multi_curve_config.derive_forward_from_discount()
    );

    // In single-curve mode:
    // - The discount curve is also used as the forward curve
    // - This is the pre-2008 methodology
    // - Used only for special cases or simplified modeling

    // Create a discount curve calibrator with single-curve mode
    let _calibrator = DiscountCurveCalibrator::new("USD-LIBOR", base_date, Currency::USD)
        .with_multi_curve_config(multi_curve_config);

    println!("\nKey principle:");
    println!("  ⚠️  Discount curve = Forward curve");
    println!("  ⚠️  No tenor basis spreads");
    println!("  ⚠️  Simplified modeling only");

    Ok(())
}

fn demo_basis_swap(base_date: Date) -> Result<()> {
    // Create market context with discount and forward curves
    let discount_curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(vec![(0.0, 1.0), (1.0, 0.95), (2.0, 0.90)])
        .build()?;

    let forward_3m = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(base_date)
        .knots(vec![(0.0, 0.052), (1.0, 0.051), (2.0, 0.050)])
        .build()?;

    let forward_6m = ForwardCurve::builder("USD-SOFR-6M", 0.5)
        .base_date(base_date)
        .knots(vec![
            (0.0, 0.0523), // 3bp spread over 3M
            (1.0, 0.0513),
            (2.0, 0.0503),
        ])
        .build()?;

    let context = MarketContext::new()
        .insert_discount(discount_curve)
        .insert_forward(forward_3m)
        .insert_forward(forward_6m);

    // Create a basis swap: 3M SOFR vs 6M SOFR + spread
    let primary_leg = BasisSwapLeg {
        forward_curve_id: "USD-SOFR-3M".into(),
        frequency: Frequency::quarterly(),
        day_count: DayCount::Act360,
        bdc: BusinessDayConvention::ModifiedFollowing,
        spread: 0.0003, // 3bp spread
    };

    let reference_leg = BasisSwapLeg {
        forward_curve_id: "USD-SOFR-6M".into(),
        frequency: Frequency::semi_annual(),
        day_count: DayCount::Act360,
        bdc: BusinessDayConvention::ModifiedFollowing,
        spread: 0.0,
    };

    let basis_swap = BasisSwap::new(
        "3Mvs6M_BASIS",
        Money::new(10_000_000.0, Currency::USD),
        date(2024, 1, 5),
        date(2026, 1, 5),
        primary_leg,
        reference_leg,
        "USD-OIS",
    );

    // Price the basis swap
    use finstack_valuations::instruments::traits::Priceable;
    let pv = basis_swap.value(&context, base_date)?;

    println!("Basis Swap Details:");
    println!("  Notional: {}", basis_swap.notional);
    println!("  Primary leg: 3M SOFR + 3bp");
    println!("  Reference leg: 6M SOFR flat");
    println!("  Maturity: 2 years");
    println!("  Present Value: {:.2}", pv);

    println!("\nKey insight:");
    println!("  Basis swaps capture the spread between different tenor forward curves");
    println!("  Essential for multi-curve calibration and risk management");

    Ok(())
}
