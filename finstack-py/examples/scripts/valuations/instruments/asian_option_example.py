"""
Asian Option Example

Demonstrates pricing and analysis of Asian options with arithmetic and geometric averaging.
"""

from datetime import date
from finstack import Money
from finstack.core.currency import USD
from finstack.core.market_data import MarketContext
from finstack.core.market_data.scalars import MarketScalar
from finstack.core.market_data.surfaces import VolSurface
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.instruments import AsianOption, AveragingMethod
from finstack.valuations.pricer import create_standard_registry


def create_market_data(val_date: date) -> MarketContext:
    """Create market data for Asian option pricing."""
    market = MarketContext()
    
    # Discount curve
    disc_curve = DiscountCurve(
        "USD.SOFR",
        val_date,
        [(0.0, 1.0), (1.0, 0.95), (2.0, 0.90)],
    )
    market.insert_discount(disc_curve)
    
    # Volatility surface for equity (expiries in years)
    vol_surface = VolSurface(
        "AAPL.VOL",
        expiries=[0.5, 1.0],
        strikes=[140.0, 150.0, 160.0, 170.0, 180.0],
        grid=[
            [0.25, 0.23, 0.21, 0.23, 0.25],  # 6M
            [0.27, 0.25, 0.23, 0.25, 0.27],  # 1Y
        ],
    )
    market.insert_surface(vol_surface)
    
    # Spot price and dividend yield
    market.insert_price("AAPL", MarketScalar.price(Money(150.0, USD)))
    market.insert_price("AAPL.DIV", MarketScalar.unitless(0.01))
    
    return market


def example_arithmetic_asian_call():
    """Example: Arithmetic average Asian call option."""
    print("\n" + "=" * 80)
    print("ARITHMETIC ASIAN CALL OPTION")
    print("=" * 80)
    
    val_date = date(2025, 1, 1)
    
    # Create Asian option with arithmetic averaging
    fixing_dates = [
        date(2025, 3, 1),
        date(2025, 6, 1),
        date(2025, 9, 1),
        date(2025, 12, 1),
    ]
    
    option = AsianOption.builder(
        instrument_id="ASIAN_CALL_001",
        ticker="AAPL",
        strike=150.0,
        expiry=date(2025, 12, 31),
        fixing_dates=fixing_dates,
        notional=Money(10000.0, USD),
        discount_curve="USD.SOFR",
        spot_id="AAPL",
        vol_surface="AAPL.VOL",
        averaging_method="arithmetic",
        option_type="call",
        div_yield_id="AAPL.DIV",
    )
    
    print(f"\nInstrument: {option}")
    print(f"  Ticker: {option.ticker}")
    print(f"  Strike: {option.strike}")
    print(f"  Averaging Method: {option.averaging_method}")
    print(f"  Option Type: {option.option_type}")
    print(f"  Expiry: {option.expiry}")
    print(f"  Number of Fixings: {len(option.fixing_dates)}")
    
    # Price the option
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.price(option, "monte_carlo_gbm", market, as_of=val_date)
    
    print(f"\nPricing Results:")
    print(f"  Present Value: {result.value}")
    
    return option, result


def example_geometric_asian_put():
    """Example: Geometric average Asian put option."""
    print("\n" + "=" * 80)
    print("GEOMETRIC ASIAN PUT OPTION")
    print("=" * 80)
    
    val_date = date(2025, 1, 1)
    
    # Create Asian option with geometric averaging
    # More frequent fixings for geometric average
    fixing_dates = [
        date(2025, 1, 31),
        date(2025, 2, 28),
        date(2025, 3, 31),
        date(2025, 4, 30),
        date(2025, 5, 31),
        date(2025, 6, 30),
    ]
    
    option = AsianOption.builder(
        instrument_id="ASIAN_PUT_001",
        ticker="AAPL",
        strike=160.0,
        expiry=date(2025, 6, 30),
        fixing_dates=fixing_dates,
        notional=Money(50000.0, USD),
        discount_curve="USD.SOFR",
        spot_id="AAPL",
        vol_surface="AAPL.VOL",
        averaging_method="geometric",
        option_type="put",
        div_yield_id="AAPL.DIV",
    )
    
    print(f"\nInstrument: {option}")
    print(f"  Ticker: {option.ticker}")
    print(f"  Strike: {option.strike}")
    print(f"  Averaging Method: {option.averaging_method}")
    print(f"  Option Type: {option.option_type}")
    print(f"  Expiry: {option.expiry}")
    print(f"  Number of Fixings: {len(option.fixing_dates)}")
    
    # Price the option
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.price(option, "monte_carlo_gbm", market, as_of=val_date)
    
    print(f"\nPricing Results:")
    print(f"  Present Value: {result.value}")
    
    return option, result


def example_averaging_method_enum():
    """Example: Using AveragingMethod enum."""
    print("\n" + "=" * 80)
    print("AVERAGING METHOD ENUM")
    print("=" * 80)
    
    # Access enum constants
    arithmetic = AveragingMethod.ARITHMETIC
    geometric = AveragingMethod.GEOMETRIC
    
    print(f"\nAveraging Methods:")
    print(f"  Arithmetic: {arithmetic}")
    print(f"  Geometric: {geometric}")
    
    # Parse from string
    from_str = AveragingMethod.from_name("arithmetic")
    print(f"\nParsed from string 'arithmetic': {from_str}")
    print(f"  Name: {from_str.name}")


def main():
    """Run all Asian option examples."""
    print("\n" + "=" * 80)
    print("ASIAN OPTION EXAMPLES")
    print("=" * 80)
    
    example_arithmetic_asian_call()
    example_geometric_asian_put()
    example_averaging_method_enum()
    
    print("\n" + "=" * 80)
    print("Examples completed successfully!")
    print("=" * 80 + "\n")


if __name__ == "__main__":
    main()

