"""Tests for attribution JSON serialization and envelope support."""

import json

import pytest


def test_attribution_from_json_minimal() -> None:
    """Test attribute_pnl_from_json with a minimal bond example."""
    from finstack.valuations import attribute_pnl_from_json

    # Minimal attribution request JSON
    spec = {
        "schema": "finstack.attribution/1",
        "attribution": {
            "instrument": {
                "type": "bond",
                "spec": {
                    "id": "TEST-BOND",
                    "notional": {"amount": "1000000", "currency": "USD"},
                    "issue": "2024-01-01",
                    "maturity": "2029-01-01",
                    "cashflow_spec": {
                        "Fixed": {
                            "coupon_type": "Cash",
                            "rate": 0.045,
                            "freq": {"count": 6, "unit": "months"},
                            "dc": "Thirty360",
                            "bdc": "following",
                            "calendar_id": None,
                            "stub": "None",
                        }
                    },
                    "discount_curve_id": "USD-OIS",
                    "credit_curve_id": None,
                    "pricing_overrides": {
                        "quoted_clean_price": None,
                        "implied_volatility": None,
                        "quoted_spread_bp": None,
                        "upfront_payment": None,
                        "ytm_bump_decimal": None,
                        "theta_period": None,
                        "mc_seed_scenario": None,
                        "adaptive_bumps": False,
                        "spot_bump_pct": None,
                        "vol_bump_pct": None,
                        "rate_bump_bp": None,
                    },
                    "call_put": None,
                    "attributes": {"tags": [], "meta": {}},
                    "settlement_days": None,
                    "ex_coupon_days": None,
                },
            },
            "market_t0": {
                "curves": [],
                "surfaces": [],
                "prices": {},
                "series": [],
                "inflation_indices": [],
                "credit_indices": [],
                "collateral": {},
            },
            "market_t1": {
                "curves": [],
                "surfaces": [],
                "prices": {},
                "series": [],
                "inflation_indices": [],
                "credit_indices": [],
                "collateral": {},
            },
            "as_of_t0": "2025-01-15",
            "as_of_t1": "2025-01-16",
            "method": "Parallel",
        },
    }

    # Serialize to JSON string
    spec_json = json.dumps(spec)

    # Call the Rust function
    # Note: This will fail with missing curve error, but that confirms the function
    # is properly parsing JSON and attempting execution
    with pytest.raises(Exception, match=r"(USD-OIS|not found)"):
        attribute_pnl_from_json(spec_json)


def test_attribution_from_json_with_waterfall() -> None:
    """Test JSON attribution with waterfall method."""
    from finstack.valuations import attribute_pnl_from_json

    spec = {
        "schema": "finstack.attribution/1",
        "attribution": {
            "instrument": {
                "type": "bond",
                "spec": {
                    "id": "WATERFALL-BOND",
                    "notional": {"amount": "1000000", "currency": "USD"},
                    "issue": "2024-01-01",
                    "maturity": "2029-01-01",
                    "cashflow_spec": {
                        "Fixed": {
                            "coupon_type": "Cash",
                            "rate": 0.045,
                            "freq": {"count": 6, "unit": "months"},
                            "dc": "Thirty360",
                            "bdc": "following",
                            "calendar_id": None,
                            "stub": "None",
                        }
                    },
                    "discount_curve_id": "USD-OIS",
                    "credit_curve_id": None,
                    "pricing_overrides": {
                        "quoted_clean_price": None,
                        "implied_volatility": None,
                        "quoted_spread_bp": None,
                        "upfront_payment": None,
                        "ytm_bump_decimal": None,
                        "theta_period": None,
                        "mc_seed_scenario": None,
                        "adaptive_bumps": False,
                        "spot_bump_pct": None,
                        "vol_bump_pct": None,
                        "rate_bump_bp": None,
                    },
                    "call_put": None,
                    "attributes": {"tags": [], "meta": {}},
                    "settlement_days": None,
                    "ex_coupon_days": None,
                },
            },
            "market_t0": {
                "curves": [],
                "surfaces": [],
                "prices": {},
                "series": [],
                "inflation_indices": [],
                "credit_indices": [],
                "collateral": {},
            },
            "market_t1": {
                "curves": [],
                "surfaces": [],
                "prices": {},
                "series": [],
                "inflation_indices": [],
                "credit_indices": [],
                "collateral": {},
            },
            "as_of_t0": "2025-01-15",
            "as_of_t1": "2025-01-16",
            "method": {"Waterfall": ["Carry", "RatesCurves", "CreditCurves"]},
        },
    }

    spec_json = json.dumps(spec)

    # This will fail with missing curve error, but confirms JSON parsing works
    with pytest.raises(Exception, match=r"(USD-OIS|not found)"):
        attribute_pnl_from_json(spec_json)


def test_attribution_from_json_with_config() -> None:
    """Test JSON attribution with optional config overrides."""
    from finstack.valuations import attribute_pnl_from_json

    spec = {
        "schema": "finstack.attribution/1",
        "attribution": {
            "instrument": {
                "type": "bond",
                "spec": {
                    "id": "CONFIG-BOND",
                    "notional": {"amount": "1000000", "currency": "USD"},
                    "issue": "2024-01-01",
                    "maturity": "2029-01-01",
                    "cashflow_spec": {
                        "Fixed": {
                            "coupon_type": "Cash",
                            "rate": 0.045,
                            "freq": {"count": 6, "unit": "months"},
                            "dc": "Thirty360",
                            "bdc": "following",
                            "calendar_id": None,
                            "stub": "None",
                        }
                    },
                    "discount_curve_id": "USD-OIS",
                    "credit_curve_id": None,
                    "pricing_overrides": {
                        "quoted_clean_price": None,
                        "implied_volatility": None,
                        "quoted_spread_bp": None,
                        "upfront_payment": None,
                        "ytm_bump_decimal": None,
                        "theta_period": None,
                        "mc_seed_scenario": None,
                        "adaptive_bumps": False,
                        "spot_bump_pct": None,
                        "vol_bump_pct": None,
                        "rate_bump_bp": None,
                    },
                    "call_put": None,
                    "attributes": {"tags": [], "meta": {}},
                    "settlement_days": None,
                    "ex_coupon_days": None,
                },
            },
            "market_t0": {
                "curves": [],
                "surfaces": [],
                "prices": {},
                "series": [],
                "inflation_indices": [],
                "credit_indices": [],
                "collateral": {},
            },
            "market_t1": {
                "curves": [],
                "surfaces": [],
                "prices": {},
                "series": [],
                "inflation_indices": [],
                "credit_indices": [],
                "collateral": {},
            },
            "as_of_t0": "2025-01-15",
            "as_of_t1": "2025-01-16",
            "method": "MetricsBased",
            "config": {
                "tolerance_abs": 0.01,
                "tolerance_pct": 0.001,
                "metrics": ["theta", "dv01"],
            },
        },
    }

    spec_json = json.dumps(spec)

    # This will fail with missing curve error, but confirms JSON parsing works
    with pytest.raises(Exception, match=r"(USD-OIS|not found)"):
        attribute_pnl_from_json(spec_json)
