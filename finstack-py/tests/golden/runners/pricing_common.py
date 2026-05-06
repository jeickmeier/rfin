"""Shared pricing helpers for instrument-level golden fixtures."""

from __future__ import annotations

import json

from finstack.core.market_data import MarketContext

from finstack.valuations import ValuationResult, price_instrument_with_metrics
from tests.golden.pricing_validation import validated_instrument_json
from tests.golden.runners import validate_source_validation_fixture
from tests.golden.schema import GoldenFixture


def run_pricing_fixture(fixture: GoldenFixture) -> dict[str, float]:
    """Run one common pricing fixture through the Python bindings."""
    validate_source_validation_fixture("pricing runner", fixture)

    inputs = fixture.inputs
    market = MarketContext.from_json(json.dumps(inputs["market"]))
    instrument_json = validated_instrument_json(inputs["instrument_json"])
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


def run(fixture: GoldenFixture) -> dict[str, float]:
    """Run a fixture that follows the shared pricing input contract."""
    return run_pricing_fixture(fixture)
