"""
Range Accrual Example

Demonstrates pricing and analysis of range accrual instruments.
"""

from datetime import date, timedelta
from finstack import Money
from finstack.core.currency import USD
from finstack.core.market_data import MarketContext
from finstack.core.market_data.scalars import MarketScalar
from finstack.core.market_data.surfaces import VolSurface
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.core.market_data import MarketContext
from finstack.valuations.instruments import RangeAccrual
from finstack.valuations.pricer import create_standard_registry


def create_market_data(val_date: date) -> MarketContext:
    """Create market data for range accrual pricing."""
    market = MarketContext()

    # Discount curve
    disc_curve = DiscountCurve(
        "USD.SOFR",
        val_date,
        [(0.0, 1.0), (1.0, 0.95), (2.0, 0.90)],
    )
    market.insert_discount(disc_curve)

    # Volatility surface (expiries in years)
    vol_surface = VolSurface(
        "SPY.VOL",
        expiries=[1.0, 2.0],
        strikes=[400.0, 450.0, 500.0, 550.0, 600.0],
        grid=[
            [0.18, 0.16, 0.14, 0.16, 0.18],
            [0.20, 0.18, 0.16, 0.18, 0.20],
        ],
    )
    market.insert_surface(vol_surface)

    market.insert_price("SPY", MarketScalar.price(Money(500.0, USD)))
    market.insert_price("SPY.DIV", MarketScalar.unitless(0.015))

    return market


def generate_daily_observations(start_date: date, end_date: date) -> list:
    """Generate daily observation dates between start and end."""
    dates = []
    current = start_date
    while current <= end_date:
        # Skip weekends (simple approximation)
        if current.weekday() < 5:
            dates.append(current)
        current += timedelta(days=1)
    return dates


def example_narrow_range_accrual():
    """Example: Range accrual with narrow range (low volatility bet)."""
    print("\n" + "=" * 80)
    print("NARROW RANGE ACCRUAL")
    print("=" * 80)
    
    
    # Daily observations over 3 months
    # Narrow range around current spot
    start_date = date(2025, 1, 1)
    end_date = date(2025, 3, 31)
    observation_dates = generate_daily_observations(start_date, end_date)[:60]  # ~60 trading days
    
    range_accrual = RangeAccrual.builder(
        instrument_id="RANGE_001",
        ticker="SPY",
        observation_dates=observation_dates,
        lower_bound=480.0,  # 4% below spot
        upper_bound=520.0,  # 4% above spot
        coupon_rate=0.08,  # 8% annual coupon if in range
        notional=Money(1000000.0, USD),
        discount_curve="USD.SOFR",
        spot_id="SPY",
        vol_surface="SPY.VOL",
        div_yield_id="SPY.DIV",
    )
    
    print(f"\nInstrument: {range_accrual}")
    print(f"  Ticker: {range_accrual.ticker}")
    print(f"  Notional: {range_accrual.notional}")
    print(f"  Lower Bound: {range_accrual.lower_bound}")
    print(f"  Upper Bound: {range_accrual.upper_bound}")
    print(f"  Coupon Rate: {range_accrual.coupon_rate:.2%}")
    print(f"  Number of Observations: {len(observation_dates)}")
    print(f"  Range Width: {(range_accrual.upper_bound - range_accrual.lower_bound):.0f} " +
          f"({(range_accrual.upper_bound / range_accrual.lower_bound - 1) * 100:.1f}%)")
    
    # Price the range accrual
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.price(range_accrual, "monte_carlo_gbm", market, as_of=val_date)
    
    print(f"\nPricing Results:")
    print(f"  Present Value: {result.value}")
    print(f"  Currency: {result.value.currency}")
    
    print(f"\n  Explanation:")
    print(f"    - Pays 8% p.a. for each day SPY stays in [480, 520] range")
    print(f"    - Coupon accrues proportionally to days in range")
    print(f"    - Bet on low volatility / rangebound market")
    print(f"    - Popular in structured notes")
    
    return range_accrual, result


def example_wide_range_accrual():
    """Example: Range accrual with wide range (safer, lower coupon)."""
    print("\n" + "=" * 80)
    print("WIDE RANGE ACCRUAL")
    print("=" * 80)
    
    
    # Weekly observations over 1 year
    observation_dates = [
        date(2025, 1, 1) + timedelta(days=7 * i) for i in range(52)
    ]
    
    range_accrual = RangeAccrual.builder(
        instrument_id="RANGE_002",
        ticker="SPY",
        observation_dates=observation_dates,
        lower_bound=400.0,  # 20% below spot
        upper_bound=600.0,  # 20% above spot
        coupon_rate=0.05,  # 5% annual coupon (lower due to wider range)
        notional=Money(2000000.0, USD),
        discount_curve="USD.SOFR",
        spot_id="SPY",
        vol_surface="SPY.VOL",
        div_yield_id="SPY.DIV",
    )
    
    print(f"\nInstrument: {range_accrual}")
    print(f"  Ticker: {range_accrual.ticker}")
    print(f"  Notional: {range_accrual.notional}")
    print(f"  Lower Bound: {range_accrual.lower_bound}")
    print(f"  Upper Bound: {range_accrual.upper_bound}")
    print(f"  Coupon Rate: {range_accrual.coupon_rate:.2%}")
    print(f"  Number of Observations: {len(observation_dates)}")
    print(f"  Range Width: {(range_accrual.upper_bound - range_accrual.lower_bound):.0f}")
    
    # Price the range accrual
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.price(range_accrual, "monte_carlo_gbm", market, as_of=val_date)
    
    print(f"\nPricing Results:")
    print(f"  Present Value: {result.value}")
    print(f"  Currency: {result.value.currency}")
    
    print(f"\n  Explanation:")
    print(f"    - Wider range [400, 600] provides more safety")
    print(f"    - Lower coupon rate compensates for higher probability")
    print(f"    - Weekly observations reduce noise vs daily")
    
    return range_accrual, result


def example_asymmetric_range_accrual():
    """Example: Range accrual with asymmetric range."""
    print("\n" + "=" * 80)
    print("ASYMMETRIC RANGE ACCRUAL")
    print("=" * 80)
    
    
    # Monthly observations over 6 months
    observation_dates = [
        date(2025, 2, 1),
        date(2025, 3, 1),
        date(2025, 4, 1),
        date(2025, 5, 1),
        date(2025, 6, 1),
        date(2025, 7, 1),
    ]
    
    # Asymmetric range: more upside room than downside
    range_accrual = RangeAccrual.builder(
        instrument_id="RANGE_003",
        ticker="SPY",
        observation_dates=observation_dates,
        lower_bound=475.0,  # 5% downside buffer
        upper_bound=550.0,  # 10% upside room
        coupon_rate=0.06,  # 6% annual coupon
        notional=Money(500000.0, USD),
        discount_curve="USD.SOFR",
        spot_id="SPY",
        vol_surface="SPY.VOL",
        div_yield_id="SPY.DIV",
    )
    
    print(f"\nInstrument: {range_accrual}")
    print(f"  Ticker: {range_accrual.ticker}")
    print(f"  Notional: {range_accrual.notional}")
    print(f"  Lower Bound: {range_accrual.lower_bound}")
    print(f"  Upper Bound: {range_accrual.upper_bound}")
    print(f"  Coupon Rate: {range_accrual.coupon_rate:.2%}")
    print(f"  Current Spot: 500.0")
    print(f"  Downside Buffer: {(500.0 - range_accrual.lower_bound) / 500.0:.1%}")
    print(f"  Upside Room: {(range_accrual.upper_bound - 500.0) / 500.0:.1%}")
    
    # Price the range accrual
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.price(range_accrual, "monte_carlo_gbm", market, as_of=val_date)
    
    print(f"\nPricing Results:")
    print(f"  Present Value: {result.value}")
    print(f"  Currency: {result.value.currency}")
    
    print(f"\n  Explanation:")
    print(f"    - Asymmetric range allows for modest uptrend")
    print(f"    - Tighter downside protection")
    print(f"    - Monthly observations smooth out noise")
    print(f"    - View: market will stay stable or drift higher")
    
    return range_accrual, result


def example_high_coupon_tight_range():
    """Example: High coupon, very tight range (aggressive bet)."""
    print("\n" + "=" * 80)
    print("HIGH COUPON TIGHT RANGE")
    print("=" * 80)
    
    
    # Daily observations over 1 month
    observation_dates = generate_daily_observations(
        date(2025, 1, 1),
        date(2025, 1, 31)
    )[:20]  # ~20 trading days
    
    range_accrual = RangeAccrual.builder(
        instrument_id="RANGE_004",
        ticker="SPY",
        observation_dates=observation_dates,
        lower_bound=495.0,  # Very tight: 1% below
        upper_bound=505.0,  # Very tight: 1% above
        coupon_rate=0.15,  # 15% annual coupon (very high!)
        notional=Money(100000.0, USD),
        discount_curve="USD.SOFR",
        spot_id="SPY",
        vol_surface="SPY.VOL",
        div_yield_id="SPY.DIV",
    )
    
    print(f"\nInstrument: {range_accrual}")
    print(f"  Ticker: {range_accrual.ticker}")
    print(f"  Notional: {range_accrual.notional}")
    print(f"  Lower Bound: {range_accrual.lower_bound}")
    print(f"  Upper Bound: {range_accrual.upper_bound}")
    print(f"  Coupon Rate: {range_accrual.coupon_rate:.2%}")
    print(f"  Range Width: ±{((range_accrual.upper_bound - 500.0) / 500.0) * 100:.1f}%")
    
    # Price the range accrual
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.price(range_accrual, "monte_carlo_gbm", market, as_of=val_date)
    
    print(f"\nPricing Results:")
    print(f"  Present Value: {result.value}")
    print(f"  Currency: {result.value.currency}")
    
    print(f"\n  Explanation:")
    print(f"    - Very tight range [495, 505] = ±1% around spot")
    print(f"    - High 15% coupon compensates for low probability")
    print(f"    - Extreme low-volatility bet")
    print(f"    - High risk/high reward structure")
    
    return range_accrual, result


def main():
    """Run all range accrual examples."""
    print("\n" + "=" * 80)
    print("RANGE ACCRUAL EXAMPLES")
    print("=" * 80)
    print("\nRange accruals pay coupons proportional to time spent in range.")
    print("Wider ranges → lower coupons, higher safety")
    print("Tighter ranges → higher coupons, higher risk")
    
    example_narrow_range_accrual()
    example_wide_range_accrual()
    example_asymmetric_range_accrual()
    example_high_coupon_tight_range()
    
    print("\n" + "=" * 80)
    print("Examples completed successfully!")
    print("=" * 80 + "\n")


if __name__ == "__main__":
    main()

