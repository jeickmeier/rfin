"""
Non-Deliverable Forward (NDF) Example
======================================

Demonstrates pricing NDFs in pre-fixing and post-fixing modes using
covered interest rate parity.
"""

from finstack import Money, Date
from finstack.core.market_data import (
    MarketContext,
    DiscountCurve,
    ForwardCurve,
    InterpolationStyle,
    FxMatrix,
)
from finstack.valuations.instruments import Ndf
from finstack.valuations.pricer import create_standard_registry


def main():
    """
    Price a 3-month USD/CNY NDF:
    - Trade date: Jan 15, 2024
    - Fixing date: Apr 13, 2024 (2 business days before settlement)
    - Settlement date: Apr 15, 2024
    - Notional: $5M (base currency)
    - Contract rate: 7.20 CNY/USD
    """
    
    # Market data setup
    as_of = Date(2024, 1, 15)
    base_date = as_of
    
    # USD discount curve
    usd_tenors = [0.0, 0.25, 0.5, 1.0, 2.0, 5.0]
    usd_dfs = [1.0, 0.9875, 0.9750, 0.9500, 0.9100, 0.8200]
    
    usd_discount = DiscountCurve(
        curve_id="USD-OIS",
        base_date=base_date,
        tenors=usd_tenors,
        values=usd_dfs,
        interpolation=InterpolationStyle.LogLinear(),
    )
    
    # CNY discount curve (onshore implied)
    cny_tenors = [0.0, 0.25, 0.5, 1.0, 2.0, 5.0]
    cny_dfs = [1.0, 0.9900, 0.9800, 0.9600, 0.9300, 0.8500]
    
    cny_discount = DiscountCurve(
        curve_id="CNY-REPO",
        base_date=base_date,
        tenors=cny_tenors,
        values=cny_dfs,
        interpolation=InterpolationStyle.LogLinear(),
    )
    
    # CNY forward curve (for FX forward rate estimation)
    cny_fwd_rates = [0.0250, 0.0255, 0.0260, 0.0270, 0.0285, 0.0300]
    
    cny_forward = ForwardCurve(
        curve_id="CNY-FORWARD",
        base_date=base_date,
        tenors=cny_tenors,
        values=cny_fwd_rates,
        interpolation=InterpolationStyle.Linear(),
    )
    
    # FX spot rate: USD/CNY = 7.18 (1 USD = 7.18 CNY)
    fx_matrix = FxMatrix()
    fx_matrix.set_rate("USD", "CNY", 7.18)
    
    # Build market context
    market = MarketContext()
    market.insert_discount(usd_discount)
    market.insert_discount(cny_discount)
    market.insert_forward(cny_forward)
    market.set_fx_provider(fx_matrix)
    
    # Add spot FX rate
    market.insert_scalar("USDCNY-SPOT", 7.18)
    
    # Create NDF (pre-fixing mode)
    ndf = (
        Ndf.builder("USDCNY-NDF-3M")
        .base_currency("USD")  # Restricted currency
        .quote_currency("CNY")  # Settlement currency (actually settles in USD)
        .settlement_currency("USD")
        .notional(5_000_000, "USD")
        .contract_rate(7.20)  # Agreed forward rate
        .trade_date(Date(2024, 1, 15))
        .fixing_date(Date(2024, 4, 13))
        .settlement_date(Date(2024, 4, 15))
        .spot_fx_id("USDCNY-SPOT")
        .base_discount_curve("USD-OIS")
        .settlement_discount_curve("USD-OIS")
        .forward_curve_id("CNY-FORWARD")
        .build()
    )
    
    # Price the NDF (pre-fixing)
    registry = create_standard_registry()
    
    result = registry.price_ndf_with_metrics(
        ndf,
        "discounting",
        market,
        ["forward_rate", "delta", "dv01"],
    )
    
    # Display results
    print("=" * 70)
    print("NDF Valuation Results (Pre-Fixing)")
    print("=" * 70)
    print(f"Instrument ID:      {ndf.instrument_id()}")
    print(f"Currency Pair:      {ndf.base_currency().code}/{ndf.quote_currency().code}")
    print(f"Notional:           {ndf.notional().amount:,.2f} {ndf.notional().currency.code}")
    print(f"Contract Rate:      {ndf.contract_rate():.4f}")
    print(f"Trade Date:         {ndf.trade_date()}")
    print(f"Fixing Date:        {ndf.fixing_date()}")
    print(f"Settlement Date:    {ndf.settlement_date()}")
    print(f"\nPresent Value:      {result.present_value.amount:,.2f} {result.present_value.currency.code}")
    print(f"\nMetrics:")
    print(f"  Forward Rate:      {result.metric('forward_rate') or 0:.4f}")
    print(f"  Delta:             {result.metric('delta') or 0:,.2f}")
    print(f"  DV01:              {result.metric('dv01') or 0:,.2f}")
    print("=" * 70)
    
    # Create post-fixing NDF (fixing has occurred)
    ndf_post_fix = (
        Ndf.builder("USDCNY-NDF-3M-FIXED")
        .base_currency("USD")
        .quote_currency("CNY")
        .settlement_currency("USD")
        .notional(5_000_000, "USD")
        .contract_rate(7.20)
        .trade_date(Date(2024, 1, 15))
        .fixing_date(Date(2024, 4, 13))
        .settlement_date(Date(2024, 4, 15))
        .fixing_rate(7.25)  # Actual fixing: 7.25 CNY/USD
        .base_discount_curve("USD-OIS")
        .settlement_discount_curve("USD-OIS")
        .build()
    )
    
    result_post = registry.price_ndf(ndf_post_fix, "discounting", market)
    
    print(f"\nPost-Fixing Mode (Fixing Rate = 7.25):")
    print(f"  Present Value:    {result_post.present_value.amount:,.2f} {result_post.present_value.currency.code}")
    
    # Manual settlement calculation
    settlement_amount = 5_000_000 * (7.25 - 7.20) / 7.25
    print(f"  Manual Calc:      ${settlement_amount:,.2f}")
    print(f"  (Notional × (Fixing - Contract) / Fixing)")
    print("=" * 70)


if __name__ == "__main__":
    main()
