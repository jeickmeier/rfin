#!/usr/bin/env python3
"""Demonstrate finstack.valuations.cashflow builder capabilities.

Run after installing the extension in editable mode:

    uv run maturin develop
    uv run python finstack-py/examples/scripts/valuations/cashflow_builder_capabilities.py

This script showcases the composable CashflowBuilder which supports:
- Fixed and floating coupon schedules
- Cash/PIK/split payment types
- Amortization (linear, step schedules)
- Step-up coupon programs
- Payment split programs (cash-to-PIK transitions)

Examples Included:
1. Simple fixed coupon bond (quarterly 5% coupons)
2. Floating rate note (SOFR + 150 bps margin)
3. PIK toggle bond (70% cash / 30% PIK)
4. Amortizing loan with linear amortization
5. Step amortization schedule
6. Step-up coupon structure (4% → 5% → 6%)
7. Payment split program (cash → 50/50 → PIK)
8. Complex structure combining amortization and step-up
"""

from __future__ import annotations

from datetime import date

import polars as pl

from finstack import Money
from finstack.core.currency import EUR, USD
from finstack.core.dates import BusinessDayConvention
from finstack.core.dates.daycount import DayCount
from finstack.core.dates.schedule import Frequency, StubKind
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.cashflow import (
    AmortizationSpec,
    CashflowBuilder,
    CouponType,
    FixedCouponSpec,
    FloatCouponParams,
    FloatingCouponSpec,
    ScheduleParams,
)


def format_cashflow_table(cf_schedule, max_rows=None):
    """Display cashflows using the Rust-generated Polars DataFrame.

    Args:
        cf_schedule: CashFlowSchedule object
        max_rows: Maximum number of rows to display (None for all)

    Note:
        All calculations (rates, outstanding) are done in Rust for performance.
        DataFrame includes separate cash_rate_pct and pik_rate_pct columns.
    """
    # Create minimal market context for DataFrame export
    market = MarketContext()
    # Use first flow date as discount curve base date (or default to 2025-01-01)
    flows = list(cf_schedule.flows())
    if flows:
        base_date = flows[0].date
    else:
        base_date = date(2025, 1, 1)
    discount_curve = DiscountCurve(
        "DISCOUNT",
        base_date,
        [(0.0, 1.0), (10.0, 0.7)],  # Simple flat curve
    )
    market.insert_discount(discount_curve)

    # Get DataFrame from Rust (all calculations done there)
    df_dict = cf_schedule.to_dataframe(market=market, discount_curve_id="DISCOUNT")
    df = pl.DataFrame(df_dict)

    # Limit rows if specified
    if max_rows is not None and len(df) > max_rows:
        df_display = df.head(max_rows)
        print(df_display)
        print(f"\n  ... ({len(df) - max_rows} more rows)")
    else:
        print(df)

    print()


def example_1_simple_fixed_coupon():
    """Example 1: Simple fixed-rate bond with quarterly 5% coupons."""
    print("\n" + "=" * 80)
    print("Example 1: Simple Fixed Coupon Bond")
    print("=" * 80)

    issue = date(2025, 1, 15)
    maturity = date(2027, 1, 15)
    notional = Money(1_000_000, USD)

    # Use convenience helper for quarterly Act/360
    schedule = ScheduleParams.quarterly_act360()

    # Define 5% fixed coupon
    fixed_spec = FixedCouponSpec.new(
        rate=0.05,
        schedule=schedule,
        coupon_type=CouponType.CASH,
    )

    # Build cashflow schedule
    builder = CashflowBuilder.new()
    builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
    builder.fixed_cf(fixed_spec)

    schedule = builder.build_with_curves(None)

    print(f"Notional: {schedule.notional.format()}")
    print(f"Day Count: {schedule.day_count.name}")
    print(f"\nCash flows ({len(schedule.flows())} total):")
    format_cashflow_table(schedule, max_rows=8)


def example_2_floating_coupon():
    """Example 2: Floating-rate note with SOFR + 150 bps margin."""
    print("\n" + "=" * 80)
    print("Example 2: Floating Rate Note (SOFR + 150 bps)")
    print("=" * 80)

    issue = date(2025, 3, 1)
    maturity = date(2028, 3, 1)
    notional = Money(5_000_000, USD)

    # Define schedule params
    schedule = ScheduleParams.quarterly_act360()

    # Define floating coupon: SOFR + 150 bps
    float_params = FloatCouponParams.new(
        index_id="USD-SOFR-3M",
        margin_bp=150.0,  # 150 basis points = 1.5%
        gearing=1.0,
        reset_lag_days=2,
    )

    float_spec = FloatingCouponSpec.new(
        params=float_params,
        schedule=schedule,
        coupon_type=CouponType.CASH,
    )

    # Build schedule
    builder = CashflowBuilder.new()
    builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
    builder.floating_cf(float_spec)

    cf_schedule = builder.build_with_curves(None)

    print(f"Notional: {cf_schedule.notional.format()}")
    print(f"Index: USD-SOFR-3M with +150 bps margin")
    print(f"\nCash flows:")
    format_cashflow_table(cf_schedule, max_rows=8)


def example_3_pik_toggle():
    """Example 3: PIK toggle bond (split between cash and payment-in-kind)."""
    print("\n" + "=" * 80)
    print("Example 3: PIK Toggle Bond (70% Cash / 30% PIK)")
    print("=" * 80)

    issue = date(2025, 1, 1)
    maturity = date(2030, 1, 1)
    notional = Money(2_000_000, EUR)

    schedule = ScheduleParams.semiannual_30360()

    # Split coupon: 70% cash, 30% PIK
    fixed_spec = FixedCouponSpec.new(
        rate=0.08,  # 8% coupon
        schedule=schedule,
        coupon_type=CouponType.split(0.7, 0.3),
    )

    builder = CashflowBuilder.new()
    builder.principal(amount=notional.amount, currency=EUR, issue=issue, maturity=maturity)
    builder.fixed_cf(fixed_spec)

    cf_schedule = builder.build_with_curves(None)

    print(f"Notional: {cf_schedule.notional.format()}")
    print(f"Coupon: 8% (70% cash, 30% PIK)")
    print(f"\nCash flows:")
    format_cashflow_table(cf_schedule, max_rows=8)


def example_4_amortizing_loan():
    """Example 4: Amortizing loan with linear amortization."""
    print("\n" + "=" * 80)
    print("Example 4: Amortizing Loan (Linear to 20% of notional)")
    print("=" * 80)

    issue = date(2025, 6, 1)
    maturity = date(2030, 6, 1)
    notional = Money(10_000_000, USD)
    final_notional = Money(2_000_000, USD)  # Amortize down to 20%

    # Create schedule with custom params
    schedule = ScheduleParams.new(
        freq=Frequency.QUARTERLY,
        day_count=DayCount.ACT_360,
        bdc=BusinessDayConvention.MODIFIED_FOLLOWING,
        calendar_id="usny",
        stub=StubKind.NONE,
    )

    # Fixed 6% coupon
    fixed_spec = FixedCouponSpec.new(
        rate=0.06,
        schedule=schedule,
        coupon_type=CouponType.CASH,
    )

    # Linear amortization
    amort_spec = AmortizationSpec.linear_to(final_notional)

    builder = CashflowBuilder.new()
    builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
    builder.amortization(amort_spec)
    builder.fixed_cf(fixed_spec)

    cf_schedule = builder.build_with_curves(None)

    print(f"Initial Notional: {notional.format()}")
    print(f"Final Notional: {final_notional.format()}")
    print(f"\nCash flows (showing interest + amortization):")
    format_cashflow_table(cf_schedule, max_rows=12)


def example_5_step_amortization():
    """Example 5: Step amortization schedule."""
    print("\n" + "=" * 80)
    print("Example 5: Step Amortization Schedule")
    print("=" * 80)

    issue = date(2025, 1, 1)
    maturity = date(2030, 1, 1)
    notional = Money(10_000_000, USD)

    schedule = ScheduleParams.annual_actact()

    fixed_spec = FixedCouponSpec.new(
        rate=0.055,  # 5.5%
        schedule=schedule,
        coupon_type=CouponType.CASH,
    )

    # Define step amortization: remaining balance at specific dates
    amort_steps = [
        (date(2027, 1, 1), Money(8_000_000, USD)),  # After 2 years: 80% remaining
        (date(2028, 1, 1), Money(6_000_000, USD)),  # After 3 years: 60% remaining
        (date(2029, 1, 1), Money(3_000_000, USD)),  # After 4 years: 30% remaining
    ]

    amort_spec = AmortizationSpec.step_remaining(amort_steps)

    builder = CashflowBuilder.new()
    builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
    builder.amortization(amort_spec)
    builder.fixed_cf(fixed_spec)

    cf_schedule = builder.build_with_curves(None)

    print(f"Initial Notional: {notional.format()}")
    print(f"Amortization steps at: 2027, 2028, 2029")
    print(f"\nCash flows:")
    format_cashflow_table(cf_schedule)


def example_6_step_up_coupon():
    """Example 6: Step-up coupon structure."""
    print("\n" + "=" * 80)
    print("Example 6: Step-Up Coupon Structure")
    print("=" * 80)

    issue = date(2025, 1, 1)
    maturity = date(2032, 1, 1)
    notional = Money(3_000_000, USD)

    schedule = ScheduleParams.semiannual_30360()

    # Define step-up program:
    # - 4% for first 2 years
    # - 5% for next 3 years
    # - 6% thereafter
    step_program = [
        (date(2027, 1, 1), 0.04),  # 4% until 2027
        (date(2030, 1, 1), 0.05),  # 5% until 2030
        (date(2032, 1, 1), 0.06),  # 6% until maturity
    ]

    builder = CashflowBuilder.new()
    builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
    builder.fixed_stepup(
        steps=step_program,
        schedule=schedule,
        default_split=CouponType.CASH,
    )

    cf_schedule = builder.build_with_curves(None)

    print(f"Notional: {cf_schedule.notional.format()}")
    print("Step-up schedule:")
    print("  2025-2027: 4%")
    print("  2027-2030: 5%")
    print("  2030-2032: 6%")
    print(f"\nCash flows:")
    format_cashflow_table(cf_schedule, max_rows=12)


def example_7_payment_split_program():
    """Example 7: Payment split program (cash-to-PIK transition)."""
    print("\n" + "=" * 80)
    print("Example 7: Payment Split Program (Cash → PIK Transition)")
    print("=" * 80)

    issue = date(2025, 1, 1)
    maturity = date(2030, 1, 1)
    notional = Money(5_000_000, USD)

    schedule = ScheduleParams.quarterly_act360()

    # Start with 7% fixed coupon
    fixed_spec = FixedCouponSpec.new(
        rate=0.07,
        schedule=schedule,
        coupon_type=CouponType.CASH,  # Initial default
    )

    # Define payment split program:
    # - Full cash until 2027
    # - 50/50 cash/PIK from 2027-2028
    # - Full PIK thereafter
    split_program = [
        (date(2027, 1, 1), CouponType.CASH),  # 100% cash
        (date(2028, 1, 1), CouponType.split(0.5, 0.5)),  # 50/50 split
        (date(2030, 1, 1), CouponType.PIK),  # 100% PIK
    ]

    builder = CashflowBuilder.new()
    builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
    builder.fixed_cf(fixed_spec)
    builder.payment_split_program(split_program)

    cf_schedule = builder.build_with_curves(None)

    print(f"Notional: {cf_schedule.notional.format()}")
    print("Payment program:")
    print("  2025-2027: 100% cash")
    print("  2027-2028: 50% cash / 50% PIK")
    print("  2028-2030: 100% PIK")
    print(f"\nCash flows (showing transitions):")
    format_cashflow_table(cf_schedule, max_rows=15)


def example_8_complex_structure():
    """Example 8: Complex structure combining amortization and step-up."""
    print("\n" + "=" * 80)
    print("Example 8: Complex Structure (Amortizing + Step-up)")
    print("=" * 80)

    issue = date(2025, 1, 1)
    maturity = date(2035, 1, 1)
    notional = Money(20_000_000, USD)
    final_notional = Money(5_000_000, USD)

    schedule = ScheduleParams.quarterly_act360()

    # Step-up coupon program
    step_program = [
        (date(2028, 1, 1), 0.06),  # 6% for first 3 years
        (date(2032, 1, 1), 0.07),  # 7% for next 4 years
        (date(2035, 1, 1), 0.08),  # 8% final 3 years
    ]

    # Linear amortization
    amort_spec = AmortizationSpec.linear_to(final_notional)

    builder = CashflowBuilder.new()
    builder.principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
    builder.amortization(amort_spec)
    builder.fixed_stepup(steps=step_program, schedule=schedule, default_split=CouponType.CASH)

    cf_schedule = builder.build_with_curves(None)

    print(f"Initial Notional: {notional.format()}")
    print(f"Final Notional: {final_notional.format()}")
    print("\nFeatures:")
    print("  • Linear amortization to 25% of notional")
    print("  • Step-up coupons: 6% → 7% → 8%")
    print(f"\nCash flows:")
    format_cashflow_table(cf_schedule, max_rows=20)


def main():
    """Run all cashflow builder examples."""
    print("\n" + "#" * 80)
    print("# FINSTACK CASHFLOW BUILDER CAPABILITIES")
    print("#" * 80)
    print("\nDemonstrating composable cashflow building with:")
    print("  • Fixed and floating coupons")
    print("  • Cash / PIK / Split payment types")
    print("  • Amortization schedules")
    print("  • Step-up coupon programs")
    print("  • Payment split programs")

    example_1_simple_fixed_coupon()
    example_2_floating_coupon()
    example_3_pik_toggle()
    example_4_amortizing_loan()
    example_5_step_amortization()
    example_6_step_up_coupon()
    example_7_payment_split_program()
    example_8_complex_structure()

    print("\n" + "#" * 80)
    print("# All examples completed successfully!")
    print("#" * 80)


if __name__ == "__main__":
    main()
