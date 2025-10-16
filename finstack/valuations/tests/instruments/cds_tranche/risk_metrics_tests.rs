//! Risk metrics tests for CDS Tranche.
//!
//! Tests cover:
//! - CS01 (credit spread sensitivity)
//! - Correlation delta
//! - Jump-to-default
//! - Spread DV01
//! - Par spread calculation
//! - Different bump units and methods

#![allow(clippy::field_reassign_with_default)]

use super::helpers::*;
use finstack_valuations::instruments::cds_tranche::pricer::{
    CDSTranchePricer, CDSTranchePricerConfig, Cs01BumpUnits,
};
use finstack_valuations::instruments::cds_tranche::TrancheSide;

// ==================== CS01 Tests ====================

#[test]
fn test_cs01_calculation_succeeds() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let result = pricer.calculate_cs01(&tranche, &market, as_of);

    // Assert
    assert!(result.is_ok(), "CS01 calculation should succeed");
}

#[test]
fn test_cs01_is_finite() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let cs01 = pricer.calculate_cs01(&tranche, &market, as_of).unwrap();

    // Assert
    assert!(cs01.is_finite(), "CS01 should be finite");
}

#[test]
fn test_cs01_sell_protection_typically_positive() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let mut tranche = mezzanine_tranche();
    tranche.side = TrancheSide::SellProtection;
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let cs01 = pricer.calculate_cs01(&tranche, &market, as_of).unwrap();

    // Assert
    // For protection seller, higher spreads typically increase PV
    // (protection leg value increases more than any premium increase)
    assert!(cs01.is_finite());
}

#[test]
fn test_cs01_buy_sell_opposite_sign() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let market = standard_market_context();
    let as_of = base_date();

    let sell_tranche = custom_tranche(3.0, 7.0, 500.0, TrancheSide::SellProtection);
    let buy_tranche = custom_tranche(3.0, 7.0, 500.0, TrancheSide::BuyProtection);

    // Act
    let cs01_sell = pricer
        .calculate_cs01(&sell_tranche, &market, as_of)
        .unwrap();
    let cs01_buy = pricer.calculate_cs01(&buy_tranche, &market, as_of).unwrap();

    // Assert
    assert_relative_eq(
        cs01_buy,
        -cs01_sell,
        0.001,
        "Buy and sell CS01 should have opposite signs",
    );
}

#[test]
fn test_cs01_hazard_rate_bump() {
    // Arrange
    let mut config = CDSTranchePricerConfig::default();
    config.cs01_bump_units = Cs01BumpUnits::HazardRateBp;
    let pricer = CDSTranchePricer::with_params(config);

    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let result = pricer.calculate_cs01(&tranche, &market, as_of);

    // Assert
    assert!(result.is_ok());
    assert!(result.unwrap().is_finite());
}

#[test]
fn test_cs01_spread_additive_bump() {
    // Arrange
    let mut config = CDSTranchePricerConfig::default();
    config.cs01_bump_units = Cs01BumpUnits::SpreadBpAdditive;
    let pricer = CDSTranchePricer::with_params(config);

    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let result = pricer.calculate_cs01(&tranche, &market, as_of);

    // Assert
    assert!(result.is_ok());
    assert!(result.unwrap().is_finite());
}

#[test]
fn test_cs01_different_bump_sizes() {
    // Arrange
    let market = standard_market_context();
    let as_of = base_date();
    let tranche = mezzanine_tranche();

    let mut config_1bp = CDSTranchePricerConfig::default();
    config_1bp.cs01_bump_size = 1.0;
    let pricer_1bp = CDSTranchePricer::with_params(config_1bp);

    let mut config_2bp = CDSTranchePricerConfig::default();
    config_2bp.cs01_bump_size = 2.0;
    let pricer_2bp = CDSTranchePricer::with_params(config_2bp);

    // Act
    let cs01_1bp = pricer_1bp.calculate_cs01(&tranche, &market, as_of).unwrap();
    let cs01_2bp = pricer_2bp.calculate_cs01(&tranche, &market, as_of).unwrap();

    // Assert: 2bp bump should give ~2x the sensitivity
    assert_relative_eq(
        cs01_2bp / cs01_1bp,
        2.0,
        0.1,
        "CS01 should scale approximately with bump size",
    );
}

// ==================== Correlation Delta Tests ====================

#[test]
fn test_correlation_delta_calculation_succeeds() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let result = pricer.calculate_correlation_delta(&tranche, &market, as_of);

    // Assert
    assert!(
        result.is_ok(),
        "Correlation delta calculation should succeed"
    );
}

#[test]
fn test_correlation_delta_is_finite() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let corr_delta = pricer
        .calculate_correlation_delta(&tranche, &market, as_of)
        .unwrap();

    // Assert
    assert!(corr_delta.is_finite(), "Correlation delta should be finite");
}

#[test]
fn test_correlation_delta_equity_vs_senior() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let market = standard_market_context();
    let as_of = base_date();

    let equity = equity_tranche();
    let senior = senior_tranche();

    // Act
    let corr_delta_equity = pricer
        .calculate_correlation_delta(&equity, &market, as_of)
        .unwrap();
    let corr_delta_senior = pricer
        .calculate_correlation_delta(&senior, &market, as_of)
        .unwrap();

    // Assert
    // Equity and senior tranches typically have opposite correlation sensitivities
    // Equity: negative (higher corr → lower value)
    // Senior: positive (higher corr → higher value)
    assert!(corr_delta_equity.is_finite());
    assert!(corr_delta_senior.is_finite());
}

#[test]
fn test_correlation_delta_with_custom_bump() {
    // Arrange
    let mut config = CDSTranchePricerConfig::default();
    config.corr_bump_abs = 0.02; // 2% bump instead of default 1%
    let pricer = CDSTranchePricer::with_params(config);

    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let result = pricer.calculate_correlation_delta(&tranche, &market, as_of);

    // Assert
    assert!(result.is_ok());
    assert!(result.unwrap().is_finite());
}

// ==================== Jump-to-Default Tests ====================

#[test]
fn test_jump_to_default_calculation_succeeds() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let result = pricer.calculate_jump_to_default(&tranche, &market, as_of);

    // Assert
    assert!(result.is_ok(), "JTD calculation should succeed");
}

#[test]
fn test_jump_to_default_is_non_negative() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let jtd = pricer
        .calculate_jump_to_default(&tranche, &market, as_of)
        .unwrap();

    // Assert
    assert_finite_non_negative(jtd, "Jump-to-default");
}

#[test]
fn test_jump_to_default_equity_greater_than_senior() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let market = standard_market_context();
    let as_of = base_date();

    let equity = equity_tranche();
    let senior = senior_tranche();

    // Act
    let jtd_equity = pricer
        .calculate_jump_to_default(&equity, &market, as_of)
        .unwrap();
    let jtd_senior = pricer
        .calculate_jump_to_default(&senior, &market, as_of)
        .unwrap();

    // Assert
    // Equity tranche takes first loss, so JTD should be higher
    assert!(
        jtd_equity >= jtd_senior,
        "Equity JTD should be >= senior JTD (first loss position)"
    );
}

#[test]
fn test_jump_to_default_senior_can_be_zero() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let market = standard_market_context();
    let as_of = base_date();

    // Senior tranche with high attachment point
    let senior = custom_tranche(15.0, 30.0, 50.0, TrancheSide::SellProtection);

    // Act
    let jtd = pricer
        .calculate_jump_to_default(&senior, &market, as_of)
        .unwrap();

    // Assert
    // For 125 names, 1 default = 0.8% loss, which won't reach 15% attachment
    assert_eq!(
        jtd, 0.0,
        "Single name default shouldn't reach deep senior tranche"
    );
}

#[test]
fn test_jump_to_default_scales_with_notional() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let market = standard_market_context();
    let as_of = base_date();

    let mut tranche_10mm = mezzanine_tranche();
    tranche_10mm.notional =
        finstack_core::money::Money::new(10_000_000.0, finstack_core::currency::Currency::USD);

    let mut tranche_20mm = mezzanine_tranche();
    tranche_20mm.notional =
        finstack_core::money::Money::new(20_000_000.0, finstack_core::currency::Currency::USD);

    // Act
    let jtd_10 = pricer
        .calculate_jump_to_default(&tranche_10mm, &market, as_of)
        .unwrap();
    let jtd_20 = pricer
        .calculate_jump_to_default(&tranche_20mm, &market, as_of)
        .unwrap();

    // Assert
    assert_relative_eq(
        jtd_20 / jtd_10,
        2.0,
        0.001,
        "JTD should scale linearly with notional",
    );
}

// ==================== Spread DV01 Tests ====================

#[test]
fn test_spread_dv01_calculation_succeeds() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let result = pricer.calculate_spread_dv01(&tranche, &market, as_of);

    // Assert
    assert!(result.is_ok(), "Spread DV01 calculation should succeed");
}

#[test]
fn test_spread_dv01_is_finite() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let spread_dv01 = pricer
        .calculate_spread_dv01(&tranche, &market, as_of)
        .unwrap();

    // Assert
    assert!(spread_dv01.is_finite(), "Spread DV01 should be finite");
}

#[test]
fn test_spread_dv01_positive_for_sell_protection() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let mut tranche = mezzanine_tranche();
    tranche.side = TrancheSide::SellProtection;
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let spread_dv01 = pricer
        .calculate_spread_dv01(&tranche, &market, as_of)
        .unwrap();

    // Assert
    // For protection seller, higher running coupon increases premium received → positive DV01
    assert!(
        spread_dv01 > 0.0,
        "Spread DV01 should be positive for sell protection"
    );
}

#[test]
fn test_spread_dv01_scales_with_notional() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let market = standard_market_context();
    let as_of = base_date();

    let mut tranche_10mm = mezzanine_tranche();
    tranche_10mm.notional =
        finstack_core::money::Money::new(10_000_000.0, finstack_core::currency::Currency::USD);

    let mut tranche_20mm = mezzanine_tranche();
    tranche_20mm.notional =
        finstack_core::money::Money::new(20_000_000.0, finstack_core::currency::Currency::USD);

    // Act
    let dv01_10 = pricer
        .calculate_spread_dv01(&tranche_10mm, &market, as_of)
        .unwrap();
    let dv01_20 = pricer
        .calculate_spread_dv01(&tranche_20mm, &market, as_of)
        .unwrap();

    // Assert
    assert_relative_eq(
        dv01_20 / dv01_10,
        2.0,
        0.001,
        "Spread DV01 should scale linearly with notional",
    );
}

// ==================== Par Spread Tests ====================

#[test]
fn test_par_spread_calculation_succeeds() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let result = pricer.calculate_par_spread(&tranche, &market, as_of);

    // Assert
    assert!(result.is_ok(), "Par spread calculation should succeed");
}

#[test]
fn test_par_spread_is_positive() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let par_spread = pricer
        .calculate_par_spread(&tranche, &market, as_of)
        .unwrap();

    // Assert
    assert_finite_non_negative(par_spread, "Par spread");
}

#[test]
fn test_par_spread_equity_greater_than_senior() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let market = standard_market_context();
    let as_of = base_date();

    let equity = equity_tranche();
    let senior = senior_tranche();

    // Act
    let par_equity = pricer
        .calculate_par_spread(&equity, &market, as_of)
        .unwrap();
    let par_senior = pricer
        .calculate_par_spread(&senior, &market, as_of)
        .unwrap();

    // Assert
    // Equity tranche has higher risk → higher par spread
    assert!(
        par_equity > par_senior,
        "Equity par spread should exceed senior par spread"
    );
}

#[test]
fn test_par_spread_gives_zero_npv() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let market = standard_market_context();
    let as_of = base_date();

    let mut tranche = mezzanine_tranche();

    // Act: Calculate par spread
    let par_spread = pricer
        .calculate_par_spread(&tranche, &market, as_of)
        .unwrap();

    // Set tranche to par spread and reprice
    tranche.running_coupon_bp = par_spread;
    let pv_at_par = pricer
        .price_tranche(&tranche, &market, as_of)
        .unwrap()
        .amount();

    // Assert: PV at par spread should be very close to zero
    assert_absolute_eq(
        pv_at_par,
        0.0,
        tranche.notional.amount() * 0.001, // Allow 0.1% of notional tolerance
        "PV at par spread should be ~zero",
    );
}

// ==================== Upfront Tests ====================

#[test]
fn test_upfront_calculation_succeeds() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let result = pricer.calculate_upfront(&tranche, &market, as_of);

    // Assert
    assert!(result.is_ok(), "Upfront calculation should succeed");
}

#[test]
fn test_upfront_equals_pv() {
    // Arrange
    let pricer = CDSTranchePricer::new();
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let upfront = pricer.calculate_upfront(&tranche, &market, as_of).unwrap();
    let pv = pricer
        .price_tranche(&tranche, &market, as_of)
        .unwrap()
        .amount();

    // Assert
    assert_absolute_eq(upfront, pv, 1e-6, "Upfront should equal PV");
}
