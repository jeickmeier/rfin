#!/usr/bin/env python3
"""Showcase equity spot and listed option bindings."""
from datetime import date

from finstack import Money
from finstack.core.currency import USD
from finstack.core.market_data import MarketContext
from finstack.core.market_data.scalars import MarketScalar
from finstack.core.market_data.surfaces import VolSurface
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.instruments import Equity, EquityOption
from finstack.valuations.pricer import create_standard_registry


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
    market.insert_discount(disc)

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
    registry = create_standard_registry()

    equity = Equity.create(
        "ACME-SPOT",
        ticker="ACME",
        currency=USD,
        shares=1_000.0,
    )
    equity_value = registry.price(equity, "discounting", market)
    print("Equity PV:", round(equity_value.value.amount, 2), equity_value.value.currency)

    call = EquityOption.european_call(
        "ACME-CALL-150",
        ticker="ACME",
        strike=150.0,
        expiry=date(2024, 12, 31),
        notional=Money(150.0, USD),
        contract_size=100.0,
    )
    call_result = registry.price_with_metrics(
        call,
        "discounting",
        market,
        ["delta", "gamma", "vega"],
    )
    print("Call PV:", round(call_result.value.amount, 2), call_result.value.currency)
    print("Call delta:", call_result.measures.get("delta"))

    put = EquityOption.european_put(
        "ACME-PUT-140",
        ticker="ACME",
        strike=140.0,
        expiry=date(2024, 9, 30),
        notional=Money(140.0, USD),
        contract_size=100.0,
    )
    put_value = registry.price(put, "discounting", market)
    print("Put PV:", round(put_value.value.amount, 2), put_value.value.currency)


if __name__ == "__main__":
    main()
