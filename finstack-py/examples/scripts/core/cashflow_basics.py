"""Demonstrate finstack.core.cashflow helpers for building cash-flow schedules.

Run after installing the extension in editable mode, for example:

    uv run maturin develop
    uv run python finstack-py/examples/core/cashflow_basics.py

The script constructs several cash-flow primitives (fixed, floating, fees,
principal exchanges) and shows how to inspect their metadata from Python.
Note: CashFlow primitives are typically created via the CashflowBuilder for
scheduled flows. This example shows direct construction for illustration.
"""

from __future__ import annotations

from datetime import date

import finstack


def main() -> None:
    cashflow = finstack.core.cashflow.CashFlow
    cfkind = finstack.core.cashflow.CFKind
    money = finstack.Money

    # Note: For production use, prefer CashflowBuilder for scheduled flows
    fixed_cf = cashflow(
        date=date(2025, 3, 15),
        amount=money(12_500.0, "USD"),
        kind=cfkind.from_name("Fixed"),
        accrual_factor=0.25,
    )
    float_cf = cashflow(
        date=date(2025, 6, 15),
        amount=money(13_750.0, "USD"),
        kind=cfkind.from_name("float_reset"),
        reset_date=date(2025, 3, 15),
        accrual_factor=0.25,
    )
    fee_cf = cashflow(
        date=date(2025, 1, 15),
        amount=money(150_000.0, "USD"),
        kind=cfkind.from_name("Fee"),
    )
    principal_cf = cashflow(
        date=date(2030, 3, 15),
        amount=money(-5_000_000.0, "USD"),
        kind=cfkind.from_name("Notional"),
    )

    for _label, _cf in [
        ("Fixed coupon", fixed_cf),
        ("Floating coupon", float_cf),
        ("Up-front fee", fee_cf),
        ("Principal exchange", principal_cf),
    ]:
        pass

    fixed_cf.to_tuple()

    schedule = sorted([fixed_cf, float_cf, fee_cf, principal_cf], key=lambda item: item.date)
    for _cf in schedule:
        pass


if __name__ == "__main__":
    main()
