"""Domain runner for `rates.irs` golden fixtures."""

from __future__ import annotations

from datetime import date
import json
from typing import Any

from finstack.core.market_data import DiscountCurve, ForwardCurve, MarketContext

from finstack.valuations import ValuationResult, price_instrument_with_metrics
from tests.golden.schema import GoldenFixture


def run(fixture: GoldenFixture) -> dict[str, float]:
    """Run one IRS fixture through the Python bindings."""
    inputs = fixture.inputs
    market = _build_market(inputs["curves"])
    result_json = price_instrument_with_metrics(
        json.dumps(inputs["instrument_json"]),
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


def _build_market(curves: dict[str, Any]) -> MarketContext:
    market = MarketContext()
    for spec in curves["discount"]:
        market = market.insert(_build_discount_curve(spec))
    for spec in curves.get("forward", []):
        market = market.insert(_build_forward_curve(spec))
    return market


def _build_discount_curve(spec: dict[str, Any]) -> DiscountCurve:
    return DiscountCurve(
        id=spec["id"],
        base_date=date.fromisoformat(spec["base_date"]),
        knots=[(float(t), float(df)) for t, df in spec["knots"]],
        interp=spec.get("interp", "linear"),
        day_count=spec.get("day_count"),
    )


def _build_forward_curve(spec: dict[str, Any]) -> ForwardCurve:
    return ForwardCurve(
        id=spec["id"],
        tenor=float(spec["tenor"]),
        base_date=date.fromisoformat(spec["base_date"]),
        knots=[(float(t), float(rate)) for t, rate in spec["knots"]],
        interp=spec.get("interp", "linear"),
        day_count=spec.get("day_count", "act_360"),
    )
