"""Lookback Option Example.

Demonstrates pricing and analysis of lookback options with fixed and floating strikes.
"""

from datetime import date

from finstack.core.currency import USD
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.scalars import MarketScalar
from finstack.core.market_data.surfaces import VolSurface
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.instruments import LookbackOption, LookbackType
from finstack.valuations.pricer import create_standard_registry

from finstack import Money


def create_market_data(val_date: date) -> MarketContext:
    """Create market data for lookback option pricing."""
    market = MarketContext()

    # Discount curve
    disc_curve = DiscountCurve(
        "USD.SOFR",
        val_date,
        [(0.0, 1.0), (0.5, 0.975), (1.0, 0.95)],
    )
    market.insert(disc_curve)

    # Volatility surface
    vol_surface = VolSurface(
        "TSLA.VOL",
        expiries=[0.5, 1.0],
        strikes=[200.0, 250.0, 300.0, 350.0, 400.0],
        grid=[
            [0.55, 0.50, 0.45, 0.50, 0.55],
            [0.60, 0.55, 0.50, 0.55, 0.60],
        ],
    )
    market.insert_surface(vol_surface)

    # Market prices
    market.insert_price("TSLA", MarketScalar.get_price(Money(300.0, USD)))
    market.insert_price("TSLA.DIV", MarketScalar.unitless(0.0))

    return market


def example_fixed_strike_call():
    """Example: Fixed-strike lookback call option."""
    # Fixed strike lookback call: payoff = max(S_max - K, 0)
    # where S_max is the maximum spot price during option life
    option = LookbackOption.builder(
        instrument_id="LOOKBACK_001",
        ticker="TSLA",
        strike=300.0,  # Fixed strike
        option_type="call",
        lookback_type="fixed_strike",
        expiry=date(2025, 12, 31),
        notional=Money(100000.0, USD),
        discount_curve="USD.SOFR",
        spot_id="TSLA",
        vol_surface="TSLA.VOL",
        div_yield_id="TSLA.DIV",
    )

    # Price the option
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.get_price(option, "monte_carlo_gbm", market, as_of=val_date)

    return option, result


def example_fixed_strike_put():
    """Example: Fixed-strike lookback put option."""
    # Fixed strike lookback put: payoff = max(K - S_min, 0)
    # where S_min is the minimum spot price during option life
    option = LookbackOption.builder(
        instrument_id="LOOKBACK_002",
        ticker="TSLA",
        strike=300.0,  # Fixed strike
        option_type="put",
        lookback_type="fixed_strike",
        expiry=date(2025, 6, 30),
        notional=Money(200000.0, USD),
        discount_curve="USD.SOFR",
        spot_id="TSLA",
        vol_surface="TSLA.VOL",
        div_yield_id="TSLA.DIV",
    )

    # Price the option
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.get_price(option, "monte_carlo_gbm", market, as_of=val_date)

    return option, result


def example_floating_strike_call():
    """Example: Floating-strike lookback call option."""
    # Floating strike lookback call: payoff = S_T - S_min
    # Strike is set to minimum price during option life
    # No fixed strike needed
    option = LookbackOption.builder(
        instrument_id="LOOKBACK_003",
        ticker="TSLA",
        strike=None,  # No fixed strike for floating lookback
        option_type="call",
        lookback_type="floating_strike",
        expiry=date(2025, 12, 31),
        notional=Money(150000.0, USD),
        discount_curve="USD.SOFR",
        spot_id="TSLA",
        vol_surface="TSLA.VOL",
        div_yield_id="TSLA.DIV",
    )

    # Price the option
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.get_price(option, "monte_carlo_gbm", market, as_of=val_date)

    return option, result


def example_floating_strike_put():
    """Example: Floating-strike lookback put option."""
    # Floating strike lookback put: payoff = S_max - S_T
    # Strike is set to maximum price during option life
    option = LookbackOption.builder(
        instrument_id="LOOKBACK_004",
        ticker="TSLA",
        strike=None,  # No fixed strike
        option_type="put",
        lookback_type="floating_strike",
        expiry=date(2025, 6, 30),
        notional=Money(75000.0, USD),
        discount_curve="USD.SOFR",
        spot_id="TSLA",
        vol_surface="TSLA.VOL",
        div_yield_id="TSLA.DIV",
    )

    # Price the option
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.get_price(option, "monte_carlo_gbm", market, as_of=val_date)

    return option, result


def example_lookback_type_enum() -> None:
    """Example: Using LookbackType enum."""
    # Access enum constants

    # Parse from string
    LookbackType.from_name("floating_strike")


def main() -> None:
    """Run all lookback option examples."""
    example_fixed_strike_call()
    example_fixed_strike_put()
    example_floating_strike_call()
    example_floating_strike_put()
    example_lookback_type_enum()


if __name__ == "__main__":
    main()
