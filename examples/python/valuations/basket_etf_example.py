#!/usr/bin/env python3
"""
Basket/ETF instrument example demonstrating generic basket support for both equity and bond ETFs.

This example shows how to create and price different types of ETFs:
1. Equity ETF (like SPY) using individual equity constituents
2. Bond ETF (like LQD) using bond instruments
3. Mixed-asset basket with both stocks and bonds

The implementation leverages existing Bond and Equity pricing infrastructure
rather than reimplementing pricing logic.
"""

import finstack as fs
from datetime import date

def main():
    print("=== Basket/ETF Implementation Example ===\n")
    
    # Create base date for examples
    base_date = date(2025, 1, 1)
    maturity_date = date(2030, 1, 1)
    
    # Create basic market context
    curves = fs.MarketContext()
    
    # Add some sample market data for pricing
    curves = (curves
        .insert_price("AAPL", fs.MarketScalar.unitless(150.0))
        .insert_price("MSFT", fs.MarketScalar.unitless(300.0))
        .insert_price("GOOGL", fs.MarketScalar.unitless(2800.0))
        .insert_price("AMZN", fs.MarketScalar.unitless(3200.0))
        .insert_price("BOND_AAPL_2030", fs.MarketScalar.unitless(98.5))
        .insert_price("BOND_MSFT_2029", fs.MarketScalar.unitless(101.2))
    )
    
    # Create discount curve for bond pricing
    discount_curve = (fs.DiscountCurve.builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80)])
        .build()
    )
    curves = curves.insert_discount(discount_curve)
    
    print("1. Creating Equity ETF (SPY-like)")
    print("-" * 40)
    
    # Create equity ETF similar to SPY
    spy_basket = (fs.Basket.builder()
        .equity_etf("SPY", "SPY", "SPDR S&P 500 ETF Trust")
        .shares_outstanding(900_000_000.0)
        .add_equity("AAPL", "AAPL", 0.071, None)      # 7.1% weight
        .add_equity("MSFT", "MSFT", 0.069, None)      # 6.9% weight  
        .add_equity("GOOGL", "GOOGL", 0.036, None)    # 3.6% weight
        .add_equity("AMZN", "AMZN", 0.032, None)      # 3.2% weight
        # Add remaining weight as cash for simplicity
        .add_market_data("CASH", "USD_CASH", fs.AssetType.cash(), 0.792, None)
        .build()
    )
    
    # Price the equity ETF
    spy_nav = spy_basket.nav(curves, base_date)
    print(f"SPY NAV per share: ${spy_nav.amount():.2f}")
    print(f"Total basket value: ${spy_basket.basket_value(curves, base_date).amount():,.0f}")
    print(f"Constituents: {spy_basket.constituent_count()}")
    print()
    
    print("2. Creating Bond ETF (LQD-like)")
    print("-" * 40)
    
    # Create some sample bonds for the bond ETF
    aapl_bond = (fs.Bond.builder()
        .id("AAPL_4.65_2030")
        .notional(fs.Money.new(1000.0, fs.Currency.USD))
        .coupon(0.0465)
        .dates(base_date, maturity_date)
        .disc_curve("USD-OIS")
        .build()
    )
    
    msft_bond = (fs.Bond.builder()
        .id("MSFT_3.50_2029") 
        .notional(fs.Money.new(1000.0, fs.Currency.USD))
        .coupon(0.035)
        .dates(base_date, date(2029, 6, 15))
        .disc_curve("USD-OIS")
        .build()
    )
    
    # Create investment grade bond ETF
    lqd_basket = (fs.Basket.builder()
        .bond_etf("LQD", "LQD", "iShares iBoxx $ Investment Grade Corporate Bond ETF")
        .shares_outstanding(200_000_000.0)
        .add_bond("AAPL_BOND", aapl_bond, 0.015, 15000.0)    # 1.5% weight, 15k bonds
        .add_bond("MSFT_BOND", msft_bond, 0.012, 12000.0)    # 1.2% weight, 12k bonds
        # Add market data references for bonds not fully modeled
        .add_market_data("GOOGL_BOND", "BOND_GOOGL_2031", fs.AssetType.bond(), 0.010, None)
        .add_market_data("CASH", "USD_CASH", fs.AssetType.cash(), 0.963, None)  # Rest as cash
        .build()
    )
    
    # Price the bond ETF
    lqd_nav = lqd_basket.nav(curves, base_date)
    print(f"LQD NAV per share: ${lqd_nav.amount():.2f}")
    print(f"Total basket value: ${lqd_basket.basket_value(curves, base_date).amount():,.0f}")
    print(f"Expense ratio: {lqd_basket.expense_ratio * 100:.2f}%")
    print()
    
    print("3. Creating Mixed-Asset Balanced ETF")
    print("-" * 40)
    
    # Create a balanced fund with both stocks and bonds
    balanced_basket = (fs.Basket.builder()
        .id("BALANCED")
        .ticker("BALANCED") 
        .name("Balanced Equity/Bond ETF")
        .currency(fs.Currency.USD)
        .expense_ratio(0.0025)  # 25 bps
        .shares_outstanding(50_000_000.0)
        # 60% equity allocation
        .add_equity("AAPL", "AAPL", 0.20, None)
        .add_equity("MSFT", "MSFT", 0.20, None)
        .add_equity("GOOGL", "GOOGL", 0.20, None)
        # 40% bond allocation
        .add_bond("BOND1", aapl_bond, 0.20, None)
        .add_bond("BOND2", msft_bond, 0.20, None)
        .build()
    )
    
    balanced_nav = balanced_basket.nav(curves, base_date)
    print(f"Balanced ETF NAV per share: ${balanced_nav.amount():.2f}")
    print(f"Asset allocation: 60% equity, 40% bonds")
    print()
    
    print("4. Computing ETF Metrics")
    print("-" * 40)
    
    # Get metrics for the SPY basket
    metrics_to_compute = [
        fs.MetricId.nav(),
        fs.MetricId.basket_value(),
        fs.MetricId.constituent_count(),
        fs.MetricId.expense_ratio(),
    ]
    
    spy_result = spy_basket.price_with_metrics(curves, base_date, metrics_to_compute)
    print("SPY Metrics:")
    for metric_name, value in spy_result.measures.items():
        print(f"  {metric_name}: {value}")
    print()
    
    print("5. Basket Validation")
    print("-" * 40)
    
    # Validate basket construction
    spy_validation = spy_basket.validate()
    lqd_validation = lqd_basket.validate()
    
    print(f"SPY validation: {'✓ Passed' if spy_validation.is_ok() else '✗ Failed'}")
    print(f"LQD validation: {'✓ Passed' if lqd_validation.is_ok() else '✗ Failed'}")
    print()
    
    print("6. Creation/Redemption Analysis")
    print("-" * 40)
    
    # Analyze creation unit composition
    creation_units = 1.0  # Price 1 creation unit
    spy_creation_basket = spy_basket.creation_basket(creation_units)
    
    print(f"SPY Creation Unit ({spy_basket.creation_unit_size:,.0f} shares):")
    print(f"  Transaction cost: ${spy_creation_basket.transaction_cost.amount():.2f}")
    if spy_creation_basket.cash_component:
        print(f"  Cash component: ${spy_creation_basket.cash_component.amount():.2f}")
    
    # Calculate arbitrage opportunity
    assumed_market_price = fs.Money.new(spy_nav.amount() * 1.001, fs.Currency.USD)  # 0.1% premium
    arb_spread = spy_basket.arbitrage_spread(assumed_market_price, spy_nav)
    print(f"  Arbitrage spread: {arb_spread * 10000:.1f} bps")
    print()
    
    print("=== Summary ===")
    print(f"✓ Implemented generic basket supporting equity and bond ETFs")
    print(f"✓ Leveraged existing Bond and Equity pricing infrastructure")
    print(f"✓ Provided NAV calculation, metrics, and creation/redemption analysis")
    print(f"✓ Maintained currency safety and validation")

if __name__ == "__main__":
    main()
