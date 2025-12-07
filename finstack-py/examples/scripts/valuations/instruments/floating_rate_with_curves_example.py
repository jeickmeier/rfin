#!/usr/bin/env python3
"""Demonstrate floating rate cashflow calculation with and without market curves.

This example shows how build() vs build_with_curves() produces different cashflow amounts
for floating rate instruments:
- build(): Uses only the margin (spread)
- build_with_curves(): Uses forward_rate * gearing + margin from market curves

Run after installing the extension:
    uv run maturin develop
    uv run python finstack-py/examples/scripts/valuations/floating_rate_with_curves_example.py
"""

from __future__ import annotations

from datetime import date

from finstack import Money
from finstack.core.currency import USD
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve, ForwardCurve
from finstack.valuations.cashflow import (
    CashflowBuilder,
    ScheduleParams,
    FloatCouponParams,
    FloatingCouponSpec,
    CouponType,
)


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
    builder = CashflowBuilder.new()
    builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
    builder.floating_cf(float_spec)

    print("=" * 80)
    print("FLOATING RATE CASHFLOW COMPARISON")
    print("=" * 80)
    print(f"\nInstrument: FRN with USD-SOFR-3M + 150 bps")
    print(f"Notional: {notional.format()}")
    print(f"Period: {issue} → {maturity}")
    print()

    # Example 1: Build WITHOUT curves (margin only)
    print("-" * 80)
    print("Example 1: build() - Margin Only (No Forward Rates)")
    print("-" * 80)
    sched_no_curves = builder.build()
    flows_no_curves = sched_no_curves.flows()

    print(f"\nCalculation: coupon = outstanding * (margin_bp * 1e-4 * gearing) * year_fraction")
    print(f"            = outstanding * (150 * 0.0001 * 1.0) * yf")
    print(f"            = outstanding * 0.0150 * yf\n")

    print("First 3 cashflows:")
    for i, flow in enumerate(flows_no_curves[:3]):
        print(f"  {flow.date}: {flow.amount.format():>15} ({flow.kind.name}, accrual={flow.accrual_factor:.6f})")

    # Example 2: Build WITH curves (forward rate + margin)
    print("\n" + "-" * 80)
    print("Example 2: build_with_curves() - Forward Rates from Market")
    print("-" * 80)

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
            (0.0, 0.0300),   # 3.00% at t=0
            (0.5, 0.0325),   # 3.25% at 6 months
            (1.0, 0.0350),   # 3.50% at 1 year
            (2.0, 0.0400),   # 4.00% at 2 years
        ],
        base_date=base_date,
    )
    market.insert_forward(forward_curve)

    print("\nForward Curve (USD-SOFR-3M):")
    for t, rate in [(0.0, 0.0300), (0.5, 0.0325), (1.0, 0.0350), (2.0, 0.0400)]:
        print(f"  t={t:.1f}y: {rate*100:.2f}%")

    print(f"\nCalculation: coupon = outstanding * (forward_rate * gearing + margin_bp * 1e-4) * yf")
    print(f"            = outstanding * (forward_rate * 1.0 + 0.0150) * yf\n")

    sched_with_curves = builder.build_with_curves(market)
    flows_with_curves = sched_with_curves.flows()

    print("First 3 cashflows:")
    for i, flow in enumerate(flows_with_curves[:3]):
        print(f"  {flow.date}: {flow.amount.format():>15} ({flow.kind.name}, accrual={flow.accrual_factor:.6f})")

    # Example 3: Comparison
    print("\n" + "=" * 80)
    print("COMPARISON: build() vs build_with_curves()")
    print("=" * 80)
    print(f"\n{'Date':<12} {'Without Curves':>18} {'With Curves':>18} {'Difference':>18}")
    print("-" * 72)

    for i in range(min(5, len(flows_no_curves), len(flows_with_curves))):
        f_no = flows_no_curves[i]
        f_with = flows_with_curves[i]
        
        if f_no.kind.name == 'float_reset':  # Only compare float flows
            diff = f_with.amount.amount - f_no.amount.amount
            print(
                f"{f_no.date}  "
                f"{f_no.amount.amount:>18,.2f}  "
                f"{f_with.amount.amount:>18,.2f}  "
                f"{diff:>18,.2f}"
            )

    print("\n" + "=" * 80)
    print("KEY INSIGHTS")
    print("=" * 80)
    print("""
1. build() - Margin Only:
   - Uses fixed margin regardless of market conditions
   - Appropriate for initial modeling without market data
   - Formula: outstanding * (margin_bp * 0.0001 * gearing) * year_fraction

2. build_with_curves() - Market-Based:
   - Incorporates forward rates from ForwardCurve
   - Reflects actual market expectations for floating rates
   - Formula: outstanding * (forward_rate * gearing + margin_bp * 0.0001) * year_fraction
   - Reset date = payment_date - reset_lag_days (adjusted for business days)

3. Use Cases:
   - Without curves: Quick estimates, template generation, initial modeling
   - With curves: Accurate valuations, what-if scenarios, portfolio analytics
    """)


if __name__ == "__main__":
    main()

