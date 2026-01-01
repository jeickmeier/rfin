"""Commodity Option Example.
=========================

Demonstrates pricing commodity options (calls/puts) on energy, metals,
and agricultural commodities using Black-76 and binomial tree models.
"""

from finstack.core.market_data import (
    DiscountCurve,
    ForwardCurve,
    InterpolationStyle,
    MarketContext,
    VolSurface,
)
from finstack.valuations.instruments import CommodityOption
from finstack.valuations.pricer import create_standard_registry

from finstack import Date


def main():
    """Price a WTI crude oil call option with:
    - Strike: $75/barrel
    - Expiry: June 15, 2025
    - Quantity: 1000 barrels
    - Exercise: European.
    """
    # Market data setup
    as_of = Date(2025, 1, 15)
    base_date = as_of

    # USD discount curve
    usd_tenors = [0.0, 0.25, 0.5, 1.0, 2.0, 5.0]
    usd_dfs = [1.0, 0.9875, 0.9750, 0.9500, 0.9100, 0.8200]

    discount_curve = DiscountCurve(
        curve_id="USD-OIS",
        base_date=base_date,
        tenors=usd_tenors,
        values=usd_dfs,
        interpolation=InterpolationStyle.LogLinear(),
    )

    # WTI forward curve ($/barrel)
    wti_tenors = [0.0, 0.25, 0.5, 0.75, 1.0, 2.0]
    wti_forwards = [72.50, 73.25, 74.00, 74.50, 75.00, 77.00]

    forward_curve = ForwardCurve(
        curve_id="WTI-FORWARD",
        base_date=base_date,
        tenors=wti_tenors,
        values=wti_forwards,
        interpolation=InterpolationStyle.Linear(),
    )

    # WTI volatility surface (implied vols)
    vol_tenors = [0.25, 0.5, 0.75, 1.0, 2.0]  # Years
    vol_strikes = [60.0, 70.0, 75.0, 80.0, 90.0]  # Strike prices

    # Vol matrix (ATM vol ~ 35%)
    vol_matrix = [
        [0.42, 0.38, 0.36, 0.38, 0.44],  # 3M
        [0.40, 0.36, 0.34, 0.36, 0.42],  # 6M
        [0.38, 0.34, 0.32, 0.34, 0.40],  # 9M
        [0.36, 0.32, 0.30, 0.32, 0.38],  # 1Y
        [0.34, 0.30, 0.28, 0.30, 0.36],  # 2Y
    ]

    vol_surface = VolSurface(
        surface_id="WTI-VOL",
        base_date=base_date,
        tenors=vol_tenors,
        strikes=vol_strikes,
        vol_matrix=vol_matrix,
        interpolation=InterpolationStyle.Linear(),
    )

    # Build market context
    market = MarketContext()
    market.insert_discount(discount_curve)
    market.insert_forward(forward_curve)
    market.insert_vol_surface(vol_surface)

    # Optional: Spot price (for American options)
    market.insert_scalar("WTI-SPOT", 72.50)

    # Create European call option
    call_option = CommodityOption.create(
        "WTI-CALL-75-2025M06",
        commodity_type="Energy",
        ticker="CL",  # NYMEX symbol for crude oil
        strike=75.0,
        option_type="call",
        exercise_style="european",
        expiry=Date(2025, 6, 15),
        quantity=1000.0,
        unit="BBL",  # Barrels
        currency="USD",
        forward_curve_id="WTI-FORWARD",
        discount_curve_id="USD-OIS",
        vol_surface_id="WTI-VOL",
        multiplier=1.0,
        settlement_type="cash",
    )

    # Price the call option (Black-76 for European)
    registry = create_standard_registry()

    result_call = registry.price_commodity_option_with_metrics(
        call_option,
        "black76",
        market,
        ["delta", "gamma", "vega", "theta", "rho"],
    )

    # Display results
    print("=" * 70)
    print("Commodity Option Valuation Results - European Call")
    print("=" * 70)
    print(f"Instrument ID:      {call_option.instrument_id()}")
    print(f"Commodity:          {call_option.commodity_type()} - {call_option.ticker()}")
    print(f"Option Type:        {call_option.option_type()}")
    print(f"Exercise Style:     {call_option.exercise_style()}")
    print(f"Strike:             ${call_option.strike():.2f}/{call_option.unit()}")
    print(f"Expiry Date:        {call_option.expiry()}")
    print(f"Quantity:           {call_option.quantity():,.0f} {call_option.unit()}")
    print(f"Settlement:         {call_option.settlement_type()}")
    print(f"\nPresent Value:      ${result_call.present_value.amount:,.2f}")
    print("\nGreeks:")
    print(f"  Delta:             {result_call.metric('delta') or 0:.4f}")
    print(f"  Gamma:             {result_call.metric('gamma') or 0:.6f}")
    print(f"  Vega:              ${result_call.metric('vega') or 0:,.2f}")
    print(f"  Theta (1d):        ${result_call.metric('theta') or 0:,.2f}")
    print(f"  Rho:               ${result_call.metric('rho') or 0:,.2f}")
    print("=" * 70)

    # Create American put option for comparison
    put_option = CommodityOption.create(
        "WTI-PUT-70-2025M06",
        commodity_type="Energy",
        ticker="CL",
        strike=70.0,
        option_type="put",
        exercise_style="american",
        expiry=Date(2025, 6, 15),
        quantity=1000.0,
        unit="BBL",
        currency="USD",
        forward_curve_id="WTI-FORWARD",
        discount_curve_id="USD-OIS",
        vol_surface_id="WTI-VOL",
        spot_price_id="WTI-SPOT",
        tree_steps=100,  # Binomial tree for American
    )

    # Price American put (binomial tree)
    result_put = registry.price_commodity_option_with_metrics(
        put_option,
        "binomial_tree",
        market,
        ["delta", "gamma", "early_exercise_premium"],
    )

    print("\nComparison - American Put (Strike $70):")
    print(f"  Present Value:    ${result_put.present_value.amount:,.2f}")
    print(f"  Delta:            {result_put.metric('delta') or 0:.4f}")
    print(f"  Early Ex. Prem:   ${result_put.metric('early_exercise_premium') or 0:,.2f}")
    print("=" * 70)

    # Intrinsic value calculation
    forward_at_expiry = 75.00  # From forward curve
    intrinsic_call = max(0, forward_at_expiry - call_option.strike())
    print("\nIntrinsic Value Analysis (Call):")
    print(f"  Forward at Expiry: ${forward_at_expiry:.2f}")
    print(f"  Strike:            ${call_option.strike():.2f}")
    print(f"  Intrinsic:         ${intrinsic_call:.2f}")
    print(f"  Option Premium:    ${result_call.present_value.amount / 1000:.2f} per barrel")
    print(f"  Time Value:        ${(result_call.present_value.amount / 1000) - intrinsic_call:.2f} per barrel")
    print("=" * 70)


if __name__ == "__main__":
    main()
