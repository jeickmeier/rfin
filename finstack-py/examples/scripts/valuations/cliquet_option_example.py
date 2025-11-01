"""
Cliquet Option Example

Demonstrates pricing and analysis of cliquet (ratchet) options with periodic resets.
"""

from datetime import date
from finstack import Money
from finstack.core.currency import USD
from finstack.core.market_data import MarketContext
from finstack.core.market_data.scalars import MarketScalar
from finstack.core.market_data.surfaces import VolSurface
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.core.market_data import MarketContext
from finstack.valuations.instruments import CliquetOption
from finstack.valuations.pricer import create_standard_registry


def create_market_data(val_date: date) -> MarketContext:
    """Create market data for cliquet option pricing."""
    market = MarketContext()

    # Discount curve
    disc_curve = DiscountCurve(
        "USD.SOFR",
        val_date,
        [(0.0, 1.0), (1.0, 0.95), (2.0, 0.90), (3.0, 0.85)],
    )
    market.insert_discount(disc_curve)

    # Volatility surface (expiries in years)
    vol_surface = VolSurface(
        "NVDA.VOL",
        expiries=[1.0, 2.0, 3.0],
        strikes=[800.0, 1000.0, 1200.0, 1400.0, 1600.0],
        grid=[
            [0.45, 0.40, 0.35, 0.40, 0.45],
            [0.48, 0.43, 0.38, 0.43, 0.48],
            [0.50, 0.45, 0.40, 0.45, 0.50],
        ],
    )
    market.insert_surface(vol_surface)

    # Prices
    market.insert_price("NVDA", MarketScalar.price(Money(1200.0, USD)))
    market.insert_price("NVDA.DIV", MarketScalar.unitless(0.002))

    return market


def example_quarterly_cliquet():
    """Example: Cliquet option with quarterly resets."""
    print("\n" + "=" * 80)
    print("QUARTERLY CLIQUET OPTION")
    print("=" * 80)
    
    
    # Quarterly reset dates over 1 year
    reset_dates = [
        date(2025, 4, 1),
        date(2025, 7, 1),
        date(2025, 10, 1),
        date(2026, 1, 1),
    ]
    
    # Local cap: 10% per period, Global cap: 30% total
    cliquet = CliquetOption.builder(
        instrument_id="CLIQUET_001",
        ticker="NVDA",
        reset_dates=reset_dates,
        local_cap=0.10,  # 10% cap per reset period
        global_cap=0.30,  # 30% total cap
        notional=Money(100000.0, USD),
        discount_curve="USD.SOFR",
        spot_id="NVDA",
        vol_surface="NVDA.VOL",
        dividend_yield_id="NVDA.DIV",
    )
    
    print(f"\nInstrument: {cliquet}")
    print(f"  Ticker: {cliquet.ticker}")
    print(f"  Notional: {cliquet.notional}")
    print(f"  Local Cap: {cliquet.local_cap:.1%}")
    print(f"  Global Cap: {cliquet.global_cap:.1%}")
    print(f"  Number of Resets: {len(cliquet.reset_dates)}")
    print(f"  Reset Dates: {cliquet.reset_dates}")
    
    # Price the cliquet
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.price(cliquet, "monte_carlo_gbm", market, as_of=val_date)
    
    print(f"\nPricing Results:")
    print(f"  Present Value: {result.value}")
    print(f"  Currency: {result.value.currency}")
    
    return cliquet, result


def example_annual_cliquet():
    """Example: Cliquet option with annual resets."""
    print("\n" + "=" * 80)
    print("ANNUAL CLIQUET OPTION")
    print("=" * 80)
    
    
    # Annual reset dates over 3 years
    reset_dates = [
        date(2026, 1, 1),
        date(2027, 1, 1),
        date(2028, 1, 1),
    ]
    
    # Higher caps for annual resets
    cliquet = CliquetOption.builder(
        instrument_id="CLIQUET_002",
        ticker="NVDA",
        reset_dates=reset_dates,
        local_cap=0.25,  # 25% cap per year
        global_cap=0.60,  # 60% total cap over 3 years
        notional=Money(500000.0, USD),
        discount_curve="USD.SOFR",
        spot_id="NVDA",
        vol_surface="NVDA.VOL",
        dividend_yield_id="NVDA.DIV",
    )
    
    print(f"\nInstrument: {cliquet}")
    print(f"  Ticker: {cliquet.ticker}")
    print(f"  Notional: {cliquet.notional}")
    print(f"  Local Cap: {cliquet.local_cap:.1%}")
    print(f"  Global Cap: {cliquet.global_cap:.1%}")
    print(f"  Number of Resets: {len(cliquet.reset_dates)}")
    
    # Price the cliquet
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.price(cliquet, "monte_carlo_gbm", market, as_of=val_date)
    
    print(f"\nPricing Results:")
    print(f"  Present Value: {result.value}")
    print(f"  Currency: {result.value.currency}")
    
    return cliquet, result


def example_monthly_cliquet():
    """Example: Cliquet option with monthly resets."""
    print("\n" + "=" * 80)
    print("MONTHLY CLIQUET OPTION")
    print("=" * 80)
    
    
    # Monthly reset dates over 6 months
    reset_dates = [
        date(2025, 2, 1),
        date(2025, 3, 1),
        date(2025, 4, 1),
        date(2025, 5, 1),
        date(2025, 6, 1),
        date(2025, 7, 1),
    ]
    
    # Lower local caps for monthly resets
    cliquet = CliquetOption.builder(
        instrument_id="CLIQUET_003",
        ticker="NVDA",
        reset_dates=reset_dates,
        local_cap=0.05,  # 5% cap per month
        global_cap=0.20,  # 20% total cap
        notional=Money(250000.0, USD),
        discount_curve="USD.SOFR",
        spot_id="NVDA",
        vol_surface="NVDA.VOL",
        dividend_yield_id="NVDA.DIV",
    )
    
    print(f"\nInstrument: {cliquet}")
    print(f"  Ticker: {cliquet.ticker}")
    print(f"  Notional: {cliquet.notional}")
    print(f"  Local Cap: {cliquet.local_cap:.1%}")
    print(f"  Global Cap: {cliquet.global_cap:.1%}")
    print(f"  Number of Resets: {len(cliquet.reset_dates)}")
    
    # Price the cliquet
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.price(cliquet, "monte_carlo_gbm", market, as_of=val_date)
    
    print(f"\nPricing Results:")
    print(f"  Present Value: {result.value}")
    print(f"  Currency: {result.value.currency}")
    
    print(f"\n  Explanation:")
    print(f"    - Cliquet options lock in gains at each reset date")
    print(f"    - Local cap limits gains per period")
    print(f"    - Global cap limits total cumulative gains")
    print(f"    - Popular for structured notes and employee compensation")
    
    return cliquet, result


def main():
    """Run all cliquet option examples."""
    print("\n" + "=" * 80)
    print("CLIQUET OPTION EXAMPLES")
    print("=" * 80)
    
    example_quarterly_cliquet()
    example_annual_cliquet()
    example_monthly_cliquet()
    
    print("\n" + "=" * 80)
    print("Examples completed successfully!")
    print("=" * 80 + "\n")


if __name__ == "__main__":
    main()

