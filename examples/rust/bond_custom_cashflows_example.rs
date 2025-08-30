//! Example demonstrating Bond instruments with custom cashflow schedules.
//!
//! This example shows how to create bonds with complex cashflow patterns
//! using the cashflow builder and pass them to bond instruments for pricing.

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::money::Money;

use finstack_valuations::cashflow::amortization_notional::AmortizationSpec;
use finstack_valuations::cashflow::builder::{cf, CouponType, FixedCouponSpec, ScheduleParams};
use finstack_valuations::instruments::fixed_income::bond::Bond;
use finstack_valuations::traits::{CashflowProvider, Priceable};

use time::Month;

fn example_stepup_bond() -> finstack_core::Result<()> {
    println!("\n=== Step-Up Bond with Custom Cashflows ===");

    // Create dates
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2028, Month::January, 15).unwrap();
    let step1 = Date::from_calendar_date(2026, Month::January, 15).unwrap();
    let step2 = Date::from_calendar_date(2027, Month::January, 15).unwrap();

    // Build custom cashflow schedule with step-up rates
    let schedule_params = ScheduleParams {
        freq: Frequency::semi_annual(),
        dc: DayCount::Act365F,
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };

    let custom_schedule = cf()
        .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
        .fixed_stepup(
            &[(step1, 0.03), (step2, 0.04), (maturity, 0.05)],
            schedule_params,
            CouponType::Cash,
        )
        .build()?;

    // Create bond from custom cashflows
    let bond = Bond::from_cashflows(
        "STEPUP_BOND_2028",
        custom_schedule.clone(),
        "USD-OIS",
        Some(98.5),
    )?;

    // Create market data
    let disc_curve = DiscountCurve::builder("USD-OIS")
        .base_date(issue)
        .knots([(0.0, 1.0), (1.0, 0.97), (2.0, 0.94), (3.0, 0.91)])
        .linear_df()
        .build()?;

    let curves = CurveSet::new().with_discount(disc_curve);

    // Price the bond
    let result = bond.price(&curves, issue)?;
    println!("Step-up bond PV: {}", result.value);
    if let Some(clean_price) = result.measures.get("clean_price") {
        println!("Clean price: {:.2}", clean_price);
    }
    if let Some(ytm) = result.measures.get("ytm") {
        println!("YTM: {:.4}%", ytm * 100.0);
    }

    // Show cashflow details
    let flows = bond.build_schedule(&curves, issue)?;
    println!("\nTotal cashflows: {}", flows.len());
    println!("First 5 flows:");
    for (i, (date, amount)) in flows.iter().enumerate().take(5) {
        println!("  Flow {}: {} -> {}", i + 1, date, amount);
    }
    if flows.len() > 5 {
        println!("  ... and {} more flows", flows.len() - 5);
    }

    Ok(())
}

fn example_pik_toggle_bond() -> finstack_core::Result<()> {
    println!("\n=== PIK Toggle Bond ===");

    let issue = Date::from_calendar_date(2025, Month::March, 1).unwrap();
    let maturity = Date::from_calendar_date(2027, Month::March, 1).unwrap();
    let toggle_date = Date::from_calendar_date(2026, Month::March, 1).unwrap();

    // Build schedule with PIK toggle
    let schedule_params = ScheduleParams {
        freq: Frequency::quarterly(),
        dc: DayCount::Thirty360,
        bdc: BusinessDayConvention::Following,
        calendar_id: Some("usd"),
        stub: StubKind::None,
    };

    let custom_schedule = cf()
        .principal(Money::new(10_000_000.0, Currency::USD), issue, maturity)
        .add_fixed_coupon_window(
            issue,
            toggle_date,
            0.08,
            schedule_params,
            CouponType::Split {
                cash_pct: 0.5,
                pik_pct: 0.5,
            },
        )
        .add_fixed_coupon_window(
            toggle_date,
            maturity,
            0.07,
            schedule_params,
            CouponType::Cash,
        )
        .build()?;

    // Create bond using builder pattern
    let bond = Bond::builder()
        .id("PIK_TOGGLE_2027")
        .cashflows(custom_schedule.clone())
        .disc_curve("USD-OIS")
        .build()?;

    // Create market data
    let disc_curve = DiscountCurve::builder("USD-OIS")
        .base_date(issue)
        .knots([(0.0, 1.0), (1.0, 0.96), (2.0, 0.92)])
        .linear_df()
        .build()?;

    let curves = CurveSet::new().with_discount(disc_curve);

    // Price the bond
    let result = bond.price(&curves, issue)?;
    println!("PIK toggle bond PV: {}", result.value);

    // Show outstanding path (PIK increases principal)
    let outstanding_path = custom_schedule.outstanding_path();
    println!("\nOutstanding principal path:");
    for (date, amount) in outstanding_path.iter().take(5) {
        println!("  {}: {}", date, amount);
    }
    if outstanding_path.len() > 5 {
        println!("  ... and {} more", outstanding_path.len() - 5);
    }

    Ok(())
}

fn example_amortizing_bond_with_fees() -> finstack_core::Result<()> {
    println!("\n=== Amortizing Bond with Fees ===");

    let issue = Date::from_calendar_date(2025, Month::June, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::June, 1).unwrap();

    // Build complex cashflow schedule
    let custom_schedule = cf()
        .principal(Money::new(50_000_000.0, Currency::EUR), issue, maturity)
        .amortization(AmortizationSpec::LinearTo {
            final_notional: Money::new(10_000_000.0, Currency::EUR),
        })
        .fixed_cf(FixedCouponSpec {
            coupon_type: CouponType::Cash,
            rate: 0.045,
            freq: Frequency::semi_annual(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some("eur"),
            stub: StubKind::ShortFront,
        })
        .build()?;

    // Create bond from cashflows
    let bond = Bond::from_cashflows("AMORT_BOND_2030", custom_schedule.clone(), "EUR-OIS", None)?;

    // Create market data with extended knots for 5-year bond
    let disc_curve = DiscountCurve::builder("EUR-OIS")
        .base_date(issue)
        .knots([
            (0.0, 1.0),
            (1.0, 0.98),
            (2.0, 0.95),
            (3.0, 0.93),
            (4.0, 0.90),
            (5.0, 0.87),
            (6.0, 0.84),
        ])
        .linear_df()
        .build()?;

    let curves = CurveSet::new().with_discount(disc_curve);

    // Price the bond
    let result = bond.price(&curves, issue)?;
    println!("Amortizing bond PV: {}", result.value);

    // Show flow breakdown by type
    use finstack_valuations::cashflow::primitives::CFKind;
    use std::collections::HashMap;

    let mut flows_by_kind: HashMap<CFKind, (usize, f64)> = HashMap::new();
    for cf in &custom_schedule.flows {
        let (count, total) = flows_by_kind.get(&cf.kind).unwrap_or(&(0, 0.0));
        flows_by_kind.insert(cf.kind, (count + 1, total + cf.amount.amount()));
    }

    println!("\nCashflow breakdown by type:");
    for (kind, (count, total)) in flows_by_kind {
        println!("  {:?}: {} flows, total {:.2} EUR", kind, count, total);
    }

    Ok(())
}

fn example_comparison_regular_vs_custom() -> finstack_core::Result<()> {
    println!("\n=== Regular Bond vs Custom Cashflow Bond Comparison ===");

    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 1).unwrap();

    // Create regular bond
    let regular_bond = Bond {
        id: "REGULAR_BOND".to_string(),
        notional: Money::new(1_000_000.0, Currency::USD),
        coupon: 0.05,
        freq: Frequency::annual(),
        dc: DayCount::Act365F,
        issue,
        maturity,
        disc_id: "USD-OIS",
        quoted_clean: None,
        call_put: None,
        amortization: None,
        custom_cashflows: None,
        attributes: finstack_valuations::traits::Attributes::new(),
    };

    // Create custom bond with higher frequency
    let custom_schedule = cf()
        .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
        .fixed_cf(FixedCouponSpec {
            coupon_type: CouponType::Cash,
            rate: 0.05,
            freq: Frequency::semi_annual(), // Higher frequency than regular bond
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        })
        .build()?;

    let custom_bond = Bond::builder()
        .id("CUSTOM_BOND")
        .cashflows(custom_schedule)
        .disc_curve("USD-OIS")
        .build()?;

    // Create market data
    let disc_curve = DiscountCurve::builder("USD-OIS")
        .base_date(issue)
        .knots([(0.0, 1.0), (1.0, 0.98)])
        .linear_df()
        .build()?;

    let curves = CurveSet::new().with_discount(disc_curve);

    // Compare pricing
    let regular_result = regular_bond.price(&curves, issue)?;
    let custom_result = custom_bond.price(&curves, issue)?;

    println!("Regular bond (annual payments): {}", regular_result.value);
    println!(
        "Custom bond (semi-annual payments): {}",
        custom_result.value
    );

    let regular_flows = regular_bond.build_schedule(&curves, issue)?;
    let custom_flows = custom_bond.build_schedule(&curves, issue)?;

    println!("\nFlow comparison:");
    println!("  Regular bond flows: {}", regular_flows.len());
    println!("  Custom bond flows: {}", custom_flows.len());
    println!("  Difference: Custom bond has more frequent payments");

    Ok(())
}

fn main() -> finstack_core::Result<()> {
    println!("{}", "=".repeat(60));
    println!("Bond Instruments with Custom Cashflow Schedules");
    println!("{}", "=".repeat(60));

    example_stepup_bond()?;
    example_pik_toggle_bond()?;
    example_amortizing_bond_with_fees()?;
    example_comparison_regular_vs_custom()?;

    println!();
    println!("{}", "=".repeat(60));
    println!("Examples completed successfully!");
    println!("{}", "=".repeat(60));

    Ok(())
}
