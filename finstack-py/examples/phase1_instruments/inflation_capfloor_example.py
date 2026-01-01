"""
Inflation Cap/Floor Example
============================

Demonstrates pricing a year-on-year inflation cap using Black-76 or
Bachelier volatility models.
"""

from finstack import Money, Date
from finstack.core.market_data import (
    MarketContext,
    DiscountCurve,
    InflationIndex,
    VolSurface,
    InterpolationStyle,
)
from finstack.valuations.instruments import InflationCapFloor
from finstack.valuations.pricer import create_standard_registry


def main():
    """
    Price a 5-year inflation cap on US CPI with:
    - Notional: $10M
    - Strike: 2.5% YoY inflation
    - Annual caplets
    - Black-76 vol model
    """
    
    # Market data setup
    as_of = Date(2024, 1, 15)
    base_date = as_of
    
    # USD discount curve (OIS)
    tenors = [0.0, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0]
    dfs = [1.0, 0.9500, 0.9100, 0.8750, 0.8200, 0.7500, 0.6500]
    
    discount_curve = DiscountCurve(
        curve_id="USD-OIS",
        base_date=base_date,
        tenors=tenors,
        values=dfs,
        interpolation=InterpolationStyle.LogLinear(),
    )
    
    # Inflation index (US CPI-U)
    index_dates = [
        Date(2024, 1, 1),
        Date(2025, 1, 1),
        Date(2026, 1, 1),
        Date(2027, 1, 1),
        Date(2028, 1, 1),
        Date(2029, 1, 1),
    ]
    index_levels = [300.0, 307.5, 315.0, 322.5, 330.0, 337.5]  # CPI levels
    
    inflation_index = InflationIndex(
        index_id="US-CPI-U",
        dates=index_dates,
        values=index_levels,
        interpolation=InterpolationStyle.Linear(),
    )
    
    # Volatility surface for inflation (YoY vol)
    vol_tenors = [1.0, 2.0, 3.0, 5.0, 7.0, 10.0]  # Years
    vol_strikes = [0.01, 0.02, 0.025, 0.03, 0.04]  # Strike inflation rates
    
    # Black-76 lognormal vols (grid: tenors × strikes)
    vol_matrix = [
        [0.30, 0.28, 0.27, 0.26, 0.24],  # 1Y
        [0.32, 0.30, 0.29, 0.28, 0.26],  # 2Y
        [0.33, 0.31, 0.30, 0.29, 0.27],  # 3Y
        [0.35, 0.33, 0.32, 0.31, 0.29],  # 5Y
        [0.36, 0.34, 0.33, 0.32, 0.30],  # 7Y
        [0.37, 0.35, 0.34, 0.33, 0.31],  # 10Y
    ]
    
    vol_surface = VolSurface(
        surface_id="USD-INFLATION-VOL",
        base_date=base_date,
        tenors=vol_tenors,
        strikes=vol_strikes,
        vol_matrix=vol_matrix,
        interpolation=InterpolationStyle.Linear(),
    )
    
    # Build market context
    market = MarketContext()
    market.insert_discount(discount_curve)
    market.insert_inflation_index(inflation_index)
    market.insert_vol_surface(vol_surface)
    
    # Create inflation cap
    cap = (
        InflationCapFloor.builder("INFLATION_CAP_001")
        .option_type("cap")
        .notional(10_000_000, "USD")
        .strike_rate(0.025)  # 2.5% YoY cap
        .start_date(Date(2024, 1, 15))
        .end_date(Date(2029, 1, 15))
        .frequency("annual")
        .inflation_index_id("US-CPI-U")
        .disc_id("USD-OIS")
        .vol_surface_id("USD-INFLATION-VOL")
        .calendar_id("USNY")
        .day_count("Act/365F")
        .vol_model("black76")  # Lognormal Black-76
        .build()
    )
    
    # Price the cap
    registry = create_standard_registry()
    
    result = registry.price_inflation_cap_floor_with_metrics(
        cap,
        "black76",
        market,
        ["num_caplets", "vega", "theta", "dv01"],
    )
    
    # Display results
    print("=" * 70)
    print("Inflation Cap Valuation Results")
    print("=" * 70)
    print(f"Instrument ID:      {cap.instrument_id()}")
    print(f"Option Type:        {cap.option_type()}")
    print(f"Notional:           {cap.notional().amount:,.2f} {cap.notional().currency.code}")
    print(f"Strike Rate:        {cap.strike_rate() * 100:.2f}%")
    print(f"Start Date:         {cap.start_date()}")
    print(f"End Date:           {cap.end_date()}")
    print(f"Frequency:          {cap.frequency()}")
    print(f"Inflation Index:    {cap.inflation_index_id()}")
    print(f"Vol Model:          {cap.vol_model()}")
    print(f"\nPresent Value:      {result.present_value.amount:,.2f} {result.present_value.currency.code}")
    print(f"\nMetrics:")
    print(f"  Number of Caplets: {int(result.metric('num_caplets') or 0)}")
    print(f"  Vega:              {result.metric('vega') or 0:,.2f}")
    print(f"  Theta (1d):        {result.metric('theta') or 0:,.2f}")
    print(f"  DV01:              {result.metric('dv01') or 0:,.2f}")
    print("=" * 70)
    
    # Create floor for comparison
    floor = (
        InflationCapFloor.builder("INFLATION_FLOOR_001")
        .option_type("floor")
        .notional(10_000_000, "USD")
        .strike_rate(0.01)  # 1.0% YoY floor
        .start_date(Date(2024, 1, 15))
        .end_date(Date(2029, 1, 15))
        .frequency("annual")
        .inflation_index_id("US-CPI-U")
        .disc_id("USD-OIS")
        .vol_surface_id("USD-INFLATION-VOL")
        .vol_model("black76")
        .build()
    )
    
    floor_result = registry.price_inflation_cap_floor(floor, "black76", market)
    
    print(f"\nFor comparison, a 1.0% floor is worth:")
    print(f"  Present Value:    {floor_result.present_value.amount:,.2f} {floor_result.present_value.currency.code}")
    print("=" * 70)


if __name__ == "__main__":
    main()
