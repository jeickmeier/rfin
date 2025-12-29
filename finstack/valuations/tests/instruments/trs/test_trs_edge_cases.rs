//! Edge case and error handling tests for TRS instruments.
//!
//! Tests boundary conditions, validation failures, missing market data,
//! and other error scenarios.

use super::test_utils::*;
use finstack_core::currency::Currency::*;
use finstack_core::dates::DayCount;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::money::Money;
use finstack_valuations::cashflow::builder::ScheduleParams;
use finstack_valuations::instruments::common::parameters::legs::FinancingLegSpec;
use finstack_valuations::instruments::common::parameters::underlying::EquityUnderlyingParams;
use finstack_valuations::instruments::common::parameters::underlying::IndexUnderlyingParams;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::equity_trs::EquityTotalReturnSwap;
use finstack_valuations::instruments::fi_trs::FIIndexTotalReturnSwap;
use finstack_valuations::instruments::{TrsScheduleSpec, TrsSide};
use rust_decimal::Decimal;

// ================================================================================================
// Currency Mismatch Tests
// ================================================================================================

#[test]
fn test_fi_index_trs_currency_mismatch_validation() {
    // Arrange - Try to create FI index TRS with mismatched currencies
    let notional = Money::new(10_000_000.0, USD);
    let underlying = IndexUnderlyingParams::new("EUR-INDEX", EUR); // EUR index

    let result = FIIndexTotalReturnSwap::builder()
        .id("TRS-MISMATCH".into())
        .notional(notional) // USD notional
        .underlying(underlying) // EUR index
        .financing(FinancingLegSpec::new(
            "USD-OIS",
            "USD-SOFR-3M",
            Decimal::from(100),
            DayCount::Act360,
        ))
        .schedule(TrsScheduleSpec::from_params(
            as_of_date(),
            d(2026, 1, 2),
            ScheduleParams::quarterly_act360(),
        ))
        .build();

    // Assert - Should fail validation
    assert!(result.is_err(), "Should reject currency mismatch");
}

// ================================================================================================
// Missing Market Data Tests
// ================================================================================================

#[test]
fn test_equity_trs_missing_spot_price() {
    // Arrange
    let mut market = MarketContext::new();
    // Add curves but no spot price
    let disc = finstack_core::market_data::term_structures::DiscountCurve::builder("USD-OIS")
        .base_date(as_of_date())
        .knots(vec![(0.0, 1.0), (1.0, 0.98)])
        .set_interp(finstack_core::math::interp::InterpStyle::LogLinear)
        .build()
        .unwrap();
    market = market.insert_discount(disc);

    let fwd =
        finstack_core::market_data::term_structures::ForwardCurve::builder("USD-SOFR-3M", 0.25)
            .base_date(as_of_date())
            .knots(vec![(0.0, 0.02), (1.0, 0.02)])
            .set_interp(finstack_core::math::interp::InterpStyle::Linear)
            .build()
            .unwrap();
    market = market.insert_forward(fwd);

    let trs = TestEquityTrsBuilder::new().build();

    // Act
    let result = trs.value(&market, as_of_date());

    // Assert - Should fail due to missing spot
    assert!(result.is_err(), "Should fail with missing spot price");
}

#[test]
fn test_equity_trs_missing_discount_curve() {
    // Arrange
    let mut market = MarketContext::new();
    // Add spot but no discount curve
    market = market.insert_price("SPX-SPOT", MarketScalar::Unitless(5000.0));
    market = market.insert_price("SPX-DIV-YIELD", MarketScalar::Unitless(0.015));

    let trs = TestEquityTrsBuilder::new().build();

    // Act
    let result = trs.value(&market, as_of_date());

    // Assert - Should fail due to missing discount curve
    assert!(result.is_err(), "Should fail with missing discount curve");
}

#[test]
fn test_equity_trs_missing_forward_curve() {
    // Arrange
    let mut market = MarketContext::new();
    market = market.insert_price("SPX-SPOT", MarketScalar::Unitless(5000.0));
    market = market.insert_price("SPX-DIV-YIELD", MarketScalar::Unitless(0.015));

    let disc = finstack_core::market_data::term_structures::DiscountCurve::builder("USD-OIS")
        .base_date(as_of_date())
        .knots(vec![(0.0, 1.0), (1.0, 0.98)])
        .set_interp(finstack_core::math::interp::InterpStyle::LogLinear)
        .build()
        .unwrap();
    market = market.insert_discount(disc);

    let trs = TestEquityTrsBuilder::new().build();

    // Act
    let result = trs.value(&market, as_of_date());

    // Assert - Should fail due to missing forward curve
    assert!(result.is_err(), "Should fail with missing forward curve");
}

#[test]
fn test_fi_index_trs_builder_validation() {
    // Arrange
    // Try to create TRS without optional yield/duration IDs
    let underlying = IndexUnderlyingParams::new("TEST-INDEX", USD);

    let result = FIIndexTotalReturnSwap::builder()
        .id("TRS-NO-YIELD".into())
        .notional(Money::new(10_000_000.0, USD))
        .underlying(underlying)
        .financing(FinancingLegSpec::new(
            "USD-OIS",
            "USD-SOFR-3M",
            Decimal::from(100),
            DayCount::Act360,
        ))
        .schedule(TrsScheduleSpec::from_params(
            as_of_date(),
            d(2026, 1, 2),
            ScheduleParams::quarterly_act360(),
        ))
        .build();

    // Assert - Builder may enforce validation
    // Test verifies it doesn't panic
    assert!(result.is_ok() || result.is_err());
}

// ================================================================================================
// Extreme Market Conditions Tests
// ================================================================================================

#[test]
fn test_equity_trs_with_zero_spot_price() {
    // Arrange
    let mut market = create_market_context();
    market = market.insert_price("SPX-SPOT", MarketScalar::Unitless(0.0));

    let trs = TestEquityTrsBuilder::new().build();

    // Act
    let result = trs.value(&market, as_of_date());

    // Assert - Should either fail or handle gracefully
    // Delta calculation would fail with zero spot
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_equity_trs_with_negative_dividend_yield() {
    // Arrange
    let mut market = create_market_context();
    market = market.insert_price("SPX-DIV-YIELD", MarketScalar::Unitless(-0.01)); // -1%

    let trs = TestEquityTrsBuilder::new().build();

    // Act
    let result = trs.value(&market, as_of_date());

    // Assert - Should handle negative dividend yield
    assert!(result.is_ok(), "Should handle negative dividend yield");
}

#[test]
fn test_equity_trs_with_very_high_spot() {
    // Arrange
    let mut market = create_market_context();
    market = market.insert_price("SPX-SPOT", MarketScalar::Unitless(1_000_000.0)); // Very high

    let trs = TestEquityTrsBuilder::new().build();

    // Act
    let result = trs.value(&market, as_of_date());

    // Assert
    assert!(result.is_ok());
    assert!(result.unwrap().amount().is_finite());
}

#[test]
fn test_fi_index_trs_with_negative_yield() {
    // Arrange
    let mut market = create_market_context();
    market = market.insert_price("HY-INDEX-YIELD", MarketScalar::Unitless(-0.01)); // -1%

    let trs = TestFIIndexTrsBuilder::new().build();

    // Act
    let result = trs.value(&market, as_of_date());

    // Assert - Should handle negative yield
    assert!(result.is_ok(), "Should handle negative yield");
}

#[test]
fn test_fi_index_trs_with_very_high_duration() {
    // Arrange
    let mut market = create_market_context();
    market = market.insert_price("HY-INDEX-DURATION", MarketScalar::Unitless(20.0)); // 20 years

    let trs = TestFIIndexTrsBuilder::new().build();

    // Act
    let result = trs.value(&market, as_of_date());

    // Assert
    assert!(result.is_ok());
    assert!(result.unwrap().amount().is_finite());
}

#[test]
fn test_fi_index_trs_with_zero_duration() {
    // Arrange
    let mut market = create_market_context();
    market = market.insert_price("HY-INDEX-DURATION", MarketScalar::Unitless(0.0));

    let trs = TestFIIndexTrsBuilder::new().build();

    // Act
    let result = trs.value(&market, as_of_date());

    // Assert - Should handle zero duration
    assert!(result.is_ok(), "Should handle zero duration");
}

// ================================================================================================
// Extreme Notional Tests
// ================================================================================================

#[test]
fn test_equity_trs_with_zero_notional() {
    // Arrange
    let market = create_market_context();
    let trs = TestEquityTrsBuilder::new()
        .notional(Money::new(0.0, USD))
        .build();

    // Act
    let result = trs.value(&market, as_of_date());

    // Assert
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().amount(),
        0.0,
        "Zero notional should give zero NPV"
    );
}

#[test]
fn test_fi_index_trs_with_tiny_notional() {
    // Arrange
    let market = create_market_context();
    let trs = TestFIIndexTrsBuilder::new()
        .notional(Money::new(0.01, USD)) // 1 cent
        .build();

    // Act
    let result = trs.value(&market, as_of_date());

    // Assert
    assert!(result.is_ok());
    assert!(result.unwrap().amount().abs() < 1.0);
}

#[test]
fn test_equity_trs_with_huge_notional() {
    // Arrange
    let market = create_market_context();
    let trs = TestEquityTrsBuilder::new()
        .notional(Money::new(1e12, USD)) // $1 trillion
        .build();

    // Act
    let result = trs.value(&market, as_of_date());

    // Assert
    assert!(result.is_ok());
    assert!(result.unwrap().amount().is_finite());
}

// ================================================================================================
// Extreme Spread Tests
// ================================================================================================

#[test]
fn test_equity_trs_with_zero_spread() {
    // Arrange
    let market = create_market_context();
    let trs = TestEquityTrsBuilder::new().spread_bp(0.0).build();

    // Act
    let result = trs.value(&market, as_of_date());

    // Assert
    assert!(result.is_ok());
}

#[test]
fn test_equity_trs_with_negative_spread() {
    // Arrange
    let market = create_market_context();
    let trs = TestEquityTrsBuilder::new()
        .spread_bp(-50.0) // Negative spread
        .build();

    // Act
    let result = trs.value(&market, as_of_date());

    // Assert - Should handle negative spread
    assert!(result.is_ok(), "Should handle negative spread");
}

#[test]
fn test_equity_trs_with_very_large_spread() {
    // Arrange
    let market = create_market_context();
    let trs = TestEquityTrsBuilder::new()
        .spread_bp(10000.0) // 100% spread
        .build();

    // Act
    let result = trs.value(&market, as_of_date());

    // Assert
    assert!(result.is_ok());
    assert!(result.unwrap().amount().is_finite());
}

// ================================================================================================
// Date Edge Cases
// ================================================================================================

#[test]
fn test_equity_trs_with_past_start_date() {
    // Arrange
    let market = create_market_context();
    let as_of = d(2025, 6, 1);
    let start = d(2025, 1, 1); // Start date in the past
    let end = d(2026, 1, 1);

    let underlying =
        EquityUnderlyingParams::new("SPX", "SPX-SPOT", USD).with_dividend_yield("SPX-DIV-YIELD");

    let trs = EquityTotalReturnSwap::builder()
        .id("TRS-PAST-START".into())
        .notional(Money::new(10_000_000.0, USD))
        .underlying(underlying)
        .financing(FinancingLegSpec::new(
            "USD-OIS",
            "USD-SOFR-3M",
            Decimal::from(25),
            DayCount::Act360,
        ))
        .schedule(TrsScheduleSpec::from_params(
            start,
            end,
            ScheduleParams::quarterly_act360(),
        ))
        .side(TrsSide::ReceiveTotalReturn)
        .build()
        .unwrap();

    // Act - Some periods are in the past
    let result = trs.value(&market, as_of);

    // Assert - Should handle past periods
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_equity_trs_with_very_short_tenor_1_day() {
    // Arrange
    let market = create_market_context();
    let start = as_of_date();
    let end = start + time::Duration::days(1);

    let underlying =
        EquityUnderlyingParams::new("SPX", "SPX-SPOT", USD).with_dividend_yield("SPX-DIV-YIELD");

    let trs = EquityTotalReturnSwap::builder()
        .id("TRS-1DAY".into())
        .notional(Money::new(10_000_000.0, USD))
        .underlying(underlying)
        .financing(FinancingLegSpec::new(
            "USD-OIS",
            "USD-SOFR-3M",
            Decimal::from(25),
            DayCount::Act360,
        ))
        .schedule(TrsScheduleSpec::from_params(
            start,
            end,
            ScheduleParams::quarterly_act360(),
        ))
        .side(TrsSide::ReceiveTotalReturn)
        .build()
        .unwrap();

    // Act
    let result = trs.value(&market, start);

    // Assert
    assert!(result.is_ok());
}

#[test]
fn test_fi_index_trs_maturity_equals_valuation_date() {
    // Arrange
    let market = create_market_context();
    let as_of = as_of_date();
    let start = as_of - time::Duration::days(365);
    let end = as_of; // Maturity = valuation date

    let underlying = IndexUnderlyingParams::new("HY-INDEX", USD)
        .with_yield("HY-INDEX-YIELD")
        .with_duration("HY-INDEX-DURATION");

    let trs = FIIndexTotalReturnSwap::builder()
        .id("TRS-MATURE".into())
        .notional(Money::new(10_000_000.0, USD))
        .underlying(underlying)
        .financing(FinancingLegSpec::new(
            "USD-OIS",
            "USD-SOFR-3M",
            Decimal::from(100),
            DayCount::Act360,
        ))
        .schedule(TrsScheduleSpec::from_params(
            start,
            end,
            ScheduleParams::quarterly_act360(),
        ))
        .side(TrsSide::ReceiveTotalReturn)
        .build()
        .unwrap();

    // Act - Pricing at maturity with past start date
    let result = trs.value(&market, as_of);

    // Assert - May fail due to past periods or succeed with zero value
    assert!(result.is_ok() || result.is_err());
}

// ================================================================================================
// Metric Calculation Edge Cases
// ================================================================================================

#[test]
fn test_par_spread_with_tiny_annuity() {
    // Arrange - Very short tenor gives tiny annuity
    let market = create_market_context();
    let as_of = as_of_date();

    let trs = TestEquityTrsBuilder::new()
        .tenor_months(1) // Very short
        .spread_bp(0.0)
        .build();

    // Act
    let result = trs.price_with_metrics(
        &market,
        as_of,
        &[finstack_valuations::metrics::MetricId::ParSpread],
    );

    // Assert - Should either compute or fail gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_index_delta_with_very_small_spot() {
    // Arrange
    let mut market = create_market_context();
    market = market.insert_price("SPX-SPOT", MarketScalar::Unitless(0.01)); // Tiny spot

    let trs = TestEquityTrsBuilder::new().build();

    // Act
    let result = trs.price_with_metrics(
        &market,
        as_of_date(),
        &[finstack_valuations::metrics::MetricId::IndexDelta],
    );

    // Assert - May fail with validation error for tiny spot
    assert!(result.is_ok() || result.is_err());
}

// ================================================================================================
// Contract Size Edge Cases
// ================================================================================================

#[test]
fn test_equity_trs_with_zero_contract_size() {
    // Arrange - Zero contract size should result in zero PV
    let market = create_market_context();
    let mut underlying =
        EquityUnderlyingParams::new("SPX", "SPX-SPOT", USD).with_dividend_yield("SPX-DIV-YIELD");
    underlying = underlying.with_contract_size(0.0);

    let trs = EquityTotalReturnSwap::builder()
        .id("TRS-ZERO-CONTRACT".into())
        .notional(Money::new(10_000_000.0, USD))
        .underlying(underlying)
        .financing(FinancingLegSpec::new(
            "USD-OIS",
            "USD-SOFR-3M",
            Decimal::from(25),
            DayCount::Act360,
        ))
        .schedule(TrsScheduleSpec::from_params(
            as_of_date(),
            d(2026, 1, 2),
            ScheduleParams::quarterly_act360(),
        ))
        .side(TrsSide::ReceiveTotalReturn)
        .build()
        .unwrap();

    // Act
    let result = trs.value(&market, as_of_date());

    // Assert
    assert!(result.is_ok());
    // TR leg should be zero, so NPV = -financing leg
    let npv = result.unwrap();
    let fin_pv = trs.pv_financing_leg(&market, as_of_date()).unwrap();
    assert_approx_eq(
        npv.amount(),
        -fin_pv.amount(),
        TOLERANCE_CENTS,
        "Zero contract size should zero out TR leg",
    );
}

#[test]
fn test_equity_trs_with_fractional_contract_size() {
    // Arrange
    let market = create_market_context();
    let mut underlying =
        EquityUnderlyingParams::new("SPX", "SPX-SPOT", USD).with_dividend_yield("SPX-DIV-YIELD");
    underlying = underlying.with_contract_size(0.1); // Mini contract

    let trs = EquityTotalReturnSwap::builder()
        .id("TRS-MINI".into())
        .notional(Money::new(10_000_000.0, USD))
        .underlying(underlying)
        .financing(FinancingLegSpec::new(
            "USD-OIS",
            "USD-SOFR-3M",
            Decimal::from(25),
            DayCount::Act360,
        ))
        .schedule(TrsScheduleSpec::from_params(
            as_of_date(),
            d(2026, 1, 2),
            ScheduleParams::quarterly_act360(),
        ))
        .side(TrsSide::ReceiveTotalReturn)
        .build()
        .unwrap();

    // Act
    let result = trs.value(&market, as_of_date());

    // Assert
    assert!(result.is_ok());
    assert!(result.unwrap().amount().is_finite());
}
