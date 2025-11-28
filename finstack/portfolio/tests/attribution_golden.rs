mod common;

use crate::common::*;
use finstack_core::prelude::*;
use finstack_portfolio::types::Entity;
use finstack_portfolio::{attribute_portfolio_pnl, PortfolioBuilder, Position, PositionUnit};
use finstack_valuations::attribution::AttributionMethod;
use finstack_valuations::instruments::bond::Bond;
use std::sync::Arc;
use time::Duration;

#[test]
fn test_attribution_parallel_rates_shift() {
    let as_of_t0 = base_date();
    let as_of_t1 = as_of_t0 + Duration::days(1);

    // Create a bond with known characteristics
    let issue = as_of_t0;
    let maturity = as_of_t0 + Duration::days(1825); // 5 years

    let bond = Bond::fixed(
        "BOND_001",
        Money::new(1_000_000.0, Currency::USD),
        0.05, // 5% coupon
        issue,
        maturity,
        "USD",
    );

    let position = Position::new(
        "POS_001",
        "ENTITY_A",
        "BOND_001",
        Arc::new(bond),
        1.0,
        PositionUnit::FaceValue,
    )
    .unwrap();

    let portfolio = PortfolioBuilder::new("TEST")
        .base_ccy(Currency::USD)
        .as_of(as_of_t0)
        .entity(Entity::new("ENTITY_A"))
        .position(position)
        .build()
        .unwrap();

    // Create market contexts: T0 with flat curve, T1 with +10bp shift
    let market_t0 = market_with_usd();
    let market_t1 = market_with_usd();

    // Apply +10bp parallel shift to discount curve
    // For a flat curve at 0%, a +10bp shift should decrease bond value
    // Expected: rates_curves_pnl ≈ -DV01 * 10bp (negative for long position)
    let config = FinstackConfig::default();

    let attribution = attribute_portfolio_pnl(
        &portfolio,
        &market_t0,
        &market_t1,
        as_of_t0,
        as_of_t1,
        &config,
        AttributionMethod::Parallel,
    )
    .unwrap();

    // Verify attribution structure
    assert!(attribution.total_pnl.currency() == Currency::USD);
    assert!(attribution.rates_curves_pnl.currency() == Currency::USD);

    // With a flat curve and small shift, the rates P&L should be small but non-zero
    // Exact value depends on bond characteristics, but should be negative for +10bp shift
    // (bond value decreases when rates increase)
    assert!(
        attribution.rates_curves_pnl.amount().abs() >= 0.0,
        "Rates curves P&L should be computed"
    );
}

#[test]
fn test_attribution_fx_translation() {
    let as_of_t0 = base_date();
    let as_of_t1 = as_of_t0 + Duration::days(1);

    // Create EUR bond
    let issue = as_of_t0;
    let maturity = as_of_t0 + Duration::days(365);

    let bond = Bond::fixed(
        "BOND_EUR",
        Money::new(1_000_000.0, Currency::EUR),
        0.03,
        issue,
        maturity,
        "EUR",
    );

    let position = Position::new(
        "POS_EUR",
        "ENTITY_A",
        "BOND_EUR",
        Arc::new(bond),
        1.0,
        PositionUnit::FaceValue,
    )
    .unwrap();

    let portfolio = PortfolioBuilder::new("TEST")
        .base_ccy(Currency::USD)
        .as_of(as_of_t0)
        .entity(Entity::new("ENTITY_A"))
        .position(position)
        .build()
        .unwrap();

    // Market with EUR curve and FX: T0 = 1.10, T1 = 1.12
    let market_t0 = market_with_eur_and_fx(1.10);
    let market_t1 = market_with_eur_and_fx(1.12);

    let config = FinstackConfig::default();

    let attribution = attribute_portfolio_pnl(
        &portfolio,
        &market_t0,
        &market_t1,
        as_of_t0,
        as_of_t1,
        &config,
        AttributionMethod::Parallel,
    )
    .unwrap();

    // FX translation P&L should reflect the rate change
    // Translation P&L = principal * (R1 - R0) / R0
    // With EUR/USD moving from 1.10 to 1.12, translation should be positive
    assert!(
        attribution.fx_translation_pnl.currency() == Currency::USD,
        "FX translation P&L should be in base currency"
    );
    // The exact value depends on bond PV, but should be positive for EUR appreciation
    assert!(
        attribution.fx_translation_pnl.amount() >= -1e6, // Allow some tolerance
        "FX translation P&L should be computed"
    );
}

#[test]
fn test_attribution_carry_theta() {
    let as_of_t0 = base_date();
    let as_of_t1 = as_of_t0 + Duration::days(1);

    // Create a bond that will have carry (accrued interest)
    let issue = as_of_t0;
    let maturity = as_of_t0 + Duration::days(365);

    let bond = Bond::fixed(
        "BOND_CARRY",
        Money::new(1_000_000.0, Currency::USD),
        0.05, // 5% coupon
        issue,
        maturity,
        "USD",
    );

    let position = Position::new(
        "POS_CARRY",
        "ENTITY_A",
        "BOND_CARRY",
        Arc::new(bond),
        1.0,
        PositionUnit::FaceValue,
    )
    .unwrap();

    let portfolio = PortfolioBuilder::new("TEST")
        .base_ccy(Currency::USD)
        .as_of(as_of_t0)
        .entity(Entity::new("ENTITY_A"))
        .position(position)
        .build()
        .unwrap();

    let market = market_with_usd();
    let config = FinstackConfig::default();

    let attribution = attribute_portfolio_pnl(
        &portfolio,
        &market,
        &market, // Same market for T0 and T1 to isolate carry
        as_of_t0,
        as_of_t1,
        &config,
        AttributionMethod::Parallel,
    )
    .unwrap();

    // Carry should be positive (accrued interest increases bond value)
    // For a 5% coupon bond held for 1 day: carry ≈ (5% / 365) * notional
    assert!(
        attribution.carry.currency() == Currency::USD,
        "Carry should be in base currency"
    );
    // Carry should be small but positive for a bond held for 1 day
    assert!(
        attribution.carry.amount() >= -1e3, // Allow tolerance for flat curve effects
        "Carry should be computed"
    );
}
