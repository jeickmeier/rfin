"""Shared pricing helpers for instrument-level golden fixtures."""

from __future__ import annotations

from datetime import date
import json
from pathlib import Path
from typing import Any

from finstack.core.market_data import (
    BaseCorrelationCurve,
    CreditIndexData,
    DiscountCurve,
    ForwardCurve,
    FxMatrix,
    HazardCurve,
    InflationCurve,
    MarketContext,
    VolSurface,
)
from jsonschema import validators

from finstack.valuations import ValuationResult, price_instrument_with_metrics, validate_instrument_json
from tests.golden.schema import GoldenFixture

WORKSPACE_ROOT = Path(__file__).resolve().parents[4]
INSTRUMENT_ENVELOPE_SCHEMA_PATH = WORKSPACE_ROOT / "finstack/valuations/schemas/instruments/1/instrument.schema.json"


def run_pricing_fixture(fixture: GoldenFixture) -> dict[str, float]:
    """Run one common pricing fixture through the Python bindings."""
    inputs = fixture.inputs
    market = _build_market(inputs["curves"])
    for spec in inputs.get("surfaces", {}).get("vol", []):
        market = market.insert(_build_vol_surface(spec))
    for spec in inputs.get("prices", []):
        _insert_price(market, spec)
    for spec in inputs.get("credit_indices", []):
        _insert_credit_index(market, spec)
    if "fx" in inputs:
        market.insert_fx(_build_fx_matrix(inputs["fx"]))
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


def _build_market(curves: dict[str, Any]) -> MarketContext:
    market = MarketContext()
    for spec in curves["discount"]:
        market = market.insert(_build_discount_curve(spec))
    for spec in curves.get("forward", []):
        market = market.insert(_build_forward_curve(spec))
    for spec in curves.get("hazard", []):
        market = market.insert(_build_hazard_curve(spec))
    for spec in curves.get("inflation", []):
        market = market.insert(_build_inflation_curve(spec))
    return market


def _build_fx_matrix(quotes: list[dict[str, Any]]) -> FxMatrix:
    fx = FxMatrix()
    for quote in quotes:
        fx.set_quote(quote["base"], quote["quote"], float(quote["rate"]))
    return fx


def _build_vol_surface(spec: dict[str, Any]) -> VolSurface:
    return VolSurface(
        id=spec["id"],
        expiries=[float(expiry) for expiry in spec["expiries"]],
        strikes=[float(strike) for strike in spec["strikes"]],
        vols_row_major=[float(vol) for vol in spec["vols_row_major"]],
        secondary_axis=spec.get("secondary_axis", "strike"),
        interpolation_mode=spec.get("interpolation_mode", spec.get("mode", "vol")),
    )


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


def _insert_price(market: MarketContext, spec: dict[str, Any]) -> None:
    market.insert_price(
        spec["id"],
        float(spec["value"]),
        currency=spec.get("currency"),
    )


def _insert_credit_index(market: MarketContext, spec: dict[str, Any]) -> None:
    index_curve = market.get_hazard(spec["index_credit_curve_id"])
    base_correlation = _build_base_correlation_curve(spec["base_correlation_curve"])
    data = CreditIndexData(
        num_constituents=int(spec["num_constituents"]),
        recovery_rate=float(spec["recovery_rate"]),
        index_credit_curve=index_curve,
        base_correlation_curve=base_correlation,
    )
    market.insert_credit_index(spec["id"], data)


def _build_base_correlation_curve(spec: dict[str, Any]) -> BaseCorrelationCurve:
    return BaseCorrelationCurve(
        id=spec["id"],
        knots=[(float(detachment), float(correlation)) for detachment, correlation in spec["knots"]],
    )


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


def _build_hazard_curve(spec: dict[str, Any]) -> HazardCurve:
    return HazardCurve(
        id=spec["id"],
        base_date=date.fromisoformat(spec["base_date"]),
        knots=[(float(t), float(rate)) for t, rate in spec["knots"]],
        recovery_rate=spec.get("recovery_rate"),
        day_count=spec.get("day_count", "act_365f"),
        par_spreads=[(float(t), float(spread)) for t, spread in spec.get("par_spreads", [])],
    )


def _build_inflation_curve(spec: dict[str, Any]) -> InflationCurve:
    return InflationCurve(
        id=spec["id"],
        base_date=date.fromisoformat(spec["base_date"]),
        base_cpi=float(spec["base_cpi"]),
        knots=[(float(t), float(cpi)) for t, cpi in spec["knots"]],
        day_count=spec.get("day_count", "act_365f"),
        indexation_lag_months=int(spec.get("indexation_lag_months", 3)),
        interp=spec.get("interp", "log_linear"),
    )
