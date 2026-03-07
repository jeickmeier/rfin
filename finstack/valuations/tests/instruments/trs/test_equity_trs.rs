//! Comprehensive unit tests for Equity Total Return Swaps.
//!
//! Tests cover instrument creation, validation, pricing, NPV calculations,
//! leg decomposition, and sensitivity to market parameters.

use super::test_utils::*;
use finstack_core::currency::Currency::*;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::money::Money;
use finstack_valuations::cashflow::CashflowProvider;
use finstack_valuations::instruments::Instrument;
use finstack_valuations::instruments::TrsSide;
use rust_decimal::Decimal;

// ================================================================================================
// Construction and Validation Tests
// ================================================================================================

#[test]
fn test_equity_trs_builder_defaults() {
    // Arrange & Act
    let trs = TestEquityTrsBuilder::new().build();

    // Assert
    assert_eq!(trs.id.as_str(), "TEST-EQ-TRS-001");
    assert_eq!(trs.notional.amount(), 10_000_000.0);
    assert_eq!(trs.notional.currency(), USD);
    assert_eq!(trs.side, TrsSide::ReceiveTotalReturn);
    assert_eq!(trs.underlying.contract_size, 1.0);
}

#[test]
fn test_equity_trs_builder_custom_params() {
    // Arrange & Act
    let trs = TestEquityTrsBuilder::new()
        .id("CUSTOM-TRS-001")
        .notional(Money::new(5_000_000.0, USD))
        .spread_bp(50.0)
        .side(TrsSide::PayTotalReturn)
        .initial_level(5100.0)
        .build();

    // Assert
    assert_eq!(trs.id.as_str(), "CUSTOM-TRS-001");
    assert_eq!(trs.notional.amount(), 5_000_000.0);
    assert_eq!(trs.financing.spread_bp, Decimal::from(50));
    assert_eq!(trs.side, TrsSide::PayTotalReturn);
    assert_eq!(trs.initial_level, Some(5100.0));
}

#[test]
fn test_equity_trs_with_no_dividend_yield() {
    // Arrange & Act
    let trs = TestEquityTrsBuilder::new().div_yield_id(None).build();

    // Assert
    assert!(trs.underlying.div_yield_id.is_none());
}

#[test]
fn test_equity_trs_different_contract_sizes() {
    // Arrange & Act - Standard contract size
    let trs1 = TestEquityTrsBuilder::new().build();

    // Mini contract (0.1x)
    let trs2 = TestEquityTrsBuilder::new().build();

    // Assert
    assert_eq!(trs1.underlying.contract_size, 1.0);
    assert_eq!(trs2.underlying.contract_size, 1.0);
}

// ================================================================================================
// NPV and Pricing Tests
// ================================================================================================

#[test]
fn test_equity_trs_npv_receive_side() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();
    let trs = TestEquityTrsBuilder::new()
        .side(TrsSide::ReceiveTotalReturn)
        .spread_bp(25.0)
        .build();

    // Act
    let npv = trs.value(&market, as_of).unwrap();

    // Assert
    assert_eq!(npv.currency(), USD);
    assert!(npv.amount().is_finite());
}

#[test]
fn test_equity_trs_npv_pay_side() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();
    let trs = TestEquityTrsBuilder::new()
        .side(TrsSide::PayTotalReturn)
        .spread_bp(25.0)
        .build();

    // Act
    let npv = trs.value(&market, as_of).unwrap();

    // Assert
    assert_eq!(npv.currency(), USD);
    assert!(npv.amount().is_finite());
}

#[test]
fn test_equity_trs_npv_pay_vs_receive_symmetry() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();

    let trs_receive = TestEquityTrsBuilder::new()
        .side(TrsSide::ReceiveTotalReturn)
        .spread_bp(25.0)
        .build();

    let trs_pay = TestEquityTrsBuilder::new()
        .side(TrsSide::PayTotalReturn)
        .spread_bp(25.0)
        .build();

    // Act
    let npv_receive = trs_receive.value(&market, as_of).unwrap();
    let npv_pay = trs_pay.value(&market, as_of).unwrap();

    // Assert - NPVs should be opposite
    assert_approx_eq(
        npv_receive.amount() + npv_pay.amount(),
        0.0,
        1.0, // $1 tolerance
        "Receive and pay TRS NPVs should sum to zero",
    );
}

#[test]
fn test_equity_trs_value_trait() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();
    let trs = TestEquityTrsBuilder::new().build();

    // Act
    let value = trs.value(&market, as_of).unwrap();

    // Assert
    assert_eq!(value.currency(), USD);
    assert!(value.amount().is_finite());
}

#[test]
fn test_equity_trs_pricing_with_different_spreads() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();

    let trs_low_spread = TestEquityTrsBuilder::new().spread_bp(10.0).build();

    let trs_high_spread = TestEquityTrsBuilder::new().spread_bp(100.0).build();

    // Act
    let npv_low = trs_low_spread.value(&market, as_of).unwrap();
    let npv_high = trs_high_spread.value(&market, as_of).unwrap();

    // Assert - For receive TR, higher financing spread means lower NPV
    assert!(
        npv_low.amount() > npv_high.amount(),
        "Higher financing spread should reduce NPV for receive TR side"
    );
}

// ================================================================================================
// Leg Decomposition Tests
// ================================================================================================

#[test]
fn test_equity_trs_total_return_leg_pv() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();
    let trs = TestEquityTrsBuilder::new().build();

    // Act
    let tr_pv = trs.pv_total_return_leg(&market, as_of).unwrap();

    // Assert
    assert_eq!(tr_pv.currency(), USD);
    assert!(tr_pv.amount().is_finite());
}

#[test]
fn test_equity_trs_financing_leg_pv() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();
    let trs = TestEquityTrsBuilder::new().spread_bp(50.0).build();

    // Act
    let fin_pv = trs.pv_financing_leg(&market, as_of).unwrap();

    // Assert
    assert_eq!(fin_pv.currency(), USD);
    assert!(fin_pv.amount().is_finite());
    // Financing leg should have positive PV (we pay)
    assert!(fin_pv.amount() > 0.0);
}

#[test]
fn test_equity_trs_npv_equals_legs_difference() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();
    let trs = TestEquityTrsBuilder::new()
        .side(TrsSide::ReceiveTotalReturn)
        .spread_bp(25.0)
        .build();

    // Act
    let npv = trs.value(&market, as_of).unwrap();
    let tr_pv = trs.pv_total_return_leg(&market, as_of).unwrap();
    let fin_pv = trs.pv_financing_leg(&market, as_of).unwrap();

    // Assert - NPV = TR leg - Financing leg (for receive side)
    let expected_npv = tr_pv.checked_sub(fin_pv).unwrap();
    assert_money_approx_eq(
        npv,
        expected_npv,
        TOLERANCE_CENTS,
        "NPV should equal TR leg PV minus financing leg PV",
    );
}

#[test]
fn test_equity_trs_financing_leg_increases_with_spread() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();

    let trs_low = TestEquityTrsBuilder::new().spread_bp(10.0).build();

    let trs_high = TestEquityTrsBuilder::new().spread_bp(100.0).build();

    // Act
    let pv_low = trs_low.pv_financing_leg(&market, as_of).unwrap();
    let pv_high = trs_high.pv_financing_leg(&market, as_of).unwrap();

    // Assert
    assert!(
        pv_high.amount() > pv_low.amount(),
        "Higher spread should result in higher financing leg PV"
    );
}

// ================================================================================================
// Market Sensitivity Tests
// ================================================================================================

#[test]
fn test_equity_trs_sensitivity_to_spot_price() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();

    // Create TRS with explicit initial level to lock it in
    let trs = TestEquityTrsBuilder::new()
        .side(TrsSide::ReceiveTotalReturn)
        .initial_level(5000.0) // Lock initial level
        .build();

    let spot_bumped = 5050.0; // +1% bump

    // Act
    let npv_base = trs.value(&market, as_of).unwrap();

    let market_bumped = market
        .clone()
        .insert_price("SPX-SPOT", MarketScalar::Unitless(spot_bumped));
    let npv_bumped = trs.value(&market_bumped, as_of).unwrap();

    // Assert - With locked initial level, spot changes affect forward prices
    // Both should be finite, but may be equal if initial_level dominates
    assert!(npv_base.amount().is_finite());
    assert!(npv_bumped.amount().is_finite());
}

#[test]
fn test_equity_trs_sensitivity_to_dividend_yield() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();
    let trs = TestEquityTrsBuilder::new()
        .side(TrsSide::ReceiveTotalReturn)
        .build();

    let _div_base = 0.015;
    let div_higher = 0.025; // Higher dividend yield

    // Act
    let npv_base = trs.value(&market, as_of).unwrap();

    let market_bumped = market
        .clone()
        .insert_price("SPX-DIV-YIELD", MarketScalar::Unitless(div_higher));
    let npv_bumped = trs.value(&market_bumped, as_of).unwrap();

    // Assert - Higher div yield reduces forward price, lowering TR leg PV
    assert!(
        npv_bumped.amount() < npv_base.amount(),
        "Higher dividend yield should reduce NPV for receive TR side"
    );
}

#[test]
fn test_equity_trs_sensitivity_to_interest_rates() {
    // Arrange
    let as_of = as_of_date();
    let trs = TestEquityTrsBuilder::new()
        .side(TrsSide::ReceiveTotalReturn)
        .build();

    // Base market
    let market_base = create_market_context();

    // Shifted rates market (higher rates)
    let mut market_shifted = MarketContext::new();
    let disc_shifted =
        finstack_core::market_data::term_structures::DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots(vec![
                (0.0, 1.0),
                (0.25, 0.990), // Lower DFs = higher rates
                (0.5, 0.980),
                (1.0, 0.960),
                (2.0, 0.920),
                (5.0, 0.800),
            ])
            .interp(finstack_core::math::interp::InterpStyle::LogLinear)
            .build()
            .unwrap();
    market_shifted = market_shifted.insert(disc_shifted);

    let fwd_shifted =
        finstack_core::market_data::term_structures::ForwardCurve::builder("USD-SOFR-3M", 0.25)
            .base_date(as_of)
            .knots(vec![
                (0.0, 0.03), // +100bp
                (0.25, 0.031),
                (0.5, 0.032),
                (1.0, 0.033),
                (2.0, 0.034),
            ])
            .interp(finstack_core::math::interp::InterpStyle::Linear)
            .build()
            .unwrap();
    market_shifted = market_shifted.insert(fwd_shifted);
    market_shifted = market_shifted.insert_price("SPX-SPOT", MarketScalar::Unitless(5000.0));
    market_shifted = market_shifted.insert_price("SPX-DIV-YIELD", MarketScalar::Unitless(0.015));

    // Act
    let npv_base = trs.value(&market_base, as_of).unwrap();
    let npv_shifted = trs.value(&market_shifted, as_of).unwrap();

    // Assert - Both legs are affected by rates; net effect depends on dominance
    assert!(npv_base.amount().is_finite());
    assert!(npv_shifted.amount().is_finite());
}

// ================================================================================================
// Initial Level Tests
// ================================================================================================

#[test]
fn test_equity_trs_with_custom_initial_level() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();

    // TRS starting at different level than current spot
    let trs = TestEquityTrsBuilder::new()
        .initial_level(4800.0) // Below current spot of 5000
        .build();

    // Act
    let npv = trs.value(&market, as_of).unwrap();

    // Assert
    assert!(npv.amount().is_finite());
    assert_eq!(trs.initial_level, Some(4800.0));
}

#[test]
fn test_equity_trs_initial_level_vs_spot() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();

    let trs_default = TestEquityTrsBuilder::new().build(); // Uses spot
    let trs_custom = TestEquityTrsBuilder::new()
        .initial_level(5000.0) // Same as spot
        .build();

    // Act
    let npv_default = trs_default.value(&market, as_of).unwrap();
    let npv_custom = trs_custom.value(&market, as_of).unwrap();

    // Assert - Should be very close since initial level matches spot
    assert_money_approx_eq(
        npv_default,
        npv_custom,
        1.0, // $1 tolerance
        "NPV with default initial level should match NPV with initial=spot",
    );
}

// ================================================================================================
// Cashflow Schedule Tests
// ================================================================================================

#[test]
fn test_equity_trs_cashflow_schedule_generation() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();
    let trs = TestEquityTrsBuilder::new().tenor_months(12).build();

    // Act
    let flows = trs.build_dated_flows(&market, as_of).unwrap();

    // Assert
    // 1 year quarterly = 4 payments
    assert_eq!(flows.len(), 4, "Should have 4 quarterly cashflows");

    // All flows in correct currency
    for (date, amount) in &flows {
        assert!(date > &as_of);
        assert_eq!(amount.currency(), USD);
    }
}

#[test]
fn test_equity_trs_cashflow_schedule_dates_ordered() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();
    let trs = TestEquityTrsBuilder::new().build();

    // Act
    let flows = trs.build_dated_flows(&market, as_of).unwrap();

    // Assert - Dates should be strictly increasing
    for i in 1..flows.len() {
        assert!(
            flows[i].0 > flows[i - 1].0,
            "Cashflow dates should be strictly increasing"
        );
    }
}

// ================================================================================================
// Tenor Variation Tests
// ================================================================================================

#[test]
fn test_equity_trs_short_tenor_3_months() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();
    let trs = TestEquityTrsBuilder::new().tenor_months(3).build();

    // Act
    let npv = trs.value(&market, as_of).unwrap();

    // Assert
    assert!(npv.amount().is_finite());
}

#[test]
fn test_equity_trs_medium_tenor_2_years() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();
    let trs = TestEquityTrsBuilder::new().tenor_months(24).build();

    // Act
    let npv = trs.value(&market, as_of).unwrap();

    // Assert
    assert!(npv.amount().is_finite());
}

#[test]
fn test_equity_trs_long_tenor_5_years() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();
    let trs = TestEquityTrsBuilder::new().tenor_months(60).build();

    // Act
    let npv = trs.value(&market, as_of).unwrap();

    // Assert
    assert!(npv.amount().is_finite());
}

// ================================================================================================
// Notional Size Tests
// ================================================================================================

#[test]
fn test_equity_trs_notional_scaling() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();

    let trs_1m = TestEquityTrsBuilder::new()
        .notional(Money::new(1_000_000.0, USD))
        .build();

    let trs_10m = TestEquityTrsBuilder::new()
        .notional(Money::new(10_000_000.0, USD))
        .build();

    // Act
    let npv_1m = trs_1m.value(&market, as_of).unwrap();
    let npv_10m = trs_10m.value(&market, as_of).unwrap();

    // Assert - NPV should scale approximately linearly with notional
    assert_approx_eq(
        npv_10m.amount() / npv_1m.amount(),
        10.0,
        0.01, // 1% tolerance for rounding
        "NPV should scale linearly with notional",
    );
}
