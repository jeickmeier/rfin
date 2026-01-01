"""
Cross-Currency Swap Example
============================

Demonstrates pricing a cross-currency floating-for-floating swap
with notional exchange.
"""

from finstack import Money, Date
from finstack.core.market_data import (
    MarketContext,
    DiscountCurve,
    ForwardCurve,
    InterpolationStyle,
    FxMatrix,
)
from finstack.valuations.instruments import CrossCurrencySwap
from finstack.valuations.pricer import create_standard_registry


def main():
    """
    Price a 5-year USD/EUR cross-currency basis swap:
    - Pay USD SOFR + 10bp quarterly
    - Receive EUR ESTR flat quarterly
    - Notional exchange at start and maturity
    """
    
    # Market data setup
    as_of = Date(2024, 1, 15)
    base_date = as_of
    
    # USD OIS discount curve
    usd_tenors = [0.0, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0]
    usd_dfs = [1.0, 0.9875, 0.9750, 0.9500, 0.9100, 0.8200, 0.6500]
    
    usd_discount = DiscountCurve(
        curve_id="USD-OIS",
        base_date=base_date,
        tenors=usd_tenors,
        values=usd_dfs,
        interpolation=InterpolationStyle.LogLinear(),
    )
    
    # EUR OIS discount curve
    eur_tenors = [0.0, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0]
    eur_dfs = [1.0, 0.9900, 0.9800, 0.9600, 0.9300, 0.8500, 0.7000]
    
    eur_discount = DiscountCurve(
        curve_id="EUR-OIS",
        base_date=base_date,
        tenors=eur_tenors,
        values=eur_dfs,
        interpolation=InterpolationStyle.LogLinear(),
    )
    
    # USD SOFR forward curve
    usd_fwd_rates = [0.0450, 0.0455, 0.0460, 0.0465, 0.0475, 0.0490, 0.0500]
    
    usd_forward = ForwardCurve(
        curve_id="USD-SOFR",
        base_date=base_date,
        tenors=usd_tenors,
        values=usd_fwd_rates,
        interpolation=InterpolationStyle.Linear(),
    )
    
    # EUR ESTR forward curve
    eur_fwd_rates = [0.0350, 0.0355, 0.0360, 0.0365, 0.0375, 0.0390, 0.0400]
    
    eur_forward = ForwardCurve(
        curve_id="EUR-ESTR",
        base_date=base_date,
        tenors=eur_tenors,
        values=eur_fwd_rates,
        interpolation=InterpolationStyle.Linear(),
    )
    
    # FX rate: EUR/USD = 1.10 (1 EUR = 1.10 USD)
    fx_matrix = FxMatrix()
    fx_matrix.set_rate("EUR", "USD", 1.10)
    
    # Build market context
    market = MarketContext()
    market.insert_discount(usd_discount)
    market.insert_discount(eur_discount)
    market.insert_forward(usd_forward)
    market.insert_forward(eur_forward)
    market.set_fx_provider(fx_matrix)
    
    # Create cross-currency swap
    swap = (
        CrossCurrencySwap.builder("XCCY_USD_EUR_001")
        .start_date(Date(2024, 1, 15))
        .maturity_date(Date(2029, 1, 15))
        .reporting_currency("USD")
        # USD leg: Pay floating
        .leg1_currency("USD")
        .leg1_notional(10_000_000, "USD")
        .leg1_side("pay")
        .leg1_forward_curve("USD-SOFR")
        .leg1_discount_curve("USD-OIS")
        .leg1_frequency("quarterly")
        .leg1_spread(0.0010)  # 10bp
        .leg1_day_count("Act/360")
        .leg1_bdc("modified_following")
        # EUR leg: Receive floating
        .leg2_currency("EUR")
        .leg2_notional(9_090_909, "EUR")  # ~10M USD / 1.10
        .leg2_side("receive")
        .leg2_forward_curve("EUR-ESTR")
        .leg2_discount_curve("EUR-OIS")
        .leg2_frequency("quarterly")
        .leg2_spread(0.0)  # Flat
        .leg2_day_count("Act/360")
        .leg2_bdc("modified_following")
        # Notional exchange
        .exchange_notional_at_start(True)
        .exchange_notional_at_maturity(True)
        .build()
    )
    
    # Price the swap
    registry = create_standard_registry()
    
    result = registry.price_cross_currency_swap_with_metrics(
        swap,
        "discounting",
        market,
        ["npv_leg1", "npv_leg2", "dv01", "cs01", "fx_delta"],
    )
    
    # Display results
    print("=" * 70)
    print("Cross-Currency Swap Valuation Results")
    print("=" * 70)
    print(f"Swap ID:            {swap.instrument_id()}")
    print(f"Start Date:         {swap.start_date()}")
    print(f"Maturity Date:      {swap.maturity_date()}")
    print(f"Reporting Currency: {swap.reporting_currency()}")
    print(f"\nLeg 1 (USD Pay):")
    print(f"  Notional:         {swap.leg1_notional().amount:,.2f} {swap.leg1_notional().currency.code}")
    print(f"  Spread:           {swap.leg1_spread() * 10000:.2f} bp")
    print(f"  Forward Curve:    {swap.leg1_forward_curve()}")
    print(f"\nLeg 2 (EUR Receive):")
    print(f"  Notional:         {swap.leg2_notional().amount:,.2f} {swap.leg2_notional().currency.code}")
    print(f"  Spread:           {swap.leg2_spread() * 10000:.2f} bp")
    print(f"  Forward Curve:    {swap.leg2_forward_curve()}")
    print(f"\nPresent Value:      {result.present_value.amount:,.2f} {result.present_value.currency.code}")
    print(f"\nMetrics:")
    print(f"  NPV Leg 1 (USD):  {result.metric('npv_leg1') or 0:,.2f}")
    print(f"  NPV Leg 2 (EUR):  {result.metric('npv_leg2') or 0:,.2f}")
    print(f"  DV01:             {result.metric('dv01') or 0:,.2f}")
    print(f"  CS01:             {result.metric('cs01') or 0:,.2f}")
    print(f"  FX Delta:         {result.metric('fx_delta') or 0:,.4f}")
    print("=" * 70)


if __name__ == "__main__":
    main()
