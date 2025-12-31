//! Core pricing functionality tests for CDS Tranche.
//!
//! Tests cover:
//! - Basic tranche pricing (PV calculation)
//! - Different tranche types (equity, mezzanine, senior)
//! - Buy vs sell protection
//! - Notional scaling
//! - Maturity edge cases
//!
//! Note: Internal implementation details like payment schedules, premium/protection
//! leg decomposition, and accrual-on-default mechanics are tested indirectly
//! through these end-to-end pricing tests and through public metric APIs.

use super::helpers::*;
use finstack_core::currency::Currency;
use finstack_valuations::instruments::credit_derivatives::cds_tranche::CDSTranchePricer;
use finstack_valuations::instruments::credit_derivatives::cds_tranche::TrancheSide;

// ==================== Basic Pricing Tests ====================

#[test]
fn test_tranche_pricing_returns_valid_pv() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let result = pricer.price_tranche(&tranche, &market, as_of);

    // Assert
    assert!(result.is_ok(), "Pricing should succeed");
    let pv = result.unwrap();
    assert_eq!(pv.currency(), Currency::USD);
    assert!(pv.amount().is_finite(), "PV should be finite");
}

#[test]
fn test_equity_tranche_pricing() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let tranche = equity_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let result = pricer.price_tranche(&tranche, &market, as_of);

    // Assert
    assert!(result.is_ok());
    let pv = result.unwrap();
    assert!(pv.amount().is_finite());
}

#[test]
fn test_senior_tranche_pricing() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let tranche = senior_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let result = pricer.price_tranche(&tranche, &market, as_of);

    // Assert
    assert!(result.is_ok());
    let pv = result.unwrap();
    assert!(pv.amount().is_finite());
}

#[test]
fn test_mezzanine_tranche_pricing() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let result = pricer.price_tranche(&tranche, &market, as_of);

    // Assert
    assert!(result.is_ok());
    let pv = result.unwrap();
    assert!(pv.amount().is_finite());
}

// ==================== Buy vs Sell Protection Tests ====================

#[test]
fn test_buy_sell_protection_symmetry() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let market = standard_market_context();
    let as_of = base_date();

    let sell_tranche = custom_tranche(3.0, 7.0, 500.0, TrancheSide::SellProtection);
    let buy_tranche = custom_tranche(3.0, 7.0, 500.0, TrancheSide::BuyProtection);

    // Act
    let sell_pv = pricer
        .price_tranche(&sell_tranche, &market, as_of)
        .unwrap()
        .amount();
    let buy_pv = pricer
        .price_tranche(&buy_tranche, &market, as_of)
        .unwrap()
        .amount();

    // Assert
    assert_relative_eq(
        buy_pv,
        -sell_pv,
        0.001,
        "Buy and sell protection should have opposite sign PVs",
    );
}

#[test]
fn test_sell_protection_pv_components() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let tranche = custom_tranche(3.0, 7.0, 500.0, TrancheSide::SellProtection);
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let pv = pricer
        .price_tranche(&tranche, &market, as_of)
        .unwrap()
        .amount();

    // Assert: For sell protection with positive coupon, should receive premium
    // Net PV can be positive or negative depending on spread levels
    assert!(pv.is_finite(), "PV should be finite");
}

// ==================== Maturity Edge Cases ====================

#[test]
fn test_at_maturity_pricing() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let tranche = mezzanine_tranche();
    let market = standard_market_context();

    // Set as_of to maturity date
    let as_of = tranche.maturity;

    // Act
    let result = pricer.price_tranche(&tranche, &market, as_of);

    // Assert: PV should be zero or minimal at maturity
    assert!(result.is_ok());
    let pv = result.unwrap();
    assert_absolute_eq(
        pv.amount(),
        0.0,
        1e-2,
        "At maturity tranche should have minimal PV",
    );
}

// ==================== Notional Scaling Tests ====================

#[test]
fn test_pv_scales_with_notional() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let market = standard_market_context();
    let as_of = base_date();

    let mut tranche_10mm = mezzanine_tranche();
    tranche_10mm.notional = finstack_core::money::Money::new(10_000_000.0, Currency::USD);

    let mut tranche_20mm = mezzanine_tranche();
    tranche_20mm.notional = finstack_core::money::Money::new(20_000_000.0, Currency::USD);

    // Act
    let pv_10 = pricer
        .price_tranche(&tranche_10mm, &market, as_of)
        .unwrap()
        .amount();
    let pv_20 = pricer
        .price_tranche(&tranche_20mm, &market, as_of)
        .unwrap()
        .amount();

    // Assert
    assert_relative_eq(
        pv_20 / pv_10,
        2.0,
        0.001,
        "PV should scale linearly with notional",
    );
}
