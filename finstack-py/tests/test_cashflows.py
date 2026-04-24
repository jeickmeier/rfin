"""Cashflow JSON bridge tests."""

from __future__ import annotations

import json

from finstack.cashflows import (
    accrued_interest,
    bond_from_cashflows,
    build_cashflow_schedule,
    dated_flows,
    validate_cashflow_schedule,
)
from finstack.valuations import price_instrument


def _cashflow_spec() -> str:
    return json.dumps({
        "notional": {
            "initial": {"amount": "1000000", "currency": "USD"},
            "amort": "None",
        },
        "issue": "2024-08-31",
        "maturity": "2025-08-31",
        "fixed_coupons": [
            {
                "coupon_type": "Cash",
                "rate": "0.06",
                "freq": {"count": 12, "unit": "months"},
                "dc": "Thirty360",
                "bdc": "following",
                "calendar_id": "weekends_only",
                "stub": "None",
                "end_of_month": False,
                "payment_lag_days": 0,
            }
        ],
    })


def _market_json() -> str:
    return json.dumps({
        "version": 2,
        "curves": [
            {
                "type": "discount",
                "id": "USD-OIS",
                "base": "2024-01-01",
                "day_count": "Act365F",
                "knot_points": [[0.0, 1.0], [1.0, 0.95], [5.0, 0.80]],
                "interp_style": "linear",
                "extrapolation": "flat_forward",
                "min_forward_rate": None,
                "allow_non_monotonic": False,
                "min_forward_tenor": 1e-6,
            }
        ],
        "fx": None,
        "surfaces": [],
        "prices": {},
        "series": [],
        "inflation_indices": [],
        "dividends": [],
        "credit_indices": [],
        "fx_delta_vol_surfaces": [],
        "vol_cubes": [],
        "collateral": {},
    })


def test_cashflows_namespace_build_validate_accrual_and_price_bond() -> None:
    schedule_json = build_cashflow_schedule(_cashflow_spec())
    schedule = json.loads(schedule_json)
    assert schedule["meta"]["issue_date"] == "2024-08-31"

    assert json.loads(validate_cashflow_schedule(schedule_json)) == schedule
    flows = json.loads(dated_flows(schedule_json))
    assert len(flows) == len(schedule["flows"])
    assert accrued_interest(schedule_json, "2025-02-28") > 0.0

    instrument_json = bond_from_cashflows("CUSTOM-CF", schedule_json, "USD-OIS", 99.0)
    instrument = json.loads(instrument_json)
    assert instrument["type"] == "bond"

    result = json.loads(price_instrument(instrument_json, _market_json(), "2024-09-03", "discounting"))
    assert result["instrument_id"] == "CUSTOM-CF"
    assert result["value"]["currency"] == "USD"
