#!/usr/bin/env python3
"""Demonstrate creating and valuing core interest-rate instruments."""
from datetime import date, timedelta

from finstack import Money
from finstack.core.currency import USD
from finstack.core.dates.daycount import DayCount
from finstack.core.dates import BusinessDayConvention
from finstack.core.market_data import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve, ForwardCurve
from finstack.core.market_data.surfaces import VolSurface
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
    deposit = Deposit(
        "USD-DEP-3M",
        Money(5_000_000, USD),
        as_of,
        as_of + timedelta(days=92),
        DayCount.ACT_360,
        "USD-OIS",
        quote_rate=0.0450,
    )
    deposit_pricing = registry.price(deposit, "discounting", market)
    print("Deposit PV:", round(deposit_pricing.value.amount, 2), deposit_pricing.value.currency)

    # FRA: receive fixed vs pay floating (SOFR 3M)
    fra = ForwardRateAgreement.create(
        "USD-FRA-3x6",
        Money(10_000_000, USD),
        fixed_rate=0.0360,
        fixing_date=as_of + timedelta(days=30),
        start_date=as_of + timedelta(days=92),
        end_date=as_of + timedelta(days=182),
        discount_curve="USD-OIS",
        forward_curve="USD-SOFR-3M",
        pay_fixed=False,
    )
    fra_result = registry.price_with_metrics(
        fra,
        "discounting",
        market,
        ["par_rate", "pv01"],
    )
    print("FRA PV:", round(fra_result.value.amount, 2), fra_result.value.currency)
    print("FRA par rate:", fra_result.measures.get("par_rate"))

    # Basis swap: SOFR 3M vs 6M with small spread
    start = as_of + timedelta(days=2)
    maturity = date(as_of.year + 5, as_of.month, as_of.day)
    leg_3m = BasisSwapLeg("USD-SOFR-3M", frequency="quarterly", spread=0.0)
    leg_6m = BasisSwapLeg("USD-SOFR-6M", frequency="semi_annual", spread=5.0)
    basis_swap = BasisSwap.create(
        "USD-BASIS-3M-6M",
        Money(25_000_000, USD),
        start,
        maturity,
        leg_3m,
        leg_6m,
        "USD-OIS",
        stub="short_front",
    )
    basis_result = registry.price_with_metrics(
        basis_swap,
        "discounting",
        market,
        ["dv01"],
    )
    print("Basis swap PV:", round(basis_result.value.amount, 2), basis_result.value.currency)

    # Interest-rate cap and floor built via helper constructors
    cap = InterestRateOption.cap(
        "USD-CAP-5Y",
        Money(10_000_000, USD),
        strike=0.04,
        start_date=start,
        end_date=date(as_of.year + 5, as_of.month, as_of.day),
        discount_curve="USD-OIS",
        forward_curve="USD-SOFR-3M",
        vol_surface="USD-CAP-VOL",
    )
    cap_result = registry.price_with_metrics(
        cap,
        "discounting",
        market,
        ["vega", "delta"],
    )
    print("Cap PV:", round(cap_result.value.amount, 2), cap_result.value.currency)

    floor = InterestRateOption.floor(
        "USD-FLOOR-5Y",
        Money(10_000_000, USD),
        strike=0.02,
        start_date=start,
        end_date=date(as_of.year + 5, as_of.month, as_of.day),
        discount_curve="USD-OIS",
        forward_curve="USD-SOFR-3M",
        vol_surface="USD-CAP-VOL",
    )
    floor_pv = registry.price(floor, "discounting", market)
    print("Floor PV:", round(floor_pv.value.amount, 2), floor_pv.value.currency)

    # Interest-rate future (SOFR) with simple contract specs
    future = InterestRateFuture.create(
        "SOFR-FUT-SEP24",
        Money(1_000_000, USD),
        quoted_price=97.25,
        expiry=date(2024, 9, 16),
        fixing_date=date(2024, 9, 18),
        period_start=date(2024, 9, 18),
        period_end=date(2024, 12, 18),
        discount_curve="USD-OIS",
        forward_curve="USD-SOFR-3M",
        position="long",
    )
    future_result = registry.price_with_metrics(
        future,
        "discounting",
        market,
        ["dv01"],
    )
    print("IR future PV:", round(future_result.value.amount, 2), future_result.value.currency)

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
    swaption_result = registry.price_with_metrics(
        swaption,
        "discounting",
        market,
        ["vega", "delta"],
    )
    print("Swaption PV:", round(swaption_result.value.amount, 2), swaption_result.value.currency)


if __name__ == "__main__":
    main()
