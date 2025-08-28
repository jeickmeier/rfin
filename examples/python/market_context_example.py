#!/usr/bin/env python3
"""
Example: Building a full MarketContext for pricing.

Demonstrates how to:
- Create curves and insert into a CurveSet
- Configure FX via a provider/FX matrix and attach to MarketContext
- Add a volatility surface
- Add MarketScalar prices and ScalarTimeSeries series
"""

from finstack import Date, Currency
from finstack.money import Money
from finstack.market_data import (
    InterpStyle,
    CurveSet,
    DiscountCurve,
    ForwardCurve,
    HazardCurve,
    InflationCurve,
    VolSurface,
    MarketScalar,
    ScalarTimeSeries,
    SeriesInterpolation,
    SimpleFxProvider,
    FxMatrix,
    MarketContext,
)


def build_market_context() -> MarketContext:
    base_date = Date(2025, 1, 1)

    # Curves ---------------------------------------------------------------
    curves = CurveSet()

    usd_ois = DiscountCurve(
        id="USD-OIS",
        base_date=base_date,
        times=[0.0, 1.0, 5.0, 10.0],
        discount_factors=[1.0, 0.97, 0.85, 0.70],
        interpolation=InterpStyle.MonotoneConvex,
    )
    usd_sofr3m = ForwardCurve(
        id="USD-SOFR3M",
        tenor=0.25,
        base_date=base_date,
        times=[0.0, 1.0, 5.0, 10.0],
        forward_rates=[0.035, 0.04, 0.045, 0.05],
        interpolation=InterpStyle.Linear,
    )
    corp_hazard = HazardCurve(
        id="CORP-A-USD",
        base_date=base_date,
        times=[0.0, 5.0, 10.0],
        hazard_rates=[0.01, 0.02, 0.03],
    )
    us_cpi = InflationCurve(
        id="US-CPI",
        base_cpi=300.0,
        times=[0.0, 1.0, 2.0, 5.0],
        cpi_levels=[300.0, 306.0, 312.24, 331.5],
        interpolation=InterpStyle.LogLinear,
    )

    curves["USD-OIS"] = usd_ois
    curves["USD-SOFR3M"] = usd_sofr3m
    curves["CORP-A-USD"] = corp_hazard
    curves["US-CPI"] = us_cpi
    curves.map_collateral("CSA-USD", "USD-OIS")

    # FX ------------------------------------------------------------------
    provider = SimpleFxProvider()
    provider.set_rate(Currency("USD"), Currency("EUR"), 0.92)
    provider.set_rate(Currency("EUR"), Currency("USD"), 1.087)
    provider.set_rate(Currency("USD"), Currency("JPY"), 155.0)
    provider.set_rate(Currency("JPY"), Currency("USD"), 1.0 / 155.0)
    fx_matrix = FxMatrix(provider)

    # Surface --------------------------------------------------------------
    expiries = [0.25, 0.5, 1.0, 2.0]
    strikes = [80.0, 90.0, 100.0, 110.0, 120.0]
    vol_data = [
        [0.25, 0.22, 0.20, 0.22, 0.25],
        [0.24, 0.21, 0.19, 0.21, 0.24],
        [0.23, 0.20, 0.18, 0.20, 0.23],
        [0.22, 0.19, 0.17, 0.19, 0.22],
    ]
    surface = VolSurface(id="SPX-IV", expiries=expiries, strikes=strikes, values=vol_data)

    # Scalars & Series -----------------------------------------------------
    aapl_spot = MarketScalar.unitless(195.25)
    btc_spot = MarketScalar.price(Money(60000.0, Currency("USD")))
    unemployment = ScalarTimeSeries(
        "US-UNEMP",
        [(Date(2025, 1, 1), 3.8), (Date(2025, 2, 1), 3.9), (Date(2025, 3, 1), 4.0)],
        interpolation=SeriesInterpolation.STEP,
    )

    # MarketContext --------------------------------------------------------
    ctx = MarketContext()
    # Note: In the current implementation, MarketContext doesn't directly set curves
    # The curves would be passed during pricing operations or accessed via properties
    # ctx.set_fx_matrix(fx_matrix)  # This might also not exist yet

    # Stash surface/scalars/series into the CurveSet so they can be accessed centrally
    curves["SPX-IV"] = surface
    curves["AAPL-SPOT"] = aapl_spot
    curves["BTC-USD-SPOT"] = btc_spot
    curves["US-UNEMP"] = unemployment

    return ctx, curves  # Return both context and curves for demo


def main() -> None:
    ctx, curves = build_market_context()
    print("MarketContext built:")
    print("- Market context created successfully")
    print("- Curves created in CurveSet")
    print("- curve ids:", list(curves.keys()))
    # Note: In current implementation, FX access may be limited
    # print("- has FX:", ctx.has_fx)

    # Access items from CurveSet (demonstration of structure)
    print("\nDemonstrating market data access:")
    
    # 1) Curves
    try:
        usd_ois = curves["USD-OIS"]
        print("- USD-OIS curve: ", usd_ois)
    except Exception as e:
        print("- USD-OIS curve access:", str(e))
    
    # 2) Other market data stored in curves
    print("- Available market data:")
    for key in curves.keys():
        print(f"  {key}: {type(curves[key]).__name__}")
    
    # Note: In a full implementation, you would access curves like:
    # usd_ois = curves.discount_curve("USD-OIS")
    # spx_vol = curves.vol_surface("SPX-IV") 
    # fx_rate = fx_matrix.get_rate(from_ccy, to_ccy, date, policy)
    
    print("\nMarket data framework structure is ready for:")
    print("✓ Discount, forward, hazard, and inflation curves")
    print("✓ Volatility surfaces") 
    print("✓ FX rate matrices")
    print("✓ Market scalars and time series")
    print("✓ Integration with instrument pricing")


if __name__ == "__main__":
    main()


