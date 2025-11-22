"""
Lookback Option Example

Demonstrates pricing and analysis of lookback options with fixed and floating strikes.
"""

from datetime import date
from finstack import Money
from finstack.core.currency import USD
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.scalars import MarketScalar
from finstack.core.market_data.surfaces import VolSurface
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.core.market_data.context import MarketContext
from finstack.valuations.instruments import LookbackOption, LookbackType
from finstack.valuations.pricer import create_standard_registry


def create_market_data(val_date: date) -> MarketContext:
    """Create market data for lookback option pricing."""
    market = MarketContext()

    # Discount curve
    disc_curve = DiscountCurve(
        "USD.SOFR",
        val_date,
        [(0.0, 1.0), (0.5, 0.975), (1.0, 0.95)],
    )
    market.insert_discount(disc_curve)

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
    market.insert_price("TSLA", MarketScalar.price(Money(300.0, USD)))
    market.insert_price("TSLA.DIV", MarketScalar.unitless(0.0))

    return market


def example_fixed_strike_call():
    """Example: Fixed-strike lookback call option."""
    print("\n" + "=" * 80)
    print("FIXED-STRIKE LOOKBACK CALL")
    print("=" * 80)
    
    
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
    
    print(f"\nInstrument: {option}")
    print(f"  Ticker: {option.ticker}")
    print(f"  Strike: {option.strike}")
    print(f"  Option Type: {option.option_type}")
    print(f"  Lookback Type: {option.lookback_type}")
    print(f"  Expiry: {option.expiry}")
    
    # Price the option
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.price(option, "monte_carlo_gbm", market, as_of=val_date)
    
    print(f"\nPricing Results:")
    print(f"  Present Value: {result.value}")
    print(f"  Currency: {result.value.currency}")
    
    print(f"\n  Explanation:")
    print(f"    - Payoff based on maximum spot price achieved")
    print(f"    - Payoff = max(S_max - {option.strike.amount}, 0)")
    print(f"    - Always worth at least as much as standard call")
    print(f"    - Eliminates timing risk for the holder")
    
    return option, result


def example_fixed_strike_put():
    """Example: Fixed-strike lookback put option."""
    print("\n" + "=" * 80)
    print("FIXED-STRIKE LOOKBACK PUT")
    print("=" * 80)
    
    
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
    
    print(f"\nInstrument: {option}")
    print(f"  Ticker: {option.ticker}")
    print(f"  Strike: {option.strike}")
    print(f"  Option Type: {option.option_type}")
    print(f"  Lookback Type: {option.lookback_type}")
    print(f"  Expiry: {option.expiry}")
    
    # Price the option
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.price(option, "monte_carlo_gbm", market, as_of=val_date)
    
    print(f"\nPricing Results:")
    print(f"  Present Value: {result.value}")
    print(f"  Currency: {result.value.currency}")
    
    print(f"\n  Explanation:")
    print(f"    - Payoff based on minimum spot price achieved")
    print(f"    - Payoff = max({option.strike.amount} - S_min, 0)")
    print(f"    - Valuable for downside protection")
    
    return option, result


def example_floating_strike_call():
    """Example: Floating-strike lookback call option."""
    print("\n" + "=" * 80)
    print("FLOATING-STRIKE LOOKBACK CALL")
    print("=" * 80)
    
    
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
    
    print(f"\nInstrument: {option}")
    print(f"  Ticker: {option.ticker}")
    print(f"  Strike: {option.strike}")  # Will be None
    print(f"  Option Type: {option.option_type}")
    print(f"  Lookback Type: {option.lookback_type}")
    print(f"  Expiry: {option.expiry}")
    
    # Price the option
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.price(option, "monte_carlo_gbm", market, as_of=val_date)
    
    print(f"\nPricing Results:")
    print(f"  Present Value: {result.value}")
    print(f"  Currency: {result.value.currency}")
    
    print(f"\n  Explanation:")
    print(f"    - Payoff = S_T - S_min (always positive)")
    print(f"    - Equivalent to buying at the lowest price")
    print(f"    - No strike price; purely path-dependent")
    print(f"    - Very expensive but guarantees profit if any upward movement")
    
    return option, result


def example_floating_strike_put():
    """Example: Floating-strike lookback put option."""
    print("\n" + "=" * 80)
    print("FLOATING-STRIKE LOOKBACK PUT")
    print("=" * 80)
    
    
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
    
    print(f"\nInstrument: {option}")
    print(f"  Ticker: {option.ticker}")
    print(f"  Strike: {option.strike}")  # Will be None
    print(f"  Option Type: {option.option_type}")
    print(f"  Lookback Type: {option.lookback_type}")
    print(f"  Expiry: {option.expiry}")
    
    # Price the option
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.price(option, "monte_carlo_gbm", market, as_of=val_date)
    
    print(f"\nPricing Results:")
    print(f"  Present Value: {result.value}")
    print(f"  Currency: {result.value.currency}")
    
    print(f"\n  Explanation:")
    print(f"    - Payoff = S_max - S_T (always positive)")
    print(f"    - Equivalent to selling at the highest price")
    print(f"    - Guarantees profit if any downward movement")
    
    return option, result


def example_lookback_type_enum():
    """Example: Using LookbackType enum."""
    print("\n" + "=" * 80)
    print("LOOKBACK TYPE ENUM")
    print("=" * 80)
    
    # Access enum constants
    fixed = LookbackType.FIXED_STRIKE
    floating = LookbackType.FLOATING_STRIKE
    
    print(f"\nLookback Types:")
    print(f"  Fixed Strike: {fixed}")
    print(f"  Floating Strike: {floating}")
    
    # Parse from string
    from_str = LookbackType.from_name("floating_strike")
    print(f"\nParsed from string 'floating_strike': {from_str}")
    print(f"  Name: {from_str.name}")


def main():
    """Run all lookback option examples."""
    print("\n" + "=" * 80)
    print("LOOKBACK OPTION EXAMPLES")
    print("=" * 80)
    
    example_fixed_strike_call()
    example_fixed_strike_put()
    example_floating_strike_call()
    example_floating_strike_put()
    example_lookback_type_enum()
    
    print("\n" + "=" * 80)
    print("Examples completed successfully!")
    print("=" * 80 + "\n")


if __name__ == "__main__":
    main()

