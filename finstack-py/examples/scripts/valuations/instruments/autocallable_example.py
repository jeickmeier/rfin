"""
Autocallable Example

Demonstrates pricing and analysis of autocallable structured products.
"""

from datetime import date
from finstack import Money
from finstack.core.currency import USD
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.scalars import MarketScalar
from finstack.core.market_data.surfaces import VolSurface
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.core.market_data.context import MarketContext
from finstack.valuations.instruments import Autocallable
from finstack.valuations.pricer import create_standard_registry


def create_market_data(val_date: date) -> MarketContext:
    """Create market data for autocallable pricing."""
    market = MarketContext()

    # Discount curve
    disc_curve = DiscountCurve(
        "USD.SOFR",
        val_date,
        [(0.0, 1.0), (1.0, 0.95), (2.0, 0.90), (3.0, 0.85)],
    )
    market.insert_discount(disc_curve)

    # Volatility surface
    vol_surface = VolSurface(
        "SPX.VOL",
        expiries=[1.0, 2.0, 3.0],
        strikes=[4000.0, 4500.0, 5000.0, 5500.0, 6000.0],
        grid=[
            [0.20, 0.18, 0.16, 0.18, 0.20],
            [0.22, 0.20, 0.18, 0.20, 0.22],
            [0.24, 0.22, 0.20, 0.22, 0.24],
        ],
    )
    market.insert_surface(vol_surface)

    market.insert_price("SPX", MarketScalar.price(Money(5000.0, USD)))
    market.insert_price("SPX.DIV", MarketScalar.unitless(0.015))

    return market


def example_participation_autocallable():
    """Example: Autocallable with participation in upside."""
    print("\n" + "=" * 80)
    print("AUTOCALLABLE WITH PARTICIPATION")
    print("=" * 80)
    
    
    # Quarterly observation dates over 3 years
    observation_dates = [
        date(2025, 4, 1),
        date(2025, 7, 1),
        date(2025, 10, 1),
        date(2026, 1, 1),
        date(2026, 4, 1),
        date(2026, 7, 1),
        date(2026, 10, 1),
        date(2027, 1, 1),
        date(2027, 4, 1),
        date(2027, 7, 1),
        date(2027, 10, 1),
        date(2028, 1, 1),
    ]
    
    # Autocall barriers start at 100% and step down
    autocall_barriers = [
        1.00, 1.00, 0.95, 0.95,  # Year 1
        0.90, 0.90, 0.85, 0.85,  # Year 2
        0.80, 0.80, 0.75, 0.75,  # Year 3
    ]
    
    # Coupons accumulate over time
    coupons = [
        0.02, 0.04, 0.06, 0.08,  # Year 1: 2%, 4%, 6%, 8%
        0.10, 0.12, 0.14, 0.16,  # Year 2: 10%, 12%, 14%, 16%
        0.18, 0.20, 0.22, 0.24,  # Year 3: 18%, 20%, 22%, 24%
    ]
    
    autocallable = Autocallable.builder(
        instrument_id="AUTO_001",
        ticker="SPX",
        observation_dates=observation_dates,
        autocall_barriers=autocall_barriers,
        coupons=coupons,
        final_barrier=0.70,  # 70% barrier at maturity
        final_payoff_type={"type": "participation", "rate": 1.0},  # 100% participation
        participation_rate=1.0,
        cap_level=1.5,  # 150% cap
        notional=Money(100000.0, USD),
        discount_curve="USD.SOFR",
        spot_id="SPX",
        vol_surface="SPX.VOL",
        div_yield_id="SPX.DIV",
    )
    
    print(f"\nInstrument: {autocallable}")
    print(f"  Ticker: {autocallable.ticker}")
    print(f"  Notional: {autocallable.notional}")
    print(f"  Final Barrier: {autocallable.final_barrier:.1%}")
    print(f"  Participation Rate: {autocallable.participation_rate:.1%}")
    print(f"  Cap Level: {autocallable.cap_level:.1%}")
    print(f"  Number of Observations: {len(observation_dates)}")
    
    # Price the autocallable
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.price(autocallable, "monte_carlo_gbm", market, as_of=val_date)
    
    print(f"\nPricing Results:")
    print(f"  Present Value: {result.value}")
    print(f"  Currency: {result.value.currency}")
    
    return autocallable, result


def example_capital_protection_autocallable():
    """Example: Autocallable with capital protection."""
    print("\n" + "=" * 80)
    print("AUTOCALLABLE WITH CAPITAL PROTECTION")
    print("=" * 80)
    
    
    # Semi-annual observations over 2 years
    observation_dates = [
        date(2025, 7, 1),
        date(2026, 1, 1),
        date(2026, 7, 1),
        date(2027, 1, 1),
    ]
    
    # Higher autocall barriers for capital protection structure
    autocall_barriers = [1.05, 1.00, 0.95, 0.90]
    
    # Fixed coupons
    coupons = [0.05, 0.10, 0.15, 0.20]
    
    autocallable = Autocallable.builder(
        instrument_id="AUTO_002",
        ticker="SPX",
        observation_dates=observation_dates,
        autocall_barriers=autocall_barriers,
        coupons=coupons,
        final_barrier=0.80,  # 80% barrier
        final_payoff_type={"type": "capital_protection", "floor": 0.90},  # 90% floor
        participation_rate=0.80,  # 80% participation
        cap_level=1.3,  # 130% cap
        notional=Money(250000.0, USD),
        discount_curve="USD.SOFR",
        spot_id="SPX",
        vol_surface="SPX.VOL",
        div_yield_id="SPX.DIV",
    )
    
    print(f"\nInstrument: {autocallable}")
    print(f"  Ticker: {autocallable.ticker}")
    print(f"  Notional: {autocallable.notional}")
    print(f"  Final Barrier: {autocallable.final_barrier:.1%}")
    print(f"  Participation Rate: {autocallable.participation_rate:.1%}")
    print(f"  Cap Level: {autocallable.cap_level:.1%}")
    
    # Price the autocallable
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.price(autocallable, "monte_carlo_gbm", market, as_of=val_date)
    
    print(f"\nPricing Results:")
    print(f"  Present Value: {result.value}")
    print(f"  Currency: {result.value.currency}")
    
    return autocallable, result


def example_knock_in_autocallable():
    """Example: Autocallable with knock-in put feature."""
    print("\n" + "=" * 80)
    print("AUTOCALLABLE WITH KNOCK-IN PUT")
    print("=" * 80)
    
    
    # Annual observations
    observation_dates = [
        date(2026, 1, 1),
        date(2027, 1, 1),
        date(2028, 1, 1),
    ]
    
    autocall_barriers = [1.10, 1.00, 0.90]
    coupons = [0.08, 0.16, 0.24]
    
    autocallable = Autocallable.builder(
        instrument_id="AUTO_003",
        ticker="SPX",
        observation_dates=observation_dates,
        autocall_barriers=autocall_barriers,
        coupons=coupons,
        final_barrier=0.70,  # 70% knock-in barrier
        final_payoff_type={"type": "knock_in_put", "strike": 5000.0},  # Strike at initial spot
        participation_rate=0.0,  # No upside participation
        cap_level=1.0,
        notional=Money(500000.0, USD),
        discount_curve="USD.SOFR",
        spot_id="SPX",
        vol_surface="SPX.VOL",
        div_yield_id="SPX.DIV",
    )
    
    print(f"\nInstrument: {autocallable}")
    print(f"  Ticker: {autocallable.ticker}")
    print(f"  Notional: {autocallable.notional}")
    print(f"  Final Barrier: {autocallable.final_barrier:.1%}")
    print(f"  Number of Observations: {len(observation_dates)}")
    
    # Price the autocallable
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.price(autocallable, "monte_carlo_gbm", market, as_of=val_date)
    
    print(f"\nPricing Results:")
    print(f"  Present Value: {result.value}")
    print(f"  Currency: {result.value.currency}")
    
    return autocallable, result


def main():
    """Run all autocallable examples."""
    print("\n" + "=" * 80)
    print("AUTOCALLABLE EXAMPLES")
    print("=" * 80)
    
    example_participation_autocallable()
    example_capital_protection_autocallable()
    example_knock_in_autocallable()
    
    print("\n" + "=" * 80)
    print("Examples completed successfully!")
    print("=" * 80 + "\n")


if __name__ == "__main__":
    main()

