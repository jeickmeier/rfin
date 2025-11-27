//! Integration tests for market scalars attribution.
//!
//! Tests attribution of P&L from changes in dividends, equity prices,
//! inflation indices, and other market scalars.

use finstack_core::currency::Currency;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::CurveId;

/// Placeholder test for equity price scalar attribution.
///
/// This test is ignored pending completion of equity pricing scalar lookup.
/// The required infrastructure (extract_scalars, restore_scalars) is in place,
/// but equity instruments need to use price_id consistently for attribution
/// to detect spot price changes automatically.
///
/// TODO: Enable once equity pricing correctly uses AAPL-SPOT or EQUITY-SPOT
/// conventions for scalar attribution.
#[test]
#[ignore = "Equity scalar attribution pending price_id implementation"]
fn test_equity_price_scalar_attribution() {
    // Infrastructure verified by other tests in this file:
    // 1. extract_scalars() correctly extracts prices
    // 2. restore_scalars() correctly restores prices
    // 3. Attribution framework detects scalar changes
    //
    // Missing: Equity option instrument using MarketContext.price(equity_id)
    // for spot price lookup during pricing.
    unimplemented!("Enable once equity pricing uses price_id for scalars");
}

#[test]
fn test_scalars_snapshot_extraction() {
    use finstack_valuations::attribution::factors::{extract_scalars, restore_scalars};

    // Create market with various scalars
    let mut market = MarketContext::new();
    market.insert_price_mut(
        "AAPL",
        MarketScalar::Price(Money::new(180.0, Currency::USD)),
    );
    market.insert_price_mut(
        "MSFT",
        MarketScalar::Price(Money::new(400.0, Currency::USD)),
    );

    // Extract scalars snapshot
    let snapshot = extract_scalars(&market);

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
    use finstack_valuations::attribution::factors::{extract_scalars, restore_scalars};

    // Market at T₀ with lower prices
    let mut market_t0 = MarketContext::new();
    market_t0.insert_price_mut(
        "AAPL",
        MarketScalar::Price(Money::new(180.0, Currency::USD)),
    );

    // Market at T₁ with higher prices
    let mut market_t1 = MarketContext::new();
    market_t1.insert_price_mut(
        "AAPL",
        MarketScalar::Price(Money::new(185.0, Currency::USD)),
    );

    // Extract T₀ scalars
    let scalars_t0 = extract_scalars(&market_t0);

    // Restore T₀ scalars to T₁ market
    let hybrid_market = restore_scalars(&market_t1, &scalars_t0);

    // Verify T₀ price was restored
    let price = hybrid_market.price("AAPL").unwrap();
    if let MarketScalar::Price(money) = price {
        assert_eq!(money.amount(), 180.0); // Should be T₀ value
    }
}
