"""Barrier Option Example.

Demonstrates pricing and analysis of barrier options with various barrier types.
"""

from datetime import date

from finstack.core.currency import USD
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.scalars import MarketScalar
from finstack.core.market_data.surfaces import VolSurface
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.instruments import BarrierOption, BarrierType
from finstack.valuations.pricer import create_standard_registry

from finstack import Money


def create_market_data(val_date: date) -> MarketContext:
    """Create market data for barrier option pricing."""
    market = MarketContext()

    # Discount curve
    disc_curve = DiscountCurve(
        "USD.SOFR",
        val_date,
        [(0.0, 1.0), (0.5, 0.975), (1.0, 0.95)],
    )
    market.insert(disc_curve)

    # Volatility surface (expiries in years)
    vol_surface = VolSurface(
        "MSFT.VOL",
        expiries=[0.5, 1.0],
        strikes=[350.0, 400.0, 450.0, 500.0, 550.0],
        grid=[
            [0.28, 0.25, 0.22, 0.25, 0.28],
            [0.30, 0.27, 0.24, 0.27, 0.30],
        ],
    )
    market.insert_surface(vol_surface)

    # Spot and dividend
    market.insert_price("MSFT", MarketScalar.get_price(Money(450.0, USD)))
    market.insert_price("MSFT.DIV", MarketScalar.unitless(0.008))

    return market


def example_up_and_out_call():
    """Example: Up-and-out barrier call option."""
    val_date = date(2025, 1, 1)
    # Up-and-out call: knocks out if spot goes above barrier
    # Cheaper than vanilla call since it can knock out
    option = BarrierOption.builder(
        instrument_id="BARRIER_001",
        ticker="MSFT",
        strike=450.0,
        barrier=550.0,  # Barrier above current spot
        option_type="call",
        barrier_type="up_and_out",
        expiry=date(2025, 12, 31),
        notional=Money(100000.0, USD),
        discount_curve="USD.SOFR",
        spot_id="MSFT",
        vol_surface="MSFT.VOL",
        div_yield_id="MSFT.DIV",
        use_gobet_miri=False,
    )

    # Price the option
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.get_price(option, "monte_carlo_gbm", market, as_of=val_date)

    return option, result


def example_down_and_in_put():
    """Example: Down-and-in barrier put option."""
    val_date = date(2025, 1, 1)
    # Down-and-in put: activates only if spot falls below barrier
    # Provides protection only if market falls significantly
    option = BarrierOption.builder(
        instrument_id="BARRIER_002",
        ticker="MSFT",
        strike=450.0,
        barrier=350.0,  # Barrier below current spot
        option_type="put",
        barrier_type="down_and_in",
        expiry=date(2025, 12, 31),
        notional=Money(200000.0, USD),
        discount_curve="USD.SOFR",
        spot_id="MSFT",
        vol_surface="MSFT.VOL",
        div_yield_id="MSFT.DIV",
        use_gobet_miri=True,  # Use Gobet-Miri approximation
    )

    # Price the option
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.get_price(option, "monte_carlo_gbm", market, as_of=val_date)

    return option, result


def example_down_and_out_put():
    """Example: Down-and-out barrier put option."""
    # Down-and-out put: knocks out if spot falls below barrier
    # Provides protection only if market doesn't fall too far
    option = BarrierOption.builder(
        instrument_id="BARRIER_003",
        ticker="MSFT",
        strike=450.0,
        barrier=380.0,  # Barrier below strike
        option_type="put",
        barrier_type="down_and_out",
        expiry=date(2025, 6, 30),
        notional=Money(150000.0, USD),
        discount_curve="USD.SOFR",
        spot_id="MSFT",
        vol_surface="MSFT.VOL",
        div_yield_id="MSFT.DIV",
        use_gobet_miri=False,
    )

    # Price the option
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.get_price(option, "monte_carlo_gbm", market, as_of=val_date)

    return option, result


def example_up_and_in_call():
    """Example: Up-and-in barrier call option."""
    # Up-and-in call: activates only if spot rises above barrier
    # Bet on strong upward movement
    option = BarrierOption.builder(
        instrument_id="BARRIER_004",
        ticker="MSFT",
        strike=450.0,
        barrier=500.0,  # Barrier above current spot
        option_type="call",
        barrier_type="up_and_in",
        expiry=date(2025, 12, 31),
        notional=Money(75000.0, USD),
        discount_curve="USD.SOFR",
        spot_id="MSFT",
        vol_surface="MSFT.VOL",
        div_yield_id="MSFT.DIV",
        use_gobet_miri=False,
    )

    # Price the option
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.get_price(option, "monte_carlo_gbm", market, as_of=val_date)

    return option, result


def example_barrier_type_enum() -> None:
    """Example: Using BarrierType enum."""
    # Access enum constants

    # Parse from string
    BarrierType.from_name("down_and_in")


def main() -> None:
    """Run all barrier option examples."""
    example_up_and_out_call()
    example_down_and_in_put()
    example_down_and_out_put()
    example_up_and_in_call()
    example_barrier_type_enum()


if __name__ == "__main__":
    main()
