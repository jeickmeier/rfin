"""
Barrier Option Example

Demonstrates pricing and analysis of barrier options with various barrier types.
"""

from datetime import date

from finstack import Money
from finstack.core.currency import USD
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.scalars import MarketScalar
from finstack.core.market_data.surfaces import VolSurface
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.instruments import BarrierOption, BarrierType
from finstack.valuations.pricer import create_standard_registry


def create_market_data(val_date: date) -> MarketContext:
    """Create market data for barrier option pricing."""
    market = MarketContext()

    # Discount curve
    disc_curve = DiscountCurve(
        "USD.SOFR",
        val_date,
        [(0.0, 1.0), (0.5, 0.975), (1.0, 0.95)],
    )
    market.insert_discount(disc_curve)

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
    market.insert_price("MSFT", MarketScalar.price(Money(450.0, USD)))
    market.insert_price("MSFT.DIV", MarketScalar.unitless(0.008))

    return market


def example_up_and_out_call():
    """Example: Up-and-out barrier call option."""
    print("\n" + "=" * 80)
    print("UP-AND-OUT BARRIER CALL")
    print("=" * 80)
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

    print(f"\nInstrument: {option}")
    print(f"  Ticker: {option.ticker}")
    print(f"  Strike: {option.strike}")
    print(f"  Barrier: {option.barrier}")
    print(f"  Option Type: {option.option_type}")
    print(f"  Barrier Type: {option.barrier_type}")
    print(f"  Expiry: {option.expiry}")

    # Price the option
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.price(option, "monte_carlo_gbm", market, as_of=val_date)

    print(f"\nPricing Results:")
    print(f"  Present Value: {result.value}")
    print(f"  Currency: {result.value.currency}")

    return option, result


def example_down_and_in_put():
    """Example: Down-and-in barrier put option."""
    print("\n" + "=" * 80)
    print("DOWN-AND-IN BARRIER PUT")
    print("=" * 80)
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

    print(f"\nInstrument: {option}")
    print(f"  Ticker: {option.ticker}")
    print(f"  Strike: {option.strike}")
    print(f"  Barrier: {option.barrier}")
    print(f"  Option Type: {option.option_type}")
    print(f"  Barrier Type: {option.barrier_type}")
    print(f"  Expiry: {option.expiry}")

    # Price the option
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.price(option, "monte_carlo_gbm", market, as_of=val_date)

    print(f"\nPricing Results:")
    print(f"  Present Value: {result.value}")
    print(f"  Currency: {result.value.currency}")

    return option, result


def example_down_and_out_put():
    """Example: Down-and-out barrier put option."""
    print("\n" + "=" * 80)
    print("DOWN-AND-OUT BARRIER PUT")
    print("=" * 80)

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

    print(f"\nInstrument: {option}")
    print(f"  Ticker: {option.ticker}")
    print(f"  Strike: {option.strike}")
    print(f"  Barrier: {option.barrier}")
    print(f"  Option Type: {option.option_type}")
    print(f"  Barrier Type: {option.barrier_type}")

    # Price the option
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.price(option, "monte_carlo_gbm", market, as_of=val_date)

    print(f"\nPricing Results:")
    print(f"  Present Value: {result.value}")

    return option, result


def example_up_and_in_call():
    """Example: Up-and-in barrier call option."""
    print("\n" + "=" * 80)
    print("UP-AND-IN BARRIER CALL")
    print("=" * 80)

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

    print(f"\nInstrument: {option}")
    print(f"  Ticker: {option.ticker}")
    print(f"  Strike: {option.strike}")
    print(f"  Barrier: {option.barrier}")
    print(f"  Option Type: {option.option_type}")
    print(f"  Barrier Type: {option.barrier_type}")

    # Price the option
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.price(option, "monte_carlo_gbm", market, as_of=val_date)

    print(f"\nPricing Results:")
    print(f"  Present Value: {result.value}")

    return option, result


def example_barrier_type_enum():
    """Example: Using BarrierType enum."""
    print("\n" + "=" * 80)
    print("BARRIER TYPE ENUM")
    print("=" * 80)

    # Access enum constants
    up_out = BarrierType.UP_AND_OUT
    up_in = BarrierType.UP_AND_IN
    down_out = BarrierType.DOWN_AND_OUT
    down_in = BarrierType.DOWN_AND_IN

    print(f"\nBarrier Types:")
    print(f"  Up-and-Out: {up_out}")
    print(f"  Up-and-In: {up_in}")
    print(f"  Down-and-Out: {down_out}")
    print(f"  Down-and-In: {down_in}")

    # Parse from string
    from_str = BarrierType.from_name("down_and_in")
    print(f"\nParsed from string 'down_and_in': {from_str}")
    print(f"  Name: {from_str.name}")


def main():
    """Run all barrier option examples."""
    print("\n" + "=" * 80)
    print("BARRIER OPTION EXAMPLES")
    print("=" * 80)

    example_up_and_out_call()
    example_down_and_in_put()
    example_down_and_out_put()
    example_up_and_in_call()
    example_barrier_type_enum()

    print("\n" + "=" * 80)
    print("Examples completed successfully!")
    print("=" * 80 + "\n")


if __name__ == "__main__":
    main()
