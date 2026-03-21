#!/usr/bin/env python3
"""Showcase equity spot and listed option bindings."""

from datetime import date

from finstack.core.currency import USD
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.scalars import MarketScalar
from finstack.core.market_data.surfaces import VolSurface
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.instruments import Equity, EquityOption
from finstack.valuations.pricer import standard_registry

from finstack import Money


def build_equity_market(as_of: date) -> MarketContext:
    """Create funding curve, implied vol surface, and reference prices."""
    market = MarketContext()

    disc = DiscountCurve(
        "USD-OIS",
        as_of,
        [
            (0.0, 1.0),
            (0.5, 0.9970),
            (1.0, 0.9940),
            (3.0, 0.9725),
            (5.0, 0.9480),
        ],
    )
    market.insert(disc)

    spot_scalar = MarketScalar.price(Money(150.0, USD))
    market.insert_price("EQUITY-SPOT", spot_scalar)

    div_scalar = MarketScalar.unitless(0.015)
    market.insert_price("EQUITY-DIVYIELD", div_scalar)

    vol_surface = VolSurface(
        "EQUITY-VOL",
        expiries=[0.25, 0.5, 1.0, 2.0],
        strikes=[120.0, 140.0, 160.0, 180.0],
        grid=[
            [0.28, 0.26, 0.25, 0.24],
            [0.27, 0.25, 0.24, 0.23],
            [0.26, 0.24, 0.23, 0.22],
            [0.25, 0.23, 0.22, 0.21],
        ],
    )
    market.insert_surface(vol_surface)

    return market


def main() -> None:
    as_of = date(2024, 1, 2)
    market = build_equity_market(as_of)
    registry = standard_registry()

    equity = (
        Equity.builder("ACME-SPOT")
        .ticker("ACME")
        .currency(USD)
        .shares(1_000.0)
        .price_id("EQUITY-SPOT")
        .div_yield_id("EQUITY-DIVYIELD")
        .build()
    )
    registry.get_price(equity, "discounting", market, as_of=as_of)

    call = (
        EquityOption.builder("ACME-CALL-150")
        .ticker("ACME")
        .strike(150.0)
        .expiry(date(2024, 12, 31))
        .notional(Money(100.0, USD))
        .option_type("call")
        .exercise_style("european")
        .disc_id("USD-OIS")
        .spot_id("EQUITY-SPOT")
        .vol_surface("EQUITY-VOL")
        .div_yield_id("EQUITY-DIVYIELD")
        .build()
    )
    registry.price_with_metrics(
        call,
        "discounting",
        market,
        ["delta", "gamma", "vega"],
        as_of=as_of,
    )

    put = (
        EquityOption.builder("ACME-PUT-140")
        .ticker("ACME")
        .strike(140.0)
        .expiry(date(2024, 9, 30))
        .notional(Money(100.0, USD))
        .option_type("put")
        .exercise_style("european")
        .disc_id("USD-OIS")
        .spot_id("EQUITY-SPOT")
        .vol_surface("EQUITY-VOL")
        .div_yield_id("EQUITY-DIVYIELD")
        .build()
    )
    registry.get_price(put, "discounting", market, as_of=as_of)


if __name__ == "__main__":
    main()
