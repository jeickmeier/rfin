"""Report-generation components example.

Walks through the structured report components exposed on
``finstack.valuations``:

1. :func:`metrics_table_from_dict` — key analytics (DV01, CS01, NPV, YTM)
2. :func:`cashflow_ladder` — time-bucketed cashflows
3. :func:`scenario_matrix` — base/upside/downside across several metrics
4. :func:`waterfall_from_steps` — P&L attribution waterfall
5. Formatting helpers — ``format_bps``, ``format_pct``, ``format_currency``,
   ``format_ratio``

Each component returns a dict with both a structured ``json`` payload and
a rendered ``markdown`` string, so the same result can feed a notebook
display, a downstream template, or a JSON API response without any further
conversion.

Run standalone:

    python finstack-py/examples/15_report_components.py
"""

from __future__ import annotations

from datetime import date, timedelta

from finstack.valuations import (
    cashflow_ladder,
    format_bps,
    format_currency,
    format_pct,
    format_ratio,
    metrics_table_from_dict,
    scenario_matrix,
    waterfall_from_steps,
)


# ---------------------------------------------------------------------------
# 1) Metrics table
# ---------------------------------------------------------------------------


def build_metrics_table() -> dict:
    """Assemble a key-metrics table for a hypothetical 5Y USD bond."""
    metrics = {
        "dv01": 425.37,        # currency-per-bp (auto-detected)
        "cs01": 180.12,        # currency-per-bp
        "ytm": 0.0475,         # percent (0.0475 -> 4.75%)
        "modified_duration": 4.25,  # years
        "convexity": 22.8,     # ratio
    }
    return metrics_table_from_dict(
        instrument_id="ACME-5Y-SNR",
        as_of="2026-04-16",
        currency="USD",
        npv=1_002_350.00,
        metrics=metrics,
    )


# ---------------------------------------------------------------------------
# 2) Cashflow ladder
# ---------------------------------------------------------------------------


def build_cashflow_ladder() -> dict:
    """20 quarterly cashflows — fixed coupon + bullet principal at the end."""
    n = 20
    notional = 1_000_000.0
    coupon_rate = 0.0475
    quarterly_interest = notional * coupon_rate / 4.0
    first = date(2026, 6, 15)

    dates: list[str] = []
    principal: list[float] = []
    interest: list[float] = []
    for i in range(n):
        # Roughly-quarterly spacing; Rust-side bucketing is by calendar period,
        # so exact day-count doesn't matter — we just need well-ordered dates.
        d = first + timedelta(days=91 * i)
        dates.append(d.isoformat())
        interest.append(quarterly_interest)
        principal.append(notional if i == n - 1 else 0.0)

    return cashflow_ladder(
        instrument_id="ACME-5Y-SNR",
        currency="USD",
        dates=dates,
        principal=principal,
        interest=interest,
        frequency="quarterly",
    )


# ---------------------------------------------------------------------------
# 3) Scenario matrix
# ---------------------------------------------------------------------------


def build_scenario_matrix() -> dict:
    """Three scenarios × four metrics, with ``Base`` as the reference."""
    scenarios = [
        (
            "Base",
            {"npv": 1_000_000.0, "dv01": 425.0, "ytm": 0.0475, "oas": 0.0125},
        ),
        (
            "Upside",
            {"npv": 1_035_000.0, "dv01": 430.0, "ytm": 0.0425, "oas": 0.0105},
        ),
        (
            "Downside",
            {"npv":   945_000.0, "dv01": 418.0, "ytm": 0.0565, "oas": 0.0185},
        ),
    ]
    return scenario_matrix(
        title="ACME-5Y-SNR scenario grid",
        scenarios=scenarios,
        base_case="Base",
    )


# ---------------------------------------------------------------------------
# 4) P&L waterfall
# ---------------------------------------------------------------------------


def build_pnl_waterfall() -> dict:
    """Classic P&L attribution waterfall: rates / credit / vol / basis."""
    start = 1_000_000.0
    steps = [
        ("Rates",  +18_250.0),
        ("Credit", +12_400.0),
        ("Vol",     -3_100.0),
        ("Basis",   +1_850.0),
    ]
    end = start + sum(v for _, v in steps)
    return waterfall_from_steps(
        title="ACME-5Y-SNR daily P&L",
        currency="USD",
        start_value=start,
        end_value=end,
        steps=steps,
    )


# ---------------------------------------------------------------------------
# 5) Formatting helpers
# ---------------------------------------------------------------------------


def show_formatters() -> None:
    print("=" * 72)
    print("Formatting helpers")
    print("=" * 72)
    print(f"  format_bps(0.0025, 1)       = {format_bps(0.0025, 1)!r}")
    print(f"  format_bps(-0.00125, 2)     = {format_bps(-0.00125, 2)!r}")
    print(f"  format_pct(0.0534, 2)       = {format_pct(0.0534, 2)!r}")
    print(f"  format_pct(1.0, 0)          = {format_pct(1.0, 0)!r}")
    print(
        f"  format_currency(1_234_567.89, 'USD', 2) = "
        f"{format_currency(1_234_567.89, 'USD', 2)!r}"
    )
    print(
        f"  format_currency(-500.0, 'EUR', 0)       = "
        f"{format_currency(-500.0, 'EUR', 0)!r}"
    )
    print(f"  format_ratio(3.5, 2)        = {format_ratio(3.5, 2)!r}")
    print()


# ---------------------------------------------------------------------------
# Driver
# ---------------------------------------------------------------------------


def _print_component(title: str, component: dict) -> None:
    print("=" * 72)
    print(title)
    print("=" * 72)
    print(component["markdown"])
    # Smoke-check that the structured payload round-trips as a native dict.
    payload = component["json"]
    assert isinstance(payload, dict), "json payload should be a dict"
    print(f"[json keys: {sorted(payload.keys())}]\n")


def main() -> None:
    _print_component("1) Metrics table", build_metrics_table())
    _print_component("2) Cashflow ladder", build_cashflow_ladder())
    _print_component("3) Scenario matrix", build_scenario_matrix())
    _print_component("4) P&L waterfall", build_pnl_waterfall())
    show_formatters()


if __name__ == "__main__":
    main()
