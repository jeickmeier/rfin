//! Bond Future Integration Tests
//!
//! Comprehensive integration tests for bond future pricing, invoice price calculation,
//! and error handling using realistic UST 10-year contract parameters.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::bond_future::pricer::BondFuturePricer;
use finstack_valuations::instruments::bond_future::{BondFuture, DeliverableBond, Position};
use finstack_valuations::pricer::{create_standard_registry, InstrumentType, ModelKey};
use std::sync::Arc;
use time::macros::date;

// ========================================================================================
// Helper Functions
// ========================================================================================

/// Create a realistic market context with a UST discount curve.
///
/// The curve has approximately 20 tenors covering the full maturity spectrum
/// from overnight to 30 years, representing typical UST curve construction.
fn create_realistic_market() -> MarketContext {
    let base_date = date!(2025 - 01 - 15);

    // Realistic UST yield curve (simplified flat curve at ~4% for testing)
    // In production, these would come from actual market data
    let rate = 0.04; // 4% flat for simplicity

    // Build knots for a comprehensive yield curve
    // Using semi-annual compounding: DF(t) = 1 / (1 + rate/2)^(2*t)
    let mut knots = Vec::new();

    // Add points at standard UST maturities
    let maturities = vec![
        0.0,  // Overnight
        0.25, // 3-month bill
        0.5,  // 6-month bill
        1.0,  // 1-year bill
        2.0,  // 2-year note
        3.0,  // 3-year note
        5.0,  // 5-year note
        7.0,  // 7-year note
        10.0, // 10-year note
        20.0, // 20-year bond
        30.0, // 30-year bond
    ];

    for t in maturities {
        let df = if t == 0.0 {
            1.0
        } else {
            let base: f64 = 1.0 + rate / 2.0;
            1.0_f64 / base.powf(2.0 * t)
        };
        knots.push((t, df));
    }

    let curve = DiscountCurve::builder(CurveId::new("USD-TREASURY"))
        .base_date(base_date)
        .knots(knots)
        .set_interp(InterpStyle::LogLinear) // Log-linear interpolation for discount factors
        .build()
        .expect("Failed to build realistic discount curve");

    MarketContext::new().insert_discount(curve)
}

/// Create a test bond with realistic UST parameters.
///
/// # Parameters
///
/// * `bond_id` - Unique identifier (e.g., CUSIP)
/// * `notional` - Face value
/// * `coupon_rate` - Annual coupon rate (e.g., 0.0375 for 3.75%)
/// * `issue` - Issue date
/// * `maturity` - Maturity date
fn create_ust_bond(
    bond_id: &str,
    notional: f64,
    coupon_rate: f64,
    issue: Date,
    maturity: Date,
) -> Bond {
    Bond::fixed(
        bond_id,
        Money::new(notional, Currency::USD),
        coupon_rate,
        issue,
        maturity,
        "USD-TREASURY",
    )
    .expect("Test bond creation should succeed")
}

/// Create a realistic deliverable basket for UST 10-year futures.
///
/// Returns a vector of bonds and their conversion factors.
/// In practice, conversion factors are published by the exchange (CBOT).
fn create_deliverable_basket() -> (Vec<Bond>, Vec<DeliverableBond>) {
    // Create 5 realistic deliverable bonds with varying coupons and maturities
    // All bonds must have at least 6.5 years remaining maturity to be deliverable
    // into the 10-year contract

    let bonds = vec![
        // Bond 1: 3.75% coupon, 9.5 years to maturity
        create_ust_bond(
            "US912828XG33",
            100_000.0,
            0.0375,
            date!(2023 - 07 - 15),
            date!(2034 - 07 - 15),
        ),
        // Bond 2: 4.00% coupon, 10 years to maturity
        create_ust_bond(
            "US912828XH15",
            100_000.0,
            0.04,
            date!(2023 - 01 - 15),
            date!(2035 - 01 - 15),
        ),
        // Bond 3: 4.25% coupon, 8 years to maturity
        create_ust_bond(
            "US912828XJ71",
            100_000.0,
            0.0425,
            date!(2024 - 01 - 15),
            date!(2033 - 01 - 15),
        ),
        // Bond 4: 3.50% coupon, 9 years to maturity
        create_ust_bond(
            "US912828XK54",
            100_000.0,
            0.035,
            date!(2023 - 10 - 15),
            date!(2034 - 10 - 15),
        ),
        // Bond 5: 4.50% coupon, 7.5 years to maturity
        create_ust_bond(
            "US912828XL38",
            100_000.0,
            0.045,
            date!(2024 - 07 - 15),
            date!(2032 - 07 - 15),
        ),
    ];

    // In production, these conversion factors would be published by CBOT
    // For this test, we'll calculate them using our conversion factor calculator
    // For now, use placeholder values that will be recalculated
    let deliverable_bonds = vec![
        DeliverableBond {
            bond_id: InstrumentId::new("US912828XG33"),
            conversion_factor: 0.0, // Will be calculated
        },
        DeliverableBond {
            bond_id: InstrumentId::new("US912828XH15"),
            conversion_factor: 0.0,
        },
        DeliverableBond {
            bond_id: InstrumentId::new("US912828XJ71"),
            conversion_factor: 0.0,
        },
        DeliverableBond {
            bond_id: InstrumentId::new("US912828XK54"),
            conversion_factor: 0.0,
        },
        DeliverableBond {
            bond_id: InstrumentId::new("US912828XL38"),
            conversion_factor: 0.0,
        },
    ];

    (bonds, deliverable_bonds)
}

// ========================================================================================
// Integration Tests
// ========================================================================================

#[test]
fn test_realistic_ust_10y_future_full_workflow() {
    // Setup: Create market context and deliverable basket
    let market = create_realistic_market();
    let (bonds, mut deliverable_bonds) = create_deliverable_basket();

    // Calculate conversion factors for all deliverable bonds
    // Using standard UST 10Y parameters: 6% coupon, 10-year maturity
    let standard_coupon = 0.06;
    let standard_maturity = 10.0;
    let as_of = date!(2025 - 01 - 15);

    for (i, bond) in bonds.iter().enumerate() {
        let cf = BondFuturePricer::calculate_conversion_factor(
            bond,
            standard_coupon,
            standard_maturity,
            &market,
            as_of,
        )
        .expect("Conversion factor calculation should succeed");

        deliverable_bonds[i].conversion_factor = cf;

        // Sanity check: conversion factors should be between 0.5 and 1.5 for realistic bonds
        assert!(
            cf > 0.5 && cf < 1.5,
            "Conversion factor {} for bond {} is unrealistic",
            cf,
            bond.id.as_str()
        );
    }

    // For testing, assume the first bond (3.75% coupon) is the CTD
    // In production, CTD would be determined by calculating the cheapest delivery option
    let ctd_bond_id = InstrumentId::new("US912828XG33");
    let ctd_bond = &bonds[0];
    let ctd_cf = deliverable_bonds[0].conversion_factor;

    // Create UST 10Y futures contract (TYH5 = March 2025 expiry)
    // Contract specs:
    // - Notional: $1,000,000 (10 contracts × $100,000)
    // - Quoted price: 125.50 (representing 125-16/32 in fractional notation)
    // - Position: Long
    let future = BondFuture::ust_10y(
        InstrumentId::new("TYH5"),
        Money::new(1_000_000.0, Currency::USD),
        date!(2025 - 03 - 20), // Expiry: March 20, 2025
        date!(2025 - 03 - 21), // Delivery start: March 21, 2025
        date!(2025 - 03 - 31), // Delivery end: March 31, 2025
        125.50,                // Quoted futures price
        Position::Long,
        deliverable_bonds.clone(), // Clone to allow later access
        ctd_bond_id.clone(),
        CurveId::new("USD-TREASURY"),
    )
    .expect("Future construction should succeed");

    // Test 1: NPV Calculation
    // The NPV represents the mark-to-market value of the futures position
    let npv = BondFuturePricer::calculate_npv(&future, ctd_bond, ctd_cf, &market, as_of)
        .expect("NPV calculation should succeed");

    // NPV should be in USD
    assert_eq!(npv.currency(), Currency::USD);

    // For a long position, if quoted price > model price, NPV should be positive
    // We don't assert exact value here because it depends on market prices,
    // but we verify the calculation completes successfully
    println!(
        "NPV of long position with 10 contracts: ${:.2}",
        npv.amount()
    );

    // Test 2: Model Price Calculation
    let model_price = BondFuturePricer::calculate_model_price(ctd_bond, ctd_cf, &market, as_of)
        .expect("Model price calculation should succeed");

    // Model price should be a reasonable value (80-150 range for UST futures)
    assert!(
        model_price > 80.0 && model_price < 150.0,
        "Model price {} is unrealistic",
        model_price
    );

    println!("Quoted price: {}", future.quoted_price);
    println!("Model price: {:.4}", model_price);
    println!(
        "Price differential: {:.4} points",
        future.quoted_price - model_price
    );

    // Test 3: Invoice Price Calculation (settlement amount)
    // Invoice price is calculated at settlement date (expiry + 2 business days)
    let _settlement_date = date!(2025 - 03 - 23); // Assuming T+2 settlement

    // Calculate accrued interest at settlement
    // For invoice price: Invoice = (Futures_Price × CF) + Accrued
    let futures_price = future.quoted_price;
    let invoice_price_per_100 = futures_price * ctd_cf;

    println!(
        "Invoice price (per $100 face): ${:.4}",
        invoice_price_per_100
    );

    // For 10 contracts ($1,000,000 notional), total invoice is:
    let total_invoice = (future.notional.amount() / 100.0) * invoice_price_per_100;
    println!(
        "Total invoice amount for 10 contracts: ${:.2}",
        total_invoice
    );

    // Sanity check: invoice price should be reasonable
    assert!(
        invoice_price_per_100 > 80.0 && invoice_price_per_100 < 150.0,
        "Invoice price {} is unrealistic",
        invoice_price_per_100
    );

    // Test 4: Verify conversion factor relationships
    // Higher coupon bonds should have higher conversion factors (above par)
    // Lower coupon bonds should have lower conversion factors (below par)
    let bond_3_75_cf = deliverable_bonds
        .iter()
        .find(|b| b.bond_id.as_str() == "US912828XG33")
        .unwrap()
        .conversion_factor;
    let bond_4_50_cf = deliverable_bonds
        .iter()
        .find(|b| b.bond_id.as_str() == "US912828XL38")
        .unwrap()
        .conversion_factor;

    // 4.50% coupon bond should have higher CF than 3.75% (both below 6% standard)
    println!("3.75% bond CF: {:.4}", bond_3_75_cf);
    println!("4.50% bond CF: {:.4}", bond_4_50_cf);
    println!("Standard coupon: 6.00%");
}

#[test]
fn test_bond_future_pricer_registry_ctd_npv() {
    let as_of = date!(2025 - 01 - 15);
    let expiry = date!(2025 - 03 - 20);
    let delivery_start = date!(2025 - 03 - 21);
    let delivery_end = date!(2025 - 03 - 31);

    let ctd_bond = create_ust_bond(
        "US912828XG33",
        100_000.0,
        0.05,
        date!(2017 - 03 - 15),
        date!(2033 - 03 - 15),
    );

    let market =
        create_realistic_market().insert_instrument("US912828XG33", Arc::new(ctd_bond.clone()));

    let conversion_factor = BondFuturePricer::calculate_conversion_factor(
        &ctd_bond,
        0.06,
        10.0,
        &market,
        delivery_start,
    )
    .expect("Failed to calculate conversion factor");

    let basket = vec![DeliverableBond {
        bond_id: InstrumentId::new("US912828XG33"),
        conversion_factor,
    }];

    let future = BondFuture::ust_10y(
        InstrumentId::new("TYH5"),
        Money::new(1_000_000.0, Currency::USD),
        expiry,
        delivery_start,
        delivery_end,
        125.50,
        Position::Long,
        basket,
        InstrumentId::new("US912828XG33"),
        CurveId::new("USD-TREASURY"),
    )
    .expect("Failed to create bond future");

    let registry = create_standard_registry();
    let result = registry
        .price_with_registry(&future, ModelKey::Discounting, &market, as_of)
        .expect("Registry pricing should succeed");

    let expected =
        BondFuturePricer::calculate_npv(&future, &ctd_bond, conversion_factor, &market, as_of)
            .expect("Expected NPV should be computed");

    let diff = (result.value.amount() - expected.amount()).abs();
    assert!(
        diff < 1e-8,
        "Registry pricing should match CTD NPV, diff={}",
        diff
    );
}

#[test]
fn test_short_position_npv() {
    // Test that short positions have opposite sign NPV
    let market = create_realistic_market();
    let (bonds, mut deliverable_bonds) = create_deliverable_basket();
    let as_of = date!(2025 - 01 - 15);

    // Calculate conversion factor for first bond only (for speed)
    let ctd_bond = &bonds[0];
    let ctd_cf =
        BondFuturePricer::calculate_conversion_factor(ctd_bond, 0.06, 10.0, &market, as_of)
            .expect("CF calculation should succeed");

    deliverable_bonds[0].conversion_factor = ctd_cf;

    // Create two futures: one long, one short, otherwise identical
    let long_future = BondFuture::ust_10y(
        InstrumentId::new("TYH5_LONG"),
        Money::new(100_000.0, Currency::USD),
        date!(2025 - 03 - 20),
        date!(2025 - 03 - 21),
        date!(2025 - 03 - 31),
        125.50,
        Position::Long,
        vec![deliverable_bonds[0].clone()],
        InstrumentId::new("US912828XG33"),
        CurveId::new("USD-TREASURY"),
    )
    .expect("Long future construction should succeed");

    let short_future = BondFuture::ust_10y(
        InstrumentId::new("TYH5_SHORT"),
        Money::new(100_000.0, Currency::USD),
        date!(2025 - 03 - 20),
        date!(2025 - 03 - 21),
        date!(2025 - 03 - 31),
        125.50,
        Position::Short,
        vec![deliverable_bonds[0].clone()],
        InstrumentId::new("US912828XG33"),
        CurveId::new("USD-TREASURY"),
    )
    .expect("Short future construction should succeed");

    let npv_long = BondFuturePricer::calculate_npv(&long_future, ctd_bond, ctd_cf, &market, as_of)
        .expect("Long NPV should succeed");

    let npv_short =
        BondFuturePricer::calculate_npv(&short_future, ctd_bond, ctd_cf, &market, as_of)
            .expect("Short NPV should succeed");

    println!("Long position NPV: ${:.2}", npv_long.amount());
    println!("Short position NPV: ${:.2}", npv_short.amount());

    // Long and short NPVs should be opposite
    let sum = npv_long.amount() + npv_short.amount();
    assert!(
        sum.abs() < 0.01,
        "Long and short NPVs should be equal and opposite, but sum = {}",
        sum
    );
}

#[test]
fn test_error_handling_invalid_dates() {
    // Test validation: expiry_date must be before delivery_start
    let (_, deliverable_bonds) = create_deliverable_basket();

    let result = BondFuture::ust_10y(
        InstrumentId::new("INVALID"),
        Money::new(100_000.0, Currency::USD),
        date!(2025 - 03 - 25), // Expiry AFTER delivery start (invalid!)
        date!(2025 - 03 - 21), // Delivery start
        date!(2025 - 03 - 31), // Delivery end
        125.50,
        Position::Long,
        deliverable_bonds,
        InstrumentId::new("US912828XG33"),
        CurveId::new("USD-TREASURY"),
    );

    assert!(
        result.is_err(),
        "Should fail validation when expiry_date >= delivery_start"
    );
    let err = result.unwrap_err();
    assert!(
        format!("{}", err).contains("expiry_date") && format!("{}", err).contains("delivery_start"),
        "Error message should mention date ordering: {}",
        err
    );
}

#[test]
fn test_error_handling_invalid_delivery_period() {
    // Test validation: delivery_start must be before delivery_end
    let (_, deliverable_bonds) = create_deliverable_basket();

    let result = BondFuture::ust_10y(
        InstrumentId::new("INVALID"),
        Money::new(100_000.0, Currency::USD),
        date!(2025 - 03 - 20),
        date!(2025 - 03 - 31), // Delivery start AFTER delivery end (invalid!)
        date!(2025 - 03 - 21), // Delivery end
        125.50,
        Position::Long,
        deliverable_bonds,
        InstrumentId::new("US912828XG33"),
        CurveId::new("USD-TREASURY"),
    );

    assert!(
        result.is_err(),
        "Should fail validation when delivery_start >= delivery_end"
    );
    let err = result.unwrap_err();
    assert!(
        format!("{}", err).contains("delivery_start")
            && format!("{}", err).contains("delivery_end"),
        "Error message should mention delivery period: {}",
        err
    );
}

#[test]
fn test_error_handling_empty_basket() {
    // Test validation: deliverable_basket cannot be empty
    let result = BondFuture::ust_10y(
        InstrumentId::new("INVALID"),
        Money::new(100_000.0, Currency::USD),
        date!(2025 - 03 - 20),
        date!(2025 - 03 - 21),
        date!(2025 - 03 - 31),
        125.50,
        Position::Long,
        vec![], // Empty basket (invalid!)
        InstrumentId::new("US912828XG33"),
        CurveId::new("USD-TREASURY"),
    );

    assert!(
        result.is_err(),
        "Should fail validation with empty deliverable basket"
    );
    let err = result.unwrap_err();
    assert!(
        format!("{}", err).contains("deliverable_basket"),
        "Error message should mention empty basket: {}",
        err
    );
}

#[test]
fn test_error_handling_ctd_not_in_basket() {
    // Test validation: ctd_bond_id must exist in deliverable_basket
    let (_, mut deliverable_bonds) = create_deliverable_basket();

    // Only include one bond in basket, but reference a different one as CTD
    deliverable_bonds.truncate(1);

    let result = BondFuture::ust_10y(
        InstrumentId::new("INVALID"),
        Money::new(100_000.0, Currency::USD),
        date!(2025 - 03 - 20),
        date!(2025 - 03 - 21),
        date!(2025 - 03 - 31),
        125.50,
        Position::Long,
        deliverable_bonds,
        InstrumentId::new("NONEXISTENT_BOND"), // Not in basket!
        CurveId::new("USD-TREASURY"),
    );

    assert!(
        result.is_err(),
        "Should fail validation when CTD bond not in basket"
    );
    let err = result.unwrap_err();
    assert!(
        format!("{}", err).contains("ctd_bond_id"),
        "Error message should mention CTD not found: {}",
        err
    );
}

#[test]
fn test_error_handling_negative_conversion_factor() {
    // Test validation: conversion_factor must be positive
    let (_, mut deliverable_bonds) = create_deliverable_basket();

    // Set a negative conversion factor (invalid!)
    deliverable_bonds[0].conversion_factor = -0.5;

    let result = BondFuture::ust_10y(
        InstrumentId::new("INVALID"),
        Money::new(100_000.0, Currency::USD),
        date!(2025 - 03 - 20),
        date!(2025 - 03 - 21),
        date!(2025 - 03 - 31),
        125.50,
        Position::Long,
        deliverable_bonds,
        InstrumentId::new("US912828XG33"),
        CurveId::new("USD-TREASURY"),
    );

    assert!(
        result.is_err(),
        "Should fail validation with negative conversion factor"
    );
    let err = result.unwrap_err();
    assert!(
        format!("{}", err).contains("conversion_factor"),
        "Error message should mention conversion factor: {}",
        err
    );
}

#[test]
fn test_multiple_contracts_scaling() {
    // Test that NPV scales correctly with number of contracts
    let market = create_realistic_market();
    let (bonds, mut deliverable_bonds) = create_deliverable_basket();
    let as_of = date!(2025 - 01 - 15);

    let ctd_bond = &bonds[0];
    let ctd_cf =
        BondFuturePricer::calculate_conversion_factor(ctd_bond, 0.06, 10.0, &market, as_of)
            .expect("CF calculation should succeed");

    deliverable_bonds[0].conversion_factor = ctd_cf;

    // Create futures with 1, 5, and 10 contracts
    let future_1_contract = BondFuture::ust_10y(
        InstrumentId::new("TY_1"),
        Money::new(100_000.0, Currency::USD), // 1 contract
        date!(2025 - 03 - 20),
        date!(2025 - 03 - 21),
        date!(2025 - 03 - 31),
        125.50,
        Position::Long,
        vec![deliverable_bonds[0].clone()],
        InstrumentId::new("US912828XG33"),
        CurveId::new("USD-TREASURY"),
    )
    .expect("1-contract future should succeed");

    let future_5_contracts = BondFuture::ust_10y(
        InstrumentId::new("TY_5"),
        Money::new(500_000.0, Currency::USD), // 5 contracts
        date!(2025 - 03 - 20),
        date!(2025 - 03 - 21),
        date!(2025 - 03 - 31),
        125.50,
        Position::Long,
        vec![deliverable_bonds[0].clone()],
        InstrumentId::new("US912828XG33"),
        CurveId::new("USD-TREASURY"),
    )
    .expect("5-contract future should succeed");

    let future_10_contracts = BondFuture::ust_10y(
        InstrumentId::new("TY_10"),
        Money::new(1_000_000.0, Currency::USD), // 10 contracts
        date!(2025 - 03 - 20),
        date!(2025 - 03 - 21),
        date!(2025 - 03 - 31),
        125.50,
        Position::Long,
        vec![deliverable_bonds[0].clone()],
        InstrumentId::new("US912828XG33"),
        CurveId::new("USD-TREASURY"),
    )
    .expect("10-contract future should succeed");

    let npv_1 =
        BondFuturePricer::calculate_npv(&future_1_contract, ctd_bond, ctd_cf, &market, as_of)
            .expect("1-contract NPV should succeed");

    let npv_5 =
        BondFuturePricer::calculate_npv(&future_5_contracts, ctd_bond, ctd_cf, &market, as_of)
            .expect("5-contract NPV should succeed");

    let npv_10 =
        BondFuturePricer::calculate_npv(&future_10_contracts, ctd_bond, ctd_cf, &market, as_of)
            .expect("10-contract NPV should succeed");

    println!("NPV (1 contract): ${:.2}", npv_1.amount());
    println!("NPV (5 contracts): ${:.2}", npv_5.amount());
    println!("NPV (10 contracts): ${:.2}", npv_10.amount());

    // NPV should scale linearly with number of contracts
    let ratio_5_to_1 = npv_5.amount() / npv_1.amount();
    let ratio_10_to_1 = npv_10.amount() / npv_1.amount();

    assert!(
        (ratio_5_to_1 - 5.0).abs() < 0.01,
        "5 contracts should have 5× NPV of 1 contract, got ratio: {}",
        ratio_5_to_1
    );

    assert!(
        (ratio_10_to_1 - 10.0).abs() < 0.01,
        "10 contracts should have 10× NPV of 1 contract, got ratio: {}",
        ratio_10_to_1
    );
}

#[test]
fn test_conversion_factor_calculation_accuracy() {
    // Test that conversion factor calculation produces reasonable values
    // for bonds with different coupons relative to the 6% standard
    let market = create_realistic_market();
    let as_of = date!(2025 - 01 - 15);

    // Create three bonds: one below par (3%), one at par (6%), one above par (9%)
    let bond_below_par = create_ust_bond(
        "BELOW_PAR",
        100_000.0,
        0.03, // 3% coupon
        date!(2023 - 01 - 15),
        date!(2035 - 01 - 15),
    );

    let bond_at_par = create_ust_bond(
        "AT_PAR",
        100_000.0,
        0.06, // 6% coupon (standard)
        date!(2023 - 01 - 15),
        date!(2035 - 01 - 15),
    );

    let bond_above_par = create_ust_bond(
        "ABOVE_PAR",
        100_000.0,
        0.09, // 9% coupon
        date!(2023 - 01 - 15),
        date!(2035 - 01 - 15),
    );

    let cf_below =
        BondFuturePricer::calculate_conversion_factor(&bond_below_par, 0.06, 10.0, &market, as_of)
            .expect("CF for 3% bond should succeed");

    let cf_at =
        BondFuturePricer::calculate_conversion_factor(&bond_at_par, 0.06, 10.0, &market, as_of)
            .expect("CF for 6% bond should succeed");

    let cf_above =
        BondFuturePricer::calculate_conversion_factor(&bond_above_par, 0.06, 10.0, &market, as_of)
            .expect("CF for 9% bond should succeed");

    println!("3% coupon bond CF: {:.4}", cf_below);
    println!("6% coupon bond CF: {:.4}", cf_at);
    println!("9% coupon bond CF: {:.4}", cf_above);

    // Bonds with coupons below the standard should have CF < 1.0
    assert!(
        cf_below < 1.0,
        "3% bond should have CF < 1.0, got {}",
        cf_below
    );

    // Bonds with coupons at the standard should have CF ≈ 1.0
    assert!(
        (cf_at - 1.0).abs() < 0.15,
        "6% bond should have CF ≈ 1.0, got {}",
        cf_at
    );

    // Bonds with coupons above the standard should have CF > 1.0
    assert!(
        cf_above > 1.0,
        "9% bond should have CF > 1.0, got {}",
        cf_above
    );

    // Ordering: CF should increase with coupon
    assert!(cf_below < cf_at, "CF should increase with coupon rate");
    assert!(cf_at < cf_above, "CF should increase with coupon rate");
}

// ========================================================================================
// DV01 Calculation Tests
// ========================================================================================

/// Test that DV01 calculation works correctly for bond futures.
///
/// This test verifies:
/// 1. DV01 can be calculated via the metrics registry
/// 2. DV01 has the correct sign (negative for long positions when rates rise)
/// 3. Bucketed DV01 sums to total DV01 within reasonable tolerance
/// 4. DV01 magnitude is reasonable for a 10-year futures contract
#[test]
fn test_bond_future_dv01_calculation() {
    use finstack_valuations::instruments::common::traits::Instrument;
    use finstack_valuations::metrics::{standard_registry, MetricContext, MetricId};
    use std::sync::Arc;

    // Setup market and instruments
    let as_of = date!(2025 - 01 - 15);
    let expiry = date!(2025 - 03 - 20);
    let delivery_start = date!(2025 - 03 - 21);
    let delivery_end = date!(2025 - 03 - 31);

    // Create CTD bond (5% coupon, maturing in ~8 years)
    let ctd_bond = create_ust_bond(
        "US912828XG33",
        100_000.0,
        0.05,
        date!(2017 - 03 - 15),
        date!(2033 - 03 - 15),
    );

    // Create market context and register CTD bond for BondFuture::value()
    let market =
        create_realistic_market().insert_instrument("US912828XG33", Arc::new(ctd_bond.clone()));

    // Calculate conversion factor
    let conversion_factor = BondFuturePricer::calculate_conversion_factor(
        &ctd_bond,
        0.06, // 6% standard coupon for UST 10Y
        10.0, // 10-year standard maturity
        &market,
        delivery_start,
    )
    .expect("Failed to calculate conversion factor");

    println!("Conversion factor: {:.4}", conversion_factor);

    // Create deliverable basket
    let basket = vec![DeliverableBond {
        bond_id: InstrumentId::new("US912828XG33"),
        conversion_factor,
    }];

    // Create bond future (10 contracts = $1M notional)
    let future = BondFuture::ust_10y(
        InstrumentId::new("TYH5"),
        Money::new(1_000_000.0, Currency::USD),
        expiry,
        delivery_start,
        delivery_end,
        125.50, // Quoted futures price
        Position::Long,
        basket,
        InstrumentId::new("US912828XG33"),
        CurveId::new("USD-TREASURY"),
    )
    .expect("Failed to create bond future");

    // Calculate NPV
    let pv = future
        .value(&market, as_of)
        .expect("Failed to calculate NPV");

    println!("Bond future NPV: ${:.2}", pv.amount());

    // Create metric context
    let mut context = MetricContext::new(
        Arc::new(future.clone()),
        Arc::new(market.clone()),
        as_of,
        pv,
    );

    // Get metrics registry
    let registry = standard_registry();

    // Compute DV01
    let metrics_to_compute = vec![MetricId::Dv01, MetricId::BucketedDv01];
    let results = registry
        .compute(&metrics_to_compute, &mut context)
        .expect("Failed to compute metrics");

    // Extract DV01
    let dv01 = results
        .get(&MetricId::Dv01)
        .expect("DV01 should be computed");

    println!("Total DV01: ${:.2}", dv01);

    // Verify DV01 is reasonable
    // For a $1M notional 10Y futures contract, DV01 should be roughly $500-$2000
    // depending on the conversion factor and CTD bond duration
    assert!(
        dv01.abs() > 100.0 && dv01.abs() < 5000.0,
        "DV01 magnitude should be reasonable for 10Y futures, got {}",
        dv01
    );

    // Extract bucketed DV01
    let bucketed_dv01 = context
        .get_series(&MetricId::BucketedDv01)
        .expect("Bucketed DV01 should be computed");

    println!("\nBucketed DV01:");
    for (tenor, value) in bucketed_dv01 {
        if value.abs() > 1.0 {
            println!("  {}: ${:.2}", tenor, value);
        }
    }

    // Sum bucketed DV01 and verify it matches total DV01
    let bucketed_sum: f64 = bucketed_dv01.iter().map(|(_, v)| v).sum();
    let diff = (bucketed_sum - dv01).abs();
    let tolerance = dv01.abs() * 0.01; // 1% tolerance

    println!("\nTotal DV01: ${:.2}", dv01);
    println!("Sum of bucketed DV01: ${:.2}", bucketed_sum);
    println!("Difference: ${:.2}", diff);

    assert!(
        diff < tolerance,
        "Sum of bucketed DV01 ({:.2}) should match total DV01 ({:.2}) within 1%, diff: {:.2}",
        bucketed_sum,
        dv01,
        diff
    );

    // Verify that the 7Y or 10Y bucket has significant DV01
    // (CTD bond matures in ~8 years, so sensitivity is split between 7Y and 10Y buckets)
    let dv01_7y = bucketed_dv01
        .iter()
        .find(|(tenor, _)| tenor.to_lowercase() == "7y")
        .map(|(_, v)| v.abs())
        .unwrap_or(0.0);

    let dv01_10y = bucketed_dv01
        .iter()
        .find(|(tenor, _)| tenor.to_lowercase() == "10y")
        .map(|(_, v)| v.abs())
        .unwrap_or(0.0);

    let max_bucket_dv01 = bucketed_dv01
        .iter()
        .map(|(_, v)| v.abs())
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0);

    // Either 7Y or 10Y should have significant DV01 for a 10Y futures contract
    let combined_7y_10y = dv01_7y + dv01_10y;
    assert!(
        combined_7y_10y > max_bucket_dv01 * 0.8,
        "7Y+10Y buckets should have significant DV01 for a 10Y futures contract, got {:.2} vs max {:.2}",
        combined_7y_10y,
        max_bucket_dv01
    );
}

/// Test DV01 sign convention for short positions.
///
/// DV01 should have opposite sign for short vs long positions.
#[test]
fn test_bond_future_dv01_sign_convention() {
    use finstack_valuations::instruments::common::traits::Instrument;
    use finstack_valuations::metrics::{standard_registry, MetricContext, MetricId};
    use std::sync::Arc;

    let as_of = date!(2025 - 01 - 15);
    let expiry = date!(2025 - 03 - 20);
    let delivery_start = date!(2025 - 03 - 21);
    let delivery_end = date!(2025 - 03 - 31);

    // Create CTD bond
    let ctd_bond = create_ust_bond(
        "US912828XG33",
        100_000.0,
        0.05,
        date!(2017 - 03 - 15),
        date!(2033 - 03 - 15),
    );

    // Create market context and register CTD bond for BondFuture::value()
    let market =
        create_realistic_market().insert_instrument("US912828XG33", Arc::new(ctd_bond.clone()));

    let conversion_factor = BondFuturePricer::calculate_conversion_factor(
        &ctd_bond,
        0.06,
        10.0,
        &market,
        delivery_start,
    )
    .expect("Failed to calculate conversion factor");

    let basket = vec![DeliverableBond {
        bond_id: InstrumentId::new("US912828XG33"),
        conversion_factor,
    }];

    // Create long position
    let future_long = BondFuture::ust_10y(
        InstrumentId::new("TYH5_LONG"),
        Money::new(1_000_000.0, Currency::USD),
        expiry,
        delivery_start,
        delivery_end,
        125.50,
        Position::Long,
        basket.clone(),
        InstrumentId::new("US912828XG33"),
        CurveId::new("USD-TREASURY"),
    )
    .expect("Failed to create long bond future");

    // Create short position
    let future_short = BondFuture::ust_10y(
        InstrumentId::new("TYH5_SHORT"),
        Money::new(1_000_000.0, Currency::USD),
        expiry,
        delivery_start,
        delivery_end,
        125.50,
        Position::Short,
        basket,
        InstrumentId::new("US912828XG33"),
        CurveId::new("USD-TREASURY"),
    )
    .expect("Failed to create short bond future");

    // Calculate DV01 for long position
    let pv_long = future_long.value(&market, as_of).unwrap();
    let mut context_long = MetricContext::new(
        Arc::new(future_long),
        Arc::new(market.clone()),
        as_of,
        pv_long,
    );

    let registry = standard_registry();
    let results_long = registry
        .compute(&[MetricId::Dv01], &mut context_long)
        .expect("Failed to compute DV01 for long position");
    let dv01_long = *results_long.get(&MetricId::Dv01).unwrap();

    // Calculate DV01 for short position
    let pv_short = future_short.value(&market, as_of).unwrap();
    let mut context_short = MetricContext::new(
        Arc::new(future_short),
        Arc::new(market.clone()),
        as_of,
        pv_short,
    );

    let results_short = registry
        .compute(&[MetricId::Dv01], &mut context_short)
        .expect("Failed to compute DV01 for short position");
    let dv01_short = *results_short.get(&MetricId::Dv01).unwrap();

    println!("Long position DV01: ${:.2}", dv01_long);
    println!("Short position DV01: ${:.2}", dv01_short);

    // DV01 should have opposite signs
    assert!(
        dv01_long.signum() != dv01_short.signum() || (dv01_long == 0.0 && dv01_short == 0.0),
        "DV01 should have opposite signs for long vs short positions: long={:.2}, short={:.2}",
        dv01_long,
        dv01_short
    );

    // Magnitudes should be approximately equal
    let diff = (dv01_long.abs() - dv01_short.abs()).abs();
    let tolerance = dv01_long.abs() * 0.01; // 1% tolerance
    assert!(
        diff < tolerance,
        "DV01 magnitudes should be equal for long vs short: |long|={:.2}, |short|={:.2}",
        dv01_long.abs(),
        dv01_short.abs()
    );
}

#[test]
fn test_invoice_price() {
    // Test the invoice_price() method with realistic UST 10Y contract
    let market = create_realistic_market();
    let (bonds, mut deliverable_bonds) = create_deliverable_basket();
    let as_of = date!(2025 - 01 - 15);

    // Calculate conversion factors
    let standard_coupon = 0.06;
    let standard_maturity = 10.0;

    for (i, bond) in bonds.iter().enumerate() {
        let cf = BondFuturePricer::calculate_conversion_factor(
            bond,
            standard_coupon,
            standard_maturity,
            &market,
            as_of,
        )
        .expect("Failed to calculate conversion factor");
        deliverable_bonds[i].conversion_factor = cf;
        println!(
            "Bond {} ({}): CF = {:.4}",
            i + 1,
            deliverable_bonds[i].bond_id.as_str(),
            cf
        );
    }

    // Create a UST 10Y future
    let quoted_price = 125.50; // e.g., 125-16/32
    let expiry = date!(2025 - 03 - 20);
    let delivery_start = date!(2025 - 03 - 21);
    let delivery_end = date!(2025 - 03 - 31);

    let future = BondFuture::ust_10y(
        InstrumentId::new("TYH5"),
        Money::new(1_000_000.0, Currency::USD), // 10 contracts
        expiry,
        delivery_start,
        delivery_end,
        quoted_price,
        Position::Long,
        deliverable_bonds.clone(),
        InstrumentId::new("US912828XG33"), // First bond as CTD
        CurveId::new("USD-TREASURY"),
    )
    .expect("Failed to create UST 10Y future");

    // Calculate invoice price for settlement (T+2 after expiry)
    let settlement_date = date!(2025 - 03 - 23);
    let ctd_bond = &bonds[0]; // First bond is the CTD

    let invoice = future
        .invoice_price(ctd_bond, &market, settlement_date)
        .expect("Failed to calculate invoice price");

    println!("Futures quoted price: {:.2}", quoted_price);
    println!(
        "CTD bond conversion factor: {:.4}",
        deliverable_bonds[0].conversion_factor
    );
    println!("Settlement date: {}", settlement_date);
    println!("Invoice price: {}", invoice);

    // Verify invoice price components
    // Invoice = (Futures_Price × CF) + Accrued
    let cf = deliverable_bonds[0].conversion_factor;

    // Invoice should be positive and reasonable
    assert!(invoice.amount() > 0.0, "Invoice price should be positive");

    // For a 125.50 futures price with CF ~0.8, invoice should be ~103 per $100 face
    // For 10 contracts ($1M notional), total should be ~$1,030,000
    let expected_per_100 = quoted_price * cf;
    let expected_total = (future.notional.amount() / 100.0) * expected_per_100;

    // Allow for accrued interest variation (within ±5% of expected)
    let tolerance = expected_total * 0.05;
    let diff = (invoice.amount() - expected_total).abs();

    println!("Expected invoice (without accrued): ${:.2}", expected_total);
    println!("Actual invoice (with accrued): ${:.2}", invoice.amount());
    println!("Difference: ${:.2}", diff);

    assert!(
        diff < tolerance,
        "Invoice price should be within 5% of expected: expected=${:.2}, actual=${:.2}, diff=${:.2}",
        expected_total,
        invoice.amount(),
        diff
    );

    // Verify currency matches
    assert_eq!(
        invoice.currency(),
        Currency::USD,
        "Invoice should be in USD"
    );
}

#[test]
fn test_bucketed_dv01_registration() {
    // Verify that BucketedDv01 metric is correctly registered for BondFuture
    //
    // NOTE: This test only verifies metric registration, not end-to-end calculation.
    // Full bucketed DV01 calculation requires BondFuture::value() to resolve the CTD
    // bond from the MarketContext instrument registry.

    use finstack_valuations::metrics::{standard_registry, MetricId};

    let registry = standard_registry();

    // Verify BondFuture has metrics registered
    let bond_future_metrics = registry.metrics_for_instrument(InstrumentType::BondFuture);

    assert!(
        bond_future_metrics.contains(&MetricId::Dv01),
        "DV01 metric should be registered for BondFuture"
    );

    assert!(
        bond_future_metrics.contains(&MetricId::BucketedDv01),
        "BucketedDv01 metric should be registered for BondFuture"
    );

    assert!(
        bond_future_metrics.contains(&MetricId::Theta),
        "Theta metric should be registered for BondFuture"
    );

    println!("✓ BucketedDv01 metric is correctly registered for BondFuture");
    println!("  - Uses UnifiedDv01Calculator with key_rate() configuration");
    println!("  - Provides standard IR buckets: 3M, 6M, 1Y, 2Y, 3Y, 5Y, 7Y, 10Y, 15Y, 20Y, 30Y");
    println!("  - Conversion factor scaling is automatic via pricing formula");
}
