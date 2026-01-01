"""
Bond Future Example
===================

Demonstrates pricing and valuation of government bond futures contracts
with deliverable baskets and conversion factors.
"""

from finstack import Money, Date
from finstack.core.market_data import (
    MarketContext,
    DiscountCurve,
    InterpolationStyle,
)
from finstack.valuations.instruments import BondFuture
from finstack.valuations.pricer import create_standard_registry


def main():
    """
    Price a 10-year US Treasury bond future contract (TY) with:
    - Contract expiry: March 2025
    - Delivery window: March 21-31, 2025
    - Entry price: 125.50 (quoted in 32nds)
    - Mark-to-market price: 126.25
    - Position: Long 10 contracts
    """
    
    # Create market data: USD discount curve
    as_of = Date(2025, 1, 15)
    base_date = as_of
    
    tenors = [0.0, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0]  # Years
    discount_factors = [1.0, 0.9875, 0.9750, 0.9500, 0.9100, 0.8200, 0.6500, 0.3500]
    
    discount_curve = DiscountCurve(
        curve_id="USD-OIS",
        base_date=base_date,
        tenors=tenors,
        values=discount_factors,
        interpolation=InterpolationStyle.LogLinear(),
    )
    
    market = MarketContext()
    market.insert_discount(discount_curve)
    
    # Define contract specifications (TY - 10-Year Treasury Note)
    contract_specs = BondFuture.ust_10y_specs()
    
    # Define deliverable basket with conversion factors
    deliverable_basket = [
        {
            "bond_id": "US912828XG33",  # Example CUSIP
            "conversion_factor": 0.8234,
            "quoted_clean_price": 98.50,
        },
        {
            "bond_id": "US912828XH16",
            "conversion_factor": 0.8156,
            "quoted_clean_price": 97.25,
        },
        {
            "bond_id": "US912828XJ71",
            "conversion_factor": 0.8089,
            "quoted_clean_price": 96.75,
        },
    ]
    
    # Create bond future instrument using builder
    future = (
        BondFuture.builder("TYH5")
        .notional(1_000_000, "USD")
        .expiry_date(Date(2025, 3, 20))
        .delivery_start(Date(2025, 3, 21))
        .delivery_end(Date(2025, 3, 31))
        .entry_price(125.50)
        .quoted_price(126.25)
        .position("long")
        .quantity(10.0)
        .contract_specs(contract_specs)
        .deliverable_basket(deliverable_basket)
        .discount_curve("USD-OIS")
        .build()
    )
    
    # Price the future
    registry = create_standard_registry()
    
    # Price with mark-to-market (uses quoted_price)
    result = registry.price_bond_future_with_metrics(
        future,
        "discounting",
        market,
        ["clean_price", "dirty_price", "accrued", "ctd_bond", "basis"],
    )
    
    # Display results
    print("=" * 70)
    print("Bond Future Valuation Results")
    print("=" * 70)
    print(f"Contract ID:        {future.instrument_id()}")
    print(f"Contract Specs:     {future.contract_specs()}")
    print(f"Position:           {future.position()}")
    print(f"Quantity:           {future.quantity()}")
    print(f"Entry Price:        {future.entry_price():.4f}")
    print(f"Quoted Price:       {future.quoted_price():.4f}")
    print(f"\nPresent Value:      {result.present_value.amount:,.2f} {result.present_value.currency.code}")
    print(f"\nMetrics:")
    print(f"  Clean Price:      {result.metric('clean_price') or 0:.4f}")
    print(f"  Dirty Price:      {result.metric('dirty_price') or 0:.4f}")
    print(f"  Accrued Interest: {result.metric('accrued') or 0:.2f}")
    print(f"  CTD Bond:         {result.metric('ctd_bond') or 'N/A'}")
    print(f"  Gross Basis:      {result.metric('basis') or 0:.4f} (32nds)")
    print("=" * 70)
    
    # Calculate P&L from entry to current quote
    pnl_per_contract = (126.25 - 125.50) * contract_specs.tick_value / contract_specs.tick_size
    total_pnl = pnl_per_contract * 10
    
    print(f"\nP&L Analysis:")
    print(f"  Price Change:     {126.25 - 125.50:.2f} points")
    print(f"  P&L per Contract: ${pnl_per_contract:,.2f}")
    print(f"  Total P&L (10 contracts): ${total_pnl:,.2f}")
    print("=" * 70)


if __name__ == "__main__":
    main()
