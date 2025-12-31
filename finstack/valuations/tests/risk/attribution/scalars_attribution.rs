//! Integration tests for market scalars attribution.
//!
//! Tests attribution of P&L from changes in dividends, equity prices,
//! inflation indices, and other market scalars.

use finstack_core::currency::Currency;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::money::Money;
use finstack_core::types::CurveId;

// TODO: Equity price scalar attribution
//
// The required infrastructure (ScalarsSnapshot::extract, restore_scalars) is in place,
// but equity instruments need to use price_id consistently for attribution
// to detect spot price changes automatically.
//
// Enable once equity pricing correctly uses AAPL-SPOT or EQUITY-SPOT
// conventions for scalar attribution. Missing: Equity option instrument
// using MarketContext.price(equity_id) for spot price lookup during pricing.

#[test]
fn test_scalars_snapshot_extraction() {
    use finstack_valuations::attribution::{restore_scalars, MarketExtractable, ScalarsSnapshot};

    // Create market with various scalars
    let market = MarketContext::new()
        .insert_price(
            "AAPL",
            MarketScalar::Price(Money::new(180.0, Currency::USD)),
        )
        .insert_price(
            "MSFT",
            MarketScalar::Price(Money::new(400.0, Currency::USD)),
        );

    // Extract scalars snapshot
    let snapshot = ScalarsSnapshot::extract(&market);

    // Verify extraction
    assert_eq!(snapshot.prices.len(), 2);
    assert!(snapshot.prices.contains_key(&CurveId::from("AAPL")));
    assert!(snapshot.prices.contains_key(&CurveId::from("MSFT")));

    // Restore to new market
    let empty_market = MarketContext::new();
    let restored = restore_scalars(&empty_market, &snapshot);

    // Verify restoration
    let aapl_price = restored.price("AAPL").unwrap();
    if let MarketScalar::Price(money) = aapl_price {
        assert_eq!(money.amount(), 180.0);
    } else {
        panic!("Expected Price scalar");
    }
}

#[test]
fn test_market_scalar_freeze_restore() {
    use finstack_valuations::attribution::{restore_scalars, MarketExtractable, ScalarsSnapshot};

    // Market at T₀ with lower prices
    let market_t0 = MarketContext::new().insert_price(
        "AAPL",
        MarketScalar::Price(Money::new(180.0, Currency::USD)),
    );

    // Market at T₁ with higher prices
    let market_t1 = MarketContext::new().insert_price(
        "AAPL",
        MarketScalar::Price(Money::new(185.0, Currency::USD)),
    );

    // Extract T₀ scalars
    let scalars_t0 = ScalarsSnapshot::extract(&market_t0);

    // Restore T₀ scalars to T₁ market
    let hybrid_market = restore_scalars(&market_t1, &scalars_t0);

    // Verify T₀ price was restored
    let price = hybrid_market.price("AAPL").unwrap();
    if let MarketScalar::Price(money) = price {
        assert_eq!(money.amount(), 180.0); // Should be T₀ value
    }
}
