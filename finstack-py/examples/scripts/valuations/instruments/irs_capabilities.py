#!/usr/bin/env python3
"""Demonstrate creating and valuing plain-vanilla interest rate swaps."""

from datetime import date, timedelta

from finstack.core.currency import USD
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve, ForwardCurve
from finstack.valuations.instruments import InterestRateSwap
from finstack.valuations.pricer import create_standard_registry

from finstack import Money


def build_market(as_of: date) -> MarketContext:
    """Create a minimal market with discount and forward curves."""
    discount_curve = DiscountCurve(
        "USD-OIS",
        as_of,
        [
            (0.0, 1.0),
            (0.5, 0.9950),
            (1.0, 0.9900),
            (2.0, 0.9750),
            (5.0, 0.9400),
        ],
    )
    forward_curve = ForwardCurve(
        "USD-SOFR-3M",
        0.25,
        [
            (0.0, 0.0300),
            (1.0, 0.0320),
            (2.0, 0.0340),
            (5.0, 0.0360),
        ],
        base_date=as_of,
    )
    market = MarketContext()
    market.insert_discount(discount_curve)
    market.insert_forward(forward_curve)
    return market


def build_swap(as_of: date, notional: Money) -> InterestRateSwap:
    """Create a receive-fixed/pay-float USD SOFR swap."""
    start = as_of + timedelta(days=2)  # standard spot lag
    maturity = date(as_of.year + 5, as_of.month, as_of.day)
    return (
        InterestRateSwap.builder("USD-SOFR-SWAP")
        .money(notional)
        .side("receive_fixed")
        .fixed_rate(0.0325)
        .start(start)
        .maturity(maturity)
        .disc_id("USD-OIS")
        .fwd_id("USD-SOFR-3M")
        .build()
    )


def main() -> None:
    as_of = date(2024, 1, 2)
    notional = Money(10_000_000, USD)

    market = build_market(as_of)
    swap = build_swap(as_of, notional)

    registry = create_standard_registry()

    # Price with metrics - now supports working metrics
    result = registry.price_with_metrics(
        swap,
        "discounting",
        market,
        ["annuity", "dv01", "par_rate"],  # Start with metrics that work
        as_of=as_of,
    )

    measures = result.measures
    measures.get("annuity", 0.0)
    measures.get("dv01", 0.0)
    measures.get("par_rate", 0.0)


if __name__ == "__main__":
    main()
