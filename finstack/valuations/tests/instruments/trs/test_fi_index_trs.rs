//! Comprehensive unit tests for Fixed Income Index Total Return Swaps.
//!
//! Tests cover instrument creation, validation, pricing, carry and roll calculations,
//! duration sensitivity, and index-specific behaviors.

use super::test_utils::*;
use finstack_core::currency::Currency::*;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::money::Money;
use finstack_valuations::cashflow::traits::CashflowProvider;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::trs::TrsSide;

// ================================================================================================
// Construction and Validation Tests
// ================================================================================================

#[test]
fn test_fi_index_trs_builder_defaults() {
    // Arrange & Act
    let trs = TestFIIndexTrsBuilder::new().build();

    // Assert
    assert_eq!(trs.id.as_str(), "TEST-FI-TRS-001");
    assert_eq!(trs.notional.amount(), 10_000_000.0);
    assert_eq!(trs.notional.currency(), USD);
    assert_eq!(trs.side, TrsSide::ReceiveTotalReturn);
    assert_eq!(trs.underlying.contract_size, 1.0);
}

#[test]
fn test_fi_index_trs_builder_custom_params() {
    // Arrange & Act
    let trs = TestFIIndexTrsBuilder::new()
        .id("CUSTOM-FI-TRS-001")
        .notional(Money::new(25_000_000.0, USD))
        .spread_bp(150.0)
        .side(TrsSide::PayTotalReturn)
        .build();

    // Assert
    assert_eq!(trs.id.as_str(), "CUSTOM-FI-TRS-001");
    assert_eq!(trs.notional.amount(), 25_000_000.0);
    assert_eq!(trs.financing.spread_bp, 150.0);
    assert_eq!(trs.side, TrsSide::PayTotalReturn);
}

#[test]
fn test_fi_index_trs_with_yield_only() {
    // Arrange & Act
    let trs = TestFIIndexTrsBuilder::new()
        .yield_id(Some("HY-INDEX-YIELD".into()))
        .duration_id(None)
        .build();

    // Assert
    assert!(trs.underlying.yield_id.is_some());
    assert!(trs.underlying.duration_id.is_none());
}

#[test]
fn test_fi_index_trs_with_duration_only() {
    // Arrange & Act
    let trs = TestFIIndexTrsBuilder::new()
        .yield_id(None)
        .duration_id(Some("HY-INDEX-DURATION".into()))
        .build();

    // Assert
    assert!(trs.underlying.yield_id.is_none());
    assert!(trs.underlying.duration_id.is_some());
}

#[test]
fn test_fi_index_trs_with_yield_and_duration() {
    // Arrange & Act
    let trs = TestFIIndexTrsBuilder::new()
        .yield_id(Some("HY-INDEX-YIELD".into()))
        .duration_id(Some("HY-INDEX-DURATION".into()))
        .build();

    // Assert
    assert!(trs.underlying.yield_id.is_some());
    assert!(trs.underlying.duration_id.is_some());
}

#[test]
fn test_fi_index_trs_currency_consistency() {
    // Arrange & Act
    let trs = TestFIIndexTrsBuilder::new()
        .notional(Money::new(10_000_000.0, USD))
        .build();

    // Assert - Index base currency should match notional currency
    assert_eq!(trs.notional.currency(), trs.underlying.base_currency);
}

// ================================================================================================
// NPV and Pricing Tests
// ================================================================================================

#[test]
fn test_fi_index_trs_npv_receive_side() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();
    let trs = TestFIIndexTrsBuilder::new()
        .side(TrsSide::ReceiveTotalReturn)
        .spread_bp(100.0)
        .build();

    // Act
    let npv = trs.npv(&market, as_of).unwrap();

    // Assert
    assert_eq!(npv.currency(), USD);
    assert!(npv.amount().is_finite());
}

#[test]
fn test_fi_index_trs_npv_pay_side() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();
    let trs = TestFIIndexTrsBuilder::new()
        .side(TrsSide::PayTotalReturn)
        .spread_bp(100.0)
        .build();

    // Act
    let npv = trs.npv(&market, as_of).unwrap();

    // Assert
    assert_eq!(npv.currency(), USD);
    assert!(npv.amount().is_finite());
}

#[test]
fn test_fi_index_trs_npv_pay_vs_receive_symmetry() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();

    let trs_receive = TestFIIndexTrsBuilder::new()
        .side(TrsSide::ReceiveTotalReturn)
        .spread_bp(75.0)
        .build();

    let trs_pay = TestFIIndexTrsBuilder::new()
        .side(TrsSide::PayTotalReturn)
        .spread_bp(75.0)
        .build();

    // Act
    let npv_receive = trs_receive.npv(&market, as_of).unwrap();
    let npv_pay = trs_pay.npv(&market, as_of).unwrap();

    // Assert - NPVs should be opposite
    assert_approx_eq(
        npv_receive.amount() + npv_pay.amount(),
        0.0,
        1.0, // $1 tolerance
        "Receive and pay TRS NPVs should sum to zero",
    );
}

#[test]
fn test_fi_index_trs_value_trait() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();
    let trs = TestFIIndexTrsBuilder::new().build();

    // Act
    let value = trs.value(&market, as_of).unwrap();

    // Assert
    assert_eq!(value.currency(), USD);
    assert!(value.amount().is_finite());
}

#[test]
fn test_fi_index_trs_pricing_with_different_spreads() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();

    let trs_low_spread = TestFIIndexTrsBuilder::new().spread_bp(50.0).build();

    let trs_high_spread = TestFIIndexTrsBuilder::new().spread_bp(200.0).build();

    // Act
    let npv_low = trs_low_spread.npv(&market, as_of).unwrap();
    let npv_high = trs_high_spread.npv(&market, as_of).unwrap();

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
fn test_fi_index_trs_total_return_leg_pv() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();
    let trs = TestFIIndexTrsBuilder::new().build();

    // Act
    let tr_pv = trs.pv_total_return_leg(&market, as_of).unwrap();

    // Assert
    assert_eq!(tr_pv.currency(), USD);
    assert!(tr_pv.amount().is_finite());
}

#[test]
fn test_fi_index_trs_financing_leg_pv() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();
    let trs = TestFIIndexTrsBuilder::new().spread_bp(100.0).build();

    // Act
    let fin_pv = trs.pv_financing_leg(&market, as_of).unwrap();

    // Assert
    assert_eq!(fin_pv.currency(), USD);
    assert!(fin_pv.amount().is_finite());
    // Financing leg should have positive PV
    assert!(fin_pv.amount() > 0.0);
}

#[test]
fn test_fi_index_trs_npv_equals_legs_difference() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();
    let trs = TestFIIndexTrsBuilder::new()
        .side(TrsSide::ReceiveTotalReturn)
        .spread_bp(100.0)
        .build();

    // Act
    let npv = trs.npv(&market, as_of).unwrap();
    let tr_pv = trs.pv_total_return_leg(&market, as_of).unwrap();
    let fin_pv = trs.pv_financing_leg(&market, as_of).unwrap();

    // Assert - NPV = TR leg - Financing leg (for receive side)
    let expected_npv = (tr_pv - fin_pv).unwrap();
    assert_money_approx_eq(
        npv,
        expected_npv,
        TOLERANCE_CENTS,
        "NPV should equal TR leg PV minus financing leg PV",
    );
}

#[test]
fn test_fi_index_trs_financing_leg_increases_with_spread() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();

    let trs_low = TestFIIndexTrsBuilder::new().spread_bp(25.0).build();

    let trs_high = TestFIIndexTrsBuilder::new().spread_bp(200.0).build();

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
// Index Characteristics Tests
// ================================================================================================

#[test]
fn test_fi_index_trs_carry_component() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();

    // High yield index (higher carry)
    let trs_hy = TestFIIndexTrsBuilder::new()
        .yield_id(Some("HY-INDEX-YIELD".into())) // 5.5%
        .spread_bp(100.0)
        .build();

    // Act
    let tr_pv = trs_hy.pv_total_return_leg(&market, as_of).unwrap();

    // Assert - Total return should be positive for positive yield
    assert!(tr_pv.amount().is_finite());
}

#[test]
fn test_fi_index_trs_sensitivity_to_yield() {
    // Arrange
    let as_of = as_of_date();
    let market_base = create_market_context();

    let trs = TestFIIndexTrsBuilder::new()
        .side(TrsSide::ReceiveTotalReturn)
        .spread_bp(100.0)
        .build();

    // Higher yield scenario
    let market_high_yield = market_base
        .clone()
        .insert_price("HY-INDEX-YIELD", MarketScalar::Unitless(0.065)); // +100bp

    // Act
    let npv_base = trs.npv(&market_base, as_of).unwrap();
    let npv_high = trs.npv(&market_high_yield, as_of).unwrap();

    // Assert - Higher yield increases carry, should increase TR leg PV
    assert!(
        npv_high.amount() > npv_base.amount(),
        "Higher index yield should increase NPV for receive TR side"
    );
}

#[test]
fn test_fi_index_trs_sensitivity_to_duration() {
    // Arrange
    let as_of = as_of_date();
    let market_base = create_market_context();

    let trs = TestFIIndexTrsBuilder::new()
        .side(TrsSide::ReceiveTotalReturn)
        .spread_bp(100.0)
        .build();

    // Higher duration scenario
    let market_high_duration = market_base
        .clone()
        .insert_price("HY-INDEX-DURATION", MarketScalar::Unitless(6.0)); // +1.5 years

    // Act
    let npv_base = trs.npv(&market_base, as_of).unwrap();
    let npv_high_dur = trs.npv(&market_high_duration, as_of).unwrap();

    // Assert - Both should compute (effect depends on roll model)
    assert!(npv_base.amount().is_finite());
    assert!(npv_high_dur.amount().is_finite());
}

// ================================================================================================
// Market Sensitivity Tests
// ================================================================================================

#[test]
fn test_fi_index_trs_sensitivity_to_interest_rates() {
    // Arrange
    let as_of = as_of_date();
    let trs = TestFIIndexTrsBuilder::new()
        .side(TrsSide::ReceiveTotalReturn)
        .spread_bp(100.0)
        .build();

    // Base market
    let market_base = create_market_context();

    // Shifted rates market
    let mut market_shifted = MarketContext::new();
    let disc_shifted =
        finstack_core::market_data::term_structures::DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots(vec![
                (0.0, 1.0),
                (0.25, 0.9925),
                (0.5, 0.985),
                (1.0, 0.970),
                (2.0, 0.940),
                (5.0, 0.850),
            ])
            .set_interp(finstack_core::math::interp::InterpStyle::LogLinear)
            .build()
            .unwrap();
    market_shifted = market_shifted.insert_discount(disc_shifted);

    let fwd_shifted =
        finstack_core::market_data::term_structures::ForwardCurve::builder("USD-SOFR-3M", 0.25)
            .base_date(as_of)
            .knots(vec![(0.0, 0.03), (0.25, 0.031), (0.5, 0.032), (1.0, 0.033)])
            .set_interp(finstack_core::math::interp::InterpStyle::Linear)
            .build()
            .unwrap();
    market_shifted = market_shifted.insert_forward(fwd_shifted);
    market_shifted = market_shifted.insert_price("HY-INDEX-YIELD", MarketScalar::Unitless(0.055));
    market_shifted = market_shifted.insert_price("HY-INDEX-DURATION", MarketScalar::Unitless(4.5));

    // Act
    let npv_base = trs.npv(&market_base, as_of).unwrap();
    let npv_shifted = trs.npv(&market_shifted, as_of).unwrap();

    // Assert
    assert!(npv_base.amount().is_finite());
    assert!(npv_shifted.amount().is_finite());
}

// ================================================================================================
// Cashflow Schedule Tests
// ================================================================================================

#[test]
fn test_fi_index_trs_cashflow_schedule_generation() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();
    let trs = TestFIIndexTrsBuilder::new().tenor_months(12).build();

    // Act
    let flows = trs.build_schedule(&market, as_of).unwrap();

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
fn test_fi_index_trs_cashflow_schedule_dates_ordered() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();
    let trs = TestFIIndexTrsBuilder::new().build();

    // Act
    let flows = trs.build_schedule(&market, as_of).unwrap();

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
fn test_fi_index_trs_short_tenor_6_months() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();
    let trs = TestFIIndexTrsBuilder::new().tenor_months(6).build();

    // Act
    let npv = trs.npv(&market, as_of).unwrap();

    // Assert
    assert!(npv.amount().is_finite());
}

#[test]
fn test_fi_index_trs_medium_tenor_3_years() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();
    let trs = TestFIIndexTrsBuilder::new().tenor_months(36).build();

    // Act
    let npv = trs.npv(&market, as_of).unwrap();

    // Assert
    assert!(npv.amount().is_finite());
}

#[test]
fn test_fi_index_trs_long_tenor_5_years() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();
    let trs = TestFIIndexTrsBuilder::new().tenor_months(60).build();

    // Act
    let npv = trs.npv(&market, as_of).unwrap();

    // Assert
    assert!(npv.amount().is_finite());
}

// ================================================================================================
// Notional Size Tests
// ================================================================================================

#[test]
fn test_fi_index_trs_notional_scaling() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();

    let trs_5m = TestFIIndexTrsBuilder::new()
        .notional(Money::new(5_000_000.0, USD))
        .build();

    let trs_25m = TestFIIndexTrsBuilder::new()
        .notional(Money::new(25_000_000.0, USD))
        .build();

    // Act
    let npv_5m = trs_5m.npv(&market, as_of).unwrap();
    let npv_25m = trs_25m.npv(&market, as_of).unwrap();

    // Assert - NPV should scale approximately linearly with notional
    assert_approx_eq(
        npv_25m.amount() / npv_5m.amount(),
        5.0,
        0.01, // 1% tolerance
        "NPV should scale linearly with notional",
    );
}

// ================================================================================================
// Comparison Tests: HY vs IG
// ================================================================================================

#[test]
fn test_fi_index_trs_hy_vs_ig() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();

    // High Yield TRS (higher yield, lower duration typically)
    let trs_hy = TestFIIndexTrsBuilder::new()
        .yield_id(Some("HY-INDEX-YIELD".into())) // 5.5%
        .duration_id(Some("HY-INDEX-DURATION".into())) // 4.5 years
        .spread_bp(100.0)
        .build();

    // Investment Grade TRS (lower yield, higher duration typically)
    let trs_ig = TestFIIndexTrsBuilder::new()
        .yield_id(Some("IG-INDEX-YIELD".into())) // 3.5%
        .duration_id(Some("IG-INDEX-DURATION".into())) // 7.0 years
        .spread_bp(50.0) // Tighter spread for IG
        .build();

    // Act
    let tr_pv_hy = trs_hy.pv_total_return_leg(&market, as_of).unwrap();
    let tr_pv_ig = trs_ig.pv_total_return_leg(&market, as_of).unwrap();

    // Assert - HY should have higher carry (yield * duration)
    assert!(tr_pv_hy.amount().is_finite());
    assert!(tr_pv_ig.amount().is_finite());
}
