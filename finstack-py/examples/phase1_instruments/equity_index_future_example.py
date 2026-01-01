"""
Equity Index Future Example
============================

Demonstrates pricing equity index futures using mark-to-market and
fair value (cost-of-carry) methodologies.
"""

from finstack import Date
from finstack.core.market_data import (
    MarketContext,
    DiscountCurve,
    InterpolationStyle,
)
from finstack.valuations.instruments import EquityIndexFuture
from finstack.valuations.pricer import create_standard_registry


def main():
    """
    Price an E-mini S&P 500 future (ES) with:
    - Expiry: March 2025
    - Contract size: $50 per index point
    - Position: Long 10 contracts
    - Entry price: 4500.0
    - Current quoted price: 4550.0
    """
    
    # Market data setup
    as_of = Date(2025, 1, 15)
    base_date = as_of
    
    # USD discount curve
    tenors = [0.0, 0.25, 0.5, 1.0, 2.0, 5.0]
    dfs = [1.0, 0.9875, 0.9750, 0.9500, 0.9100, 0.8200]
    
    discount_curve = DiscountCurve(
        curve_id="USD-OIS",
        base_date=base_date,
        tenors=tenors,
        values=dfs,
        interpolation=InterpolationStyle.LogLinear(),
    )
    
    market = MarketContext()
    market.insert_discount(discount_curve)
    
    # Optional: Add spot index price for fair value calculation
    market.insert_scalar("SPX-SPOT", 4525.0)  # S&P 500 spot index level
    
    # Optional: Add dividend yield for fair value
    market.insert_scalar("SPX-DIVIDEND-YIELD", 0.015)  # 1.5% continuous yield
    
    # Get E-mini S&P 500 contract specifications
    contract_specs = EquityIndexFuture.sp500_emini_specs()
    
    # Create the future contract
    future = (
        EquityIndexFuture.builder("ES-2025M03")
        .index_ticker("SPX")
        .currency("USD")
        .quantity(10.0)  # 10 contracts
        .expiry_date(Date(2025, 3, 21))
        .last_trading_date(Date(2025, 3, 20))
        .entry_price(4500.0)
        .quoted_price(4550.0)  # Mark-to-market price
        .position("long")
        .contract_specs(contract_specs)
        .discount_curve("USD-OIS")
        .index_price_id("SPX-SPOT")
        .dividend_yield_id("SPX-DIVIDEND-YIELD")
        .build()
    )
    
    # Price the future
    registry = create_standard_registry()
    
    result = registry.price_equity_index_future_with_metrics(
        future,
        "discounting",
        market,
        ["fair_value", "basis", "carry_cost", "mtm_pnl"],
    )
    
    # Display results
    print("=" * 70)
    print("Equity Index Future Valuation Results")
    print("=" * 70)
    print(f"Contract ID:        {future.instrument_id()}")
    print(f"Index Ticker:       {future.index_ticker()}")
    print(f"Contract Specs:     {future.contract_specs()}")
    print(f"Position:           {future.position()}")
    print(f"Quantity:           {future.quantity()}")
    print(f"Entry Price:        {future.entry_price():.2f}")
    print(f"Quoted Price:       {future.quoted_price():.2f}")
    print(f"Expiry Date:        {future.expiry_date()}")
    print(f"\nPresent Value (MTM): {result.present_value.amount:,.2f} {result.present_value.currency.code}")
    print(f"\nMetrics:")
    print(f"  Fair Value:        {result.metric('fair_value') or 0:.2f}")
    print(f"  Basis:             {result.metric('basis') or 0:.2f} (Fair - Quoted)")
    print(f"  Carry Cost:        {result.metric('carry_cost') or 0:.2f}")
    print(f"  MTM P&L:           {result.metric('mtm_pnl') or 0:,.2f}")
    print("=" * 70)
    
    # Manual P&L calculation
    price_change = future.quoted_price() - future.entry_price()
    pnl_per_contract = price_change * contract_specs.multiplier
    total_pnl = pnl_per_contract * future.quantity()
    
    print(f"\nManual P&L Verification:")
    print(f"  Price Change:      {price_change:.2f} index points")
    print(f"  Multiplier:        ${contract_specs.multiplier:.2f} per point")
    print(f"  P&L per Contract:  ${pnl_per_contract:,.2f}")
    print(f"  Total P&L:         ${total_pnl:,.2f}")
    print("=" * 70)
    
    # Show contract specs
    print(f"\nContract Specifications:")
    print(f"  Multiplier:        ${contract_specs.multiplier}")
    print(f"  Tick Size:         {contract_specs.tick_size}")
    print(f"  Tick Value:        ${contract_specs.tick_value}")
    print(f"  Settlement:        {contract_specs.settlement_method}")
    print("=" * 70)


if __name__ == "__main__":
    main()
