#!/usr/bin/env python3
"""Demonstrate FX instrument bindings: spot, swap, and options."""

from datetime import date, timedelta

from finstack import Money
from finstack.core.currency import EUR, JPY, USD
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.fx import FxMatrix
from finstack.core.market_data.surfaces import VolSurface
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.instruments import FxOption, FxSpot, FxSwap
from finstack.valuations.pricer import create_standard_registry


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

    market.insert_discount(usd_disc)
    market.insert_discount(eur_disc)
    market.insert_discount(jpy_disc)

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
    registry = create_standard_registry()

    # FX spot trade settling T+2 with explicit notional
    spot = FxSpot.create(
        "EURUSD-SPOT",
        EUR,
        USD,
        settlement=as_of + timedelta(days=2),
        spot_rate=1.0860,
        notional=Money(1_000_000, EUR),
    )
    spot_value = registry.price(spot, "discounting", market, as_of=as_of)
    print("FX spot PV:", round(spot_value.value.amount, 2), spot_value.value.currency)

    # FX swap exchanging notionals
    near = as_of + timedelta(days=2)
    far = near + timedelta(days=180)
    fx_swap = FxSwap.create(
        "EURUSD-SWAP",
        EUR,
        USD,
        Money(5_000_000, EUR),
        near,
        far,
        "USD-OIS",
        "EUR-OIS",
        near_rate=1.0865,
        far_rate=1.0920,
    )
    swap_result = registry.price_with_metrics(
        fx_swap,
        "discounting",
        market,
        ["carry_pv"],
        as_of=as_of,
    )
    print("FX swap PV:", round(swap_result.value.amount, 2), swap_result.value.currency)

    # European FX option (call on EURUSD)
    fx_call = FxOption.european_call(
        "EURUSD-CALL-1Y",
        EUR,
        USD,
        strike=1.10,
        expiry=date(2025, 1, 2),
        notional=Money(2_000_000, EUR),
        vol_surface="FX-VOL",
    )
    option_result = registry.price_with_metrics(
        fx_call,
        "discounting",
        market,
        ["delta", "gamma"],
        as_of=as_of,
    )
    print("FX option PV:", round(option_result.value.amount, 2), option_result.value.currency)
    print("FX option delta:", option_result.measures.get("delta"))

    # Put option via helper for completeness
    fx_put = FxOption.european_put(
        "EURUSD-PUT-6M",
        EUR,
        USD,
        strike=1.06,
        expiry=date(2024, 7, 2),
        notional=Money(1_500_000, EUR),
        vol_surface="FX-VOL",
    )
    put_value = registry.price(fx_put, "discounting", market, as_of=as_of)
    print("FX put PV:", round(put_value.value.amount, 2), put_value.value.currency)


if __name__ == "__main__":
    main()
