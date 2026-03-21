"""Range Accrual Example.

Demonstrates pricing and analysis of range accrual instruments.
"""

from datetime import date, timedelta

from finstack.core.currency import USD
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.scalars import MarketScalar
from finstack.core.market_data.surfaces import VolSurface
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.instruments import RangeAccrual
from finstack.valuations.pricer import standard_registry

from finstack import Money


def create_market_data(val_date: date) -> MarketContext:
    """Create market data for range accrual pricing."""
    market = MarketContext()

    # Discount curve
    disc_curve = DiscountCurve(
        "USD.SOFR",
        val_date,
        [(0.0, 1.0), (1.0, 0.95), (2.0, 0.90)],
    )
    market.insert(disc_curve)

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
    # Daily observations over 3 months
    # Narrow range around current spot
    start_date = date(2025, 1, 1)
    end_date = date(2025, 3, 31)
    observation_dates = generate_daily_observations(start_date, end_date)[:60]  # ~60 trading days

    range_accrual = (
        RangeAccrual.builder("RANGE_001")
        .ticker("SPY")
        .observation_dates(observation_dates)
        .lower_bound(480.0)
        .upper_bound(520.0)
        .coupon_rate(0.08)
        .notional(Money(1000000.0, USD))
        .discount_curve("USD.SOFR")
        .spot_id("SPY")
        .vol_surface("SPY.VOL")
        .div_yield_id("SPY.DIV")
        .build()
    )

    # Price the range accrual
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = standard_registry()
    result = registry.get_price(range_accrual, "monte_carlo_gbm", market, as_of=val_date)

    return range_accrual, result


def example_wide_range_accrual():
    """Example: Range accrual with wide range (safer, lower coupon)."""
    # Weekly observations over 1 year
    observation_dates = [date(2025, 1, 1) + timedelta(days=7 * i) for i in range(52)]

    range_accrual = (
        RangeAccrual.builder("RANGE_002")
        .ticker("SPY")
        .observation_dates(observation_dates)
        .lower_bound(400.0)
        .upper_bound(600.0)
        .coupon_rate(0.05)
        .notional(Money(2000000.0, USD))
        .discount_curve("USD.SOFR")
        .spot_id("SPY")
        .vol_surface("SPY.VOL")
        .div_yield_id("SPY.DIV")
        .build()
    )

    # Price the range accrual
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = standard_registry()
    result = registry.get_price(range_accrual, "monte_carlo_gbm", market, as_of=val_date)

    return range_accrual, result


def example_asymmetric_range_accrual():
    """Example: Range accrual with asymmetric range."""
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
    range_accrual = (
        RangeAccrual.builder("RANGE_003")
        .ticker("SPY")
        .observation_dates(observation_dates)
        .lower_bound(475.0)
        .upper_bound(550.0)
        .coupon_rate(0.06)
        .notional(Money(500000.0, USD))
        .discount_curve("USD.SOFR")
        .spot_id("SPY")
        .vol_surface("SPY.VOL")
        .div_yield_id("SPY.DIV")
        .build()
    )

    # Price the range accrual
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = standard_registry()
    result = registry.get_price(range_accrual, "monte_carlo_gbm", market, as_of=val_date)

    return range_accrual, result


def example_high_coupon_tight_range():
    """Example: High coupon, very tight range (aggressive bet)."""
    # Daily observations over 1 month
    observation_dates = generate_daily_observations(date(2025, 1, 1), date(2025, 1, 31))[:20]  # ~20 trading days

    range_accrual = (
        RangeAccrual.builder("RANGE_004")
        .ticker("SPY")
        .observation_dates(observation_dates)
        .lower_bound(495.0)
        .upper_bound(505.0)
        .coupon_rate(0.15)
        .notional(Money(100000.0, USD))
        .discount_curve("USD.SOFR")
        .spot_id("SPY")
        .vol_surface("SPY.VOL")
        .div_yield_id("SPY.DIV")
        .build()
    )

    # Price the range accrual
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = standard_registry()
    result = registry.get_price(range_accrual, "monte_carlo_gbm", market, as_of=val_date)

    return range_accrual, result


def main() -> None:
    """Run all range accrual examples."""
    example_narrow_range_accrual()
    example_wide_range_accrual()
    example_asymmetric_range_accrual()
    example_high_coupon_tight_range()


if __name__ == "__main__":
    main()
