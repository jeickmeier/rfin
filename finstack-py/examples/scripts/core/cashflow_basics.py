"""Demonstrate finstack.core.cashflow helpers for building cash-flow schedules.

Run after installing the extension in editable mode, for example:

    uv run maturin develop
    uv run python finstack-py/examples/core/cashflow_basics.py

The script constructs several cash-flow primitives (fixed, floating, fees,
principal exchanges) and shows how to inspect their metadata from Python.
"""

from __future__ import annotations

from datetime import date

import finstack


def main() -> None:
    cashflow = finstack.core.cashflow.CashFlow
    money = finstack.Money

    print("=== Cashflow primitives ===")
    fixed_cf = cashflow.fixed(
        date(2025, 3, 15),
        money(12_500.0, "USD"),
        accrual_factor=0.25,
    )
    float_cf = cashflow.floating(
        date(2025, 6, 15),
        (13_750.0, finstack.Currency("USD")),
        reset_date=date(2025, 3, 15),
        accrual_factor=0.25,
    )
    fee_cf = cashflow.fee(date(2025, 1, 15), money(150_000.0, "USD"))
    principal_cf = cashflow.principal_exchange(date(2030, 3, 15), money(-5_000_000.0, "USD"))

    for label, cf in [
        ("Fixed coupon", fixed_cf),
        ("Floating coupon", float_cf),
        ("Up-front fee", fee_cf),
        ("Principal exchange", principal_cf),
    ]:
        print(f"{label}: {cf}")
        print(
            "  kind=",
            cf.kind.name,
            "; date=",
            cf.date,
            "; amount=",
            cf.amount.format(),
            "; accrual=",
            cf.accrual_factor,
        )

    print("\n=== Tuple conversion ===")
    tuple_view = fixed_cf.to_tuple()
    print("CashFlow tuple:", tuple_view)

    print("\n=== Sorting by date ===")
    schedule = sorted([fixed_cf, float_cf, fee_cf, principal_cf], key=lambda item: item.date)
    for cf in schedule:
        print(cf.date, cf.kind.name, cf.amount.amount)


if __name__ == "__main__":
    main()
