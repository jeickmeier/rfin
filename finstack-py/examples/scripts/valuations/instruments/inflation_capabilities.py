#!/usr/bin/env python3
"""Demonstrate inflation-linked bond and zero-coupon inflation swap usage."""
from datetime import date

from finstack import Money
from finstack.core.currency import USD
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve, InflationCurve
from finstack.valuations.instruments import InflationLinkedBond, InflationSwap
from finstack.valuations.pricer import create_standard_registry


def build_market(as_of: date) -> MarketContext:
    market = MarketContext()

    disc = DiscountCurve(
        "USD-OIS",
        as_of,
        [
            (0.0, 1.0),
            (0.5, 0.9980),
            (1.0, 0.9960),
            (3.0, 0.9820),
            (5.0, 0.9600),
        ],
    )
    market.insert_discount(disc)

    inflation_curve = InflationCurve(
        "US-CPI",
        base_cpi=300.0,
        knots=[
            (0.0, 300.0),
            (1.0, 303.0),
            (2.0, 306.5),
            (5.0, 320.0),
            (10.0, 345.0),
        ],
    )
    market.insert_inflation(inflation_curve)

    return market


def main() -> None:
    as_of = date(2024, 1, 2)
    market = build_market(as_of)
    registry = create_standard_registry()

    ilb = InflationLinkedBond.create(
        "US-TIPS-2033",
        Money(1_000_000, USD),
        real_coupon=0.0125,
        issue=as_of,
        maturity=date(2034, 1, 15),
        base_index=300.0,
        discount_curve="USD-OIS",
        inflation_curve="US-CPI",
        indexation="tips",
    )
    ilb_result = registry.price_with_metrics(
        ilb,
        "discounting",
        market,
        ["real_duration", "breakeven_inflation"],
    )
    print("Inflation-linked bond PV:", round(ilb_result.value.amount, 2), ilb_result.value.currency)

    inf_swap = InflationSwap.create(
        "US-ZC-INFLATION-SWAP",
        Money(5_000_000, USD),
        fixed_rate=0.025,
        start_date=as_of,
        maturity=date(2030, 1, 2),
        discount_curve="USD-OIS",
        inflation_curve="US-CPI",
        side="pay_fixed",
    )
    swap_result = registry.price_with_metrics(
        inf_swap,
        "discounting",
        market,
        ["par_rate", "npv01"],
    )
    print("Inflation swap PV:", round(swap_result.value.amount, 2), swap_result.value.currency)


if __name__ == "__main__":
    main()
