"""JSON pricing tests for valuation registry routes."""

from __future__ import annotations

import json

import pytest

from finstack.valuations import price_instrument


def _money(amount: str) -> dict[str, str]:
    return {"amount": amount, "currency": "USD"}


def _credit_enhancement() -> dict[str, object]:
    return {
        "subordination": _money("0"),
        "overcollateralization": _money("0"),
        "reserve_account": _money("0"),
        "excess_spread": 0.0,
        "cash_trap_active": False,
    }


def _tranche(
    tranche_id: str,
    attachment: float,
    detachment: float,
    seniority: str,
    balance: str,
    rate: float,
    priority: int,
) -> dict[str, object]:
    return {
        "id": tranche_id,
        "attachment_point": attachment,
        "detachment_point": detachment,
        "behavior_type": "Standard",
        "seniority": seniority,
        "rating": None,
        "original_balance": _money(balance),
        "current_balance": _money(balance),
        "target_balance": None,
        "coupon": {"Fixed": {"rate": rate}},
        "oc_trigger": None,
        "ic_trigger": None,
        "credit_enhancement": _credit_enhancement(),
        "frequency": {"count": 3, "unit": "months"},
        "day_count": "Act360",
        "deferred_interest": _money("0"),
        "pik_enabled": False,
        "is_revolving": False,
        "can_reinvest": False,
        "maturity": "2026-01-01",
        "expected_maturity": None,
        "payment_priority": priority,
        "attributes": {},
    }


def _structured_credit_json() -> str:
    pool = {
        "id": "POOL",
        "deal_type": "ABS",
        "assets": [
            {
                "id": "A1",
                "asset_type": {"type": "HighYieldBond", "industry": None},
                "balance": _money("1000000"),
                "rate": 0.06,
                "spread_bps": None,
                "index_id": None,
                "maturity": "2026-01-01",
                "credit_quality": None,
                "industry": None,
                "obligor_id": None,
                "is_defaulted": False,
                "recovery_amount": None,
                "purchase_price": None,
                "acquisition_date": None,
                "day_count": "Thirty360",
                "smm_override": None,
                "mdr_override": None,
            }
        ],
        "cumulative_defaults": _money("0"),
        "cumulative_recoveries": _money("0"),
        "cumulative_prepayments": _money("0"),
        "cumulative_scheduled_amortization": None,
        "reinvestment_period": None,
        "collection_account": _money("0"),
        "reserve_account": _money("0"),
        "excess_spread_account": _money("0"),
        "rep_lines": None,
    }
    spec = {
        "id": "ABS-STOCH-PV",
        "deal_type": "ABS",
        "pool": pool,
        "tranches": {
            "tranches": [
                _tranche("SR", 0.0, 80.0, "Senior", "800000", 0.05, 1),
                _tranche("EQ", 80.0, 100.0, "Equity", "200000", 0.0, 4),
            ],
            "total_size": _money("1000000"),
        },
        "closing_date": "2024-01-01",
        "first_payment_date": "2025-02-01",
        "reinvestment_end_date": None,
        "maturity": "2026-01-01",
        "frequency": {"count": 1, "unit": "months"},
        "payment_calendar_id": "nyse",
        "discount_curve_id": "USD-OIS",
        "pricing_overrides": {"mc_paths": 1},
        "attributes": {},
        "prepayment_spec": {"cpr": 0.0, "curve": None},
        "default_spec": {"cdr": 0.0, "curve": None},
        "recovery_spec": {"rate": 0.4, "recovery_lag": 0},
        "stochastic_prepay_spec": {"model": "Deterministic", "cpr": 0.0, "curve": None},
        "stochastic_default_spec": {"model": "Deterministic", "cdr": 0.0, "curve": None},
        "market_conditions": {
            "refi_rate": 0.04,
            "original_rate": None,
            "hpa": None,
            "unemployment": None,
            "seasonal_factor": 1.0,
            "custom_factors": {},
        },
        "credit_factors": {
            "credit_score": None,
            "dti": None,
            "ltv": None,
            "delinquency_days": 0,
            "unemployment_rate": None,
            "custom_factors": {},
        },
    }
    return json.dumps({"type": "structured_credit", "spec": spec})


def _market_json(include_discount: bool = True) -> str:
    curves: list[dict[str, object]] = []
    if include_discount:
        curves.append({
            "type": "discount",
            "id": "USD-OIS",
            "base": "2024-01-01",
            "day_count": "Act360",
            "knot_points": [[0.0, 1.0], [5.0, 0.90]],
            "interp_style": "monotone_convex",
            "extrapolation": "flat_forward",
            "min_forward_rate": None,
            "allow_non_monotonic": False,
            "min_forward_tenor": 1e-6,
        })
    return json.dumps({
        "version": 2,
        "curves": curves,
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


def test_structured_credit_stochastic_json_details_include_all_tranches() -> None:
    result = json.loads(
        price_instrument(
            _structured_credit_json(),
            _market_json(),
            "2024-01-01",
            "structured_credit_stochastic",
        )
    )

    details = result["details"]
    assert details["type"] == "structured_credit_stochastic"
    assert len(details["data"]["tranche_results"]) == 2
    assert {row["tranche_id"] for row in details["data"]["tranche_results"]} == {"SR", "EQ"}


def test_structured_credit_stochastic_json_missing_market_data_raises() -> None:
    with pytest.raises(ValueError, match="Curve not found"):
        price_instrument(
            _structured_credit_json(),
            _market_json(include_discount=False),
            "2024-01-01",
            "structured_credit_stochastic",
        )
