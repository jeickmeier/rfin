"""Shared pricing helpers for instrument-level golden fixtures."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from finstack.core.market_data import MarketContext
from jsonschema import validators

from finstack.valuations import ValuationResult, price_instrument_with_metrics, validate_instrument_json
from tests.golden.schema import GoldenFixture

WORKSPACE_ROOT = Path(__file__).resolve().parents[4]
INSTRUMENT_ENVELOPE_SCHEMA_PATH = WORKSPACE_ROOT / "finstack/valuations/schemas/instruments/1/instrument.schema.json"


def run_pricing_fixture(fixture: GoldenFixture) -> dict[str, float]:
    """Run one common pricing fixture through the Python bindings."""
    inputs = fixture.inputs
    market = MarketContext.from_json(json.dumps(inputs["market"]))
    instrument_json = _validated_instrument_json(inputs["instrument_json"])
    result_json = price_instrument_with_metrics(
        instrument_json,
        market,
        inputs["valuation_date"],
        model=inputs["model"],
        metrics=list(inputs["metrics"]),
    )
    result = ValuationResult.from_json(result_json)

    actuals: dict[str, float] = {}
    for metric in fixture.expected_outputs:
        if metric == "npv":
            actuals[metric] = float(result.price)
            continue
        value = result.get_metric(metric)
        if value is None:
            raise ValueError(f"result missing metric {metric!r}")
        actuals[metric] = float(value)
    return actuals


def _validated_instrument_json(instrument_json: dict[str, Any]) -> str:
    if _is_instrument_envelope(instrument_json):
        _validate_instrument_envelope_schema(instrument_json)
        return json.dumps(instrument_json)
    return validate_instrument_json(json.dumps(instrument_json))


def _is_instrument_envelope(instrument_json: dict[str, Any]) -> bool:
    return "schema" in instrument_json and "instrument" in instrument_json


def _validate_instrument_envelope_schema(instrument_json: dict[str, Any]) -> None:
    schema = json.loads(INSTRUMENT_ENVELOPE_SCHEMA_PATH.read_text(encoding="utf-8"))
    validator_cls = validators.validator_for(schema)
    validator_cls.check_schema(schema)
    validator = validator_cls(schema)
    errors = sorted(validator.iter_errors(instrument_json), key=lambda error: list(error.path))
    if errors:
        details = "\n  ".join(error.message for error in errors)
        msg = f"instrument_json failed {INSTRUMENT_ENVELOPE_SCHEMA_PATH.name} validation:\n  {details}"
        raise ValueError(msg)
