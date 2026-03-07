"""Cliquet Option Example.

Demonstrates pricing and analysis of cliquet (ratchet) options with periodic resets.
"""

from datetime import date

from finstack.core.currency import USD
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.scalars import MarketScalar
from finstack.core.market_data.surfaces import VolSurface
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.instruments import CliquetOption
from finstack.valuations.pricer import create_standard_registry

from finstack import Money


def create_market_data(val_date: date) -> MarketContext:
    """Create market data for cliquet option pricing."""
    market = MarketContext()

    # Discount curve
    disc_curve = DiscountCurve(
        "USD.SOFR",
        val_date,
        [(0.0, 1.0), (1.0, 0.95), (2.0, 0.90), (3.0, 0.85)],
    )
    market.insert(disc_curve)

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
    market.insert_price("NVDA", MarketScalar.get_price(Money(1200.0, USD)))
    market.insert_price("NVDA.DIV", MarketScalar.unitless(0.002))

    return market


def example_quarterly_cliquet():
    """Example: Cliquet option with quarterly resets."""
    # Quarterly reset dates over 1 year
    # NOTE: reset_dates must be future dates
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
        # Removed start_date
        reset_dates=reset_dates,
        local_cap=0.10,  # 10% cap per reset period
        global_cap=0.30,  # 30% total cap
        notional=Money(100000.0, USD),
        discount_curve="USD.SOFR",
        spot_id="NVDA",
        vol_surface="NVDA.VOL",
        div_yield_id="NVDA.DIV",
    )

    # Price the cliquet
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.get_price(cliquet, "monte_carlo_gbm", market, as_of=val_date)

    return cliquet, result


def example_annual_cliquet():
    """Example: Cliquet option with annual resets."""
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
        # Removed start_date
        reset_dates=reset_dates,
        local_cap=0.25,  # 25% cap per year
        global_cap=0.60,  # 60% total cap over 3 years
        notional=Money(500000.0, USD),
        discount_curve="USD.SOFR",
        spot_id="NVDA",
        vol_surface="NVDA.VOL",
        div_yield_id="NVDA.DIV",
    )

    # Price the cliquet
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.get_price(cliquet, "monte_carlo_gbm", market, as_of=val_date)

    return cliquet, result


def example_monthly_cliquet():
    """Example: Cliquet option with monthly resets."""
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
        # Removed start_date
        reset_dates=reset_dates,
        local_cap=0.05,  # 5% cap per month
        global_cap=0.20,  # 20% total cap
        notional=Money(250000.0, USD),
        discount_curve="USD.SOFR",
        spot_id="NVDA",
        vol_surface="NVDA.VOL",
        div_yield_id="NVDA.DIV",
    )

    # Price the cliquet
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.get_price(cliquet, "monte_carlo_gbm", market, as_of=val_date)

    return cliquet, result


def main() -> None:
    """Run all cliquet option examples."""
    example_quarterly_cliquet()
    example_annual_cliquet()
    example_monthly_cliquet()


if __name__ == "__main__":
    main()
