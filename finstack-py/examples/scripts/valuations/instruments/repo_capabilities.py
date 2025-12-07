#!/usr/bin/env python3
"""Example for the repo instrument, including collateral specification."""

from datetime import date, timedelta

from finstack import Money
from finstack.core.currency import USD
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.scalars import MarketScalar
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.instruments import Repo, RepoCollateral
from finstack.valuations.pricer import create_standard_registry


def build_market(as_of: date) -> MarketContext:
    market = MarketContext()

    disc = DiscountCurve(
        "USD-OIS",
        as_of,
        [
            (0.0, 1.0),
            (0.25, 0.9985),
            (0.5, 0.9970),
            (1.0, 0.9940),
        ],
    )
    market.insert_discount(disc)

    market.insert_price(
        "UST-COLLATERAL",
        MarketScalar.price(Money(1.0, USD)),
    )

    return market


def build_repo(as_of: date) -> Repo:
    collateral = RepoCollateral(
        "UST-10Y",
        quantity=10_500_000.0,
        market_value_id="UST-COLLATERAL",
    )
    start = as_of
    maturity = as_of + timedelta(days=14)
    return Repo.create(
        "UST-TERM-REPO",
        Money(10_000_000.0, USD),
        collateral,
        repo_rate=0.032,
        start_date=start,
        maturity=maturity,
        discount_curve="USD-OIS",
        repo_type="term",
        haircut=0.02,
        calendar="usny",
    )


def main() -> None:
    as_of = date(2024, 1, 2)
    market = build_market(as_of)
    repo = build_repo(as_of)
    registry = create_standard_registry()

    result = registry.price_with_metrics(repo, "discounting", market, ["accrued_interest"])
    print("Repo PV:", round(result.value.amount, 2), result.value.currency)
    print("Repo accrued interest:", result.measures.get("accrued_interest"))


if __name__ == "__main__":
    main()
