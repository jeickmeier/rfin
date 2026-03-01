#!/usr/bin/env python3
"""Demonstrate floating rate cashflow calculation with and without market curves.

This example shows how build_with_curves(None) vs build_with_curves(market) produces different cashflow amounts
for floating rate instruments:
- build_with_curves(None): Uses only the margin (spread)
- build_with_curves(market): Uses forward_rate * gearing + margin from market curves

Run after installing the extension:
    uv run maturin develop
    uv run python finstack-py/examples/scripts/valuations/floating_rate_with_curves_example.py
"""

from __future__ import annotations

from datetime import date

from finstack.core.currency import USD
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve, ForwardCurve
from finstack.valuations.cashflow import (
    CashFlowBuilder,
    CouponType,
    FloatCouponParams,
    FloatingCouponSpec,
    ScheduleParams,
)

from finstack import Money


def main() -> None:
    issue = date(2025, 1, 15)
    maturity = date(2027, 1, 15)
    notional = Money(5_000_000, USD)

    # Define floating rate spec: USD-SOFR-3M + 150 bps
    schedule_params = ScheduleParams.quarterly_act360()
    float_params = FloatCouponParams.new(
        index_id="USD-SOFR-3M",
        margin_bp=150.0,  # 150 basis points
        gearing=1.0,
        reset_lag_days=2,
    )
    float_spec = FloatingCouponSpec.new(
        params=float_params,
        schedule=schedule_params,
        coupon_type=CouponType.CASH,
    )

    # Build cashflow builder
    builder = CashFlowBuilder.new()
    builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
    builder.floating_cf(float_spec)

    # Example 1: Build WITHOUT curves (margin only)
    sched_no_curves = builder.build_with_curves(None)
    flows_no_curves = sched_no_curves.flows()

    for i, _flow in enumerate(flows_no_curves[:3]):
        pass

    # Example 2: Build WITH curves (forward rate + margin)

    # Create market context with forward curve
    base_date = date(2025, 1, 2)
    market = MarketContext()

    discount_curve = DiscountCurve(
        "USD-OIS",
        base_date,
        [(0.0, 1.0), (1.0, 0.9950), (2.0, 0.9880), (3.0, 0.9800)],
    )
    market.insert_discount(discount_curve)

    # Forward curve with varying rates: 3.0% → 3.5% → 4.0%
    forward_curve = ForwardCurve(
        "USD-SOFR-3M",
        0.25,  # 3-month tenor
        [
            (0.0, 0.0300),  # 3.00% at t=0
            (0.5, 0.0325),  # 3.25% at 6 months
            (1.0, 0.0350),  # 3.50% at 1 year
            (2.0, 0.0400),  # 4.00% at 2 years
        ],
        base_date=base_date,
    )
    market.insert_forward(forward_curve)

    for _t, _rate in [(0.0, 0.0300), (0.5, 0.0325), (1.0, 0.0350), (2.0, 0.0400)]:
        pass

    sched_with_curves = builder.build_with_curves(market)
    flows_with_curves = sched_with_curves.flows()

    for i, _flow in enumerate(flows_with_curves[:3]):
        pass

    # Example 3: Comparison

    for i in range(min(5, len(flows_no_curves), len(flows_with_curves))):
        f_no = flows_no_curves[i]
        f_with = flows_with_curves[i]

        if f_no.kind.name == "float_reset":  # Only compare float flows
            f_with.amount.amount - f_no.amount.amount


if __name__ == "__main__":
    main()
