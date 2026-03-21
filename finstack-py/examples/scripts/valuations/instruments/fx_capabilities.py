#!/usr/bin/env python3
"""Demonstrate FX instrument bindings: spot, swap, and options."""

from datetime import date, timedelta

from finstack.core.currency import EUR, JPY, USD
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.fx import FxMatrix
from finstack.core.market_data.surfaces import VolSurface
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.instruments import FxOption, FxSpot, FxSwap
from finstack.valuations.pricer import standard_registry

from finstack import Money


def build_fx_market(as_of: date) -> MarketContext:
    """Construct discount curves, FX quotes, and a simple volatility surface."""
    market = MarketContext()

    usd_disc = DiscountCurve(
        "USD-OIS",
        as_of,
        [
            (0.0, 1.0),
            (0.5, 0.9975),
            (1.0, 0.9945),
            (3.0, 0.9720),
            (5.0, 0.9450),
        ],
    )
    eur_disc = DiscountCurve(
        "EUR-OIS",
        as_of,
        [
            (0.0, 1.0),
            (0.5, 0.9980),
            (1.0, 0.9960),
            (3.0, 0.9800),
            (5.0, 0.9550),
        ],
    )

    jpy_disc = DiscountCurve(
        "JPY-OIS",
        as_of,
        [
            (0.0, 1.0),
            (0.5, 0.9990),
            (1.0, 0.9980),
            (3.0, 0.9920),
            (5.0, 0.9850),
        ],
    )

    market.insert(usd_disc)
    market.insert(eur_disc)
    market.insert(jpy_disc)

    fx_matrix = FxMatrix()
    fx_matrix.set_quote(EUR, USD, 1.0850)
    fx_matrix.set_quote(USD, JPY, 148.50)
    fx_matrix.set_quote(EUR, JPY, 161.20)
    market.insert_fx(fx_matrix)

    fx_vol_surface = VolSurface(
        "FX-VOL",
        expiries=[0.25, 0.5, 1.0, 2.0],
        strikes=[1.05, 1.10, 1.15],
        grid=[
            [0.14, 0.13, 0.12],
            [0.13, 0.12, 0.11],
            [0.12, 0.11, 0.10],
            [0.11, 0.10, 0.095],
        ],
    )
    market.insert_surface(fx_vol_surface)

    return market


def main() -> None:
    as_of = date(2024, 1, 2)
    market = build_fx_market(as_of)
    registry = standard_registry()

    # FX spot trade settling T+2 with explicit notional
    spot = (
        FxSpot.builder("EURUSD-SPOT")
        .base_currency(EUR)
        .quote_currency(USD)
        .settlement(as_of + timedelta(days=2))
        .spot_rate(1.0860)
        .notional(Money(1_000_000, EUR))
        .build()
    )
    registry.get_price(spot, "discounting", market, as_of=as_of)

    # FX swap exchanging notionals
    near = as_of + timedelta(days=2)
    far = near + timedelta(days=180)
    fx_swap = (
        FxSwap.builder("EURUSD-SWAP")
        .base_currency(EUR)
        .quote_currency(USD)
        .notional(Money(5_000_000, EUR))
        .near_date(near)
        .far_date(far)
        .domestic_discount_curve("USD-OIS")
        .foreign_discount_curve("EUR-OIS")
        .near_rate(1.0865)
        .far_rate(1.0920)
        .build()
    )
    registry.price_with_metrics(
        fx_swap,
        "discounting",
        market,
        as_of,
        metrics=["carry_pv"],
    )

    # European FX option (call on EURUSD)
    fx_call = (
        FxOption.builder("EURUSD-CALL-1Y")
        .base_currency(EUR)
        .quote_currency(USD)
        .strike(1.10)
        .expiry(date(2025, 1, 2))
        .notional(Money(2_000_000, EUR))
        .domestic_discount_curve("USD-OIS")
        .foreign_discount_curve("EUR-OIS")
        .vol_surface("FX-VOL")
        .option_type("call")
        .build()
    )
    registry.price_with_metrics(
        fx_call,
        "discounting",
        market,
        as_of,
        metrics=["delta", "gamma"],
    )

    # Put option via helper for completeness
    fx_put = (
        FxOption.builder("EURUSD-PUT-6M")
        .base_currency(EUR)
        .quote_currency(USD)
        .strike(1.06)
        .expiry(date(2024, 7, 2))
        .notional(Money(1_500_000, EUR))
        .domestic_discount_curve("USD-OIS")
        .foreign_discount_curve("EUR-OIS")
        .vol_surface("FX-VOL")
        .option_type("put")
        .build()
    )
    registry.get_price(fx_put, "discounting", market, as_of=as_of)


if __name__ == "__main__":
    main()
