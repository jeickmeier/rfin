#!/usr/bin/env python3
"""Demonstrate creating and valuing core interest-rate instruments."""

from datetime import date, timedelta

from finstack.core.currency import USD
from finstack.core.dates import BusinessDayConvention
from finstack.core.dates.daycount import DayCount
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.surfaces import VolSurface
from finstack.core.market_data.term_structures import DiscountCurve, ForwardCurve
from finstack.valuations.instruments import (
    BasisSwap,
    BasisSwapLeg,
    Deposit,
    ForwardRateAgreement,
    InterestRateFuture,
    InterestRateOption,
    Swaption,
)
from finstack.valuations.pricer import create_standard_registry

from finstack import Money


def build_rate_market(as_of: date) -> MarketContext:
    """Create a minimal USD OIS/SOFR market with vol surfaces."""
    market = MarketContext()

    # Discount and forward curves
    disc = DiscountCurve(
        "USD-OIS",
        as_of,
        [
            (0.0, 1.0),
            (0.5, 0.9975),
            (1.0, 0.9940),
            (3.0, 0.9700),
            (5.0, 0.9350),
            (7.0, 0.8950),
        ],
    )
    sofr_3m = ForwardCurve(
        "USD-SOFR-3M",
        0.25,
        [
            (0.0, 0.0350),
            (1.0, 0.0360),
            (3.0, 0.0385),
            (5.0, 0.0400),
        ],
        base_date=as_of,
    )
    sofr_6m = ForwardCurve(
        "USD-SOFR-6M",
        0.5,
        [
            (0.0, 0.0355),
            (1.0, 0.0370),
            (3.0, 0.0390),
            (5.0, 0.0410),
        ],
        base_date=as_of,
    )

    market.insert_discount(disc)
    market.insert_forward(sofr_3m)
    market.insert_forward(sofr_6m)

    # Simple volatility surfaces used by caps/floors and swaptions
    cap_surface = VolSurface(
        "USD-CAP-VOL",
        expiries=[0.5, 1.0, 2.0, 5.0],
        strikes=[0.01, 0.02, 0.03, 0.04],
        grid=[
            [0.38, 0.36, 0.34, 0.32],
            [0.35, 0.33, 0.31, 0.30],
            [0.32, 0.31, 0.29, 0.28],
            [0.28, 0.27, 0.26, 0.25],
        ],
    )
    swaption_surface = VolSurface(
        "SWAPTION-VOL",
        expiries=[1.0, 2.0, 5.0, 7.0],
        strikes=[0.02, 0.03, 0.04],
        grid=[
            [0.30, 0.29, 0.28],
            [0.28, 0.27, 0.26],
            [0.26, 0.25, 0.24],
            [0.25, 0.24, 0.23],
        ],
    )
    market.insert_surface(cap_surface)
    market.insert_surface(swaption_surface)

    return market


def main() -> None:
    as_of = date(2024, 1, 2)
    market = build_rate_market(as_of)
    registry = create_standard_registry()

    # Deposit example: 3M USD term deposit priced off OIS curve
    deposit = (
        Deposit.builder("USD-DEP-3M")
        .money(Money(5_000_000, USD))
        .start(as_of)
        .maturity(as_of + timedelta(days=92))
        .day_count(DayCount.ACT_360)
        .disc_id("USD-OIS")
        .quote_rate(0.0450)
        .build()
    )
    registry.price(deposit, "discounting", market, as_of=as_of)

    # FRA: receive fixed vs pay floating (SOFR 3M)
    fra = (
        ForwardRateAgreement.builder("USD-FRA-3x6")
        .money(Money(10_000_000, USD))
        .fixed_rate(0.0360)
        .fixing_date(as_of + timedelta(days=30))
        .start_date(as_of + timedelta(days=92))
        .end_date(as_of + timedelta(days=182))
        .disc_id("USD-OIS")
        .fwd_id("USD-SOFR-3M")
        .pay_fixed(False)
        .build()
    )
    registry.price_with_metrics(
        fra,
        "discounting",
        market,
        ["par_rate", "pv01"],
        as_of=as_of,
    )

    # Basis swap: SOFR 3M vs 6M with small spread
    start = as_of + timedelta(days=2)
    maturity = date(as_of.year + 5, as_of.month, as_of.day)
    leg_3m = BasisSwapLeg(
        "USD-SOFR-3M",
        frequency="3M",
        day_count=DayCount.ACT_360,
        business_day_convention=BusinessDayConvention.MODIFIED_FOLLOWING,
        spread_bp=0.0,
    )
    leg_6m = BasisSwapLeg(
        "USD-SOFR-6M",
        frequency="6M",
        day_count=DayCount.ACT_360,
        business_day_convention=BusinessDayConvention.MODIFIED_FOLLOWING,
        spread_bp=5.0,  # 5bp
    )
    basis_swap = (
        BasisSwap.builder("USD-BASIS-3M-6M")
        .money(Money(25_000_000, USD))
        .start_date(start)
        .maturity(maturity)
        .primary_leg(leg_3m)
        .reference_leg(leg_6m)
        .disc_id("USD-OIS")
        .calendar("usny")
        .stub("none")
        .build()
    )
    registry.price_with_metrics(
        basis_swap,
        "discounting",
        market,
        ["dv01"],
        as_of=as_of,
    )

    # Interest-rate cap and floor built via helper constructors
    cap = (
        InterestRateOption.builder("USD-CAP-5Y")
        .kind("cap")
        .money(Money(10_000_000, USD))
        .strike(0.04)
        .start_date(start)
        .end_date(date(as_of.year + 5, as_of.month, as_of.day))
        .disc_id("USD-OIS")
        .fwd_id("USD-SOFR-3M")
        .vol_surface("USD-CAP-VOL")
        .payments_per_year(4)
        .day_count(DayCount.ACT_360)
        .build()
    )
    registry.price_with_metrics(
        cap,
        "discounting",
        market,
        ["vega", "delta"],
        as_of=as_of,
    )

    floor = (
        InterestRateOption.builder("USD-FLOOR-5Y")
        .kind("floor")
        .money(Money(10_000_000, USD))
        .strike(0.02)
        .start_date(start)
        .end_date(date(as_of.year + 5, as_of.month, as_of.day))
        .disc_id("USD-OIS")
        .fwd_id("USD-SOFR-3M")
        .vol_surface("USD-CAP-VOL")
        .payments_per_year(4)
        .day_count(DayCount.ACT_360)
        .build()
    )
    registry.price(floor, "discounting", market, as_of=as_of)

    # Interest-rate future (SOFR) with simple contract specs
    future = (
        InterestRateFuture.builder("SOFR-FUT-SEP24")
        .money(Money(1_000_000, USD))
        .quoted_price(97.25)
        .expiry(date(2024, 9, 16))
        .fixing_date(date(2024, 9, 18))
        .period_start(date(2024, 9, 18))
        .period_end(date(2024, 12, 18))
        .disc_id("USD-OIS")
        .fwd_id("USD-SOFR-3M")
        .position("long")
        .convexity_adjustment(0.0)
        .build()
    )
    registry.price_with_metrics(
        future,
        "discounting",
        market,
        ["dv01"],
        as_of=as_of,
    )

    # Swaption: payer on 5y underlying swap starting in 1y
    swaption = Swaption.payer(
        "USD-SWAPTION-1Yx5Y",
        Money(15_000_000, USD),
        strike=0.0325,
        expiry=date(as_of.year + 1, as_of.month, as_of.day),
        swap_start=date(as_of.year + 1, as_of.month, as_of.day),
        swap_end=date(as_of.year + 6, as_of.month, as_of.day),
        discount_curve="USD-OIS",
        forward_curve="USD-SOFR-3M",
        vol_surface="SWAPTION-VOL",
        exercise="european",
        settlement="physical",
    )
    registry.price_with_metrics(
        swaption,
        "discounting",
        market,
        ["vega", "delta"],
        as_of=as_of,
    )


if __name__ == "__main__":
    main()
