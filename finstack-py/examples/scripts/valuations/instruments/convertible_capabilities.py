#!/usr/bin/env python3
"""Convertible bond example showcasing conversion features and pricing."""

from datetime import date

from finstack.core.currency import USD
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.scalars import MarketScalar
from finstack.core.market_data.surfaces import VolSurface
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.cashflow import CouponType, FixedCouponSpec, ScheduleParams
from finstack.valuations.instruments import ConversionEvent, ConversionPolicy, ConversionSpec, ConvertibleBond
from finstack.valuations.pricer import standard_registry

from finstack import Money


def build_market(as_of: date) -> MarketContext:
    """Prepare discounting, equity spot inputs, dividend yield, and vol surface."""
    market = MarketContext()

    disc = DiscountCurve(
        "USD-OIS",
        as_of,
        [
            (0.0, 1.0),
            (0.5, 0.9970),
            (1.0, 0.9940),
            (3.0, 0.9730),
            (5.0, 0.9490),
        ],
    )
    market.insert(disc)

    # Underlying equity data consumed by convertible pricing
    market.insert_price("EQUITY-SPOT", MarketScalar.price(Money(150.0, USD)))
    market.insert_price("EQUITY-DIVYIELD", MarketScalar.unitless(0.015))

    vol_surface = VolSurface(
        "EQUITY-VOL",
        expiries=[0.25, 0.5, 1.0, 2.0, 3.0],
        strikes=[120.0, 140.0, 160.0, 180.0],
        grid=[
            [0.28, 0.26, 0.25, 0.24],
            [0.27, 0.25, 0.24, 0.23],
            [0.26, 0.24, 0.23, 0.22],
            [0.25, 0.23, 0.22, 0.21],
            [0.24, 0.22, 0.21, 0.20],
        ],
    )
    market.insert_surface(vol_surface)

    return market


def build_convertible(issue: date) -> ConvertibleBond:
    """Create a simple USD convertible with semi-annual coupons."""
    maturity = date(issue.year + 5, issue.month, issue.day)
    schedule = ScheduleParams.semiannual_30360()
    fixed_coupon = FixedCouponSpec.new(
        rate=0.035,
        schedule=schedule,
        coupon_type=CouponType.CASH,
    )

    conversion_policy = ConversionPolicy.upon_event(ConversionEvent.price_trigger(160.0, 30))
    conversion_spec = ConversionSpec(
        conversion_policy,
        ratio=20.0,  # 20 shares per bond
        anti_dilution=None,
    )

    return (
        ConvertibleBond.builder("ACME-CB-2029")
        .notional(Money(1_000_000, USD))
        .issue(issue)
        .maturity(maturity)
        .discount_curve("USD-OIS")
        .conversion(conversion_spec)
        .underlying_equity_id("EQUITY-SPOT")
        .call_schedule([(date(issue.year + 3, issue.month, issue.day), 102.5)])
        .fixed_coupon(fixed_coupon)
        .build()
    )


def main() -> None:
    as_of = date(2024, 1, 2)
    market = build_market(as_of)
    registry = standard_registry()

    convertible = build_convertible(as_of)

    registry.price_with_metrics(
        convertible,
        "discounting",
        market,
        as_of,
        metrics=["delta", "gamma", "vega"],
    )

    convertible.parity(market)


if __name__ == "__main__":
    main()
