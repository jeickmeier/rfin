#!/usr/bin/env python3
"""Variance swap example using historical prices and implied volatility surface."""
from datetime import date

from finstack import Money
from finstack.core.currency import USD
from finstack.core.dates.daycount import DayCount
from finstack.core.dates.schedule import Frequency
from finstack.core.market_data import MarketContext
from finstack.core.market_data.scalars import MarketScalar, ScalarTimeSeries, SeriesInterpolation
from finstack.core.market_data.surfaces import VolSurface
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.instruments import VarianceSwap
from finstack.valuations.pricer import create_standard_registry


def build_market(as_of: date) -> MarketContext:
    market = MarketContext()

    discount_curve = DiscountCurve(
        "USD-OIS",
        as_of,
        [
            (0.0, 1.0),
            (0.5, 0.9980),
            (1.0, 0.9960),
            (3.0, 0.9820),
        ],
    )
    market.insert_discount(discount_curve)

    observations = [
        (date(2023, 9, 29), 4200.0),
        (date(2023, 12, 29), 4305.0),
        (date(2024, 3, 28), 4380.0),
        (date(2024, 6, 28), 4450.0),
    ]
    series = ScalarTimeSeries(
        "SPX-LEVELS",
        observations,
        currency=USD,
        interpolation=SeriesInterpolation.LINEAR,
    )
    market.insert_series(series)
    market.insert_price("SPX", MarketScalar.price(Money(observations[-1][1], USD)))

    vol_surface = VolSurface(
        "SPX-VOL",
        expiries=[0.25, 0.5, 1.0],
        strikes=[3500.0, 4000.0, 4500.0],
        grid=[
            [0.22, 0.21, 0.20],
            [0.24, 0.22, 0.21],
            [0.26, 0.24, 0.22],
        ],
    )
    market.insert_surface(vol_surface)

    return market


def main() -> None:
    as_of = date(2024, 7, 1)
    market = build_market(as_of)
    registry = create_standard_registry()

    variance_swap = VarianceSwap.create(
        "SPX-VAR-SWAP",
        underlying_id="SPX",
        notional=Money(1_000_000, USD),
        strike_variance=0.04,
        start_date=date(2024, 1, 1),
        maturity=date(2024, 12, 31),
        discount_curve="USD-OIS",
        observation_frequency=Frequency.QUARTERLY,
        realized_method=None,
        side="receive",
        day_count=DayCount.ACT_365F,
    )

    result = registry.price_with_metrics(
        variance_swap,
        "discounting",
        market,
        ["variance_vega", "variance_expected"],
    )
    print("Variance swap PV:", round(result.value.amount, 2), result.value.currency)
    print("Variance swap vega:", result.measures.get("variance_vega"))


if __name__ == "__main__":
    main()
